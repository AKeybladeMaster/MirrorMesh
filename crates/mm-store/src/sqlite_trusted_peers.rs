use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use mm_domain::{
    DeviceId, PeerFingerprint, PeerGrant, PeerPublicKey, TrustedPeer, TrustedPeerGrants,
};
use mm_ports::{PortError, TrustedPeerPort};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

use crate::StoreError;

const SCHEMA_VERSION: i64 = 1;
pub const MAX_TRUSTED_PEERS: usize = 1024;

const SCHEMA_V1: &str = "
CREATE TABLE trusted_peers (
    peer_id TEXT PRIMARY KEY NOT NULL CHECK (length(peer_id) = 36),
    display_name TEXT NOT NULL CHECK (length(display_name) BETWEEN 1 AND 80),
    public_key BLOB NOT NULL CHECK (length(public_key) BETWEEN 1 AND 512),
    fingerprint BLOB NOT NULL CHECK (length(fingerprint) = 32),
    grant_view INTEGER NOT NULL CHECK (grant_view IN (0, 1)),
    grant_send INTEGER NOT NULL CHECK (grant_send IN (0, 1)),
    grant_annotate INTEGER NOT NULL CHECK (grant_annotate IN (0, 1)),
    paired_at_ms INTEGER NOT NULL CHECK (paired_at_ms >= 0),
    last_seen_at_ms INTEGER CHECK (
        last_seen_at_ms IS NULL OR last_seen_at_ms >= paired_at_ms
    ),
    revoked_at_ms INTEGER CHECK (
        revoked_at_ms IS NULL OR revoked_at_ms >= paired_at_ms
    ),
    CHECK (
        revoked_at_ms IS NULL
        OR last_seen_at_ms IS NULL
        OR revoked_at_ms >= last_seen_at_ms
    )
);
";

const SELECT_COLUMNS: &str = "
SELECT peer_id, display_name, public_key, fingerprint,
       grant_view, grant_send, grant_annotate,
       paired_at_ms, last_seen_at_ms, revoked_at_ms
FROM trusted_peers
";

const UPSERT_PEER: &str = "
INSERT INTO trusted_peers (
    peer_id, display_name, public_key, fingerprint,
    grant_view, grant_send, grant_annotate,
    paired_at_ms, last_seen_at_ms, revoked_at_ms
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
ON CONFLICT(peer_id) DO UPDATE SET
    display_name = excluded.display_name,
    public_key = excluded.public_key,
    fingerprint = excluded.fingerprint,
    grant_view = excluded.grant_view,
    grant_send = excluded.grant_send,
    grant_annotate = excluded.grant_annotate,
    paired_at_ms = excluded.paired_at_ms,
    last_seen_at_ms = excluded.last_seen_at_ms,
    revoked_at_ms = excluded.revoked_at_ms
";

pub struct SqliteTrustedPeers {
    connection: Mutex<Connection>,
}

impl SqliteTrustedPeers {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let mut connection = Connection::open(path)?;
        Self::initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let mut connection = Connection::open_in_memory()?;
        Self::initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn initialize(connection: &mut Connection) -> Result<(), StoreError> {
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if !(0..=SCHEMA_VERSION).contains(&version) {
            return Err(StoreError::UnsupportedSchema(version));
        }
        if version == SCHEMA_VERSION {
            return Ok(());
        }

        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(SCHEMA_V1)?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        transaction.commit()?;
        Ok(())
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, PortError> {
        self.connection
            .lock()
            .map_err(|_| PortError::StorageFailure)
    }
}

impl TrustedPeerPort for SqliteTrustedPeers {
    fn get_peer(&self, peer_id: DeviceId) -> Result<Option<TrustedPeer>, PortError> {
        let connection = self.lock()?;
        let query = format!("{SELECT_COLUMNS} WHERE peer_id = ?1");
        let stored = connection
            .query_row(&query, params![peer_id.to_string()], StoredPeer::from_row)
            .optional()
            .map_err(map_sqlite_error)?;
        stored.map(StoredPeer::into_peer).transpose()
    }

    fn save_peer(&self, peer: &TrustedPeer) -> Result<(), PortError> {
        let id = peer.id().to_string();
        let paired_at_ms = to_sqlite_time(peer.paired_at_ms())?;
        let last_seen_at_ms = peer.last_seen_at_ms().map(to_sqlite_time).transpose()?;
        let revoked_at_ms = peer.revoked_at_ms().map(to_sqlite_time).transpose()?;
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;

        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM trusted_peers WHERE peer_id = ?1)",
                params![id],
                |row| row.get(0),
            )
            .map_err(map_sqlite_error)?;
        if !exists {
            let count: i64 = transaction
                .query_row("SELECT count(*) FROM trusted_peers", [], |row| row.get(0))
                .map_err(map_sqlite_error)?;
            if count >= MAX_TRUSTED_PEERS as i64 {
                return Err(PortError::StorageFailure);
            }
        }

        transaction
            .execute(
                UPSERT_PEER,
                params![
                    id,
                    peer.display_name(),
                    peer.public_key().as_bytes(),
                    peer.fingerprint().as_bytes().as_slice(),
                    peer.grants().allows(PeerGrant::View) as i64,
                    peer.grants().allows(PeerGrant::Send) as i64,
                    peer.grants().allows(PeerGrant::Annotate) as i64,
                    paired_at_ms,
                    last_seen_at_ms,
                    revoked_at_ms
                ],
            )
            .map_err(map_sqlite_error)?;
        transaction.commit().map_err(map_sqlite_error)
    }

    fn list_peers(&self) -> Result<Vec<TrustedPeer>, PortError> {
        let connection = self.lock()?;
        let query = format!("{SELECT_COLUMNS} ORDER BY paired_at_ms DESC, peer_id ASC LIMIT ?1");
        let mut statement = connection.prepare(&query).map_err(map_sqlite_error)?;
        let rows = statement
            .query_map([MAX_TRUSTED_PEERS as i64 + 1], StoredPeer::from_row)
            .map_err(map_sqlite_error)?;
        let stored_peers = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sqlite_error)?;
        if stored_peers.len() > MAX_TRUSTED_PEERS {
            return Err(PortError::StorageFailure);
        }
        stored_peers
            .into_iter()
            .map(StoredPeer::into_peer)
            .collect()
    }

    fn revoke(&self, peer_id: DeviceId, revoked_at_unix_ms: u64) -> Result<(), PortError> {
        let revoked_at_ms = to_sqlite_time(revoked_at_unix_ms)?;
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;
        let query = format!("{SELECT_COLUMNS} WHERE peer_id = ?1");
        let mut peer = transaction
            .query_row(&query, params![peer_id.to_string()], StoredPeer::from_row)
            .optional()
            .map_err(map_sqlite_error)?
            .ok_or(PortError::StorageFailure)?
            .into_peer()?;

        if !peer
            .revoke(revoked_at_unix_ms)
            .map_err(|_| PortError::StorageFailure)?
        {
            transaction.commit().map_err(map_sqlite_error)?;
            return Ok(());
        }

        let changed = transaction
            .execute(
                "UPDATE trusted_peers SET revoked_at_ms = ?1 WHERE peer_id = ?2",
                params![revoked_at_ms, peer_id.to_string()],
            )
            .map_err(map_sqlite_error)?;
        if changed != 1 {
            return Err(PortError::StorageFailure);
        }
        transaction.commit().map_err(map_sqlite_error)
    }
}

struct StoredPeer {
    peer_id: String,
    display_name: String,
    public_key: Vec<u8>,
    fingerprint: Vec<u8>,
    grant_view: i64,
    grant_send: i64,
    grant_annotate: i64,
    paired_at_ms: i64,
    last_seen_at_ms: Option<i64>,
    revoked_at_ms: Option<i64>,
}

impl StoredPeer {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            peer_id: row.get(0)?,
            display_name: row.get(1)?,
            public_key: row.get(2)?,
            fingerprint: row.get(3)?,
            grant_view: row.get(4)?,
            grant_send: row.get(5)?,
            grant_annotate: row.get(6)?,
            paired_at_ms: row.get(7)?,
            last_seen_at_ms: row.get(8)?,
            revoked_at_ms: row.get(9)?,
        })
    }

    fn into_peer(self) -> Result<TrustedPeer, PortError> {
        let id = DeviceId::parse(&self.peer_id).map_err(|_| PortError::StorageFailure)?;
        let public_key =
            PeerPublicKey::new(self.public_key).map_err(|_| PortError::StorageFailure)?;
        let fingerprint_bytes: [u8; 32] = self
            .fingerprint
            .as_slice()
            .try_into()
            .map_err(|_| PortError::StorageFailure)?;
        let fingerprint = PeerFingerprint::from_bytes(fingerprint_bytes);
        let grants = TrustedPeerGrants::new(
            from_sqlite_bool(self.grant_view)?,
            from_sqlite_bool(self.grant_send)?,
            from_sqlite_bool(self.grant_annotate)?,
        )
        .map_err(|_| PortError::StorageFailure)?;
        let mut peer = TrustedPeer::new(
            id,
            self.display_name,
            public_key,
            fingerprint,
            grants,
            from_sqlite_time(self.paired_at_ms)?,
        )
        .map_err(|_| PortError::StorageFailure)?;
        if let Some(last_seen_at_ms) = self.last_seen_at_ms {
            peer.record_seen(from_sqlite_time(last_seen_at_ms)?)
                .map_err(|_| PortError::StorageFailure)?;
        }
        if let Some(revoked_at_ms) = self.revoked_at_ms {
            peer.revoke(from_sqlite_time(revoked_at_ms)?)
                .map_err(|_| PortError::StorageFailure)?;
        }
        Ok(peer)
    }
}

fn from_sqlite_bool(value: i64) -> Result<bool, PortError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(PortError::StorageFailure),
    }
}

fn from_sqlite_time(value: i64) -> Result<u64, PortError> {
    value.try_into().map_err(|_| PortError::StorageFailure)
}

fn to_sqlite_time(value: u64) -> Result<i64, PortError> {
    value.try_into().map_err(|_| PortError::StorageFailure)
}

fn map_sqlite_error(_: rusqlite::Error) -> PortError {
    PortError::StorageFailure
}
