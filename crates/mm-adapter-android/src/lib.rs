#![cfg(target_os = "android")]

mod identity;
mod runtime;

#[cfg(target_os = "android")]
mod android_store;

#[cfg(target_os = "android")]
pub use android_store::AndroidPairingIdentity;

#[cfg(target_os = "android")]
pub use runtime::{create_tv_pairing_service, initialize_pairing_runtime, revoke_trusted_peer};

pub use mm_adapter_signaling::native_service::{
    NativePairingRequest as AndroidTvPairingRequest,
    NativePairingService as AndroidTvPairingService,
    NativePairingServiceError as AndroidTvPairingServiceError,
    PendingNativePairing as PendingAndroidTvPairing,
};

#[cfg(target_os = "android")]
pub type AndroidTvPairingServiceHandle =
    AndroidTvPairingService<mm_store::SqliteTrustedPeers, AndroidPairingIdentity>;
