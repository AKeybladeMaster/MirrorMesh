use std::ffi::CStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use mm_domain::{DeviceId, PeerFingerprint};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
use mm_runtime::PairingRuntime;
use mm_store::SqliteTrustedPeers;

use crate::IosPairingIdentity;

const DEVICE_ID_BYTES: usize = 16;
const PAIRING_RUNTIME_OK: i32 = 0;
const PAIRING_RUNTIME_INVALID_ARGUMENT: i32 = 1;
const PAIRING_RUNTIME_UNAVAILABLE: i32 = 2;

type IosRuntime = PairingRuntime<SqliteTrustedPeers, IosPairingIdentity>;

static IOS_PAIRING_RUNTIME: OnceLock<Mutex<Option<(DeviceId, IosRuntime)>>> = OnceLock::new();
static IOS_DATABASE_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

fn device_id_from_fingerprint(fingerprint: PeerFingerprint) -> DeviceId {
    let mut id = [0_u8; DEVICE_ID_BYTES];
    id.copy_from_slice(&fingerprint.as_bytes()[..DEVICE_ID_BYTES]);
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    DeviceId::from_bytes(id)
}

fn initialize_pairing_runtime(database_path: &Path) -> Result<DeviceId, PortError> {
    let mut runtime_slot = IOS_PAIRING_RUNTIME
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| PortError::StorageFailure)?;
    if let Some((device_id, _)) = runtime_slot.as_ref() {
        return Ok(*device_id);
    }

    let identity = Arc::new(IosPairingIdentity::open_or_create()?);
    let public_key = identity.local_public_key()?;
    let fingerprint = identity.fingerprint_public_key(&public_key)?;
    let device_id = device_id_from_fingerprint(fingerprint);
    let trusted_peers =
        Arc::new(SqliteTrustedPeers::open(database_path).map_err(|_| PortError::StorageFailure)?);
    let pairing_runtime = PairingRuntime::new(device_id, identity, trusted_peers)?;
    *runtime_slot = Some((device_id, pairing_runtime));
    let _ = IOS_DATABASE_PATH.set(database_path.to_path_buf());
    Ok(device_id)
}

pub fn local_device_id() -> Result<DeviceId, PortError> {
    IOS_PAIRING_RUNTIME
        .get()
        .ok_or(PortError::Unavailable)?
        .lock()
        .map_err(|_| PortError::StorageFailure)?
        .as_ref()
        .map(|(id, _)| *id)
        .ok_or(PortError::Unavailable)
}

pub fn create_pairing_service(
    address: std::net::SocketAddr,
) -> Result<
    mm_adapter_signaling::native_service::NativePairingService<
        SqliteTrustedPeers,
        IosPairingIdentity,
    >,
    mm_adapter_signaling::native_service::NativePairingServiceError,
> {
    use mm_adapter_signaling::native_service::{NativePairingService, NativePairingServiceError};
    let id = local_device_id().map_err(NativePairingServiceError::Platform)?;
    let identity = Arc::new(
        IosPairingIdentity::open_or_create().map_err(NativePairingServiceError::Platform)?,
    );
    let path = IOS_DATABASE_PATH
        .get()
        .ok_or(NativePairingServiceError::Platform(PortError::Unavailable))?;
    let peers = Arc::new(
        SqliteTrustedPeers::open(path)
            .map_err(|_| NativePairingServiceError::Platform(PortError::StorageFailure))?,
    );
    let runtime = PairingRuntime::new(id, Arc::clone(&identity), peers)
        .map_err(NativePairingServiceError::Platform)?;
    NativePairingService::bind(address, runtime, identity)
}

pub fn revoke_trusted_peer(id: DeviceId, now: u64) -> Result<(), PortError> {
    use mm_ports::TrustedPeerPort;
    let path = IOS_DATABASE_PATH.get().ok_or(PortError::Unavailable)?;
    SqliteTrustedPeers::open(path)
        .map_err(|_| PortError::StorageFailure)?
        .revoke(id, now)
}

/// Initializes local iOS pairing state after the Flutter UI's explicit user action.
///
/// # Safety
/// `database_path` must point to a readable NUL-terminated UTF-8 string and
/// `device_id_out` must point to `device_id_capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_ios_prepare_pairing_runtime(
    database_path: *const std::ffi::c_char,
    device_id_out: *mut u8,
    device_id_capacity: usize,
) -> i32 {
    if database_path.is_null() || device_id_out.is_null() || device_id_capacity < DEVICE_ID_BYTES {
        return PAIRING_RUNTIME_INVALID_ARGUMENT;
    }

    // SAFETY: The C caller guarantees both pointer contracts documented above.
    let output = unsafe { std::slice::from_raw_parts_mut(device_id_out, device_id_capacity) };
    output.fill(0);

    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The C caller guarantees a readable NUL-terminated path.
        let path = unsafe { CStr::from_ptr(database_path) }
            .to_str()
            .map_err(|_| PortError::InvalidConfiguration)?;
        initialize_pairing_runtime(Path::new(path))
    }));

    match result {
        Ok(Ok(device_id)) => {
            output[..DEVICE_ID_BYTES].copy_from_slice(device_id.as_bytes());
            PAIRING_RUNTIME_OK
        }
        Ok(Err(_)) | Err(_) => PAIRING_RUNTIME_UNAVAILABLE,
    }
}
