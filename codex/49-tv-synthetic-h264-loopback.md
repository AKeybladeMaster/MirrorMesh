# Task 49 — Android TV native synthetic H.264 loopback

**Owner:** TV. **Scope:** Android TV instrumentation and emulator build only. This task adds no shared protocol or production signaling contract.

## Patch plan

- Add a test-only `VideoCapturer` that generates bounded, moving I420 color bars directly into the native libwebrtc `VideoSource`.
- Build two in-process libwebrtc peers with H.264-only sender preferences. Deliver SDP/ICE through a fake approved signaling channel to the production `TvWebRtcReceiver`.
- Inject Android's software-only `MediaCodec` decoder factory only through a package-private receiver test seam; the production factory remains hardware-only.
- Skip the H.264 instrumentation case when the pinned libwebrtc lacks a software H.264 encoder or the emulator lacks a software H.264 decoder, instead of reporting a false pass or failure for an unavailable codec.
- Assert ICE connection, a decoded-frame callback, and an explicit JPEG capture from a received frame.
- Build the installable receiver for ARM64 by default and add an x86_64 ABI option for the API 35 emulator instrumentation run.
- Run JDK/Gradle/Android CI on `task_*` pushes and preserve the existing ARM64 debug APK evidence.

## Verification

Planned integration command from `apps/android-tv/`:

```sh
gradle :app:connectedDebugAndroidTest -PmmAndroidAbi=x86_64
```

The workflow creates an API 35 Google APIs x86_64 emulator and invokes this command. It separately builds the default ARM64 APK. Emulator execution would establish `BUILD_VERIFIED` and `AUTOMATED_INTEGRATION_VERIFIED` for this software-only loopback after a successful run; it will not establish physical Android TV or MediaCodec hardware verification.

At source authoring time, this workstation has no Gradle or Android SDK, so the instrumentation code, Gradle ABI option, and workflow have not been compiled or run. No media integration is claimed until CI reports the test passing.

## Dependency review

- Pinned AndroidX Test `runner:1.7.0` and `ext:junit:1.3.0` from the current stable release table. The AndroidX Test source project is Apache-2.0 licensed.
- Existing `io.github.webrtc-sdk:android:150.7871.01` remains unchanged.
- References: [Android instrumented tests](https://developer.android.com/training/testing/instrumented-tests), [AndroidX Test release notes](https://developer.android.com/jetpack/androidx/releases/test), and [AndroidX Test license](https://github.com/android/android-test/blob/main/LICENSE).

## Boundaries

This same-process test uses fake in-process approved signaling, a software H.264 sender encoder when the pinned library provides it, and Android's software-only MediaCodec decoder when the emulator provides it. It skips when either codec is unavailable. It does not verify QR-pinned Rust WSS, a Windows sender, real network ICE, hardware MediaCodec, physical display output, or Android TV hardware. The production receiver still requires hardware H.264 support.
