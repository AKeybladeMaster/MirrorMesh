use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mm_domain::{DeviceId, PeerFingerprint, PeerPublicKey, TrustedPeer, TrustedPeerGrants};
use mm_ports::{PortError, TrustedPeerPort};
use mm_store::{MAX_TRUSTED_PEERS, SqliteTrustedPeers, StoreError};
use rusqlite::Connection;

static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(0);

struct DatabaseFile(PathBuf);

impl DatabaseFile {
    fn new() -> Self {
        let unique = NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!(
            "mirrormesh-trusted-peers-{}-{unique}.sqlite",
            std::process::id()
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DatabaseFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(self.0.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(self.0.with_extension("sqlite-shm"));
    }
}

fn peer(id_suffix: u64, name: &str, key_byte: u8, paired_at_ms: u64) -> TrustedPeer {
    let peer_id = DeviceId::parse(&format!("00000000-0000-0000-0000-{id_suffix:012x}")).unwrap();
    TrustedPeer::new(
        peer_id,
        name.to_owned(),
        PeerPublicKey::new(vec![key_byte; 32]).unwrap(),
        PeerFingerprint::from_bytes([key_byte.wrapping_add(1); 32]),
        TrustedPeerGrants::new(true, false, true).unwrap(),
        paired_at_ms,
    )
    .unwrap()
}

fn unique_peer(index: usize) -> TrustedPeer {
    peer(
        index as u64 + 1,
        "Capacity test peer",
        (index % 250) as u8 + 1,
        100,
    )
}

#[test]
fn persists_all_peer_fields_across_reopen_and_binds_untrusted_text() {
    let file = DatabaseFile::new();
    let mut expected = peer(
        1,
        "O'Reilly; DROP TABLE trusted_peers; --",
        0x41,
        1_700_000_000_000,
    );
    expected.record_seen(1_700_000_000_123).unwrap();

    {
        let store = SqliteTrustedPeers::open(file.path()).unwrap();
        store.save_peer(&expected).unwrap();
    }

    let store = SqliteTrustedPeers::open(file.path()).unwrap();
    assert_eq!(
        store.get_peer(expected.id()).unwrap(),
        Some(expected.clone())
    );
    assert_eq!(store.list_peers().unwrap(), vec![expected]);
}

#[test]
fn migration_sets_schema_version_and_creates_trusted_peer_table() {
    let file = DatabaseFile::new();
    drop(SqliteTrustedPeers::open(file.path()).unwrap());

    let connection = Connection::open(file.path()).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    let table_count: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'trusted_peers'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(version, 1);
    assert_eq!(table_count, 1);
}

#[test]
fn rejects_a_future_schema_version_without_mutating_it() {
    let file = DatabaseFile::new();
    let connection = Connection::open(file.path()).unwrap();
    connection.pragma_update(None, "user_version", 2).unwrap();
    drop(connection);

    assert!(matches!(
        SqliteTrustedPeers::open(file.path()),
        Err(StoreError::UnsupportedSchema(2))
    ));

    let connection = Connection::open(file.path()).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}

#[test]
fn migration_rolls_back_when_an_unversioned_database_has_a_conflicting_schema() {
    let file = DatabaseFile::new();
    let connection = Connection::open(file.path()).unwrap();
    connection
        .execute_batch("CREATE TABLE trusted_peers (unexpected TEXT);")
        .unwrap();
    drop(connection);

    assert!(matches!(
        SqliteTrustedPeers::open(file.path()),
        Err(StoreError::Sqlite(_))
    ));

    let connection = Connection::open(file.path()).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 0);
}

#[test]
fn keeps_revoked_peers_listed_and_revocation_is_idempotent() {
    let file = DatabaseFile::new();
    let store = SqliteTrustedPeers::open(file.path()).unwrap();
    let peer = peer(2, "Living room", 0x52, 1_700_000_100_000);
    let peer_id = peer.id();

    store.save_peer(&peer).unwrap();
    store.revoke(peer_id, 1_700_000_100_500).unwrap();
    store.revoke(peer_id, 1_700_000_100_600).unwrap();

    let revoked = store.get_peer(peer_id).unwrap().unwrap();
    assert_eq!(revoked.revoked_at_ms(), Some(1_700_000_100_500));
    assert!(!store.is_trusted(peer_id).unwrap());
    assert_eq!(store.list_peers().unwrap(), vec![revoked]);
}

#[test]
fn explicit_reapproval_replaces_the_revoked_record() {
    let store = SqliteTrustedPeers::open_in_memory().unwrap();
    let mut old_peer = peer(3, "Office laptop", 0x63, 1_700_000_200_000);
    let peer_id = old_peer.id();
    old_peer.revoke(1_700_000_200_100).unwrap();
    store.save_peer(&old_peer).unwrap();

    let reapproved = peer(3, "Office laptop", 0x64, 1_700_000_300_000);
    store.save_peer(&reapproved).unwrap();

    assert_eq!(store.get_peer(peer_id).unwrap(), Some(reapproved));
    assert!(store.is_trusted(peer_id).unwrap());
}

#[test]
fn rejects_corrupt_rows_instead_of_reconstructing_partial_peer_data() {
    let file = DatabaseFile::new();
    let peer = peer(4, "Bedroom display", 0x74, 1_700_000_400_000);
    let peer_id = peer.id();
    {
        let store = SqliteTrustedPeers::open(file.path()).unwrap();
        store.save_peer(&peer).unwrap();
    }

    let connection = Connection::open(file.path()).unwrap();
    connection
        .execute_batch(
            "PRAGMA ignore_check_constraints = ON;
             UPDATE trusted_peers SET fingerprint = x'00';",
        )
        .unwrap();
    drop(connection);

    let store = SqliteTrustedPeers::open(file.path()).unwrap();
    assert!(matches!(
        store.get_peer(peer_id),
        Err(PortError::StorageFailure)
    ));
}

#[test]
fn caps_the_trusted_peer_store() {
    let store = SqliteTrustedPeers::open_in_memory().unwrap();
    for index in 0..MAX_TRUSTED_PEERS {
        store.save_peer(&unique_peer(index)).unwrap();
    }

    assert!(matches!(
        store.save_peer(&unique_peer(MAX_TRUSTED_PEERS)),
        Err(PortError::StorageFailure)
    ));
    assert_eq!(store.list_peers().unwrap().len(), MAX_TRUSTED_PEERS);
}

#[test]
fn rejects_timestamps_that_precede_pairing() {
    let file = DatabaseFile::new();
    let store = SqliteTrustedPeers::open(file.path()).unwrap();
    let peer = peer(5, "Test peer", 0x85, 100);
    let peer_id = peer.id();

    store.save_peer(&peer).unwrap();
    assert!(matches!(
        store.revoke(peer_id, 99),
        Err(PortError::StorageFailure)
    ));
    assert_eq!(store.get_peer(peer_id).unwrap(), Some(peer));
}

#[test]
fn rejects_a_missing_peer_revoke_request() {
    let store = SqliteTrustedPeers::open_in_memory().unwrap();
    let missing_id = DeviceId::parse("00000000-0000-0000-0000-000000000099").unwrap();

    assert!(matches!(
        store.revoke(missing_id, 100),
        Err(PortError::StorageFailure)
    ));
}
