mod capabilities;
mod device;
mod identifier;
mod session;
mod trusted_peer;

pub use capabilities::{CapabilitySet, NegotiatedCapabilities, VideoCodec};
pub use device::{Device, DeviceRole};
pub use identifier::{DeviceId, MessageId, SessionId};
pub use session::{SessionEvent, SessionState, try_transition};
pub use trusted_peer::{
    MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, MAX_TRUSTED_PEER_PUBLIC_KEY_BYTES, PeerFingerprint,
    PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
};

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainError {
    InvalidIdentifier,
    InvalidDisplayName,
    InvalidCapabilities,
    InvalidCaptureConfig,
    InvalidTrustedPeer,
    AuthorizationDenied,
    IllegalTransition,
    TrustedPeerRevoked,
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidIdentifier => "identifier is not a canonical UUID",
            Self::InvalidDisplayName => "display name is empty, too long, or contains controls",
            Self::InvalidCapabilities => "capability values are outside supported bounds",
            Self::InvalidCaptureConfig => "capture configuration is outside supported bounds",
            Self::InvalidTrustedPeer => "trusted peer data violates its validation rules",
            Self::AuthorizationDenied => {
                "required identity, approval, or capture consent is missing"
            }
            Self::IllegalTransition => "session event is not valid in the current state",
            Self::TrustedPeerRevoked => "trusted peer has been revoked",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DomainError {}

/// A user-selected source. Platform identifiers remain opaque to the shared domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureSource {
    Display(u32),
    Window(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    pub source: CaptureSource,
    pub max_width: u32,
    pub max_height: u32,
    pub target_fps: u16,
    pub video_kbps: u32,
    pub include_audio: bool,
}

impl CaptureConfig {
    pub fn new(
        source: CaptureSource,
        max_width: u32,
        max_height: u32,
        target_fps: u16,
        video_kbps: u32,
        include_audio: bool,
    ) -> Result<Self, DomainError> {
        let source_is_valid = match &source {
            CaptureSource::Display(_) => true,
            CaptureSource::Window(id) => !id.is_empty() && id.len() <= 128,
        };
        if !source_is_valid
            || !(1..=7_680).contains(&max_width)
            || !(1..=4_320).contains(&max_height)
            || !(1..=120).contains(&target_fps)
            || !(1..=200_000).contains(&video_kbps)
        {
            return Err(DomainError::InvalidCaptureConfig);
        }
        Ok(Self {
            source,
            max_width,
            max_height,
            target_fps,
            video_kbps,
            include_audio,
        })
    }
}
