#[cfg(windows)]
mod identity;

#[cfg(windows)]
mod runtime;

#[cfg(windows)]
mod sender_pairing;

#[cfg(windows)]
pub use identity::WindowsPairingIdentity;

#[cfg(windows)]
pub use sender_pairing::{
    MAX_PAIRING_INVITATION_BYTES, SenderPairingError, SenderPairingSnapshot, SenderPairingState,
    SenderSessionEvent, begin_sender_pairing, begin_sender_session, cancel_sender_pairing,
    poll_sender_pairing, poll_sender_session_event, send_sender_session_ice,
    shutdown_sender_pairing, start_sender_session,
};

#[cfg(windows)]
pub use runtime::initialize_pairing_runtime;

#[cfg(all(test, windows))]
mod tests;
