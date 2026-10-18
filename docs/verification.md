# Verification, performance and acceptance plan

**Status (2026-10-18):** Rust source is formatted, compiled, linted, unit- and integration-tested on Windows with Rust/Cargo 1.99.1 and Visual Studio C++ Build Tools (69 unit tests and 3 FFI integration tests). The Flutter shell passes analysis and its full suite passes all 20 tests with the built Windows FFI DLL configured, including Flutter actions that verify the Rust path `Idle → Discovered → Pairing → Stopped`. The earlier pre-plugin Windows shell build packaged `mm_ffi.dll` from a writable temporary mirror. Current Flutter unit/widget tests ran from the app workspace. The WebRTC plugin build exits before compilation because Windows Developer Mode is disabled and plugin symlinks are required. The demo remains local-only: no peer verification, network use, capture permission, or media stream occurs; screenshot export is disabled. Linux and Android ARM64 library packaging and iOS device static linking with a generated FRB export list are configured, but no platform runner has executed these steps. The iOS simulator bridge remains unavailable. The TV browser TypeScript capability and remote-focus source passes strict type-checking, compiles to JavaScript, and passes seven Node tests from a locked temporary validation mirror. A headless Microsoft Edge 155.0.4283.62 localhost smoke (exit 0) loaded the page, detected a secure origin, WebRTC, and H.264, kept Connect disabled because authenticated signaling is unconfigured, and left autoplay unverified. The screenshot is [the TV web shell preview](assets/tv-web-shell-edge-155.png); no TV browser, signaling, or media test ran. The in-workspace build still cannot write its ignored `dist/` output under this sandbox. Android TV XML/source and local state checks pass, but no Android build ran. No authenticated signaling, production transport, or hardware scenario has been verified. No Flutter full-app screenshot/GIF was captured. **MM-PLAN-001:** parallel four-platform sender/receiver engineering with native Android TV as primary destination and TV browser as verified-only fallback. Expected results below are release gates, not measurements.

## Current CORE kickoff evidence

| Requirement | Source present | Verification status |
|---|---|---|
| M-06 | Session transitions require verified identity, capture permission, and a presented frame; trusted-peer records enforce scoped grants and one-way revocation. | Rust unit tests pass for the domain record, state transition, and fake-backed pairing coordinator. Proposed ADR-0007 documents an Ed25519 encoding candidate; no production cryptographic provider, durable store, local signing/key lifecycle API, or pairing UX exists. |
| S-04 | `mm-security` has a bounded numeric-IP parser, strict QR parser, OS-random single-use nonce registry; `mm-application` coordinates proof validation and explicit approval. | Focused tests cover endpoint/QR parsing, expiry/replay, signed-message domain separation and grant binding, wrong/unknown proof rejection, and no trust persistence before explicit subset approval. Identity tests use fakes. The Ed25519 provider candidate is documented in [ADR-0007](adr/0007-ed25519-identity-candidate.md); its exact source could not be fetched, so no dependency was added. QR UI, production identity, persistent trust, DNS fallback, rate limiting, and signaling remain unimplemented. |
| Trusted-peer record | `mm-domain` defines validated public peer-key/fingerprint/grant records; `mm-ports::TrustedPeerPort` defines get/save/list/revoke operations and `mm-application` calls save only after explicit approval. | Focused tests pass for canonical fingerprint hex, name/key bounds, grant scope, timestamp ordering, revocation, and fake-backed approval persistence. No SQLite implementation or production signature verification is present. |
| M-07 | Domain types, adapter ports, runtime manager, state-only FFI DTOs, and test-only synthetic sequence/timestamp capture fixture exist. | Rust unit tests pass. No raw media frame API crosses the FFI surface. No native adapter exists. |
| M-10 | Bounded strict JSON parser, message-specific validation, UUIDv4 session IDs, per-peer/session sequence tracking, and one golden offer fixture exist. | Rust unit tests pass. A seeded parser fuzz target and scheduled workflow are authored but have not run. Cross-track review and CI are pending; v1 is not frozen. |
| M-09 | Android TV D-pad shell demonstrates a local sender request and explicit Accept/Reject/Disconnect; TV browser shell reports secure-origin, WebRTC, H.264, signaling, and autoplay status with remote-arrow navigation. | Android TV JDK state test and XML/permission/D-pad source checks pass. Headless Edge 155 localhost smoke detects secure origin, WebRTC, and H.264; Connect remains disabled because signaling is absent. Screenshot: [TV web shell preview](assets/tv-web-shell-edge-155.png). No Android APK/install, TV hardware, TV-browser, signaling, or media test ran. |
| GUI shell | Flutter Connect, Session, Devices, Settings pages, generated FRB bindings, normalized annotation overlay, style controls, and session keyboard shortcuts exist. | The full Flutter suite passes 20 tests, including 6 fake-backed engine adapter tests, with the Windows FFI DLL configured; the bridge integration test clicks scan, pairing-start and end, then confirms Rust states `Discovered`, `Pairing`, and `Stopped`. Analysis passes. The UI displays Pairing only; it cannot mark a peer verified or a session streaming. The demo does not use the network, capture, or media. Widget coverage includes the local placeholder/annotations, geometry, overlay golden, styles, responsive navigation, and keyboard shortcuts. See [ADR-0008](adr/0008-flutter-demo-session-ffi.md) and the [mock/domain-state spike](spikes/flutter-mock-domain-state.md). Linux bundle test, Android ARM64 APK packaging, and iOS device static linking are configured but not run on their platform runners. iOS simulator FFI is unavailable. Screenshot export remains disabled without a received frame. |
| CI/tooling | Rust, Flutter cross-platform shell-build, TV browser, Android TV static/state checks and APK build, scheduled Rust dependency advisory/license/source gates, protocol parser fuzzing, Rust and Flutter runtime package notice snapshots/drift checks, and source SBOM generation; root ignore/editor settings; contribution/security templates. | Workflow files are authored and the repository is pushed; hosted Actions results have not yet been checked. |
| Release metadata | Per-platform release manifest, exact SDK versions, staged SBOM and third-party notices, and SHA-256 checksums. | Generator and template are authored; no eligible package or complete package notice/SBOM bundle exists, so release metadata was not produced. |
| P3 integration | Candidate protocol parser/golden fixture, local Android TV approval UI, and TV browser capability shell exist. | **BLOCKED** at Task 07 entry gate: no sender/native Android TV protocol exchange, native transport build, built APK, or physical device run. ADR-0002 selects libwebrtc and the Flutter adapter exists, but it is not connected to authenticated signaling or a media renderer. See [integration readiness](spikes/integration-readiness.md). |
| Configuration | Local preference file store exists with size bounds and temporary-file replacement. | Rust unit tests pass. Preferences only; not a trust store, secret store, or planned SQLite store. |

## Traceability

| Requirement | Automated validation | Manual hardware test |
|---|---|---|
| M-01/M-03 | session state + event bridge tests | Win↔Win and Win→Android TV connect/disconnect |
| M-02/M-06 | handshake, cert fingerprint/expired QR, SRTP negotiating tests | rogue peer, network isolation, firewall |
| S-04 | manual endpoint bounds, literal-IP grammar, approved-port tests | manual entry + pinned peer identity on owned LAN |
| M-04 | saved file checksum + PNG color/orientation snapshot | Windows scale 100/150%, HDR mode detection |
| Surface / snapshot | behavioral proposal only; no `SurfaceHandle`, render port, native surface, or snapshot service is implemented | cross-track lifecycle tests and per-platform raw/composited screenshot evidence; see [surface/snapshot spike](spikes/flutter-surface-snapshot.md) |
| M-05 | normalized transform tests for rotation, fit/crop, letterboxing and resize; overlay golden, per-stroke style, and 2x-DPR responsive navigation widget tests | draw lines during high-motion stream, undo/clear |
| F-07 | widget tests cover per-stroke style, semantic color/width labels, P pen toggle, and Ctrl/Command+Z undo; overlay golden verifies line and tap painting | verify native keyboard behavior and local pen latency on each receiver |
| M-07 | bounded-queue tests + encoder drop policy + sampling spans | 1080p60 jitter/latency device profile |
| M-08 | CI artifact digests/SBOM/code signature | clean install/uninstall + no secret logs |
| M-01 / S-01 | Linux portal adapter mock, feature gate | GNOME/KDE Wayland, X11, permission |
| M-01 / S-02 | Android service lifecycle tests | Android capture/stop, background/rotation |
| S-03 | adaptation tests | stable 60 min run + 720 fallback |
| M-01 / C-01 | Swift SDK availability gate, native receiver tests | actual iOS model/OS/TestFlight; older versions separate |
| M-09 / S-06 | TV native decoder tests + browser codec capability tests | real Android TV TV-native receiver, and named TV browser/HTTPS fallback |

## Actual local verification (2026-10-18)

| Scope | Command | Exit | Result |
|---|---|---:|---|
| Manual endpoint, pairing nonce, and QR parser | `cargo test --locked --offline -p mm-security` | 0 | 17 `mm-security` unit tests passed, including IPv4/IPv6 authority grammar, approved ports, canonical nonce/fingerprint/UUID encoding, single-use and expiry behavior, exact QR fields, version checks, registry capacity/collision bounds, clock overflow, source failure, QR formatting, original nonce deadline retention, and the Windows OS random source. This package does not verify signed identity, display a QR, or connect to a peer. |
| Pairing application coordinator | `cargo test --locked --offline -p mm-application` | 0 | 9 `mm-application` unit tests passed with fake identity/store ports: canonical invitation round-trip, domain-separated proof bytes bound to grants, no persistence before owner approval, grant reduction, signature rejection, replay/unknown nonce rejection, original approval deadline, receiver-bound pending approval, and size/scope bounds. This does not exercise production cryptography, UI, network signaling, rate limiting, or durable storage. |
| Trusted-peer domain record | `cargo test --locked --offline -p mm-domain` | 0 | 14 `mm-domain` unit tests passed, including canonical 32-byte fingerprint encoding, public-key/name/grant bounds, grant scoping, grant subset checks, one-way revocation, and timestamp order. This does not verify cryptographic fingerprints or persistence. |
| SQLite trusted-peer adapter spike | `cargo fetch` with Rust/Cargo 1.99.1 | 1 | `index.crates.io:443` was unreachable; the proposed rusqlite source is not cached. No dependency, lockfile change, adapter, or passing persistence test is claimed. See [spike report](spikes/sqlite-store-dependency.md). |
| Flutter runtime package notices (earlier lockfile) | `python -m py_compile scripts/create_flutter_notices.py`; generator run against the Flutter validation mirror's `.dart_tool/package_config.json` and `package_graph.json` | 0 | At that earlier checkpoint, generated 14 runtime package sections (1,298,582 bytes); repeated generation produced the same SHA-256. The later WebRTC dependency update superseded that snapshot with 45 package notices (see the `flutter pub get` row below). Native SDK/Gradle/Apple/codec notices remain unverified. |
| Android TV CI build manifest helper | Python AST parse and mocked `create_manifest` smoke with exact tool output fixtures | 0 | Two generated manifests matched; captured pinned Gradle/AGP and Android packages, validated CycloneDX JSON, and independently recomputed the APK SHA-256. This does not run the Android toolchain or hosted workflow. |
| Rust third-party notice generation | `cargo-about 0.9.2 generate` with Cargo 1.99.1 and `docs/THIRD-PARTY-NOTICES-rust.hbs` | 0 | Regenerated `docs/THIRD-PARTY-NOTICES-rust.txt` at 147,057 bytes with `getrandom 0.4.3` Apache-2.0 and MIT texts; repeated generation matched SHA-256 `F4D38A12D24D625F1AD1FB4F4D29BC5B90CBE1A156AF62C97B4FD0C3D25EAE51`. cargo-about warned that `allo-isolate 0.1.27` has no manifest license field; its bundled Apache-2.0 text is included and reviewed separately. |
| Protocol fuzz manifest/lock | cargo metadata --locked --offline --manifest-path fuzz/Cargo.toml --format-version 1; rustfmt --check fuzz/fuzz_targets/parse_message.rs; Python tomllib parsed fuzz/Cargo.lock | 0 | Full locked offline metadata resolution, Rust formatting, and lock TOML parsing passed after fetching the fuzz crates into the temporary Cargo cache; 24 lock entries include the fuzz package. The fuzzer itself has not run. |
| Release metadata generator | Python AST parse, JSON template parse, CLI help, temporary artifact smoke fixture, and expected template rejection | 0 | Generated a manifest and three checksums in a temp directory; independently recomputed every hash successfully. Unfilled template was rejected with expected exit 2. Fixture was not a MirrorMesh release package. |
| Rust dependency audit and policy | cargo-audit 0.22.2 `audit --no-fetch --file Cargo.lock` and fuzz lock; cargo-deny 0.20.2 offline root check with `deny.toml` and fuzz check with `fuzz/deny.toml` | 0 for all four checks | Both lockfiles passed with no advisory findings using the cached 1,338-advisory database. Root and fuzz license/source checks passed. Root cargo-deny still reports the upstream `allo-isolate 0.1.27` missing manifest license field; fuzz config has only the NCSA exception needed by `libfuzzer-sys 0.4.13`. The cache was not refreshed from the network; hosted CI remains unrun. |
| Rust formatting | `cargo fmt --all`; `cargo fmt --all -- --check` | 0 / 0 | pass |
| Rust compile | `cargo check --workspace --all-targets --locked` | 0 | pass |
| Rust lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | pass |
| Rust tests | `cargo test --locked --offline --workspace --all-features` (Rust 1.99.1 toolchain binary) | 0 | Fresh run: 69 unit + 3 FFI integration tests passed; doc tests passed. |
| Task 02 transport controls | Included in `cargo test --locked --offline --workspace --all-features` | 0 | All 15 `mm-transport` tests passed for bounded SDP/ICE, lifecycle/reconnect, RTP timestamps, queue recovery and the synthetic I420 source. These tests do not instantiate libwebrtc or encode H.264. |
| Annotation geometry, painter, and style | `flutter test --no-pub --update-goldens test/annotation_overlay_golden_test.dart`; full `flutter test --no-pub` from the writable validation mirror | 0 / 0 | Five mapper tests, painter golden, and style widget test passed. The 128×96 PNG combines a known RGB test frame with a normalized multi-segment stroke and tap dot; SHA-256 `03E442585691D31334843D2F21B21875B07DA46D46ADA93EB639D40BE698A44A`. Full suite: 11 passed, 1 skipped. Synthetic frame only; no native stream/snapshot path is connected. |
| Flutter WebRTC adapter | `flutter test --no-pub test/webrtc_engine_test.dart` | 0 | Six fake-backed tests cover lazy engine creation, metadata-only events, SDP/ICE limits and cleanup after native-close failure. They do not instantiate a native peer connection or encode media. |
| Flutter WebRTC package | `flutter pub get` | 0 | `flutter_webrtc` 1.6.2+hotfix.4 resolved into the lockfile; the generated notice file contains 45 runtime package notices. The first attempt stopped at plugin symlink setup; a later resolution completed. |
| Flutter Windows build with WebRTC plugin | `flutter build windows --release --no-pub` | 1 | Flutter stopped before compilation: plugin builds require Windows Developer Mode for symlink support. No native plugin build evidence. |
| Flutter native session bridge + UI | `flutter test --no-pub` with `MIRRORMESH_FFI_LIBRARY=target/debug/mm_ffi.dll` | 0 | Full suite: 20 passed, including the native bridge and 6 fake-backed WebRTC adapter tests. Scan, pairing-start, and end UI actions resulted in Rust states `Discovered`, `Pairing`, and `Stopped`. The widget suite also covers semantic color/width labels, P to toggle local drawing, Ctrl/Command+Z to undo, and compact/wide navigation at 2x device-pixel ratio. |
| Flutter format | `dart format --output=none --set-exit-if-changed lib test` | 0 | 15 Dart source and test files checked; none changed. |
| Flutter analyzer | `flutter analyze --no-pub` | 0 | Flutter 3.47.7; no issues, including the pinned WebRTC adapter. |
| Flutter Windows desktop build after Step C bridge/UI changes | `flutter build windows --release --no-pub` from a writable workspace-shaped mirror | 0 | Release EXE built and `mm_ffi.dll` staged beside it. MSBuild emitted `MSB8029` warnings because intermediate/output paths were under Temp. The first shallow temp copy failed with MSB3191 because CMake resolved the Rust workspace outside the mirror; the workspace-shaped mirror corrected that path. No live video or hardware test was produced. |
| Rust bridge build | `cargo +1.99.1 build --package mm-ffi --target x86_64-pc-windows-msvc --release` | 0 | `mm_ffi.dll` built |
| Flutter tests | `flutter test --no-pub` from temp source mirror, with `MIRRORMESH_FFI_LIBRARY` set to the packaged DLL | 0 | 5 passed: 4 UI widget tests and 1 native bridge state-view test |
| Flutter Windows build | `flutter build windows --release --no-pub` from a temporary full workspace mirror | 0 | Release EXE and adjacent `mm_ffi.dll` produced; temporary-directory MSBuild warnings only |
| Windows startup smoke | Launch updated packaged EXE hidden for 3 seconds and inspect loaded modules | 0 | Process stayed alive and loaded adjacent `mm_ffi.dll`; no visual inspection |
| Linux bridge and bundle | Linux Cargo build, bridge widget test, `flutter build linux --release --no-pub` | not run | CMake and CI configured for `libmm_ffi.so` in `bundle/lib`; this Windows host cannot execute Linux builds |
| Android Rust/APK package | Android Cargo target build + ARM64 APK + zip entry verification | not run | CI configured for NDK 28.2.13676358 and `lib/arm64-v8a/libmm_ffi.so`; this host has no Android SDK/NDK |
| iOS Rust/device link | Xcode Rust build phase + `flutter build ios --no-codesign --no-pub` | not run | Device ARM64 static link generates a narrow FRB exported-symbols list and preserves global exports; macOS/Xcode is unavailable on this host; simulator FFI remains disabled |
| iOS bridge script/project syntax | `bash -n ios/scripts/build-rust-bridge.sh`; static Xcode project setting check | 0 | Script syntax and balanced project delimiters pass; no Apple linker or Xcode project parser is available locally |
| Flutter Rust Bridge codegen | `flutter_rust_bridge_codegen generate --config-file apps/mirrormesh_flutter/flutter_rust_bridge.yaml` | 0 | Generated bindings match the locked 2.13.0 configuration; the default full-dependency mode does not emit a C header |
| iOS capture API research | Review Apple iOS ScreenCaptureKit sample and API docs | evidence recorded | Current sample says iOS 27+; no SDK compile or device test on this host; see `docs/spikes/ios-capture-spike.md` |
| TV web type check | `npm run typecheck` from `apps/tv-web` | 0 | TypeScript 7.0.2 strict check passes without writing build output |
| TV web locked build/test | `npm ci --offline --ignore-scripts --no-audit --no-fund`; `npm test` in a writable temporary mirror | 0 | TypeScript compiled; 7 Node tests passed, including secure origin, WebRTC/H.264 gates, autoplay messaging, and remote-focus navigation |
| TV web emitted syntax | `node --check dist/app.js`; `node --check dist/capabilities.js` in the temporary mirror | 0 | both compiled modules pass Node syntax checks |
| TV web desktop-browser smoke | Pinned TypeScript build to a writable `%TEMP%\MirrorMeshTVWebHeadless-*` directory; `python -m http.server 48765 --bind 127.0.0.1`; Microsoft Edge `155.0.4283.62 --headless=new --dump-dom --screenshot` at `http://127.0.0.1:48765/` | 0 | Loaded the shell on localhost. Secure origin, WebRTC, and H.264 were available; authenticated signaling stayed unavailable, Connect stayed disabled, and autoplay remained unverified. Screenshot is `docs/assets/tv-web-shell-edge-155.png`. This is desktop-browser UI evidence only; no TV browser, signaling, or media was tested. |
| TV web workspace build | `npm test` from `apps/tv-web` | 1 | TypeScript emitted TS5033 because the sandbox denied creating ignored `dist/`; the same locked build and tests passed from the temporary mirror |
| Android TV static check | `powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\verify-shell.ps1` | 0 | 7 XML resources valid; five D-pad buttons; explicit local approval flow; changing help text is a polite accessibility live region; no permissions or media APIs |
| Android TV local state | `powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\test-demo-state.ps1` | 0 | Java state test covers pending approval, explicit accept/reject, disconnect, repeat request, and idempotent behavior |
| GitHub workflow/issue YAML | Dart 3.13.5 YAML 3.1.3 parser over `.github/workflows/*.yml` and `.github/ISSUE_TEMPLATE/*.yml` | 0 | All 11 workflow and issue-template YAML files parsed in the previous check; the changed dependency-safety workflow was parsed again after its fuzz-config update. |
| Android TV build | `gradle :app:assembleDebug` | not run locally | Gradle and Android SDK are unavailable here. The Android TV workflow builds the debug APK, asks Syft for a package-file CycloneDX preview, records observed runner/SDK versions and artifact hashes, verifies `SHA256SUMS`, and retains the APK/evidence for seven days. Hosted workflow has not run. No APK or emulator test is available as local evidence. |
| Rust dependency metadata | `cargo metadata --locked --offline --format-version 1` | 0 | Ran with the installed Rust 1.99.1 toolchain binaries to review the lockfile's license expressions without network access. The scan found Apache-2.0, MIT, Unicode-3.0, 0BSD, Unlicense, and Zlib expressions; the `allo-isolate` package uses an Apache-2.0 license file. This is metadata review, not an advisory or complete legal audit. |
| Rust dependency gates | cargo-audit 0.22.2 `--no-fetch` on root/fuzz locks; cargo-deny 0.20.2 offline with graph-specific configs | 0 locally | Root audit scanned 98 crates and fuzz audit scanned 24; neither reported advisories in the cached database (1,338 advisories loaded). Root and fuzz advisories/licenses/sources checks passed. Root warns `allo-isolate` lacks a manifest license field; its bundled Apache-2.0 text is present in the regenerated notice. No network refresh was performed during this local run; hosted workflow results are tracked separately below. |
| Source SBOM | Anchore Syft SBOM action | not run | A pinned workflow is configured to create a CycloneDX JSON source inventory from repository lockfiles, print SHA-256, and retain a seven-day review artifact. Its hosted run status has not been reviewed, and it is not a release-package SBOM. |
| Flutter iOS build | `flutter build ios --no-codesign --no-pub` | not run | iOS device static linking and FRB symbol export are configured for the macOS runner; macOS/Xcode is unavailable in this Windows session |
| Flutter full-app screenshot/GIF | Interactive Flutter shell | not captured | No interactive CUA desktop session was available. The separate TV web shell screenshot is recorded above; it is not a Flutter-app or TV-device screenshot. |
| Hosted CI | authored GitHub Actions workflows | pending review | main was pushed to GitHub; workflow results have not yet been checked |

Rust commands ran against the direct Rust 1.99.1 toolchain using a writable temporary `RUSTUP_HOME` because the configured rustup home denied writes. The latest verified Rust run passed 69 unit tests and 3 `mm-ffi` integration tests, including the transport controls and safe demo transition path. Flutter's SDK was copied to a writable temporary directory because its original protected AppData install could not update the SDK lock/telemetry files. Flutter test and Windows build output went to a temporary project mirror because the sandbox denied writes under the app's ignored `build/` directory. The generated Windows executable launched in a hidden three-second smoke check and loaded the adjacent DLL. No Flutter full-app screenshot/GIF was captured; the separate TV web shell screenshot is recorded above.

## Parallel-track release gate matrix (fill from real evidence)

| Track | Sender E2E required | Receiver E2E required | Install/tested artifact | Status |
|---|---|---|---|---|
| Windows x64 | WIN→Android TV + WIN→WIN | WIN→WIN | MSIX/exe on clean system | NOT RUN |
| Linux x64 | LNX→Android TV + LNX→WIN | WIN→LNX | `.deb`/AppImage, GNOME/KDE | NOT RUN |
| Android ARM64 | AND→Android TV + AND→WIN | WIN→Android phone | APK/AAB + permission stop | NOT RUN |
| iOS ARM64 | IOS→Android TV + IOS→WIN **only if supported SDK/system picker** | WIN→iOS | signed IPA/TestFlight + real iPhone | NOT RUN |
| **Android TV primary** | n/a | known sender→native TV APK; remote pairing/stop | TV APK on actual TV | NOT RUN |
| **TV web fallback** | n/a | WIN→named TV browser via HTTPS/WebRTC/H264 | web deploy and TV model/engine | NOT RUN |

The complete pathway-by-pathway Task 07 status is recorded in the [integration readiness spike](spikes/integration-readiness.md); there are no E2E PASS rows or representative hardware screenshots yet.

Status values: `NOT RUN | MOCK ONLY | COMPILES | E2E PASS | FAIL | BLOCKED`. A platform only ships full sender+receiver when both rows are E2E PASS. A receiver-only iOS preview may be released with explicit scope; no unsupported-version streaming claim. The native-TV gate **cannot** be satisfied by browser playback; browser testing cannot certify all TV models.

## Unit tests

1. Codec configuration intersection and H264 profile-level negotiation (including absent codec).
2. Pure session state machine transitions and 50 repeated disconnect/cleanup cycles.
3. `serde_json` envelope parser rejects >256KiB, invalid enum, extra/unexpected fields, invalid UUID, duplicate/reordered sequence, oversized annotations.
4. QR expiry/nonce one-time use, fingerprint mismatch, peer revocation mid-session.
5. SQLite migration rollback when interrupted, FK tests, key store not persisting private key.
6. Annotation hit mapping, rotate/scale/letterbox transform, stroke bounding and undo/clear.
7. Event stream lifecycle on GUI page disposal and blocked native callback; never transfer pixel buffers across FFI.

## Integration tests

- Fake peer sender/receiver over loopback with controlled lag/loss; authenticated test identity and invalid identity.
- mDNS on lab LAN with manual address fallback when multicast is disabled.
- WebRTC SDP/ICE/H264 interoperability against current Chrome and Firefox where supported; test DTLS/SRTP and deliberate codec mismatch.
- Faults: drop Wi-Fi, suspend/restart receiver, revoke peer while sending, deny capture access, close window, rotate Android, overflow frame queue, remove display, change DPI, graphics device loss.
- Protocol fuzzing: mutation-based envelope parser, random data channel messages, ICE candidate bounds, malformed SDP forwarded only into standard vetted stack; no custom SDP parser when avoidable.
- 60-min soak every release; 8h optional nightly; monitor memory, GPU surfaces, file/socket descriptors and total CPU.

## End-to-end hardware fixtures

Record for each test: `date | sender model | GPU/driver | OS | receiver device | AP standard | RSSI | band | hops | codec | target | actual | transport implementation | test ID | log link`.

- Fixture A: Win11 PC (named CPU/GPU) → Win11 laptop, 5GHz Wi-Fi 6 AP or direct Ethernet.
- Fixture B: same sender → Android TV physical appliance, `MediaCodec` receiver.
- Fixture C: Linux Wayland PipeWire → Windows receiver, GPU-specific adapter.
- Fixture D: Android phone hardware H264 encoder → Windows receiver.
- Fixture E: iOS supported device → desktop receiver with system chooser.
- TV browsers sampled across manufacturers; each row certified individually, not “all TVs”.

## Measurement method

Instrument monotonic timestamps per pipeline stage:
`t_capture_acquired`, `t_encode_enqueued`, `t_encoded`, `t_webrtc_sent`, `t_rx_encoded`, `t_decoded`, `t_present_submitted`. Never subtract clocks across devices without clock sync. Use external high-speed camera filming local source interaction and remote rendered image to calculate **interaction→photons**. Capture at least 100 samples per fixture with p50/p95 and confidence/uncertainty. Include effective frame rate and confidence interval.

For annotations, measure local pointer-event-to-overlay-paint latency on the receiver with at least 100 samples at idle and while the stream is under load. Compare the p95 against the ≤1-frame response target; record display refresh, device, viewport size, and stream condition. The coordinate mapper has synthetic geometry tests only; no frame surface or timing measurement exists yet.

Targets (release proposals; adjust after observed baseline):
- no frame transfer over Dart bridge; bounded frame queues max 2 pre-encode and 3 receive decode buffers if codec permits;
- low-latency profile 1080p60 where HW supports; automatic 720p30 under sustained packet loss/thermal pressure;
- p95 interaction-to-photon **≤150ms** on documented compatible local LAN fixture; **not universal guarantee**;
- time-to-first-visible frame **≤3 seconds after user-approved session connect** on test fixture;
- 60-minute session no crash, no monotonic resource growth beyond documented caches;
- unauthorized pairing/capture **0 permitted media frames**;
- stopped/revoked stream ceases within 1s under normal OS scheduling, with immediate encoder cancellation.

Pairing identity benchmark: not run. Once the provider is implemented, record key generation (where the OS key store supports it), proof signing, signature verification, and SHA-256 fingerprint computation separately on representative Windows, Linux, Android, iOS, and Android TV targets. Report warm and cold p50/p95 with CPU, OS, crate/lockfile version, and hardware details. The current provider candidate has no exact fetched source or target build, so no timing or compatibility result is claimed.

## Release verification ledger (Codex populates)

| Build SHA | Test fixture | Command / action | Actual result | Pass? | Evidence |
|---|---|---|---|---|---|
| not built | none | `cargo test --workspace` | not run | unverified | none |
| not built | none | `flutter test` | not run | unverified | none |
| not built | none | Windows native capture sample | not run | unverified | none |
| not built | none | iOS screen permission | not run | unverified | none |

Fail closed on any release without security/capture acceptance and platform-specific install test. Public README only promotes rows from this ledger.
