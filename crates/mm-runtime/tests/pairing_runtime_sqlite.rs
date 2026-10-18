use std::net::{IpAddr, Ipv4Addr};

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_application::PairingProof;
use mm_domain::{DeviceId, PeerGrant, PeerPublicKey, TrustedPeerGrants};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
use mm_runtime::{PairingRuntime, PairingRuntimeError};
use mm_security::{PairingNonce, parse_pairing_qr};
use mm_store::SqliteTrustedPeers;

const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);

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
    runtime: &mut PairingRuntime<'_, SqliteTrustedPeers, TestIdentity>,
) -> PairingNonce {
    let invitation = runtime
        .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
        .unwrap();
    parse_pairing_qr(invitation.uri(), 200, &[8765])
        .unwrap()
        .nonce()
}

fn signed_proof(nonce: PairingNonce, tamper_signature: bool) -> PairingProof {
    let sender_key = SigningKey::from_bytes(&[0x42; 32]);
    let sender_public_key =
        PeerPublicKey::new(sender_key.verifying_key().to_bytes().to_vec()).unwrap();
    let grants = TrustedPeerGrants::new(true, true, true).unwrap();
    let unsigned = PairingProof::new(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        sender_public_key.clone(),
        grants,
        vec![0; 64],
    )
    .unwrap();
    let mut signature = sender_key.sign(&unsigned.signing_message()).to_bytes();
    if tamper_signature {
        signature[0] ^= 1;
    }

    PairingProof::new(
        RECEIVER_ID,
        nonce,
        SENDER_ID,
        "Sender laptop".to_owned(),
        sender_public_key,
        grants,
        signature.to_vec(),
    )
    .unwrap()
}

#[test]
fn verifies_real_signature_and_persists_sqlite_trust_only_after_approval() {
    let identity = TestIdentity::new([0x11; 32]);
    let peers = SqliteTrustedPeers::open_in_memory().unwrap();
    let mut runtime = PairingRuntime::new(RECEIVER_ID, &identity, &peers).unwrap();
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
    let identity = TestIdentity::new([0x11; 32]);
    let peers = SqliteTrustedPeers::open_in_memory().unwrap();
    let mut runtime = PairingRuntime::new(RECEIVER_ID, &identity, &peers).unwrap();
    let nonce = invitation_nonce(&mut runtime);

    assert!(matches!(
        runtime.verify_proof_from(source_ip(), signed_proof(nonce, true), 101),
        Err(PairingRuntimeError::Coordinator(
            mm_application::PairingCoordinatorError::IdentityRejected
        ))
    ));
    assert!(peers.get_peer(SENDER_ID).unwrap().is_none());
}
