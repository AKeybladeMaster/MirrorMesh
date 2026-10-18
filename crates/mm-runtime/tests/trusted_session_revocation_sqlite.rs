use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mm_domain::{
    DeviceId, PeerFingerprint, PeerPublicKey, SessionId, TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{PortError, TransportPort, TrustedPeerPort};
use mm_protocol::Envelope;
use mm_runtime::{TrustedSessionError, TrustedSessionRuntime};
use mm_store::SqliteTrustedPeers;

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDatabase(PathBuf);

impl TempDatabase {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "mirrormesh-session-revocation-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self) -> PathBuf {
        self.0.join("trusted-peers.sqlite")
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<TransportState>>);

#[derive(Default)]
struct TransportState {
    started: Vec<(SessionId, DeviceId)>,
    closed: Vec<SessionId>,
}

impl TransportPort for RecordingTransport {
    fn start(&mut self, session_id: SessionId, peer_id: DeviceId) -> Result<(), PortError> {
        self.0.lock().unwrap().started.push((session_id, peer_id));
        Ok(())
    }

    fn send_control(&mut self, _message: &Envelope<Vec<u8>>) -> Result<(), PortError> {
        Ok(())
    }

    fn close(&mut self, session_id: SessionId) -> Result<(), PortError> {
        self.0.lock().unwrap().closed.push(session_id);
        Ok(())
    }
}

fn peer(id: u8, fingerprint: u8) -> TrustedPeer {
    TrustedPeer::new(
        DeviceId::from_bytes([id; 16]),
        format!("Peer {id}"),
        PeerPublicKey::new(vec![id; 32]).unwrap(),
        PeerFingerprint::from_bytes([fingerprint; 32]),
        TrustedPeerGrants::new(true, false, false).unwrap(),
        1_000,
    )
    .unwrap()
}

fn session(value: u8) -> SessionId {
    SessionId::from_bytes([value; 16])
}

fn view_scope() -> TrustedPeerGrants {
    TrustedPeerGrants::new(true, false, false).unwrap()
}

#[test]
fn revoke_closes_only_owned_sessions_and_survives_sqlite_reopen() {
    let database = TempDatabase::new();
    let first_peer = peer(1, 11);
    let second_peer = peer(2, 22);
    let first_peer_id = first_peer.id();
    let second_peer_id = second_peer.id();
    let store = SqliteTrustedPeers::open(database.file()).unwrap();
    store.save_peer(&first_peer).unwrap();
    store.save_peer(&second_peer).unwrap();

    let transport = RecordingTransport::default();
    let transport_state = Arc::clone(&transport.0);
    {
        let mut runtime = TrustedSessionRuntime::new(&store, transport);
        runtime
            .start_session(first_peer_id, session(1), view_scope())
            .unwrap();
        runtime
            .start_session(first_peer_id, session(2), view_scope())
            .unwrap();
        runtime
            .start_session(second_peer_id, session(3), view_scope())
            .unwrap();

        runtime.revoke_peer(first_peer_id, 2_000).unwrap();

        assert_eq!(
            transport_state.lock().unwrap().closed,
            vec![session(1), session(2)]
        );
        assert!(!store.is_trusted(first_peer_id).unwrap());
        assert!(store.is_trusted(second_peer_id).unwrap());
    }
    drop(store);

    let reopened_store = SqliteTrustedPeers::open(database.file()).unwrap();
    assert!(!reopened_store.is_trusted(first_peer_id).unwrap());
    assert!(reopened_store.is_trusted(second_peer_id).unwrap());

    let restarted_transport = RecordingTransport::default();
    let restarted_state = Arc::clone(&restarted_transport.0);
    let mut restarted_runtime = TrustedSessionRuntime::new(&reopened_store, restarted_transport);
    assert_eq!(
        restarted_runtime.start_session(first_peer_id, session(4), view_scope()),
        Err(TrustedSessionError::PeerNotTrusted)
    );
    restarted_runtime
        .start_session(second_peer_id, session(5), view_scope())
        .unwrap();
    assert_eq!(
        restarted_state.lock().unwrap().started,
        vec![(session(5), second_peer_id)]
    );
}
