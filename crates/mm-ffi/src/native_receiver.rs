use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::native_platform::{
    PlatformService as AndroidTvPairingServiceHandle, create_platform_service, local_device_id,
    revoke_peer,
};
use mm_adapter_signaling::PairingConnection;
use mm_adapter_signaling::native_service::{
    NativePairingServiceError, PendingNativePairing as PendingAndroidTvPairing,
};
use mm_domain::{DeviceId, MessageId, PeerGrant, SessionId, TrustedPeerGrants};
use mm_protocol::{
    MessagePayload, PairRevoke, SessionAnswer, SessionIce, SessionStop, encode_pair_revoke,
    encode_session_answer, encode_session_ice, encode_session_stop,
};
use serde_json::{Value, json};

const PAIRING_PORT: u16 = 8765;
const PAIRING_WORKER_STOP_TIMEOUT: Duration = Duration::from_secs(11);

static SERVICE: OnceLock<Mutex<Option<Arc<AndroidTvPairingServiceHandle>>>> = OnceLock::new();
static PENDING: OnceLock<Mutex<Option<(i64, PendingAndroidTvPairing)>>> = OnceLock::new();
static CONNECTIONS: OnceLock<Mutex<HashMap<i64, ApprovedControlConnection>>> = OnceLock::new();
static NEXT_REQUEST_ID: AtomicI64 = AtomicI64::new(1);
static NEXT_CONNECTION_ID: AtomicI64 = AtomicI64::new(1);
static WAITING_FOR_REQUEST: AtomicBool = AtomicBool::new(false);
static WAIT_GENERATION: AtomicU64 = AtomicU64::new(0);
static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static LIFECYCLE: Mutex<()> = Mutex::new(());

pub(crate) fn service_slot() -> &'static Mutex<Option<Arc<AndroidTvPairingServiceHandle>>> {
    SERVICE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn pending_slot() -> &'static Mutex<Option<(i64, PendingAndroidTvPairing)>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

pub(crate) fn connection_slot() -> &'static Mutex<HashMap<i64, ApprovedControlConnection>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) struct ApprovedControlConnection {
    channel: PairingConnection,
    peer_id: DeviceId,
    peer_fingerprint: String,
    grants: TrustedPeerGrants,
    session_id: Option<SessionId>,
    outbound_sequence: u64,
}

pub(crate) fn monotonic_millis() -> u64 {
    PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub(crate) fn unix_millis() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())?
        .as_millis()
        .try_into()
        .map_err(|_| "system clock value is out of range".to_owned())
}

pub(crate) fn get_service() -> Result<Arc<AndroidTvPairingServiceHandle>, String> {
    service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?
        .as_ref()
        .cloned()
        .ok_or_else(|| "receiver pairing has not been started".to_owned())
}

pub(crate) fn start_pairing(address: &str) -> Result<String, String> {
    let _lifecycle = LIFECYCLE
        .lock()
        .map_err(|_| "receiver lifecycle lock unavailable".to_owned())?;
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
        create_platform_service(address)
            .map_err(|_| "receiver identity or WSS listener could not start".to_owned())?,
    );
    let now_unix_seconds = unix_millis()? / 1000;
    let invitation = service
        .create_invitation(&address.to_string(), monotonic_millis(), now_unix_seconds)
        .map_err(|_| "receiver pairing invitation could not be created".to_owned())?;
    let uri = invitation.uri().to_owned();
    #[cfg(windows)]
    {
        // Discovery is optional and never changes the pinned invitation or consent flow.
        if let Ok(id) = local_device_id() {
            let _ = crate::network_devices::publish_receiver(id, address);
        }
    }
    *slot = Some(service);
    Ok(uri)
}

pub(crate) fn refresh_pairing_invitation() -> Result<Value, String> {
    let service = get_service()?;
    if !service.is_accepting() {
        return Err("receiver pairing has been stopped".to_owned());
    }
    let address = service
        .local_addr()
        .map_err(|_| "receiver local address is unavailable".to_owned())?;
    let invitation = service
        .create_invitation(
            &address.to_string(),
            monotonic_millis(),
            unix_millis()? / 1000,
        )
        .map_err(|_| "a fresh receiver invitation could not be created".to_owned())?;
    Ok(json!({ "ok": true, "invitation": invitation.uri() }))
}

pub(crate) fn cancel_pairing_wait() -> Result<Value, String> {
    let service_guard = service_slot()
        .lock()
        .map_err(|_| "receiver service lock is unavailable".to_owned())?;
    let service = service_guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "receiver pairing has not been started".to_owned())?;
    if !service.is_accepting() {
        return Err("receiver pairing has been stopped".to_owned());
    }
    WAIT_GENERATION.fetch_add(1, Ordering::AcqRel);
    service.cancel_wait();
    let pending = pending_slot()
        .lock()
        .map_err(|_| "pending request lock is unavailable".to_owned())?
        .take();
    drop(pending);
    Ok(json!({ "ok": true }))
}

pub(crate) fn accept_one_request() -> Result<Value, String> {
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

    let wait_generation = WAIT_GENERATION.load(Ordering::Acquire);
    let service = get_service()?;
    let pending = service
        .accept_request(monotonic_millis())
        .map_err(|error| match error {
            NativePairingServiceError::Listener(
                mm_adapter_signaling::WssListenerError::Cancelled,
            ) if service.is_accepting() => "receiver wait was cancelled".to_owned(),
            NativePairingServiceError::Listener(
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
    if WAIT_GENERATION.load(Ordering::Acquire) != wait_generation {
        return Err("receiver wait was cancelled".to_owned());
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

pub(crate) fn approve_request(
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
        .insert(
            connection_id,
            ApprovedControlConnection {
                channel: connection,
                peer_id: peer.id(),
                peer_fingerprint: peer.fingerprint().to_hex(),
                grants: peer.grants(),
                session_id: None,
                outbound_sequence: 0,
            },
        );
    drop(service_guard);
    Ok(json!({
        "ok": true,
        "peer_id": peer.id().to_string(),
        "peer_fingerprint": peer.fingerprint().to_hex(),
        "connection_id": connection_id
    }))
}

pub(crate) fn reject_request(request_id: i64) -> Result<Value, String> {
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

pub(crate) fn stop_pairing() -> Result<Value, String> {
    let _lifecycle = LIFECYCLE
        .lock()
        .map_err(|_| "receiver lifecycle lock unavailable".to_owned())?;
    #[cfg(windows)]
    crate::network_devices::stop_advertisement();
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
        if connection.channel.close().is_err() {
            close_failures += 1;
        }
    }
    let worker_deadline = Instant::now() + PAIRING_WORKER_STOP_TIMEOUT;
    while WAITING_FOR_REQUEST.load(Ordering::Acquire) && Instant::now() < worker_deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if WAITING_FOR_REQUEST.load(Ordering::Acquire) {
        return Err("pairing worker did not exit after listener cancellation".to_owned());
    }
    Ok(json!({
        "ok": true,
        "was_active": was_active,
        "closed_connections": closed_connections,
        "close_failures": close_failures
    }))
}

pub(crate) fn take_pending(request_id: i64) -> Result<PendingAndroidTvPairing, String> {
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

pub(crate) fn close_connection(connection_id: i64) -> Result<bool, String> {
    let Some(mut connection) = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?
        .remove(&connection_id)
    else {
        return Ok(false);
    };
    if let Some(session_id) = connection.session_id {
        // Sending is best effort; local closure always proceeds after a bounded socket timeout.
        if let (Ok(sequence), Ok(message_id), Ok(now)) = (
            next_outbound_sequence(&mut connection),
            random_message_id(),
            unix_millis(),
        ) && let Ok(frame) = encode_session_stop(
            message_id,
            session_id,
            sequence,
            now,
            &SessionStop {
                reason: "receiver-disconnected".to_owned(),
            },
        ) {
            let _ = connection.channel.send_control_message(&frame);
        }
    }
    connection
        .channel
        .close()
        .map_err(|_| "the approved control channel could not close cleanly".to_owned())?;
    Ok(true)
}

pub(crate) fn poll_control_message(connection_id: i64) -> Result<Value, String> {
    let mut connections = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?;
    let connection = connections
        .get_mut(&connection_id)
        .ok_or_else(|| "approved control connection is unavailable".to_owned())?;
    if !connection.grants.allows(PeerGrant::View) {
        return Err("screen viewing was not approved for this sender".to_owned());
    }

    let Some(message) = connection
        .channel
        .try_receive_control_message()
        .map_err(|_| "approved control message failed protocol or transport checks".to_owned())?
    else {
        return Ok(json!({ "ok": true, "pending": true }));
    };

    match message.payload {
        MessagePayload::SessionOffer(offer) => {
            if offer.role != "sender" {
                return Err("incoming TV sessions must be offered by a sender".to_owned());
            }
            let session_id = message
                .header
                .session_id
                .ok_or_else(|| "session offer has no session identifier".to_owned())?;
            if connection
                .session_id
                .is_some_and(|current| current != session_id)
            {
                return Err("sender attempted to replace the approved TV session".to_owned());
            }
            connection.session_id = Some(session_id);
            Ok(json!({
                "ok": true,
                "pending": false,
                "kind": "offer",
                "session_id": session_id.to_string(),
                "peer_fingerprint": connection.peer_fingerprint.clone(),
                "sdp": offer.sdp
            }))
        }
        MessagePayload::SessionIce(candidate) => {
            let session_id = message
                .header
                .session_id
                .ok_or_else(|| "ICE candidate has no session identifier".to_owned())?;
            if connection.session_id != Some(session_id) {
                return Err("ICE candidate does not belong to the approved TV session".to_owned());
            }
            let media_id = candidate
                .sdp_mid
                .ok_or_else(|| "ICE candidate has no media identifier".to_owned())?;
            let media_line_index = candidate
                .mline_index
                .ok_or_else(|| "ICE candidate has no media line index".to_owned())?;
            Ok(json!({
                "ok": true,
                "pending": false,
                "kind": "ice",
                "media_id": media_id,
                "media_line_index": media_line_index,
                "candidate": candidate.candidate
            }))
        }
        MessagePayload::SessionStop(stop) => {
            if message.header.session_id != connection.session_id {
                return Err("session stop does not belong to the approved TV session".to_owned());
            }
            connection.session_id = None;
            Ok(json!({
                "ok": true,
                "pending": false,
                "kind": "stop",
                "reason": stop.reason
            }))
        }
        MessagePayload::SessionStart(start) => {
            if connection.session_id != message.header.session_id {
                return Err("session start does not belong to the approved TV session".to_owned());
            }
            Ok(json!({
                "ok": true,
                "pending": false,
                "kind": "start",
                "source_id": start.source_id
            }))
        }
        MessagePayload::PairRevoke(revoke) => {
            let local_id =
                local_device_id().map_err(|_| "receiver identity is unavailable".to_owned())?;
            if revoke.peer_id != local_id.to_string() {
                return Err("revocation is not addressed to this receiver".to_owned());
            }
            revoke_peer(connection.peer_id, unix_millis()?)
                .map_err(|_| "peer revocation could not be saved".to_owned())?;
            Ok(json!({ "ok": true, "pending": false, "kind": "revoke" }))
        }
        _ => Err("approved TV channel received an unsupported control message".to_owned()),
    }
}

pub(crate) fn revoke_connection(connection_id: i64) -> Result<Value, String> {
    let mut connection = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?
        .remove(&connection_id)
        .ok_or_else(|| "approved connection is already closed".to_owned())?;
    let result = (|| {
        let now = unix_millis()?;
        revoke_peer(connection.peer_id, now)
            .map_err(|_| "connection closed, but revocation could not be saved".to_owned())?;
        let sequence = next_outbound_sequence(&mut connection)?;
        let frame = encode_pair_revoke(
            random_message_id()?,
            sequence,
            now,
            &PairRevoke {
                peer_id: connection.peer_id.to_string(),
            },
        )
        .map_err(|_| "revocation message could not be encoded".to_owned())?;
        // Persisted revocation remains valid even if the peer has already left.
        let delivered = connection.channel.send_control_message(&frame).is_ok();
        Ok(json!({ "ok": true, "revoked": true, "delivered": delivered }))
    })();
    let _ = connection.channel.close();
    result
}

pub(crate) fn send_session_answer(connection_id: i64, sdp: &str) -> Result<Value, String> {
    let mut connections = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?;
    let connection = connections
        .get_mut(&connection_id)
        .ok_or_else(|| "approved control connection is unavailable".to_owned())?;
    if !connection.grants.allows(PeerGrant::View) {
        return Err("screen viewing was not approved for this sender".to_owned());
    }
    let session_id = connection
        .session_id
        .ok_or_else(|| "a sender offer must be received before answering".to_owned())?;
    let sequence = next_outbound_sequence(connection)?;
    let frame = encode_session_answer(
        random_message_id()?,
        session_id,
        sequence,
        unix_millis()?,
        &SessionAnswer {
            sdp: sdp.to_owned(),
            selected_codec: "H264".to_owned(),
        },
    )
    .map_err(|_| "WebRTC answer failed the shared protocol checks".to_owned())?;
    connection
        .channel
        .send_control_message(&frame)
        .map_err(|_| "WebRTC answer could not be sent".to_owned())?;
    Ok(json!({ "ok": true }))
}

pub(crate) fn send_session_ice(
    connection_id: i64,
    media_id: &str,
    media_line_index: i64,
    candidate: &str,
) -> Result<Value, String> {
    let media_line_index: u16 = media_line_index
        .try_into()
        .map_err(|_| "ICE media line index is invalid".to_owned())?;
    let mut connections = connection_slot()
        .lock()
        .map_err(|_| "approved connection registry is unavailable".to_owned())?;
    let connection = connections
        .get_mut(&connection_id)
        .ok_or_else(|| "approved control connection is unavailable".to_owned())?;
    if !connection.grants.allows(PeerGrant::View) {
        return Err("screen viewing was not approved for this sender".to_owned());
    }
    let session_id = connection
        .session_id
        .ok_or_else(|| "a sender offer must be received before ICE".to_owned())?;
    let sequence = next_outbound_sequence(connection)?;
    let frame = encode_session_ice(
        random_message_id()?,
        session_id,
        sequence,
        unix_millis()?,
        &SessionIce {
            candidate: candidate.to_owned(),
            sdp_mid: Some(media_id.to_owned()),
            mline_index: Some(media_line_index),
        },
    )
    .map_err(|_| "WebRTC ICE candidate failed the shared protocol checks".to_owned())?;
    connection
        .channel
        .send_control_message(&frame)
        .map_err(|_| "WebRTC ICE candidate could not be sent".to_owned())?;
    Ok(json!({ "ok": true }))
}

pub(crate) fn next_outbound_sequence(
    connection: &mut ApprovedControlConnection,
) -> Result<u64, String> {
    connection.outbound_sequence = connection
        .outbound_sequence
        .checked_add(1)
        .ok_or_else(|| "TV control sequence space is exhausted".to_owned())?;
    Ok(connection.outbound_sequence)
}

pub(crate) fn random_message_id() -> Result<MessageId, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| "operating-system randomness is unavailable".to_owned())?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(MessageId::from_bytes(bytes))
}

pub(crate) fn response_json(result: Result<Value, String>) -> String {
    match result {
        Ok(value) => value.to_string(),
        Err(error) => json!({ "ok": false, "error": error }).to_string(),
    }
}
