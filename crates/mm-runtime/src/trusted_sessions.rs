use std::collections::{HashMap, HashSet};
use std::fmt;

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
pub struct TrustedSessionRuntime<'a, P, T>
where
    P: TrustedPeerPort,
    T: TransportPort,
{
    peers: &'a P,
    transport: T,
    sessions: HashMap<SessionId, DeviceId>,
    revoked_peers: HashSet<DeviceId>,
}

impl<'a, P, T> TrustedSessionRuntime<'a, P, T>
where
    P: TrustedPeerPort,
    T: TransportPort,
{
    pub fn new(peers: &'a P, transport: T) -> Self {
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

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use mm_domain::{
        DeviceId, PeerFingerprint, PeerPublicKey, SessionId, TrustedPeer, TrustedPeerGrants,
    };
    use mm_ports::{PortError, TransportPort, TrustedPeerPort};
    use mm_protocol::Envelope;

    use super::{TrustedSessionError, TrustedSessionRuntime};

    #[derive(Default)]
    struct FakePeers {
        trusted: Mutex<HashMap<DeviceId, TrustedPeer>>,
        revoked: Mutex<Vec<DeviceId>>,
        fail_revoke: AtomicBool,
    }

    impl FakePeers {
        fn trust(&self, peer_id: DeviceId) {
            self.trust_with(peer_id, grants(true, true, true));
        }

        fn trust_with(&self, peer_id: DeviceId, grants: TrustedPeerGrants) {
            let value = peer_id.as_bytes()[0];
            let trusted_peer = TrustedPeer::new(
                peer_id,
                format!("Peer {value}"),
                PeerPublicKey::new(vec![value; 32]).unwrap(),
                PeerFingerprint::from_bytes([value; 32]),
                grants,
                1_000,
            )
            .unwrap();
            self.trusted.lock().unwrap().insert(peer_id, trusted_peer);
        }
    }

    impl TrustedPeerPort for FakePeers {
        fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError> {
            Ok(self.trusted.lock().unwrap().get(&peer_id).cloned())
        }

        fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError> {
            self.trusted.lock().unwrap().insert(peer.id(), peer.clone());
            Ok(())
        }

        fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError> {
            Ok(self.trusted.lock().unwrap().values().cloned().collect())
        }

        fn revoke(&self, peer_id: DeviceId, _revoked_at_unix_ms: u64) -> Result<(), PortError> {
            if self.fail_revoke.load(Ordering::SeqCst) {
                return Err(PortError::StorageFailure);
            }
            if let Some(peer) = self.trusted.lock().unwrap().get_mut(&peer_id) {
                peer.revoke(_revoked_at_unix_ms)
                    .map_err(|_| PortError::StorageFailure)?;
            } else {
                return Err(PortError::StorageFailure);
            }
            self.revoked.lock().unwrap().push(peer_id);
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    struct FakeTransport(Arc<Mutex<FakeTransportState>>);

    #[derive(Default)]
    struct FakeTransportState {
        started: Vec<(SessionId, DeviceId)>,
        closed: Vec<SessionId>,
        fail_start: HashSet<SessionId>,
        fail_close: HashSet<SessionId>,
    }

    impl TransportPort for FakeTransport {
        fn start(&mut self, session_id: SessionId, peer_id: DeviceId) -> Result<(), PortError> {
            let mut state = self.0.lock().unwrap();
            state.started.push((session_id, peer_id));
            if state.fail_start.contains(&session_id) {
                Err(PortError::TransportFailure)
            } else {
                Ok(())
            }
        }

        fn send_control(&mut self, _message: &Envelope<Vec<u8>>) -> Result<(), PortError> {
            Ok(())
        }

        fn close(&mut self, session_id: SessionId) -> Result<(), PortError> {
            let mut state = self.0.lock().unwrap();
            if state.fail_close.contains(&session_id) {
                return Err(PortError::TransportFailure);
            }
            state.closed.push(session_id);
            Ok(())
        }
    }

    fn peer(value: u8) -> DeviceId {
        DeviceId::from_bytes([value; 16])
    }

    fn session(value: u8) -> SessionId {
        SessionId::from_bytes([value; 16])
    }

    fn grants(view: bool, send: bool, annotate: bool) -> TrustedPeerGrants {
        TrustedPeerGrants::new(view, send, annotate).unwrap()
    }

    fn view_scope() -> TrustedPeerGrants {
        grants(true, false, false)
    }

    fn send_scope() -> TrustedPeerGrants {
        grants(false, true, false)
    }

    #[test]
    fn refuses_to_start_a_session_for_an_untrusted_peer() {
        let peers = FakePeers::default();
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);

        assert_eq!(
            runtime.start_session(peer(1), session(1), view_scope()),
            Err(TrustedSessionError::PeerNotTrusted)
        );
        assert!(transport_state.lock().unwrap().started.is_empty());
    }

    #[test]
    fn rejects_a_session_scope_the_peer_was_not_granted() {
        let peers = FakePeers::default();
        peers.trust_with(peer(1), view_scope());
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);

        assert_eq!(
            runtime.start_session(peer(1), session(1), send_scope()),
            Err(TrustedSessionError::GrantNotAuthorized)
        );
        assert!(transport_state.lock().unwrap().started.is_empty());
        runtime
            .start_session(peer(1), session(1), view_scope())
            .unwrap();
        assert_eq!(
            transport_state.lock().unwrap().started,
            vec![(session(1), peer(1))]
        );
    }

    #[test]
    fn failed_start_is_closed_and_cleanup_failure_remains_tracked() {
        let peers = FakePeers::default();
        peers.trust(peer(1));
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        {
            let mut state = transport_state.lock().unwrap();
            state.fail_start.insert(session(1));
            state.fail_start.insert(session(2));
            state.fail_close.insert(session(2));
        }
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);

        assert_eq!(
            runtime.start_session(peer(1), session(1), view_scope()),
            Err(TrustedSessionError::TransportStart {
                session_id: session(1),
                source: PortError::TransportFailure,
            })
        );
        assert_eq!(
            runtime.start_session(peer(1), session(2), view_scope()),
            Err(TrustedSessionError::StartCleanupFailed {
                session_id: session(2),
                start_error: PortError::TransportFailure,
                close_error: PortError::TransportFailure,
            })
        );
        assert_eq!(
            runtime.start_session(peer(1), session(2), view_scope()),
            Err(TrustedSessionError::SessionAlreadyTracked)
        );

        transport_state
            .lock()
            .unwrap()
            .fail_close
            .remove(&session(2));
        runtime.stop_session(session(2)).unwrap();
        assert_eq!(
            transport_state.lock().unwrap().closed,
            vec![session(1), session(2)]
        );
    }

    #[test]
    fn revoking_a_peer_closes_its_sessions_and_blocks_reconnection() {
        let peers = FakePeers::default();
        peers.trust(peer(1));
        peers.trust(peer(2));
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);

        runtime
            .start_session(peer(1), session(1), view_scope())
            .unwrap();
        runtime
            .start_session(peer(1), session(2), view_scope())
            .unwrap();
        runtime
            .start_session(peer(2), session(3), view_scope())
            .unwrap();

        runtime.revoke_peer(peer(1), 1_700_000_000_000).unwrap();

        assert_eq!(
            transport_state.lock().unwrap().closed,
            vec![session(1), session(2)]
        );
        assert_eq!(*peers.revoked.lock().unwrap(), vec![peer(1)]);
        assert_eq!(
            runtime.start_session(peer(1), session(4), view_scope()),
            Err(TrustedSessionError::PeerRevoked)
        );
        runtime.stop_session(session(3)).unwrap();
        assert_eq!(
            transport_state.lock().unwrap().closed,
            vec![session(1), session(2), session(3)]
        );
    }

    #[test]
    fn failed_revocation_close_remains_tracked_and_can_be_retried() {
        let peers = FakePeers::default();
        peers.trust(peer(1));
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        transport_state
            .lock()
            .unwrap()
            .fail_close
            .insert(session(1));
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);
        runtime
            .start_session(peer(1), session(1), view_scope())
            .unwrap();

        assert_eq!(
            runtime.revoke_peer(peer(1), 1_700_000_000_000),
            Err(TrustedSessionError::RevocationIncomplete {
                storage_error: None,
                close_failures: vec![(session(1), PortError::TransportFailure)],
            })
        );
        assert_eq!(
            runtime.start_session(peer(1), session(2), view_scope()),
            Err(TrustedSessionError::PeerRevoked)
        );

        transport_state
            .lock()
            .unwrap()
            .fail_close
            .remove(&session(1));
        runtime.stop_session(session(1)).unwrap();
        assert_eq!(transport_state.lock().unwrap().closed, vec![session(1)]);
    }

    #[test]
    fn store_failure_still_closes_sessions_and_blocks_peer_locally() {
        let peers = FakePeers::default();
        peers.trust(peer(1));
        peers.fail_revoke.store(true, Ordering::SeqCst);
        let transport = FakeTransport::default();
        let transport_state = Arc::clone(&transport.0);
        let mut runtime = TrustedSessionRuntime::new(&peers, transport);
        runtime
            .start_session(peer(1), session(1), view_scope())
            .unwrap();

        assert_eq!(
            runtime.revoke_peer(peer(1), 1_700_000_000_000),
            Err(TrustedSessionError::RevocationIncomplete {
                storage_error: Some(PortError::StorageFailure),
                close_failures: Vec::new(),
            })
        );
        assert_eq!(transport_state.lock().unwrap().closed, vec![session(1)]);
        assert_eq!(
            runtime.start_session(peer(1), session(2), view_scope()),
            Err(TrustedSessionError::PeerRevoked)
        );
    }
}
