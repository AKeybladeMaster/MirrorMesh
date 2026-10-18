# ADR-0005 — SQLite trusted-peer store adapter

- **Decision date:** 2026-10-18
- **Status:** Proposed; implementation deferred because Cargo cannot reach crates.io in this environment

## Context

Task 03 requires a local SQLite table for public trusted-peer data. The current `mm-store` only has a bounded preferences file and no SQLite library. The store must work on Windows/Linux and the Android/iOS targets without storing private keys or depending on a preinstalled SQLite version. An implementation attempt could not fetch the candidate crate because no `rusqlite` source is cached and Cargo could not reach the crates.io index; no dependency or unresolved lockfile change was left in the project.

## Decision

- Use `rusqlite` pinned to `0.39.0`, default features disabled, and the `bundled` feature enabled when the reviewed dependency source is available.
- Bundle the SQLite engine so application builds do not depend on a machine's system SQLite installation. Keep the database limited to device ID, display name, public key, fingerprint, explicit grants, and pairing/last-seen/revocation timestamps.
- Apply schema version 1 transactionally and reject unsupported future schema versions. Use prepared statements and bound values for all peer data.
- Enforce the shared record limits in the domain and database constraints; cap returned peer records and preserve revoked rows for the settings/revocation view.
- Keep local private keys in OS keychain adapters; never add them or keychain secret material to this database.

## License and dependency review

The official rusqlite 0.39.0 package declares MIT and documents the `bundled` feature. The bundled SQLite source is public domain. This stays within the repository's existing MIT policy; the dependency graph and generated notice preview still need local `cargo-deny`, `cargo-audit`, and `cargo-about` verification after the lockfile update.

## Consequences

- SQLite will give `TrustedPeerPort` a durable local adapter and transaction boundary once implemented.
- Bundling adds a native C build and a larger package; Windows is the only platform runner available here. Android, iOS, and Linux builds remain unverified until their target toolchains are available.
- The database is not encrypted by this adapter. Store only public peer data; rely on app sandbox permissions and OS keychain protection for private keys.
- The existing preferences file remains separate and is not migrated by this ADR.

## Official references

- [rusqlite 0.39.0 documentation](https://docs.rs/crate/rusqlite/0.39.0)
- [SQLite copyright and public-domain statement](https://www.sqlite.org/copyright.html)
