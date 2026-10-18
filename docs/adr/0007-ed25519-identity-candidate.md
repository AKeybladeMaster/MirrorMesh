# ADR-0007 — Ed25519 identity candidate

- **Decision date:** 2026-10-18
- **Status:** Proposed encoding only; crate/version unresolved; cross-track/security review pending

## Context

The Task 03 pairing coordinator verifies a signed proof through `PairingIdentityPort`, but that port has only fake implementations. [ADR-0004](0004-trusted-peer-records.md) defines a SHA-256 fingerprint without fixing the public-key encoding, and [ADR-0006](0006-pairing-coordinator-proof-contract.md) leaves the key and signature algorithms open. Platform teams need one candidate encoding before implementing QR identity comparison and signed challenge handling.

## Proposed decision

- Use Ed25519 as specified by RFC 8032 for the pairing identity proof.
- Carry the raw 32-byte Ed25519 public key and raw 64-byte Ed25519 signature. The signed message remains the exact domain-separated `PairingProof::signed_bytes()` contract in ADR-0006.
- Compute the existing 32-byte SHA-256 peer fingerprint over the exact 32 raw public-key bytes. Preserve the QR representation of that digest as 64 lowercase hexadecimal characters.
- Require verification to reject malformed/non-canonical key encodings, weak/small-order public keys, and invalid signatures. Confirm those properties against the exact provider source and add negative test vectors before adopting the contract.
- Keep private-key generation, signing, rotation, and deletion in a platform identity adapter using the operating system's protected key store. Never put private-key material or key-store secrets in SQLite or FFI events.
- No implementation crate is selected. The earlier `ed25519-compact = 2.6.0` candidate could not be verified: accessible docs.rs views in the current audit differ, and the exact-version page and package index were not accessible. Reconfirm the published version before reviewing its exact source, features, licenses, target builds, and advisories. Do not reuse 2.4.x metadata as evidence for 2.6.0.

## Consequences and open gates

- This encoding is a proposed interoperability contract only. It must not be treated as frozen until cross-track and security review accept the ADR and confirm exact-byte behavior.
- `PairingIdentityPort` currently verifies and fingerprints remote keys but has no local signer or key-lifecycle API. Define that port before implementing outbound pairing identity.
- `cargo fetch` failed because this environment cannot reach crates.io. The exact candidate version is unresolved and no source/archive, manifest/license, feature graph, target build, or advisory audit is available; do not change the workspace manifest or lockfile until those facts are verified.
- Alternative `ed25519-dalek 3.0.0` is outside the current license allow-list because it declares BSD-3-Clause. Reconsider only through explicit license-policy review.

## References

- [RFC 8032 — Edwards-Curve Digital Signature Algorithm (EdDSA)](https://www.rfc-editor.org/rfc/rfc8032.html)
- [ed25519-compact docs.rs latest manifest](https://docs.rs/crate/ed25519-compact/latest/source/Cargo.toml.orig) (the page opened in this audit identifies version 2.4.0)
- [ed25519-compact crates.io package page](https://crates.io/crates/ed25519-compact)
- [ed25519-compact 2.4.0 manifest (MIT metadata)](https://docs.rs/crate/ed25519-compact/2.4.0/source/Cargo.toml.orig)
- [ed25519-dalek 3.0.0 manifest](https://docs.rs/crate/ed25519-dalek/latest/source/Cargo.toml.orig)
- [Identity provider spike evidence](../spikes/identity-provider.md)
