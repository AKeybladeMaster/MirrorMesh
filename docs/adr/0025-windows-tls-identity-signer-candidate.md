# ADR-0025: Windows TLS identity signer candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed Windows adapter candidate; security and cross-track review pending
- **Scope:** Windows identity adapter only

## Context

[ADR-0023](0023-tls13-raw-public-key-pinning-candidate.md) adds `LocalTlsIdentityPort` for TLS 1.3 server CertificateVerify signing. [ADR-0009](0009-windows-identity-key-storage.md) already stores the pairing Ed25519 seed in Windows Credential Manager and reconstructs a short-lived signer for each operation. Native WSS needs the same public key that the receiver QR advertises, while the existing pairing signer must continue rejecting non-pairing inputs.

## Decision

Implement `LocalTlsIdentityPort` for both the fakeable `IdentityCore<S>` and public `WindowsPairingIdentity`. The TLS methods use the existing Credential Manager seed and return its Ed25519 public key. Each signing request is checked for the TLS 1.3 server CertificateVerify form: 64 leading `0x20` bytes, the exact `TLS 1.3, server CertificateVerify\0` context, and a 32-byte or 48-byte transcript hash. Other messages fail with `PortError::InvalidConfiguration` before reading the credential. The pairing signer remains limited to its canonical proof payload.

`WindowsPairingIdentity` acquires the existing identity mutex for public-key reads and signing, so those operations serialize with rotation and deletion. The TLS adapter captures the public key when building its server config; if the credential rotates before CertificateVerify signing, the new signature will not match the old advertised key and the TLS handshake must fail.

## Verification

Fake-store tests cover valid SHA-256/SHA-384-shaped transcript inputs, application/wrong-context/wrong-length rejection, read/deletion failures, and signature mismatch after key rotation. These tests do not invoke the real Windows Credential Manager, create a live OS credential, or run a TLS socket handshake through `WindowsPairingIdentity`.

No dependency changed. The focused `mm-adapter-win` suite is expected to pass on the Windows host with Rust 1.99.1 and the workspace MSRV Rust 1.85. Full workspace format, strict Clippy, and locked/offline all-feature tests remain required for this task.

## Security limits

The underlying Credential Manager candidate stores an exportable seed that the same-user process reads into memory. ADR-0009 documents that the seed is not hardware-backed or non-exportable. Tests use only a fake credential store, and no Windows app startup or WSS listener uses this implementation yet. Do not describe this candidate as a hardware-backed identity.
