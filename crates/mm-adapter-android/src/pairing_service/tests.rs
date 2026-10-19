use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_adapter_signaling::WssPairingClient;
use mm_domain::{
    DeviceId, PeerFingerprint, PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{
    LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort,
};

use super::AndroidTvPairingService;

const RECEIVER_ID: DeviceId = DeviceId::from_bytes([0x31; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([0x42; 16]);
fn view_grant() -> TrustedPeerGrants {
    TrustedPeerGrants::new(true, false, false).unwrap()
}
const OFFER: &str = include_str!("../../../../tests/protocol-golden/session-offer-v1.json");
const ANSWER: &str = r#"{"v":1,"type":"session.answer","message_id":"3182f5b4-a037-4f19-b2f7-c686ed10bd82","session_id":"8b3d8788-1476-4aae-a287-334a428b5b6f","seq":1,"sent_at_ms":1792316000001,"payload":{"sdp":"v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=MirrorMesh test answer\r\nt=0 0\r\n","selected_codec":"H264"}}"#;

struct TestIdentity(SigningKey);

impl TestIdentity {
    fn new(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    fn public_key(&self) -> Result<PeerPublicKey, PortError> {
        PeerPublicKey::new(self.0.verifying_key().to_bytes().to_vec())
            .map_err(|_| PortError::IdentityRejected)
    }
}

impl LocalPairingIdentityPort for TestIdentity {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        self.public_key()
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        Ok(self.0.sign(message).to_bytes().to_vec())
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        Err(PortError::Unavailable)
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        Err(PortError::Unavailable)
    }
}

impl LocalTlsIdentityPort for TestIdentity {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError> {
        self.public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        Ok(self.0.sign(message).to_bytes().to_vec())
    }
}

impl PairingIdentityPort for TestIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError> {
        Ed25519PairingIdentity.fingerprint_public_key(public_key)
    }

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        Ed25519PairingIdentity.verify_signature(public_key, message, signature)
    }
}

#[derive(Default)]
struct MemoryPeers(Mutex<HashMap<DeviceId, TrustedPeer>>);

impl TrustedPeerPort for MemoryPeers {
    fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError> {
        self.0
            .lock()
            .map(|peers| peers.get(&peer_id).cloned())
            .map_err(|_| PortError::StorageFailure)
    }

    fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError> {
        self.0
            .lock()
            .map(|mut peers| peers.insert(peer.id(), peer.clone()))
            .map(|_| ())
            .map_err(|_| PortError::StorageFailure)
    }

    fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError> {
        self.0
            .lock()
            .map(|peers| peers.values().cloned().collect())
            .map_err(|_| PortError::StorageFailure)
    }

    fn revoke(&self, peer_id: DeviceId, revoked_at_unix_ms: u64) -> Result<(), PortError> {
        let mut peers = self.0.lock().map_err(|_| PortError::StorageFailure)?;
        peers
            .get_mut(&peer_id)
            .ok_or(PortError::Unavailable)?
            .revoke(revoked_at_unix_ms)
            .map(|_| ())
            .map_err(|_| PortError::StorageFailure)
    }
}

#[test]
fn explicit_android_tv_service_pairs_over_the_shared_client_and_control_channel() {
    let receiver_identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(MemoryPeers::default());
    let runtime = mm_runtime::PairingRuntime::new(
        RECEIVER_ID,
        Arc::clone(&receiver_identity),
        Arc::clone(&peers),
    )
    .unwrap();
    let service = Arc::new(
        AndroidTvPairingService::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            runtime,
            Arc::clone(&receiver_identity),
        )
        .unwrap(),
    );
    let address = service.local_addr().unwrap();
    let invitation = service
        .create_invitation(&address.to_string(), 500, 10_000)
        .unwrap();

    let accepting_service = Arc::clone(&service);
    let pending_request = thread::spawn(move || accepting_service.accept_request(501));
    let sender_identity = TestIdentity::new([0x42; 32]);
    let pending_client = WssPairingClient::begin_pairing(
        invitation.uri(),
        10_000,
        &[address.port()],
        SENDER_ID,
        "Windows sender",
        view_grant(),
        &sender_identity,
        10_001_000,
    )
    .unwrap();
    let pending = pending_request.join().unwrap().unwrap();
    let request = pending.request();
    assert_eq!(request.source_ip, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(request.peer_id, SENDER_ID);
    assert_eq!(request.display_name, "Windows sender");
    assert!(request.requested_grants.allows(PeerGrant::View));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());

    let (approved, mut receiver_connection) = service
        .approve(pending, view_grant(), 502, 10_001_001)
        .unwrap();
    assert_eq!(approved.id(), SENDER_ID);
    assert_eq!(peers.get_peer(SENDER_ID).unwrap(), Some(approved));

    let mut sender_connection = pending_client.wait_for_decision().unwrap().connection;
    assert!(
        receiver_connection
            .try_receive_control_message()
            .unwrap()
            .is_none()
    );
    sender_connection
        .send_control_message(OFFER.as_bytes())
        .unwrap();
    let received_offer = receiver_connection
        .try_receive_control_message()
        .unwrap()
        .expect("a sent offer is available after the bounded poll");
    assert_eq!(
        received_offer.header.kind,
        mm_protocol::MessageKind::SessionOffer
    );
    receiver_connection
        .send_control_message(ANSWER.as_bytes())
        .unwrap();
    let received_answer = sender_connection.receive_control_message().unwrap();
    assert_eq!(
        received_answer.header.kind,
        mm_protocol::MessageKind::SessionAnswer
    );
}

#[test]
fn receiver_service_rejects_a_tls_identity_that_differs_from_runtime_identity() {
    let runtime_identity = Arc::new(TestIdentity::new([0x11; 32]));
    let tls_identity = Arc::new(TestIdentity::new([0x12; 32]));
    let peers = Arc::new(MemoryPeers::default());
    let runtime =
        mm_runtime::PairingRuntime::new(RECEIVER_ID, Arc::clone(&runtime_identity), peers).unwrap();

    let result = AndroidTvPairingService::bind(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        runtime,
        tls_identity,
    );

    assert!(matches!(
        result,
        Err(super::AndroidTvPairingServiceError::IdentityMismatch)
    ));
}

#[test]
fn explicit_android_tv_rejection_closes_without_saving_sender_trust() {
    let receiver_identity = Arc::new(TestIdentity::new([0x21; 32]));
    let peers = Arc::new(MemoryPeers::default());
    let runtime = mm_runtime::PairingRuntime::new(
        RECEIVER_ID,
        Arc::clone(&receiver_identity),
        Arc::clone(&peers),
    )
    .unwrap();
    let service = Arc::new(
        AndroidTvPairingService::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            runtime,
            receiver_identity,
        )
        .unwrap(),
    );
    let address = service.local_addr().unwrap();
    let invitation = service
        .create_invitation(&address.to_string(), 800, 20_000)
        .unwrap();
    let accepting_service = Arc::clone(&service);
    let pending_request = thread::spawn(move || accepting_service.accept_request(801));
    let sender_identity = TestIdentity::new([0x22; 32]);
    let pending_client = WssPairingClient::begin_pairing(
        invitation.uri(),
        20_000,
        &[address.port()],
        SENDER_ID,
        "Windows sender",
        view_grant(),
        &sender_identity,
        20_001_000,
    )
    .unwrap();
    let pending = pending_request.join().unwrap().unwrap();

    pending.reject("user_rejected", 20_001_001).unwrap();
    assert!(matches!(
        pending_client.wait_for_decision(),
        Err(mm_adapter_signaling::WssClientError::Rejected(reason))
            if reason == "user_rejected"
    ));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());
}

#[test]
fn stopping_receiver_service_cancels_an_idle_pairing_accept() {
    let receiver_identity = Arc::new(TestIdentity::new([0x51; 32]));
    let peers = Arc::new(MemoryPeers::default());
    let runtime =
        mm_runtime::PairingRuntime::new(RECEIVER_ID, Arc::clone(&receiver_identity), peers)
            .unwrap();
    let service = Arc::new(
        AndroidTvPairingService::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            runtime,
            receiver_identity,
        )
        .unwrap(),
    );

    let waiting_service = Arc::clone(&service);
    let waiting = thread::spawn(move || waiting_service.accept_request(901));
    thread::sleep(std::time::Duration::from_millis(20));
    service.stop_accepting();

    assert!(matches!(
        waiting.join().unwrap(),
        Err(super::AndroidTvPairingServiceError::Listener(
            mm_adapter_signaling::WssListenerError::Cancelled
        ))
    ));
    assert!(!service.is_accepting());
}

#[test]
fn cancelling_receiver_wait_releases_runtime_for_a_fresh_single_use_invitation() {
    let receiver_identity = Arc::new(TestIdentity::new([0x61; 32]));
    let peers = Arc::new(MemoryPeers::default());
    let runtime =
        mm_runtime::PairingRuntime::new(RECEIVER_ID, Arc::clone(&receiver_identity), peers)
            .unwrap();
    let service = Arc::new(
        AndroidTvPairingService::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            runtime,
            receiver_identity,
        )
        .unwrap(),
    );

    let waiting_service = Arc::clone(&service);
    let waiting = thread::spawn(move || waiting_service.accept_request(1001));
    thread::sleep(std::time::Duration::from_millis(20));
    service.cancel_wait();
    assert!(service.is_accepting());
    assert!(matches!(
        waiting.join().unwrap(),
        Err(super::AndroidTvPairingServiceError::Listener(
            mm_adapter_signaling::WssListenerError::Cancelled
        ))
    ));
    assert!(service.is_accepting());

    let address = service.local_addr().unwrap();
    let refreshed = service
        .create_invitation(&address.to_string(), 1002, 30_000)
        .unwrap();
    assert!(refreshed.expires_at_unix_seconds() > 30_000);
}
