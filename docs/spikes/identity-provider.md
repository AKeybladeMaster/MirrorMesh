# Spike report — pairing identity provider candidate

**Date:** 2026-10-18  
**Result:** Ed25519 encoding proposed; implementation crate/version unresolved; exact-source and security review blocked on registry access.

## Question

Which production signature/key representation can back `PairingIdentityPort` and the signed proof contract in [ADR-0006](../adr/0006-pairing-coordinator-proof-contract.md), while respecting the current dependency license allow-list?

## Proposed identity representation

- Use Ed25519 as specified by [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html): a 32-byte raw public key and a 64-byte signature. Use the canonical raw public-key bytes as the input to the existing SHA-256 fingerprint contract; expose the fingerprint as the existing 64-character lowercase hexadecimal string.
- Verify the existing `PairingProof::signed_bytes()` payload exactly. The verifier must reject malformed or non-canonical public keys and signatures, including weak/small-order public keys. Add published test vectors and malformed-key/signature tests before wiring the provider into the coordinator.
- Keep local private-key generation, signing, rotation, and deletion behind a platform identity adapter backed by the OS key store. The current `PairingIdentityPort` is receiver-side verification/fingerprint only; local signing and key lifecycle need an explicit port/API before a sender implementation is complete.

## Crate candidates

No crate version is currently verified as the implementation candidate. A prior entry named `ed25519-compact 2.6.0` based on a docs.rs `latest` page, but a renewed upstream lookup produced inconsistent views: the accessible [docs.rs latest manifest](https://docs.rs/crate/ed25519-compact/latest/source/Cargo.toml.orig) identifies itself as 2.4.0 and declares MIT with optional `getrandom ^0.4`; a [docs.rs package-page result](https://docs.rs/crate/ed25519-compact/2.0.6) listed versions through 2.4.2. The exact 2.6.0 docs.rs pages were inaccessible in this session and the [crates.io page](https://crates.io/crates/ed25519-compact) rendered only its JavaScript requirement. These views may be stale; they do not prove that 2.6.0 is unpublished. Treat 2.6.0 as an unverified proposed pin, and do not transfer 2.4.x metadata or source claims to it. The exact 2.6.0 archive, manifest, source, license file, feature graph, target support, and advisory status remain unaudited. The upstream project's claim of formally verified Curve25519 field arithmetic applies to that arithmetic component only; it is not a claim that the whole crate or MirrorMesh pairing flow is formally verified.

`ed25519-dalek 3.0.0` has strict verification APIs, but its [manifest declares BSD-3-Clause](https://docs.rs/crate/ed25519-dalek/latest/source/Cargo.toml.orig), which is not currently in the repository's `deny.toml` license allow-list. It is therefore not selected under current policy without a separate license-policy review.

## Environment evidence

- A Cargo fetch probe for the exact `ed25519-compact = 2.6.0` candidate failed with exit code 101 while trying to reach `https://index.crates.io/config.json` (`Could not connect to server`).
- The isolated probe was retried during the next continuation; it again failed with exit code 101 at `index.crates.io:443` after Cargo's three connection attempts. The registry access blocker is still active.
- A renewed official web lookup did not verify the 2.6.0 version: accessible docs.rs views differed (2.4.0 manifest versus a version list through 2.4.2); the exact 2.6.0 docs.rs page and crates.io version API were not accessible. This is inconclusive evidence, not proof that the version does not exist.
- No Ed25519 provider was added to a workspace manifest or lockfile. No production identity provider is implemented; existing pairing tests use a fake identity port.
- The source could not be audited, so no dependency or security approval is inferred from documentation or the upstream verification claim.

## Next evidence needed

First confirm the exact published version through an authoritative package index or approved local registry/cache. Then fetch its exact archive and transitive graph; inspect source, manifest, license, features, targets, and advisories; verify malformed/weak-key rejection behavior; and only then decide whether to add a locked dependency and test-first provider. Define the local signing/key-lifecycle port and OS-keystore adapter separately. Update notices and rerun dependency security gates after any lockfile change.
