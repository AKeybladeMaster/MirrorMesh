use mm_domain::{DeviceId, PeerFingerprint, SessionId};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

mod attempt_limiter;
mod compact_invitation;

pub use compact_invitation::format_pairing_qr;

pub use attempt_limiter::{
    DEFAULT_MAX_TRACKED_PAIRING_SOURCES, DEFAULT_PAIRING_ATTEMPT_LIMIT,
    DEFAULT_PAIRING_ATTEMPT_WINDOW_MS, DEFAULT_PAIRING_COOLDOWN_MS, PairingAttemptDecision,
    PairingAttemptLimiter, PairingAttemptLimiterError, PairingAttemptPolicy,
    PairingAttemptPolicyError,
};

pub const MAX_MANUAL_ENDPOINT_BYTES: usize = 512;
pub const MAX_PAIRING_QR_BYTES: usize = 1024;
pub const PAIRING_QR_TTL_SECONDS: u64 = 120;
pub const PAIRING_NONCE_TTL_MS: u64 = 120_000;
pub const MAX_PENDING_PAIRING_NONCES: usize = 1024;
const MAX_NONCE_GENERATION_ATTEMPTS: usize = 8;

/// A 128-bit QR-pairing nonce represented on the wire as 32 lowercase hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PairingNonce([u8; 16]);

impl PairingNonce {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(32);
        for byte in self.0 {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        encoded
    }

    pub fn from_hex(encoded: &str) -> Result<Self, NonceParseError> {
        if encoded.len() != 32 {
            return Err(NonceParseError::InvalidLength);
        }

        let mut bytes = [0_u8; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            let high =
                decode_lower_hex(encoded.as_bytes()[offset]).ok_or(NonceParseError::InvalidHex)?;
            let low = decode_lower_hex(encoded.as_bytes()[offset + 1])
                .ok_or(NonceParseError::InvalidHex)?;
            *byte = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceParseError {
    InvalidLength,
    InvalidHex,
}

/// Error returned when the operating-system random source cannot fill a nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonceSourceError;

/// Injectable source boundary for deterministic lifecycle tests.
pub trait PairingNonceSource: Send {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), NonceSourceError>;
}

/// Uses the platform's operating-system cryptographic random source.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemPairingNonceSource;

impl PairingNonceSource for SystemPairingNonceSource {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), NonceSourceError> {
        getrandom::fill(output).map_err(|_| NonceSourceError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingNonceTicket {
    pub nonce: PairingNonce,
    /// Deadline in the issuing process's monotonic millisecond clock domain.
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceIssueError {
    RandomSourceUnavailable,
    RegistryFull,
    ExpiryOverflow,
    CollisionLimitExceeded,
}

/// Bounded, process-local single-use nonce registry.
///
/// Callers provide monotonic milliseconds from one local clock domain. The
/// separate wall-clock expiry placed in a QR payload must not be compared to
/// this local deadline.
pub struct PendingPairingNonces {
    pending: HashMap<PairingNonce, u64>,
    source: Box<dyn PairingNonceSource>,
}

impl Default for PendingPairingNonces {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingPairingNonces {
    pub fn new() -> Self {
        Self::with_source(SystemPairingNonceSource)
    }

    pub fn with_source(source: impl PairingNonceSource + 'static) -> Self {
        Self {
            pending: HashMap::new(),
            source: Box::new(source),
        }
    }

    pub fn issue(&mut self, now_monotonic_ms: u64) -> Result<PairingNonceTicket, NonceIssueError> {
        let expires_at_ms = now_monotonic_ms
            .checked_add(PAIRING_NONCE_TTL_MS)
            .ok_or(NonceIssueError::ExpiryOverflow)?;

        self.prune_expired(now_monotonic_ms);
        if self.pending.len() >= MAX_PENDING_PAIRING_NONCES {
            return Err(NonceIssueError::RegistryFull);
        }

        for _ in 0..MAX_NONCE_GENERATION_ATTEMPTS {
            let mut bytes = [0_u8; 16];
            self.source
                .fill_nonce(&mut bytes)
                .map_err(|_| NonceIssueError::RandomSourceUnavailable)?;
            let nonce = PairingNonce::from_bytes(bytes);
            if let std::collections::hash_map::Entry::Vacant(entry) = self.pending.entry(nonce) {
                entry.insert(expires_at_ms);
                return Ok(PairingNonceTicket {
                    nonce,
                    expires_at_ms,
                });
            }
        }

        Err(NonceIssueError::CollisionLimitExceeded)
    }

    /// Atomically removes a nonce and returns true only for a live first use.
    /// Unknown, expired, and already-consumed values share the same result.
    pub fn consume(&mut self, nonce: PairingNonce, now_monotonic_ms: u64) -> bool {
        self.consume_ticket(nonce, now_monotonic_ms).is_some()
    }

    /// Atomically removes a nonce and returns its original local deadline once.
    pub fn consume_ticket(
        &mut self,
        nonce: PairingNonce,
        now_monotonic_ms: u64,
    ) -> Option<PairingNonceTicket> {
        self.prune_expired(now_monotonic_ms);
        self.pending
            .remove(&nonce)
            .filter(|expires_at_ms| now_monotonic_ms < *expires_at_ms)
            .map(|expires_at_ms| PairingNonceTicket {
                nonce,
                expires_at_ms,
            })
    }

    /// Check whether a nonce is live without consuming it. The caller must
    /// still consume it after proof verification while holding exclusive access.
    pub fn is_pending(&mut self, nonce: PairingNonce, now_monotonic_ms: u64) -> bool {
        self.prune_expired(now_monotonic_ms);
        self.pending.contains_key(&nonce)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    fn prune_expired(&mut self, now_monotonic_ms: u64) {
        self.pending
            .retain(|_, expires_at_ms| now_monotonic_ms < *expires_at_ms);
    }
}

/// A numeric IP endpoint parsed from a user-entered address.
///
/// Hostnames are deliberately excluded here. A future DNS path must pin one
/// resolution result for the connection and still verify the peer identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualIpEndpoint {
    socket_addr: SocketAddr,
}

impl ManualIpEndpoint {
    pub const fn socket_addr(self) -> SocketAddr {
        self.socket_addr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualEndpointError {
    EmptyInput,
    TooLong,
    PrivateKeyMaterial,
    InvalidAuthority,
    InvalidIpLiteral,
    Ipv6MustBeBracketed,
    InvalidPort,
    PortNotApproved,
}

impl fmt::Display for ManualEndpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyInput => "manual endpoint is empty",
            Self::TooLong => "manual endpoint exceeds the input limit",
            Self::PrivateKeyMaterial => "private key material is not an endpoint",
            Self::InvalidAuthority => "manual endpoint must contain only an IP address and port",
            Self::InvalidIpLiteral => "manual endpoint contains an invalid IP address",
            Self::Ipv6MustBeBracketed => "IPv6 addresses must be enclosed in brackets",
            Self::InvalidPort => "manual endpoint port is invalid",
            Self::PortNotApproved => "manual endpoint port has not been approved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ManualEndpointError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingQrError {
    EmptyInput,
    TooLong,
    InvalidUri,
    InvalidQuery,
    DuplicateField,
    UnknownField,
    MissingField,
    UnsupportedVersion,
    InvalidEndpoint(ManualEndpointError),
    InvalidDeviceId,
    InvalidFingerprint,
    InvalidNonce,
    InvalidExpiry,
    Expired,
    ExpiryTooFar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingQrPayload {
    endpoint: ManualIpEndpoint,
    device_id: DeviceId,
    fingerprint: PeerFingerprint,
    nonce: PairingNonce,
    expires_at_unix_seconds: u64,
}

impl PairingQrPayload {
    pub const fn endpoint(self) -> ManualIpEndpoint {
        self.endpoint
    }

    pub const fn device_id(self) -> DeviceId {
        self.device_id
    }

    pub const fn fingerprint(self) -> PeerFingerprint {
        self.fingerprint
    }

    pub const fn nonce(self) -> PairingNonce {
        self.nonce
    }

    pub const fn expires_at_unix_seconds(self) -> u64 {
        self.expires_at_unix_seconds
    }
}

/// Format the exact v1 QR URI after the endpoint, identity, nonce, and wall-clock
/// expiration have been approved by the caller.
pub fn format_pairing_qr_v1(
    endpoint: ManualIpEndpoint,
    device_id: DeviceId,
    fingerprint: PeerFingerprint,
    nonce: PairingNonce,
    expires_at_unix_seconds: u64,
) -> String {
    let socket_addr = endpoint.socket_addr();
    let host = match socket_addr.ip() {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };
    format!(
        "mirrormesh://pair?v=1&host={host}&port={}&device={device_id}&fp={}&nonce={}&expires={expires_at_unix_seconds}",
        socket_addr.port(),
        fingerprint.to_hex(),
        nonce.to_hex()
    )
}

/// Parse the strict QR pairing URI without resolving names or opening sockets.
///
/// The caller supplies the current UTC Unix time and the ports explicitly
/// approved by the user. Nonce replay and local monotonic expiry are enforced
/// separately by `PendingPairingNonces`.
pub fn parse_pairing_qr(
    input: &str,
    now_unix_seconds: u64,
    approved_ports: &[u16],
) -> Result<PairingQrPayload, PairingQrError> {
    if input.is_empty() {
        return Err(PairingQrError::EmptyInput);
    }
    if input.len() > MAX_PAIRING_QR_BYTES {
        return Err(PairingQrError::TooLong);
    }
    if let Some(encoded) = input.strip_prefix("mirrormesh://p/") {
        return compact_invitation::parse(encoded, now_unix_seconds, approved_ports);
    }

    let query = input
        .strip_prefix("mirrormesh://pair?")
        .ok_or(PairingQrError::InvalidUri)?;
    if query.is_empty()
        || query.contains('#')
        || query.contains('?')
        || input
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(PairingQrError::InvalidQuery);
    }

    let mut fields = BTreeMap::new();
    for field in query.split('&') {
        let (name, value) = field.split_once('=').ok_or(PairingQrError::InvalidQuery)?;
        if name.is_empty() || value.is_empty() || value.contains('=') {
            return Err(PairingQrError::InvalidQuery);
        }
        if !matches!(
            name,
            "v" | "host" | "port" | "device" | "fp" | "nonce" | "expires"
        ) {
            return Err(PairingQrError::UnknownField);
        }
        if fields.insert(name, value).is_some() {
            return Err(PairingQrError::DuplicateField);
        }
    }

    if fields.len() != 7 {
        return Err(PairingQrError::MissingField);
    }
    if fields.get("v") != Some(&"1") {
        return Err(PairingQrError::UnsupportedVersion);
    }

    let host = fields.get("host").ok_or(PairingQrError::MissingField)?;
    let port = fields.get("port").ok_or(PairingQrError::MissingField)?;
    let endpoint_text = format!("{host}:{port}");
    let endpoint = parse_manual_ip_endpoint(&endpoint_text, approved_ports)
        .map_err(PairingQrError::InvalidEndpoint)?;

    let device_text = fields.get("device").ok_or(PairingQrError::MissingField)?;
    let device_id = DeviceId::parse(device_text).map_err(|_| PairingQrError::InvalidDeviceId)?;
    if device_id.to_string() != *device_text {
        return Err(PairingQrError::InvalidDeviceId);
    }
    let fingerprint =
        PeerFingerprint::from_hex(fields.get("fp").ok_or(PairingQrError::MissingField)?)
            .map_err(|_| PairingQrError::InvalidFingerprint)?;
    let nonce = PairingNonce::from_hex(fields.get("nonce").ok_or(PairingQrError::MissingField)?)
        .map_err(|_| PairingQrError::InvalidNonce)?;
    let expires_at_unix_seconds =
        parse_canonical_u64(fields.get("expires").ok_or(PairingQrError::MissingField)?)
            .ok_or(PairingQrError::InvalidExpiry)?;
    if expires_at_unix_seconds <= now_unix_seconds {
        return Err(PairingQrError::Expired);
    }
    if expires_at_unix_seconds - now_unix_seconds > PAIRING_QR_TTL_SECONDS {
        return Err(PairingQrError::ExpiryTooFar);
    }

    Ok(PairingQrPayload {
        endpoint,
        device_id,
        fingerprint,
        nonce,
        expires_at_unix_seconds,
    })
}

fn parse_canonical_u64(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

/// Parse a numeric IPv4/IPv6 `address:port` after the caller has collected
/// explicit approval for the listed ports. This function never resolves DNS.
pub fn parse_manual_ip_endpoint(
    input: &str,
    approved_ports: &[u16],
) -> Result<ManualIpEndpoint, ManualEndpointError> {
    if input.is_empty() {
        return Err(ManualEndpointError::EmptyInput);
    }
    if input.len() > MAX_MANUAL_ENDPOINT_BYTES {
        return Err(ManualEndpointError::TooLong);
    }

    let uppercase = input.to_ascii_uppercase();
    if uppercase.contains("PRIVATE KEY")
        || uppercase.contains("-----BEGIN")
        || uppercase.contains("-----END")
    {
        return Err(ManualEndpointError::PrivateKeyMaterial);
    }
    if input
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
        || input.chars().any(|character| "/\\@?#".contains(character))
        || input.contains("://")
    {
        return Err(ManualEndpointError::InvalidAuthority);
    }

    let (ip, port_text) = if let Some(bracketed) = input.strip_prefix('[') {
        let (host_text, suffix) = bracketed
            .split_once(']')
            .ok_or(ManualEndpointError::InvalidAuthority)?;
        let port_text = suffix
            .strip_prefix(':')
            .ok_or(ManualEndpointError::InvalidAuthority)?;
        let ip = host_text
            .parse::<Ipv6Addr>()
            .map_err(|_| ManualEndpointError::InvalidIpLiteral)?;
        (IpAddr::V6(ip), port_text)
    } else {
        if input.matches(':').count() > 1 {
            return Err(ManualEndpointError::Ipv6MustBeBracketed);
        }
        let (host_text, port_text) = input
            .split_once(':')
            .ok_or(ManualEndpointError::InvalidAuthority)?;
        let ip = host_text
            .parse::<Ipv4Addr>()
            .map_err(|_| ManualEndpointError::InvalidIpLiteral)?;
        (IpAddr::V4(ip), port_text)
    };

    if port_text.is_empty()
        || (port_text.len() > 1 && port_text.starts_with('0'))
        || !port_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ManualEndpointError::InvalidPort);
    }
    let port = port_text
        .parse::<u16>()
        .map_err(|_| ManualEndpointError::InvalidPort)?;
    if port == 0 {
        return Err(ManualEndpointError::InvalidPort);
    }
    if !approved_ports.contains(&port) {
        return Err(ManualEndpointError::PortNotApproved);
    }

    Ok(ManualIpEndpoint {
        socket_addr: SocketAddr::new(ip, port),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantScope {
    View,
    Send,
    Annotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentGrant {
    pub peer_id: DeviceId,
    pub session_id: SessionId,
    pub view: bool,
    pub send: bool,
    pub annotate: bool,
    pub expires_at_ms: u64,
}

impl ConsentGrant {
    pub fn allows(&self, scope: GrantScope, now_ms: u64) -> bool {
        if now_ms >= self.expires_at_ms {
            return false;
        }
        match scope {
            GrantScope::View => self.view,
            GrantScope::Send => self.send,
            GrantScope::Annotate => self.annotate,
        }
    }
}

/// Cryptographic verification is supplied by the selected TLS/identity adapter.
/// This crate intentionally contains no custom cryptographic implementation.
pub trait PeerIdentityVerifier: Send + Sync {
    fn verify_fingerprint(
        &self,
        expected_fingerprint: &[u8],
        presented_fingerprint: &[u8],
    ) -> Result<(), IdentityError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    FingerprintMismatch,
    InvalidFingerprint,
}
