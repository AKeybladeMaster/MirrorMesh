# ADR-0007 — Ed25519 identity candidate

- **Decision date:** 2026-10-18
- **Status:** Proposed encoding and verifier candidate; cross-track/security review pending

## Context

The Task 03 pairing coordinator verifies a signed proof through `PairingIdentityPort`. [ADR-0004](0004-trusted-peer-records.md) defines a SHA-256 fingerprint without fixing the public-key encoding, and [ADR-0006](0006-pairing-coordinator-proof-contract.md) leaves the key and signature algorithms open. A stateless public-key verifier now exists in `crates/mm-adapter-identity`; platform teams still need cross-track review before relying on its encoding and verification profile.

## Proposed decision

- Use Ed25519 as specified by RFC 8032 for the pairing identity proof.
- Carry the raw 32-byte Ed25519 public key and raw 64-byte Ed25519 signature. The signed message remains the exact domain-separated `PairingProof::signed_bytes()` contract in ADR-0006.
- Compute the existing 32-byte SHA-256 peer fingerprint over the exact 32 raw public-key bytes. Preserve the QR representation of that digest as 64 lowercase hexadecimal characters.
- Require verification to reject malformed/non-canonical key encodings, weak/small-order public keys, and invalid signatures. The current candidate checks canonical point encodings and small-order keys, then uses `ed25519-dalek`'s `verify_strict`; RFC 8032 and malformed-input tests cover these checks. `verify_strict` is stricter than the RFC 8032 acceptance rules, so the exact profile remains subject to cross-track review.
- Keep private-key generation, signing, rotation, and deletion in a platform identity adapter using the operating system's protected key store. Never put private-key material or key-store secrets in SQLite or FFI events.
- Pin the verifier candidate to `ed25519-dalek = 3.0.0`, `curve25519-dalek = 5.0.0`, and `sha2 = 0.11.0`. The first two packages declare BSD-3-Clause; that SPDX license was reviewed and added to the dependency allow-list. Defaults are disabled; `ed25519-dalek` enables `zeroize` only. The exact graph is locked and has passed local offline tests and `cargo-deny` checks. This does not constitute a security audit or protocol approval.

## Consequences and open gates

- This encoding is a proposed interoperability contract only. It must not be treated as frozen until cross-track and security review accept the ADR and confirm exact-byte behavior.
- `PairingIdentityPort` still has no local signer or key-lifecycle API. Define that contract and implement protected OS key storage separately. The verifier is not yet wired to the pairing coordinator, and no QR UI, SQLite repository, discovery, authenticated signaling, or transport closure exists.
- The exact candidate versions, package metadata, licenses, and source were checked; the locked package tests, workspace tests, formatting, Clippy, and offline `cargo-deny` policy check pass. The cached advisory database was not refreshed from the network, and a target-platform build has not been run.
## References

- [RFC 8032 — Edwards-Curve Digital Signature Algorithm (EdDSA)](https://www.rfc-editor.org/rfc/rfc8032.html)
- [ed25519-dalek 3.0.0 package metadata](https://docs.rs/crate/ed25519-dalek/3.0.0)
- [ed25519-dalek 3.0.0 `VerifyingKey` API](https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/struct.VerifyingKey.html)
- [curve25519-dalek 5.0.0 package metadata](https://docs.rs/crate/curve25519-dalek/5.0.0)
- [sha2 0.11.0 package metadata](https://docs.rs/crate/sha2/0.11.0)
- [BSD 3-Clause license](https://opensource.org/license/bsd-3-clause)
- [Identity provider spike evidence](../spikes/identity-provider.md)
