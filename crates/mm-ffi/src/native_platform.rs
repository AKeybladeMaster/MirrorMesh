use mm_domain::DeviceId;
use mm_ports::{LocalPairingIdentityPort, PortError};
use std::sync::Arc;

#[cfg(target_os = "android")]
pub use mm_adapter_android::create_tv_pairing_service as create_platform_service;
#[cfg(target_os = "android")]
use mm_adapter_android::{AndroidPairingIdentity as Identity, initialize_pairing_runtime};
#[cfg(target_os = "ios")]
pub use mm_adapter_ios::create_pairing_service as create_platform_service;
#[cfg(target_os = "ios")]
use mm_adapter_ios::{
    IosPairingIdentity as Identity, local_device_id as initialize_pairing_runtime,
};
#[cfg(target_os = "windows")]
pub use mm_adapter_win::create_pairing_service as create_platform_service;
#[cfg(target_os = "windows")]
use mm_adapter_win::{WindowsPairingIdentity as Identity, initialize_pairing_runtime};

pub type PlatformService = mm_adapter_signaling::native_service::NativePairingService<
    mm_store::SqliteTrustedPeers,
    Identity,
>;

pub fn local_device_id() -> Result<DeviceId, PortError> {
    initialize_pairing_runtime()
}
pub fn sender_identity() -> Result<(DeviceId, Arc<dyn LocalPairingIdentityPort>), PortError> {
    Ok((local_device_id()?, Arc::new(Identity::open_or_create()?)))
}

pub fn revoke_peer(id: DeviceId, now: u64) -> Result<(), PortError> {
    #[cfg(target_os = "android")]
    {
        mm_adapter_android::revoke_trusted_peer(id, now)
    }
    #[cfg(target_os = "windows")]
    {
        mm_adapter_win::revoke_trusted_peer(id, now)
    }
    #[cfg(target_os = "ios")]
    {
        mm_adapter_ios::revoke_trusted_peer(id, now)
    }
}
