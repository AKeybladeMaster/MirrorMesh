# Verification, performance and acceptance plan

**Status (2026-10-19):** Rust source is formatted, compiled, linted, and tested on Windows with Rust/Cargo 1.99.1 and Visual Studio C++ Build Tools (153 unit tests, 27 integration tests, and doc tests pass). The total includes 3 FFI, 6 pairing-runtime, 1 trusted-session revocation, 6 identity, 10 SQLite, 7 TLS adapter, and 5 signaling-listener unit tests; integration coverage includes the two-process signaling exchange. Unit coverage also includes 13 protocol tests, 15 application tests, and 17 pairing/session-runtime tests. The focused protocol/runtime tests and the five signaling tests plus two-process exchange pass on Rust 1.85. Tasks 15–19 connect strict inbound `pair.begin` parsing, shared outbound encoding, sender proof creation, native TLS 1.3 raw-public-key pinning, and a bounded native WSS listener candidate; no Flutter files changed. Flutter analysis and its full 20-test suite have prior local evidence. The WebRTC plugin build exits before compilation because Windows Developer Mode is disabled and plugin symlinks are required. The Flutter and TV shells remain local demos: no peer verification, capture permission, or media stream occurs. The TV browser TypeScript source passes strict checks and seven Node tests; its Edge 155 localhost smoke kept Connect disabled because app signaling is unconfigured. Android TV source/state checks pass, but no Android build ran. The listener has only local loopback evidence; no LAN, production app integration, browser, platform, or hardware scenario has been verified. The Linux identity candidate uses fake-store tests only; it did not access D-Bus or a real keyring. **MM-PLAN-001:** parallel four-platform sender/receiver engineering with native Android TV as primary destination and TV browser as verified-only fallback. Expected results below are release gates, not measurements.

## Current CORE kickoff evidence

| Requirement | Source present | Verification status |
|---|---|---|
| M-06 | Session transitions require verified identity, capture permission, and a presented frame; trusted-peer records enforce scoped grants and one-way revocation. | Rust tests pass for the domain record, state transition, fake-backed pairing coordinator, six-test Ed25519 verifier candidate, ten-test SQLite adapter, and eleven-test fake-backed `PairingRuntime`. Six integration tests combine the real verifier and SQLite store, covering proof creation, outbound encoding, strict raw `pair.begin` frames, approval-only persistence, tampered signatures, and malformed-frame source admission. Windows and Linux identity lifecycle tests use fake stores; Linux GNU target compilation passes on Rust 1.99.1 and 1.85. The mDNS candidate list remains unauthenticated and is not assembled into startup. Proposed [ADR-0020](adr/0020-pair-begin-wire-encoding-candidate.md), [ADR-0021](adr/0021-pair-begin-outbound-encoding-candidate.md), and [ADR-0022](adr/0022-sender-proof-creation-candidate.md) record these candidate APIs; cross-track/security review remains pending. Live keyring access, listener integration, and pairing UX remain open. |
| S-04 | `mm-security` has a bounded numeric-IP parser, strict QR parser, OS-random single-use nonce registry; `mm-application` coordinates proof validation and explicit approval. | Focused tests cover endpoint/QR parsing, expiry/replay, signed-message domain separation and grant binding, wrong/unknown proof rejection, local proof creation through the identity port, and no trust persistence before explicit subset approval. The separate `mm-adapter-identity` candidate passes RFC 8032 and negative-input tests; Windows and Linux identity adapters pass fake-store lifecycle tests. Six `mm-runtime` integration tests exercise real Ed25519 verification and SQLite persistence/rejection through proof creation, outbound encoding, and raw-frame paths. Linux Secret Service is not assembled into platform startup and has not accessed D-Bus. QR UI, Android/iOS key stores, DNS fallback, authenticated listener, and signaling remain open. The in-memory admission candidate is composed in the runtime; its policy and wire schema remain pending review under ADR-0015/0016/0020/0021/0022. |
| Trusted-peer record | `mm-domain` defines validated public peer-key/fingerprint/grant records; `mm-ports::TrustedPeerPort` defines get/save/list/revoke operations and `mm-application` calls save only after explicit approval. | Focused tests pass for canonical fingerprint hex, name/key bounds, grant scope, timestamp ordering, revocation, and fake-backed approval persistence. The SQLite adapter separately passes ten persistence/migration tests; `PairingRuntime` composes the trusted-peer port, but SQLite is not yet assembled into platform app startup. |
| M-07 | Domain types, adapter ports, runtime manager, state-only FFI DTOs, and test-only synthetic sequence/timestamp capture fixture exist. | Rust unit tests pass. No raw media frame API crosses the FFI surface. No native adapter exists. |
| M-10 | Bounded strict JSON parser, message-specific validation, UUIDv4 session IDs, per-peer/session sequence tracking, and golden offer/pairing fixtures exist. | Thirteen protocol tests validate strict inbound fields, canonical encodings, and outbound `pair.begin` encoding; runtime/SQLite tests cover the signed frame path. The seeded parser fuzz target and scheduled workflow are authored but have not run. Cross-track review and CI are pending; v1 is not frozen. |
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
| MirrorMesh DNS-SD discovery and publisher record | `cargo test --locked --offline -p mm-adapter-discovery` on Rust 1.99.1; focused package tests on Rust 1.85; GNU/Linux cross-target checks on both | 0 / 0 | Twenty-one tests cover v1 parsing, outgoing record construction, and endpoint conversion, including DNS case-insensitivity, protocol markers, identity/role/capability bounds, RFC1918/link-local IPv4 and ULA/link-local IPv6 publication filters, invalid host/name data, address caps, inactive-stop idempotence, and conversion from `mdns-sd::ResolvedService`. Tests do not start a daemon or open sockets. The endpoint-preserving contract and publisher candidate are not wired into app startup; no LAN multicast or publication was verified. |
| Manual endpoint, pairing nonce, QR parser, and attempt limiter | `cargo test --locked --offline -p mm-security` | 0 | 25 `mm-security` unit tests passed, including eight pairing-attempt admission tests plus IPv4/IPv6 authority grammar, approved ports, canonical nonce/fingerprint/UUID encoding, single-use and expiry behavior, exact QR fields, version checks, registry capacity/collision bounds, clock overflow, source failure, QR formatting, original nonce deadline retention, and the Windows OS random source. This package does not verify signed identity, display a QR, or connect to a peer. |
| Pairing and discovery application coordinators | `cargo test --locked --offline -p mm-application` | 0 | 12 `mm-application` unit tests passed: nine pairing tests use fake identity/store ports and cover canonical invitation round-trip, domain-separated proof bytes bound to grants, no persistence before owner approval, grant reduction, signature rejection, replay/unknown nonce rejection, original approval deadline, receiver-bound pending approval, and size/scope bounds. Three discovery tests cover local-device filtering, stable ordering, endpointless and ambiguous duplicate claims, stop, and adapter-error propagation. The coordinators are not wired into production network signaling, UI, or durable platform storage; the limiter is not integrated into a signaling listener. |
| Pairing runtime with real verifier and SQLite | Rust 1.99.1 and Rust 1.85 `cargo test --locked --offline -p mm-runtime --test pairing_runtime_sqlite` (Rust 1.85 uses an isolated `CARGO_TARGET_DIR`) | 0 / 0 | Two integration tests pass on both toolchains using deterministic test-only Ed25519 keys and `SqliteTrustedPeers::open_in_memory()`. A valid signed proof is verified, remains untrusted before user approval, and is stored only with an approved grant subset; a modified signature is rejected without persistence. No OS key store, network, or UI is exercised. |
| Ed25519 identity verifier | `cargo test --locked --offline -p mm-adapter-identity --test verification` | 0 | Six tests passed: RFC 8032 valid proof and known fingerprint, message/signature rejection, malformed lengths, non-canonical public key, and low-order key rejection. |
| Trusted-peer domain record | `cargo test --locked --offline -p mm-domain` | 0 | 14 `mm-domain` unit tests passed, including canonical 32-byte fingerprint encoding, public-key/name/grant bounds, grant scoping, grant subset checks, one-way revocation, and timestamp order. This does not verify cryptographic fingerprints or persistence. |
| SQLite trusted-peer adapter | `cargo test --locked --offline -p mm-store --test sqlite_trusted_peers`; Rust 1.85 `cargo check --locked --offline -p mm-store --all-targets` | 0 | Ten integration tests pass on Rust 1.99.1 and 1.85. The minimum-version check includes the bundled Windows SQLite C build. Concrete adapter assembly into platform application startup and Linux/Android/iOS builds remain pending. See [ADR-0005](adr/0005-sqlite-trusted-peer-store.md) and the [spike report](spikes/sqlite-store-dependency.md). |
| Windows local signing identity | cargo test --locked --offline -p mm-adapter-win --all-targets (Rust 1.99.1; Rust 1.85.0 with an isolated CARGO_TARGET_DIR) | 0 / 0 | Ten fake-store tests cover create/reopen/sign, rotation, deletion, malformed seeds, storage failures, and canonical pairing-proof validation; Windows Credential Manager calls compile but the real credential store was not accessed. |
| Linux local signing identity | Rust 1.99.1 `cargo test --locked --offline -p mm-adapter-linux`; Rust 1.85 same focused test command with an isolated `CARGO_TARGET_DIR`; GNU Linux `cargo check --locked --offline --target x86_64-unknown-linux-gnu -p mm-adapter-linux` on both toolchains; Linux target Clippy on Rust 1.99.1 | 0 | Eight fake-store tests pass under Rust 1.99.1 and Rust 1.85. The Linux adapter compiles for `x86_64-unknown-linux-gnu` under both toolchains and passes strict Clippy under Rust 1.99.1. Tests do not connect to D-Bus or access a desktop keyring. |
| Flutter runtime package notices (earlier lockfile) | `python -m py_compile scripts/create_flutter_notices.py`; generator run against the Flutter validation mirror's `.dart_tool/package_config.json` and `package_graph.json` | 0 | At that earlier checkpoint, generated 14 runtime package sections (1,298,582 bytes); repeated generation produced the same SHA-256. The later WebRTC dependency update superseded that snapshot with 45 package notices (see the `flutter pub get` row below). Native SDK/Gradle/Apple/codec notices remain unverified. |
| Android TV CI build manifest helper | Python AST parse and mocked `create_manifest` smoke with exact tool output fixtures | 0 | Two generated manifests matched; captured pinned Gradle/AGP and Android packages, validated CycloneDX JSON, and independently recomputed the APK SHA-256. This does not run the Android toolchain or hosted workflow. |
| Rust third-party notice generation | `cargo-about 0.9.2 generate -o docs/THIRD-PARTY-NOTICES-rust.txt docs/THIRD-PARTY-NOTICES-rust.hbs` | 0 | Regenerated the notice to 299,900 bytes after adding Secret Service and mDNS dependencies; SHA-256 `D3EC0B90E6057AC3C18F8B5C2259B9BCB25B49B99D20330F1DB0AFD5BD273CDE`. cargo-about warned that `allo-isolate 0.1.27` has no manifest license field; its bundled Apache-2.0 text is included and reviewed separately. |
| Protocol fuzz manifest/lock | cargo metadata --locked --offline --manifest-path fuzz/Cargo.toml --format-version 1; rustfmt --check fuzz/fuzz_targets/parse_message.rs; Python tomllib parsed fuzz/Cargo.lock | 0 | Full locked offline metadata resolution, Rust formatting, and lock TOML parsing passed after fetching the fuzz crates into the temporary Cargo cache; 24 lock entries include the fuzz package. The fuzzer itself has not run. |
| Release metadata generator | Python AST parse, JSON template parse, CLI help, temporary artifact smoke fixture, and expected template rejection | 0 | Generated a manifest and three checksums in a temp directory; independently recomputed every hash successfully. Unfilled template was rejected with expected exit 2. Fixture was not a MirrorMesh release package. |
| Rust dependency review | `cargo metadata --format-version 1 --locked`; `cargo-deny 0.20.2 --offline --metadata-path <temp> --config <temp> check` | 0 / 0 | Current locked graph passed cargo-deny advisory, ban, license, and source checks using the cached RustSec database; no exceptions were added. Existing warnings remain for missing `allo-isolate` manifest license metadata and duplicate crate versions. `cargo-audit` was not run against the updated graph; the earlier Task 05 audit covered the prior lockfile only. Hosted CI remains unrun. |
| Rust formatting | `cargo fmt --all -- --check` | 0 | pass |
| Rust lint | `cargo clippy --locked --offline --workspace --all-targets -- -D warnings` (Rust 1.99.1) | 0 | pass, including the host-compiled Linux adapter fake-store tests. |
| Linux GNU target compile | Rust 1.99.1 and Rust 1.85: `cargo check --locked --offline --target x86_64-unknown-linux-gnu -p mm-adapter-linux`; separately `cargo check --locked --offline --target x86_64-unknown-linux-gnu -p mm-security -p mm-ports -p mm-application -p mm-adapter-discovery` and `cargo check --locked --offline --target x86_64-unknown-linux-gnu -p mm-runtime` | 0 / 0 | Both toolchains compile the Linux identity adapter, pairing limiter, endpoint-preserving contracts, application coordinator, mDNS adapter, and pairing runtime for GNU/Linux. These are cross-target compiles from Windows, not a Linux runtime, live keyring, or multicast test. |
| Rust tests | `cargo test --locked --offline --workspace --all-features` (Rust 1.99.1 toolchain binary) | 0 | 141 unit tests (including 13 protocol tests, 21 mDNS discovery/publication and endpoint-conversion tests, 15 application tests, 8 pairing-attempt limiter tests, 10 Windows identity tests, 8 Linux identity tests, and 17 pairing/session-runtime tests) + 3 FFI integration + 6 pairing-runtime integration + 1 trusted-session revocation integration + 6 identity integration + 10 SQLite integration tests passed; doc tests passed. |
| Pairing application/runtime MSRV | Rust 1.85 `cargo test --locked --offline -p mm-application -p mm-runtime` with an isolated target directory | 0 | All 15 application tests, 17 runtime unit tests, 6 pairing-runtime integrations, and 1 trusted-session revocation integration pass under the declared workspace MSRV; application coverage includes canonical proof creation and self-pair/signer-failure rejection. |
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

Rust commands ran against the direct Rust 1.99.1 toolchain using a writable temporary `RUSTUP_HOME` because the configured rustup home denied writes. The Task 03 Rust run passed 79 unit tests (including 10 mm-adapter-win identity tests), 3 `mm-ffi` integration tests, 6 `mm-adapter-identity` integration tests, and 10 `mm-store` SQLite integration tests, including transport controls, the safe demo transition path, verifier vectors, and trust persistence. Flutter's SDK was copied to a writable temporary directory because its original protected AppData install could not update the SDK lock/telemetry files. Flutter test and Windows build output went to a temporary project mirror because the sandbox denied writes under the app's ignored `build/` directory. The generated Windows executable launched in a hidden three-second smoke check and loaded the adjacent DLL. No Flutter full-app screenshot/GIF was captured; the separate TV web shell screenshot is recorded above.

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

The mDNS adapter candidate bounds its discovery collection window to two seconds. No time-to-discovery, multicast throughput, CPU use, or LAN interoperability measurement has been made; record these on the owned-LAN fixture after local publication and app assembly exist.

Pairing identity benchmark: not run. Once the provider is implemented, record key generation (where the OS key store supports it), proof signing, signature verification, and SHA-256 fingerprint computation separately on representative Windows, Linux, Android, iOS, and Android TV targets. Report warm and cold p50/p95 with CPU, OS, crate/lockfile version, and hardware details. The current provider candidate has no exact fetched source or target build, so no timing or compatibility result is claimed.

Session grant-gate benchmark: not run. Task 14 provides correctness tests for rejecting an unapproved scope before transport start; no latency or throughput measurement was taken.

## Release verification ledger (Codex populates)

| Build SHA | Test fixture | Command / action | Actual result | Pass? | Evidence |
|---|---|---|---|---|---|
| not built | none | `cargo test --workspace` | not run | unverified | none |
| not built | none | `flutter test` | not run | unverified | none |
| not built | none | Windows native capture sample | not run | unverified | none |
| not built | none | iOS screen permission | not run | unverified | none |

Fail closed on any release without security/capture acceptance and platform-specific install test. Public README only promotes rows from this ledger.

## Task 11: trusted-session revocation runtime (2026-10-18)

This CORE candidate addresses the controlled disconnect/revocation parts of M-01 and M-06, and Task 03 step 7. TDD first failed compilation because the trusted-session runtime API did not exist; the implementation then exposed nondeterministic `HashMap` close order in one test, which was fixed by sorting session IDs before closure.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 131 unit tests, 21 integration tests, and doc tests pass. |
| `cargo test --locked --offline -p mm-runtime` — Rust 1.99.1 | 0 | 16 runtime unit tests and 2 pairing integration tests pass. |
| `cargo test --locked --offline -p mm-runtime` — Rust 1.85 | 0 | The same 16 unit tests and 2 integrations pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes on both toolchains; no Linux runtime was executed. |

Because `cargo` is absent from the PowerShell PATH in this environment, Rust commands used the installed toolchain's `cargo.exe` directly; Rust 1.85 checks set `RUSTC` to that toolchain's `rustc.exe`. The only lockfile change records the local `mm-protocol` test dependency; no external dependency was added. The fake-backed tests do not open sockets. No platform adapter, authenticated signaling, capture/codec teardown, benchmark, LAN, or physical-device test ran. `TrustedSessionRuntime` and its ordering remain proposed pending security and cross-track review.

## Task 12: SQLite-backed revocation integration (2026-10-18)

Added `crates/mm-runtime/tests/trusted_session_revocation_sqlite.rs`. The test stores two active peers in a temporary file-backed SQLite database, starts two sessions for one peer and one for the other, revokes the first peer, verifies only its transports close, drops and reopens the store, and confirms a new runtime still denies the revoked peer while allowing the other. The transport remains fake-backed; this is not a process restart, network exchange, platform test, or hardware result. TDD caught a test helper returning a reference to a temporary path; it now returns an owned `PathBuf`.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 131 unit tests, 22 integration tests, and doc tests pass. |
| `cargo test --locked --offline -p mm-runtime` — Rust 1.85 | 0 | 16 runtime unit tests and 3 runtime integration tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes on both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

No dependency changed for this integration test. No network socket, authenticated signaling, OS key store, platform capture/codec, LAN, benchmark, or physical device was exercised.

## Task 13: pairing and session runtime composition (2026-10-18)

Added `MirrorMeshRuntime` as a proposed facade over `PairingRuntime` and `TrustedSessionRuntime`; both components receive the same identity and `TrustedPeerPort`. The composed integration uses the real Ed25519 verifier and SQLite with a fake transport. It verifies that a peer cannot start before approval, can start after approval, closes on revoke, and remains blocked after runtime recreation. `start_session` opens only the control transport; no offer, media, capture, or platform grant logic runs.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 131 unit tests, 23 integration tests, and doc tests pass. |
| `cargo test --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | 16 runtime unit tests and 4 runtime integration tests pass on both toolchains. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes on both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

No dependency or wire-protocol change was made. ADR-0018 and the facade remain proposed pending security/cross-track review. No listener, network, platform, hardware, benchmark, or media-path test ran.

## Task 14: enforce persisted grant scope before session start (2026-10-18)

`TrustedSessionRuntime::start_session` and `MirrorMeshRuntime::start_session` now take the requested `TrustedPeerGrants` and compare them with the active persisted peer record before starting the control transport. Insufficient grants return `GrantNotAuthorized` without calling `TransportPort::start`. TDD first failed compilation on the missing argument and error variant. The fake-backed unit test rejects an ungranted send scope, confirms no transport start occurred, then accepts the approved view scope. The real-verifier/SQLite composition integration confirms the same behavior using the grant saved only after explicit pairing approval.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test --locked --offline -p mm-runtime` — Rust 1.99.1 | 0 | 17 runtime unit tests, 3 pairing/SQLite integration tests, 1 file-backed revocation integration test, and doc tests pass. |
| `cargo test --locked --offline -p mm-runtime` — Rust 1.85 | 0 | The same 17 unit tests and 4 integration tests pass on the MSRV toolchain. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 132 unit tests, 23 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes with both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

The full workspace gates and GNU/Linux target checks above pass. Proposed ADR-0019 remains pending security/cross-track review. The caller must choose the correct operation scope; the runtime does not infer direction, and no signaling, capture, media, platform, hardware, or performance path was exercised.

## Task 15: strict `pair.begin` frame to pairing runtime (2026-10-19)

`mm-protocol::PairBegin` now matches the application proof fields and enforces the proposed strict schema in ADR-0020: canonical lowercase UUIDs, a 16-byte lowercase-hex nonce, fixed-size lowercase-hex key/signature, valid non-empty grants, and a bounded control-free name. `PairingRuntime::verify_pair_begin_from` reserves the source budget before parsing, decodes the proof fields, verifies through the existing coordinator, and returns pending approval. `MirrorMeshRuntime` exposes the same entry point. TDD first failed on the stale DTO and absent runtime API, then a regression test caught malformed frames bypassing source admission; all pass after the bridge was implemented. Real test-only Ed25519 and SQLite integration confirms trust is not persisted before explicit approval and tampered signatures do not persist.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test --locked --offline -p mm-protocol -p mm-runtime` — Rust 1.99.1 | 0 | 10 protocol tests, 17 runtime unit tests, 6 pairing/SQLite integration tests, 1 revocation integration test, and doc tests pass. |
| `cargo test --locked --offline -p mm-protocol -p mm-runtime` — Rust 1.85 | 0 | The same 10 protocol tests, 17 runtime unit tests, and 7 runtime integration tests pass. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 135 unit tests, 26 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes on both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

No external dependency was added; `mm-protocol` moved from `mm-runtime` development dependencies to runtime dependencies, and the lockfile graph is unchanged. ADR-0020 is a candidate pending security/cross-track review. No listener, TLS, network, platform app, hardware, or media path was exercised. **Pairing-frame parser benchmark: not run.**

## Task 16: shared outbound `pair.begin` encoder (2026-10-19)

`PairingProof` now exposes read-only accessors while keeping its validated fields private. `mm-protocol::encode_pair_begin` accepts the protocol DTO plus typed message ID, sequence, and timestamp, validates with the exact same helper as the parser, and emits a sessionless v1 envelope. A private serializable view ensures the public DTO cannot skip validation through a derived `Serialize` implementation. The runtime/SQLite test now maps an actual signed proof through those accessors and the shared encoder before exercising receiver verification. TDD first failed because the encoder was absent; protocol tests then passed for round-trip decoding, zero-sequence rejection, and non-canonical payload rejection.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test --locked --offline -p mm-protocol -p mm-runtime` — Rust 1.99.1 | 0 | 13 protocol tests, 17 runtime unit tests, 6 pairing/SQLite integrations, 1 revocation integration, and doc tests pass. |
| `cargo test --locked --offline -p mm-protocol -p mm-runtime` — Rust 1.85 | 0 | The same 13 protocol tests, 17 runtime unit tests, and 7 runtime integrations pass. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 138 unit tests, 26 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes on both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

No dependency or lockfile change was made. ADR-0021 remains a candidate pending cross-track/security review. The encoder does not sign proofs or establish TLS identity. No listener, network, platform app, hardware, or media path was exercised. **Outbound encoder benchmark: not run.**

## Task 17: sender-side pairing proof creation (2026-10-19)

`PairingProof::create_signed` now obtains the local public key through `LocalPairingIdentityPort`, validates the proof fields, signs the canonical domain-separated application message, and returns a fully validated proof. It rejects self-pairing before signer access and returns explicit errors for signing or proof-validation failures. The caller remains responsible for validated invitation values, receiver-fingerprint policy, expiry, and local device-ID binding. The real-verifier/SQLite integration helper now creates a signed proof through this API before outbound encoding and receiver verification. TDD first failed because the signing API was missing; focused tests then passed for canonical signer input, early self-pair rejection, and signer failure.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test --locked --offline -p mm-application -p mm-runtime` — Rust 1.99.1 | 0 | 15 application tests, 17 runtime unit tests, 6 pairing/SQLite integrations, 1 revocation integration, and doc tests pass. |
| `cargo test --locked --offline -p mm-application -p mm-runtime` — Rust 1.85 | 0 | The same 15 application tests, 17 runtime unit tests, 6 pairing/SQLite integrations, and 1 revocation integration pass. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 141 unit tests, 26 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-runtime` — Rust 1.99.1 and 1.85 | 0 | GNU/Linux target compilation passes with both toolchains; no Linux runtime was executed. |
| `git diff --check` | 0 | Patch whitespace check passes. |

No dependency or lockfile change was made. Proposed ADR-0022 awaits cross-track/security review. Proof construction does not validate the QR, authenticate the receiver or signaling transport, or persist trust. No listener, network, platform app, hardware, or media path was exercised. **Sender proof creation benchmark: not run.**

## Task 18: native TLS 1.3 raw-public-key pinning candidate

`mm-adapter-tls` now constructs a native Rustls TLS 1.3 server config backed by the dedicated `LocalTlsIdentityPort` and a client verifier that pins the SHA-256 fingerprint of the raw Ed25519 key from the validated QR. The server advertises the matching SPKI as an RFC 7250 raw public key. Client signature verification runs through Rustls’ raw-key helper. The signer accepts only the exact TLS 1.3 server CertificateVerify input shape and rejects arbitrary application data. TLS 1.2, client resumption, server sessions/tickets, and early data are disabled. TDD first failed at the missing TLS APIs; the initial handshake attempt then showed that Rustls `CertifiedKey::keys_match()` parses an X.509 certificate and is not applicable to an RPK. The adapter now relies on the constructed shared SPKI plus TLS CertificateVerify, which the passing matching and mismatched-signer handshakes exercise.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test -p mm-adapter-tls --locked --offline` — Rust 1.99.1 | 0 | Seven tests pass: matching QR pin, wrong pin, malformed SPKI, non-Ed25519 key, signer failure, signer/public-key mismatch, and rejection of non-CertificateVerify signing input. |
| `cargo test -p mm-adapter-tls --locked --offline` — Rust 1.85 | 0 | The same seven tests and doc tests pass on the Windows host. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 148 unit tests, 26 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-adapter-tls` — Rust 1.99.1 and 1.85 | 1 | `ring` cannot cross-compile because `x86_64-linux-gnu-gcc` is absent from the Windows runner. |
| `cargo check --target aarch64-linux-android --locked --offline -p mm-adapter-tls` — Rust 1.99.1 | 1 | `ring` cannot cross-compile because the Android NDK clang toolchain is absent. No Android SDK/NDK environment is configured. |
| `cargo metadata --locked --offline --format-version 1` — Rust 1.99.1 | 0 | Locked dependency metadata resolves. The new Rustls/ring dependency license expressions fit the allow-list after adding ISC. `cargo-deny` is not installed locally, so the full deny/advisory gate did not run. |
| `git diff --check` | 0 | No whitespace errors. |

No socket, WebSocket, QR UI, OS key store, browser, app startup, platform listener, physical device, LAN, or end-to-end pairing flow was exercised. The browser fallback needs a separately trusted HTTPS/WSS endpoint. Proposed [ADR-0023](adr/0023-tls13-raw-public-key-pinning-candidate.md) requires security/cross-track review. No benchmark was run.

## Task 19: bounded native WSS pairing listener candidate

`mm-adapter-signaling` binds a caller-selected socket to the TLS configuration from ADR-0023, accepts only `GET /v1/control` without a query or browser `Origin`, and reads one bounded text `pair.begin`. It captures the peer IP immediately after accept. The runtime reserves that IP's attempt budget after TLS and WebSocket framing but before JSON parsing/signature verification. A wrapper enforces one absolute ten-second deadline from accept through the first complete application message; eight concurrent accept/handshake/pending slots and 256 KiB frame/message limits bound the listener candidate. A verified request returns as pending approval alongside the live connection; the listener never approves or persists trust. The application still must enforce the approval expiry and drop denied/stale connections.

| Command / toolchain | Exit | Result |
|---|---:|---|
| `cargo test -p mm-adapter-signaling --locked --offline` — Rust 1.99.1 | 0 | Five unit tests and one two-process loopback integration test pass. The slow partial-read regression reaches the absolute deadline; the process test verifies QR-pinned TLS, signed `pair.begin`, and approval-only persistence. |
| `cargo test -p mm-adapter-signaling --locked --offline` — Rust 1.85 | 0 | The same five unit tests and two-process loopback integration pass using an isolated target directory. |
| `cargo fmt --all -- --check` — Rust 1.99.1 | 0 | Workspace formatting passes. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — Rust 1.99.1 | 0 | Strict workspace lint passes. |
| `cargo test --locked --offline --workspace --all-features` — Rust 1.99.1 | 0 | 153 unit tests, 27 integration tests, and doc tests pass. |
| `cargo check --target x86_64-unknown-linux-gnu --locked --offline -p mm-adapter-signaling` — Rust 1.99.1 | 1 | Cross-compilation is blocked because `ring` cannot find `x86_64-linux-gnu-gcc` on this Windows runner. |
| `cargo check --target aarch64-linux-android --locked --offline -p mm-adapter-signaling` — Rust 1.99.1 | 1 | Cross-compilation is blocked because the Android NDK clang toolchain is absent. |
| `cargo metadata --locked --offline --format-version 1` — Rust 1.99.1 | 0 | Locked dependency metadata resolves; Tungstenite 0.30.0 and its transitive license expressions were reviewed against the repository allowlist. `cargo-deny` is not installed locally, so the full deny/advisory gate did not run. |
| `git diff --check` | 0 | No whitespace errors. |

The loopback test opens a local TCP/TLS/WebSocket connection only; it is not LAN, browser, platform-app, UI, or device evidence. The listener rejects browser `Origin` requests, so the TV web fallback still requires an ordinary trusted HTTPS/WSS endpoint. Per-source limiting begins after TLS/WebSocket framing; the absolute deadline and connection cap bound the earlier window but do not provide per-IP handshake throttling. The Windows runner has no iOS/macOS toolchain, so no iOS target build was possible. Proposed [ADR-0024](adr/0024-bounded-wss-pairing-listener-candidate.md) requires security/cross-track review. No benchmark was run.
