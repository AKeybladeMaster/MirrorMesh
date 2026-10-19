# Spike report  trusted-peer SQLite adapter

**Date:** 2026-10-18  
**Result:** The earlier registry-access blocker is cleared for the ADR-pinned rusqlite version. A transactional SQLite adapter now implements `TrustedPeerPort`; Android/iOS/Linux builds and concrete application startup assembly remain open.

## Question

Can CORE provide durable trusted-peer storage with a reviewed, locked SQLite dependency while preserving the public-data-only boundary?

## Candidate and dependency review

- The existing [ADR-0005](../adr/0005-sqlite-trusted-peer-store.md) pins `rusqlite 0.39.0`, with default features disabled and only `bundled` enabled. Cargo info reports MIT for this exact release; the upstream manifest also declares MIT. The current latest release is newer, but this implementation follows the existing versioned ADR pending review.
- `rusqlite 0.39.0` resolves `libsqlite3-sys 0.37.0`; both package metadata entries declare MIT. The bundled SQLite engine is public domain according to [SQLite's copyright statement](https://www.sqlite.org/copyright.html).
- The `rusqlite 0.39.0` manifest does not declare `rust-version`. The selected graph compiled and passed the focused integration tests on Rust 1.85, matching the workspace's declared MSRV.
- The lockfile adds 12 packages. `cargo-deny 0.20.2 --offline check` passes advisory, ban, license, and source checks against the cached advisory database. The database was not refreshed from the network; duplicate-version and existing `allo-isolate` manifest-license warnings remain.

## Implementation and local evidence

- `crates/mm-store/src/sqlite_trusted_peers.rs` implements `SqliteTrustedPeers` over the existing `TrustedPeerPort`.
- Schema version 1 is created transactionally. Unknown future versions fail closed. Prepared statements bind peer data. The schema stores only public peer fields, constrains lengths/grants/timestamps, retains revoked records, and caps the store at 1,024 records.
- `cargo test --locked --offline -p mm-store --test sqlite_trusted_peers` passes 10 tests on both Rust 1.99.1 and Rust 1.85. Tests cover persistence, SQL value binding, migration/rollback, future schemas, revocation/idempotence, reapproval, malformed data, missing peers, and capacity.
- `cargo check --locked --offline -p mm-store --all-targets` passes on Rust 1.85, including the bundled Windows SQLite C build.
- Workspace formatting and Clippy pass. The full `cargo test --locked --offline --workspace --all-features` suite passes: 69 unit tests, 3 FFI integration tests, 6 identity integration tests, 10 SQLite integration tests, and doc tests.
- Rust dependency notices were regenerated after the lockfile update. The updated file size and repeated-generation SHA-256 are recorded in [verification](../verification.md).

## Limits and next evidence

The adapter is a standalone port implementation; application startup does not yet inject it into `PairingCoordinator`. Its Windows build and tests do not prove Linux, Android, or iOS support. The store keeps only public data and does not encrypt its database. Remaining Task 03 work includes concrete platform assembly, local signing/key lifecycle, protected OS key storage, discovery, authenticated TLS/WSS signaling and rate limiting, pairing UI, wiring revocation closure and grant checks into signaling/platform resource teardown, and owned-LAN/device checks.
