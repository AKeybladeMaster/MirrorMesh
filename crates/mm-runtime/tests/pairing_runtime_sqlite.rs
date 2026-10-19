use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_application::PairingProof;
use mm_domain::{DeviceId, MessageId, PeerGrant, PeerPublicKey, SessionId, TrustedPeerGrants};
use mm_ports::{
    LocalPairingIdentityPort, PairingIdentityPort, PortError, TransportPort, TrustedPeerPort,
};
use mm_protocol::{Envelope, PairBegin, encode_pair_begin};
use mm_runtime::{MirrorMeshRuntime, PairingRuntime, PairingRuntimeError, TrustedSessionError};
use mm_security::{PairingNonce, parse_pairing_qr};
use mm_store::SqliteTrustedPeers;

const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);

fn view_scope() -> TrustedPeerGrants {
    TrustedPeerGrants::new(true, false, false).unwrap()
}

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<TransportLog>>);

#[derive(Default)]
struct TransportLog {
    started: Vec<(SessionId, DeviceId)>,
    closed: Vec<SessionId>,
}

impl TransportPort for RecordingTransport {
    fn start(&mut self, session_id: SessionId, peer_id: DeviceId) -> Result<(), PortError> {
        self.0.lock().unwrap().started.push((session_id, peer_id));
        Ok(())
    }

    fn send_control(&mut self, _message: &Envelope<Vec<u8>>) -> Result<(), PortError> {
        Ok(())
    }

    fn close(&mut self, session_id: SessionId) -> Result<(), PortError> {
        self.0.lock().unwrap().closed.push(session_id);
        Ok(())
    }
}

fn source_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))
}

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

impl PairingIdentityPort for TestIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<mm_domain::PeerFingerprint, PortError> {
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

fn invitation_nonce(
    runtime: &mut PairingRuntime<SqliteTrustedPeers, TestIdentity>,
) -> PairingNonce {
    let invitation = runtime
        .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
        .unwrap();
    parse_pairing_qr(invitation.uri(), 200, &[8765])
        .unwrap()
        .nonce()
}

fn signed_proof(nonce: PairingNonce, tamper_signature: bool) -> PairingProof {
    let sender_identity = TestIdentity::new([0x42; 32]);
    let grants = TrustedPeerGrants::new(true, true, true).unwrap();
    let proof = PairingProof::create_signed(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        grants,
        &sender_identity,
    )
    .unwrap();
    if !tamper_signature {
        return proof;
    }

    let mut signature = proof.signature().to_vec();
    signature[0] ^= 1;
    PairingProof::new(
        proof.receiver_id(),
        proof.nonce(),
        proof.peer_id(),
        proof.display_name().to_owned(),
        proof.public_key().clone(),
        proof.requested_grants(),
        signature,
    )
    .unwrap()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn signed_pair_begin_frame(nonce: PairingNonce, tamper_signature: bool) -> Vec<u8> {
    let proof = signed_proof(nonce, tamper_signature);
    let grants = proof.requested_grants();
    let requested_grants = [
        (PeerGrant::View, "view"),
        (PeerGrant::Send, "send"),
        (PeerGrant::Annotate, "annotate"),
    ]
    .into_iter()
    .filter(|(grant, _)| grants.allows(*grant))
    .map(|(_, name)| name.to_owned())
    .collect();
    let payload = PairBegin {
        receiver_id: proof.receiver_id().to_string(),
        nonce: proof.nonce().to_hex(),
        peer_id: proof.peer_id().to_string(),
        display_name: proof.display_name().to_owned(),
        public_key: hex(proof.public_key().as_bytes()),
        requested_grants,
        signature: hex(proof.signature()),
    };

    encode_pair_begin(MessageId::from_bytes([0xb8; 16]), 1, 100, &payload).unwrap()
}

#[test]
fn verifies_real_signature_and_persists_sqlite_trust_only_after_approval() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let mut runtime =
        PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers)).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    let pending = runtime
        .verify_proof_from(source_ip(), signed_proof(nonce, false), 101)
        .unwrap();
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());

    let approved_grants = TrustedPeerGrants::new(true, false, false).unwrap();
    let approved = runtime
        .approve(pending, approved_grants, 102, 1_000)
        .unwrap();

    assert_eq!(peers.get_peer(SENDER_ID).unwrap(), Some(approved.clone()));
    assert!(approved.grants().allows(PeerGrant::View));
    assert!(!approved.grants().allows(PeerGrant::Send));
    assert_eq!(
        approved.fingerprint(),
        Ed25519PairingIdentity
            .fingerprint_public_key(approved.public_key())
            .unwrap()
    );
}

#[test]
fn rejects_modified_real_signature_without_persisting_trust() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let mut runtime =
        PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers)).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    assert!(matches!(
        runtime.verify_proof_from(source_ip(), signed_proof(nonce, true), 101),
        Err(PairingRuntimeError::Coordinator(
            mm_application::PairingCoordinatorError::IdentityRejected
        ))
    ));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());
}

#[test]
fn verifies_signed_pair_begin_frame_without_persisting_before_approval() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let mut runtime =
        PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers)).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    let pending = runtime
        .verify_pair_begin_from(source_ip(), &signed_pair_begin_frame(nonce, false), 101)
        .unwrap();
    assert_eq!(pending.approval().peer_id(), SENDER_ID);
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());

    let approved = runtime.approve(pending, view_scope(), 102, 1_000).unwrap();
    assert_eq!(peers.get_peer(SENDER_ID).unwrap(), Some(approved));
}

#[test]
fn rejects_tampered_pair_begin_frame_without_persisting_trust() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let mut runtime =
        PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers)).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    assert!(matches!(
        runtime.verify_pair_begin_from(source_ip(), &signed_pair_begin_frame(nonce, true), 101),
        Err(PairingRuntimeError::Coordinator(
            mm_application::PairingCoordinatorError::IdentityRejected
        ))
    ));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());
}

#[test]
fn malformed_frames_consume_the_source_attempt_budget_before_parsing() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let mut runtime =
        PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers)).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    for _ in 0..5 {
        assert!(matches!(
            runtime.verify_pair_begin_from(source_ip(), &[], 101),
            Err(PairingRuntimeError::Protocol(
                mm_protocol::ProtocolError::EmptyFrame
            ))
        ));
    }
    assert!(matches!(
        runtime.verify_pair_begin_from(source_ip(), &signed_pair_begin_frame(nonce, false), 101),
        Err(PairingRuntimeError::CoolingDown { .. })
    ));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());
}

#[test]
fn composed_runtime_pairs_starts_and_revokes_through_one_store() {
    let identity = Arc::new(TestIdentity::new([0x11; 32]));
    let peers = Arc::new(SqliteTrustedPeers::open_in_memory().unwrap());
    let transport = RecordingTransport::default();
    let transport_log = Arc::clone(&transport.0);
    let mut runtime = MirrorMeshRuntime::new(
        RECEIVER_ID,
        Arc::clone(&identity),
        Arc::clone(&peers),
        transport,
    )
    .unwrap();
    let session_id = SessionId::from_bytes([9; 16]);

    assert_eq!(
        runtime.start_session(SENDER_ID, session_id, view_scope()),
        Err(TrustedSessionError::PeerNotTrusted)
    );

    let invitation = runtime
        .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
        .unwrap();
    let nonce = parse_pairing_qr(invitation.uri(), 200, &[8765])
        .unwrap()
        .nonce();
    let pending = runtime
        .verify_pair_begin_from(source_ip(), &signed_pair_begin_frame(nonce, false), 101)
        .unwrap();
    assert!(!peers.is_trusted(SENDER_ID).unwrap());

    runtime
        .approve(
            pending,
            TrustedPeerGrants::new(true, false, false).unwrap(),
            102,
            1_000,
        )
        .unwrap();
    assert!(peers.is_trusted(SENDER_ID).unwrap());

    let send_scope = TrustedPeerGrants::new(false, true, false).unwrap();
    assert_eq!(
        runtime.start_session(SENDER_ID, session_id, send_scope),
        Err(TrustedSessionError::GrantNotAuthorized)
    );
    assert!(transport_log.lock().unwrap().started.is_empty());

    let granted_view_scope = TrustedPeerGrants::new(true, false, false).unwrap();
    runtime
        .start_session(SENDER_ID, session_id, granted_view_scope)
        .unwrap();
    runtime.revoke_peer(SENDER_ID, 2_000).unwrap();
    assert!(!peers.is_trusted(SENDER_ID).unwrap());
    assert_eq!(transport_log.lock().unwrap().closed, vec![session_id]);

    let restarted_transport = RecordingTransport::default();
    let restarted_log = Arc::clone(&restarted_transport.0);
    let mut restarted = MirrorMeshRuntime::new(
        RECEIVER_ID,
        Arc::clone(&identity),
        Arc::clone(&peers),
        restarted_transport,
    )
    .unwrap();
    assert_eq!(
        restarted.start_session(SENDER_ID, SessionId::from_bytes([10; 16]), view_scope(),),
        Err(TrustedSessionError::PeerNotTrusted)
    );
    assert!(restarted_log.lock().unwrap().started.is_empty());
}
