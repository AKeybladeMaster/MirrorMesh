mod mirrormesh;
mod pairing;
mod trusted_sessions;

pub use mirrormesh::MirrorMeshRuntime;
pub use pairing::{AdmittedPendingPairingApproval, PairingRuntime, PairingRuntimeError};
pub use trusted_sessions::{TrustedSessionError, TrustedSessionRuntime};

use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};

use mm_domain::{DomainError, SessionEvent, SessionId, SessionState, try_transition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    SessionAlreadyExists,
    UnknownSession,
    LockPoisoned,
    Domain(DomainError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionAlreadyExists => formatter.write_str("session already exists"),
            Self::UnknownSession => formatter.write_str("session does not exist"),
            Self::LockPoisoned => formatter.write_str("session manager lock was poisoned"),
            Self::Domain(error) => write!(formatter, "session transition failed: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<DomainError> for RuntimeError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

#[derive(Debug, Default)]
struct SessionManagerState {
    sessions: HashMap<SessionId, SessionState>,
    subscribers: HashMap<SessionId, Vec<SyncSender<SessionState>>>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    inner: Arc<Mutex<SessionManagerState>>,
}

impl SessionManager {
    pub fn create(&self, session_id: SessionId) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().map_err(|_| RuntimeError::LockPoisoned)?;
        if inner.sessions.contains_key(&session_id) {
            return Err(RuntimeError::SessionAlreadyExists);
        }
        inner.sessions.insert(session_id, SessionState::Idle);
        Ok(())
    }

    pub fn apply(
        &self,
        session_id: SessionId,
        event: SessionEvent,
    ) -> Result<SessionState, RuntimeError> {
        let mut inner = self.inner.lock().map_err(|_| RuntimeError::LockPoisoned)?;
        let current = *inner
            .sessions
            .get(&session_id)
            .ok_or(RuntimeError::UnknownSession)?;
        let next = try_transition(current, event)?;
        inner.sessions.insert(session_id, next);
        if next == current {
            return Ok(next);
        }
        if let Some(subscribers) = inner.subscribers.get_mut(&session_id) {
            subscribers.retain(|subscriber| match subscriber.try_send(next) {
                Ok(()) => true,
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
            });
        }
        Ok(next)
    }

    pub fn state(&self, session_id: SessionId) -> Result<SessionState, RuntimeError> {
        self.inner
            .lock()
            .map_err(|_| RuntimeError::LockPoisoned)?
            .sessions
            .get(&session_id)
            .copied()
            .ok_or(RuntimeError::UnknownSession)
    }

    /// Subscribes to bounded state-only updates and immediately queues the current snapshot.
    /// Slow subscribers are disconnected if their bounded queue overflows.
    pub fn subscribe(&self, session_id: SessionId) -> Result<Receiver<SessionState>, RuntimeError> {
        let mut inner = self.inner.lock().map_err(|_| RuntimeError::LockPoisoned)?;
        let current = *inner
            .sessions
            .get(&session_id)
            .ok_or(RuntimeError::UnknownSession)?;
        let (sender, receiver) = sync_channel(16);
        let _ = sender.try_send(current);
        inner
            .subscribers
            .entry(session_id)
            .or_default()
            .push(sender);
        Ok(receiver)
    }

    pub fn remove_stopped(&self, session_id: SessionId) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().map_err(|_| RuntimeError::LockPoisoned)?;
        match inner.sessions.get(&session_id) {
            Some(SessionState::Stopped) => {
                inner.sessions.remove(&session_id);
                inner.subscribers.remove(&session_id);
                Ok(())
            }
            Some(_) => Err(RuntimeError::Domain(DomainError::IllegalTransition)),
            None => Err(RuntimeError::UnknownSession),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeError, SessionManager};
    use mm_domain::{SessionEvent as E, SessionId, SessionState as S};

    fn session(value: u8) -> SessionId {
        SessionId::from_bytes([value; 16])
    }

    #[test]
    fn session_lifecycle_rejects_unauthorized_transition_without_mutation() {
        let manager = SessionManager::default();
        let id = session(1);
        manager.create(id).unwrap();
        manager.apply(id, E::PeerDiscovered).unwrap();
        manager
            .apply(
                id,
                E::TrustedPeerVerified {
                    fingerprint_verified: true,
                },
            )
            .unwrap();
        manager.apply(id, E::BeginNegotiation).unwrap();

        assert!(
            manager
                .apply(
                    id,
                    E::StartStreaming {
                        peer_authenticated: true,
                        capture_permitted: false,
                        first_frame_presented: true,
                    }
                )
                .is_err()
        );
        assert_eq!(manager.state(id), Ok(S::Negotiating));
    }

    #[test]
    fn disconnect_and_cleanup_are_idempotent_at_runtime_boundary() {
        let manager = SessionManager::default();
        let id = session(2);
        manager.create(id).unwrap();
        manager.apply(id, E::PeerDiscovered).unwrap();
        manager
            .apply(
                id,
                E::TrustedPeerVerified {
                    fingerprint_verified: true,
                },
            )
            .unwrap();
        manager.apply(id, E::BeginNegotiation).unwrap();
        manager
            .apply(
                id,
                E::StartStreaming {
                    peer_authenticated: true,
                    capture_permitted: true,
                    first_frame_presented: true,
                },
            )
            .unwrap();
        for _ in 0..50 {
            assert_eq!(manager.apply(id, E::Stop), Ok(S::Stopping));
        }
        assert_eq!(manager.apply(id, E::CleanupComplete), Ok(S::Stopped));
        manager.remove_stopped(id).unwrap();
        assert_eq!(manager.state(id), Err(RuntimeError::UnknownSession));
    }

    #[test]
    fn independent_sessions_can_transition_concurrently() {
        let manager = SessionManager::default();
        let mut workers = Vec::new();
        for value in 1..=8 {
            let worker_manager = manager.clone();
            workers.push(std::thread::spawn(move || {
                let id = session(value);
                worker_manager.create(id).unwrap();
                worker_manager.apply(id, E::PeerDiscovered).unwrap();
                assert_eq!(worker_manager.state(id), Ok(S::Discovered));
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn state_subscriptions_emit_snapshots_and_close_after_session_removal() {
        let manager = SessionManager::default();
        let id = session(9);
        manager.create(id).unwrap();
        let updates = manager.subscribe(id).unwrap();
        assert_eq!(updates.recv().unwrap(), S::Idle);

        manager.apply(id, E::PeerDiscovered).unwrap();
        assert_eq!(updates.recv().unwrap(), S::Discovered);
        manager
            .apply(
                id,
                E::TrustedPeerVerified {
                    fingerprint_verified: true,
                },
            )
            .unwrap();
        manager.apply(id, E::BeginNegotiation).unwrap();
        manager
            .apply(
                id,
                E::StartStreaming {
                    peer_authenticated: true,
                    capture_permitted: true,
                    first_frame_presented: true,
                },
            )
            .unwrap();
        assert_eq!(updates.recv().unwrap(), S::Paired);
        assert_eq!(updates.recv().unwrap(), S::Negotiating);
        assert_eq!(updates.recv().unwrap(), S::Streaming);
        manager.apply(id, E::Stop).unwrap();
        assert_eq!(updates.recv().unwrap(), S::Stopping);
        manager.apply(id, E::CleanupComplete).unwrap();
        assert_eq!(updates.recv().unwrap(), S::Stopped);
        manager.remove_stopped(id).unwrap();
        assert!(updates.recv().is_err());
    }
}
