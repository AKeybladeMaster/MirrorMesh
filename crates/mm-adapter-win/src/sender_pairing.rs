use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mm_adapter_signaling::{PairingConnection, WssClientError, WssPairingClient};
use mm_domain::{MessageId, PeerGrant, SessionId, TrustedPeerGrants};
use mm_protocol::{
    Capabilities, MessagePayload, SessionAnswer, SessionIce, SessionOffer, SessionStart,
    SessionStop, encode_session_ice, encode_session_offer, encode_session_start,
    encode_session_stop,
};

use crate::{WindowsPairingIdentity, initialize_pairing_runtime};

pub const MAX_PAIRING_INVITATION_BYTES: usize = 4096;
pub const MAX_SESSION_STOP_REASON_BYTES: usize = 128;
const ANDROID_TV_PAIRING_PORT: u16 = 8765;
const LOCAL_DISPLAY_NAME: &str = "MirrorMesh Windows PC";

const PAIRING_ERROR_NONE: u32 = 0;
const PAIRING_ERROR_RUNTIME_UNAVAILABLE: u32 = 1;
const PAIRING_ERROR_INVITATION_REJECTED: u32 = 2;
const PAIRING_ERROR_SIGNALING_FAILED: u32 = 3;
const PAIRING_ERROR_CONTROL_CHANNEL_CLOSED: u32 = 4;
const PAIRING_ERROR_INVALID_APPROVAL: u32 = 5;
const PAIRING_ERROR_INTERNAL: u32 = 6;
const PAIRING_ERROR_CONTROL_CHANNEL: u32 = 7;

const MAX_SESSION_DESCRIPTION_BYTES: usize = 128 * 1024;
const MAX_ICE_CANDIDATE_BYTES: usize = 4 * 1024;
const MAX_ICE_MEDIA_ID_BYTES: usize = 256;
const MAX_SESSION_SOURCE_ID_BYTES: usize = 128;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderPairingState {
    Connecting = 1,
    AwaitingApproval = 2,
    Approved = 3,
    Rejected = 4,
    Failed = 5,
    Cancelling = 6,
    Cancelled = 7,
}

impl SenderPairingState {
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::AwaitingApproval | Self::Approved | Self::Cancelling
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderPairingSnapshot {
    pub state: SenderPairingState,
    pub error_code: u32,
    pub receiver_fingerprint: [u8; 32],
}

impl SenderPairingSnapshot {
    const fn connecting() -> Self {
        Self {
            state: SenderPairingState::Connecting,
            error_code: PAIRING_ERROR_NONE,
            receiver_fingerprint: [0; 32],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderPairingError {
    InvalidInvitation,
    InvalidSessionMessage,
    AlreadyActive,
    SessionAlreadyStarted,
    SessionNotStarted,
    UnknownRequest,
    ControlFailed,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderSessionEvent {
    Pending,
    Answer {
        session_id: String,
        sdp: String,
    },
    IceCandidate {
        session_id: String,
        candidate: String,
        media_id: String,
        media_line_index: u16,
    },
    Stopped {
        session_id: String,
        reason: String,
    },
    Revoked {
        peer_id: String,
    },
}

struct SenderSession {
    id: SessionId,
    outbound_sequence: u64,
    answer_received: bool,
    started: bool,
}

struct SenderPairingTask {
    request_id: u64,
    snapshot: Arc<Mutex<SenderPairingSnapshot>>,
    cancel_requested: Arc<AtomicBool>,
    commands: SyncSender<SenderSessionCommand>,
    events: Mutex<Receiver<SenderSessionEvent>>,
    session_id: Arc<Mutex<Option<SessionId>>>,
    session_start_requested: Arc<AtomicBool>,
}

enum SenderSessionCommand {
    Begin {
        id: SessionId,
        sdp: String,
    },
    Ice {
        id: SessionId,
        candidate: String,
        media_id: String,
        media_line_index: u16,
    },
    Start {
        id: SessionId,
        source_id: String,
    },
    Stop {
        id: SessionId,
        reason: String,
        response: SyncSender<Result<(), SenderPairingError>>,
    },
}

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static SENDER_PAIRING_TASK: OnceLock<Mutex<Option<SenderPairingTask>>> = OnceLock::new();

fn task_slot() -> &'static Mutex<Option<SenderPairingTask>> {
    SENDER_PAIRING_TASK.get_or_init(|| Mutex::new(None))
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn next_request_id() -> Option<u64> {
    let mut current = NEXT_REQUEST_ID.load(Ordering::Relaxed);
    loop {
        if current == 0 || current >= i64::MAX as u64 {
            return None;
        }
        match NEXT_REQUEST_ID.compare_exchange_weak(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(current),
            Err(observed) => current = observed,
        }
    }
}

pub fn validate_invitation_uri(invitation_uri: &str) -> Result<(), SenderPairingError> {
    if invitation_uri.is_empty()
        || invitation_uri.len() > MAX_PAIRING_INVITATION_BYTES
        || invitation_uri.trim() != invitation_uri
        || invitation_uri.chars().any(char::is_control)
    {
        return Err(SenderPairingError::InvalidInvitation);
    }

    Ok(())
}

/// Starts the QR-pinned Windows-to-receiver pairing flow on a worker thread.
///
/// The worker retains an approved WSS control connection until the request is cancelled or the
/// process shuts down. It requests only the receiver's `view` grant; media setup is a later step.
pub fn begin_sender_pairing(invitation_uri: String) -> Result<u64, SenderPairingError> {
    validate_invitation_uri(&invitation_uri)?;

    let mut slot = lock_recover(task_slot());
    if let Some(task) = slot.as_ref() {
        let state = lock_recover(&task.snapshot).state;
        if state.is_active() {
            return Err(SenderPairingError::AlreadyActive);
        }
    }

    let request_id = next_request_id().ok_or(SenderPairingError::Unavailable)?;
    let snapshot = Arc::new(Mutex::new(SenderPairingSnapshot::connecting()));
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let (command_sender, command_receiver) = sync_channel(32);
    let (event_sender, event_receiver) = sync_channel(128);
    let session_id = Arc::new(Mutex::new(None));
    let session_start_requested = Arc::new(AtomicBool::new(false));

    let worker_snapshot = Arc::clone(&snapshot);
    let worker_cancel_requested = Arc::clone(&cancel_requested);
    let worker_session_id = Arc::clone(&session_id);
    std::thread::Builder::new()
        .name("mirrormesh-windows-pairing".to_owned())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                run_sender_pairing(
                    &invitation_uri,
                    &worker_snapshot,
                    &worker_cancel_requested,
                    command_receiver,
                    event_sender,
                    worker_session_id,
                );
            }));
            if result.is_err() {
                set_terminal_state(
                    &worker_snapshot,
                    SenderPairingState::Failed,
                    PAIRING_ERROR_INTERNAL,
                );
            }
        })
        .map_err(|_| SenderPairingError::Unavailable)?;

    *slot = Some(SenderPairingTask {
        request_id,
        snapshot,
        cancel_requested,
        commands: command_sender,
        events: Mutex::new(event_receiver),
        session_id,
        session_start_requested,
    });
    Ok(request_id)
}

/// Queues one H.264 sender offer for the approved WSS control worker.
pub fn begin_sender_session(request_id: u64, sdp: &str) -> Result<SessionId, SenderPairingError> {
    if sdp.is_empty() || sdp.len() > MAX_SESSION_DESCRIPTION_BYTES {
        return Err(SenderPairingError::InvalidSessionMessage);
    }

    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;
    if lock_recover(&task.snapshot).state != SenderPairingState::Approved {
        return Err(SenderPairingError::SessionNotStarted);
    }

    let mut current_session_id = lock_recover(&task.session_id);
    if current_session_id.is_some() {
        return Err(SenderPairingError::SessionAlreadyStarted);
    }
    let session_id = random_session_id().ok_or(SenderPairingError::Unavailable)?;
    *current_session_id = Some(session_id);
    task.session_start_requested.store(false, Ordering::Release);
    match task.commands.try_send(SenderSessionCommand::Begin {
        id: session_id,
        sdp: sdp.to_owned(),
    }) {
        Ok(()) => Ok(session_id),
        Err(_) => {
            *current_session_id = None;
            Err(SenderPairingError::Unavailable)
        }
    }
}

/// Queues the privacy-safe source-start control message after the peer connection is ready.
pub fn start_sender_session(request_id: u64, source_id: &str) -> Result<(), SenderPairingError> {
    validate_session_source_id(source_id)?;

    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;
    if lock_recover(&task.snapshot).state != SenderPairingState::Approved {
        return Err(SenderPairingError::SessionNotStarted);
    }
    let session_id = lock_recover(&task.session_id)
        .as_ref()
        .copied()
        .ok_or(SenderPairingError::SessionNotStarted)?;
    task.session_start_requested
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| SenderPairingError::SessionAlreadyStarted)?;

    if task
        .commands
        .try_send(SenderSessionCommand::Start {
            id: session_id,
            source_id: source_id.to_owned(),
        })
        .is_err()
    {
        task.session_start_requested.store(false, Ordering::Release);
        return Err(SenderPairingError::Unavailable);
    }
    Ok(())
}

/// Sends a bounded explicit stop and waits briefly for the worker's write result.
/// The caller should still close local media and signaling if the acknowledgement is lost.
pub fn stop_sender_session(request_id: u64, reason: &str) -> Result<(), SenderPairingError> {
    validate_session_stop_reason(reason)?;

    let (commands, session_id) = {
        let slot = lock_recover(task_slot());
        let task = slot
            .as_ref()
            .filter(|task| task.request_id == request_id)
            .ok_or(SenderPairingError::UnknownRequest)?;
        if lock_recover(&task.snapshot).state != SenderPairingState::Approved {
            return Err(SenderPairingError::SessionNotStarted);
        }
        let session_id = lock_recover(&task.session_id)
            .as_ref()
            .copied()
            .ok_or(SenderPairingError::SessionNotStarted)?;
        (task.commands.clone(), session_id)
    };

    let (response_sender, response_receiver) = sync_channel(1);
    commands
        .try_send(SenderSessionCommand::Stop {
            id: session_id,
            reason: reason.to_owned(),
            response: response_sender,
        })
        .map_err(|_| SenderPairingError::Unavailable)?;
    response_receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| SenderPairingError::Unavailable)?
}

fn validate_session_source_id(source_id: &str) -> Result<(), SenderPairingError> {
    if source_id.is_empty()
        || source_id.len() > MAX_SESSION_SOURCE_ID_BYTES
        || source_id.chars().any(char::is_control)
    {
        return Err(SenderPairingError::InvalidSessionMessage);
    }
    Ok(())
}

fn validate_session_stop_reason(reason: &str) -> Result<(), SenderPairingError> {
    if reason.is_empty()
        || reason.len() > MAX_SESSION_STOP_REASON_BYTES
        || reason.chars().any(char::is_control)
    {
        return Err(SenderPairingError::InvalidSessionMessage);
    }
    Ok(())
}

/// Queues one bounded trickled ICE candidate for the active sender session.
pub fn send_sender_session_ice(
    request_id: u64,
    candidate: &str,
    media_id: &str,
    media_line_index: u16,
) -> Result<(), SenderPairingError> {
    if candidate.is_empty()
        || candidate.len() > MAX_ICE_CANDIDATE_BYTES
        || media_id.is_empty()
        || media_id.len() > MAX_ICE_MEDIA_ID_BYTES
        || candidate.chars().any(char::is_control)
        || media_id.chars().any(char::is_control)
        || media_line_index > 1024
    {
        return Err(SenderPairingError::InvalidSessionMessage);
    }

    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;
    if lock_recover(&task.snapshot).state != SenderPairingState::Approved {
        return Err(SenderPairingError::SessionNotStarted);
    }
    let session_id = lock_recover(&task.session_id)
        .as_ref()
        .copied()
        .ok_or(SenderPairingError::SessionNotStarted)?;
    task.commands
        .try_send(SenderSessionCommand::Ice {
            id: session_id,
            candidate: candidate.to_owned(),
            media_id: media_id.to_owned(),
            media_line_index,
        })
        .map_err(|_| SenderPairingError::Unavailable)
}

/// Reads one already-buffered answer, ICE, stop, or revoke event without performing socket I/O.
pub fn poll_sender_session_event(
    request_id: u64,
) -> Result<SenderSessionEvent, SenderPairingError> {
    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;
    match lock_recover(&task.events).try_recv() {
        Ok(event) => Ok(event),
        Err(TryRecvError::Empty) if lock_recover(&task.session_id).is_some() => {
            Ok(SenderSessionEvent::Pending)
        }
        Err(TryRecvError::Empty) => Err(SenderPairingError::SessionNotStarted),
        Err(TryRecvError::Disconnected) => Err(SenderPairingError::ControlFailed),
    }
}

fn run_sender_session_worker(
    mut connection: PairingConnection,
    snapshot: &Mutex<SenderPairingSnapshot>,
    cancel_requested: &AtomicBool,
    commands: Receiver<SenderSessionCommand>,
    events: SyncSender<SenderSessionEvent>,
    session_id_slot: &Mutex<Option<SessionId>>,
) {
    let mut session: Option<SenderSession> = None;
    loop {
        if cancel_requested.load(Ordering::Acquire) {
            let _ = connection.close();
            lock_recover(session_id_slot).take();
            set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
            return;
        }

        loop {
            match commands.try_recv() {
                Ok(SenderSessionCommand::Begin { id, sdp }) => {
                    if session.is_some() {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    let Some(message_id) = random_message_id() else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let offer = SessionOffer {
                        role: "sender".to_owned(),
                        capabilities: Capabilities {
                            video: vec!["H264".to_owned()],
                            audio: Vec::new(),
                            max_width: 1920,
                            max_height: 1080,
                            max_fps: 60,
                        },
                        sdp,
                    };
                    let Ok(frame) = encode_session_offer(
                        message_id,
                        id,
                        1,
                        now.as_millis().min(u64::MAX as u128) as u64,
                        &offer,
                    ) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    if connection.send_control_message(&frame).is_err() {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    session = Some(SenderSession {
                        id,
                        outbound_sequence: 1,
                        answer_received: false,
                        started: false,
                    });
                }
                Ok(SenderSessionCommand::Ice {
                    id,
                    candidate,
                    media_id,
                    media_line_index,
                }) => {
                    let Some(current) = session.as_mut().filter(|current| current.id == id) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Some(next_sequence) = current.outbound_sequence.checked_add(1) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Some(message_id) = random_message_id() else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let ice = SessionIce {
                        candidate,
                        sdp_mid: Some(media_id),
                        mline_index: Some(media_line_index),
                    };
                    let Ok(frame) = encode_session_ice(
                        message_id,
                        id,
                        next_sequence,
                        now.as_millis().min(u64::MAX as u128) as u64,
                        &ice,
                    ) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    if connection.send_control_message(&frame).is_err() {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    current.outbound_sequence = next_sequence;
                }
                Ok(SenderSessionCommand::Start { id, source_id }) => {
                    let Some(current) = session.as_mut().filter(|current| current.id == id) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    if !current.answer_received || current.started {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    let Some(next_sequence) = current.outbound_sequence.checked_add(1) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Some(message_id) = random_message_id() else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let start = SessionStart { source_id };
                    let Ok(frame) = encode_session_start(
                        message_id,
                        id,
                        next_sequence,
                        now.as_millis().min(u64::MAX as u128) as u64,
                        &start,
                    ) else {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    if connection.send_control_message(&frame).is_err() {
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    current.outbound_sequence = next_sequence;
                    current.started = true;
                }
                Ok(SenderSessionCommand::Stop {
                    id,
                    reason,
                    response,
                }) => {
                    let Some(current) = session.as_mut().filter(|current| current.id == id) else {
                        let _ = response.send(Err(SenderPairingError::SessionNotStarted));
                        continue;
                    };
                    let Some(next_sequence) = current.outbound_sequence.checked_add(1) else {
                        let _ = response.send(Err(SenderPairingError::Unavailable));
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Some(message_id) = random_message_id() else {
                        let _ = response.send(Err(SenderPairingError::Unavailable));
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                        let _ = response.send(Err(SenderPairingError::Unavailable));
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    let stop = SessionStop { reason };
                    let Ok(frame) = encode_session_stop(
                        message_id,
                        id,
                        next_sequence,
                        now.as_millis().min(u64::MAX as u128) as u64,
                        &stop,
                    ) else {
                        let _ = response.send(Err(SenderPairingError::InvalidSessionMessage));
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    };
                    if connection.send_control_message(&frame).is_err() {
                        let _ = response.send(Err(SenderPairingError::ControlFailed));
                        fail_sender_session_worker(snapshot, session_id_slot);
                        return;
                    }
                    current.outbound_sequence = next_sequence;
                    session = None;
                    lock_recover(session_id_slot).take();
                    let _ = response.send(Ok(()));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let _ = connection.close();
                    lock_recover(session_id_slot).take();
                    set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
                    return;
                }
            }
        }

        let message = match connection.try_receive_control_message() {
            Ok(message) => message,
            Err(_) => {
                fail_sender_session_worker(snapshot, session_id_slot);
                return;
            }
        };
        let Some(message) = message else {
            continue;
        };
        let active_session_id = session.as_ref().map(|current| current.id);
        let event = match message.payload {
            MessagePayload::SessionAnswer(SessionAnswer { sdp, .. })
                if active_session_id.is_some()
                    && message.header.session_id == active_session_id =>
            {
                let active_session_id = active_session_id.expect("checked active session");
                if let Some(current) = session.as_mut() {
                    current.answer_received = true;
                }
                SenderSessionEvent::Answer {
                    session_id: active_session_id.to_string(),
                    sdp,
                }
            }
            MessagePayload::SessionIce(ice)
                if active_session_id.is_some()
                    && message.header.session_id == active_session_id =>
            {
                let active_session_id = active_session_id.expect("checked active session");
                let (Some(media_id), Some(media_line_index)) = (ice.sdp_mid, ice.mline_index)
                else {
                    fail_sender_session_worker(snapshot, session_id_slot);
                    return;
                };
                SenderSessionEvent::IceCandidate {
                    session_id: active_session_id.to_string(),
                    candidate: ice.candidate,
                    media_id,
                    media_line_index,
                }
            }
            MessagePayload::SessionStop(stop)
                if active_session_id.is_some()
                    && message.header.session_id == active_session_id =>
            {
                let active_session_id = active_session_id.expect("checked active session");
                session = None;
                lock_recover(session_id_slot).take();
                SenderSessionEvent::Stopped {
                    session_id: active_session_id.to_string(),
                    reason: stop.reason,
                }
            }
            MessagePayload::PairRevoke(revoke) => {
                let event = SenderSessionEvent::Revoked {
                    peer_id: revoke.peer_id,
                };
                lock_recover(session_id_slot).take();
                let _ = events.try_send(event);
                set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
                return;
            }
            _ => {
                fail_sender_session_worker(snapshot, session_id_slot);
                return;
            }
        };
        if events.try_send(event).is_err() {
            fail_sender_session_worker(snapshot, session_id_slot);
            return;
        }
    }
}

fn fail_sender_session_worker(
    snapshot: &Mutex<SenderPairingSnapshot>,
    session_id_slot: &Mutex<Option<SessionId>>,
) {
    lock_recover(session_id_slot).take();
    set_terminal_state(
        snapshot,
        SenderPairingState::Failed,
        PAIRING_ERROR_CONTROL_CHANNEL,
    );
}

fn random_session_id() -> Option<SessionId> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(SessionId::from_bytes(bytes))
}

fn random_message_id() -> Option<MessageId> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(MessageId::from_bytes(bytes))
}

pub fn poll_sender_pairing(request_id: u64) -> Result<SenderPairingSnapshot, SenderPairingError> {
    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;
    Ok(*lock_recover(&task.snapshot))
}

pub fn cancel_sender_pairing(request_id: u64) -> Result<(), SenderPairingError> {
    let slot = lock_recover(task_slot());
    let task = slot
        .as_ref()
        .filter(|task| task.request_id == request_id)
        .ok_or(SenderPairingError::UnknownRequest)?;

    task.cancel_requested.store(true, Ordering::Release);
    let state = lock_recover(&task.snapshot).state;
    match state {
        SenderPairingState::Connecting | SenderPairingState::AwaitingApproval => {
            let mut snapshot = lock_recover(&task.snapshot);
            snapshot.state = SenderPairingState::Cancelling;
            snapshot.error_code = PAIRING_ERROR_NONE;
        }
        SenderPairingState::Approved => {
            lock_recover(&task.session_id).take();
            let mut snapshot = lock_recover(&task.snapshot);
            snapshot.state = SenderPairingState::Cancelling;
            snapshot.error_code = PAIRING_ERROR_NONE;
        }
        SenderPairingState::Cancelling
        | SenderPairingState::Rejected
        | SenderPairingState::Failed
        | SenderPairingState::Cancelled => {}
    }
    Ok(())
}

/// Cancels a pending attempt and releases any approved control connection without waiting on the UI thread.
pub fn shutdown_sender_pairing() {
    let request_id = lock_recover(task_slot())
        .as_ref()
        .map(|task| task.request_id);
    if let Some(request_id) = request_id {
        let _ = cancel_sender_pairing(request_id);
    }
}

fn run_sender_pairing(
    invitation_uri: &str,
    snapshot: &Mutex<SenderPairingSnapshot>,
    cancel_requested: &AtomicBool,
    commands: Receiver<SenderSessionCommand>,
    events: SyncSender<SenderSessionEvent>,
    session_id_slot: Arc<Mutex<Option<SessionId>>>,
) {
    if cancel_requested.load(Ordering::Acquire) {
        set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
        return;
    }

    let local_device_id = match initialize_pairing_runtime() {
        Ok(device_id) => device_id,
        Err(_) => {
            set_error_or_cancel(
                snapshot,
                PAIRING_ERROR_RUNTIME_UNAVAILABLE,
                cancel_requested,
            );
            return;
        }
    };
    let identity = match WindowsPairingIdentity::open_or_create() {
        Ok(identity) => identity,
        Err(_) => {
            set_error_or_cancel(
                snapshot,
                PAIRING_ERROR_RUNTIME_UNAVAILABLE,
                cancel_requested,
            );
            return;
        }
    };
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(now) => now,
        Err(_) => {
            set_error_or_cancel(snapshot, PAIRING_ERROR_INTERNAL, cancel_requested);
            return;
        }
    };
    let requested_grants = match TrustedPeerGrants::new(true, false, false) {
        Ok(grants) => grants,
        Err(_) => {
            set_error_or_cancel(snapshot, PAIRING_ERROR_INTERNAL, cancel_requested);
            return;
        }
    };
    let mut pending = match WssPairingClient::begin_pairing(
        invitation_uri,
        now.as_secs(),
        &[ANDROID_TV_PAIRING_PORT],
        local_device_id,
        LOCAL_DISPLAY_NAME,
        requested_grants,
        &identity,
        now.as_millis().min(u64::MAX as u128) as u64,
    ) {
        Ok(pending) => pending,
        Err(WssClientError::Rejected(_)) => {
            set_error_or_cancel(
                snapshot,
                PAIRING_ERROR_INVITATION_REJECTED,
                cancel_requested,
            );
            return;
        }
        Err(_) => {
            set_error_or_cancel(snapshot, PAIRING_ERROR_SIGNALING_FAILED, cancel_requested);
            return;
        }
    };

    if set_awaiting_approval_or_cancel(snapshot, cancel_requested) {
        pending.cancel();
        set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
        return;
    }

    loop {
        if cancel_requested.load(Ordering::Acquire) {
            pending.cancel();
            set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
            return;
        }

        match pending.try_wait_for_decision() {
            Ok(None) => {}
            Ok(Some(approved)) => {
                if cancel_requested.load(Ordering::Acquire) {
                    drop(approved);
                    set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
                    return;
                }
                if !approved.accepted_grants.allows(PeerGrant::View) {
                    drop(approved);
                    set_terminal_state(
                        snapshot,
                        SenderPairingState::Failed,
                        PAIRING_ERROR_INVALID_APPROVAL,
                    );
                    return;
                }
                let fingerprint = *approved.peer_fingerprint.as_bytes();
                let mut current = lock_recover(snapshot);
                if cancel_requested.load(Ordering::Acquire) {
                    current.state = SenderPairingState::Cancelled;
                    current.error_code = PAIRING_ERROR_NONE;
                    drop(current);
                    drop(approved.connection);
                } else {
                    current.receiver_fingerprint = fingerprint;
                    current.error_code = PAIRING_ERROR_NONE;
                    current.state = SenderPairingState::Approved;
                    drop(current);
                    run_sender_session_worker(
                        approved.connection,
                        snapshot,
                        cancel_requested,
                        commands,
                        events,
                        &session_id_slot,
                    );
                }
                return;
            }
            Err(WssClientError::Rejected(_)) => {
                set_error_or_cancel(
                    snapshot,
                    PAIRING_ERROR_INVITATION_REJECTED,
                    cancel_requested,
                );
                return;
            }
            Err(WssClientError::Closed) => {
                set_error_or_cancel(
                    snapshot,
                    PAIRING_ERROR_CONTROL_CHANNEL_CLOSED,
                    cancel_requested,
                );
                return;
            }
            Err(_) => {
                set_error_or_cancel(snapshot, PAIRING_ERROR_SIGNALING_FAILED, cancel_requested);
                return;
            }
        }
    }
}

fn set_error_or_cancel(
    snapshot: &Mutex<SenderPairingSnapshot>,
    error_code: u32,
    cancel_requested: &AtomicBool,
) {
    if cancel_requested.load(Ordering::Acquire) {
        set_terminal_state(snapshot, SenderPairingState::Cancelled, PAIRING_ERROR_NONE);
    } else {
        let state = if error_code == PAIRING_ERROR_INVITATION_REJECTED {
            SenderPairingState::Rejected
        } else {
            SenderPairingState::Failed
        };
        set_terminal_state(snapshot, state, error_code);
    }
}

/// Returns true if cancellation was requested while publishing the pending state.
fn set_awaiting_approval_or_cancel(
    snapshot: &Mutex<SenderPairingSnapshot>,
    cancel_requested: &AtomicBool,
) -> bool {
    let mut current = lock_recover(snapshot);
    if cancel_requested.load(Ordering::Acquire) {
        current.state = SenderPairingState::Cancelling;
        current.error_code = PAIRING_ERROR_NONE;
        true
    } else {
        current.state = SenderPairingState::AwaitingApproval;
        false
    }
}

fn set_terminal_state(
    snapshot: &Mutex<SenderPairingSnapshot>,
    state: SenderPairingState,
    error_code: u32,
) {
    let mut current = lock_recover(snapshot);
    current.state = state;
    current.error_code = error_code;
    if state != SenderPairingState::Approved {
        current.receiver_fingerprint = [0; 32];
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_PAIRING_INVITATION_BYTES, SenderPairingError, SenderPairingState, stop_sender_session,
        validate_invitation_uri, validate_session_source_id, validate_session_stop_reason,
    };

    #[test]
    fn accepts_a_bounded_nonempty_invitation_uri() {
        assert!(validate_invitation_uri("mirrormesh://pair?v=1").is_ok());
    }

    #[test]
    fn rejects_empty_oversized_or_control_bearing_invitation_text() {
        assert!(validate_invitation_uri("").is_err());
        assert!(validate_invitation_uri(" \t").is_err());
        assert!(validate_invitation_uri("mirrormesh://pair\n?v=1").is_err());
        assert!(validate_invitation_uri(&"x".repeat(MAX_PAIRING_INVITATION_BYTES + 1)).is_err());
    }

    #[test]
    fn rejects_outer_whitespace_so_a_pasted_uri_is_not_silently_changed() {
        assert!(validate_invitation_uri(" mirrormesh://pair?v=1").is_err());
        assert!(validate_invitation_uri("mirrormesh://pair?v=1 ").is_err());
    }

    #[test]
    fn accepts_only_bounded_control_free_session_source_ids() {
        assert!(validate_session_source_id("desktop-capture").is_ok());
        assert!(validate_session_source_id("x".repeat(128).as_str()).is_ok());
        assert!(validate_session_source_id("").is_err());
        assert!(validate_session_source_id("x".repeat(129).as_str()).is_err());
        assert!(validate_session_source_id("window\nname").is_err());
    }

    #[test]
    fn accepts_only_bounded_control_free_session_stop_reasons() {
        assert!(validate_session_stop_reason("user-disconnected").is_ok());
        assert!(validate_session_stop_reason("x".repeat(128).as_str()).is_ok());
        assert!(validate_session_stop_reason("").is_err());
        assert!(validate_session_stop_reason("x".repeat(129).as_str()).is_err());
        assert!(validate_session_stop_reason("user\nname").is_err());
    }

    #[test]
    fn stop_sender_session_validates_reason_before_pairing_lookup() {
        assert_eq!(
            stop_sender_session(0, ""),
            Err(SenderPairingError::InvalidSessionMessage)
        );
        assert_eq!(
            stop_sender_session(0, "user-disconnected"),
            Err(SenderPairingError::UnknownRequest)
        );
    }

    #[test]
    fn only_pending_or_approved_pairings_keep_the_sender_slot_active() {
        assert!(SenderPairingState::Connecting.is_active());
        assert!(SenderPairingState::AwaitingApproval.is_active());
        assert!(SenderPairingState::Approved.is_active());
        assert!(SenderPairingState::Cancelling.is_active());
        assert!(!SenderPairingState::Rejected.is_active());
        assert!(!SenderPairingState::Cancelled.is_active());
        assert!(!SenderPairingState::Failed.is_active());
    }
}
