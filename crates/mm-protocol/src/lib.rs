use std::collections::HashMap;
use std::fmt;
use std::str;

use mm_domain::{DeviceId, MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, MessageId, SessionId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
pub const MAX_SDP_BYTES: usize = 128 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 4 * 1024;
pub const MAX_ANNOTATION_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    DeviceHello,
    PairBegin,
    PairApprove,
    PairReject,
    PairRevoke,
    SessionOffer,
    SessionAnswer,
    SessionIce,
    SessionStart,
    SessionStop,
    SessionPause,
    SessionResume,
    SessionQuality,
    SessionStats,
    SessionError,
    AnnotationStrokeStart,
    AnnotationStrokePoints,
    AnnotationStrokeEnd,
    AnnotationUndo,
    AnnotationClear,
}

impl MessageKind {
    fn parse(value: &str) -> Result<Self, ProtocolError> {
        match value {
            "device.hello" => Ok(Self::DeviceHello),
            "pair.begin" => Ok(Self::PairBegin),
            "pair.approve" => Ok(Self::PairApprove),
            "pair.reject" => Ok(Self::PairReject),
            "pair.revoke" => Ok(Self::PairRevoke),
            "session.offer" => Ok(Self::SessionOffer),
            "session.answer" => Ok(Self::SessionAnswer),
            "session.ice" => Ok(Self::SessionIce),
            "session.start" => Ok(Self::SessionStart),
            "session.stop" => Ok(Self::SessionStop),
            "session.pause" => Ok(Self::SessionPause),
            "session.resume" => Ok(Self::SessionResume),
            "session.quality" => Ok(Self::SessionQuality),
            "session.stats" => Ok(Self::SessionStats),
            "session.error" => Ok(Self::SessionError),
            "annotation.stroke.start" => Ok(Self::AnnotationStrokeStart),
            "annotation.stroke.points" => Ok(Self::AnnotationStrokePoints),
            "annotation.stroke.end" => Ok(Self::AnnotationStrokeEnd),
            "annotation.undo" => Ok(Self::AnnotationUndo),
            "annotation.clear" => Ok(Self::AnnotationClear),
            _ => Err(ProtocolError::UnknownMessageType),
        }
    }

    fn requires_session(self) -> bool {
        !matches!(
            self,
            Self::DeviceHello
                | Self::PairBegin
                | Self::PairApprove
                | Self::PairReject
                | Self::PairRevoke
        )
    }

    fn is_annotation(self) -> bool {
        matches!(
            self,
            Self::AnnotationStrokeStart
                | Self::AnnotationStrokePoints
                | Self::AnnotationStrokeEnd
                | Self::AnnotationUndo
                | Self::AnnotationClear
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeHeader {
    pub version: u16,
    pub kind: MessageKind,
    pub message_id: MessageId,
    pub session_id: Option<SessionId>,
    pub sequence: u64,
    pub sent_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<T> {
    pub header: EnvelopeHeader,
    pub payload: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    EmptyFrame,
    FrameTooLarge,
    InvalidUtf8,
    InvalidJson,
    InvalidEnvelope,
    InvalidIdentifier,
    InvalidPayload,
    UnknownMessageType,
    UnsupportedVersion,
    InvalidSequence,
    SequenceReplay,
    SequenceTrackerFull,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyFrame => "control message is empty",
            Self::FrameTooLarge => "control message exceeds the configured byte limit",
            Self::InvalidUtf8 => "control message is not valid UTF-8",
            Self::InvalidJson => "control message is not valid JSON",
            Self::InvalidEnvelope => "control message envelope is invalid",
            Self::InvalidIdentifier => "control message contains an invalid identifier",
            Self::InvalidPayload => "control message payload is invalid",
            Self::UnknownMessageType => "control message type is unknown",
            Self::UnsupportedVersion => "control message protocol version is unsupported",
            Self::InvalidSequence => "control message sequence is invalid",
            Self::SequenceReplay => "control message sequence is stale or replayed",
            Self::SequenceTrackerFull => "too many control streams are being tracked",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug)]
pub struct ValidatedMessage {
    pub header: EnvelopeHeader,
    pub payload: MessagePayload,
}

#[derive(Debug)]
pub enum MessagePayload {
    DeviceHello(DeviceHello),
    PairBegin(PairBegin),
    PairApprove(PairApprove),
    PairReject(PairReject),
    PairRevoke(PairRevoke),
    SessionOffer(SessionOffer),
    SessionAnswer(SessionAnswer),
    SessionIce(SessionIce),
    SessionStart(SessionStart),
    SessionStop(SessionStop),
    SessionPause(SessionPause),
    SessionResume(SessionResume),
    SessionQuality(SessionQuality),
    SessionStats(SessionStats),
    SessionError(SessionError),
    AnnotationStrokeStart(AnnotationStrokeStart),
    AnnotationStrokePoints(AnnotationStrokePoints),
    AnnotationStrokeEnd(AnnotationStrokeEnd),
    AnnotationUndo(AnnotationUndo),
    AnnotationClear(AnnotationClear),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub video: Vec<String>,
    pub audio: Vec<String>,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceHello {
    pub device_id: String,
    pub protocol_versions: Vec<u16>,
    pub capabilities: Capabilities,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairBegin {
    pub receiver_id: String,
    pub nonce: String,
    pub peer_id: String,
    pub display_name: String,
    pub public_key: String,
    pub requested_grants: Vec<String>,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairApprove {
    pub accepted_grants: Vec<String>,
    pub peer_identity_proof: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairReject {
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairRevoke {
    pub peer_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionOffer {
    pub role: String,
    pub capabilities: Capabilities,
    pub sdp: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionAnswer {
    pub sdp: String,
    pub selected_codec: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionIce {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub mline_index: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStart {
    pub source_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStop {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionPause {
    pub pause: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionResume {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionQuality {
    pub max_width: u32,
    pub max_height: u32,
    pub target_fps: u16,
    pub video_kbps: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStats {
    pub fps: f32,
    pub loss_percent: f32,
    pub rtt_ms: u32,
    pub bitrate_kbps: u32,
    pub queue_depth: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationStrokeStart {
    pub stroke_id: String,
    pub color_rgba: String,
    pub width_px: u8,
    pub transform_rev: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationPoint {
    pub u: f32,
    pub v: f32,
    pub dt_ms: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationStrokePoints {
    pub stroke_id: String,
    pub points: Vec<AnnotationPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationStrokeEnd {
    pub stroke_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationUndo {
    pub stroke_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationClear {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope<'a> {
    v: u16,
    #[serde(rename = "type")]
    message_type: String,
    message_id: String,
    session_id: Option<String>,
    seq: u64,
    sent_at_ms: u64,
    #[serde(borrow)]
    payload: &'a RawValue,
}

#[derive(Serialize)]
struct PairBeginEnvelope<'a> {
    v: u16,
    #[serde(rename = "type")]
    message_type: &'static str,
    message_id: String,
    seq: u64,
    sent_at_ms: u64,
    payload: PairBeginPayloadRef<'a>,
}

#[derive(Serialize)]
struct PairBeginPayloadRef<'a> {
    receiver_id: &'a str,
    nonce: &'a str,
    peer_id: &'a str,
    display_name: &'a str,
    public_key: &'a str,
    requested_grants: &'a [String],
    signature: &'a str,
}

#[derive(Serialize)]
struct PairApproveEnvelope<'a> {
    v: u16,
    #[serde(rename = "type")]
    message_type: &'static str,
    message_id: String,
    seq: u64,
    sent_at_ms: u64,
    payload: PairApprovePayloadRef<'a>,
}

#[derive(Serialize)]
struct PairApprovePayloadRef<'a> {
    accepted_grants: &'a [String],
    peer_identity_proof: &'a str,
}

#[derive(Serialize)]
struct PairRejectEnvelope<'a> {
    v: u16,
    #[serde(rename = "type")]
    message_type: &'static str,
    message_id: String,
    seq: u64,
    sent_at_ms: u64,
    payload: PairRejectPayloadRef<'a>,
}

#[derive(Serialize)]
struct PairRejectPayloadRef<'a> {
    reason: &'a str,
}

/// Performs byte and UTF-8 checks before a JSON decoder allocates a payload tree.
/// This is deliberately a preflight step, not a JSON parser.
pub fn preflight_json(bytes: &[u8]) -> Result<&str, ProtocolError> {
    if bytes.is_empty() {
        return Err(ProtocolError::EmptyFrame);
    }
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidUtf8)
}

pub fn validate_version(version: u16) -> Result<(), ProtocolError> {
    if version == 1 {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedVersion)
    }
}

/// Encodes one validated `pair.begin` payload in the common sessionless v1 envelope.
/// The caller must provide a signature over the canonical application proof bytes.
pub fn encode_pair_begin(
    message_id: MessageId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &PairBegin,
) -> Result<Vec<u8>, ProtocolError> {
    if sequence == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    validate_pair_begin(payload)?;

    let envelope = PairBeginEnvelope {
        v: 1,
        message_type: "pair.begin",
        message_id: message_id.to_string(),
        seq: sequence,
        sent_at_ms,
        payload: PairBeginPayloadRef {
            receiver_id: &payload.receiver_id,
            nonce: &payload.nonce,
            peer_id: &payload.peer_id,
            display_name: &payload.display_name,
            public_key: &payload.public_key,
            requested_grants: &payload.requested_grants,
            signature: &payload.signature,
        },
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| ProtocolError::InvalidPayload)?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    Ok(bytes)
}

/// Encodes the receiver's explicit approval decision in the sessionless v1 pairing envelope.
/// `peer_identity_proof` is the canonical lowercase SHA-256 fingerprint of the receiver key
/// pinned by the QR invitation.
pub fn encode_pair_approve(
    message_id: MessageId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &PairApprove,
) -> Result<Vec<u8>, ProtocolError> {
    if sequence == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    validate_pair_approve(payload)?;

    let envelope = PairApproveEnvelope {
        v: 1,
        message_type: "pair.approve",
        message_id: message_id.to_string(),
        seq: sequence,
        sent_at_ms,
        payload: PairApprovePayloadRef {
            accepted_grants: &payload.accepted_grants,
            peer_identity_proof: &payload.peer_identity_proof,
        },
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| ProtocolError::InvalidPayload)?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    Ok(bytes)
}

/// Encodes the receiver's explicit rejection decision in the sessionless v1 pairing envelope.
pub fn encode_pair_reject(
    message_id: MessageId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &PairReject,
) -> Result<Vec<u8>, ProtocolError> {
    if sequence == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    validate_pair_reject(payload)?;

    let envelope = PairRejectEnvelope {
        v: 1,
        message_type: "pair.reject",
        message_id: message_id.to_string(),
        seq: sequence,
        sent_at_ms,
        payload: PairRejectPayloadRef {
            reason: &payload.reason,
        },
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| ProtocolError::InvalidPayload)?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    Ok(bytes)
}

/// Encodes revocation of the addressed peer on an authenticated control channel.
pub fn encode_pair_revoke(
    message_id: MessageId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &PairRevoke,
) -> Result<Vec<u8>, ProtocolError> {
    if sequence == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    validate_canonical_identifier(&payload.peer_id)?;
    let bytes = serde_json::to_vec(&serde_json::json!({
        "v": 1, "type": "pair.revoke", "message_id": message_id.to_string(),
        "seq": sequence, "sent_at_ms": sent_at_ms, "payload": payload
    }))
    .map_err(|_| ProtocolError::InvalidPayload)?;
    parse_message(&bytes)?;
    Ok(bytes)
}

/// Encodes a session answer in the common v1 control envelope.
pub fn encode_session_answer(
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &SessionAnswer,
) -> Result<Vec<u8>, ProtocolError> {
    encode_session_control(
        "session.answer",
        message_id,
        session_id,
        sequence,
        sent_at_ms,
        payload,
    )
}

/// Encodes a sender's H.264-capable session offer in the common v1 control envelope.
pub fn encode_session_offer(
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &SessionOffer,
) -> Result<Vec<u8>, ProtocolError> {
    if payload.role != "sender" {
        return Err(ProtocolError::InvalidPayload);
    }
    validate_capabilities(&payload.capabilities)?;
    bounded_sdp(&payload.sdp)?;
    encode_session_control(
        "session.offer",
        message_id,
        session_id,
        sequence,
        sent_at_ms,
        payload,
    )
}

/// Encodes the sender's source-start notification after the peer connection is ready.
pub fn encode_session_start(
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &SessionStart,
) -> Result<Vec<u8>, ProtocolError> {
    encode_session_control(
        "session.start",
        message_id,
        session_id,
        sequence,
        sent_at_ms,
        payload,
    )
}

/// Encodes an explicit sender-initiated session stop in the common v1 envelope.
pub fn encode_session_stop(
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &SessionStop,
) -> Result<Vec<u8>, ProtocolError> {
    bounded_text(&payload.reason, 1, 128)?;
    encode_session_control(
        "session.stop",
        message_id,
        session_id,
        sequence,
        sent_at_ms,
        payload,
    )
}

/// Encodes a trickled ICE candidate in the common v1 control envelope.
pub fn encode_session_ice(
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &SessionIce,
) -> Result<Vec<u8>, ProtocolError> {
    encode_session_control(
        "session.ice",
        message_id,
        session_id,
        sequence,
        sent_at_ms,
        payload,
    )
}

fn encode_session_control<T: Serialize>(
    message_type: &'static str,
    message_id: MessageId,
    session_id: SessionId,
    sequence: u64,
    sent_at_ms: u64,
    payload: &T,
) -> Result<Vec<u8>, ProtocolError> {
    if sequence == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    let envelope = SessionControlEnvelopeRef {
        v: 1,
        message_type,
        message_id: message_id.to_string(),
        session_id: session_id.to_string(),
        seq: sequence,
        sent_at_ms,
        payload,
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| ProtocolError::InvalidPayload)?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    parse_message(&bytes)?;
    Ok(bytes)
}

#[derive(Serialize)]
struct SessionControlEnvelopeRef<'a, T> {
    v: u16,
    #[serde(rename = "type")]
    message_type: &'static str,
    message_id: String,
    session_id: String,
    seq: u64,
    sent_at_ms: u64,
    payload: &'a T,
}

/// Parses a complete v1 JSON control message with strict envelope and payload schemas.
/// The caller must authenticate the peer and apply session/grant authorization separately.
pub fn parse_message(bytes: &[u8]) -> Result<ValidatedMessage, ProtocolError> {
    let text = preflight_json(bytes)?;
    let raw: RawEnvelope<'_> =
        serde_json::from_str(text).map_err(|_| ProtocolError::InvalidJson)?;
    validate_version(raw.v)?;
    let kind = MessageKind::parse(&raw.message_type)?;
    let message_id =
        MessageId::parse(&raw.message_id).map_err(|_| ProtocolError::InvalidIdentifier)?;
    let session_id = raw
        .session_id
        .as_deref()
        .map(SessionId::parse)
        .transpose()
        .map_err(|_| ProtocolError::InvalidIdentifier)?;

    if kind.requires_session() != session_id.is_some() {
        return Err(ProtocolError::InvalidEnvelope);
    }
    if raw.seq == 0 {
        return Err(ProtocolError::InvalidSequence);
    }
    if kind.is_annotation() && bytes.len() > MAX_ANNOTATION_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }

    let header = EnvelopeHeader {
        version: raw.v,
        kind,
        message_id,
        session_id,
        sequence: raw.seq,
        sent_at_ms: raw.sent_at_ms,
    };
    let payload = match kind {
        MessageKind::DeviceHello => {
            let value: DeviceHello = decode(raw.payload)?;
            validate_identifier(&value.device_id)?;
            validate_capabilities(&value.capabilities)?;
            if value.protocol_versions.is_empty()
                || value.protocol_versions.len() > 8
                || value.protocol_versions.contains(&0)
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::DeviceHello(value)
        }
        MessageKind::PairBegin => {
            let value: PairBegin = decode(raw.payload)?;
            validate_pair_begin(&value)?;
            MessagePayload::PairBegin(value)
        }
        MessageKind::PairApprove => {
            let value: PairApprove = decode(raw.payload)?;
            validate_pair_approve(&value)?;
            MessagePayload::PairApprove(value)
        }
        MessageKind::PairReject => {
            let value: PairReject = decode(raw.payload)?;
            validate_pair_reject(&value)?;
            MessagePayload::PairReject(value)
        }
        MessageKind::PairRevoke => {
            let value: PairRevoke = decode(raw.payload)?;
            validate_identifier(&value.peer_id)?;
            MessagePayload::PairRevoke(value)
        }
        MessageKind::SessionOffer => {
            let value: SessionOffer = decode(raw.payload)?;
            if value.role != "sender" {
                return Err(ProtocolError::InvalidPayload);
            }
            validate_capabilities(&value.capabilities)?;
            bounded_sdp(&value.sdp)?;
            MessagePayload::SessionOffer(value)
        }
        MessageKind::SessionAnswer => {
            let value: SessionAnswer = decode(raw.payload)?;
            bounded_sdp(&value.sdp)?;
            if value.selected_codec != "H264" {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::SessionAnswer(value)
        }
        MessageKind::SessionIce => {
            let value: SessionIce = decode(raw.payload)?;
            bounded_text(&value.candidate, 1, MAX_CANDIDATE_BYTES)?;
            if value
                .sdp_mid
                .as_deref()
                .is_some_and(|mid| mid.is_empty() || mid.len() > 256)
                || value.sdp_mid.as_deref().is_some_and(contains_control)
                || value.mline_index.is_some_and(|index| index > 1024)
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::SessionIce(value)
        }
        MessageKind::SessionStart => {
            let value: SessionStart = decode(raw.payload)?;
            bounded_text(&value.source_id, 1, 128)?;
            MessagePayload::SessionStart(value)
        }
        MessageKind::SessionStop => {
            let value: SessionStop = decode(raw.payload)?;
            bounded_text(&value.reason, 1, 128)?;
            MessagePayload::SessionStop(value)
        }
        MessageKind::SessionPause => {
            let value: SessionPause = decode(raw.payload)?;
            if !value.pause {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::SessionPause(value)
        }
        MessageKind::SessionResume => MessagePayload::SessionResume(decode(raw.payload)?),
        MessageKind::SessionQuality => {
            let value: SessionQuality = decode(raw.payload)?;
            if !(1..=7680).contains(&value.max_width)
                || !(1..=4320).contains(&value.max_height)
                || !(1..=120).contains(&value.target_fps)
                || !(1..=200_000).contains(&value.video_kbps)
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::SessionQuality(value)
        }
        MessageKind::SessionStats => {
            let value: SessionStats = decode(raw.payload)?;
            if !value.fps.is_finite()
                || !(0.0..=120.0).contains(&value.fps)
                || !value.loss_percent.is_finite()
                || !(0.0..=100.0).contains(&value.loss_percent)
                || value.rtt_ms > 600_000
                || value.bitrate_kbps > 200_000
                || value.queue_depth > 100_000
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::SessionStats(value)
        }
        MessageKind::SessionError => {
            let value: SessionError = decode(raw.payload)?;
            bounded_text(&value.code, 1, 64)?;
            bounded_text(&value.message, 1, 256)?;
            MessagePayload::SessionError(value)
        }
        MessageKind::AnnotationStrokeStart => {
            let value: AnnotationStrokeStart = decode(raw.payload)?;
            validate_identifier(&value.stroke_id)?;
            if value.color_rgba.len() != 9
                || !value.color_rgba.starts_with('#')
                || !value.color_rgba[1..]
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || !(1..=64).contains(&value.width_px)
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::AnnotationStrokeStart(value)
        }
        MessageKind::AnnotationStrokePoints => {
            let value: AnnotationStrokePoints = decode(raw.payload)?;
            validate_identifier(&value.stroke_id)?;
            if value.points.is_empty()
                || value.points.len() > 256
                || value.points.iter().any(|point| {
                    !point.u.is_finite()
                        || !point.v.is_finite()
                        || !(0.0..=1.0).contains(&point.u)
                        || !(0.0..=1.0).contains(&point.v)
                        || point.dt_ms > 60_000
                })
            {
                return Err(ProtocolError::InvalidPayload);
            }
            MessagePayload::AnnotationStrokePoints(value)
        }
        MessageKind::AnnotationStrokeEnd => {
            let value: AnnotationStrokeEnd = decode(raw.payload)?;
            validate_identifier(&value.stroke_id)?;
            MessagePayload::AnnotationStrokeEnd(value)
        }
        MessageKind::AnnotationUndo => {
            let value: AnnotationUndo = decode(raw.payload)?;
            validate_identifier(&value.stroke_id)?;
            MessagePayload::AnnotationUndo(value)
        }
        MessageKind::AnnotationClear => MessagePayload::AnnotationClear(decode(raw.payload)?),
    };

    Ok(ValidatedMessage { header, payload })
}

fn decode<T: DeserializeOwned>(raw: &RawValue) -> Result<T, ProtocolError> {
    serde_json::from_str(raw.get()).map_err(|_| ProtocolError::InvalidPayload)
}

fn validate_identifier(value: &str) -> Result<(), ProtocolError> {
    MessageId::parse(value)
        .map(|_| ())
        .map_err(|_| ProtocolError::InvalidIdentifier)
}

fn validate_canonical_identifier(value: &str) -> Result<(), ProtocolError> {
    let device_id = DeviceId::parse(value).map_err(|_| ProtocolError::InvalidIdentifier)?;
    if device_id.to_string() != value {
        return Err(ProtocolError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_pair_begin(value: &PairBegin) -> Result<(), ProtocolError> {
    validate_canonical_identifier(&value.receiver_id)?;
    validate_lower_hex(&value.nonce, 16)?;
    validate_canonical_identifier(&value.peer_id)?;
    if value.display_name.is_empty()
        || value.display_name.chars().count() > MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS
        || contains_control(&value.display_name)
    {
        return Err(ProtocolError::InvalidPayload);
    }
    validate_lower_hex(&value.public_key, 32)?;
    validate_grants(&value.requested_grants)?;
    if value.requested_grants.is_empty() {
        return Err(ProtocolError::InvalidPayload);
    }
    validate_lower_hex(&value.signature, 64)?;
    Ok(())
}

fn validate_pair_approve(value: &PairApprove) -> Result<(), ProtocolError> {
    validate_grants(&value.accepted_grants)?;
    if value.accepted_grants.is_empty() {
        return Err(ProtocolError::InvalidPayload);
    }
    validate_lower_hex(&value.peer_identity_proof, 32)
}

fn validate_pair_reject(value: &PairReject) -> Result<(), ProtocolError> {
    bounded_text(&value.reason, 1, 128)
}

fn validate_capabilities(value: &Capabilities) -> Result<(), ProtocolError> {
    if value.video.is_empty()
        || value.video.len() > 8
        || value.audio.len() > 8
        || value.video.iter().any(|codec| codec != "H264")
        || value.audio.iter().any(|codec| codec != "opus")
        || !(1..=7680).contains(&value.max_width)
        || !(1..=4320).contains(&value.max_height)
        || !(1..=120).contains(&value.max_fps)
    {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(())
}

fn bounded_text(value: &str, min: usize, max: usize) -> Result<(), ProtocolError> {
    if value.len() < min || value.len() > max || contains_control(value) {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(())
}

fn bounded_sdp(value: &str) -> Result<(), ProtocolError> {
    if value.is_empty()
        || value.len() > MAX_SDP_BYTES
        || value
            .chars()
            .any(|character| character.is_control() && character != '\r' && character != '\n')
    {
        return Err(ProtocolError::InvalidPayload);
    }
    let bytes = value.as_bytes();
    if bytes.iter().enumerate().any(|(index, byte)| {
        (*byte == b'\r' && bytes.get(index + 1) != Some(&b'\n'))
            || (*byte == b'\n' && (index == 0 || bytes[index - 1] != b'\r'))
    }) {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(())
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn bounded_list(values: &[String]) -> Result<(), ProtocolError> {
    if values.len() > 32 {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(())
}

fn validate_string_list(values: &[String], max_item_bytes: usize) -> Result<(), ProtocolError> {
    for value in values {
        bounded_text(value, 1, max_item_bytes)?;
    }
    Ok(())
}

fn validate_grants(values: &[String]) -> Result<(), ProtocolError> {
    bounded_list(values)?;
    validate_string_list(values, 64)?;
    if values
        .iter()
        .any(|grant| !matches!(grant.as_str(), "view" | "send" | "annotate"))
    {
        return Err(ProtocolError::InvalidPayload);
    }
    for (index, grant) in values.iter().enumerate() {
        if values[..index].contains(grant) {
            return Err(ProtocolError::InvalidPayload);
        }
    }
    Ok(())
}

fn validate_lower_hex(value: &str, expected_bytes: usize) -> Result<(), ProtocolError> {
    if value.len() != expected_bytes * 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(())
}

/// Tracks monotonically increasing sequence numbers per authenticated peer/session.
/// The caller must remove a session when it is closed to keep this map bounded.
pub struct SequenceTracker {
    max_streams: usize,
    latest: HashMap<(SessionId, DeviceId), u64>,
}

impl SequenceTracker {
    pub fn new(max_streams: usize) -> Self {
        Self {
            max_streams,
            latest: HashMap::new(),
        }
    }

    pub fn accept(
        &mut self,
        session_id: SessionId,
        peer_id: DeviceId,
        sequence: u64,
    ) -> Result<(), ProtocolError> {
        if sequence == 0 {
            return Err(ProtocolError::InvalidSequence);
        }
        let key = (session_id, peer_id);
        if let Some(previous) = self.latest.get_mut(&key) {
            if sequence > *previous {
                *previous = sequence;
                return Ok(());
            }
            return Err(ProtocolError::SequenceReplay);
        }
        if self.latest.len() >= self.max_streams {
            return Err(ProtocolError::SequenceTrackerFull);
        }
        self.latest.insert(key, sequence);
        Ok(())
    }

    pub fn forget_session(&mut self, session_id: SessionId) {
        self.latest
            .retain(|(tracked_session, _), _| *tracked_session != session_id);
    }
}
