use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use mm_application::{
    PairingCoordinator, PairingCoordinatorError, PairingInvitation, PairingProof,
    PendingPairingApproval,
};
use mm_domain::{DeviceId, PeerFingerprint, PeerPublicKey, TrustedPeer, TrustedPeerGrants};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
use mm_protocol::{MessagePayload, ProtocolError, parse_message};
use mm_security::{
    PairingAttemptDecision, PairingAttemptLimiter, PairingAttemptLimiterError, PairingNonce,
    PendingPairingNonces,
};

#[derive(Debug, PartialEq, Eq)]
pub struct AdmittedPendingPairingApproval {
    source_ip: IpAddr,
    approval: PendingPairingApproval,
}

impl AdmittedPendingPairingApproval {
    pub const fn source_ip(&self) -> IpAddr {
        self.source_ip
    }

    pub const fn approval(&self) -> &PendingPairingApproval {
        &self.approval
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingRuntimeError {
    Coordinator(PairingCoordinatorError),
    Protocol(ProtocolError),
    InvalidPairBegin,
    UnexpectedMessageType,
    AdmissionState(PairingAttemptLimiterError),
    AdmissionCleanup(PairingAttemptLimiterError),
    CoolingDown { retry_after_ms: u64 },
    CapacityLimited,
}

impl fmt::Display for PairingRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordinator(error) => write!(formatter, "pairing coordinator failed: {error:?}"),
            Self::Protocol(error) => write!(formatter, "pairing frame is invalid: {error}"),
            Self::InvalidPairBegin => formatter.write_str("pair.begin proof fields are invalid"),
            Self::UnexpectedMessageType => {
                formatter.write_str("pairing frame is not a pair.begin message")
            }
            Self::AdmissionState(error) => write!(formatter, "pairing admission failed: {error}"),
            Self::AdmissionCleanup(error) => {
                write!(
                    formatter,
                    "peer was saved but admission cleanup failed: {error}"
                )
            }
            Self::CoolingDown { retry_after_ms } => {
                write!(
                    formatter,
                    "pairing source is cooling down for {retry_after_ms} ms"
                )
            }
            Self::CapacityLimited => formatter.write_str("pairing source capacity is full"),
        }
    }
}

impl std::error::Error for PairingRuntimeError {}

impl From<PairingCoordinatorError> for PairingRuntimeError {
    fn from(error: PairingCoordinatorError) -> Self {
        Self::Coordinator(error)
    }
}

impl From<PairingAttemptLimiterError> for PairingRuntimeError {
    fn from(error: PairingAttemptLimiterError) -> Self {
        Self::AdmissionState(error)
    }
}

/// Runtime composition for the consent-gated pairing flow.
///
/// This runtime captures the local key fingerprint when it is constructed. Drop
/// and recreate it after rotating or deleting the local identity before issuing
/// another invitation. Keep it alive for the listener lifetime so its in-memory
/// per-source attempt budgets are not reset between requests.
pub struct PairingRuntime<P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
{
    coordinator: PairingCoordinator<P, I>,
    attempt_limiter: PairingAttemptLimiter,
}

impl<P, I> PairingRuntime<P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
{
    /// Initializes pairing from the current platform identity and trusted-peer store.
    ///
    /// The runtime retains shared ownership of both ports. Keep this instance alive across
    /// listener callbacks to preserve its invitation nonces and attempt limiter.
    ///
    /// The nonce registry uses the operating system random source. Identity errors
    /// fail initialization; there is no ephemeral-key fallback.
    pub fn new(
        local_device_id: DeviceId,
        identity: Arc<I>,
        peers: Arc<P>,
    ) -> Result<Self, PortError> {
        Self::new_with_limiter(
            local_device_id,
            identity,
            peers,
            PairingAttemptLimiter::default(),
        )
    }

    /// Initializes pairing with an injected attempt limiter for explicit policy ownership.
    pub fn new_with_limiter(
        local_device_id: DeviceId,
        identity: Arc<I>,
        peers: Arc<P>,
        attempt_limiter: PairingAttemptLimiter,
    ) -> Result<Self, PortError> {
        let local_public_key = identity.local_public_key()?;
        let local_fingerprint = identity.fingerprint_public_key(&local_public_key)?;
        let coordinator = PairingCoordinator::new(
            local_device_id,
            local_fingerprint,
            PendingPairingNonces::new(),
            Arc::clone(&identity),
            peers,
        );
        Ok(Self {
            coordinator,
            attempt_limiter,
        })
    }

    /// Returns the local key fingerprint captured when this runtime was created.
    pub const fn local_fingerprint(&self) -> PeerFingerprint {
        self.coordinator.local_fingerprint()
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

    /// Reserves an attempt before proof verification and binds the source to pending approval.
    pub fn verify_proof_from(
        &mut self,
        source_ip: IpAddr,
        proof: PairingProof,
        now_monotonic_ms: u64,
    ) -> Result<AdmittedPendingPairingApproval, PairingRuntimeError> {
        self.reserve_source_attempt(source_ip, now_monotonic_ms)?;
        self.verify_admitted_proof(source_ip, proof, now_monotonic_ms)
    }

    fn reserve_source_attempt(
        &self,
        source_ip: IpAddr,
        now_monotonic_ms: u64,
    ) -> Result<(), PairingRuntimeError> {
        match self
            .attempt_limiter
            .begin_attempt(source_ip, now_monotonic_ms)?
        {
            PairingAttemptDecision::Allowed => Ok(()),
            PairingAttemptDecision::CoolingDown { retry_after_ms } => {
                Err(PairingRuntimeError::CoolingDown { retry_after_ms })
            }
            PairingAttemptDecision::CapacityLimited => Err(PairingRuntimeError::CapacityLimited),
        }
    }

    fn verify_admitted_proof(
        &mut self,
        source_ip: IpAddr,
        proof: PairingProof,
        now_monotonic_ms: u64,
    ) -> Result<AdmittedPendingPairingApproval, PairingRuntimeError> {
        let approval = self.coordinator.verify_proof(proof, now_monotonic_ms)?;
        Ok(AdmittedPendingPairingApproval {
            source_ip,
            approval,
        })
    }

    /// Parses a bounded v1 `pair.begin` frame, then admits and verifies its signed proof.
    /// Reserves the source budget before parsing so malformed frames are throttled as well.
    pub fn verify_pair_begin_from(
        &mut self,
        source_ip: IpAddr,
        frame: &[u8],
        now_monotonic_ms: u64,
    ) -> Result<AdmittedPendingPairingApproval, PairingRuntimeError> {
        self.reserve_source_attempt(source_ip, now_monotonic_ms)?;
        let message = parse_message(frame).map_err(PairingRuntimeError::Protocol)?;
        let MessagePayload::PairBegin(payload) = message.payload else {
            return Err(PairingRuntimeError::UnexpectedMessageType);
        };

        let receiver_id = DeviceId::parse(&payload.receiver_id)
            .map_err(|_| PairingRuntimeError::InvalidPairBegin)?;
        let nonce = PairingNonce::from_hex(&payload.nonce)
            .map_err(|_| PairingRuntimeError::InvalidPairBegin)?;
        let peer_id =
            DeviceId::parse(&payload.peer_id).map_err(|_| PairingRuntimeError::InvalidPairBegin)?;
        let public_key = PeerPublicKey::new(decode_lower_hex(&payload.public_key, 32)?)
            .map_err(|_| PairingRuntimeError::InvalidPairBegin)?;
        let requested_grants = TrustedPeerGrants::new(
            payload.requested_grants.iter().any(|grant| grant == "view"),
            payload.requested_grants.iter().any(|grant| grant == "send"),
            payload
                .requested_grants
                .iter()
                .any(|grant| grant == "annotate"),
        )
        .map_err(|_| PairingRuntimeError::InvalidPairBegin)?;
        let signature = decode_lower_hex(&payload.signature, 64)?;
        let proof = PairingProof::new(
            receiver_id,
            nonce,
            peer_id,
            payload.display_name,
            public_key,
            requested_grants,
            signature,
        )?;

        self.verify_admitted_proof(source_ip, proof, now_monotonic_ms)
    }

    /// Persists only the explicitly approved subset of the sender's requested grants.
    pub fn approve(
        &self,
        pending: AdmittedPendingPairingApproval,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<TrustedPeer, PairingRuntimeError> {
        let source_ip = pending.source_ip;
        let peer = self.coordinator.approve(
            pending.approval,
            approved_grants,
            now_monotonic_ms,
            paired_at_unix_ms,
        )?;
        self.attempt_limiter
            .record_success(source_ip)
            .map_err(PairingRuntimeError::AdmissionCleanup)?;
        Ok(peer)
    }
}

fn decode_lower_hex(value: &str, expected_bytes: usize) -> Result<Vec<u8>, PairingRuntimeError> {
    if value.len() != expected_bytes * 2 {
        return Err(PairingRuntimeError::InvalidPairBegin);
    }
    let mut decoded = Vec::with_capacity(expected_bytes);
    for index in 0..expected_bytes {
        let offset = index * 2;
        let high =
            decode_nibble(value.as_bytes()[offset]).ok_or(PairingRuntimeError::InvalidPairBegin)?;
        let low = decode_nibble(value.as_bytes()[offset + 1])
            .ok_or(PairingRuntimeError::InvalidPairBegin)?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn decode_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use mm_application::{PairingCoordinatorError, PairingProof};
    use mm_domain::{
        DeviceId, PeerFingerprint, PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
    };
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
    use mm_security::{
        PairingAttemptLimiter, PairingAttemptPolicy, PairingNonce, parse_pairing_qr,
    };

    use super::{PairingRuntime, PairingRuntimeError};

    const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
    const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);

    struct FakeIdentity {
        local_key: PeerPublicKey,
        fail_local_key: bool,
        fail_fingerprint: bool,
        reject_signature: bool,
        verify_calls: AtomicUsize,
    }

    impl FakeIdentity {
        fn new(local_key: PeerPublicKey) -> Self {
            Self {
                local_key,
                fail_local_key: false,
                fail_fingerprint: false,
                reject_signature: false,
                verify_calls: AtomicUsize::new(0),
            }
        }

        fn failing_local_key() -> Self {
            Self {
                local_key: PeerPublicKey::new(vec![9; 32]).unwrap(),
                fail_local_key: true,
                fail_fingerprint: false,
                reject_signature: false,
                verify_calls: AtomicUsize::new(0),
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
            self.verify_calls.fetch_add(1, Ordering::Relaxed);
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

    fn admission_limiter(max_attempts: u8) -> PairingAttemptLimiter {
        PairingAttemptLimiter::new(
            PairingAttemptPolicy::new(max_attempts, 60_000, 30_000, 4).unwrap(),
        )
    }

    fn owned_pairing_runtime() -> (
        PairingRuntime<FakeTrustedPeers, FakeIdentity>,
        Arc<FakeTrustedPeers>,
    ) {
        let identity = Arc::new(FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap()));
        let peers = Arc::new(FakeTrustedPeers::default());
        let runtime = PairingRuntime::new(RECEIVER_ID, identity, Arc::clone(&peers)).unwrap();
        (runtime, peers)
    }

    fn source_ip() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))
    }

    #[test]
    fn cooldown_rejects_before_signature_verification() {
        let identity = Arc::new(FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap()));
        let store = Arc::new(FakeTrustedPeers::default());
        let mut runtime = PairingRuntime::new_with_limiter(
            RECEIVER_ID,
            Arc::clone(&identity),
            Arc::clone(&store),
            admission_limiter(1),
        )
        .unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());
        let invalid_proof = || {
            PairingProof::new(
                RECEIVER_ID,
                nonce,
                SENDER_ID,
                "Sender laptop".to_owned(),
                PeerPublicKey::new(vec![7; 32]).unwrap(),
                requested_grants(),
                vec![1],
            )
            .unwrap()
        };

        assert_eq!(
            runtime.verify_proof_from(source_ip(), invalid_proof(), 101),
            Err(PairingRuntimeError::Coordinator(
                PairingCoordinatorError::IdentityRejected
            ))
        );
        assert_eq!(
            runtime.verify_proof_from(source_ip(), invalid_proof(), 102),
            Err(PairingRuntimeError::CoolingDown {
                retry_after_ms: 29_999
            })
        );
        assert_eq!(identity.verify_calls.load(Ordering::Relaxed), 1);
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn approved_pairing_clears_the_admitted_source_budget() {
        let identity = Arc::new(FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap()));
        let store = Arc::new(FakeTrustedPeers::default());
        let mut runtime = PairingRuntime::new_with_limiter(
            RECEIVER_ID,
            Arc::clone(&identity),
            Arc::clone(&store),
            admission_limiter(1),
        )
        .unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let pending = runtime
            .verify_proof_from(
                source_ip(),
                pairing_proof(invitation_nonce(invitation.uri())),
                101,
            )
            .unwrap();

        runtime
            .approve(pending, requested_grants(), 102, 500)
            .unwrap();

        let next_invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 103, 200)
            .unwrap();
        assert!(
            runtime
                .verify_proof_from(
                    source_ip(),
                    pairing_proof(invitation_nonce(next_invitation.uri())),
                    104,
                )
                .is_ok()
        );
        assert_eq!(identity.verify_calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn derives_local_fingerprint_and_persists_only_after_explicit_approval() {
        let identity = Arc::new(FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap()));
        let store = Arc::new(FakeTrustedPeers::default());
        let mut runtime =
            PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&store)).unwrap();

        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let payload = parse_pairing_qr(invitation.uri(), 200, &[8765]).unwrap();
        assert_eq!(payload.fingerprint(), PeerFingerprint::from_bytes([9; 32]));

        let pending = runtime
            .verify_proof_from(source_ip(), pairing_proof(payload.nonce()), 101)
            .unwrap();
        assert!(store.list_peers().unwrap().is_empty());

        let approved_grants = TrustedPeerGrants::new(true, false, false).unwrap();
        let peer = runtime.approve(pending, approved_grants, 102, 500).unwrap();

        assert_eq!(store.list_peers().unwrap(), vec![peer.clone()]);
        assert!(peer.grants().allows(PeerGrant::View));
        assert!(!peer.grants().allows(PeerGrant::Send));
    }

    #[test]
    fn owned_runtime_keeps_invitation_nonce_and_shared_store_across_calls() {
        let (mut runtime, peers) = owned_pairing_runtime();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());
        let pending = runtime
            .verify_proof_from(source_ip(), pairing_proof(nonce), 101)
            .unwrap();

        assert!(peers.list_peers().unwrap().is_empty());
        runtime
            .approve(pending, requested_grants(), 102, 500)
            .unwrap();

        assert_eq!(peers.list_peers().unwrap().len(), 1);
    }

    #[test]
    fn refuses_to_start_when_local_identity_is_unavailable() {
        let identity = Arc::new(FakeIdentity::failing_local_key());
        let store = Arc::new(FakeTrustedPeers::default());

        assert!(matches!(
            PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&store)),
            Err(PortError::Unavailable)
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn refuses_to_start_when_local_fingerprint_is_unavailable() {
        let mut identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        identity.fail_fingerprint = true;
        let identity = Arc::new(identity);
        let store = Arc::new(FakeTrustedPeers::default());

        assert!(matches!(
            PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&store)),
            Err(PortError::StorageFailure)
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn rejects_sender_signature_without_persisting_trust() {
        let mut identity = FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap());
        identity.reject_signature = true;
        let identity = Arc::new(identity);
        let store = Arc::new(FakeTrustedPeers::default());
        let mut runtime =
            PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&store)).unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());

        assert!(matches!(
            runtime.verify_proof_from(source_ip(), pairing_proof(nonce), 101),
            Err(PairingRuntimeError::Coordinator(
                PairingCoordinatorError::IdentityRejected
            ))
        ));
        assert!(store.list_peers().unwrap().is_empty());
    }

    #[test]
    fn reports_storage_failure_only_after_explicit_approval() {
        let identity = Arc::new(FakeIdentity::new(PeerPublicKey::new(vec![9; 32]).unwrap()));
        let store = Arc::new(FakeTrustedPeers::failing_save());
        let mut runtime =
            PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&store)).unwrap();
        let invitation = runtime
            .create_invitation("127.0.0.1:8765", &[8765], 100, 200)
            .unwrap();
        let nonce = invitation_nonce(invitation.uri());
        let pending = runtime
            .verify_proof_from(source_ip(), pairing_proof(nonce), 101)
            .unwrap();

        assert!(store.list_peers().unwrap().is_empty());
        assert_eq!(
            runtime.approve(pending, requested_grants(), 102, 500),
            Err(PairingRuntimeError::Coordinator(
                PairingCoordinatorError::StoreFailure
            ))
        );
        assert!(store.list_peers().unwrap().is_empty());
    }
}
