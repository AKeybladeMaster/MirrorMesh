# ADR-0023 — TLS 1.3 raw-public-key pinning candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed CORE security/API candidate; cross-track and security review pending

## Context

The receiver QR fingerprint is SHA-256 over its 32-byte Ed25519 public key. Pairing proof signing is already isolated behind `LocalPairingIdentityPort`, which intentionally signs only pairing-domain messages. A TLS server-authentication signer therefore needs a separate port operation; private key bytes must remain in the platform identity adapter.

The signaling endpoint is specified as `wss://<receiver-host>:8765/v1/control`. The connection must authenticate the receiver before `pair.begin` is sent. A self-signed X.509 certificate would require a separate certificate parser and custom trust path, while the QR already pins the receiver’s Ed25519 identity key.

## Decision

- Add `LocalTlsIdentityPort` with a TLS 1.3 CertificateVerify signing operation and the corresponding public key. Implementations keep private material inside platform adapters. This interface is distinct from `LocalPairingIdentityPort`; it does not widen the pairing proof signer. The adapter checks the exact TLS 1.3 server CertificateVerify context and transcript-hash length before calling the port, preventing an arbitrary-message signing oracle.
- Add `mm-adapter-tls` using Rustls 0.23.46 and its `ring` provider, pinned in `Cargo.lock`. The server presents the same Ed25519 key as an RFC 7250 raw public key (SPKI DER) and delegates the TLS 1.3 signature to `LocalTlsIdentityPort`.
- Native clients request a raw public key, extract the 32-byte Ed25519 key, compare SHA-256 of those raw bytes with the fingerprint from an already validated QR, and verify the TLS 1.3 CertificateVerify using Rustls’ raw-key verification helper. Do not skip CertificateVerify after the fingerprint matches.
- Configure TLS 1.3 only. Disable client session resumption, server session storage, TLS 1.3 tickets, and early data so a resumed session cannot outlive a key rotation or bypass the pin check.
- Do not use the native raw-key verifier in a browser WebSocket client. The browser fallback needs a separately trusted HTTPS/WSS endpoint and normal browser certificate validation.
- Keep QR parsing/expiry validation, socket admission, WebSocket framing, source-address rate limiting, explicit approval, and application startup outside this adapter. No listener is implemented by this decision.

## Consequences

- The TLS identity and pairing identity share one key and one QR fingerprint, avoiding a second public-key field in the invitation.
- The TLS handshake proves possession of the private key while the client’s QR pin authenticates the peer. The server does not use TLS client authentication; the receiver verifies the sender’s signed `pair.begin` at the application layer before approval.
- `CertifiedKey::keys_match()` parses an X.509 certificate and cannot validate the raw SPKI representation. The adapter constructs the advertised SPKI and signer from the same port-provided public key; a successful TLS handshake additionally proves that the signer controls that key. The in-memory handshake tests cover both matching and mismatching signer behavior.
- At this decision point no OS identity adapter implemented `LocalTlsIdentityPort`. Task 20 adds a Windows Credential Manager candidate with fake-store tests; it has not accessed a live credential. Other platform signers and app assembly remain open.
- RFC 7250 support is native-client-only for this design. The TV browser fallback remains on the hosted/publicly trusted WSS path and is not enabled by this adapter.

## Dependency and source review

- Rustls documents TLS 1.3 raw public-key support and the `SigningKey`/`ServerCertVerifier` extension points: [feature list](https://docs.rs/rustls/0.23.46/rustls/manual/_04_features/index.html), [signing-key customization](https://docs.rs/rustls/0.23.46/rustls/manual/_03_howto/index.html), [raw-key verifier](https://docs.rs/rustls/0.23.46/rustls/crypto/fn.verify_tls13_signature_with_raw_key.html).
- The raw-public-key encoding is specified by [RFC 7250](https://www.rfc-editor.org/rfc/rfc7250.html); TLS 1.3 CertificateVerify is defined by [RFC 8446](https://www.rfc-editor.org/rfc/rfc8446.html).
- Rustls 0.23.46 declares `Apache-2.0 OR ISC OR MIT`; the selected `ring` provider (0.17.14) declares `Apache-2.0 AND ISC`; `rustls-pki-types` 1.15.1 declares `MIT OR Apache-2.0`; `rustls-webpki` 0.103.15 and `untrusted` 0.9.0 declare `ISC`; and Windows target support crates declare `MIT OR Apache-2.0`. `ISC` was added to the repository’s existing license allow-list. `cargo metadata --locked --offline` confirmed these package manifests; the `cargo-deny` executable is not installed in this environment, so its license/advisory check did not run.
- Rustls requires Rust 1.71; the workspace minimum is 1.85. GNU/Linux and mobile target compilation remain separate verification items.

## Verification

An in-memory Rustls client/server handshake must succeed with the QR-pinned key and fail for a wrong pin, a malformed public-key encoding, a mismatched signer, or a signing error. This is protocol-level automated evidence only. No TCP socket, WebSocket, OS keystore, browser, app startup, physical device, LAN, or end-to-end pairing flow is covered.
