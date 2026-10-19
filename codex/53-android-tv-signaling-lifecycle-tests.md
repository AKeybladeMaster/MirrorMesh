# Task 53 — Android TV signaling lifecycle integration tests

**Owner:** TV. **Scope:** Android TV instrumentation tests for the existing production Java signaling adapter. No shared protocol or session-engine change.

## Patch plan

- Add AndroidX instrumentation coverage for remote `session.stop`, pairing revocation, and malformed control-message failure through `RustApprovedSignalingChannel`.
- Assert each terminal event notifies its listener, closes the native control handle once, and leaves the adapter no longer approved.
- Keep a test-only bridge double; do not claim Rust WSS authentication or sender interoperability from these tests.
- Extend the Android TV static source check to keep the lifecycle cases present.

## Verification

From `apps/android-tv/`:

```sh
gradle :app:connectedDebugAndroidTest -PmmAndroidAbi=x86_64
gradle :app:assembleDebug
```

Also run `cargo fmt --all -- --check`, `git diff --check`, and the direct JDK TV state/policy/screenshot tests from the repository root. The Android workflow runs all instrumentation tests on API 35 x86_64 and builds the ARM64 debug APK.

## Evidence and limits

Passing instrumentation will verify Java adapter event delivery and cleanup with an injected native bridge. It will not verify the real Rust JNI/WSS transport, the Windows sender, hardware MediaCodec, or physical Android TV behavior. Keep `BUILD_VERIFIED`, `AUTOMATED_INTEGRATION_VERIFIED`, and `HARDWARE_VERIFIED` distinct.
