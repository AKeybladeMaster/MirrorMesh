use std::net::IpAddr;

use mm_application::{PairingCoordinatorError, PairingInvitation, PairingProof};
use mm_domain::{DeviceId, SessionId, TrustedPeer, TrustedPeerGrants};
use mm_ports::{
    LocalPairingIdentityPort, PairingIdentityPort, PortError, TransportPort, TrustedPeerPort,
};

use crate::{
    AdmittedPendingPairingApproval, PairingRuntime, PairingRuntimeError, TrustedSessionError,
    TrustedSessionRuntime,
};

/// Coordinates consent-gated pairing and trusted sessions over one identity and peer store.
///
/// Keep one instance for the local identity lifecycle. Recreate it after identity rotation or
/// deletion, and serialize its mutating methods on one application runtime owner.
pub struct MirrorMeshRuntime<'a, P, I, T>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
    T: TransportPort,
{
    pairing: PairingRuntime<'a, P, I>,
    sessions: TrustedSessionRuntime<'a, P, T>,
}

impl<'a, P, I, T> MirrorMeshRuntime<'a, P, I, T>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort,
    T: TransportPort,
{
    /// Builds both runtime components over the same identity and trusted-peer store.
    pub fn new(
        local_device_id: DeviceId,
        identity: &'a I,
        peers: &'a P,
        transport: T,
    ) -> Result<Self, PortError> {
        Ok(Self {
            pairing: PairingRuntime::new(local_device_id, identity, peers)?,
            sessions: TrustedSessionRuntime::new(peers, transport),
        })
    }

    /// Creates an invitation for an endpoint the user explicitly approved.
    pub fn create_invitation(
        &mut self,
        endpoint_text: &str,
        approved_ports: &[u16],
        now_monotonic_ms: u64,
        now_unix_seconds: u64,
    ) -> Result<PairingInvitation, PairingCoordinatorError> {
        self.pairing.create_invitation(
            endpoint_text,
            approved_ports,
            now_monotonic_ms,
            now_unix_seconds,
        )
    }

    /// Verifies a proof after source-aware attempt admission.
    pub fn verify_proof_from(
        &mut self,
        source_ip: IpAddr,
        proof: PairingProof,
        now_monotonic_ms: u64,
    ) -> Result<AdmittedPendingPairingApproval, PairingRuntimeError> {
        self.pairing
            .verify_proof_from(source_ip, proof, now_monotonic_ms)
    }

    /// Persists only the grant subset explicitly approved by the local user.
    pub fn approve(
        &self,
        pending: AdmittedPendingPairingApproval,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<TrustedPeer, PairingRuntimeError> {
        self.pairing.approve(
            pending,
            approved_grants,
            now_monotonic_ms,
            paired_at_unix_ms,
        )
    }

    /// Starts a session only if the same store used by pairing still marks the peer active.
    pub fn start_session(
        &mut self,
        peer_id: DeviceId,
        session_id: SessionId,
    ) -> Result<(), TrustedSessionError> {
        self.sessions.start_session(peer_id, session_id)
    }

    /// Closes a tracked session, retaining it for retry if the transport close fails.
    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), TrustedSessionError> {
        self.sessions.stop_session(session_id)
    }

    /// Revokes persisted trust, blocks future starts, and closes this peer's tracked sessions.
    pub fn revoke_peer(
        &mut self,
        peer_id: DeviceId,
        revoked_at_unix_ms: u64,
    ) -> Result<(), TrustedSessionError> {
        self.sessions.revoke_peer(peer_id, revoked_at_unix_ms)
    }
}
