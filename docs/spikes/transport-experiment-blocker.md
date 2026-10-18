# Task 02 transport implementation and deferred verification

**Status (2026-10-18):** Software work is active. Hardware, authenticated-peer and media interoperability checks are deferred; they are not a gate for independent code. The initial environment report below is retained as historical evidence. ADR-0002 now selects a libwebrtc integration path for implementation.

## Current software implementation

`crates/mm-transport/` contains a transport session state model, bounded SDP and trickle ICE staging, candidate arrival ordering, reconnect/restart transitions, a 90 kHz monotonic timestamp clock, a bounded encoded-frame queue that requests a keyframe after overflow, and a deterministic timestamped I420 color-bar generator. `crates/mm-ports/` defines an asynchronous engine port that emits SDP/ICE/connection/track metadata; it has no video-frame event.

These are compiled software controls, not a WebRTC engine. The I420 source is raw test input for a future encoder adapter and is not H.264. The encoded queue tests use synthetic bytes and do not establish valid H.264 access units. No signaling socket, DTLS-SRTP handshake, peer authentication, media decode/render, or latency measurement is implemented yet.

## Current environment recheck

- Rust/Cargo 1.99.1 is available at `C:\Users\Beasty\.rustup\toolchains\1.99.1-x86_64-pc-windows-msvc\bin`; it is not on the shell's default `PATH`.
- Flutter is configured at `C:\Users\Beasty\AppData\Local\Programs\MirrorMeshFlutter\flutter`; `flutter_webrtc` is not in the local pub cache. The existing Flutter Windows build predates this dependency decision.
- The user-provided ADB directory exists at `C:\Program Files\adb-tools` and contains `adb.exe`; it is not on `PATH`. Android SDK and Gradle executables were not found through the current shell/environment probe.
- `ffmpeg`, `ffprobe`, GStreamer, and Linux SDK tools are not available through the current shell probe. The browser evidence is a desktop Edge 155 localhost capability-shell screenshot, not an authenticated peer or TV browser test.
- The GitHub repository URL was queried read-only with permission after the sandbox denied network access; `git ls-remote` returned no refs. This is consistent with a new empty remote. The local workspace has no `.git` metadata.

These checks record this Windows shell, not the user's entire machine inventory. User-deferred hardware tests may be run later from the provided ADB path; they are not required for code work.

## Candidate choice

ADR-0002 selects the published `flutter_webrtc` package as the initial libwebrtc integration for Flutter targets, a native Kotlin libwebrtc adapter for Android TV, and browser-native WebRTC APIs for the web fallback. The Rust transport layer owns bounded control state and media metadata, not the codec or socket implementation. The package and Android artifact still need exact local dependency resolution, lockfile updates, notice review and target builds.

The choice uses the current published target matrix and the requirement for native media surfaces across Windows, Linux, Android and iOS. `str0m` remains viable but requires MirrorMesh to provide I/O/event driving and separate codec adapters; upstream lists Android/iOS compiled but not tested. GStreamer remains viable but has extra plugin/bundle and license review costs across mobile and desktop. This decision is an implementation direction, not measured interoperability evidence.

## Deferred verification work

1. Add the pinned Flutter package and confirm its exact Android/WebRTC artifact revisions; update the Flutter lockfile and platform notices.
2. Implement the Flutter Windows engine adapter and prove offer/answer plus trickle ICE against a local synthetic peer.
3. Complete Task 03 authenticated TLS/WSS signaling and peer verification before any unknown peer can stream.
4. Add the Kotlin Android TV engine, explicit approval, MediaCodec decoder, surface renderer and cleanup path.
5. Use a real encoder to generate H.264 from the I420 test source; verify negotiated profile/packetization, secure DTLS-SRTP media, stop and reconnect.
6. Run target-specific 720p30/1080p60, 15-minute soak, 20 reconnect and latency checks when named devices are available. Do not report hardware support before those results exist.

## Historical environment/API findings

The initial 2026-10-18 audit found no local `str0m` crate, GStreamer/FFmpeg tools, Android SDK/NDK or TV hardware; crates.io was unreachable in the default shell. Edge 155 was later used for a desktop capability-shell smoke only. These constraints blocked candidate media experiments at the time, not the current implementation work.

Official references used for the decision: [`flutter_webrtc` package versions](https://pub.dev/packages/flutter_webrtc/versions), [Flutter WebRTC feature matrix](https://github.com/flutter-webrtc/flutter-webrtc), [Flutter WebRTC Android dependency](https://github.com/flutter-webrtc/flutter-webrtc/blob/main/android/build.gradle), [`str0m` design and platform notes](https://github.com/algesten/str0m), [GStreamer WebRTC documentation](https://gstreamer.freedesktop.org/documentation/webrtc/), [native WebRTC development](https://webrtc.googlesource.com/src/+/main/docs/native-code/development/), and [Android MediaCodec capability APIs](https://developer.android.com/media/optimize/performance/codec).
