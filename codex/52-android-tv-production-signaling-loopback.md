# Task 52 — Android TV production signaling adapter loopback

**Owner:** TV. **Scope:** Android TV Java signaling adapter and emulator instrumentation. No shared protocol, Rust FFI contract, or production session engine changes.

## Patch plan

- Inject a package-private bridge around the existing Rust JNI calls used by `RustApprovedSignalingChannel`; the normal constructor continues to use `NativePairingBridge`.
- Replace the test-only `ApprovedSignalingChannel` implementation with a test-only native-bridge double that exchanges offer, answer, and ICE JSON through the production Java adapter.
- Keep the real libwebrtc H.264 sender, synthetic I420 source, production `TvWebRtcReceiver`, software-decoder test seam, and explicit screenshot assertion.
- Update the Android TV source check so it requires this production-adapter loopback.

## Verification

From `apps/android-tv/`:

```sh
gradle :app:connectedDebugAndroidTest -PmmAndroidAbi=x86_64
gradle :app:assembleDebug
```

Also run `cargo fmt --all -- --check` and `git diff --check` from the repository root. The Android workflow runs the instrumentation command on an API 35 x86_64 emulator and builds the ARM64 debug APK.

## Evidence and limits

The instrumentation source now exercises offer/ICE mapping through `RustApprovedSignalingChannel` and asserts real libwebrtc connection, an H.264 decoded frame, and an explicit JPEG capture. The injected control bridge is a test double: it does not run Rust WSS authentication, Windows interoperability, the Android APK build, hardware MediaCodec, or a physical TV. Keep `BUILD_VERIFIED`, `AUTOMATED_INTEGRATION_VERIFIED`, and `HARDWARE_VERIFIED` pending until their respective evidence exists.
