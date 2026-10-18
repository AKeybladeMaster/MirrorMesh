use mm_domain::{SessionEvent, SessionId, SessionState};
use mm_ffi::{FfiCommand, FfiController};
use std::time::Duration;

fn session_id() -> SessionId {
    SessionId::parse("9a8c1b3e-4f25-4f86-9d42-18b0c350c067").expect("valid test UUID")
}

#[test]
fn state_event_subscription_carries_transitions_and_stops_cleanly() {
    let controller = FfiController::default();
    let id = session_id();
    controller.create_session(id).expect("session created");

    let events = controller
        .subscribe_session(id)
        .expect("event stream subscribed");
    assert_eq!(events.recv().expect("initial snapshot"), SessionState::Idle);

    let transitions = [
        (SessionEvent::PeerDiscovered, SessionState::Discovered),
        (SessionEvent::BeginPairing, SessionState::Pairing),
        (
            SessionEvent::PairingApproved {
                fingerprint_verified: true,
                user_approved: true,
            },
            SessionState::Paired,
        ),
        (SessionEvent::BeginNegotiation, SessionState::Negotiating),
        (
            SessionEvent::StartStreaming {
                peer_authenticated: true,
                capture_permitted: true,
                first_frame_presented: true,
            },
            SessionState::Streaming,
        ),
        (SessionEvent::Stop, SessionState::Stopping),
        (SessionEvent::CleanupComplete, SessionState::Stopped),
    ];

    for (event, expected_state) in transitions {
        controller
            .dispatch(FfiCommand::ApplySessionEvent {
                session_id: id,
                event,
            })
            .expect("event accepted");
        assert_eq!(
            events
                .recv_timeout(Duration::from_secs(1))
                .expect("state event"),
            expected_state,
        );
    }

    assert!(events.recv_timeout(Duration::from_millis(25)).is_err());
}

#[test]
fn unauthorized_stream_start_does_not_change_ffi_session_state() {
    let controller = FfiController::default();
    let id = session_id();
    controller.create_session(id).expect("session created");
    for event in [
        SessionEvent::PeerDiscovered,
        SessionEvent::BeginPairing,
        SessionEvent::PairingApproved {
            fingerprint_verified: true,
            user_approved: true,
        },
        SessionEvent::BeginNegotiation,
    ] {
        controller
            .dispatch(FfiCommand::ApplySessionEvent {
                session_id: id,
                event,
            })
            .expect("transition accepted");
    }

    let result = controller.dispatch(FfiCommand::ApplySessionEvent {
        session_id: id,
        event: SessionEvent::StartStreaming {
            peer_authenticated: true,
            capture_permitted: false,
            first_frame_presented: true,
        },
    });

    assert!(result.is_err());
    assert_eq!(
        controller
            .current_state(id)
            .expect("session remains available"),
        SessionState::Negotiating,
    );
}

#[test]
fn demo_receiver_flow_uses_only_safe_domain_transitions() {
    let controller = FfiController::default();
    let id = session_id();
    controller.create_session(id).expect("session created");

    assert!(controller.begin_demo_pairing(id).is_err());
    assert_eq!(
        controller.discover_demo_receiver(id),
        Ok(SessionState::Discovered),
    );
    assert_eq!(controller.begin_demo_pairing(id), Ok(SessionState::Pairing),);
    assert_eq!(controller.current_state(id), Ok(SessionState::Pairing));
    assert_eq!(
        controller.disconnect_demo_receiver(id),
        Ok(SessionState::Stopped),
    );
    assert_eq!(controller.current_state(id), Ok(SessionState::Stopped));

    assert_eq!(
        controller.discover_demo_receiver(id),
        Ok(SessionState::Discovered),
    );
}
