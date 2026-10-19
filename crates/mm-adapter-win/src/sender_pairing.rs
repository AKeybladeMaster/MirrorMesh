use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use mm_adapter_signaling::{PairingConnection, WssClientError, WssPairingClient};
use mm_domain::{PeerGrant, TrustedPeerGrants};

use crate::{WindowsPairingIdentity, initialize_pairing_runtime};

pub const MAX_PAIRING_INVITATION_BYTES: usize = 4096;
const ANDROID_TV_PAIRING_PORT: u16 = 8765;
const LOCAL_DISPLAY_NAME: &str = "MirrorMesh Windows PC";

const PAIRING_ERROR_NONE: u32 = 0;
const PAIRING_ERROR_RUNTIME_UNAVAILABLE: u32 = 1;
const PAIRING_ERROR_INVITATION_REJECTED: u32 = 2;
const PAIRING_ERROR_SIGNALING_FAILED: u32 = 3;
const PAIRING_ERROR_CONTROL_CHANNEL_CLOSED: u32 = 4;
const PAIRING_ERROR_INVALID_APPROVAL: u32 = 5;
const PAIRING_ERROR_INTERNAL: u32 = 6;

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
    AlreadyActive,
    UnknownRequest,
    Unavailable,
}

struct SenderPairingTask {
    request_id: u64,
    snapshot: Arc<Mutex<SenderPairingSnapshot>>,
    cancel_requested: Arc<AtomicBool>,
    approved_connection: Arc<Mutex<Option<PairingConnection>>>,
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
    let approved_connection = Arc::new(Mutex::new(None));

    let worker_snapshot = Arc::clone(&snapshot);
    let worker_cancel_requested = Arc::clone(&cancel_requested);
    let worker_connection = Arc::clone(&approved_connection);
    std::thread::Builder::new()
        .name("mirrormesh-windows-pairing".to_owned())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                run_sender_pairing(
                    &invitation_uri,
                    &worker_snapshot,
                    &worker_cancel_requested,
                    &worker_connection,
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
        approved_connection,
    });
    Ok(request_id)
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
            drop(lock_recover(&task.approved_connection).take());
            set_terminal_state(
                &task.snapshot,
                SenderPairingState::Cancelled,
                PAIRING_ERROR_NONE,
            );
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
    approved_connection: &Mutex<Option<PairingConnection>>,
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
                *lock_recover(approved_connection) = Some(approved.connection);
                let mut current = lock_recover(snapshot);
                if cancel_requested.load(Ordering::Acquire) {
                    current.state = SenderPairingState::Cancelled;
                    current.error_code = PAIRING_ERROR_NONE;
                    drop(current);
                    drop(lock_recover(approved_connection).take());
                } else {
                    current.receiver_fingerprint = fingerprint;
                    current.error_code = PAIRING_ERROR_NONE;
                    current.state = SenderPairingState::Approved;
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
    use super::{MAX_PAIRING_INVITATION_BYTES, SenderPairingState, validate_invitation_uri};

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
