# ADR-0026: Linux TLS identity signer candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed Linux adapter candidate; security and cross-track review pending
- **Scope:** Linux Secret Service identity adapter only

## Context

[ADR-0023](0023-tls13-raw-public-key-pinning-candidate.md) introduces `LocalTlsIdentityPort` for TLS 1.3 server CertificateVerify signing. The Linux identity candidate in [ADR-0011](0011-linux-identity-key-storage.md) already stores the Ed25519 seed used by pairing in the user's Secret Service collection. Native WSS must present the same public key used by the receiver QR fingerprint while keeping pairing-proof signing restricted to its canonical message.

## Decision

Implement `LocalTlsIdentityPort` for the fakeable `IdentityCore<S>` and public `LinuxPairingIdentity`. Return the same Ed25519 public key used by pairing and sign only inputs shaped as the TLS 1.3 server CertificateVerify message: 64 leading `0x20` bytes, the exact `TLS 1.3, server CertificateVerify\0` context, and a 32-byte or 48-byte transcript hash. Reject any other input with `PortError::InvalidConfiguration` before reading the credential. The pairing signer remains restricted to its canonical proof payload.

`LinuxPairingIdentity` acquires the existing per-user `IdentityLock` for TLS key reads and signing, serializing these operations with rotation and deletion. The TLS server configuration captures the public key when built; if the credential rotates before signing, the TLS signature will not match that captured key and the handshake must fail.

## Verification

Fake-store tests cover valid 32-byte and 48-byte transcript hashes, application-shaped messages, wrong context or padding, unsupported hash lengths, current-key signing after rotation, and read/deletion failures. These tests do not connect to D-Bus, access a Secret Service keyring, or run a TLS socket handshake through `LinuxPairingIdentity`.

The adapter uses the existing `secret-service 5.1.0` dependency; no dependency changed. The Secret Service API is blocking, so call it on a dedicated worker rather than an async executor thread. Rust 1.85 and 1.99.1 verification results are recorded in [verification](../verification.md).

## Security limits

The Secret Service seed remains exportable through its API and may be requested by same-user processes; it is not hardware-backed. The implementation has not been tested against GNOME Keyring, KWallet, a live D-Bus session, or a real TLS handshake. No Linux application startup path currently assembles the listener with this identity. Cross-track protocol and security review remain required.
