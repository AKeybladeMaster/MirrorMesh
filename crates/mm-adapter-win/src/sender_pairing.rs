use crate::{WindowsPairingIdentity, initialize_pairing_runtime};
pub use mm_adapter_signaling::sender_driver::{
    MAX_PAIRING_INVITATION_BYTES, MAX_SESSION_STOP_REASON_BYTES, SenderPairingError,
    SenderPairingSnapshot, SenderPairingState, SenderSessionEvent, begin_sender_session,
    cancel_sender_pairing, poll_sender_pairing, poll_sender_session_event, send_sender_session_ice,
    shutdown_sender_pairing, start_sender_session, stop_sender_session,
};
use std::sync::Arc;

/// Opens the Windows protected identity, then delegates to the common native sender driver.
pub fn begin_sender_pairing(invitation_uri: String) -> Result<u64, SenderPairingError> {
    let id = initialize_pairing_runtime().map_err(|_| SenderPairingError::Unavailable)?;
    let identity =
        WindowsPairingIdentity::open_or_create().map_err(|_| SenderPairingError::Unavailable)?;
    mm_adapter_signaling::sender_driver::begin_sender_pairing(
        invitation_uri,
        id,
        "MirrorMesh Windows PC".to_owned(),
        Arc::new(identity),
    )
}
