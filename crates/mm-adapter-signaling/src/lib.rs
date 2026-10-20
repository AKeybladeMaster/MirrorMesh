use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use mm_adapter_tls::{TlsAdapterError, tls13_pinned_client_config};
use mm_application::{PairingProof, PairingProofSigningError};
use mm_domain::{
    DeviceId, MessageId, PeerFingerprint, PeerGrant, SessionId, TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, TrustedPeerPort};
use mm_protocol::{
    EnvelopeHeader, MAX_ENVELOPE_BYTES, MessageKind, PairApprove, PairBegin, PairReject,
    ProtocolError, SequenceTracker, ValidatedMessage, encode_pair_approve, encode_pair_begin,
    encode_pair_reject, parse_message,
};
use mm_runtime::{AdmittedPendingPairingApproval, PairingRuntime, PairingRuntimeError};
use mm_security::{PairingQrError, parse_pairing_qr};
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, ServerConfig, ServerConnection, StreamOwned};
use tungstenite::client::{IntoClientRequest, client_with_config};
use tungstenite::handshake::HandshakeError;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http::{Method, StatusCode};
use tungstenite::protocol::{Message, WebSocket, WebSocketConfig};
use tungstenite::{accept_hdr_with_config, error::Error as WebSocketError};

pub const CONTROL_WEBSOCKET_PATH: &str = "/v1/control";
pub const MAX_PAIR_BEGIN_BYTES: usize = MAX_ENVELOPE_BYTES;
pub const MAX_PENDING_PAIRING_CONNECTIONS: usize = 8;

const SOCKET_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const LISTENER_ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const SOCKET_READ_BUFFER_BYTES: usize = 8 * 1024;
const SOCKET_WRITE_BUFFER_BYTES: usize = 4 * 1024;
const CONTROL_READ_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub type PendingPairingWebSocket =
    WebSocket<StreamOwned<ServerConnection, HandshakeDeadlineStream>>;

trait WssSocket: Send {
    fn send_message(&mut self, message: Message) -> Result<(), WebSocketError>;
    fn read_message(&mut self) -> Result<Option<Message>, WebSocketError>;
    fn flush_socket(&mut self) -> Result<(), WebSocketError>;
    fn close_socket(&mut self) -> Result<(), WebSocketError>;
}

impl<S> WssSocket for WebSocket<S>
where
    S: Read + Write + Send + 'static,
{
    fn send_message(&mut self, message: Message) -> Result<(), WebSocketError> {
        self.send(message)
    }

    fn read_message(&mut self) -> Result<Option<Message>, WebSocketError> {
        match self.read() {
            Ok(message) => Ok(Some(message)),
            Err(WebSocketError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn flush_socket(&mut self) -> Result<(), WebSocketError> {
        self.flush()
    }

    fn close_socket(&mut self) -> Result<(), WebSocketError> {
        self.close(None)
    }
}

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
    ) -> Result<(TrustedPeer, PairingConnection), WssApprovalError>
    where
        P: TrustedPeerPort,
        I: LocalPairingIdentityPort + PairingIdentityPort,
    {
        let Self {
            admitted,
            mut connection,
        } = self;
        let peer = runtime
            .approve(
                admitted,
                approved_grants,
                now_monotonic_ms,
                paired_at_unix_ms,
            )
            .map_err(WssApprovalError::Runtime)?;
        connection
            .send_pairing_approval(&peer, runtime.local_fingerprint(), paired_at_unix_ms)
            .map_err(WssApprovalError::Control)?;
        Ok((peer, connection))
    }

    /// Sends an explicit rejection decision without persisting trust, then closes the connection.
    pub fn reject(mut self, reason: &str, sent_at_ms: u64) -> Result<(), WssControlChannelError> {
        let payload = PairReject {
            reason: reason.to_owned(),
        };
        let frame = encode_pair_reject(random_message_id()?, 1, sent_at_ms, &payload)?;
        let text = String::from_utf8(frame)
            .map_err(|_| WssControlChannelError::Protocol(ProtocolError::InvalidUtf8))?;
        self.connection
            .socket
            .send_message(Message::text(text))
            .map_err(WssControlChannelError::WebSocket)?;
        self.connection
            .socket
            .close_socket()
            .map_err(WssControlChannelError::WebSocket)
    }
}

/// A pairing approval can be persisted successfully even if its response cannot be delivered.
#[derive(Debug)]
pub enum WssApprovalError {
    Runtime(PairingRuntimeError),
    Control(WssControlChannelError),
}

impl fmt::Display for WssApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(formatter, "pairing approval failed: {error}"),
            Self::Control(error) => write!(formatter, "approved peer response failed: {error}"),
        }
    }
}

impl std::error::Error for WssApprovalError {}

/// A live WSS connection that retains its listener capacity slot until dropped.
pub struct PairingConnection {
    peer_id: DeviceId,
    socket: Box<dyn WssSocket>,
    inbound_sequences: ControlSequenceState,
    outbound_sequences: ControlSequenceState,
    control_ready: bool,
    _slot: Option<PendingConnectionSlot>,
}

impl PairingConnection {
    fn new(
        peer_id: DeviceId,
        websocket: PendingPairingWebSocket,
        slot: PendingConnectionSlot,
    ) -> Self {
        Self {
            peer_id,
            socket: Box::new(websocket),
            inbound_sequences: ControlSequenceState::new(),
            outbound_sequences: ControlSequenceState::new(),
            control_ready: false,
            _slot: Some(slot),
        }
    }

    fn new_client(peer_id: DeviceId, socket: Box<dyn WssSocket>) -> Self {
        Self {
            peer_id,
            socket,
            inbound_sequences: ControlSequenceState::new(),
            outbound_sequences: ControlSequenceState::new(),
            control_ready: true,
            _slot: None,
        }
    }

    /// Returns the peer identity established by the approved pairing proof.
    pub const fn peer_id(&self) -> DeviceId {
        self.peer_id
    }

    /// Sends one bounded v1 control message on the approved WSS connection.
    ///
    /// Pairing handshake messages and `device.hello` are rejected here. The caller remains
    /// responsible for authorizing session operations and applying trusted-peer grants before
    /// sending them.
    pub fn send_control_message(
        &mut self,
        frame: &[u8],
    ) -> Result<EnvelopeHeader, WssControlChannelError> {
        self.ensure_control_ready()?;
        let message = parse_post_approval_control_frame(frame)?;
        self.outbound_sequences
            .accept(self.peer_id, &message.header)?;
        let header = message.header;
        let text = std::str::from_utf8(frame)
            .map_err(|_| WssControlChannelError::Protocol(ProtocolError::InvalidUtf8))?;
        self.socket
            .send_message(Message::text(text))
            .map_err(WssControlChannelError::WebSocket)?;
        Ok(header)
    }

    /// Reads, validates, and replay-checks one control message from the approved peer.
    ///
    /// This is blocking I/O and belongs on a dedicated worker. Ping frames are answered by
    /// flushing the WebSocket; binary frames are never accepted as protocol messages.
    pub fn receive_control_message(&mut self) -> Result<ValidatedMessage, WssControlChannelError> {
        loop {
            if let Some(message) = self.try_receive_control_message()? {
                return Ok(message);
            }
        }
    }

    /// Polls the approved WSS channel for one validated message without blocking past its
    /// bounded read interval. Platform bridges can share a connection between reads, sends,
    /// and shutdown without holding a connection lock indefinitely.
    pub fn try_receive_control_message(
        &mut self,
    ) -> Result<Option<ValidatedMessage>, WssControlChannelError> {
        self.ensure_control_ready()?;
        let frame = match self.socket.read_message() {
            Ok(Some(Message::Text(text))) => text,
            Ok(Some(Message::Ping(_))) => {
                self.socket
                    .flush_socket()
                    .map_err(WssControlChannelError::WebSocket)?;
                return Ok(None);
            }
            Ok(Some(Message::Pong(_))) => return Ok(None),
            Ok(Some(Message::Binary(_))) => {
                return Err(WssControlChannelError::UnexpectedBinaryMessage);
            }
            Ok(Some(Message::Close(_))) => return Err(WssControlChannelError::Closed),
            Ok(Some(Message::Frame(_))) => {
                unreachable!("tungstenite does not expose raw frames")
            }
            Ok(None) => return Ok(None),
            Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => {
                return Err(WssControlChannelError::Closed);
            }
            Err(error) => return Err(WssControlChannelError::WebSocket(error)),
        };
        let message = parse_post_approval_control_frame(frame.as_str().as_bytes())?;
        self.inbound_sequences
            .accept(self.peer_id, &message.header)?;
        Ok(Some(message))
    }

    /// Releases sequence-tracker capacity after the caller has fully closed a session.
    pub fn forget_session(&mut self, session_id: SessionId) {
        self.inbound_sequences.forget_session(session_id);
        self.outbound_sequences.forget_session(session_id);
    }

    /// Starts a WebSocket close handshake. Dropping this connection also releases its slot.
    pub fn close(&mut self) -> Result<(), WssControlChannelError> {
        self.socket
            .close_socket()
            .map_err(WssControlChannelError::WebSocket)
    }

    fn send_pairing_approval(
        &mut self,
        trusted_peer: &TrustedPeer,
        local_fingerprint: PeerFingerprint,
        sent_at_ms: u64,
    ) -> Result<(), WssControlChannelError> {
        if self.control_ready {
            return Err(WssControlChannelError::AlreadyApproved);
        }
        let grants = [
            (PeerGrant::View, "view"),
            (PeerGrant::Send, "send"),
            (PeerGrant::Annotate, "annotate"),
        ]
        .into_iter()
        .filter(|(grant, _)| trusted_peer.grants().allows(*grant))
        .map(|(_, value)| value.to_owned())
        .collect();
        let payload = PairApprove {
            accepted_grants: grants,
            peer_identity_proof: local_fingerprint.to_hex(),
        };
        let frame = encode_pair_approve(random_message_id()?, 1, sent_at_ms, &payload)?;
        let text = String::from_utf8(frame)
            .map_err(|_| WssControlChannelError::Protocol(ProtocolError::InvalidUtf8))?;
        self.socket
            .send_message(Message::text(text))
            .map_err(WssControlChannelError::WebSocket)?;
        self.control_ready = true;
        Ok(())
    }

    fn ensure_control_ready(&self) -> Result<(), WssControlChannelError> {
        if self.control_ready {
            Ok(())
        } else {
            Err(WssControlChannelError::NotApproved)
        }
    }
}

fn random_message_id() -> Result<MessageId, WssControlChannelError> {
    random_message_id_raw().ok_or(WssControlChannelError::RandomnessUnavailable)
}

fn random_message_id_raw() -> Option<MessageId> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(MessageId::from_bytes(bytes))
}

/// Errors from the authenticated, post-approval WSS control channel.
#[derive(Debug)]
pub enum WssControlChannelError {
    Protocol(ProtocolError),
    WebSocket(WebSocketError),
    UnexpectedMessageKind,
    UnexpectedBinaryMessage,
    NotApproved,
    AlreadyApproved,
    RandomnessUnavailable,
    Closed,
}

impl fmt::Display for WssControlChannelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "invalid WSS control message: {error}"),
            Self::WebSocket(error) => write!(formatter, "WSS control transport failed: {error}"),
            Self::UnexpectedMessageKind => {
                formatter.write_str("message kind is not allowed after pairing approval")
            }
            Self::UnexpectedBinaryMessage => {
                formatter.write_str("binary WebSocket messages are not control messages")
            }
            Self::NotApproved => {
                formatter.write_str("pairing approval has not completed on this connection")
            }
            Self::AlreadyApproved => {
                formatter.write_str("pairing approval has already been sent on this connection")
            }
            Self::RandomnessUnavailable => {
                formatter.write_str("operating-system randomness is unavailable")
            }
            Self::Closed => formatter.write_str("WSS control connection is closed"),
        }
    }
}

impl std::error::Error for WssControlChannelError {}

impl From<ProtocolError> for WssControlChannelError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// Parses the shared protocol envelope and rejects pairing-only messages on an approved channel.
fn parse_post_approval_control_frame(
    frame: &[u8],
) -> Result<ValidatedMessage, WssControlChannelError> {
    let message = parse_message(frame)?;
    if !matches!(
        message.header.kind,
        MessageKind::PairRevoke
            | MessageKind::SessionOffer
            | MessageKind::SessionAnswer
            | MessageKind::SessionIce
            | MessageKind::SessionStart
            | MessageKind::SessionStop
            | MessageKind::SessionPause
            | MessageKind::SessionResume
            | MessageKind::SessionQuality
            | MessageKind::SessionStats
            | MessageKind::SessionError
            | MessageKind::AnnotationStrokeStart
            | MessageKind::AnnotationStrokePoints
            | MessageKind::AnnotationStrokeEnd
            | MessageKind::AnnotationUndo
            | MessageKind::AnnotationClear
    ) {
        return Err(WssControlChannelError::UnexpectedMessageKind);
    }
    Ok(message)
}

struct ControlSequenceState {
    sessions: SequenceTracker,
    last_pair_revoke_sequence: Option<u64>,
}

impl ControlSequenceState {
    fn new() -> Self {
        Self {
            sessions: SequenceTracker::new(MAX_PENDING_PAIRING_CONNECTIONS),
            last_pair_revoke_sequence: None,
        }
    }

    fn accept(
        &mut self,
        peer_id: DeviceId,
        header: &EnvelopeHeader,
    ) -> Result<(), WssControlChannelError> {
        match header.kind {
            MessageKind::PairRevoke if header.session_id.is_none() => {
                if self
                    .last_pair_revoke_sequence
                    .is_some_and(|previous| header.sequence <= previous)
                {
                    return Err(ProtocolError::SequenceReplay.into());
                }
                self.last_pair_revoke_sequence = Some(header.sequence);
                Ok(())
            }
            MessageKind::PairRevoke => Err(ProtocolError::InvalidEnvelope.into()),
            MessageKind::DeviceHello
            | MessageKind::PairBegin
            | MessageKind::PairApprove
            | MessageKind::PairReject => Err(WssControlChannelError::UnexpectedMessageKind),
            _ => {
                let session_id = header.session_id.ok_or(ProtocolError::InvalidEnvelope)?;
                self.sessions
                    .accept(session_id, peer_id, header.sequence)
                    .map_err(Into::into)
            }
        }
    }

    fn forget_session(&mut self, session_id: SessionId) {
        self.sessions.forget_session(session_id);
    }
}

/// Sender-side WSS pairing entry point for a validated QR invitation.
pub struct WssPairingClient;

impl WssPairingClient {
    /// Validates a QR invitation, pins TLS to its fingerprint, signs `pair.begin`, and submits it.
    #[allow(clippy::too_many_arguments)]
    pub fn begin_pairing<I>(
        invitation_uri: &str,
        now_unix_seconds: u64,
        approved_ports: &[u16],
        local_device_id: DeviceId,
        display_name: &str,
        requested_grants: TrustedPeerGrants,
        identity: &I,
        sent_at_ms: u64,
    ) -> Result<PendingWssPairingClient, WssClientError>
    where
        I: LocalPairingIdentityPort,
    {
        let invitation = parse_pairing_qr(invitation_uri, now_unix_seconds, approved_ports)
            .map_err(WssClientError::PairingQr)?;
        let decision_deadline = Instant::now()
            + Duration::from_secs(
                invitation
                    .expires_at_unix_seconds()
                    .saturating_sub(now_unix_seconds),
            );
        let proof = PairingProof::create_signed(
            invitation.device_id(),
            invitation.nonce(),
            local_device_id,
            display_name.to_owned(),
            requested_grants,
            identity,
        )
        .map_err(WssClientError::Proof)?;
        let payload = PairBegin {
            receiver_id: proof.receiver_id().to_string(),
            nonce: proof.nonce().to_hex(),
            peer_id: proof.peer_id().to_string(),
            display_name: proof.display_name().to_owned(),
            public_key: encode_lower_hex(proof.public_key().as_bytes()),
            requested_grants: grants_to_wire(proof.requested_grants()),
            signature: encode_lower_hex(proof.signature()),
        };
        let message_id = random_message_id_raw().ok_or(WssClientError::RandomnessUnavailable)?;
        let frame = encode_pair_begin(message_id, 1, sent_at_ms, &payload)
            .map_err(WssClientError::Protocol)?;

        let deadline = Instant::now() + SOCKET_HANDSHAKE_TIMEOUT;
        let socket = TcpStream::connect_timeout(
            &invitation.endpoint().socket_addr(),
            SOCKET_HANDSHAKE_TIMEOUT,
        )
        .map_err(WssClientError::Io)?;
        socket.set_nodelay(true).map_err(WssClientError::Io)?;
        let tls_config =
            tls13_pinned_client_config(invitation.fingerprint()).map_err(WssClientError::Tls)?;
        let server_name = ServerName::try_from("mirrormesh.local")
            .expect("static MirrorMesh TLS server name is valid");
        let tls = StreamOwned::new(
            ClientConnection::new(tls_config, server_name).map_err(WssClientError::TlsSetup)?,
            HandshakeDeadlineStream::new(socket, deadline),
        );
        let request = format!(
            "wss://mirrormesh.local:{}/v1/control",
            invitation.endpoint().socket_addr().port()
        )
        .into_client_request()
        .map_err(|error| WssClientError::WebSocketHandshake(error.to_string()))?;
        let (mut websocket, _) = client_with_config(request, tls, Some(websocket_config()))
            .map_err(|error| WssClientError::WebSocketHandshake(error.to_string()))?;
        websocket
            .get_mut()
            .sock
            .clear_deadline()
            .map_err(WssClientError::Io)?;
        websocket
            .get_mut()
            .sock
            .set_control_poll_timeout()
            .map_err(WssClientError::Io)?;
        let text = String::from_utf8(frame)
            .map_err(|_| WssClientError::Protocol(ProtocolError::InvalidUtf8))?;
        websocket
            .send(Message::text(text))
            .map_err(WssClientError::WebSocket)?;

        Ok(PendingWssPairingClient {
            socket: Some(Box::new(websocket)),
            receiver_id: invitation.device_id(),
            peer_fingerprint: invitation.fingerprint(),
            requested_grants,
            decision_deadline,
        })
    }
}

/// A sender connection waiting for an explicit TV/receiver approval or rejection.
pub struct PendingWssPairingClient {
    socket: Option<Box<dyn WssSocket>>,
    receiver_id: DeviceId,
    peer_fingerprint: PeerFingerprint,
    requested_grants: TrustedPeerGrants,
    decision_deadline: Instant,
}

impl PendingWssPairingClient {
    /// Waits for one authenticated `pair.approve` or `pair.reject` response.
    pub fn wait_for_decision(mut self) -> Result<WssClientApprovedPairing, WssClientError> {
        loop {
            if let Some(approved) = self.try_wait_for_decision()? {
                return Ok(approved);
            }
        }
    }

    /// Polls for an authenticated `pair.approve` or `pair.reject` response.
    ///
    /// Returns `Ok(None)` when no complete decision is available during the bounded socket
    /// read interval. The pending connection remains owned by this value and can be polled
    /// again or explicitly cancelled.
    pub fn try_wait_for_decision(
        &mut self,
    ) -> Result<Option<WssClientApprovedPairing>, WssClientError> {
        if self.socket.is_none() {
            return Err(WssClientError::Closed);
        }
        if Instant::now() >= self.decision_deadline {
            self.cancel();
            return Err(WssClientError::ApprovalTimedOut);
        }
        let socket = self.socket.as_mut().ok_or(WssClientError::Closed)?;
        let frame = match socket.read_message() {
            Ok(Some(Message::Text(text))) => text,
            Ok(Some(Message::Ping(_))) => {
                socket.flush_socket().map_err(WssClientError::WebSocket)?;
                return Ok(None);
            }
            Ok(Some(Message::Pong(_))) | Ok(None) => return Ok(None),
            Ok(Some(Message::Binary(_))) => {
                return Err(WssClientError::UnexpectedBinaryMessage);
            }
            Ok(Some(Message::Close(_))) => return Err(WssClientError::Closed),
            Ok(Some(Message::Frame(_))) => {
                unreachable!("tungstenite does not expose raw frames")
            }
            Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => {
                return Err(WssClientError::Closed);
            }
            Err(WebSocketError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::BrokenPipe
                ) =>
            {
                return Err(WssClientError::Closed);
            }
            Err(error) => return Err(WssClientError::WebSocket(error)),
        };
        let message = parse_message(frame.as_str().as_bytes()).map_err(WssClientError::Protocol)?;
        if message.header.sequence != 1 {
            return Err(WssClientError::Protocol(ProtocolError::InvalidSequence));
        }
        match message.payload {
            mm_protocol::MessagePayload::PairApprove(approval) => {
                let accepted_grants = validate_pair_approval(
                    &approval,
                    self.peer_fingerprint,
                    self.requested_grants,
                )?;
                let socket = self.socket.take().ok_or(WssClientError::Closed)?;
                let connection = PairingConnection::new_client(self.receiver_id, socket);
                Ok(Some(WssClientApprovedPairing {
                    accepted_grants,
                    peer_fingerprint: self.peer_fingerprint,
                    connection,
                }))
            }
            mm_protocol::MessagePayload::PairReject(rejection) => {
                drop(self.socket.take());
                Err(WssClientError::Rejected(rejection.reason))
            }
            _ => Err(WssClientError::UnexpectedMessageKind),
        }
    }

    /// Closes the pending pairing socket and makes this value terminal.
    ///
    /// Calling this more than once is safe. A subsequent poll returns `Closed`.
    pub fn cancel(&mut self) {
        drop(self.socket.take());
    }
}

/// A successfully approved sender-side connection and the grant subset selected by the receiver.
pub struct WssClientApprovedPairing {
    pub accepted_grants: TrustedPeerGrants,
    pub peer_fingerprint: PeerFingerprint,
    pub connection: PairingConnection,
}

/// Errors from QR-pinned sender-side pairing and decision handling.
#[derive(Debug)]
pub enum WssClientError {
    ApprovalTimedOut,
    PairingQr(PairingQrError),
    Proof(PairingProofSigningError),
    Tls(TlsAdapterError),
    TlsSetup(rustls::Error),
    Io(io::Error),
    WebSocket(WebSocketError),
    WebSocketHandshake(String),
    Protocol(ProtocolError),
    PeerIdentityMismatch,
    GrantsNotSubset,
    UnexpectedMessageKind,
    UnexpectedBinaryMessage,
    RandomnessUnavailable,
    Rejected(String),
    Closed,
}

impl fmt::Display for WssClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApprovalTimedOut => {
                formatter.write_str("receiver approval was not received before invitation expiry")
            }
            Self::PairingQr(error) => write!(formatter, "pairing QR is invalid: {error:?}"),
            Self::Proof(error) => write!(formatter, "sender proof could not be created: {error}"),
            Self::Tls(error) => write!(formatter, "pinned TLS setup failed: {error}"),
            Self::TlsSetup(error) => write!(formatter, "pinned TLS connection failed: {error}"),
            Self::Io(error) => write!(formatter, "WSS client socket failed: {error}"),
            Self::WebSocket(error) => write!(formatter, "WSS client protocol failed: {error}"),
            Self::WebSocketHandshake(error) => {
                write!(formatter, "WSS client handshake failed: {error}")
            }
            Self::Protocol(error) => write!(formatter, "pairing response is invalid: {error}"),
            Self::PeerIdentityMismatch => {
                formatter.write_str("pairing approval identity does not match the QR pin")
            }
            Self::GrantsNotSubset => {
                formatter.write_str("receiver approved grants that the sender did not request")
            }
            Self::UnexpectedMessageKind => {
                formatter.write_str("receiver sent an unexpected pairing response")
            }
            Self::UnexpectedBinaryMessage => {
                formatter.write_str("binary WebSocket messages are not pairing responses")
            }
            Self::RandomnessUnavailable => {
                formatter.write_str("operating-system randomness is unavailable")
            }
            Self::Rejected(reason) => write!(formatter, "receiver rejected pairing: {reason}"),
            Self::Closed => formatter.write_str("receiver closed the pairing connection"),
        }
    }
}

impl std::error::Error for WssClientError {}

fn grants_to_wire(grants: TrustedPeerGrants) -> Vec<String> {
    [
        (PeerGrant::View, "view"),
        (PeerGrant::Send, "send"),
        (PeerGrant::Annotate, "annotate"),
    ]
    .into_iter()
    .filter(|(grant, _)| grants.allows(*grant))
    .map(|(_, name)| name.to_owned())
    .collect()
}

fn grants_from_wire(grants: &[String]) -> Result<TrustedPeerGrants, WssClientError> {
    TrustedPeerGrants::new(
        grants.iter().any(|grant| grant == "view"),
        grants.iter().any(|grant| grant == "send"),
        grants.iter().any(|grant| grant == "annotate"),
    )
    .map_err(|_| WssClientError::Protocol(ProtocolError::InvalidPayload))
}

fn validate_pair_approval(
    approval: &PairApprove,
    expected_fingerprint: PeerFingerprint,
    requested_grants: TrustedPeerGrants,
) -> Result<TrustedPeerGrants, WssClientError> {
    let peer_fingerprint = PeerFingerprint::from_hex(&approval.peer_identity_proof)
        .map_err(|_| WssClientError::PeerIdentityMismatch)?;
    if peer_fingerprint != expected_fingerprint {
        return Err(WssClientError::PeerIdentityMismatch);
    }
    let accepted_grants = grants_from_wire(&approval.accepted_grants)?;
    if !accepted_grants.is_subset_of(requested_grants) {
        return Err(WssClientError::GrantsNotSubset);
    }
    Ok(accepted_grants)
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
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

    fn set_control_poll_timeout(&self) -> io::Result<()> {
        self.socket
            .set_read_timeout(Some(CONTROL_READ_POLL_INTERVAL))
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

/// A cancellable listener candidate for native TLS 1.3 raw-public-key WSS pairing.
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
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
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
        self.accept_pairing_request_inner(runtime, now_monotonic_ms, None)
    }

    /// Accepts and verifies one request, returning promptly if the caller cancels while idle.
    pub fn accept_pairing_request_cancellable<P, I>(
        &self,
        runtime: &mut PairingRuntime<P, I>,
        now_monotonic_ms: u64,
        cancellation: &AtomicBool,
    ) -> Result<PendingPairingConnection, WssListenerError>
    where
        P: TrustedPeerPort,
        I: LocalPairingIdentityPort + PairingIdentityPort,
    {
        self.accept_pairing_request_inner(runtime, now_monotonic_ms, Some(cancellation))
    }

    fn accept_pairing_request_inner<P, I>(
        &self,
        runtime: &mut PairingRuntime<P, I>,
        now_monotonic_ms: u64,
        cancellation: Option<&AtomicBool>,
    ) -> Result<PendingPairingConnection, WssListenerError>
    where
        P: TrustedPeerPort,
        I: LocalPairingIdentityPort + PairingIdentityPort,
    {
        let slot = PendingConnectionSlot::try_acquire(Arc::clone(&self.active_pending_connections))
            .ok_or(WssListenerError::PendingCapacityLimited)?;
        let (socket, _) = loop {
            match self.listener.accept() {
                Ok(accepted) => break accepted,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if cancellation.is_some_and(|token| token.load(Ordering::Acquire)) {
                        return Err(WssListenerError::Cancelled);
                    }
                    std::thread::sleep(LISTENER_ACCEPT_POLL_INTERVAL);
                }
                Err(error) => return Err(WssListenerError::Io(error)),
            }
        };
        socket
            .set_nonblocking(false)
            .map_err(WssListenerError::Io)?;
        if cancellation.is_some_and(|token| token.load(Ordering::Acquire)) {
            return Err(WssListenerError::Cancelled);
        }
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
        if cancellation.is_some_and(|token| token.load(Ordering::Acquire)) {
            return Err(WssListenerError::Cancelled);
        }
        websocket
            .get_mut()
            .sock
            .clear_deadline()
            .map_err(WssListenerError::Io)?;
        websocket
            .get_mut()
            .sock
            .set_control_poll_timeout()
            .map_err(WssListenerError::Io)?;
        let peer_id = admitted.approval().peer_id();

        Ok(PendingPairingConnection {
            admitted,
            connection: PairingConnection::new(peer_id, websocket, slot),
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
    Cancelled,
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
            Self::Cancelled => formatter.write_str("WSS pairing listener was stopped"),
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
