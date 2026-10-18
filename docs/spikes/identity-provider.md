# Spike report  pairing identity provider candidate

**Date:** 2026-10-18  
**Result:** A stateless Ed25519 public-key verifier candidate is implemented and locally tested. Its wire acceptance profile remains proposed pending cross-track and security review.

## Question

Which signature and public-key representation can back `PairingIdentityPort` and the signed proof contract in [ADR-0006](../adr/0006-pairing-coordinator-proof-contract.md), while respecting the repository's dependency license policy?

## Proposed identity representation

- Use Ed25519 as specified by [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html): a 32-byte raw public key and a 64-byte signature.
- Compute the existing SHA-256 fingerprint over the exact raw public-key bytes and expose it as 64 lowercase hexadecimal characters.
- Verify the exact domain-separated `PairingProof::signed_bytes()` payload. The current provider only implements the existing receiver-side fingerprint and verification port.
- Keep local private-key generation, signing, rotation, and deletion behind a separate platform identity adapter backed by the operating system's protected key store.

## Candidate dependency profile

- `ed25519-dalek = 3.0.0`, `curve25519-dalek = 5.0.0`, and `sha2 = 0.11.0` are exact-pinned in the workspace and lockfile. The SHA-2 version respects the workspace Rust 1.85 MSRV.
- Exact package metadata, source, and license terms were reviewed. `ed25519-dalek` and `curve25519-dalek` declare BSD-3-Clause; that license was reviewed and added to `deny.toml` and `about.toml`. `ed25519-dalek` has default features disabled and enables `zeroize` only; `curve25519-dalek` defaults are disabled.
- The provider checks 32-byte key length, canonical point encoding by decompressing and recompressing, and rejects small-order points. It checks 64-byte signature length and verifies with `VerifyingKey::verify_strict`.
- Upstream documents that `verify_strict` rejects weak keys and applies stricter signature checks than RFC 8032's generic verification behavior. The exact acceptance profile must be reviewed by every track before protocol adoption. See the [versioned VerifyingKey docs](https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/struct.VerifyingKey.html) and [package metadata](https://docs.rs/crate/ed25519-dalek/3.0.0).

## Local evidence

- Cargo resolved and fetched the exact pinned dependency graph. The dependency graph passed `cargo-deny 0.20.2 --offline check` for advisories, bans, licenses, and sources using the cached advisory database. The advisory database was not refreshed from the network. Warnings remain for `allo-isolate 0.1.27`'s missing manifest license field and duplicate crate versions.
- Six `mm-adapter-identity` tests pass: an RFC 8032 valid proof, known SHA-256 fingerprint, message/signature rejection, malformed input lengths, non-canonical public-key rejection, and low-order public-key rejection.
- `cargo fmt --all -- --check`, workspace Clippy with warnings denied, and `cargo test --locked --offline --workspace --all-features` pass. The workspace run includes 69 unit tests, 3 FFI integration tests, 6 identity integration tests, and doc tests.
- Rust notices were regenerated twice with `cargo-about 0.9.2`; the 174,946-byte snapshot hash is `1E2CA97DE557BC8925D25A217D44132664C52D1674C391C214F6685685140C25`.

## Limits and next evidence

The verifier is not wired into `PairingCoordinator`, and it does not create or manage private keys. No local signing/key-lifecycle API, protected OS key store, SQLite trusted-peer adapter, mDNS discovery, authenticated TLS/WSS listener, rate limiter, QR/approval UI, or revocation-driven transport closure is implemented. No target-platform build, owned-LAN pairing, or cross-track interoperability test ran. These remain separate Task 03 work; the proposed algorithm and verification profile are not frozen.