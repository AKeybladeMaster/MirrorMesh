use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use mm_domain::{DeviceId, PeerFingerprint};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
use mm_runtime::PairingRuntime;
use mm_store::SqliteTrustedPeers;

use crate::WindowsPairingIdentity;

const TRUSTED_PEERS_DATABASE: &str = "trusted-peers-v1.sqlite";

type WindowsPairingRuntime = PairingRuntime<SqliteTrustedPeers, WindowsPairingIdentity>;

static WINDOWS_PAIRING_RUNTIME: OnceLock<Mutex<Option<(DeviceId, WindowsPairingRuntime)>>> =
    OnceLock::new();

fn device_id_from_fingerprint(fingerprint: PeerFingerprint) -> DeviceId {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&fingerprint.as_bytes()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    DeviceId::from_bytes(bytes)
}

fn trusted_peer_database_path(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("MirrorMesh")
        .join(TRUSTED_PEERS_DATABASE)
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

/// Opens the per-user pairing identity and SQLite peer store after explicit user action.
pub fn initialize_pairing_runtime() -> Result<DeviceId, PortError> {
    initialize_once(&WINDOWS_PAIRING_RUNTIME, || {
        let local_app_data = env::var_os("LOCALAPPDATA").ok_or(PortError::Unavailable)?;
        let database_path = trusted_peer_database_path(Path::new(&local_app_data));
        let database_directory = database_path
            .parent()
            .ok_or(PortError::InvalidConfiguration)?;
        fs::create_dir_all(database_directory).map_err(|_| PortError::StorageFailure)?;

        let trusted_peers = Arc::new(
            SqliteTrustedPeers::open(&database_path).map_err(|_| PortError::StorageFailure)?,
        );
        let identity = Arc::new(WindowsPairingIdentity::open_or_create()?);
        let public_key = identity.local_public_key()?;
        let fingerprint = identity.fingerprint_public_key(&public_key)?;
        let device_id = device_id_from_fingerprint(fingerprint);
        let runtime = PairingRuntime::new(device_id, identity, trusted_peers)?;
        Ok((device_id, runtime))
    })
}
