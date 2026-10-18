# ADR-0009 - Windows local pairing identity storage

- **Decision date:** 2026-10-18
- **Status:** Proposed Windows adapter candidate; security and cross-track review pending

## Context

Task 03 now has a proposed Ed25519 encoding and a stateless verifier in `mm-adapter-identity`. The application still needs a local signing boundary and persistent private-key lifecycle. [ADR-0007](0007-ed25519-identity-candidate.md) requires platform identity adapters to keep private material in operating-system protected storage.

## Proposed decision

- Add `LocalPairingIdentityPort` with operations to return the public key, sign a pairing-domain message, rotate the identity, and delete it. It never returns the private seed.
- Implement the Windows candidate in `crates/mm-adapter-win/`. Store one 32-byte Ed25519 seed as a generic Windows Credential Manager credential named `MirrorMesh/Identity/Ed25519/v1`, using `CRED_PERSIST_LOCAL_MACHINE` for persistence across logons of the same user on this computer.
- Generate new seeds with the existing OS random source. Do not create an in-memory fallback when Credential Manager or the random source fails. Reject malformed stored seed lengths. Keep the seed in zeroizing buffers and rely on the enabled Ed25519 zeroize feature for the live signing key.
- Sign only messages with the exact `MirrorMesh.PairingProof.v1\0` domain prefix. The signer also validates the canonical bytes emitted by CORE PairingProof: the domain prefix, three 16-byte identifiers/nonces, bounded UTF-8 display name, exact message length, and known non-empty grant bits.
- Serialize create, rotate, and delete operations with a named `Local\\` mutex in the current Windows session. The mutex does not coordinate separate logon sessions for the same user; concurrent multi-session use remains an open limitation.
- Rotation changes the public key and fingerprint. Existing peers must pair again. Deleting the local key does not notify peers or revoke their stored trust.

## Security limits

Credential Manager protects the stored blob at rest under the Windows user account, but it does not make this Ed25519 key non-exportable or hardware-backed. Processes running as that same user can read the credential. The adapter keeps no long-lived signer: each operation loads the seed, constructs a short-lived signing key, and wipes managed buffers at scope end. The OS API and process still handle key material, and this cannot guarantee removal of copies retained by the operating system or a compromised process. Do not describe this as TPM-backed protection.

This candidate has not passed security review, cross-track API review, or a real Credential Manager persistence test. Its tests use a fake credential backend. Windows APIs compile locally, but no Windows UI, multi-logon-session behavior, or hardware security module was exercised.

## Dependency and API evidence

- `windows-sys = 0.61.2` is pinned for raw Credential Manager and mutex bindings. The package declares MIT OR Apache-2.0 and Rust 1.71; both licenses are already allowed by repository policy. The workspace MSRV remains Rust 1.85.
- Microsoft documents the generic credential blob and persistence contract in [CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw), with [CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw), [CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew), and [CredDeleteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creddeletew).
- Exact package metadata and feature graph are documented in [windows-sys 0.61.2](https://docs.rs/crate/windows-sys/0.61.2/source/Cargo.toml.orig).

## Consequences and open gates

- The shared signing port is a proposed public contract and requires CORE/cross-track review before other platform teams depend on it.
- Generic coordinator/runtime composition is proposed in [ADR-0010](0010-pairing-runtime-composition.md). Concrete Windows adapter assembly plus Linux Secret Service, Android Keystore, iOS Keychain, pairing UI, authenticated signaling, and wiring the revocation/grant-gated runtime into platform resources remain open.
- The existing verifier profile in ADR-0007 remains proposed. No network pairing, trust UI, real Credential Manager run, or owned-LAN acceptance scenario has passed.
