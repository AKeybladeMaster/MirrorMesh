#[cfg(any(target_os = "ios", target_os = "macos"))]
mod identity;

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod keychain;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use keychain::IosPairingIdentity;

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod runtime;
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use runtime::{create_pairing_service, local_device_id, revoke_trusted_peer};
