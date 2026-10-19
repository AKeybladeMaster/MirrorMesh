use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_adapter_tls::{tls13_pinned_client_config, tls13_server_config};
use mm_application::PairingProof;
use mm_domain::{DeviceId, MessageId, PeerGrant, PeerPublicKey, TrustedPeerGrants};
use mm_ports::{
    LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort,
};
use mm_protocol::{PairBegin, encode_pair_begin};
use mm_runtime::PairingRuntime;
use mm_security::parse_pairing_qr;
use mm_store::SqliteTrustedPeers;
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};
use tungstenite::client::IntoClientRequest;
use tungstenite::client::client_with_config;
use tungstenite::protocol::Message;

const CHILD_MODE: &str = "MIRRORMESH_WSS_PAIRING_CHILD";
const RECEIVER_ID: DeviceId = DeviceId::from_bytes([1; 16]);
const SENDER_ID: DeviceId = DeviceId::from_bytes([2; 16]);
const RECEIVER_SEED: [u8; 32] = [0x11; 32];
const SENDER_SEED: [u8; 32] = [0x42; 32];
const NOW_MONOTONIC_MS: u64 = 500;
const NOW_UNIX_SECONDS: u64 = 1_000_000;

#[test]
fn wss_pair_begin_is_pinned_and_waits_for_approval() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_server_child();
        return;
    }

    run_client_parent();
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
    let (admitted, websocket) = pending.into_parts();
    let approval = admitted.approval();
    assert_eq!(admitted.source_ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
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
        admitted.source_ip(),
        approval.peer_id(),
        approval.fingerprint().to_hex()
    );
    std::io::stdout().flush().expect("flush pending approval");

    let mut approval_command = String::new();
    std::io::stdin()
        .read_line(&mut approval_command)
        .expect("read explicit test approval");
    assert_eq!(approval_command.trim(), "approve");

    let approved = runtime
        .approve(
            admitted,
            TrustedPeerGrants::new(true, false, false).expect("view grant scope"),
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
    drop(websocket);
    println!("APPROVED={}|view", approved.id());
    std::io::stdout().flush().expect("flush approved peer");
}

fn run_client_parent() {
    let mut child = spawn_server_child();
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
    let proof = PairingProof::create_signed(
        qr.device_id(),
        qr.nonce(),
        SENDER_ID,
        "Sender test process".to_owned(),
        TrustedPeerGrants::new(true, false, false).expect("view grant scope"),
        &sender,
    )
    .expect("create signed sender proof");
    let frame = encode_proof(&proof);
    let client_config = tls13_pinned_client_config(qr.fingerprint()).expect("pin QR fingerprint");
    let socket = TcpStream::connect(qr.endpoint().socket_addr()).expect("connect to child process");
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set client read timeout");
    socket
        .set_write_timeout(Some(Duration::from_secs(10)))
        .expect("set client write timeout");
    let server_name = ServerName::try_from("mirrormesh.local").expect("static server name");
    let tls = StreamOwned::new(
        ClientConnection::new(client_config, server_name).expect("create pinned TLS client"),
        socket,
    );
    let request = format!("wss://127.0.0.1:{port}/v1/control")
        .into_client_request()
        .expect("build control WebSocket request");
    let (mut websocket, _) = client_with_config(request, tls, None)
        .expect("complete QR-pinned TLS and WebSocket handshakes");
    websocket
        .send(Message::text(
            String::from_utf8(frame).expect("pair.begin frame is UTF-8 JSON"),
        ))
        .expect("send signed pair.begin");

    let pending_line = read_prefixed_line(&mut output, &mut transcript, "PENDING=");
    assert!(pending_line.ends_with("|untrusted"), "{pending_line}");
    let mut child_stdin = child.stdin.take().expect("open approval channel to child");
    child_stdin
        .write_all(b"approve\n")
        .expect("send explicit test approval");
    child_stdin.flush().expect("flush explicit test approval");

    let approved_line = read_prefixed_line(&mut output, &mut transcript, "APPROVED=");
    assert!(approved_line.ends_with("|view"), "{approved_line}");
    drop(websocket);
    drop(child_stdin);
    let status = child.wait().expect("wait for WSS server child");
    assert!(
        status.success(),
        "server child failed; output:\n{transcript}"
    );
}

fn spawn_server_child() -> Child {
    Command::new(std::env::current_exe().expect("current integration test executable"))
        .arg("--exact")
        .arg("wss_pair_begin_is_pinned_and_waits_for_approval")
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

fn encode_proof(proof: &PairingProof) -> Vec<u8> {
    let grants = proof.requested_grants();
    let requested_grants = [
        (PeerGrant::View, "view"),
        (PeerGrant::Send, "send"),
        (PeerGrant::Annotate, "annotate"),
    ]
    .into_iter()
    .filter(|(grant, _)| grants.allows(*grant))
    .map(|(_, name)| name.to_owned())
    .collect();
    let payload = PairBegin {
        receiver_id: proof.receiver_id().to_string(),
        nonce: proof.nonce().to_hex(),
        peer_id: proof.peer_id().to_string(),
        display_name: proof.display_name().to_owned(),
        public_key: encode_hex(proof.public_key().as_bytes()),
        requested_grants,
        signature: encode_hex(proof.signature()),
    };
    encode_pair_begin(
        MessageId::from_bytes([0xB8; 16]),
        1,
        NOW_UNIX_SECONDS * 1000,
        &payload,
    )
    .expect("encode bounded pair.begin frame")
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}
