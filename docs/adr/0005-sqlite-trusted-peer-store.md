# ADR-0005  SQLite trusted-peer store adapter

- **Decision date:** 2026-10-18
- **Status:** Proposed; adapter implemented and locally verified, cross-platform builds pending

## Context

Task 03 requires durable local storage for public trusted-peer data. `mm-store` previously contained only a bounded preferences file. The adapter must use a bundled SQLite engine so app builds do not rely on a system SQLite installation, and it must never persist private keys or key-store secrets.

## Decision

- Use `rusqlite = 0.39.0` with default features disabled and only `bundled` enabled.
- Bundle SQLite for application builds and keep the database limited to device ID, display name, public key, fingerprint, explicit grants, and pairing/last-seen/revocation timestamps.
- Apply schema version 1 in an immediate transaction and reject negative or unsupported future schema versions without changing them.
- Use prepared statements and bound values for all peer data.
- Enforce domain bounds and database constraints. Cap the store and returned list at 1,024 peers; fail closed if an externally modified database exceeds that bound.
- Retain revoked rows for settings and review. The application coordinator calls `save_peer` only after explicit approval; an approved re-pair for the same device ID replaces the stored row and clears its prior revocation.
- Keep private keys in OS keychain adapters; never add private-key material or keychain secrets to this database.

## License and dependency review

The exact `rusqlite 0.39.0` package declares MIT and its `bundled` feature compiles the SQLite sources. Its selected dependency graph passed the repository's offline `cargo-deny` advisory, ban, license, and source checks. The delivered SQLite engine code is public domain. The generated Rust notice snapshot includes the new dependency graph. The Cargo manifest does not declare an MSRV; focused Windows checks and tests passed on the workspace minimum Rust 1.85.

## Consequences and evidence

- `SqliteTrustedPeers` implements `TrustedPeerPort` in `crates/mm-store/`, serializes access to one connection, migrates an empty database transactionally, and rejects malformed stored records.
- Ten integration tests cover persistence across reopen, safe parameter binding, schema migration/rollback/future-version rejection, revocation and reapproval, malformed rows, and the peer cap.
- The Windows Rust 1.85 check and focused test suite pass. Linux, Android, and iOS builds remain unverified; the adapter has not yet been wired into application startup.
- The database is not encrypted. It stores public peer data only and relies on the app sandbox; private keys remain in OS-protected storage.
- The existing preferences file remains separate and is not migrated by this ADR.

## Official references

- [rusqlite 0.39.0 package metadata](https://docs.rs/crate/rusqlite/0.39.0)
- [rusqlite 0.39.0 manifest and license](https://docs.rs/crate/rusqlite/0.39.0/source/Cargo.toml.orig)
- [rusqlite 0.39.0 feature list](https://docs.rs/crate/rusqlite/0.39.0/features)
- [SQLite copyright and public-domain statement](https://www.sqlite.org/copyright.html)