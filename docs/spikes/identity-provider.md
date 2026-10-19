# Spike report - pairing identity provider candidate

**Date:** 2026-10-18  
**Result:** A stateless Ed25519 verifier and Windows and Linux local-signing adapter candidates exist. The encoding, verifier profile, shared signing port, and platform storage behavior remain proposed pending security and cross-track review.

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

## Linux local identity candidate

The Linux adapter in `crates/mm-adapter-linux/` stores the seed as the secret of a Secret Service item in the user's default collection. Lookup attributes identify only the application and purpose; they contain no key bytes. It uses a DH-encrypted Secret Service session and serializes lifecycle operations with a restrictive lock file under `XDG_RUNTIME_DIR`. Missing runtime-directory configuration, unsafe ownership or permissions, ambiguous items, locked/unavailable storage, authorization cancellation, malformed seeds, and service errors fail closed. Opening the identity may block on the desktop keyring's own authorization prompt, so startup should occur after the user begins pairing.

Secret Service provides storage through the active desktop service and collection policy. The seed remains exportable to authorized same-user callers and is not claimed to be hardware-backed. See proposed [ADR-0011](../adr/0011-linux-identity-key-storage.md).

## Candidate dependency profile

- `ed25519-dalek = 3.0.0`, `curve25519-dalek = 5.0.0`, `sha2 = 0.11.0`, `getrandom = 0.4.3`, and `zeroize = 1.9.1` are exact-pinned.
- `windows-sys = 0.61.2` is the Windows-only raw binding dependency. Its declared MIT OR Apache-2.0 license and Rust 1.71 minimum are compatible with the repository policy and Rust 1.85 MSRV.
- Linux adds `secret-service = 5.1.0` (MIT OR Apache-2.0, Rust 1.77) and `libc = 0.2.177`. Secret Service 5.2.0 requires Rust 1.87, so 5.1.0 is pinned for the repository's Rust 1.85 MSRV. The Linux target graph pins compatible `zbus`, `zvariant`, and `uuid` versions in `Cargo.lock`.
- The refreshed RustSec database reports informational unmaintained notices for `zbus_names 4.3.1`, `zvariant 5.9.2`, and `zvariant_derive 5.9.2`. `cargo audit` reports no vulnerable dependency; these maintenance notices remain open for review and are not suppressed.
- The license allow-list includes every declared license. The dependency lock and generated Rust notices are updated after the Linux addition.

## Local evidence

- Ten fake-store tests cover first creation, reopen persistence, signature verification by the real verifier, explicit rotation, deletion, malformed seed data, storage failures, and canonical proof-message validation.
- The focused Windows adapter test suite passes on the Windows host. It compiles the Windows Credential Manager and mutex call sites but uses a fake store and does not write to the user's Credential Manager.
- Eight fake-store tests cover Linux adapter lifecycle semantics. Rust 1.99.1 and Rust 1.85 compile the `x86_64-unknown-linux-gnu` target; the host test suite uses the fake and never connects to D-Bus or accesses a keyring.
- Workspace formatting, Clippy, full Rust tests, offline `cargo-deny`, deterministic `cargo-about`, and Rust 1.85 checks are recorded in [verification.md](../verification.md).
- The exact Windows API contracts are in Microsoft's [CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw), [CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw), [CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew), and [CredDeleteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creddeletew) documentation.
- Secret Service behavior follows the [freedesktop specification](https://specifications.freedesktop.org/secret-service/latest-single/) and the pinned [`secret-service 5.1.0` API](https://docs.rs/secret-service/5.1.0/secret_service/).

## Limits and next evidence

Neither identity adapter is wired into platform application startup, the Flutter pairing flow, or FFI. No real Credential Manager or Secret Service persistence/rotation/deletion was exercised. Linux unlock/cancel behavior, Android Keystore, iOS Keychain, QR UI, authenticated TLS/WSS listener, mDNS, rate limiter, revocation closure, and owned-LAN pairing remain unverified. The cross-track and security reviews remain pending.
