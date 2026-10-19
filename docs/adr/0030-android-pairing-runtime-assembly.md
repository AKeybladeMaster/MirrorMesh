# ADR-0030: Android explicit-action pairing runtime assembly

- **Status:** Proposed
- **Date:** 2026-10-19
- **Owners:** AND, CORE, security review pending
- **Related:** [ADR-0027](0027-android-keystore-identity-candidate.md), [ADR-0028](0028-android-ndk-context-bootstrap.md), [ADR-0029](0029-owned-pairing-runtime-state.md), [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562.html)

## Context

The Android identity and the shared pairing runtime must outlive individual listener callbacks, while key creation and trusted-peer storage must not be opened during ordinary app startup. The app needs an explicit user action that assembles local identity and storage without implying that another device has been contacted or trusted.

## Decision

The Android phone app exposes `preparePairingRuntime` through a narrow Kotlin method channel. Flutter presents this action on the Devices page. The call runs on a worker thread and enters the Android-only JNI initializer. Existing startup context initialization remains separate and does not create identity or open the database.

The initializer opens `AndroidPairingIdentity` and `SqliteTrustedPeers` under Android's app-private `noBackupFilesDir`, then retains one `PairingRuntime` in a process-wide `OnceLock<Mutex<Option<_>>>`. Repeated calls return the same local device ID and retain the same runtime. A failed initialization leaves the slot empty so a later explicit retry can run. Identity creation can precede a database-open failure; a retry reuses that identity.

The local device ID is formed from the first 16 bytes of the SHA-256 public-key fingerprint, with UUID version 8 and RFC variant bits set. This leaves 122 fingerprint-derived bits. UUIDv8 is a custom format; uniqueness is implementation-specific and this decision does not claim global uniqueness. The identifier is public metadata, not a secret or an authentication token. Because it is stable for one key, it can correlate the same installation across launches if exposed. Rotating the identity changes the derived device ID; cleanup or migration of existing peer records on identity deletion remains open.

The action opens local identity and trusted-peer state only. It does not start discovery, a listener, network contact, trust approval, capture, or streaming. The UI exposes the ID returned by the native initializer and uses generic errors without returning native storage details.

## Consequences and limits

- Runtime nonce and admission state persist for the Android process lifetime once initialized.
- The method channel does not expose proof, invitation, approval, transport, or capture operations yet.
- `OnceLock` retains the runtime for the process lifetime; ports remain subject to their adapter concurrency contract and app-level serialization.
- No Android target build, APK, Keystore call, or device test is established by host-side tests.
- Cross-track and security review are required before treating this candidate assembly as a stable platform API.

## Verification evidence

Host tests cover stable UUIDv8 derivation and exactly-once runtime construction. Flutter tests cover the method-channel request and missing-ID response. Rust workspace and Flutter checks are recorded in [verification](../verification.md). Android ARM64 compilation is blocked locally because `aarch64-linux-android-clang` / `clang.exe` is unavailable.
