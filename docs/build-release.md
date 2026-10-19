# Build systems, packaging, GitHub Actions and README expectations

The initial local gates and CI workflow definitions are in place, but hosted workflow runs are still unverified. Keep platform compile checks distinct from real-device, capture-permission, streaming, and release-signing evidence. Android TV debug CI records its observed runner and SDK manifest; every actual release package still needs its own exact runner/tool manifest.

## Folder layout and toolchains

```
Cargo.toml / Cargo.lock / rust-toolchain.toml
apps/mirrormesh_flutter/     # Windows/Linux/Android/iOS Flutter shell
apps/android-tv/             # Kotlin TV receiver, possibly shared Android Flutter module
apps/tv-web/                 # TypeScript browser receiver + web runtime capability tests
crates/mm-{domain,application,ports,protocol,security,store,runtime,ffi}/
crates/mm-adapter-{win,linux,android,ios}/
crates/mm-transport-webrtc/  # selected at ADR-0002
docs/                        # SPECS, ADRs, benchmarks, compatibility
.github/workflows/
scripts/
```

Pin exact tested versions after initial CI: stable Rust toolchain/target triples, Flutter SDK, Dart, Android SDK/NDK, Gradle, Kotlin, Xcode and libwebrtc or GStreamer binary set. Prefer reproducible SDK/container/rust toolchain and locked dependencies. Do not hardcode "latest" in release jobs.

## Windows x64

- Build on `windows-latest` runner with pinned Rust/Flutter and Windows SDK. Target: `x86_64-pc-windows-msvc`.
- The Flutter Windows CMake build compiles `mm-ffi` and installs `mm_ffi.dll` beside the executable. Cargo and the pinned Rust target must be available; the bundled library exposes state-only session control, not pairing or media.
- Native graphics: WGC/Direct3D 11/Media Foundation; test Intel/NVIDIA/AMD.
- Release **MSIX x64** or signed exe installer (e.g., WiX) plus portable zip as secondary. Certificate kept in protected Actions secrets or trusted signing service. Bundle VC runtime/media dependencies properly; test uninstall, startup, firewall and unsigned install warnings.
- Smoke-test clean Windows 10/11 x64 VM and physical GPU host; VM will not substitute GPU latency testing.

## Linux x64

- Target `x86_64-unknown-linux-gnu` with Rust 1.99.1. The Flutter Linux CMake build compiles `mm-ffi` and installs `libmm_ffi.so` in the relocatable bundle's `lib/` directory; the app loads it from there.
- CI builds the Rust library, builds the Linux Flutter release shell, then runs the native state bridge test against the `.so` in the produced `bundle/lib`. This packaging path is configured but still needs a hosted Linux runner result.
- Build on pinned supported Ubuntu runner; create tested `.deb`, AppImage; Flatpak/RPM optional.
- Include/declare precise PipeWire, XDG Desktop Portal, graphics/codec runtime deps; don't assume proprietary codec packages redistributable.
- Smoke test Wayland GNOME/KDE and X11 session; packaged app permission UX, portals, SELinux/AppArmor issues.

## Android

- The Flutter 3.47.7 toolchain pins NDK `28.2.13676358` and Android platform 36. CI cross-compiles `mm-ffi` for `aarch64-linux-android` using the NDK clang linker for API 21, stages `libmm_ffi.so` at `android/app/src/main/jniLibs/arm64-v8a`, builds an ARM64 debug APK, and verifies the packaged library path. `MainActivity` attempts to load the packaged library by soname and, when loaded, passes `applicationContext` to the JNI bootstrap before configuring Flutter. If the library is absent, the existing unavailable-state screen remains usable. This proves packaging configuration only; no phone runtime test has run.
- `flutter build apk --release --target-platform android-arm64`, `flutter build appbundle --release` as baseline only after native integration configured.
- Kotlin MediaProjection + MediaCodec adapters and Rust `aarch64-linux-android` packaged `.so`; proper ABI alignment and foreground service manifest; user-initiated capture only.
- Separate Android TV manifest/launcher icons/remote control focus and TV Play distribution track.
- The Android TV debug workflow records the observed GitHub runner image, Java/Gradle/Android SDK package versions, APK SHA-256, and a CycloneDX preview generated from the APK. It writes and verifies `SHA256SUMS` for the APK, SBOM, and build manifest, then retains the debug APK and evidence for seven days; this is CI review data, not release packaging or device support.
- Signing keystores stored outside repo; test Android OS 14/15/16/17 where hardware available; ensure specific SDK policies verified.

## iOS/iPadOS

- The macOS Xcode device build compiles Rust `aarch64-apple-ios` as a static library, force-loads it into Runner, writes an exported-symbols list from the archive, and retains global symbols for flutter_rust_bridge process lookup. The script verifies required bridge entry points before linking. `flutter build ios --no-codesign --no-pub` is configured in CI, pending a macOS runner result. The simulator build intentionally omits the Rust library and reports it unavailable.
- Swift ScreenCaptureKit and VideoToolbox capture/receive work remains separate from this state-only bridge packaging.
- Apple code signing/provisioning/App Store privacy manifest and relevant ScreenCaptureKit permission strings. `flutter build ipa --release` only after native integration; TestFlight acceptance required.
- macOS build host and paid developer account likely required for broad distribution; jailbreak/homebrew unsupported.

## Required parallel artifact gates

| Pipeline | PR gate | Release / hardware gate |
|---|---|---|
| `windows-x64` | Rust/Flutter Windows compile + fake media + protocol fixtures | WGC/MF stream to Android TV + desktop, MSIX install |
| `linux-x64` | Rust/Flutter Linux build + portal mocks + fixtures | PipeWire Wayland to Android TV + desktop, `.deb`/AppImage clean install |
| `android-phone-arm64` | JNI/Kotlin/Flutter build + projection state tests | real MediaProjection phone→Android TV/desktop, APK install |
| `ios-arm64` | macOS/Xcode compile + Swift availability checks | native iOS receiver test, capture only on supported tested OS, signed IPA/TestFlight |
| `android-tv-arm64` | separate TV Android app build + remote UI tests | real Android TV H264 receiver, D-pad/QR/stop, APK install |
| `tv-web` | JS/TS compile + WebRTC feature and HTTPS tests | actual model/browser H264 WebRTC playback + user approval; fallback only |

Use GitHub Actions **independent jobs** and path filters; split physical-device/manual evidence from compiled CI claims. Failed platform gates do not erase progress on another target but do prevent that platform's supported-release designation.

## CI suggested jobs

- `core`: fmt + clippy + cargo test + cargo deny/audit + license compliance.
- `flutter`: Dart format/analyze/widget tests; platform runner builds gated on platform adapters.
- `protocol-fuzz`: cargo-fuzz on the bounded envelope/annotation message parser (weekly, manual, and protocol-change PR runs); the separate fuzz lockfile is included in advisory, license, and source checks.
- `windows`: build Windows x64, package unsigned test artifact in PR.
- `linux`: build linux x64 and smoke container/GUI test.
- `android-phone`: Gradle/Flutter native build + emulator media projection unit tests where possible.
- `android-tv`: separate TV receiver APK with D-pad/remote widget tests and real TV smoke gate.
- `ios`: macOS build/sign only with SDK and provisioned credentials; independently test receive UI even when capture API gate is blocked; physical hardware tests external.
- `tv-web`: secure-origin JavaScript build, certificate/capability tests, real TV-browser evidence before labeling fallback supported.
- `release`: explicit approval on Git tag, dependency license review, SBOM, sign, hash, notarization/store upload as needed; attach only platform artifacts passing install, privacy and hardware acceptance.

## README template and portfolio hygiene

Top section:
1. Original product icon/app preview image at `docs/assets/app-preview.png` once real app exists; do **not** use Microsoft Store competitor icon.
2. Centered title/one-line description; technology badge row like root README; target OS badges.
3. Honest status banner + feature summary + compatibility table **distinguishing planned vs tested**.
4. 15s demo GIF/video (optimized), hardware/OS specs, short architecture diagram.
5. Install links per OS after artifacts exist; SHA256 checksums.
6. Usage tutorial with user-consent screenshots + troubleshooting.
7. Detailed `docs/` links, developer prerequisites, build/test instructions, security/privacy, contribution/license and acknowledgment.
8. Portfolio case study: challenge, approach, alternatives/ADRs, test fixtures, measured outcome, failures/limitations, 3–5 screenshots and link to source/release.

Badge examples:

```html
<p align="center">
  <img src="https://img.shields.io/badge/Core-Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/UI-Flutter-02569B?style=flat-square&logo=flutter&logoColor=white" alt="Flutter"/>
  <img src="https://img.shields.io/badge/Android-Kotlin-3DDC84?style=flat-square&logo=android&logoColor=white" alt="Kotlin"/>
  <img src="https://img.shields.io/badge/iOS-Swift-F05138?style=flat-square&logo=swift&logoColor=white" alt="Swift"/>
  <img src="https://img.shields.io/badge/Media-WebRTC-333333?style=flat-square&logo=webrtc&logoColor=white" alt="WebRTC"/>
</p>
```

**Caution:** user-provided badges for Java/C#/WebView/PowerShell were examples, not actual implementation choices. Do not claim those technologies in project badges unless used.

## Per-package release metadata

After a platform's build, install, privacy, security, and hardware gates pass, stage its package, package-level CycloneDX or SPDX JSON SBOM, and a complete UTF-8 THIRD-PARTY-NOTICES.txt in one directory. Copy docs/release-manifest.template.json and fill it with the exact source commit, track ID, hosted runner image version, SDK/tool versions, successful build command, and reproducibility status. Remove SDK entries that do not apply to that platform. Do not use latest, guessed values, or a source-only SBOM.

Generate deterministic package checksums and the release manifest with:

    python scripts/create_release_metadata.py --artifacts dist/android-tv --context dist/android-tv-context.json --output dist/android-tv-metadata

The output directory must be outside the staged package directory. The script rejects template values, latest/current labels, missing or empty artifacts, absent SBOM/notices, and unrecognized SBOM JSON format. It lists SHA-256 and byte size for every staged file in release-manifest.json and writes a matching SHA256SUMS. It does not create or validate the legal completeness of notices, sign packages, verify reproducible builds, or establish device support. No release package is currently eligible for this step.

The checked-in Rust and Flutter license-text previews are compared against their locked dependency graphs in CI and retained as review artifacts. The Flutter preview includes 14 runtime packages and the Flutter SDK license text. A release owner must combine these previews with any additional dependencies actually packaged by each platform toolchain and review the complete package notice; neither preview alone is suitable for distribution.
