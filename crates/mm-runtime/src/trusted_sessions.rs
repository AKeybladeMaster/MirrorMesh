use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use mm_domain::{DeviceId, SessionId, TrustedPeerGrants};
use mm_ports::{PortError, TransportPort, TrustedPeerPort};

/// Errors from the trusted-peer session lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustedSessionError {
    PeerNotTrusted,
    PeerRevoked,
    GrantNotAuthorized,
    SessionAlreadyTracked,
    UnknownSession,
    Storage(PortError),
    TransportStart {
        session_id: SessionId,
        source: PortError,
    },
    StartCleanupFailed {
        session_id: SessionId,
        start_error: PortError,
        close_error: PortError,
    },
    TransportClose {
        session_id: SessionId,
        source: PortError,
    },
    RevocationIncomplete {
        storage_error: Option<PortError>,
        close_failures: Vec<(SessionId, PortError)>,
    },
}

impl fmt::Display for TrustedSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PeerNotTrusted => formatter.write_str("peer is not trusted"),
            Self::PeerRevoked => formatter.write_str("peer has been revoked in this runtime"),
            Self::GrantNotAuthorized => {
                formatter.write_str("peer was not granted the requested session scope")
            }
            Self::SessionAlreadyTracked => formatter.write_str("session is already tracked"),
            Self::UnknownSession => formatter.write_str("session is not tracked"),
            Self::Storage(error) => write!(formatter, "trusted-peer storage failed: {error}"),
            Self::TransportStart { session_id, source } => {
                write!(formatter, "could not start session {session_id}: {source}")
            }
            Self::StartCleanupFailed {
                session_id,
                start_error,
                close_error,
            } => write!(
                formatter,
                "session {session_id} failed to start ({start_error}) and cleanup failed ({close_error})"
            ),
            Self::TransportClose { session_id, source } => {
                write!(formatter, "could not close session {session_id}: {source}")
            }
            Self::RevocationIncomplete {
                storage_error,
                close_failures,
            } => {
                if let Some(error) = storage_error {
                    write!(
                        formatter,
                        "peer was blocked locally but revocation persistence failed: {error}"
                    )?;
                } else {
                    formatter.write_str("peer revocation was persisted")?;
                }
                if !close_failures.is_empty() {
                    write!(
                        formatter,
                        "; {} session transport(s) could not be closed",
                        close_failures.len()
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for TrustedSessionError {}

/// Serial owner for trusted peer sessions and their control transports.
///
/// Mutating methods require one caller at a time. Revocation is persisted first, then the
/// runtime blocks new sessions for that peer and closes every tracked transport even if
/// persistence fails. Failed closes remain tracked so the caller can retry with
/// [`Self::stop_session`]. A storage error means the revocation may not survive process restart.
pub struct TrustedSessionRuntime<P, T>
where
    P: TrustedPeerPort,
    T: TransportPort,
{
    peers: Arc<P>,
    transport: T,
    sessions: HashMap<SessionId, DeviceId>,
    revoked_peers: HashSet<DeviceId>,
}

impl<P, T> TrustedSessionRuntime<P, T>
where
    P: TrustedPeerPort,
    T: TransportPort,
{
    /// Creates a session runtime that retains shared ownership of its trusted-peer store.
    pub fn new(peers: Arc<P>, transport: T) -> Self {
        Self {
            peers,
            transport,
            sessions: HashMap::new(),
            revoked_peers: HashSet::new(),
        }
    }

    /// Starts a control transport only when stored trust and the requested grants are active.
    pub fn start_session(
        &mut self,
        peer_id: DeviceId,
        session_id: SessionId,
        required_grants: TrustedPeerGrants,
    ) -> Result<(), TrustedSessionError> {
        if self.revoked_peers.contains(&peer_id) {
            return Err(TrustedSessionError::PeerRevoked);
        }
        let peer = self
            .peers
            .get_peer(peer_id)
            .map_err(TrustedSessionError::Storage)?
            .filter(|peer| peer.is_active())
            .ok_or(TrustedSessionError::PeerNotTrusted)?;
        if !required_grants.is_subset_of(peer.grants()) {
            return Err(TrustedSessionError::GrantNotAuthorized);
        }
        if self.sessions.contains_key(&session_id) {
            return Err(TrustedSessionError::SessionAlreadyTracked);
        }

        match self.transport.start(session_id, peer_id) {
            Ok(()) => {
                self.sessions.insert(session_id, peer_id);
                Ok(())
            }
            Err(start_error) => match self.transport.close(session_id) {
                Ok(()) => Err(TrustedSessionError::TransportStart {
                    session_id,
                    source: start_error,
                }),
                Err(close_error) => {
                    self.sessions.insert(session_id, peer_id);
                    Err(TrustedSessionError::StartCleanupFailed {
                        session_id,
                        start_error,
                        close_error,
                    })
                }
            },
        }
    }

    /// Closes one tracked transport; a failed close remains tracked for a later retry.
    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), TrustedSessionError> {
        if !self.sessions.contains_key(&session_id) {
            return Err(TrustedSessionError::UnknownSession);
        }
        self.transport
            .close(session_id)
            .map_err(|source| TrustedSessionError::TransportClose { session_id, source })?;
        self.sessions.remove(&session_id);
        Ok(())
    }

    /// Blocks new sessions, attempts to persist peer revocation, then closes all of its sessions.
    ///
    /// The revoked cache is set before storage access, so a persistence failure still closes
    /// tracked sessions and prevents new starts for this runtime lifetime. If storage fails,
    /// the peer may become active again after process restart unless the caller retries the
    /// persistence operation. Every tracked transport is closed even when persistence fails;
    /// failed closes are returned with their session IDs for retry.
    pub fn revoke_peer(
        &mut self,
        peer_id: DeviceId,
        revoked_at_unix_ms: u64,
    ) -> Result<(), TrustedSessionError> {
        self.revoked_peers.insert(peer_id);
        let storage_error = self.peers.revoke(peer_id, revoked_at_unix_ms).err();

        let mut session_ids: Vec<_> = self
            .sessions
            .iter()
            .filter_map(|(session_id, owner)| (*owner == peer_id).then_some(*session_id))
            .collect();
        session_ids.sort_unstable();
        let mut failures = Vec::new();
        for session_id in session_ids {
            match self.transport.close(session_id) {
                Ok(()) => {
                    self.sessions.remove(&session_id);
                }
                Err(error) => failures.push((session_id, error)),
            }
        }

        if storage_error.is_none() && failures.is_empty() {
            Ok(())
        } else {
            Err(TrustedSessionError::RevocationIncomplete {
                storage_error,
                close_failures: failures,
            })
        }
    }
}
