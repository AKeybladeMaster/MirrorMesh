# Spike report - pairing identity provider candidate

**Date:** 2026-10-18  
**Result:** A stateless Ed25519 verifier and a Windows local-signing adapter candidate exist. The encoding, verifier profile, shared signing port, and platform storage behavior remain proposed pending security and cross-track review.

## Question

Which signature representation and local key lifecycle can back the Task 03 signed-proof contract while keeping private material out of SQLite, FFI, and the protocol?

## Proposed identity representation

- Use Ed25519 as specified by [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html): 32-byte raw public keys and 64-byte signatures.
- Compute SHA-256 over the exact raw public-key bytes and represent the fingerprint as 64 lowercase hexadecimal characters.
- Verify the exact domain-separated pairing proof bytes from [ADR-0006](../adr/0006-pairing-coordinator-proof-contract.md).
- The receiver-side verifier in `mm-adapter-identity` checks key length, canonical point encoding, small-order points, signature length, and `verify_strict`. The exact strict acceptance profile remains subject to cross-track review.
- `mm-ports::LocalPairingIdentityPort` provides local public-key, sign, rotate, and delete operations without exporting the private seed.

## Windows local identity candidate

The Windows adapter in `crates/mm-adapter-win/` stores a 32-byte Ed25519 seed in a generic Windows Credential Manager entry scoped to the current Windows user. It creates a new seed only when the credential is missing, writes a replacement seed before returning the new public key; each public-key/sign operation rereads the persisted seed instead of caching a signer, rejects malformed stored values, and does not fall back to memory when storage or random generation fails. See proposed [ADR-0009](../adr/0009-windows-identity-key-storage.md).

Credential Manager protects the blob at rest for that user account, but same-user processes can read it; this is not a non-exportable or hardware-backed key. A named mutex serializes lifecycle operations within the current logon session. Separate concurrent logon sessions are not coordinated.

## Candidate dependency profile

- `ed25519-dalek = 3.0.0`, `curve25519-dalek = 5.0.0`, `sha2 = 0.11.0`, `getrandom = 0.4.3`, and `zeroize = 1.9.1` are exact-pinned.
- `windows-sys = 0.61.2` is the Windows-only raw binding dependency. Its declared MIT OR Apache-2.0 license and Rust 1.71 minimum are compatible with the repository policy and Rust 1.85 MSRV.
- The license allow-list includes every declared license. The dependency lock and generated Rust notices are updated.

## Local evidence

- Ten fake-store tests cover first creation, reopen persistence, signature verification by the real verifier, explicit rotation, deletion, malformed seed data, storage failures, and canonical proof-message validation.
- The focused Windows adapter test suite passes on the Windows host. It compiles the Windows Credential Manager and mutex call sites but uses a fake store and does not write to the user's Credential Manager.
- Workspace formatting, Clippy, full Rust tests, offline `cargo-deny`, deterministic `cargo-about`, and Rust 1.85 checks are recorded in [verification.md](../verification.md).
- The exact Windows API contracts are in Microsoft's [CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw), [CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw), [CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew), and [CredDeleteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creddeletew) documentation.

## Limits and next evidence

The Windows identity adapter is not wired into the pairing coordinator, runtime, or FFI. No real Credential Manager persistence/rotation/deletion, separate-logon-session behavior, Linux Secret Service, Android Keystore, iOS Keychain, QR UI, authenticated TLS/WSS listener, mDNS, rate limiter, revocation closure, or owned-LAN pairing was exercised. The cross-track and security reviews remain pending.
