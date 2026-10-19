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
