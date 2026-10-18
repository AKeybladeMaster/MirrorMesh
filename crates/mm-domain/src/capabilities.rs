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

#[cfg(test)]
mod tests {
    use super::CapabilitySet;

    #[test]
    fn negotiation_clamps_to_both_devices() {
        let local = CapabilitySet::new(1920, 1080, 60, true, true).unwrap();
        let remote = CapabilitySet::new(1280, 720, 30, true, false).unwrap();
        let negotiated = local.negotiate(remote).unwrap();
        assert_eq!(
            (
                negotiated.max_width,
                negotiated.max_height,
                negotiated.max_fps
            ),
            (1280, 720, 30)
        );
        assert!(!negotiated.opus);
    }

    #[test]
    fn refuses_video_when_h264_is_unavailable() {
        let local = CapabilitySet::new(1920, 1080, 60, true, false).unwrap();
        let remote = CapabilitySet::new(1920, 1080, 60, false, false).unwrap();
        assert!(local.negotiate(remote).is_err());
    }
}
