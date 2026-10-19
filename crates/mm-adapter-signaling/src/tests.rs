use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::thread;
use std::time::{Duration, Instant};

use tungstenite::http::{Method, Request, Uri};

use super::{
    HandshakeDeadlineStream, MAX_PAIR_BEGIN_BYTES, MAX_PENDING_PAIRING_CONNECTIONS,
    PendingConnectionSlot, is_control_request, remaining_until, websocket_config,
};

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
