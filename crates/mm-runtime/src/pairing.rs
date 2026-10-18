use mm_application::{
    PairingCoordinator, PairingCoordinatorError, PairingInvitation, PairingProof,
    PendingPairingApproval,
};
use mm_domain::{DeviceId, TrustedPeer, TrustedPeerGrants};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
use mm_security::PendingPairingNonces;

/// Runtime composition for the consent-gated pairing flow.
///
/// This runtime captures the local key fingerprint when it is constructed. Drop
/// and recreate it after rotating or deleting the local identity before issuing
/// another invitation.
pub struct PairingRuntime<'a, P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
{
    coordinator: PairingCoordinator<'a, P, I>,
}

impl<'a, P, I> PairingRuntime<'a, P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
{
    /// Initializes pairing from the current platform identity and trusted-peer store.
    ///
    /// The nonce registry uses the operating system random source. Identity errors
    /// fail initialization; there is no ephemeral-key fallback.
    pub fn new(
        local_device_id: DeviceId,
        identity: &'a I,
        peers: &'a P,
    ) -> Result<Self, PortError> {
        let local_public_key = identity.local_public_key()?;
        let local_fingerprint = identity.fingerprint_public_key(&local_public_key)?;
        let coordinator = PairingCoordinator::new(
            local_device_id,
            local_fingerprint,
            PendingPairingNonces::new(),
            identity,
            peers,
        );
        Ok(Self { coordinator })
    }

    /// Creates a user-displayable invitation for an explicitly approved endpoint.
    pub fn create_invitation(
        &mut self,
        endpoint_text: &str,
        approved_ports: &[u16],
        now_monotonic_ms: u64,
        now_unix_seconds: u64,
    ) -> Result<PairingInvitation, PairingCoordinatorError> {
        self.coordinator.create_invitation(
            endpoint_text,
            approved_ports,
            now_monotonic_ms,
            now_unix_seconds,
        )
    }

    /// Verifies possession and returns a pending approval without persisting trust.
    pub fn verify_proof(
        &mut self,
        proof: PairingProof,
        now_monotonic_ms: u64,
    ) -> Result<PendingPairingApproval, PairingCoordinatorError> {
        self.coordinator.verify_proof(proof, now_monotonic_ms)
    }

    /// Persists only the explicitly approved subset of the sender's requested grants.
    pub fn approve(
        &self,
        pending: PendingPairingApproval,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<TrustedPeer, PairingCoordinatorError> {
        self.coordinator.approve(
            pending,
            approved_grants,
            now_monotonic_ms,
            paired_at_unix_ms,
        )
    }
}
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use mm_application::{PairingCoordinatorError, PairingProof};
    use mm_domain::{
        DeviceId, PeerFingerprint, PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
    };
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
    use mm_security::{PairingNonce, parse_pairing_qr};

    use super::PairingRuntime;

    const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
    const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);

    struct FakeIdentity {
        local_key: PeerPublicKey,
        fail_local_key: bool,
        fail_fingerprint: bool,
        reject_signature: bool,
    }

    impl FakeIdentity {
        fn new(local_key: PeerPublicKey) -> Self {
            Self {
                local_key,
                fail_local_key: false,
                fail_fingerprint: false,
                reject_signature: false,
            }
        }

        fn failing_local_key() -> Self {
            Self {
                local_key: PeerPublicKey::new(vec![9; 32]).unwrap(),
                fail_local_key: true,
                fail_fingerprint: false,
                reject_signature: false,
            }
        }
    }

    impl LocalPairingIdentityPort for FakeIdentity {
        fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
            if self.fail_local_key {
                return Err(PortError::Unavailable);
            }
            Ok(self.local_key.clone())
        }

        fn sign_pairing_message(&self, _message: &[u8]) -> Result<Vec<u8>, PortError> {
            Ok(vec![1; 64])
        }

        fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
            Ok(self.local_key.clone())
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
            if self.fail_fingerprint {
                return Err(PortError::StorageFailure);
            }
            let first_byte = *public_key
                .as_bytes()
                .first()
                .ok_or(PortError::IdentityRejected)?;
            Ok(PeerFingerprint::from_bytes([first_byte; 32]))
        }

        fn verify_signature(
            &self,
            _public_key: &PeerPublicKey,
            _message: &[u8],
            signature: &[u8],
        ) -> Result<(), PortError> {
            if self.reject_signature || signature.len() != 64 {
                return Err(PortError::IdentityRejected);
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeTrustedPeers {
        peers: Mutex<Vec<TrustedPeer>>,
        fail_save: bool,
    }

    impl FakeTrustedPeers {
        fn failing_save() -> Self {
            Self {
                peers: Mutex::new(Vec::new()),
                fail_save: true,
            }
        }
    }

    impl TrustedPeerPort for FakeTrustedPeers {
        fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError> {
            Ok(self
                .peers
                .lock()
                .map_err(|_| PortError::StorageFailure)?
                .iter()
                .find(|peer| peer.id() == peer_id)
                .cloned())
        }

        fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError> {
            if self.fail_save {
                return Err(PortError::StorageFailure);
            }
            self.peers
                .lock()
                .map_err(|_| PortError::StorageFailure)?
                .push(peer.clone());
            Ok(())
        }

        fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError> {
            Ok(self
                .peers
                .lock()
                .map_err(|_| PortError::StorageFailure)?
                .clone())
        }

        fn revoke(&self, _peer_id: DeviceId, _revoked_at_unix_ms: u64) -> Result<(), PortError> {
            Ok(())
        }
    }

    fn requested_grants() -> TrustedPeerGrants {
        TrustedPeerGrants::new(true, true, true).unwrap()
    }

    fn pairing_proof(nonce: PairingNonce) -> PairingProof {
        PairingProof::new(
            RECEIVER_ID,
            nonce,
            SENDER_ID,
            "Sender laptop".to_owned(),
            PeerPublicKey::new(vec![7; 32]).unwrap(),
            requested_grants(),
            vec![1; 64],
        )
        .unwrap()
    }

    fn invitation_nonce(invitation: &str) -> PairingNonce {
        parse_pairing_qr(invitation, 200, &[8765]).unwrap().nonce()
    }

    #[test]
    fn derives_local_fingerprint_and_persists_only_after_explicit_approval() {
        let identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        let store = FakeTrustedPeers::default();
        let mut runtime = PairingRuntime::new(RECEIVER_ID, &identity, &store).unwrap();

        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let payload = parse_pairing_qr(invitation.uri(), 200, &[8765]).unwrap();
        assert_eq!(payload.fingerprint(), PeerFingerprint::from_bytes([9; 32]));

        let pending = runtime
            .verify_proof(pairing_proof(payload.nonce()), 101)
            .unwrap();
        assert!(store.list_peers().unwrap().is_empty());

        let approved_grants = TrustedPeerGrants::new(true, false, false).unwrap();
        let peer = runtime.approve(pending, approved_grants, 102, 500).unwrap();

        assert_eq!(store.list_peers().unwrap(), vec![peer.clone()]);
        assert!(peer.grants().allows(PeerGrant::View));
        assert!(!peer.grants().allows(PeerGrant::Send));
    }

    #[test]
    fn refuses_to_start_when_local_identity_is_unavailable() {
        let identity = FakeIdentity::failing_local_key();
        let store = FakeTrustedPeers::default();

        assert!(matches!(
            PairingRuntime::new(RECEIVER_ID, &identity, &store),
            Err(PortError::Unavailable)
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn refuses_to_start_when_local_fingerprint_is_unavailable() {
        let mut identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        identity.fail_fingerprint = true;
        let store = FakeTrustedPeers::default();

        assert!(matches!(
            PairingRuntime::new(RECEIVER_ID, &identity, &store),
            Err(PortError::StorageFailure)
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn rejects_sender_signature_without_persisting_trust() {
        let mut identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        identity.reject_signature = true;
        let store = FakeTrustedPeers::default();
        let mut runtime = PairingRuntime::new(RECEIVER_ID, &identity, &store).unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());

        assert!(matches!(
            runtime.verify_proof(pairing_proof(nonce), 101),
            Err(PairingCoordinatorError::IdentityRejected)
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn reports_storage_failure_only_after_explicit_approval() {
        let identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        let store = FakeTrustedPeers::failing_save();
        let mut runtime = PairingRuntime::new(RECEIVER_ID, &identity, &store).unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());
        let pending = runtime.verify_proof(pairing_proof(nonce), 101).unwrap();

        assert!(store.list_peers().unwrap().is_empty());
        assert_eq!(
            runtime.approve(pending, requested_grants(), 102, 500),
            Err(PairingCoordinatorError::StoreFailure)
        );
        assert!(store.list_peers().unwrap().is_empty());
    }
}
