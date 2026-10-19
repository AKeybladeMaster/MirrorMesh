use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mm_domain::{
    DeviceId, PeerFingerprint, PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
use mm_security::{
    NonceSourceError, PairingNonce, PairingNonceSource, PendingPairingNonces, parse_pairing_qr,
};

use super::{
    MAX_PAIRING_SIGNATURE_BYTES, PairingCoordinator, PairingCoordinatorError, PairingProof,
    PairingProofSigningError,
};

const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);

struct FixedNonceSource(u128);

impl PairingNonceSource for FixedNonceSource {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), NonceSourceError> {
        *output = self.0.to_le_bytes();
        self.0 += 1;
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MemoryPeers(Arc<Mutex<HashMap<DeviceId, TrustedPeer>>>);

impl TrustedPeerPort for MemoryPeers {
    fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError> {
        self.0
            .lock()
            .map_err(|_| PortError::StorageFailure)
            .map(|peers| peers.get(&peer_id).cloned())
    }

    fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError> {
        self.0
            .lock()
            .map_err(|_| PortError::StorageFailure)?
            .insert(peer.id(), peer.clone());
        Ok(())
    }

    fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError> {
        self.0
            .lock()
            .map_err(|_| PortError::StorageFailure)
            .map(|peers| peers.values().cloned().collect())
    }

    fn revoke(&self, peer_id: DeviceId, revoked_at_unix_ms: u64) -> Result<(), PortError> {
        if let Some(peer) = self
            .0
            .lock()
            .map_err(|_| PortError::StorageFailure)?
            .get_mut(&peer_id)
        {
            peer.revoke(revoked_at_unix_ms)
                .map_err(|_| PortError::StorageFailure)?;
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeIdentity {
    verified_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    signed_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    fail_signing: bool,
}

impl LocalPairingIdentityPort for FakeIdentity {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        PeerPublicKey::new(vec![9; 32]).map_err(|_| PortError::IdentityRejected)
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        if self.fail_signing {
            return Err(PortError::Unavailable);
        }
        self.signed_messages
            .lock()
            .map_err(|_| PortError::IdentityRejected)?
            .push(message.to_vec());
        Ok(b"valid-signature".to_vec())
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        self.local_public_key()
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        Ok(())
    }
}

impl PairingIdentityPort for FakeIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError> {
        let first = *public_key
            .as_bytes()
            .first()
            .ok_or(PortError::IdentityRejected)?;
        Ok(PeerFingerprint::from_bytes([first; 32]))
    }

    fn verify_signature(
        &self,
        _public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        if signature != b"valid-signature" {
            return Err(PortError::IdentityRejected);
        }
        self.verified_messages
            .lock()
            .map_err(|_| PortError::IdentityRejected)?
            .push(message.to_vec());
        Ok(())
    }
}

fn coordinator(
    peers: &MemoryPeers,
    identity: &FakeIdentity,
) -> PairingCoordinator<MemoryPeers, FakeIdentity> {
    PairingCoordinator::new(
        RECEIVER_ID,
        PeerFingerprint::from_bytes([9; 32]),
        PendingPairingNonces::with_source(FixedNonceSource(7)),
        Arc::new(identity.clone()),
        Arc::new(peers.clone()),
    )
}

fn invitation(coordinator: &mut PairingCoordinator<MemoryPeers, FakeIdentity>) -> String {
    coordinator
        .create_invitation("192.168.1.5:8765", &[8765], 100, 1_000)
        .unwrap()
        .uri()
        .to_owned()
}

fn proof(uri: &str, signature: &[u8], grants: TrustedPeerGrants) -> PairingProof {
    let qr = parse_pairing_qr(uri, 1_000, &[8765]).unwrap();
    PairingProof::new(
        RECEIVER_ID,
        qr.nonce(),
        SENDER_ID,
        "Sender laptop".to_owned(),
        PeerPublicKey::new(vec![7; 32]).unwrap(),
        grants,
        signature.to_vec(),
    )
    .unwrap()
}

#[test]
fn creates_sender_proof_from_the_canonical_message_using_local_identity() {
    let identity = FakeIdentity::default();
    let nonce = PairingNonce::from_bytes([7; 16]);
    let grants = TrustedPeerGrants::new(true, false, true).unwrap();

    let proof = PairingProof::create_signed(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        grants,
        &identity,
    )
    .unwrap();

    assert_eq!(proof.receiver_id(), RECEIVER_ID);
    assert_eq!(proof.peer_id(), SENDER_ID);
    assert_eq!(proof.nonce(), nonce);
    assert_eq!(proof.public_key().as_bytes(), &[9; 32]);
    assert_eq!(proof.requested_grants(), grants);
    assert_eq!(proof.signature(), b"valid-signature");
    assert_eq!(
        *identity.signed_messages.lock().unwrap(),
        vec![proof.signing_message()]
    );
}

#[test]
fn refuses_self_pairing_before_calling_the_local_signer() {
    let identity = FakeIdentity::default();

    assert_eq!(
        PairingProof::create_signed(
            RECEIVER_ID,
            PairingNonce::from_bytes([8; 16]),
            RECEIVER_ID,
            "Sender laptop".to_owned(),
            TrustedPeerGrants::new(true, false, false).unwrap(),
            &identity,
        )
        .unwrap_err(),
        PairingProofSigningError::SelfPairing
    );
    assert!(identity.signed_messages.lock().unwrap().is_empty());
}

#[test]
fn reports_local_signer_failure_without_returning_a_proof() {
    let identity = FakeIdentity {
        fail_signing: true,
        ..FakeIdentity::default()
    };

    assert_eq!(
        PairingProof::create_signed(
            RECEIVER_ID,
            PairingNonce::from_bytes([9; 16]),
            SENDER_ID,
            "Sender laptop".to_owned(),
            TrustedPeerGrants::new(true, false, false).unwrap(),
            &identity,
        )
        .unwrap_err(),
        PairingProofSigningError::Identity(PortError::Unavailable)
    );
    assert!(identity.signed_messages.lock().unwrap().is_empty());
}

fn requested_grants() -> TrustedPeerGrants {
    TrustedPeerGrants::new(true, false, true).unwrap()
}

#[test]
fn signed_proof_message_is_domain_separated_and_binds_requested_grants() {
    let nonce = PairingNonce::from_bytes([3; 16]);
    let public_key = PeerPublicKey::new(vec![7; 32]).unwrap();
    let view_and_annotate = PairingProof::new(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        public_key.clone(),
        requested_grants(),
        b"valid-signature".to_vec(),
    )
    .unwrap();
    let view_only = PairingProof::new(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        public_key,
        TrustedPeerGrants::new(true, false, false).unwrap(),
        b"valid-signature".to_vec(),
    )
    .unwrap();

    assert!(
        view_and_annotate
            .signing_message()
            .starts_with(b"MirrorMesh.PairingProof.v1\0")
    );
    assert_ne!(
        view_and_annotate.signing_message(),
        view_only.signing_message()
    );
}

#[test]
fn invitation_contains_a_parseable_canonical_uri_and_bounded_expiry() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);

    let uri = invitation(&mut coordinator);
    let parsed = parse_pairing_qr(&uri, 1_000, &[8765]).unwrap();

    assert_eq!(parsed.device_id(), RECEIVER_ID);
    assert_eq!(parsed.fingerprint(), PeerFingerprint::from_bytes([9; 32]));
    assert_eq!(parsed.expires_at_unix_seconds(), 1_120);
}

#[test]
fn peer_is_not_saved_until_explicit_approval_and_approval_can_reduce_scopes() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);
    let uri = invitation(&mut coordinator);
    let proof = proof(&uri, b"valid-signature", requested_grants());

    let pending = coordinator.verify_proof(proof, 101).unwrap();
    assert!(peers.list_peers().unwrap().is_empty());
    assert!(pending.requested_grants().allows(PeerGrant::Annotate));

    let approved = TrustedPeerGrants::new(true, false, false).unwrap();
    let stored = coordinator.approve(pending, approved, 102, 10_001).unwrap();

    assert_eq!(stored.id(), SENDER_ID);
    assert!(stored.allows(PeerGrant::View));
    assert!(!stored.allows(PeerGrant::Annotate));
    assert_eq!(peers.list_peers().unwrap(), vec![stored]);
}

#[test]
fn pending_approval_cannot_be_persisted_by_a_different_receiver() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut original = coordinator(&peers, &identity);
    let uri = invitation(&mut original);
    let pending = original
        .verify_proof(proof(&uri, b"valid-signature", requested_grants()), 101)
        .unwrap();
    let other_receiver = PairingCoordinator::new(
        DeviceId::from_bytes([4; 16]),
        PeerFingerprint::from_bytes([8; 32]),
        PendingPairingNonces::with_source(FixedNonceSource(9)),
        Arc::new(identity.clone()),
        Arc::new(peers.clone()),
    );

    assert_eq!(
        other_receiver.approve(pending, requested_grants(), 102, 10_001),
        Err(PairingCoordinatorError::WrongReceiver)
    );
    assert!(peers.list_peers().unwrap().is_empty());
}

#[test]
fn invalid_signature_does_not_create_trust_and_nonce_replay_is_rejected() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);
    let uri = invitation(&mut coordinator);

    assert_eq!(
        coordinator.verify_proof(proof(&uri, b"invalid", requested_grants()), 101),
        Err(PairingCoordinatorError::IdentityRejected)
    );
    assert!(peers.list_peers().unwrap().is_empty());

    let valid_proof = proof(&uri, b"valid-signature", requested_grants());
    let pending = coordinator.verify_proof(valid_proof, 102).unwrap();
    let replay = proof(&uri, b"valid-signature", requested_grants());
    assert_eq!(
        coordinator.verify_proof(replay, 103),
        Err(PairingCoordinatorError::NonceRejected)
    );
    drop(pending);
    assert!(peers.list_peers().unwrap().is_empty());
}

#[test]
fn unknown_nonce_is_rejected_before_signature_verification() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);
    let _uri = invitation(&mut coordinator);
    let proof = PairingProof::new(
        RECEIVER_ID,
        PairingNonce::from_bytes([88; 16]),
        SENDER_ID,
        "Sender laptop".to_owned(),
        PeerPublicKey::new(vec![7; 32]).unwrap(),
        requested_grants(),
        b"valid-signature".to_vec(),
    )
    .unwrap();

    assert_eq!(
        coordinator.verify_proof(proof, 101),
        Err(PairingCoordinatorError::NonceRejected)
    );
    assert!(identity.verified_messages.lock().unwrap().is_empty());
}

#[test]
fn approval_expires_with_the_original_nonce_deadline() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);
    let uri = invitation(&mut coordinator);
    let pending = coordinator
        .verify_proof(proof(&uri, b"valid-signature", requested_grants()), 101)
        .unwrap();

    assert_eq!(
        coordinator.approve(pending, requested_grants(), 120_100, 10_001),
        Err(PairingCoordinatorError::ApprovalExpired)
    );
    assert!(peers.list_peers().unwrap().is_empty());
}

#[test]
fn approval_cannot_grant_a_scope_the_sender_did_not_request() {
    let peers = MemoryPeers::default();
    let identity = FakeIdentity::default();
    let mut coordinator = coordinator(&peers, &identity);
    let uri = invitation(&mut coordinator);
    let requested = TrustedPeerGrants::new(true, false, false).unwrap();
    let pending = coordinator
        .verify_proof(proof(&uri, b"valid-signature", requested), 101)
        .unwrap();

    assert_eq!(
        coordinator.approve(
            pending,
            TrustedPeerGrants::new(false, true, false).unwrap(),
            102,
            10_001
        ),
        Err(PairingCoordinatorError::GrantNotRequested)
    );
    assert!(peers.list_peers().unwrap().is_empty());
}

#[test]
fn proof_size_is_bounded_before_signature_verification() {
    let signature = vec![1; MAX_PAIRING_SIGNATURE_BYTES + 1];
    assert!(
        PairingProof::new(
            RECEIVER_ID,
            PairingNonce::from_bytes([1; 16]),
            SENDER_ID,
            "Sender".to_owned(),
            PeerPublicKey::new(vec![7; 32]).unwrap(),
            requested_grants(),
            signature
        )
        .is_err()
    );
}
