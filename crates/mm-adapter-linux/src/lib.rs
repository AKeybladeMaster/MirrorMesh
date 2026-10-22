mod identity;

#[cfg(target_os = "linux")]
mod runtime;

#[cfg(target_os = "linux")]
pub use identity::LinuxPairingIdentity;

#[cfg(target_os = "linux")]
pub use runtime::{create_pairing_service, initialize_pairing_runtime, revoke_trusted_peer};
