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
