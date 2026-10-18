# Research sources and version check ledger

Reviewed online **2026-10-18**. Websites and API availability evolve; Codex must recheck the specific target at implementation time and pin dependency lockfiles. The reference Microsoft Store's web listing is sparse and does **not** disclose internal source, listener ports, protocols, codecs or full features.

## Reference and agent tools

- AirMirroring Microsoft Store product: https://apps.microsoft.com/detail/9P2K9G20SWZ6?hl=en&gl=IT&ocid=pdpshare — listing describes receive iPhone/iPad/Mac → Windows; no internal protocol evidence.
- REA official: https://github.com/morluto/rea — `npx rea-agents setup`; tooling/provider OS caveats.
- REA skills.sh: https://skills.sh/morluto/rea/reverse-engineer-anything — skill alone does not set up MCP.
- skills.sh directory: https://skills.sh/ — listing and agent support.
- skills.sh CLI: https://skills.sh/docs/cli — installation syntax.

## Official capture/media/network APIs

- Windows.Graphics.Capture: https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture
- DXGI desktop duplication alternative: https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- Microsoft MSIX protected install semantics: https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes
- Android MediaProjection: https://developer.android.com/media/grow/media-projection — user consent and foreground service.
- Android foreground service types: https://developer.android.com/develop/background-work/services/fgs/service-types
- Apple ScreenCaptureKit: https://developer.apple.com/documentation/screencapturekit — Apple now positions it as ReplayKit replacement for streaming/mirroring.
- Apple iOS ScreenCaptureKit sample: https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios — sample states iOS 27+.
- Apple ReplayKit legacy picker: https://developer.apple.com/documentation/replaykit/rpsystembroadcastpickerview — presently marked deprecated.
- GStreamer WebRTCbin: https://gstreamer.freedesktop.org/documentation/webrtc/
- Rust GStreamer bindings: https://docs.rs/gstreamer/latest/gstreamer/ — showed `gstreamer 0.25.4`; do not infer bundled runtime version.
- Rust str0m: https://github.com/algesten/str0m — sans-I/O WebRTC; implement IO/polling externally.
- Flutter deployment matrix: https://docs.flutter.dev/reference/supported-platforms
- TypeScript compiler install guidance: https://www.typescriptlang.org/download/ â€” supports a project-local compiler and lockfile.
- TypeScript v7.0.2 package: https://www.npmjs.com/package/typescript â€” pinned as a WEB build-only dependency; lockfile records Apache-2.0 and integrity hashes.
- TypeScript source license: https://github.com/microsoft/TypeScript/blob/main/LICENSE.txt â€” Apache-2.0; no browser runtime package added.
- flutter_rust_bridge: https://github.com/fzyzcjy/flutter_rust_bridge ; releases: https://github.com/fzyzcjy/flutter_rust_bridge/releases — stable `v2.13.0` listed, beta later. Do not adopt beta automatically.
- Android TV native app design + D-pad: https://developer.android.com/media/implement/surfaces/android-tv
- Google Cast Web Receiver: https://developers.google.com/cast/docs/web_receiver — default vs registered custom receivers.
- Cast receiver registration: https://developers.google.com/cast/docs/registration
- Cast Web Sender requirements: https://developers.google.com/cast/docs/web_sender
- Google Cast integration overview: https://developers.google.com/cast/docs/get-started

## Pairing identity provider research

- [RFC 8032 — EdDSA](https://www.rfc-editor.org/rfc/rfc8032.html) — Ed25519 key/signature specification; candidate encoding is proposed in [ADR-0007](adr/0007-ed25519-identity-candidate.md).
- [ed25519-compact docs.rs latest manifest](https://docs.rs/crate/ed25519-compact/latest/source/Cargo.toml.orig) and [crates.io package page](https://crates.io/crates/ed25519-compact) — the exact proposed 2.6.0 version could not be confirmed in this session; accessible docs.rs views were inconsistent and Cargo could not reach the registry. See [identity-provider spike](spikes/identity-provider.md).
- [ed25519-dalek 3.0.0 manifest](https://docs.rs/crate/ed25519-dalek/latest/source/Cargo.toml.orig) — declares BSD-3-Clause, outside the current allow-list.

## Similar established projects (study behavior/performance/licensing; do not copy unreviewed code)

- scrcpy (Android over USB/TCP; low-latency): https://github.com/Genymobile/scrcpy
- Sunshine (hardware-accelerated host, Moonlight clients): https://github.com/LizardByte/Sunshine
- Deskreen (browser-based secondary display): https://github.com/pavlobu/deskreen
- UxPlay (Unix AirPlay receiver): https://github.com/FDH2/UxPlay

## Dependency/version lock checklist for Codex

| Component | Current online indication | Required implementation action |
|---|---|---|
| Rust | toolchain version not pinned here | choose supported stable, commit `rust-toolchain.toml` + Cargo.lock |
| Flutter | latest supported matrix confirmed via docs | pin SDK channel/version in CI |
| `flutter_rust_bridge` | v2.13.0 stable seen in releases | pin stable, test Android/iOS target compatibility and native texture integration |
| TypeScript compiler | v7.0.2 package pinned; Apache-2.0 | keep as WEB-only build dependency; compile and test before publishing static assets |
| `gstreamer` Rust crate | docs.rs displayed 0.25.4 | only use if runtime binary/codec plugin license & package audit passes |
| `str0m` | official sans-I/O API confirmed | choose pinned crate after H264/ICE/bundle spike |
| WebRTC/native libs | candidates, no chosen revision | ADR-0002 benchmark + lock source digest |
| Android SDK | moving API | target SDK/install policy validation via official current Android docs |
| Xcode/iOS | 27+ sample available | conditional availability/legacy path on actual devices |

Never generate a precise API call from stale memory when official docs/source code show a new API; verify signature in pinned released dependency.
