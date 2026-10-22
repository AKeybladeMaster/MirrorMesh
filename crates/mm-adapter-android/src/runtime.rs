use mm_domain::{DeviceId, PeerFingerprint};
use mm_ports::PortError;
use std::sync::{Mutex, OnceLock};

fn device_id_from_fingerprint(fingerprint: PeerFingerprint) -> DeviceId {
    let mut id = [0_u8; 16];
    id.copy_from_slice(&fingerprint.as_bytes()[..16]);
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    DeviceId::from_bytes(id)
}

fn initialize_once<R>(
    slot: &OnceLock<Mutex<Option<(DeviceId, R)>>>,
    initialize: impl FnOnce() -> Result<(DeviceId, R), PortError>,
) -> Result<DeviceId, PortError> {
    let mut runtime_slot = slot
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| PortError::StorageFailure)?;
    if let Some((device_id, _)) = runtime_slot.as_ref() {
        return Ok(*device_id);
    }

    let (device_id, runtime) = initialize()?;
    *runtime_slot = Some((device_id, runtime));
    Ok(device_id)
}

#[cfg(target_os = "android")]
mod android {
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex, OnceLock};

    use mm_domain::DeviceId;
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
    use mm_runtime::PairingRuntime;
    use mm_store::SqliteTrustedPeers;

    use crate::{AndroidPairingIdentity, AndroidTvPairingService, AndroidTvPairingServiceError};

    use super::device_id_from_fingerprint;

    const TRUSTED_PEERS_DATABASE: &str = "mirrormesh-trusted-peers-v1.sqlite";

    type AndroidPairingRuntime = PairingRuntime<SqliteTrustedPeers, AndroidPairingIdentity>;
    #[derive(Clone)]
    struct AndroidPairingOwner {
        identity: Arc<AndroidPairingIdentity>,
        runtime: Arc<Mutex<AndroidPairingRuntime>>,
        peers: Arc<SqliteTrustedPeers>,
    }

    static ANDROID_PAIRING_RUNTIME: OnceLock<Mutex<Option<(DeviceId, AndroidPairingOwner)>>> =
        OnceLock::new();

    pub fn initialize_pairing_runtime() -> Result<DeviceId, PortError> {
        super::initialize_once(&ANDROID_PAIRING_RUNTIME, || {
            let directory = crate::android_store::android_data_directory()?;
            let identity = Arc::new(AndroidPairingIdentity::open_or_create()?);
            let public_key = identity.local_public_key()?;
            let fingerprint = identity.fingerprint_public_key(&public_key)?;
            let device_id = device_id_from_fingerprint(fingerprint);
            let trusted_peers = Arc::new(
                SqliteTrustedPeers::open(directory.join(TRUSTED_PEERS_DATABASE))
                    .map_err(|_| PortError::StorageFailure)?,
            );
            let pairing_runtime =
                PairingRuntime::new(device_id, Arc::clone(&identity), Arc::clone(&trusted_peers))?;
            Ok((
                device_id,
                AndroidPairingOwner {
                    identity,
                    runtime: Arc::new(Mutex::new(pairing_runtime)),
                    peers: trusted_peers,
                },
            ))
        })
    }

    pub fn create_tv_pairing_service(
        address: SocketAddr,
    ) -> Result<
        AndroidTvPairingService<SqliteTrustedPeers, AndroidPairingIdentity>,
        AndroidTvPairingServiceError,
    > {
        initialize_pairing_runtime().map_err(AndroidTvPairingServiceError::Platform)?;
        let owner = ANDROID_PAIRING_RUNTIME
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| AndroidTvPairingServiceError::Platform(PortError::StorageFailure))?
            .as_ref()
            .map(|(_, owner)| owner.clone())
            .ok_or(AndroidTvPairingServiceError::Platform(
                PortError::Unavailable,
            ))?;
        AndroidTvPairingService::bind_shared(address, owner.runtime, owner.identity)
    }

    pub fn revoke_trusted_peer(peer_id: DeviceId, now_unix_ms: u64) -> Result<(), PortError> {
        let peers = ANDROID_PAIRING_RUNTIME
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| PortError::StorageFailure)?
            .as_ref()
            .map(|(_, owner)| Arc::clone(&owner.peers))
            .ok_or(PortError::Unavailable)?;
        peers.revoke(peer_id, now_unix_ms)
    }
}

#[cfg(target_os = "android")]
pub use android::{create_tv_pairing_service, initialize_pairing_runtime, revoke_trusted_peer};
