#![cfg(any(test, target_os = "android"))]

mod identity;
mod pairing_service;
mod runtime;

#[cfg(any(test, target_os = "android"))]
mod android_store;

#[cfg(target_os = "android")]
pub use android_store::AndroidPairingIdentity;

#[cfg(target_os = "android")]
pub use runtime::{create_tv_pairing_service, initialize_pairing_runtime};

pub use pairing_service::{
    AndroidTvPairingRequest, AndroidTvPairingService, AndroidTvPairingServiceError,
    PendingAndroidTvPairing,
};

#[cfg(target_os = "android")]
pub type AndroidTvPairingServiceHandle =
    AndroidTvPairingService<mm_store::SqliteTrustedPeers, AndroidPairingIdentity>;

#[cfg(test)]
mod tests;
