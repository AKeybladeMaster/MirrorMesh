use std::panic::{AssertUnwindSafe, catch_unwind};

use mm_adapter_win::{
    MAX_PAIRING_INVITATION_BYTES, SenderPairingError, SenderPairingSnapshot, SenderSessionEvent,
    begin_sender_pairing, begin_sender_session, cancel_sender_pairing, initialize_pairing_runtime,
    poll_sender_pairing, poll_sender_session_event, send_sender_session_ice,
    shutdown_sender_pairing, start_sender_session,
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
const SENDER_PAIRING_INVALID_STATE: i32 = 5;
const SENDER_PAIRING_CONTROL_FAILED: i32 = 6;
const MAX_SESSION_EVENT_JSON_BYTES: usize = 256 * 1024;
const MAX_SESSION_SOURCE_ID_BYTES: usize = 128;

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

/// Starts an H.264 sender session on the approved pairing channel.
///
/// # Safety
/// `sdp` must point to `sdp_length` readable bytes. `session_id_out` must point to at least
/// 36 writable bytes and `session_id_length_out` to one writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_begin_sender_session(
    request_id: u64,
    sdp: *const u8,
    sdp_length: usize,
    session_id_out: *mut u8,
    session_id_capacity: usize,
    session_id_length_out: *mut usize,
) -> i32 {
    if sdp.is_null()
        || sdp_length == 0
        || session_id_out.is_null()
        || session_id_capacity < 36
        || session_id_length_out.is_null()
    {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }

    // SAFETY: The C caller provides the readable byte range documented above.
    let bytes = unsafe { std::slice::from_raw_parts(sdp, sdp_length) };
    let Ok(sdp) = std::str::from_utf8(bytes) else {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    };
    match catch_unwind(AssertUnwindSafe(|| begin_sender_session(request_id, sdp))) {
        Ok(Ok(session_id)) => {
            let session_id = session_id.to_string();
            let session_id_bytes = session_id.as_bytes();
            // SAFETY: Both output ranges are checked and documented above.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    session_id_bytes.as_ptr(),
                    session_id_out,
                    session_id_bytes.len(),
                );
                session_id_length_out.write(session_id_bytes.len());
            }
            SENDER_PAIRING_OK
        }
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

/// Sends one bounded trickled ICE candidate on an active sender session.
///
/// # Safety
/// `media_id` and `candidate` must point to their respective readable byte ranges.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_send_sender_session_ice(
    request_id: u64,
    media_id: *const u8,
    media_id_length: usize,
    media_line_index: u32,
    candidate: *const u8,
    candidate_length: usize,
) -> i32 {
    if media_id.is_null()
        || media_id_length == 0
        || media_id_length > 256
        || candidate.is_null()
        || candidate_length == 0
        || candidate_length > 4 * 1024
        || media_line_index > 1024
    {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }
    // SAFETY: The C caller provides the readable byte ranges documented above.
    let media_id = unsafe { std::slice::from_raw_parts(media_id, media_id_length) };
    // SAFETY: The C caller provides the readable byte ranges documented above.
    let candidate = unsafe { std::slice::from_raw_parts(candidate, candidate_length) };
    let (Ok(media_id), Ok(candidate)) = (
        std::str::from_utf8(media_id),
        std::str::from_utf8(candidate),
    ) else {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        send_sender_session_ice(request_id, candidate, media_id, media_line_index as u16)
    })) {
        Ok(Ok(())) => SENDER_PAIRING_OK,
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

/// Sends the selected, privacy-safe source identifier after WebRTC connects.
///
/// # Safety
/// `source_id` must point to `source_id_length` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_start_sender_session(
    request_id: u64,
    source_id: *const u8,
    source_id_length: usize,
) -> i32 {
    if source_id.is_null()
        || source_id_length == 0
        || source_id_length > MAX_SESSION_SOURCE_ID_BYTES
    {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }
    // SAFETY: The C caller provides the readable byte range documented above.
    let source_id = unsafe { std::slice::from_raw_parts(source_id, source_id_length) };
    let Ok(source_id) = std::str::from_utf8(source_id) else {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        start_sender_session(request_id, source_id)
    })) {
        Ok(Ok(())) => SENDER_PAIRING_OK,
        Ok(Err(error)) => sender_pairing_error_code(error),
        Err(_) => SENDER_PAIRING_UNAVAILABLE,
    }
}

/// Polls one bounded response from the TV over the approved control channel as JSON.
///
/// # Safety
/// `event_json_out` must point to `event_json_capacity` writable bytes, and
/// `event_json_length_out` must point to one writable `usize`. Capacity must be at least
/// `MAX_SESSION_EVENT_JSON_BYTES` so a received event is never discarded on a short buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_windows_poll_sender_session_event(
    request_id: u64,
    event_json_out: *mut u8,
    event_json_capacity: usize,
    event_json_length_out: *mut usize,
) -> i32 {
    if event_json_out.is_null()
        || event_json_capacity < MAX_SESSION_EVENT_JSON_BYTES
        || event_json_length_out.is_null()
    {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }
    let event = match catch_unwind(AssertUnwindSafe(|| poll_sender_session_event(request_id))) {
        Ok(Ok(event)) => event,
        Ok(Err(error)) => return sender_pairing_error_code(error),
        Err(_) => return SENDER_PAIRING_UNAVAILABLE,
    };
    let value = match event {
        SenderSessionEvent::Pending => serde_json::json!({ "pending": true }),
        SenderSessionEvent::Answer { session_id, sdp } => serde_json::json!({
            "pending": false,
            "kind": "answer",
            "sessionId": session_id,
            "sdp": sdp
        }),
        SenderSessionEvent::IceCandidate {
            session_id,
            candidate,
            media_id,
            media_line_index,
        } => serde_json::json!({
            "pending": false,
            "kind": "ice",
            "sessionId": session_id,
            "candidate": candidate,
            "mediaId": media_id,
            "mediaLineIndex": media_line_index
        }),
        SenderSessionEvent::Stopped { session_id, reason } => serde_json::json!({
            "pending": false,
            "kind": "stop",
            "sessionId": session_id,
            "reason": reason
        }),
        SenderSessionEvent::Revoked { peer_id } => serde_json::json!({
            "pending": false,
            "kind": "revoke",
            "peerId": peer_id
        }),
    };
    let Ok(bytes) = serde_json::to_vec(&value) else {
        return SENDER_PAIRING_UNAVAILABLE;
    };
    if bytes.len() > event_json_capacity {
        return SENDER_PAIRING_INVALID_ARGUMENT;
    }
    // SAFETY: The output buffer capacity was checked before receiving the event.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), event_json_out, bytes.len());
        event_json_length_out.write(bytes.len());
    }
    SENDER_PAIRING_OK
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
        SenderPairingError::InvalidInvitation | SenderPairingError::InvalidSessionMessage => {
            SENDER_PAIRING_INVALID_ARGUMENT
        }
        SenderPairingError::AlreadyActive | SenderPairingError::SessionAlreadyStarted => {
            SENDER_PAIRING_BUSY
        }
        SenderPairingError::SessionNotStarted => SENDER_PAIRING_INVALID_STATE,
        SenderPairingError::UnknownRequest => SENDER_PAIRING_UNKNOWN_REQUEST,
        SenderPairingError::ControlFailed => SENDER_PAIRING_CONTROL_FAILED,
        SenderPairingError::Unavailable => SENDER_PAIRING_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_SESSION_EVENT_JSON_BYTES, MAX_SESSION_SOURCE_ID_BYTES,
        PAIRING_RUNTIME_INVALID_ARGUMENT, SENDER_PAIRING_INVALID_ARGUMENT,
        SENDER_PAIRING_UNKNOWN_REQUEST, mirrormesh_windows_begin_sender_pairing,
        mirrormesh_windows_begin_sender_session, mirrormesh_windows_cancel_sender_pairing,
        mirrormesh_windows_poll_sender_pairing, mirrormesh_windows_poll_sender_session_event,
        mirrormesh_windows_prepare_pairing_runtime, mirrormesh_windows_send_sender_session_ice,
        mirrormesh_windows_start_sender_session,
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

    #[test]
    fn exported_sender_session_rejects_invalid_buffers_and_unknown_requests() {
        let sdp = b"v=0\r\n";
        let mut session_id = [0_u8; 36];
        let mut session_id_length = 0_usize;
        assert_eq!(
            unsafe {
                mirrormesh_windows_begin_sender_session(
                    0,
                    sdp.as_ptr(),
                    sdp.len(),
                    session_id.as_mut_ptr(),
                    session_id.len(),
                    &mut session_id_length,
                )
            },
            SENDER_PAIRING_UNKNOWN_REQUEST
        );
        assert_eq!(
            unsafe {
                mirrormesh_windows_begin_sender_session(
                    0,
                    sdp.as_ptr(),
                    sdp.len(),
                    session_id.as_mut_ptr(),
                    session_id.len() - 1,
                    &mut session_id_length,
                )
            },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe {
                mirrormesh_windows_send_sender_session_ice(
                    0,
                    b"0".as_ptr(),
                    1,
                    0,
                    b"candidate:1".as_ptr(),
                    11,
                )
            },
            SENDER_PAIRING_UNKNOWN_REQUEST
        );
        assert_eq!(
            unsafe { mirrormesh_windows_start_sender_session(0, std::ptr::null(), 0) },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        let invalid_source_id = [0xff_u8];
        assert_eq!(
            unsafe {
                mirrormesh_windows_start_sender_session(
                    0,
                    invalid_source_id.as_ptr(),
                    invalid_source_id.len(),
                )
            },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        let oversized_source_id = [b'x'; MAX_SESSION_SOURCE_ID_BYTES + 1];
        assert_eq!(
            unsafe {
                mirrormesh_windows_start_sender_session(
                    0,
                    oversized_source_id.as_ptr(),
                    oversized_source_id.len(),
                )
            },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe { mirrormesh_windows_start_sender_session(0, b"desktop-capture".as_ptr(), 15) },
            SENDER_PAIRING_UNKNOWN_REQUEST
        );

        let mut short_event = [0_u8; 16];
        let mut event_length = 0_usize;
        assert_eq!(
            unsafe {
                mirrormesh_windows_poll_sender_session_event(
                    0,
                    short_event.as_mut_ptr(),
                    short_event.len(),
                    &mut event_length,
                )
            },
            SENDER_PAIRING_INVALID_ARGUMENT
        );
        let mut event = vec![0_u8; MAX_SESSION_EVENT_JSON_BYTES];
        assert_eq!(
            unsafe {
                mirrormesh_windows_poll_sender_session_event(
                    0,
                    event.as_mut_ptr(),
                    event.len(),
                    &mut event_length,
                )
            },
            SENDER_PAIRING_UNKNOWN_REQUEST
        );
    }
}
