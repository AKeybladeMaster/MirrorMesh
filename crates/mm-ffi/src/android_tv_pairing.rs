use crate::native_receiver::*;
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jlong, jstring},
};
use serde_json::json;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_connectionCommand(
    mut env: JNIEnv,
    _bridge: JClass,
    input: JString,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let input = java_input(&mut env, input)?;
        if input.len() > 256 * 1024 {
            return Err("Connection command too large".to_owned());
        }
        crate::connection_command::dispatch(&input)
    }))
    .unwrap_or_else(|_| Err("Connection command could not complete".to_owned()));
    java_string(&mut env, response_json(result))
}
fn java_string(env: &mut JNIEnv<'_>, value: String) -> jstring {
    env.new_string(value)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

fn java_input(env: &mut JNIEnv<'_>, value: JString<'_>) -> Result<String, String> {
    env.get_string(&value)
        .map(|value| value.into())
        .map_err(|_| "input string could not be read".to_owned())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_startPairing(
    mut env: JNIEnv,
    _bridge: JClass,
    address: JString,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let address = java_input(&mut env, address)?;
        let uri = start_pairing(&address)?;
        let id = mm_adapter_android::initialize_pairing_runtime()
            .map_err(|_| "identity unavailable".to_owned())?;
        Ok(json!({ "ok": true, "invitation": uri, "device_id": id.to_string() }))
    }))
    .unwrap_or_else(|_| Err("receiver pairing could not start".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_refreshPairingInvitation(
    mut env: JNIEnv,
    _bridge: JClass,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(refresh_pairing_invitation))
        .unwrap_or_else(|_| Err("a fresh receiver invitation could not be created".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_cancelPairingWait(
    mut env: JNIEnv,
    _bridge: JClass,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(cancel_pairing_wait))
        .unwrap_or_else(|_| Err("receiver wait could not be cancelled".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_waitForPairingRequest(
    mut env: JNIEnv,
    _bridge: JClass,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(accept_one_request))
        .unwrap_or_else(|_| Err("receiver request failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_approvePairing(
    mut env: JNIEnv,
    _bridge: JClass,
    request_id: jlong,
    view: jboolean,
    send: jboolean,
    annotate: jboolean,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        approve_request(
            request_id,
            view == JNI_TRUE,
            send == JNI_TRUE,
            annotate == JNI_TRUE,
        )
    }))
    .unwrap_or_else(|_| Err("pairing approval failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_rejectPairing(
    mut env: JNIEnv,
    _bridge: JClass,
    request_id: jlong,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| reject_request(request_id)))
        .unwrap_or_else(|_| Err("pairing rejection failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_stopPairing(
    mut env: JNIEnv,
    _bridge: JClass,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(stop_pairing))
        .unwrap_or_else(|_| Err("receiver pairing could not stop cleanly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_pollControlMessage(
    mut env: JNIEnv,
    _bridge: JClass,
    connection_id: jlong,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| poll_control_message(connection_id)))
        .unwrap_or_else(|_| Err("approved control message failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_sendSessionAnswer(
    mut env: JNIEnv,
    _bridge: JClass,
    connection_id: jlong,
    sdp: JString,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let sdp = java_input(&mut env, sdp)?;
        send_session_answer(connection_id, &sdp)
    }))
    .unwrap_or_else(|_| Err("WebRTC answer failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_sendSessionIce(
    mut env: JNIEnv,
    _bridge: JClass,
    connection_id: jlong,
    media_id: JString,
    media_line_index: jint,
    candidate: JString,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let media_id = java_input(&mut env, media_id)?;
        let candidate = java_input(&mut env, candidate)?;
        send_session_ice(
            connection_id,
            &media_id,
            i64::from(media_line_index),
            &candidate,
        )
    }))
    .unwrap_or_else(|_| Err("WebRTC ICE candidate failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_revokeControlChannel(
    mut env: JNIEnv,
    _bridge: JClass,
    connection_id: jlong,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| revoke_connection(connection_id)))
        .unwrap_or_else(|_| Err("revocation failed unexpectedly".to_owned()));
    java_string(&mut env, response_json(result))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_tv_NativePairingBridge_closeControlChannel(
    _env: JNIEnv,
    _bridge: JClass,
    connection_id: jlong,
) -> jboolean {
    match catch_unwind(AssertUnwindSafe(|| close_connection(connection_id))) {
        Ok(Ok(true)) => JNI_TRUE,
        Ok(Ok(false)) | Ok(Err(_)) | Err(_) => JNI_FALSE,
    }
}
