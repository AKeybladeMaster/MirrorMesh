use std::panic::{AssertUnwindSafe, catch_unwind};

use mm_adapter_linux::initialize_pairing_runtime;

const DEVICE_ID_BYTES: usize = 16;
const PAIRING_RUNTIME_OK: i32 = 0;
const PAIRING_RUNTIME_INVALID_ARGUMENT: i32 = 1;
const PAIRING_RUNTIME_UNAVAILABLE: i32 = 2;

/// Initializes Linux pairing support after an explicit user action.
///
/// # Safety
/// `device_id_out` must point to `device_id_capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_linux_prepare_pairing_runtime(
    device_id_out: *mut u8,
    device_id_capacity: usize,
) -> i32 {
    if device_id_out.is_null() || device_id_capacity < DEVICE_ID_BYTES {
        return PAIRING_RUNTIME_INVALID_ARGUMENT;
    }

    // SAFETY: The native caller guarantees the output buffer contract documented above.
    let output = unsafe { std::slice::from_raw_parts_mut(device_id_out, DEVICE_ID_BYTES) };
    output.fill(0);

    match catch_unwind(AssertUnwindSafe(initialize_pairing_runtime)) {
        Ok(Ok(device_id)) => {
            output.copy_from_slice(device_id.as_bytes());
            PAIRING_RUNTIME_OK
        }
        Ok(Err(_)) | Err(_) => PAIRING_RUNTIME_UNAVAILABLE,
    }
}
