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
    use std::sync::{Arc, Mutex, OnceLock};

    use mm_domain::DeviceId;
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
    use mm_runtime::PairingRuntime;
    use mm_store::SqliteTrustedPeers;

    use crate::AndroidPairingIdentity;

    use super::device_id_from_fingerprint;

    const TRUSTED_PEERS_DATABASE: &str = "mirrormesh-trusted-peers-v1.sqlite";

    type AndroidPairingRuntime = PairingRuntime<SqliteTrustedPeers, AndroidPairingIdentity>;

    static ANDROID_PAIRING_RUNTIME: OnceLock<Mutex<Option<(DeviceId, AndroidPairingRuntime)>>> =
        OnceLock::new();

    pub(super) fn initialize_pairing_runtime() -> Result<DeviceId, PortError> {
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
            let pairing_runtime = PairingRuntime::new(device_id, identity, trusted_peers)?;
            Ok((device_id, pairing_runtime))
        })
    }
}

#[cfg(target_os = "android")]
pub(super) use android::initialize_pairing_runtime;

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use mm_domain::{DeviceId, PeerFingerprint};
    use mm_ports::PortError;

    use super::{device_id_from_fingerprint, initialize_once};

    #[test]
    fn local_device_id_is_stably_derived_from_the_public_fingerprint() {
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
        assert_eq!(device_id_from_fingerprint(fingerprint), expected);
        assert_eq!(expected.to_string(), "10213243-5465-8687-98a9-bacbdcedfe0f");
    }

    #[test]
    fn process_runtime_initializer_runs_once_and_returns_the_same_device_id() {
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
    fn failed_process_runtime_initialization_can_be_retried() {
        static RUNTIME: OnceLock<Mutex<Option<(DeviceId, u8)>>> = OnceLock::new();
        let expected_id = DeviceId::from_bytes([9; 16]);

        let first = initialize_once(&RUNTIME, || Err(PortError::StorageFailure));
        assert_eq!(first, Err(PortError::StorageFailure));

        let retry = initialize_once(&RUNTIME, || Ok((expected_id, 1))).unwrap();
        assert_eq!(retry, expected_id);
    }
}
