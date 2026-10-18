# Spike report — trusted-peer SQLite dependency availability

**Date:** 2026-10-18  
**Result:** BLOCKED on local crate registry access; no SQLite adapter or dependency was added.

## Question

Can CORE implement the required persistent `TrustedPeerPort` adapter with a reviewed, locked SQLite library in this environment?

## Candidate reviewed

`rusqlite 0.39.0` documents its `bundled` feature for compiling SQLite into the application and declares MIT licensing. The bundled SQLite source is public domain. The proposed manifest disables default features and enables only `bundled`, limiting optional dependency surface. See [rusqlite 0.39.0](https://docs.rs/crate/rusqlite/0.39.0) and [SQLite's copyright statement](https://www.sqlite.org/copyright.html).

## Environment evidence

- No `rusqlite` or `libsqlite3-sys` source/archive is present in the local Cargo registry cache.
- `cargo fetch` was attempted with the pinned Rust 1.99.1 Cargo executable on 2026-10-18. It failed while updating the crates.io index with `Could not connect to server` for `index.crates.io:443`.
- No SQLite Cargo dependency was left in `Cargo.toml`; no `Cargo.lock` update was made. Temporary adapter tests were removed rather than leaving a red workspace.
- The trusted-peer domain type and `TrustedPeerPort` interface remain in the project. They have no persistent implementation.

## Next evidence needed

Restore crates.io access or provide an approved local registry/cache, then resolve exact transitive versions, update the lockfile, regenerate the Rust license notice, run advisory/license/source checks, and implement the transactional schema migration and repository tests. Validate Android/iOS/Linux builds on their actual runners before claiming cross-platform packaging.
