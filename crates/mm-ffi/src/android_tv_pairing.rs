use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jlong, jstring};
use mm_adapter_android::{AndroidTvPairingServiceHandle, PendingAndroidTvPairing};
use mm_adapter_signaling::PairingConnection;
use mm_domain::TrustedPeerGrants;
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

const PAIRING_PORT: u16 = 8765;

static SERVICE: OnceLock<Mutex<Option<Arc<AndroidTvPairingServiceHandle>>>> = OnceLock::new();
static PENDING: OnceLock<Mutex<Option<(i64, PendingAndroidTvPairing)>>> = OnceLock::new();
static CONNECTIONS: OnceLock<Mutex<HashMap<i64, PairingConnection>>> = OnceLock::new();
static NEXT_REQUEST_ID: AtomicI64 = AtomicI64::new(1);
static NEXT_CONNECTION_ID: AtomicI64 = AtomicI64::new(1);
static WAITING_FOR_REQUEST: AtomicBool = AtomicBool::new(false);
static PROCESS_START: OnceLock<Instant> = OnceLock::new();

fn service_slot() -> &'static Mutex<Option<Arc<AndroidTvPairingServiceHandle>>> {
    SERVICE.get_or_init(|| Mutex::new(None))
}

fn pending_slot() -> &'static Mutex<Option<(i64, PendingAndroidTvPairing)>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

fn connection_slot() -> &'static Mutex<HashMap<i64, PairingConnection>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn monotonic_millis() -> u64 {
    PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn unix_millis() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())?
        .as_millis()
        .try_into()
        .map_err(|_| "system clock value is out of range".to_owned())
}

fn get_service() -> Result<Arc<AndroidTvPairingServiceHandle>, String> {
    service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?
        .as_ref()
        .cloned()
        .ok_or_else(|| "receiver pairing has not been started".to_owned())
}

fn start_pairing(address: &str) -> Result<String, String> {
    let address = SocketAddr::from_str(address)
        .map_err(|_| "enter a numeric local IP address and port".to_owned())?;
    if address.port() != PAIRING_PORT {
        return Err(format!("receiver pairing uses port {PAIRING_PORT}"));
    }
    if address.ip().is_unspecified() || address.ip().is_loopback() {
        return Err("select a reachable local network address".to_owned());
    }

    let mut slot = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?;
    if slot.is_some() {
        return Err("receiver pairing is already active".to_owned());
    }

    let service = Arc::new(
        mm_adapter_android::create_tv_pairing_service(address)
            .map_err(|_| "receiver identity or WSS listener could not start".to_owned())?,
    );
    let now_unix_seconds = unix_millis()? / 1000;
    let invitation = service
        .create_invitation(&address.to_string(), monotonic_millis(), now_unix_seconds)
        .map_err(|_| "receiver pairing invitation could not be created".to_owned())?;
    let uri = invitation.uri().to_owned();
    *slot = Some(service);
    Ok(uri)
}

fn accept_one_request() -> Result<Value, String> {
    if WAITING_FOR_REQUEST
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("a receiver request is already being accepted".to_owned());
    }
    let _waiting = WaitingRequestGuard;
    if pending_slot()
        .lock()
        .map_err(|_| "pending request lock is unavailable".to_owned())?
        .is_some()
    {
        return Err("decide on the current receiver request first".to_owned());
    }

    let service = get_service()?;
    let pending = service
        .accept_request(monotonic_millis())
        .map_err(|error| match error {
            mm_adapter_android::AndroidTvPairingServiceError::Listener(
                mm_adapter_signaling::WssListenerError::Cancelled,
            ) => "receiver pairing was stopped".to_owned(),
            _ => "sender request failed identity or protocol checks".to_owned(),
        })?;
    let request = pending.request().clone();
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    if request_id <= 0 {
        return Err("receiver request identifier space is exhausted".to_owned());
    }
    // Serialize the handoff with stop_pairing so shutdown can always drain this request.
    let service_guard = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?;
    if service_guard.is_none() || !service.is_accepting() {
        return Err("receiver pairing was stopped".to_owned());
    }
    let mut slot = pending_slot()
        .lock()
        .map_err(|_| "pending request lock is unavailable".to_owned())?;
    if slot.is_some() {
        return Err("decide on the current receiver request first".to_owned());
    }
    *slot = Some((request_id, pending));
    drop(service_guard);
    Ok(json!({
        "ok": true,
        "request_id": request_id,
        "peer_id": request.peer_id.to_string(),
        "display_name": request.display_name,
        "fingerprint": request.fingerprint.to_hex(),
        "source_ip": request.source_ip.to_string(),
        "requested_grants": {
            "view": request.requested_grants.allows(mm_domain::PeerGrant::View),
            "send": request.requested_grants.allows(mm_domain::PeerGrant::Send),
            "annotate": request.requested_grants.allows(mm_domain::PeerGrant::Annotate)
        }
    }))
}

struct WaitingRequestGuard;

impl Drop for WaitingRequestGuard {
    fn drop(&mut self) {
        WAITING_FOR_REQUEST.store(false, Ordering::Release);
    }
}

fn approve_request(
    request_id: i64,
    view: bool,
    send: bool,
    annotate: bool,
) -> Result<Value, String> {
    let grants = TrustedPeerGrants::new(view, send, annotate)
        .map_err(|_| "approve at least one requested permission".to_owned())?;
    let service_guard = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?;
    let service = service_guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "receiver pairing has not been started".to_owned())?;
    let pending = take_pending(request_id)?;
    let (peer, connection) = service
        .approve(pending, grants, monotonic_millis(), unix_millis()?)
        .map_err(|_| "pairing approval failed; inspect trust before retrying".to_owned())?;
    let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
    if connection_id <= 0 {
        return Err("connection identifier space is exhausted".to_owned());
    }
    connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?
        .insert(connection_id, connection);
    drop(service_guard);
    Ok(json!({
        "ok": true,
        "peer_id": peer.id().to_string(),
        "connection_id": connection_id
    }))
}

fn reject_request(request_id: i64) -> Result<Value, String> {
    let service_guard = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?;
    if service_guard.is_none() {
        return Err("receiver pairing has not been started".to_owned());
    }
    let pending = take_pending(request_id)?;
    pending
        .reject("user_rejected", unix_millis()?)
        .map_err(|_| "the rejection response could not be delivered".to_owned())?;
    drop(service_guard);
    Ok(json!({ "ok": true }))
}

fn stop_pairing() -> Result<Value, String> {
    let service = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?
        .take();
    let was_active = service.is_some();
    if let Some(service) = &service {
        service.stop_accepting();
    }

    let pending = pending_slot()
        .lock()
        .map_err(|_| "pending request lock is unavailable".to_owned())?
        .take();
    drop(pending);

    let mut connections = std::mem::take(
        &mut *connection_slot()
            .lock()
            .map_err(|_| "approved connection registry is unavailable".to_owned())?,
    );
    let closed_connections = connections.len();
    let mut close_failures = 0;
    for connection in connections.values_mut() {
        if connection.close().is_err() {
            close_failures += 1;
        }
    }
    Ok(json!({
        "ok": true,
        "was_active": was_active,
        "closed_connections": closed_connections,
        "close_failures": close_failures
    }))
}

fn take_pending(request_id: i64) -> Result<PendingAndroidTvPairing, String> {
    let mut slot = pending_slot()
        .lock()
        .map_err(|_| "pending request lock is unavailable".to_owned())?;
    match slot.take() {
        Some((stored_id, pending)) if stored_id == request_id => Ok(pending),
        Some(value) => {
            *slot = Some(value);
            Err("receiver request has expired or changed".to_owned())
        }
        None => Err("there is no pending receiver request".to_owned()),
    }
}

fn close_connection(connection_id: i64) -> Result<bool, String> {
    let Some(mut connection) = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?
        .remove(&connection_id)
    else {
        return Ok(false);
    };
    connection
        .close()
        .map_err(|_| "the approved control channel could not close cleanly".to_owned())?;
    Ok(true)
}

fn response_json(result: Result<Value, String>) -> String {
    match result {
        Ok(value) => value.to_string(),
        Err(error) => json!({ "ok": false, "error": error }).to_string(),
    }
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
        start_pairing(&address).map(|uri| json!({ "ok": true, "invitation": uri }))
    }))
    .unwrap_or_else(|_| Err("receiver pairing could not start".to_owned()));
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
