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
    Ok(device_id)
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

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use mm_domain::{DeviceId, PeerFingerprint};

    use super::device_id_from_fingerprint;

    #[test]
    fn device_id_is_uuid_v8_shaped_and_stably_derived_from_the_public_key_fingerprint() {
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
    fn exported_initializer_retains_one_runtime_and_returns_the_same_device_id() {
        let memory_database = CString::new(":memory:").unwrap();
        let second_database = CString::new("ignored-after-first-initialization.sqlite").unwrap();
        let mut first_id = [0_u8; 16];
        let mut second_id = [0_u8; 16];

        let first_status = unsafe {
            super::mirrormesh_ios_prepare_pairing_runtime(
                memory_database.as_ptr(),
                first_id.as_mut_ptr(),
                first_id.len(),
            )
        };
        let second_status = unsafe {
            super::mirrormesh_ios_prepare_pairing_runtime(
                second_database.as_ptr(),
                second_id.as_mut_ptr(),
                second_id.len(),
            )
        };

        assert_eq!(first_status, super::PAIRING_RUNTIME_OK);
        assert_eq!(second_status, super::PAIRING_RUNTIME_OK);
        assert_eq!(first_id, second_id);
        assert_eq!(first_id[6] >> 4, 8);
        assert_eq!(first_id[8] & 0xc0, 0x80);
    }

    #[test]
    fn exported_initializer_rejects_invalid_pointers_and_buffer_sizes() {
        let memory_database = CString::new(":memory:").unwrap();
        let mut short_id = [0_u8; 15];

        assert_eq!(
            unsafe {
                super::mirrormesh_ios_prepare_pairing_runtime(
                    std::ptr::null(),
                    short_id.as_mut_ptr(),
                    short_id.len(),
                )
            },
            super::PAIRING_RUNTIME_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe {
                super::mirrormesh_ios_prepare_pairing_runtime(
                    memory_database.as_ptr(),
                    short_id.as_mut_ptr(),
                    short_id.len(),
                )
            },
            super::PAIRING_RUNTIME_INVALID_ARGUMENT
        );
    }
}
