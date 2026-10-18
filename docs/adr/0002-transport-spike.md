# ADR-0002 — WebRTC transport and media engine

- **Status:** Selected for implementation; build, interoperability and hardware validation pending
- **Decision date:** 2026-10-18
- **Owners:** CORE + platform adapters
- **Requirements:** M-02, M-06, M-07, M-09, S-03, S-06

## Context

MirrorMesh needs browser-interoperable WebRTC with H.264, ICE, DTLS-SRTP, platform capture/codec support, and native rendering. Capture and presentation stay in OS adapters. Video pixels must not cross Dart FFI. Android TV remains the primary television receiver; the TV browser remains a separate fallback.

The previous transport spike stopped before a candidate build because its original environment audit found no media/Android SDKs or TV fixture. The user's code-first direction now defers those runtime checks rather than blocking independent source implementation. No product WebRTC engine has yet been linked or run.

## Decision

Use the upstream `flutter_webrtc` package as the initial libwebrtc integration for the Windows, Linux, Android phone and iOS Flutter applications. Pin `1.6.2+hotfix.4` when adding it to the Flutter package graph. This release is active and its published feature matrix covers media and WebRTC APIs on those target families; per-OS capture and codec availability still requires capability checks.

Keep the Android TV application native Kotlin. Its adapter will use the same libwebrtc API family directly, with `io.github.webrtc-sdk:android:150.7871.01` as the current artifact candidate declared by the Flutter plugin's Android build. Confirm that exact artifact against the selected package tag, checksum, notices and Android SDK before pinning it in the TV Gradle build.

The browser fallback uses the browser's native `RTCPeerConnection`, ICE and media rendering APIs. It shares the app-owned signaling messages with native clients and must run on a secure origin. Browser compatibility remains device-specific.

Rust remains responsible for session/control policy, bounded protocol data, timestamps, reconnect policy and platform ports. Native WebRTC engines own ICE sockets, DTLS-SRTP, codec execution and media surfaces. `WebRtcEnginePort` carries SDP/candidate/track metadata only; raw video pixels do not cross Dart FFI. Task 03 owns authenticated signaling and peer identity, which must be complete before an engine can stream to an unknown peer.

## Rationale and trade-offs

- `flutter_webrtc` provides one maintained integration surface across the four Flutter targets and wraps libwebrtc rather than introducing a new RTP/DTLS stack. Its latest published version is `1.6.2+hotfix.4` (2026-10-07). The project also documents a WebRTC Android library dependency and native platform code.
- A direct native WebRTC adapter keeps the Android TV UI Kotlin-based and allows MediaCodec/Surface rendering without an embedded browser.
- Browser-native WebRTC avoids bundling a second media engine into the web fallback.
- `str0m` remains a viable Rust-native alternative, but its Sans-I/O design leaves network I/O/event driving and codec/capture integration to MirrorMesh. Its upstream table marks Android and iOS as compiled but not tested, so it adds platform integration work before it solves a requirement.
- GStreamer `webrtcbin` remains technically viable but creates a larger per-platform plugin, binary and license review across desktop and mobile. The current Windows workspace has no local GStreamer pipeline to reuse.

This is an implementation choice based on the target matrix and current upstream support, not a performance or interoperability conclusion. No 720p/1080p, H.264 profile, encrypted packet, reconnect, package-size or latency result is claimed.

## Implementation evidence and limits

`crates/mm-transport/` now provides tested control-state logic, bounded trickle ICE buffering, retry deadlines, RTP timestamp conversion, a bounded encoded-frame queue, and a timestamped synthetic I420 source for a native encoder. `crates/mm-ports/` exposes an async engine boundary with control and track metadata events only.

The Flutter package is not yet in `pubspec.yaml`/`pubspec.lock` and is not present in the local pub cache. The existing Windows Flutter build predates this transport code. Android SDK/Gradle and the physical Android TV run are unavailable in this workspace. Therefore, the selected library has **not** been compiled or integrated here; its public support matrix is not local build or device evidence.

## Security and compatibility constraints

1. Use only verified libwebrtc DTLS-SRTP; do not add custom RTP cryptography or accept-all certificate behavior.
2. Connect only after Task 03 authenticates the peer and the receiver explicitly approves the session. Capture still requires OS consent.
3. Keep signaling on authenticated TLS/WSS and validate every SDP/candidate before passing it to the engine. Bound candidates and close resources on Stop, revoke, timeout and shutdown.
4. Request H.264 capabilities from the actual codec engine/device. Disable unsupported resolutions or receivers truthfully; do not assume all TV SoCs support an identical profile.
5. Pin package/artifact versions and refresh Flutter/native third-party notices before release packaging.

## Follow-up validation

1. Add and lock `flutter_webrtc` and verify Windows compilation with local source/package access; then add Linux, Android and iOS runner builds.
2. Implement Task 03 authenticated signaling, and connect the browser and native clients to the same protocol fixture.
3. Add the Kotlin Android TV peer connection, explicit approval, MediaCodec decoder and native surface renderer.
4. Generate synthetic video through a real encoder; verify selected H.264 profile/packetization, ICE candidate completion, DTLS-SRTP, disconnect and reconnect.
5. Run 720p30 and 1080p60 on named receivers, a 15-minute soak and 20 reconnects; record capture/render timings and external-camera latency when hardware is available.

Physical TV/browser measurements are deferred. Software integration, local compilation and automated mocks continue independently.

## Sources

- [Flutter WebRTC package versions](https://pub.dev/packages/flutter_webrtc/versions)
- [Flutter WebRTC feature matrix and setup](https://github.com/flutter-webrtc/flutter-webrtc)
- [Flutter WebRTC 1.6.2+hotfix.4 changelog](https://github.com/flutter-webrtc/flutter-webrtc/blob/main/CHANGELOG.md)
- [Flutter WebRTC Android Gradle dependency](https://github.com/flutter-webrtc/flutter-webrtc/blob/main/android/build.gradle)
- [WebRTC native development guide](https://webrtc.googlesource.com/src/+/main/docs/native-code/development/)
- [`str0m` design and platform notes](https://github.com/algesten/str0m)
- [GStreamer WebRTC documentation](https://gstreamer.freedesktop.org/documentation/webrtc/)
- [Android MediaCodec capabilities](https://developer.android.com/media/optimize/performance/codec)
