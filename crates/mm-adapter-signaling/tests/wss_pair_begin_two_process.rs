use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_adapter_tls::tls13_server_config;
use mm_application::PairingCoordinatorError;
use mm_domain::{DeviceId, PeerGrant, PeerPublicKey, TrustedPeerGrants};
use mm_ports::{
    LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort,
};
use mm_protocol::{
    MessagePayload, SessionStart, encode_session_offer, encode_session_start, parse_message,
};
use mm_runtime::{PairingRuntime, PairingRuntimeError};
use mm_security::parse_pairing_qr;
use mm_store::SqliteTrustedPeers;

const CHILD_MODE: &str = "MIRRORMESH_WSS_PAIRING_CHILD";
const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);
const RECEIVER_SEED: [u8; 32] = [0x11; 32];
const SENDER_SEED: [u8; 32] = [0x42; 32];
const NOW_MONOTONIC_MS: u64 = 500;
const NOW_UNIX_SECONDS: u64 = 1_000_000;
const VALID_SESSION_OFFER: &str =
    include_str!("../../../tests/protocol-golden/session-offer-v1.json");
const VALID_SESSION_ANSWER: &str = r#"{
  "v": 1,
  "type": "session.answer",
  "message_id": "3182f5b4-a037-4f19-b2f7-c686ed10bd82",
  "session_id": "8b3d8788-1476-4aae-a287-334a428b5b6f",
  "seq": 1,
  "sent_at_ms": 1792316000001,
  "payload": {
    "sdp": "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=MirrorMesh test answer\r\nt=0 0\r\n",
    "selected_codec": "H264"
  }
}"#;

#[test]
fn wss_pair_begin_requires_approval_and_exchanges_control_messages() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_server_child();
        return;
    }

    run_client_parent(TestDecision::Approve);
}

#[test]
fn wss_pair_reject_is_sent_without_persisting_trust() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_server_child();
        return;
    }

    run_client_parent(TestDecision::Reject);
}

#[test]
fn expired_wss_pairing_approval_is_rejected_and_drops_connection() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_server_child();
        return;
    }

    run_client_parent(TestDecision::Expire);
}

#[test]
fn wss_sender_can_cancel_while_waiting_for_receiver_approval() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_server_child();
        return;
    }

    run_client_parent(TestDecision::Cancel);
}

#[derive(Clone, Copy)]
enum TestDecision {
    Approve,
    Reject,
    Expire,
    Cancel,
}

struct TestIdentity(SigningKey);

impl TestIdentity {
    fn new(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    fn public_key(&self) -> Result<PeerPublicKey, PortError> {
        PeerPublicKey::new(self.0.verifying_key().to_bytes().to_vec())
            .map_err(|_| PortError::IdentityRejected)
    }
}

impl LocalPairingIdentityPort for TestIdentity {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        self.public_key()
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        Ok(self.0.sign(message).to_bytes().to_vec())
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        Err(PortError::Unavailable)
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        Err(PortError::Unavailable)
    }
}

impl LocalTlsIdentityPort for TestIdentity {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError> {
        self.public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        Ok(self.0.sign(message).to_bytes().to_vec())
    }
}

impl PairingIdentityPort for TestIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<mm_domain::PeerFingerprint, PortError> {
        Ed25519PairingIdentity.fingerprint_public_key(public_key)
    }

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        Ed25519PairingIdentity.verify_signature(public_key, message, signature)
    }
}

fn run_server_child() {
    let identity = Arc::new(TestIdentity::new(RECEIVER_SEED));
    let peers =
        Arc::new(SqliteTrustedPeers::open_in_memory().expect("open in-memory trusted-peer store"));
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let tls_identity: Arc<dyn LocalTlsIdentityPort> = Arc::new(TestIdentity::new(RECEIVER_SEED));
    let tls = tls13_server_config(tls_identity).expect("build QR identity TLS config");
    let listener = mm_adapter_signaling::WssPairingListener::bind(address, tls)
        .expect("bind local WSS listener");
    let local_address = listener.local_addr().expect("read bound listener address");
    let mut runtime = PairingRuntime::new(RECEIVER_ID, Arc::clone(&identity), Arc::clone(&peers))
        .expect("build pairing runtime");
    let invitation = runtime
        .create_invitation(
            &local_address.to_string(),
            &[local_address.port()],
            NOW_MONOTONIC_MS,
            NOW_UNIX_SECONDS,
        )
        .expect("create receiver QR invitation");

    println!("INVITATION={}", invitation.uri());
    std::io::stdout().flush().expect("flush invitation");

    let pending = listener
        .accept_pairing_request(&mut runtime, NOW_MONOTONIC_MS + 1)
        .expect("accept and verify the incoming WSS pairing request");
    let approval = pending.admitted().approval();
    assert_eq!(
        pending.admitted().source_ip(),
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    );
    assert_eq!(approval.peer_id(), SENDER_ID);
    assert!(approval.requested_grants().allows(PeerGrant::View));
    assert!(
        peers
            .get_peer(SENDER_ID)
            .expect("read trusted peer before approval")
            .is_none()
    );
    println!(
        "PENDING={}|{}|{}|untrusted",
        pending.admitted().source_ip(),
        approval.peer_id(),
        approval.fingerprint().to_hex()
    );
    std::io::stdout().flush().expect("flush pending approval");

    let mut approval_command = String::new();
    std::io::stdin()
        .read_line(&mut approval_command)
        .expect("read explicit test approval decision");

    let requested = TrustedPeerGrants::new(true, false, false).expect("view grant scope");
    match approval_command.trim() {
        "approve" => {
            let (approved, mut connection) = pending
                .approve(
                    &runtime,
                    requested,
                    NOW_MONOTONIC_MS + 2,
                    NOW_UNIX_SECONDS * 1000 + 1,
                )
                .expect("persist the explicitly approved view grant");
            assert_eq!(
                peers
                    .get_peer(SENDER_ID)
                    .expect("read trusted peer after approval"),
                Some(approved.clone())
            );
            assert_eq!(connection.peer_id(), SENDER_ID);
            println!("APPROVED={}|view", approved.id());
            std::io::stdout().flush().expect("flush approved peer");
            let offer = connection
                .receive_control_message()
                .expect("receive a typed session offer after approval");
            assert_eq!(offer.header.kind, mm_protocol::MessageKind::SessionOffer);
            connection
                .send_control_message(VALID_SESSION_ANSWER.as_bytes())
                .expect("send a typed session answer after approval");
            let start = connection
                .receive_control_message()
                .expect("receive the post-connect session start message");
            assert_eq!(start.header.kind, mm_protocol::MessageKind::SessionStart);
            assert_eq!(start.header.session_id, offer.header.session_id);
            match start.payload {
                MessagePayload::SessionStart(start) => {
                    assert_eq!(start.source_id, "desktop-capture");
                }
                _ => panic!("session start had a different payload"),
            }
            println!("CONTROL_EXCHANGED=session.offer/session.answer/session.start");
            std::io::stdout()
                .flush()
                .expect("flush control exchange result");
        }
        "expire" => {
            let expires_at = pending.admitted().approval().expires_at_monotonic_ms();
            assert!(matches!(
                pending.approve(&runtime, requested, expires_at, NOW_UNIX_SECONDS * 1000 + 1,),
                Err(mm_adapter_signaling::WssApprovalError::Runtime(
                    PairingRuntimeError::Coordinator(PairingCoordinatorError::ApprovalExpired)
                ))
            ));
            assert!(
                peers
                    .get_peer(SENDER_ID)
                    .expect("read trusted peer after expired approval")
                    .is_none()
            );
            println!("EXPIRED=untrusted");
            std::io::stdout()
                .flush()
                .expect("flush expired approval result");
        }
        "reject" => {
            pending
                .reject("user_rejected", NOW_UNIX_SECONDS * 1000 + 1)
                .expect("send explicit pair.reject without persisting trust");
            assert!(
                peers
                    .get_peer(SENDER_ID)
                    .expect("read trusted peer after rejection")
                    .is_none()
            );
            println!("REJECTED=untrusted");
            std::io::stdout()
                .flush()
                .expect("flush rejected approval result");
        }
        "cancel" => {
            drop(pending);
            println!("CANCELLED=untrusted");
            std::io::stdout()
                .flush()
                .expect("flush cancelled approval result");
        }
        other => panic!("unexpected test approval decision: {other}"),
    }
}

fn run_client_parent(decision: TestDecision) {
    let (test_name, decision_text) = match decision {
        TestDecision::Approve => (
            "wss_pair_begin_requires_approval_and_exchanges_control_messages",
            "approve",
        ),
        TestDecision::Reject => ("wss_pair_reject_is_sent_without_persisting_trust", "reject"),
        TestDecision::Expire => (
            "expired_wss_pairing_approval_is_rejected_and_drops_connection",
            "expire",
        ),
        TestDecision::Cancel => (
            "wss_sender_can_cancel_while_waiting_for_receiver_approval",
            "cancel",
        ),
    };
    let mut child = spawn_server_child(test_name);
    let stdout = child.stdout.take().expect("capture server stdout");
    let mut output = BufReader::new(stdout);
    let mut transcript = String::new();

    let invitation = read_prefixed_line(&mut output, &mut transcript, "INVITATION=");
    let port = invitation_port(&invitation);
    let qr = parse_pairing_qr(&invitation, NOW_UNIX_SECONDS, &[port])
        .expect("parse the server's QR invitation");
    assert_eq!(qr.device_id(), RECEIVER_ID);
    assert_eq!(qr.endpoint().socket_addr().port(), port);

    let sender = TestIdentity::new(SENDER_SEED);
    let mut client = mm_adapter_signaling::WssPairingClient::begin_pairing(
        &invitation,
        NOW_UNIX_SECONDS,
        &[port],
        SENDER_ID,
        "Sender test process",
        TrustedPeerGrants::new(true, false, false).expect("view grant scope"),
        &sender,
        NOW_UNIX_SECONDS * 1000,
    )
    .expect("connect to the QR-pinned receiver and send signed pair.begin");

    let pending_line = read_prefixed_line(&mut output, &mut transcript, "PENDING=");
    assert!(pending_line.ends_with("|untrusted"), "{pending_line}");
    assert!(
        client
            .try_wait_for_decision()
            .expect("poll while receiver approval is pending")
            .is_none(),
        "no approval or rejection has been sent yet"
    );
    let mut child_stdin = child.stdin.take().expect("open approval channel to child");
    child_stdin
        .write_all(format!("{decision_text}\n").as_bytes())
        .expect("send explicit test approval decision");
    child_stdin.flush().expect("flush explicit test approval");

    match decision {
        TestDecision::Expire => {
            let expired_line = read_prefixed_line(&mut output, &mut transcript, "EXPIRED=");
            assert_eq!(expired_line, "untrusted");
            let error = match client.wait_for_decision() {
                Err(error) => error,
                Ok(_) => panic!("expired approval unexpectedly returned an approved channel"),
            };
            assert!(
                matches!(&error, mm_adapter_signaling::WssClientError::Closed),
                "expired approval returned {error:?}"
            );
        }
        TestDecision::Reject => {
            let rejected_line = read_prefixed_line(&mut output, &mut transcript, "REJECTED=");
            assert_eq!(rejected_line, "untrusted");
            assert!(matches!(
                client.wait_for_decision(),
                Err(mm_adapter_signaling::WssClientError::Rejected(reason))
                    if reason == "user_rejected"
            ));
        }
        TestDecision::Approve => {
            let approved_line = read_prefixed_line(&mut output, &mut transcript, "APPROVED=");
            assert!(approved_line.ends_with("|view"), "{approved_line}");
            let mut approved = client
                .try_wait_for_decision()
                .expect("poll for approval from the QR-pinned receiver")
                .expect("receiver approved the pairing request");
            assert_eq!(
                approved.accepted_grants,
                TrustedPeerGrants::new(true, false, false).expect("view grant scope")
            );
            assert_eq!(approved.peer_fingerprint, qr.fingerprint());
            let parsed_offer = parse_message(VALID_SESSION_OFFER.as_bytes())
                .expect("shared sender offer fixture is valid");
            let session_id = parsed_offer
                .header
                .session_id
                .expect("session offer has a session ID");
            let message_id = parsed_offer.header.message_id;
            let sequence = parsed_offer.header.sequence;
            let sent_at_ms = parsed_offer.header.sent_at_ms;
            let offer = match parsed_offer.payload {
                MessagePayload::SessionOffer(offer) => offer,
                _ => panic!("golden offer fixture has a different payload"),
            };
            let encoded_offer =
                encode_session_offer(message_id, session_id, sequence, sent_at_ms, &offer)
                    .expect("shared sender offer encoder accepts the fixture");
            approved
                .connection
                .send_control_message(&encoded_offer)
                .expect("send a session offer over the approved WSS channel");
            let answer = approved
                .connection
                .receive_control_message()
                .expect("receive a session answer over the approved WSS channel");
            assert_eq!(answer.header.kind, mm_protocol::MessageKind::SessionAnswer);
            assert_eq!(answer.header.session_id, Some(session_id));
            let start = encode_session_start(
                mm_domain::MessageId::from_bytes([0xd4; 16]),
                session_id,
                sequence + 1,
                sent_at_ms + 1,
                &SessionStart {
                    source_id: "desktop-capture".to_owned(),
                },
            )
            .expect("encode source-start control message");
            approved
                .connection
                .send_control_message(&start)
                .expect("send session start after the peer answer");
            let exchanged_line =
                read_prefixed_line(&mut output, &mut transcript, "CONTROL_EXCHANGED=");
            assert_eq!(exchanged_line, "session.offer/session.answer/session.start");
        }
        TestDecision::Cancel => {
            client.cancel();
            client.cancel();
            assert!(matches!(
                client.try_wait_for_decision(),
                Err(mm_adapter_signaling::WssClientError::Closed)
            ));
            let cancelled_line = read_prefixed_line(&mut output, &mut transcript, "CANCELLED=");
            assert_eq!(cancelled_line, "untrusted");
        }
    }
    drop(child_stdin);
    let status = child.wait().expect("wait for WSS server child");
    assert!(
        status.success(),
        "server child failed; output:\n{transcript}"
    );
}

fn spawn_server_child(test_name: &str) -> Child {
    Command::new(std::env::current_exe().expect("current integration test executable"))
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(CHILD_MODE, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("start local WSS server process")
}

fn read_prefixed_line(
    reader: &mut BufReader<impl Read>,
    transcript: &mut String,
    prefix: &str,
) -> String {
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .expect("read server process output");
        assert_ne!(
            read, 0,
            "server process ended before {prefix}; output:\n{transcript}"
        );
        transcript.push_str(&line);
        if let Some(value) = line.strip_prefix(prefix) {
            return value.trim().to_owned();
        }
    }
}

fn invitation_port(invitation: &str) -> u16 {
    invitation
        .split('&')
        .find_map(|field| field.strip_prefix("port="))
        .expect("QR invitation has a port")
        .parse()
        .expect("QR invitation port is numeric")
}
