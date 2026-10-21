#[cfg(target_os = "ios")]
mod identity;

#[cfg(target_os = "ios")]
mod keychain;

#[cfg(target_os = "ios")]
pub use keychain::IosPairingIdentity;

#[cfg(target_os = "ios")]
mod runtime;
#[cfg(target_os = "ios")]
pub use runtime::{create_pairing_service, local_device_id, revoke_trusted_peer};
