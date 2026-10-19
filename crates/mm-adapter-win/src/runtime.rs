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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use mm_domain::{DeviceId, PeerFingerprint};

    use super::{device_id_from_fingerprint, initialize_once, trusted_peer_database_path};

    #[test]
    fn device_id_is_stably_derived_as_uuid_v8_from_the_local_fingerprint() {
        let fingerprint = PeerFingerprint::from_bytes([
            0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98, 0xa9, 0xba, 0xcb, 0xdc, 0xed,
            0xfe, 0x0f, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff, 0x00,
        ]);
        let expected = DeviceId::from_bytes([
            0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x86, 0x87, 0x98, 0xa9, 0xba, 0xcb, 0xdc, 0xed,
            0xfe, 0x0f,
        ]);

        assert_eq!(device_id_from_fingerprint(fingerprint), expected);
        assert_eq!(expected.to_string(), "10213243-5465-8687-98a9-bacbdcedfe0f");
    }

    #[test]
    fn process_initializer_retains_one_runtime_and_returns_the_same_id() {
        static RUNTIME: OnceLock<Mutex<Option<(DeviceId, u8)>>> = OnceLock::new();
        let calls = AtomicUsize::new(0);
        let expected_id = DeviceId::from_bytes([7; 16]);

        let first = initialize_once(&RUNTIME, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok((expected_id, 1))
        })
        .unwrap();
        let second = initialize_once(&RUNTIME, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok((DeviceId::from_bytes([8; 16]), 2))
        })
        .unwrap();

        assert_eq!(first, expected_id);
        assert_eq!(second, expected_id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_initialization_can_be_retried_without_retaining_partial_state() {
        static RUNTIME: OnceLock<Mutex<Option<(DeviceId, u8)>>> = OnceLock::new();
        let expected_id = DeviceId::from_bytes([9; 16]);

        assert!(initialize_once(&RUNTIME, || Err(mm_ports::PortError::Unavailable)).is_err());
        assert_eq!(
            initialize_once(&RUNTIME, || Ok((expected_id, 1))).unwrap(),
            expected_id
        );
    }

    #[test]
    fn trusted_peer_database_uses_a_fixed_file_under_the_user_local_data_directory() {
        let base = Path::new(r"C:\Users\MirrorMesh\AppData\Local");

        assert_eq!(
            trusted_peer_database_path(base),
            PathBuf::from(r"C:\Users\MirrorMesh\AppData\Local\MirrorMesh\trusted-peers-v1.sqlite")
        );
    }
}
