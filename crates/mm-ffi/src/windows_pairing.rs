use std::panic::{AssertUnwindSafe, catch_unwind};

use mm_adapter_win::initialize_pairing_runtime;

const DEVICE_ID_BYTES: usize = 16;
const PAIRING_RUNTIME_OK: i32 = 0;
const PAIRING_RUNTIME_INVALID_ARGUMENT: i32 = 1;
const PAIRING_RUNTIME_UNAVAILABLE: i32 = 2;

/// Initializes Windows pairing support after an explicit user action.
///
/// # Safety
/// `device_id_out` must point to `device_id_capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_prepare_pairing_runtime(
    device_id_out: *mut u8,
    device_id_capacity: usize,
) -> i32 {
    if device_id_out.is_null() || device_id_capacity < DEVICE_ID_BYTES {
        return PAIRING_RUNTIME_INVALID_ARGUMENT;
    }

    // SAFETY: The C caller guarantees the output buffer contract documented above.
    let output = unsafe { std::slice::from_raw_parts_mut(device_id_out, DEVICE_ID_BYTES) };
    output.fill(0);

    match catch_unwind(AssertUnwindSafe(initialize_pairing_runtime)) {
        Ok(Ok(device_id)) => {
            output[..DEVICE_ID_BYTES].copy_from_slice(device_id.as_bytes());
            PAIRING_RUNTIME_OK
        }
        Ok(Err(_)) | Err(_) => PAIRING_RUNTIME_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::{PAIRING_RUNTIME_INVALID_ARGUMENT, mirrormesh_windows_prepare_pairing_runtime};

    #[test]
    fn exported_initializer_rejects_null_or_short_device_id_buffers() {
        let device_id = [0_u8; 16];
        let mut short_device_id = [0_u8; 15];

        assert_eq!(
            unsafe {
                mirrormesh_windows_prepare_pairing_runtime(std::ptr::null_mut(), device_id.len())
            },
            PAIRING_RUNTIME_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe {
                mirrormesh_windows_prepare_pairing_runtime(
                    short_device_id.as_mut_ptr(),
                    short_device_id.len(),
                )
            },
            PAIRING_RUNTIME_INVALID_ARGUMENT
        );
    }
}
