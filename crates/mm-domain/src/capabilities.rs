use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u16,
    pub h264: bool,
    pub opus: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiatedCapabilities {
    pub codec: VideoCodec,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u16,
    pub opus: bool,
}

impl CapabilitySet {
    pub fn new(
        max_width: u32,
        max_height: u32,
        max_fps: u16,
        h264: bool,
        opus: bool,
    ) -> Result<Self, DomainError> {
        if !(1..=7_680).contains(&max_width)
            || !(1..=4_320).contains(&max_height)
            || !(1..=120).contains(&max_fps)
        {
            return Err(DomainError::InvalidCapabilities);
        }
        Ok(Self {
            max_width,
            max_height,
            max_fps,
            h264,
            opus,
        })
    }

    pub fn negotiate(self, peer: Self) -> Result<NegotiatedCapabilities, DomainError> {
        if !self.h264 || !peer.h264 {
            return Err(DomainError::InvalidCapabilities);
        }
        Ok(NegotiatedCapabilities {
            codec: VideoCodec::H264,
            max_width: self.max_width.min(peer.max_width),
            max_height: self.max_height.min(peer.max_height),
            max_fps: self.max_fps.min(peer.max_fps),
            opus: self.opus && peer.opus,
        })
    }
}
