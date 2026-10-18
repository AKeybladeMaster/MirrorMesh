# TASK 05c — AND: Android phone/tablet ARM64 sender AND receiver (P2-AND)

**Owner:** AND; paths `crates/mm-adapter-android/`, `apps/mirrormesh_flutter/android/` (phone app only), phone package scripts. SDK and UI work can begin immediately; **real WebRTC gate:** CORE contract + ADR-0002.

1. Kotlin MediaProjection flow uses explicit user action and `createScreenCaptureIntent()`, mediaProjection foreground-service type, ongoing notification, and one-time projection semantics on applicable API levels.
2. `createVirtualDisplay` to MediaCodec encoder input Surface. Handle rotation/reconfigure, frame timestamps and bounded buffers. Do not build a hidden always-on capture service.
3. Build receiver `MediaCodec` surface playback + Flutter UI controls and screenshot/overlay integration. Plan native surface lifecycle on activity pause/destroy.
4. Rust Android `aarch64-linux-android` ABI packaging; no raw video through JNI/Flutter events.
5. Native test phone sender `AND→Android TV` and `AND→Windows` plus `WIN→Android phone` receiver; test stop via notification, permission denial/revoke and network handoff.
6. Produce signed test APK/Play AAB (not TV APK), documented manifest permissions and hardware decoder/encoder matrix.

**Acceptance:** physical phone tests, active sharing UX and clean stop, baseline 720p30 fallback, Android ARM64 install. Android TV release is separate 05e.
