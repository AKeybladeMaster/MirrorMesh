use std::fmt;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use mm_domain::{
    CaptureConfig, Device, DeviceId, NegotiatedCapabilities, PeerFingerprint, PeerPublicKey,
    SessionId, TrustedPeer, VideoCodec,
};
use mm_protocol::Envelope;
use mm_transport::{IceCandidate, RtcConnectionState, SdpType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortError {
    Unavailable,
    PermissionDenied,
    InvalidConfiguration,
    StorageFailure,
    TransportFailure,
    IdentityRejected,
    Cancelled,
}

impl fmt::Display for PortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PortError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePermission {
    NotRequested,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaptureHandle(u64);

impl CaptureHandle {
    pub const fn from_adapter(value: u64) -> Self {
        Self(value)
    }

    pub const fn adapter_value(self) -> u64 {
        self.0
    }
}

pub trait ClockPort: Send + Sync {
    fn monotonic_millis(&self) -> u64;
}

pub trait DiscoveryPort: Send + Sync {
    fn discover(&self) -> Result<Vec<Device>, PortError>;
    fn stop(&self) -> Result<(), PortError>;
}

/// A single unauthenticated device hint and the resolved network addresses associated with it.
///
/// The device identifier, metadata, and endpoints are attacker-controlled until pairing proves
/// the peer identity. This value must never imply trust or authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryCandidate {
    pub device: Device,
    pub endpoints: Vec<SocketAddr>,
}

/// Endpoint-preserving discovery extension for use cases that need to initiate pairing.
pub trait DiscoveryCandidatesPort: DiscoveryPort {
    fn discover_candidates(&self) -> Result<Vec<DiscoveryCandidate>, PortError>;
}

pub trait CapturePort: Send {
    fn request_permission(&mut self) -> Result<CapturePermission, PortError>;
    fn start(&mut self, config: &CaptureConfig) -> Result<CaptureHandle, PortError>;
    fn stop(&mut self, handle: CaptureHandle) -> Result<(), PortError>;
}

pub trait CodecPort: Send {
    fn available_video_codecs(&self) -> Result<Vec<VideoCodec>, PortError>;
    fn configure(&mut self, capabilities: NegotiatedCapabilities) -> Result<(), PortError>;
    fn stop(&mut self) -> Result<(), PortError>;
}

pub trait TransportPort: Send {
    fn start(&mut self, session_id: SessionId, peer_id: DeviceId) -> Result<(), PortError>;
    fn send_control(&mut self, message: &Envelope<Vec<u8>>) -> Result<(), PortError>;
    fn close(&mut self, session_id: SessionId) -> Result<(), PortError>;
}

pub type WebRtcFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, PortError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebRtcEngineEvent {
    LocalIceCandidate(IceCandidate),
    ConnectionStateChanged(RtcConnectionState),
    RemoteVideoTrackReady { track_id: String },
}

/// Control and metadata boundary for a native/browser WebRTC engine.
/// Video pixels remain in the engine's native renderer and never enter Dart FFI events.
pub trait WebRtcEngineEventSink: Send + Sync {
    fn on_event(&self, event: WebRtcEngineEvent);
}

/// Async WebRTC operations shared by platform adapters.
/// Adapters must use verified DTLS-SRTP, emit bounded trickle ICE metadata, and close all
/// native tracks/renderers before returning from `close`.
pub trait WebRtcEnginePort: Send + Sync {
    fn set_event_sink(&self, sink: Arc<dyn WebRtcEngineEventSink>) -> Result<(), PortError>;

    fn create_offer(&self) -> WebRtcFuture<'_, String>;
    fn create_answer(&self) -> WebRtcFuture<'_, String>;
    fn set_local_description(&self, kind: SdpType, sdp: String) -> WebRtcFuture<'_, ()>;
    fn set_remote_description(&self, kind: SdpType, sdp: String) -> WebRtcFuture<'_, ()>;
    fn add_remote_ice_candidate(&self, candidate: IceCandidate) -> WebRtcFuture<'_, ()>;
    fn connection_state(&self) -> RtcConnectionState;
    fn close(&self) -> WebRtcFuture<'_, ()>;
}

pub trait PreferencesPort: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, PortError>;
    fn set(&self, key: &str, value: &str) -> Result<(), PortError>;
    fn remove(&self, key: &str) -> Result<(), PortError>;
}

pub trait TrustedPeerPort: Send + Sync {
    fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError>;

    /// Persist a public peer record only after explicit user approval.
    fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError>;

    /// Include revoked records so the UI can show and manage prior trust.
    fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError>;

    /// Record UTC Unix milliseconds and retain the peer row for audit/review.
    fn revoke(&self, peer_id: DeviceId, revoked_at_unix_ms: u64) -> Result<(), PortError>;

    fn is_trusted(&self, peer_id: DeviceId) -> Result<bool, PortError> {
        self.get_peer(peer_id)
            .map(|peer| peer.is_some_and(|peer| peer.is_active()))
    }
}

/// Local signing and lifecycle boundary for a device's pairing identity.
///
/// Implementations must keep private material inside the platform identity adapter,
/// refuse to sign non-domain-separated pairing messages, and never export the private seed.
pub trait LocalPairingIdentityPort: Send + Sync {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError>;

    /// Signs the canonical domain-separated message produced by the pairing application.
    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError>;

    /// Rotates the local key only after the replacement is durably protected by the OS.
    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError>;

    /// Removes the persisted key and disables signing through this loaded identity.
    fn delete_identity(&self) -> Result<(), PortError>;
}

/// TLS 1.3 server-authentication signing boundary for a local device identity.
///
/// Implementations keep private material inside the platform identity adapter and must never
/// export a private key. They must reject inputs that do not contain the TLS 1.3 server
/// CertificateVerify context and transcript hash; this operation must not sign pairing proofs or
/// arbitrary application messages. The public key must be the same Ed25519 key whose SHA-256
/// fingerprint is placed in the pairing QR.
pub trait LocalTlsIdentityPort: Send + Sync {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError>;

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError>;
}

/// Cryptographic identity boundary for the first-time pairing proof.
///
/// Implementations must compute SHA-256 over the agreed canonical public-key
/// encoding and verify the signature over the coordinator's domain-separated
/// pairing message. No software fallback or accept-all verifier is provided.
pub trait PairingIdentityPort: Send + Sync {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError>;

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError>;
}
