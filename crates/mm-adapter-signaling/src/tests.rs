use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::thread;
use std::time::{Duration, Instant};

use mm_domain::{DeviceId, MessageId, SessionId};
use mm_protocol::{MessageKind, ProtocolError};
use tungstenite::http::{Method, Request, Uri};

use super::{
    ControlSequenceState, HandshakeDeadlineStream, MAX_PAIR_BEGIN_BYTES,
    MAX_PENDING_PAIRING_CONNECTIONS, PendingConnectionSlot, WssControlChannelError,
    is_control_request, parse_post_approval_control_frame, remaining_until, websocket_config,
};

const VALID_SESSION_OFFER: &str =
    include_str!("../../../tests/protocol-golden/session-offer-v1.json");
const VALID_PAIR_BEGIN: &str = include_str!("../../../tests/protocol-golden/pair-begin-v1.json");

#[test]
fn only_accepts_the_exact_native_control_websocket_endpoint() {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/control")
        .body(())
        .unwrap();
    assert!(is_control_request(&request));

    for uri in ["/", "/v1/control/", "/v1/control?debug=1", "/other"] {
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(())
            .unwrap();
        assert!(!is_control_request(&request), "accepted {uri}");
    }

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/control")
        .body(())
        .unwrap();
    assert!(!is_control_request(&request));

    let request = Request::builder()
        .method(Method::GET)
        .uri(Uri::from_static("/v1/control"))
        .header("Origin", "https://untrusted.example")
        .body(())
        .unwrap();
    assert!(!is_control_request(&request));
}

#[test]
fn websocket_frame_and_message_caps_match_the_protocol_envelope_limit() {
    let config = websocket_config();

    assert_eq!(MAX_PAIR_BEGIN_BYTES, 256 * 1024);
    assert_eq!(config.max_message_size, Some(MAX_PAIR_BEGIN_BYTES));
    assert_eq!(config.max_frame_size, Some(MAX_PAIR_BEGIN_BYTES));
    assert!(!config.accept_unmasked_frames);
    assert!(config.read_buffer_size <= 8 * 1024);
    assert!(config.max_write_buffer_size > config.write_buffer_size);
}

#[test]
fn pending_connection_slots_are_bounded_and_released_on_drop() {
    let active = Arc::new(AtomicUsize::new(0));
    let mut slots = Vec::new();

    for _ in 0..MAX_PENDING_PAIRING_CONNECTIONS {
        slots.push(
            PendingConnectionSlot::try_acquire(Arc::clone(&active))
                .expect("capacity remains for each permitted connection"),
        );
    }
    assert!(PendingConnectionSlot::try_acquire(Arc::clone(&active)).is_none());

    drop(slots);
    assert!(PendingConnectionSlot::try_acquire(Arc::clone(&active)).is_some());
}

#[test]
fn handshake_deadline_is_absolute_across_successive_reads() {
    use std::io::ErrorKind;

    let started = Instant::now();
    let deadline = started + Duration::from_secs(10);

    assert_eq!(
        remaining_until(deadline, started).unwrap(),
        Duration::from_secs(10)
    );
    assert_eq!(
        remaining_until(deadline, started + Duration::from_secs(7)).unwrap(),
        Duration::from_secs(3)
    );
    assert_eq!(
        remaining_until(deadline, deadline).unwrap_err().kind(),
        ErrorKind::TimedOut
    );
}

#[test]
fn slow_partial_handshake_cannot_extend_the_absolute_deadline() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let writer = thread::spawn(move || {
        let mut socket = TcpStream::connect(address).unwrap();
        socket.write_all(b"a").unwrap();
        thread::sleep(Duration::from_millis(900));
        socket.write_all(b"b").unwrap();
        thread::sleep(Duration::from_millis(900));
        let _ = socket.write_all(b"c");
    });

    let (socket, _) = listener.accept().unwrap();
    let mut stream =
        HandshakeDeadlineStream::new(socket, Instant::now() + Duration::from_millis(1_500));
    let mut byte = [0];

    assert_eq!(stream.read(&mut byte).unwrap(), 1);
    assert_eq!(stream.read(&mut byte).unwrap(), 1);
    let error = stream.read(&mut byte).unwrap_err();

    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ));
    writer.join().unwrap();
}

#[test]
fn approved_control_channel_accepts_v1_session_messages_and_rejects_pairing_frames() {
    let offer = parse_post_approval_control_frame(VALID_SESSION_OFFER.as_bytes()).unwrap();
    assert_eq!(offer.header.kind, MessageKind::SessionOffer);
    assert!(offer.header.session_id.is_some());

    assert!(matches!(
        parse_post_approval_control_frame(VALID_PAIR_BEGIN.as_bytes()),
        Err(WssControlChannelError::UnexpectedMessageKind)
    ));
}

#[test]
fn approved_control_channel_rejects_oversized_and_malformed_frames() {
    let oversized = vec![b'x'; MAX_PAIR_BEGIN_BYTES + 1];
    assert!(matches!(
        parse_post_approval_control_frame(&oversized),
        Err(WssControlChannelError::Protocol(
            ProtocolError::FrameTooLarge
        ))
    ));
    assert!(matches!(
        parse_post_approval_control_frame(b"not-json"),
        Err(WssControlChannelError::Protocol(ProtocolError::InvalidJson))
    ));
}

#[test]
fn approved_control_channel_enforces_per_peer_session_and_revoke_sequences() {
    let mut sequences = ControlSequenceState::new();
    let peer = DeviceId::from_bytes([0x22; 16]);
    let offer = mm_protocol::parse_message(VALID_SESSION_OFFER.as_bytes()).unwrap();

    sequences.accept(peer, &offer.header).unwrap();
    assert!(matches!(
        sequences.accept(peer, &offer.header),
        Err(WssControlChannelError::Protocol(
            ProtocolError::SequenceReplay
        ))
    ));

    let next_session = SessionId::from_bytes([0x55; 16]);
    let independent_session = mm_protocol::EnvelopeHeader {
        session_id: Some(next_session),
        ..offer.header
    };
    sequences.accept(peer, &independent_session).unwrap();

    let revoke_header = mm_protocol::EnvelopeHeader {
        version: 1,
        kind: MessageKind::PairRevoke,
        message_id: MessageId::from_bytes([0x44; 16]),
        session_id: None,
        sequence: 5,
        sent_at_ms: 10,
    };
    sequences.accept(peer, &revoke_header).unwrap();
    let stale_revoke = mm_protocol::EnvelopeHeader {
        sequence: 4,
        ..revoke_header
    };
    assert!(matches!(
        sequences.accept(peer, &stale_revoke),
        Err(WssControlChannelError::Protocol(
            ProtocolError::SequenceReplay
        ))
    ));
}
