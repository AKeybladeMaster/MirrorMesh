use std::panic::{AssertUnwindSafe, catch_unwind};

use mm_adapter_win::{
    MAX_PAIRING_INVITATION_BYTES, SenderPairingError, SenderPairingSnapshot, begin_sender_pairing,
    cancel_sender_pairing, initialize_pairing_runtime, poll_sender_pairing,
    shutdown_sender_pairing,
};

const DEVICE_ID_BYTES: usize = 16;
const PAIRING_RUNTIME_OK: i32 = 0;
const PAIRING_RUNTIME_INVALID_ARGUMENT: i32 = 1;
const PAIRING_RUNTIME_UNAVAILABLE: i32 = 2;
const SENDER_PAIRING_OK: i32 = 0;
const SENDER_PAIRING_INVALID_ARGUMENT: i32 = 1;
const SENDER_PAIRING_UNAVAILABLE: i32 = 2;
const SENDER_PAIRING_BUSY: i32 = 3;
const SENDER_PAIRING_UNKNOWN_REQUEST: i32 = 4;

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

/// Starts a QR-pinned sender pairing attempt on a background worker.
///
/// # Safety
/// `invitation_uri` must point to `invitation_uri_length` readable bytes, and
/// `request_id_out` must point to one writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_begin_sender_pairing(
    invitation_uri: *const u8,
    invitation_uri_length: usize,
    request_id_out: *mut u64,
) -> i32 {
    if invitation_uri.is_null()
        || request_id_out.is_null()
        || invitation_uri_length == 0
        || invitation_uri_length > MAX_PAIRING_INVITATION_BYTES
    {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }

    // SAFETY: The C caller provides the readable byte range documented above.
    let bytes = unsafe { std::slice::from_raw_parts(invitation_uri, invitation_uri_length) };
    let Ok(invitation_uri) = std::str::from_utf8(bytes) else {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    };
    // SAFETY: The C caller provides one writable u64 as documented above.
    unsafe { request_id_out.write(0) };

    match catch_unwind(AssertUnwindSafe(|| {
        begin_sender_pairing(invitation_uri.to_owned())
    })) {
        Ok(Ok(request_id)) => {
            // SAFETY: The C caller provides one writable u64 as documented above.
            unsafe { request_id_out.write(request_id) };
            SENDER_PAIRING_OK
        }
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

/// Copies the latest bounded sender-pairing status into the caller's snapshot.
///
/// # Safety
/// `snapshot_out` must point to one writable `SenderPairingSnapshot`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_poll_sender_pairing(
    request_id: u64,
    snapshot_out: *mut SenderPairingSnapshot,
) -> i32 {
    if snapshot_out.is_null() {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }

    match catch_unwind(AssertUnwindSafe(|| poll_sender_pairing(request_id))) {
        Ok(Ok(snapshot)) => {
            // SAFETY: The C caller provides one writable snapshot as documented above.
            unsafe { snapshot_out.write(snapshot) };
            SENDER_PAIRING_OK
        }
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mirrormesh_windows_cancel_sender_pairing(request_id: u64) -> i32 {
    match catch_unwind(AssertUnwindSafe(|| cancel_sender_pairing(request_id))) {
        Ok(Ok(())) => SENDER_PAIRING_OK,
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mirrormesh_windows_shutdown_sender_pairing() {
    let _ = catch_unwind(AssertUnwindSafe(shutdown_sender_pairing));
}

fn sender_pairing_error_code(error: SenderPairingError) -> i32 {
    match error {
        SenderPairingError::InvalidInvitation => SENDER_PAIRING_INVALID_ARGUMENT,
        SenderPairingError::AlreadyActive => SENDER_PAIRING_BUSY,
        SenderPairingError::UnknownRequest => SENDER_PAIRING_UNKNOWN_REQUEST,
        SenderPairingError::Unavailable => SENDER_PAIRING_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PAIRING_RUNTIME_INVALID_ARGUMENT, SENDER_PAIRING_INVALID_ARGUMENT,
        SENDER_PAIRING_UNKNOWN_REQUEST, mirrormesh_windows_begin_sender_pairing,
        mirrormesh_windows_cancel_sender_pairing, mirrormesh_windows_poll_sender_pairing,
        mirrormesh_windows_prepare_pairing_runtime,
    };

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

    #[test]
    fn exported_sender_pairing_rejects_invalid_input_without_starting_a_worker() {
        let mut request_id = 99_u64;
        let empty = [];

        assert_eq!(
            unsafe {
                mirrormesh_windows_begin_sender_pairing(
                    empty.as_ptr(),
                    empty.len(),
                    &mut request_id,
                )
            },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        assert_eq!(request_id, 99);
    }

    #[test]
    fn exported_sender_pairing_reports_unknown_request_ids() {
        let mut snapshot = std::mem::MaybeUninit::uninit();
        assert_eq!(
            unsafe { mirrormesh_windows_poll_sender_pairing(0, snapshot.as_mut_ptr()) },
            SENDER_PAIRING_UNKNOWN_REQUEST
        );
        assert_eq!(
            mirrormesh_windows_cancel_sender_pairing(0),
            SENDER_PAIRING_UNKNOWN_REQUEST
        );
    }
}
