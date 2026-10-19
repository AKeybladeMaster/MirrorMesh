# TASK 05c — AND: Android phone/tablet ARM64 sender AND receiver (P2-AND)

**Owner:** AND; paths `crates/mm-adapter-android/`, `apps/mirrormesh_flutter/android/` (phone app only), phone package scripts. SDK and UI work can begin immediately; **real WebRTC gate:** CORE contract + ADR-0002.

**Current code checkpoint (Task 25):** `crates/mm-adapter-android/` has a proposed Android Keystore-wrapped Ed25519 identity and TLS signer in [ADR-0027](../docs/adr/0027-android-keystore-identity-candidate.md). Proposed [ADR-0028](../docs/adr/0028-android-ndk-context-bootstrap.md) initializes Android context through JNI. Task 25 adds a user-triggered Flutter method-channel action that opens `AndroidPairingIdentity` and `SqliteTrustedPeers`, retains one process-owned `PairingRuntime`, and returns a stable fingerprint-derived UUIDv8; see proposed [ADR-0030](../docs/adr/0030-android-pairing-runtime-assembly.md). The action opens local state only; invitations, network contact, approval, and capture remain unimplemented. Host Rust/Flutter checks pass. Android `mm-ffi` ARM64 compilation stops in `dart-sys` because `aarch64-linux-android-clang` / `clang.exe` is unavailable. No APK, Keystore, or device scenario ran.

1. Kotlin MediaProjection flow uses explicit user action and `createScreenCaptureIntent()`, mediaProjection foreground-service type, ongoing notification, and one-time projection semantics on applicable API levels.
2. `createVirtualDisplay` to MediaCodec encoder input Surface. Handle rotation/reconfigure, frame timestamps and bounded buffers. Do not build a hidden always-on capture service.
3. Build receiver `MediaCodec` surface playback + Flutter UI controls and screenshot/overlay integration. Plan native surface lifecycle on activity pause/destroy.
4. Rust Android `aarch64-linux-android` ABI packaging; no raw video through JNI/Flutter events.
5. Native test phone sender `AND→Android TV` and `AND→Windows` plus `WIN→Android phone` receiver; test stop via notification, permission denial/revoke and network handoff.
6. Produce signed test APK/Play AAB (not TV APK), documented manifest permissions and hardware decoder/encoder matrix.

**Acceptance:** physical phone tests, active sharing UX and clean stop, baseline 720p30 fallback, Android ARM64 install. Android TV release is separate 05e.
