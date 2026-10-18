use std::thread;

use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use mm_domain::{SessionEvent, SessionId, SessionState};

use crate::FfiController;

/// Narrow Flutter API. Pairing, identity evidence, and capture permission are not writable from Dart.
#[frb(opaque)]
pub struct BridgeController {
    inner: FfiController,
}

impl BridgeController {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: FfiController::default(),
        }
    }

    #[frb(sync)]
    pub fn create_session(&self, session_id: String) -> Result<(), String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .create_session(id)
            .map_err(|error| error.to_string())
    }

    #[frb(sync)]
    pub fn current_state(&self, session_id: String) -> Result<String, String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .current_state(id)
            .map(|state| format!("{state:?}"))
            .map_err(|error| error.to_string())
    }

    #[frb(sync)]
    pub fn discover_demo_receiver(&self, session_id: String) -> Result<String, String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .discover_demo_receiver(id)
            .map(|state| format!("{state:?}"))
            .map_err(|error| error.to_string())
    }

    #[frb(sync)]
    pub fn begin_demo_pairing(&self, session_id: String) -> Result<String, String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .begin_demo_pairing(id)
            .map(|state| format!("{state:?}"))
            .map_err(|error| error.to_string())
    }

    #[frb(sync)]
    pub fn disconnect_demo_receiver(&self, session_id: String) -> Result<String, String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .disconnect_demo_receiver(id)
            .map(|state| format!("{state:?}"))
            .map_err(|error| error.to_string())
    }

    #[frb(sync)]
    pub fn stop_session(&self, session_id: String) -> Result<String, String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        self.inner
            .dispatch(crate::FfiCommand::ApplySessionEvent {
                session_id: id,
                event: SessionEvent::Stop,
            })
            .map(|change| format!("{:?}", change.state))
            .map_err(|error| error.to_string())
    }

    pub fn session_state_events(
        &self,
        session_id: String,
        sink: StreamSink<String>,
    ) -> Result<(), String> {
        let id = SessionId::parse(&session_id).map_err(|error| error.to_string())?;
        let updates = self
            .inner
            .subscribe_session(id)
            .map_err(|error| error.to_string())?;
        thread::spawn(move || {
            while let Ok(state) = updates.recv() {
                if sink.add(format!("{state:?}")).is_err() || state == SessionState::Stopped {
                    break;
                }
            }
        });
        Ok(())
    }
}
