//! Control-plane surface intended for a generated Flutter bridge.
//! Events contain session state only; video frames and audio buffers never cross this API.

#[cfg(any(
    target_os = "windows",
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos"
))]
mod connection_command;
mod frb_generated;
#[cfg(any(
    target_os = "windows",
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos"
))]
mod native_platform;
#[cfg(any(
    target_os = "windows",
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos"
))]
mod native_receiver;
mod network_devices;

#[cfg(target_os = "android")]
mod android_context;

#[cfg(target_os = "android")]
mod android_tv_pairing;

#[cfg(target_os = "linux")]
mod linux_pairing;

#[cfg(target_os = "windows")]
mod windows_pairing;

pub mod bridge_api;

use mm_domain::{DomainError, SessionEvent, SessionId, SessionState};
use mm_runtime::{RuntimeError, SessionManager};
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCommand {
    ApplySessionEvent {
        session_id: SessionId,
        event: SessionEvent,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionStateChanged {
    pub session_id: SessionId,
    pub state: SessionState,
}

#[derive(Debug, Clone, Default)]
pub struct FfiController {
    sessions: SessionManager,
}

impl FfiController {
    pub fn create_session(&self, session_id: SessionId) -> Result<(), RuntimeError> {
        self.sessions.create(session_id)
    }

    pub fn dispatch(&self, command: FfiCommand) -> Result<SessionStateChanged, RuntimeError> {
        match command {
            FfiCommand::ApplySessionEvent { session_id, event } => {
                let state = self.sessions.apply(session_id, event)?;
                Ok(SessionStateChanged { session_id, state })
            }
        }
    }

    pub fn current_state(&self, session_id: SessionId) -> Result<SessionState, RuntimeError> {
        self.sessions.state(session_id)
    }

    /// Records only discovery for the local Flutter demo; it cannot assert trust or permission.
    pub fn discover_demo_receiver(
        &self,
        session_id: SessionId,
    ) -> Result<SessionState, RuntimeError> {
        match self.sessions.state(session_id)? {
            SessionState::Idle => self
                .sessions
                .apply(session_id, SessionEvent::PeerDiscovered),
            SessionState::Discovered => Ok(SessionState::Discovered),
            SessionState::Stopped => {
                self.sessions.apply(session_id, SessionEvent::Reset)?;
                self.sessions
                    .apply(session_id, SessionEvent::PeerDiscovered)
            }
            _ => Err(RuntimeError::Domain(DomainError::IllegalTransition)),
        }
    }

    /// Begins pairing after discovery without asserting a verified identity or approval.
    pub fn begin_demo_pairing(&self, session_id: SessionId) -> Result<SessionState, RuntimeError> {
        match self.sessions.state(session_id)? {
            SessionState::Discovered => self.sessions.apply(session_id, SessionEvent::BeginPairing),
            SessionState::Pairing => Ok(SessionState::Pairing),
            _ => Err(RuntimeError::Domain(DomainError::IllegalTransition)),
        }
    }

    /// Stops the local demo and completes its in-memory cleanup without starting media.
    pub fn disconnect_demo_receiver(
        &self,
        session_id: SessionId,
    ) -> Result<SessionState, RuntimeError> {
        match self.sessions.state(session_id)? {
            SessionState::Idle | SessionState::Stopped => self.sessions.state(session_id),
            _ => {
                let state = self.sessions.apply(session_id, SessionEvent::Stop)?;
                if state == SessionState::Stopping {
                    self.sessions
                        .apply(session_id, SessionEvent::CleanupComplete)
                } else {
                    Ok(state)
                }
            }
        }
    }

    pub fn subscribe_session(
        &self,
        session_id: SessionId,
    ) -> Result<Receiver<SessionState>, RuntimeError> {
        self.sessions.subscribe(session_id)
    }
}
