use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

pub const MAX_SDP_BYTES: usize = 128 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 4 * 1024;
pub const DEFAULT_PENDING_CANDIDATE_COUNT: usize = 128;
pub const DEFAULT_PENDING_CANDIDATE_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_RECONNECT_ATTEMPTS: u8 = 3;
pub const DEFAULT_RECONNECT_DEADLINE_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdpType {
    Offer,
    Answer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcSessionState {
    Created,
    LocalDescriptionSet,
    RemoteDescriptionSet,
    Negotiating,
    Connected,
    Disconnected,
    Reconnecting,
    Failed,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    InvalidDescription,
    InvalidTransition,
    CandidateQueueFull,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDescription => "WebRTC session description is invalid",
            Self::InvalidTransition => "WebRTC session transition is invalid",
            Self::CandidateQueueFull => "pending ICE candidate queue is full",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SessionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IceCandidate {
    candidate: String,
    sdp_mid: String,
    mline_index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceCandidateError {
    EmptyCandidate,
    CandidateTooLarge,
    InvalidCharacter,
    InvalidMid,
    BufferFull,
    SessionUnavailable,
}

impl IceCandidate {
    pub fn new(
        candidate: impl Into<String>,
        sdp_mid: impl Into<String>,
        mline_index: u16,
    ) -> Result<Self, IceCandidateError> {
        let candidate = candidate.into();
        let sdp_mid = sdp_mid.into();
        if candidate.is_empty() {
            return Err(IceCandidateError::EmptyCandidate);
        }
        if candidate.len() > MAX_CANDIDATE_BYTES {
            return Err(IceCandidateError::CandidateTooLarge);
        }
        if !candidate.starts_with("candidate:") {
            return Err(IceCandidateError::InvalidCharacter);
        }
        if candidate.chars().any(char::is_control) {
            return Err(IceCandidateError::InvalidCharacter);
        }
        if sdp_mid.is_empty() || sdp_mid.len() > 128 || sdp_mid.chars().any(char::is_control) {
            return Err(IceCandidateError::InvalidMid);
        }
        Ok(Self {
            candidate,
            sdp_mid,
            mline_index,
        })
    }

    pub fn candidate(&self) -> &str {
        &self.candidate
    }

    pub fn sdp_mid(&self) -> &str {
        &self.sdp_mid
    }

    pub const fn mline_index(&self) -> u16 {
        self.mline_index
    }

    fn retained_bytes(&self) -> usize {
        self.candidate.len() + self.sdp_mid.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateDisposition {
    Buffered,
    ReadyToApply(IceCandidate),
}

#[derive(Debug)]
pub struct RtcSession {
    state: RtcSessionState,
    local_description_set: bool,
    remote_description_set: bool,
    pending_remote_candidates: VecDeque<IceCandidate>,
    pending_candidate_bytes: usize,
    max_pending_candidates: usize,
    max_pending_candidate_bytes: usize,
}

impl Default for RtcSession {
    fn default() -> Self {
        Self::new()
    }
}

impl RtcSession {
    pub fn new() -> Self {
        Self::with_candidate_limits(
            DEFAULT_PENDING_CANDIDATE_COUNT,
            DEFAULT_PENDING_CANDIDATE_BYTES,
        )
    }

    pub fn with_candidate_limits(max_candidates: usize, max_bytes: usize) -> Self {
        Self {
            state: RtcSessionState::Created,
            local_description_set: false,
            remote_description_set: false,
            pending_remote_candidates: VecDeque::new(),
            pending_candidate_bytes: 0,
            max_pending_candidates: max_candidates,
            max_pending_candidate_bytes: max_bytes,
        }
    }

    pub const fn state(&self) -> RtcSessionState {
        self.state
    }

    pub fn pending_remote_candidates(&self) -> usize {
        self.pending_remote_candidates.len()
    }

    pub fn set_local_description(&mut self, description: &str) -> Result<(), SessionError> {
        validate_description(description)?;
        if self.local_description_set
            || !matches!(
                self.state,
                RtcSessionState::Created | RtcSessionState::RemoteDescriptionSet
            )
        {
            return Err(SessionError::InvalidTransition);
        }
        self.local_description_set = true;
        self.state = if self.remote_description_set {
            RtcSessionState::Negotiating
        } else {
            RtcSessionState::LocalDescriptionSet
        };
        Ok(())
    }

    /// Sets the remote SDP and returns queued trickle ICE candidates in arrival order.
    pub fn set_remote_description(
        &mut self,
        description: &str,
    ) -> Result<Vec<IceCandidate>, SessionError> {
        validate_description(description)?;
        if self.remote_description_set
            || !matches!(
                self.state,
                RtcSessionState::Created | RtcSessionState::LocalDescriptionSet
            )
        {
            return Err(SessionError::InvalidTransition);
        }
        self.remote_description_set = true;
        self.state = if self.local_description_set {
            RtcSessionState::Negotiating
        } else {
            RtcSessionState::RemoteDescriptionSet
        };
        self.pending_candidate_bytes = 0;
        Ok(self.pending_remote_candidates.drain(..).collect())
    }

    pub fn add_remote_candidate(
        &mut self,
        candidate: IceCandidate,
    ) -> Result<CandidateDisposition, IceCandidateError> {
        if matches!(
            self.state,
            RtcSessionState::Closed | RtcSessionState::Failed
        ) {
            return Err(IceCandidateError::SessionUnavailable);
        }
        if self.remote_description_set {
            return Ok(CandidateDisposition::ReadyToApply(candidate));
        }
        let next_bytes = self
            .pending_candidate_bytes
            .checked_add(candidate.retained_bytes())
            .ok_or(IceCandidateError::BufferFull)?;
        if self.pending_remote_candidates.len() >= self.max_pending_candidates
            || next_bytes > self.max_pending_candidate_bytes
        {
            return Err(IceCandidateError::BufferFull);
        }
        self.pending_candidate_bytes = next_bytes;
        self.pending_remote_candidates.push_back(candidate);
        Ok(CandidateDisposition::Buffered)
    }

    pub fn mark_connected(&mut self) -> Result<(), SessionError> {
        if !matches!(
            self.state,
            RtcSessionState::Negotiating | RtcSessionState::Reconnecting
        ) {
            return Err(SessionError::InvalidTransition);
        }
        self.state = RtcSessionState::Connected;
        Ok(())
    }

    pub fn mark_disconnected(&mut self) -> Result<(), SessionError> {
        if self.state != RtcSessionState::Connected {
            return Err(SessionError::InvalidTransition);
        }
        self.state = RtcSessionState::Disconnected;
        Ok(())
    }

    pub fn begin_reconnect(&mut self) -> Result<(), SessionError> {
        if self.state != RtcSessionState::Disconnected {
            return Err(SessionError::InvalidTransition);
        }
        self.state = RtcSessionState::Reconnecting;
        Ok(())
    }

    /// Clears negotiation metadata for an ICE restart while retaining the same logical session.
    pub fn restart_negotiation(&mut self) -> Result<(), SessionError> {
        if self.state != RtcSessionState::Reconnecting {
            return Err(SessionError::InvalidTransition);
        }
        self.local_description_set = false;
        self.remote_description_set = false;
        self.pending_remote_candidates.clear();
        self.pending_candidate_bytes = 0;
        self.state = RtcSessionState::Created;
        Ok(())
    }

    pub fn mark_failed(&mut self) -> Result<(), SessionError> {
        if matches!(
            self.state,
            RtcSessionState::Closed | RtcSessionState::Failed
        ) {
            return Err(SessionError::InvalidTransition);
        }
        self.state = RtcSessionState::Failed;
        self.pending_remote_candidates.clear();
        self.pending_candidate_bytes = 0;
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = RtcSessionState::Closed;
        self.pending_remote_candidates.clear();
        self.pending_candidate_bytes = 0;
    }
}

fn validate_description(description: &str) -> Result<(), SessionError> {
    if description.is_empty()
        || description.len() > MAX_SDP_BYTES
        || description
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\r' | '\n'))
    {
        return Err(SessionError::InvalidDescription);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    initial_delay_ms: u64,
    max_delay_ms: u64,
    max_attempts: u8,
    deadline_ms: u64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 500,
            max_delay_ms: 4_000,
            max_attempts: DEFAULT_MAX_RECONNECT_ATTEMPTS,
            deadline_ms: DEFAULT_RECONNECT_DEADLINE_MS,
        }
    }
}

impl ReconnectPolicy {
    pub fn delay_for_attempt(self, attempt: u8) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }
        let multiplier = 1_u64.checked_shl(u32::from(attempt)).unwrap_or(u64::MAX);
        Some(Duration::from_millis(
            self.initial_delay_ms
                .saturating_mul(multiplier)
                .min(self.max_delay_ms),
        ))
    }

    pub fn within_deadline(self, elapsed_ms: u64) -> bool {
        elapsed_ms < self.deadline_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampError {
    CaptureTimeWentBackwards,
}

/// Converts monotonic nanoseconds to the 90 kHz clock used by video RTP.
#[derive(Debug, Default)]
pub struct RtpTimestampClock {
    origin_ns: Option<u64>,
    last_capture_ns: Option<u64>,
}

impl RtpTimestampClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timestamp_for_capture_ns(&mut self, capture_ns: u64) -> Result<u32, TimestampError> {
        if self.last_capture_ns.is_some_and(|last| capture_ns < last) {
            return Err(TimestampError::CaptureTimeWentBackwards);
        }
        let origin = *self.origin_ns.get_or_insert(capture_ns);
        self.last_capture_ns = Some(capture_ns);
        let elapsed_ns = u128::from(capture_ns - origin);
        let timestamp = elapsed_ns.saturating_mul(90_000) / 1_000_000_000;
        Ok(timestamp as u32)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H264Frame {
    timestamp_90khz: u32,
    keyframe: bool,
    bytes: Arc<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    EmptyAccessUnit,
    FrameTooLarge,
    InvalidQueueLimit,
}

impl H264Frame {
    /// Creates an encoded H.264 access unit. Pixel buffers are intentionally not accepted here.
    pub fn new(
        timestamp_90khz: u32,
        keyframe: bool,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self, FrameError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(FrameError::EmptyAccessUnit);
        }
        Ok(Self {
            timestamp_90khz,
            keyframe,
            bytes: Arc::from(bytes),
        })
    }

    pub const fn timestamp_90khz(&self) -> u32 {
        self.timestamp_90khz
    }

    pub const fn is_keyframe(&self) -> bool {
        self.keyframe
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePushOutcome {
    pub queued: bool,
    pub request_keyframe: bool,
    pub dropped_until_keyframe: bool,
}

/// A bounded queue for encoded access units. Overflow discards stale dependencies and
/// holds deltas until a fresh keyframe arrives, keeping memory and latency bounded.
#[derive(Debug)]
pub struct BoundedEncodedVideoQueue {
    frames: VecDeque<H264Frame>,
    queued_bytes: usize,
    max_frames: usize,
    max_bytes: usize,
    awaiting_keyframe: bool,
}

impl BoundedEncodedVideoQueue {
    pub fn new(max_frames: usize, max_bytes: usize) -> Result<Self, FrameError> {
        if max_frames == 0 || max_bytes == 0 {
            return Err(FrameError::InvalidQueueLimit);
        }
        Ok(Self {
            frames: VecDeque::new(),
            queued_bytes: 0,
            max_frames,
            max_bytes,
            awaiting_keyframe: false,
        })
    }

    pub fn push(&mut self, frame: H264Frame) -> Result<QueuePushOutcome, FrameError> {
        let frame_bytes = frame.bytes.len();
        if frame_bytes > self.max_bytes {
            return Err(FrameError::FrameTooLarge);
        }
        if self.awaiting_keyframe && !frame.keyframe {
            return Ok(QueuePushOutcome {
                queued: false,
                request_keyframe: false,
                dropped_until_keyframe: true,
            });
        }
        if frame.keyframe {
            self.clear();
            self.awaiting_keyframe = false;
            self.enqueue(frame, frame_bytes);
            return Ok(QueuePushOutcome {
                queued: true,
                request_keyframe: false,
                dropped_until_keyframe: false,
            });
        }
        let exceeds_limit = self.frames.len() >= self.max_frames
            || self.queued_bytes.saturating_add(frame_bytes) > self.max_bytes;
        if exceeds_limit {
            self.clear();
            self.awaiting_keyframe = true;
            return Ok(QueuePushOutcome {
                queued: false,
                request_keyframe: true,
                dropped_until_keyframe: true,
            });
        }
        self.enqueue(frame, frame_bytes);
        Ok(QueuePushOutcome {
            queued: true,
            request_keyframe: false,
            dropped_until_keyframe: false,
        })
    }

    pub fn pop(&mut self) -> Option<H264Frame> {
        let frame = self.frames.pop_front()?;
        self.queued_bytes -= frame.bytes.len();
        Some(frame)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    fn enqueue(&mut self, frame: H264Frame, frame_bytes: usize) {
        self.queued_bytes += frame_bytes;
        self.frames.push_back(frame);
    }

    fn clear(&mut self) {
        self.frames.clear();
        self.queued_bytes = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticSourceError {
    InvalidDimensions,
    InvalidFrameRate,
    TimestampOverflow,
}

/// One synthetic I420 test frame. Its planes are source pixels, not WebRTC-encoded data.
#[derive(Debug, PartialEq, Eq)]
pub struct SyntheticI420Frame {
    width: u16,
    height: u16,
    frame_index: u64,
    captured_at_ns: u64,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
}

impl SyntheticI420Frame {
    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub const fn frame_index(&self) -> u64 {
        self.frame_index
    }

    pub const fn captured_at_ns(&self) -> u64 {
        self.captured_at_ns
    }

    pub fn y_plane(&self) -> &[u8] {
        &self.y
    }

    pub fn u_plane(&self) -> &[u8] {
        &self.u
    }

    pub fn v_plane(&self) -> &[u8] {
        &self.v
    }
}

/// Generates a deterministic grayscale/color-bar I420 source for encoder and pipeline tests.
/// The caller owns each returned frame; the source does not retain pixel buffers.
#[derive(Debug)]
pub struct SyntheticVideoSource {
    width: u16,
    height: u16,
    fps: u16,
    origin_ns: u64,
    next_frame_index: u64,
}

impl SyntheticVideoSource {
    pub fn new(
        width: u16,
        height: u16,
        fps: u16,
        origin_ns: u64,
    ) -> Result<Self, SyntheticSourceError> {
        if width < 16
            || height < 2
            || width & 1 != 0
            || height & 1 != 0
            || width > 1_920
            || height > 1_080
        {
            return Err(SyntheticSourceError::InvalidDimensions);
        }
        if !(1..=60).contains(&fps) {
            return Err(SyntheticSourceError::InvalidFrameRate);
        }
        Ok(Self {
            width,
            height,
            fps,
            origin_ns,
            next_frame_index: 0,
        })
    }

    pub fn next_frame(&mut self) -> Result<SyntheticI420Frame, SyntheticSourceError> {
        let frame_index = self.next_frame_index;
        let elapsed_ns = u128::from(frame_index)
            .checked_mul(1_000_000_000)
            .ok_or(SyntheticSourceError::TimestampOverflow)?
            / u128::from(self.fps);
        let captured_at_ns = u64::try_from(u128::from(self.origin_ns) + elapsed_ns)
            .map_err(|_| SyntheticSourceError::TimestampOverflow)?;
        let next_frame_index = frame_index
            .checked_add(1)
            .ok_or(SyntheticSourceError::TimestampOverflow)?;
        let (y, u, v) = color_bars(self.width, self.height);
        self.next_frame_index = next_frame_index;
        Ok(SyntheticI420Frame {
            width: self.width,
            height: self.height,
            frame_index,
            captured_at_ns,
            y,
            u,
            v,
        })
    }
}

fn color_bars(width: u16, height: u16) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    const LUMA: [u8; 8] = [235, 210, 170, 145, 106, 81, 41, 16];
    const CHROMA_U: [u8; 8] = [128, 16, 166, 54, 202, 90, 240, 128];
    const CHROMA_V: [u8; 8] = [128, 146, 16, 34, 222, 240, 110, 128];
    let width_usize = usize::from(width);
    let height_usize = usize::from(height);
    let luma_bar_width = width_usize / LUMA.len();
    let mut y = vec![0; width_usize * height_usize];
    for row in 0..height_usize {
        for column in 0..width_usize {
            y[row * width_usize + column] = LUMA[(column / luma_bar_width).min(7)];
        }
    }
    let chroma_width = width_usize / 2;
    let chroma_height = height_usize / 2;
    let mut u = vec![0; chroma_width * chroma_height];
    let mut v = vec![0; chroma_width * chroma_height];
    for row in 0..chroma_height {
        for column in 0..chroma_width {
            let bar = ((column * 2) / luma_bar_width).min(7);
            let index = row * chroma_width + column;
            u[index] = CHROMA_U[bar];
            v[index] = CHROMA_V[bar];
        }
    }
    (y, u, v)
}
