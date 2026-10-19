use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use mm_domain::{TrustedPeer, TrustedPeerGrants};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, TrustedPeerPort};
use mm_protocol::MAX_ENVELOPE_BYTES;
use mm_runtime::{AdmittedPendingPairingApproval, PairingRuntime, PairingRuntimeError};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use tungstenite::handshake::HandshakeError;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http::{Method, StatusCode};
use tungstenite::protocol::{Message, WebSocket, WebSocketConfig};
use tungstenite::{accept_hdr_with_config, error::Error as WebSocketError};

pub const CONTROL_WEBSOCKET_PATH: &str = "/v1/control";
pub const MAX_PAIR_BEGIN_BYTES: usize = MAX_ENVELOPE_BYTES;
pub const MAX_PENDING_PAIRING_CONNECTIONS: usize = 8;

const SOCKET_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const SOCKET_READ_BUFFER_BYTES: usize = 8 * 1024;
const SOCKET_WRITE_BUFFER_BYTES: usize = 4 * 1024;

pub type PendingPairingWebSocket =
    WebSocket<StreamOwned<ServerConnection, HandshakeDeadlineStream>>;

/// A pending pairing request and the authenticated WSS connection that carried it.
///
/// The application owns this value while it shows an explicit approval prompt. The listener
/// never persists trust or approves grants. Drop the connection when the request is denied or
/// expires; do not begin a session before `PairingRuntime::approve` succeeds.
pub struct PendingPairingConnection {
    admitted: AdmittedPendingPairingApproval,
    connection: PairingConnection,
}

impl PendingPairingConnection {
    pub fn admitted(&self) -> &AdmittedPendingPairingApproval {
        &self.admitted
    }

    pub fn websocket_mut(&mut self) -> &mut PendingPairingWebSocket {
        &mut self.connection.websocket
    }

    /// Persists the explicitly approved grants only while the pending approval is valid.
    ///
    /// The live connection is returned only after runtime approval succeeds. If approval has
    /// expired or otherwise fails, consuming `self` drops the connection and releases its slot.
    pub fn approve<P, I>(
        self,
        runtime: &PairingRuntime<P, I>,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<(TrustedPeer, PairingConnection), PairingRuntimeError>
    where
        P: TrustedPeerPort,
        I: LocalPairingIdentityPort + PairingIdentityPort,
    {
        let Self {
            admitted,
            connection,
        } = self;
        let peer = runtime.approve(
            admitted,
            approved_grants,
            now_monotonic_ms,
            paired_at_unix_ms,
        )?;
        Ok((peer, connection))
    }
}

/// A live WSS connection that retains its listener capacity slot until dropped.
pub struct PairingConnection {
    websocket: PendingPairingWebSocket,
    _slot: PendingConnectionSlot,
}

impl PairingConnection {
    pub fn websocket_mut(&mut self) -> &mut PendingPairingWebSocket {
        &mut self.websocket
    }
}

struct PendingConnectionSlot {
    active: Arc<AtomicUsize>,
}

impl PendingConnectionSlot {
    fn try_acquire(active: Arc<AtomicUsize>) -> Option<Self> {
        let mut current = active.load(Ordering::Acquire);
        loop {
            if current >= MAX_PENDING_PAIRING_CONNECTIONS {
                return None;
            }
            match active.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self { active }),
                Err(observed) => current = observed,
            }
        }
    }
}

impl Drop for PendingConnectionSlot {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[doc(hidden)]
pub struct HandshakeDeadlineStream {
    socket: TcpStream,
    deadline: Option<Instant>,
}

impl HandshakeDeadlineStream {
    fn new(socket: TcpStream, deadline: Instant) -> Self {
        Self {
            socket,
            deadline: Some(deadline),
        }
    }

    fn clear_deadline(&mut self) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            remaining_until(deadline, Instant::now())?;
            self.deadline = None;
        }
        self.socket.set_read_timeout(None)?;
        self.socket.set_write_timeout(None)
    }

    fn update_read_timeout(&self) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            self.socket
                .set_read_timeout(Some(remaining_until(deadline, Instant::now())?))?;
        }
        Ok(())
    }

    fn update_write_timeout(&self) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            self.socket
                .set_write_timeout(Some(remaining_until(deadline, Instant::now())?))?;
        }
        Ok(())
    }
}

impl Read for HandshakeDeadlineStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.update_read_timeout()?;
        self.socket.read(buffer)
    }
}

impl Write for HandshakeDeadlineStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.update_write_timeout()?;
        self.socket.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.update_write_timeout()?;
        self.socket.flush()
    }
}

fn remaining_until(deadline: Instant, now: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(now)
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "WSS handshake deadline elapsed"))
}

/// A blocking listener candidate for native TLS 1.3 raw-public-key WSS pairing.
///
/// Run `accept_pairing_request` on a dedicated worker. It accepts one connection at a time,
/// captures the socket peer IP before TLS/WebSocket parsing, and hands the first valid `pair.begin`
/// to the application runtime for source admission and signature verification.
pub struct WssPairingListener {
    listener: TcpListener,
    tls_config: Arc<ServerConfig>,
    active_pending_connections: Arc<AtomicUsize>,
}

impl WssPairingListener {
    pub fn bind(address: SocketAddr, tls_config: Arc<ServerConfig>) -> Result<Self, io::Error> {
        Ok(Self {
            listener: TcpListener::bind(address)?,
            tls_config,
            active_pending_connections: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.listener.local_addr()
    }

    /// Accepts and verifies one pairing request without approving or persisting it.
    pub fn accept_pairing_request<P, I>(
        &self,
        runtime: &mut PairingRuntime<P, I>,
        now_monotonic_ms: u64,
    ) -> Result<PendingPairingConnection, WssListenerError>
    where
        P: TrustedPeerPort,
        I: LocalPairingIdentityPort + PairingIdentityPort,
    {
        let slot = PendingConnectionSlot::try_acquire(Arc::clone(&self.active_pending_connections))
            .ok_or(WssListenerError::PendingCapacityLimited)?;
        let (socket, _) = self.listener.accept().map_err(WssListenerError::Io)?;
        let handshake_deadline = Instant::now() + SOCKET_HANDSHAKE_TIMEOUT;
        let source_ip = socket.peer_addr().map_err(WssListenerError::Io)?.ip();
        socket.set_nodelay(true).map_err(WssListenerError::Io)?;

        let tls = StreamOwned::new(
            ServerConnection::new(Arc::clone(&self.tls_config)).map_err(WssListenerError::Tls)?,
            HandshakeDeadlineStream::new(socket, handshake_deadline),
        );
        let websocket = accept_hdr_with_config(
            tls,
            validate_control_websocket_handshake,
            Some(websocket_config()),
        )
        .map_err(map_handshake_error)?;

        let mut websocket = websocket;
        let frame = loop {
            match websocket.read().map_err(WssListenerError::WebSocket)? {
                Message::Text(text) => break text,
                Message::Ping(_) => websocket.flush().map_err(WssListenerError::WebSocket)?,
                Message::Pong(_) => {}
                Message::Binary(_) | Message::Close(_) => {
                    return Err(WssListenerError::UnexpectedFirstMessage);
                }
                Message::Frame(_) => unreachable!("tungstenite does not expose raw frames"),
            }
        };

        let admitted = runtime
            .verify_pair_begin_from(source_ip, frame.as_str().as_bytes(), now_monotonic_ms)
            .map_err(WssListenerError::Pairing)?;
        websocket
            .get_mut()
            .sock
            .clear_deadline()
            .map_err(WssListenerError::Io)?;

        Ok(PendingPairingConnection {
            admitted,
            connection: PairingConnection {
                websocket,
                _slot: slot,
            },
        })
    }
}

#[derive(Debug)]
pub enum WssListenerError {
    Io(io::Error),
    Tls(rustls::Error),
    WebSocket(WebSocketError),
    WebSocketHandshake(String),
    Pairing(PairingRuntimeError),
    UnexpectedFirstMessage,
    PendingCapacityLimited,
}

impl fmt::Display for WssListenerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "WSS socket failed: {error}"),
            Self::Tls(error) => write!(formatter, "WSS TLS setup failed: {error}"),
            Self::WebSocket(error) => write!(formatter, "WSS protocol failed: {error}"),
            Self::WebSocketHandshake(error) => {
                write!(formatter, "WSS upgrade handshake failed: {error}")
            }
            Self::Pairing(error) => write!(formatter, "pairing request was rejected: {error}"),
            Self::UnexpectedFirstMessage => {
                formatter.write_str("first WSS application message must be text pair.begin")
            }
            Self::PendingCapacityLimited => {
                formatter.write_str("pending WSS pairing connection capacity is full")
            }
        }
    }
}

impl std::error::Error for WssListenerError {}

fn map_handshake_error<S, C>(
    error: HandshakeError<tungstenite::handshake::server::ServerHandshake<S, C>>,
) -> WssListenerError
where
    S: io::Read + io::Write,
    C: tungstenite::handshake::server::Callback,
{
    WssListenerError::WebSocketHandshake(error.to_string())
}

fn is_control_request(request: &Request) -> bool {
    request.method() == Method::GET
        && request.uri().path() == CONTROL_WEBSOCKET_PATH
        && request.uri().query().is_none()
        && !request.headers().contains_key("origin")
}

// Tungstenite's Callback API requires this concrete, non-boxed HTTP error response type.
#[allow(clippy::result_large_err)]
fn validate_control_websocket_handshake(
    request: &Request,
    response: Response,
) -> Result<Response, ErrorResponse> {
    if is_control_request(request) {
        Ok(response)
    } else {
        Err(rejected_upgrade_response(request))
    }
}

fn rejected_upgrade_response(request: &Request) -> ErrorResponse {
    let status = if request.headers().contains_key("origin") {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::NOT_FOUND
    };
    Response::builder()
        .status(status)
        .body(Some("WSS pairing endpoint unavailable".to_owned()))
        .expect("static WebSocket rejection response is valid")
}

fn websocket_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .read_buffer_size(SOCKET_READ_BUFFER_BYTES)
        .write_buffer_size(SOCKET_WRITE_BUFFER_BYTES)
        .max_write_buffer_size(SOCKET_WRITE_BUFFER_BYTES + MAX_PAIR_BEGIN_BYTES)
        .max_message_size(Some(MAX_PAIR_BEGIN_BYTES))
        .max_frame_size(Some(MAX_PAIR_BEGIN_BYTES))
}

#[cfg(test)]
mod tests;
