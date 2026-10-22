use std::fmt;
use std::sync::Arc;

use mm_domain::{
    DeviceId, MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, PeerFingerprint, PeerGrant, PeerPublicKey,
    TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort};
use mm_security::{
    ManualEndpointError, NonceIssueError, PairingNonce, PendingPairingNonces, format_pairing_qr,
    parse_manual_ip_endpoint, parse_pairing_qr,
};

pub const MAX_PAIRING_SIGNATURE_BYTES: usize = 512;
use mm_domain::{PAIRING_PROOF_DOMAIN_V1, PAIRING_PROOF_DOMAIN_V2, ReportedDeviceDetails};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingCoordinatorError {
    InvalidEndpoint(ManualEndpointError),
    NonceIssue(NonceIssueError),
    ExpiryOverflow,
    InvalidInvitation,
    InvalidProof,
    WrongReceiver,
    SelfPairing,
    IdentityRejected,
    NonceRejected,
    ApprovalExpired,
    GrantNotRequested,
    StoreFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingProofSigningError {
    SelfPairing,
    Identity(PortError),
    InvalidProof(PairingCoordinatorError),
}

impl fmt::Display for PairingProofSigningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfPairing => formatter.write_str("a device cannot pair with itself"),
            Self::Identity(error) => write!(formatter, "pairing signer failed: {error}"),
            Self::InvalidProof(error) => write!(formatter, "pairing proof is invalid: {error:?}"),
        }
    }
}

impl std::error::Error for PairingProofSigningError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingInvitation {
    uri: String,
    expires_at_unix_seconds: u64,
    expires_at_monotonic_ms: u64,
}

impl PairingInvitation {
    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub const fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }

    pub const fn expires_at_monotonic_ms(&self) -> u64 {
        self.expires_at_monotonic_ms
    }
}

/// A sender's possession proof, bound to one receiver, nonce, name, and request.
#[derive(Debug)]
pub struct PairingProof {
    receiver_id: DeviceId,
    nonce: PairingNonce,
    peer_id: DeviceId,
    display_name: String,
    device_details: Option<ReportedDeviceDetails>,
    public_key: PeerPublicKey,
    requested_grants: TrustedPeerGrants,
    signature: Vec<u8>,
}

impl PairingProof {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receiver_id: DeviceId,
        nonce: PairingNonce,
        peer_id: DeviceId,
        display_name: String,
        public_key: PeerPublicKey,
        requested_grants: TrustedPeerGrants,
        signature: Vec<u8>,
    ) -> Result<Self, PairingCoordinatorError> {
        let character_count = display_name.chars().count();
        if character_count == 0
            || character_count > MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS
            || display_name.chars().any(char::is_control)
            || signature.is_empty()
            || signature.len() > MAX_PAIRING_SIGNATURE_BYTES
        {
            return Err(PairingCoordinatorError::InvalidProof);
        }
        Ok(Self {
            receiver_id,
            nonce,
            peer_id,
            display_name,
            device_details: None,
            public_key,
            requested_grants,
            signature,
        })
    }

    /// Creates a possession proof using a device's protected local signing identity.
    ///
    /// The caller must obtain the receiver ID and nonce from a validated invitation, verify the
    /// receiver fingerprint through its connection policy, and supply the sender's local device
    /// ID. This method only validates and signs the canonical proof; it does not validate QR
    /// expiry or authenticate a transport connection.
    pub fn create_signed(
        receiver_id: DeviceId,
        nonce: PairingNonce,
        peer_id: DeviceId,
        display_name: String,
        requested_grants: TrustedPeerGrants,
        identity: &(impl LocalPairingIdentityPort + ?Sized),
    ) -> Result<Self, PairingProofSigningError> {
        if receiver_id == peer_id {
            return Err(PairingProofSigningError::SelfPairing);
        }

        let public_key = identity
            .local_public_key()
            .map_err(PairingProofSigningError::Identity)?;
        let details = identity.device_details();
        let unsigned = Self::new(
            receiver_id,
            nonce,
            peer_id,
            display_name.clone(),
            public_key.clone(),
            requested_grants,
            vec![0],
        )
        .map_err(PairingProofSigningError::InvalidProof)?
        .with_device_details(details.clone());
        let signature = identity
            .sign_pairing_message(&unsigned.signing_message())
            .map_err(PairingProofSigningError::Identity)?;

        Self::new(
            receiver_id,
            nonce,
            peer_id,
            display_name,
            public_key,
            requested_grants,
            signature,
        )
        .map(|proof| proof.with_device_details(details))
        .map_err(PairingProofSigningError::InvalidProof)
    }

    /// Canonical bytes the sender signs and the receiver verifies.
    pub fn signing_message(&self) -> Vec<u8> {
        let domain = if self.device_details.is_some() {
            PAIRING_PROOF_DOMAIN_V2
        } else {
            PAIRING_PROOF_DOMAIN_V1
        };
        let name_bytes = self.display_name.as_bytes();
        let name_length = (name_bytes.len() as u32).to_be_bytes();
        let grants = self.requested_grants;
        let grant_bits = u8::from(grants.allows(PeerGrant::View))
            | (u8::from(grants.allows(PeerGrant::Send)) << 1)
            | (u8::from(grants.allows(PeerGrant::Annotate)) << 2);

        let mut message =
            Vec::with_capacity(domain.len() + 16 + 16 + 16 + 4 + name_bytes.len() + 1);
        message.extend_from_slice(domain);
        message.extend_from_slice(self.receiver_id.as_bytes());
        message.extend_from_slice(self.nonce.as_bytes());
        message.extend_from_slice(self.peer_id.as_bytes());
        message.extend_from_slice(&name_length);
        message.extend_from_slice(name_bytes);
        message.push(grant_bits);
        if let Some(details) = &self.device_details {
            details.append_canonical(&mut message);
        }
        message
    }

    pub const fn receiver_id(&self) -> DeviceId {
        self.receiver_id
    }

    pub const fn nonce(&self) -> PairingNonce {
        self.nonce
    }

    pub const fn peer_id(&self) -> DeviceId {
        self.peer_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Attach validated facts before signing or verification. This never alters a signature.
    pub fn with_device_details(mut self, details: Option<ReportedDeviceDetails>) -> Self {
        self.device_details = details;
        self
    }

    pub fn device_details(&self) -> Option<&ReportedDeviceDetails> {
        self.device_details.as_ref()
    }

    pub fn public_key(&self) -> &PeerPublicKey {
        &self.public_key
    }

    pub const fn requested_grants(&self) -> TrustedPeerGrants {
        self.requested_grants
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PendingPairingApproval {
    receiver_id: DeviceId,
    peer_id: DeviceId,
    display_name: String,
    device_details: Option<ReportedDeviceDetails>,
    public_key: PeerPublicKey,
    fingerprint: PeerFingerprint,
    requested_grants: TrustedPeerGrants,
    expires_at_monotonic_ms: u64,
}

impl PendingPairingApproval {
    pub fn device_details(&self) -> Option<&ReportedDeviceDetails> {
        self.device_details.as_ref()
    }
    pub const fn peer_id(&self) -> DeviceId {
        self.peer_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn fingerprint(&self) -> PeerFingerprint {
        self.fingerprint
    }

    pub const fn requested_grants(&self) -> TrustedPeerGrants {
        self.requested_grants
    }

    pub const fn expires_at_monotonic_ms(&self) -> u64 {
        self.expires_at_monotonic_ms
    }
}

pub struct PairingCoordinator<P: TrustedPeerPort, I: PairingIdentityPort> {
    local_device_id: DeviceId,
    local_fingerprint: PeerFingerprint,
    nonces: PendingPairingNonces,
    identity: Arc<I>,
    peers: Arc<P>,
}

impl<P: TrustedPeerPort, I: PairingIdentityPort> PairingCoordinator<P, I> {
    /// Creates a coordinator that retains shared ownership of its identity and peer store.
    pub fn new(
        local_device_id: DeviceId,
        local_fingerprint: PeerFingerprint,
        nonces: PendingPairingNonces,
        identity: Arc<I>,
        peers: Arc<P>,
    ) -> Self {
        Self {
            local_device_id,
            local_fingerprint,
            nonces,
            identity,
            peers,
        }
    }

    /// Returns the public identity fingerprint captured when this coordinator was created.
    pub const fn local_fingerprint(&self) -> PeerFingerprint {
        self.local_fingerprint
    }

    /// Create a user-displayable QR URI for a previously approved numeric endpoint.
    pub fn create_invitation(
        &mut self,
        endpoint_text: &str,
        approved_ports: &[u16],
        now_monotonic_ms: u64,
        now_unix_seconds: u64,
    ) -> Result<PairingInvitation, PairingCoordinatorError> {
        let endpoint = parse_manual_ip_endpoint(endpoint_text, approved_ports)
            .map_err(PairingCoordinatorError::InvalidEndpoint)?;
        let expires_at_unix_seconds = now_unix_seconds
            .checked_add(mm_security::PAIRING_QR_TTL_SECONDS)
            .ok_or(PairingCoordinatorError::ExpiryOverflow)?;
        let ticket = self
            .nonces
            .issue(now_monotonic_ms)
            .map_err(PairingCoordinatorError::NonceIssue)?;
        let uri = format_pairing_qr(
            endpoint,
            self.local_device_id,
            self.local_fingerprint,
            ticket.nonce,
            expires_at_unix_seconds,
        );
        parse_pairing_qr(&uri, now_unix_seconds, approved_ports)
            .map_err(|_| PairingCoordinatorError::InvalidInvitation)?;

        Ok(PairingInvitation {
            uri,
            expires_at_unix_seconds,
            expires_at_monotonic_ms: ticket.expires_at_ms,
        })
    }

    /// Verify possession and return a short-lived approval request without saving trust.
    pub fn verify_proof(
        &mut self,
        proof: PairingProof,
        now_monotonic_ms: u64,
    ) -> Result<PendingPairingApproval, PairingCoordinatorError> {
        if proof.receiver_id != self.local_device_id {
            return Err(PairingCoordinatorError::WrongReceiver);
        }
        if proof.peer_id == self.local_device_id {
            return Err(PairingCoordinatorError::SelfPairing);
        }
        if !self.nonces.is_pending(proof.nonce, now_monotonic_ms) {
            return Err(PairingCoordinatorError::NonceRejected);
        }

        let signing_message = proof.signing_message();
        self.identity
            .verify_signature(&proof.public_key, &signing_message, &proof.signature)
            .map_err(|_| PairingCoordinatorError::IdentityRejected)?;
        let fingerprint = self
            .identity
            .fingerprint_public_key(&proof.public_key)
            .map_err(|_| PairingCoordinatorError::IdentityRejected)?;
        let ticket = self
            .nonces
            .consume_ticket(proof.nonce, now_monotonic_ms)
            .ok_or(PairingCoordinatorError::NonceRejected)?;

        Ok(PendingPairingApproval {
            receiver_id: self.local_device_id,
            peer_id: proof.peer_id,
            display_name: proof.display_name,
            device_details: proof.device_details,
            public_key: proof.public_key,
            fingerprint,
            requested_grants: proof.requested_grants,
            expires_at_monotonic_ms: ticket.expires_at_ms,
        })
    }

    /// Persist trust only after the UI has explicitly approved a subset of requested grants.
    pub fn approve(
        &self,
        pending: PendingPairingApproval,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<TrustedPeer, PairingCoordinatorError> {
        if pending.receiver_id != self.local_device_id {
            return Err(PairingCoordinatorError::WrongReceiver);
        }
        if now_monotonic_ms >= pending.expires_at_monotonic_ms {
            return Err(PairingCoordinatorError::ApprovalExpired);
        }
        if !approved_grants.is_subset_of(pending.requested_grants) {
            return Err(PairingCoordinatorError::GrantNotRequested);
        }
        let peer = TrustedPeer::new(
            pending.peer_id,
            pending.display_name,
            pending.public_key,
            pending.fingerprint,
            approved_grants,
            paired_at_unix_ms,
        )
        .map_err(|_| PairingCoordinatorError::InvalidProof)?;
        self.peers
            .save_peer(&peer)
            .map_err(|_| PairingCoordinatorError::StoreFailure)?;
        Ok(peer)
    }
}
