use crate::{native_platform, native_receiver as receiver};
use mm_adapter_signaling::sender_driver as sender;
use serde_json::{Value, json};
use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) fn dispatch(input: &str) -> Result<Value, String> {
    let command: Value =
        serde_json::from_str(input).map_err(|_| "Invalid connection command".to_owned())?;
    let text = |key: &str| {
        command[key]
            .as_str()
            .ok_or_else(|| format!("Missing {key}"))
    };
    let number = |key: &str| {
        command[key]
            .as_i64()
            .ok_or_else(|| format!("Missing {key}"))
    };
    let failed = |_| "Native session operation failed".to_owned();
    match text("op")? {
        "prepare" => Ok(
            json!({"ok":true, "device_id":native_platform::local_device_id().map_err(|_| "Device identity unavailable")?.to_string()}),
        ),
        "configure_device" => {
            let details = mm_domain::ReportedDeviceDetails::new(
                text("platform")?.to_owned(),
                text("manufacturer")?.to_owned(),
                text("model")?.to_owned(),
                text("os_version")?.to_owned(),
            )
            .map_err(|_| "Invalid device details".to_owned())?;
            sender::configure_device_profile(text("name")?.to_owned(), details).map_err(failed)?;
            Ok(json!({"ok": true}))
        }
        "receive_start" => Ok(
            json!({"ok":true,"invitation":receiver::start_pairing(text("address")?)?,
            "device_id":native_platform::local_device_id().map_err(|_| "Device identity unavailable")?.to_string(),
            "fingerprint":receiver::get_service()?.local_fingerprint().map_err(|_| "Identity unavailable")?.to_hex()}),
        ),
        "receive_wait" => receiver::accept_one_request(),
        "receive_refresh" => receiver::refresh_pairing_invitation(),
        "receive_cancel_wait" => receiver::cancel_pairing_wait(),
        "receive_stop" => receiver::stop_pairing(),
        "receive_decide" => {
            if command["approve"] == true {
                receiver::approve_request(number("id")?, true, false, false)
            } else {
                receiver::reject_request(number("id")?)
            }
        }
        "receive_poll" => receiver::poll_control_message(number("id")?),
        "receive_answer" => receiver::send_session_answer(number("id")?, text("sdp")?),
        "receive_ice" => receiver::send_session_ice(
            number("id")?,
            text("mid")?,
            number("line")?,
            text("candidate")?,
        ),
        "receive_close" => Ok(json!({"ok":receiver::close_connection(number("id")?)?})),
        "receive_revoke" => receiver::revoke_connection(number("id")?),
        "send_pair" => {
            let (id, identity) =
                native_platform::sender_identity().map_err(|_| "Device identity unavailable")?;
            let request = sender::begin_sender_pairing(
                text("invitation")?.to_owned(),
                id,
                "MirrorMesh sender".to_owned(),
                identity,
            )
            .map_err(failed)?;
            Ok(json!({"ok":true,"id":request}))
        }
        "send_poll" => {
            let status = sender::poll_sender_pairing(number("id")? as u64).map_err(failed)?;
            Ok(
                json!({"ok":true,"state":status.state as u32,"error":status.error_code,
                "fingerprint":mm_domain::PeerFingerprint::from_bytes(status.receiver_fingerprint).to_hex()}),
            )
        }
        "send_direct" => {
            let (id, identity) =
                native_platform::sender_identity().map_err(|_| "Device identity unavailable")?;
            let request = sender::begin_direct_pairing(
                text("endpoint")?
                    .parse()
                    .map_err(|_| "Invalid LAN endpoint")?,
                mm_domain::DeviceId::parse(text("receiver_id")?)
                    .map_err(|_| "Invalid device ID")?,
                mm_domain::PeerFingerprint::from_hex(text("fingerprint")?)
                    .map_err(|_| "Invalid fingerprint")?,
                id,
                "MirrorMesh PC".to_owned(),
                identity,
            )
            .map_err(failed)?;
            Ok(json!({"ok":true,"id":request}))
        }
        "send_verification" => Ok(
            json!({"ok":true,"code":sender::direct_verification_code(number("id")? as u64).map_err(failed)?}),
        ),
        "send_confirm" => {
            sender::confirm_direct_pairing(number("id")? as u64, text("code")?).map_err(failed)?;
            Ok(json!({"ok":true}))
        }
        "send_cancel" => {
            sender::cancel_sender_pairing(number("id")? as u64).map_err(failed)?;
            Ok(json!({"ok":true}))
        }
        "send_offer" => {
            let id =
                sender::begin_sender_session(number("id")? as u64, text("sdp")?).map_err(failed)?;
            Ok(json!({"ok":true,"session_id":id.to_string()}))
        }
        "send_ice" => {
            sender::send_sender_session_ice(
                number("id")? as u64,
                text("candidate")?,
                text("mid")?,
                u16::try_from(number("line")?).map_err(|_| "Invalid ICE line")?,
            )
            .map_err(failed)?;
            Ok(json!({"ok":true}))
        }
        "send_start" => {
            sender::start_sender_session(number("id")? as u64, text("source")?).map_err(failed)?;
            Ok(json!({"ok":true}))
        }
        "send_stop" => {
            sender::stop_sender_session(
                number("id")? as u64,
                command["reason"].as_str().unwrap_or("user-stopped"),
            )
            .map_err(failed)?;
            Ok(json!({"ok":true}))
        }
        "send_event" => {
            let event = sender::poll_sender_session_event(number("id")? as u64).map_err(failed)?;
            Ok(match event {
                sender::SenderSessionEvent::Pending => json!({"ok":true,"pending":true}),
                sender::SenderSessionEvent::Answer { session_id, sdp } => {
                    json!({"ok":true,"kind":"answer","session_id":session_id,"sdp":sdp})
                }
                sender::SenderSessionEvent::IceCandidate {
                    session_id,
                    candidate,
                    media_id,
                    media_line_index,
                } => {
                    json!({"ok":true,"kind":"ice","session_id":session_id,"candidate":candidate,"mid":media_id,"line":media_line_index})
                }
                sender::SenderSessionEvent::Stopped { session_id, reason } => {
                    json!({"ok":true,"kind":"stop","session_id":session_id,"reason":reason})
                }
                sender::SenderSessionEvent::Revoked { peer_id } => {
                    json!({"ok":true,"kind":"revoke","peer_id":peer_id})
                }
            })
        }
        _ => Err("Unknown connection command".to_owned()),
    }
}

/// Allocates a bounded metadata command buffer. Must be returned using `mirrormesh_release_command`.
#[unsafe(no_mangle)]
pub extern "C" fn mirrormesh_alloc_command(length: usize) -> *mut u8 {
    if length == 0 || length > 256 * 1024 {
        return std::ptr::null_mut();
    }
    Box::into_raw(vec![0_u8; length].into_boxed_slice()).cast::<u8>()
}

/// # Safety
/// Pointer and length must describe an unreleased allocation from `mirrormesh_alloc_command`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_release_command(input: *mut u8, length: usize) {
    if !input.is_null() {
        // SAFETY: The caller returns the exact original allocation and length once.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(input, length)) });
    }
}

/// # Safety
/// Input points to `length` readable UTF-8 metadata bytes. Returned JSON must be freed with `mirrormesh_free_json`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_connection_command(
    input: *const u8,
    length: usize,
) -> *mut c_char {
    let response = catch_unwind(AssertUnwindSafe(|| {
        if input.is_null() || length == 0 || length > 256 * 1024 {
            return Err("Invalid command size".to_owned());
        }
        // SAFETY: Readable input byte range is guaranteed by the caller contract.
        let bytes = unsafe { std::slice::from_raw_parts(input, length) };
        dispatch(std::str::from_utf8(bytes).map_err(|_| "Invalid command encoding")?)
    }))
    .unwrap_or_else(|_| Err("Connection operation could not complete".to_owned()));
    CString::new(receiver::response_json(response))
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
