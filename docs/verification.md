# Verification boundaries for the test branch

The user requested builds and source integration without test files or automated test workflows.
No unit, instrumentation, synthetic-media or GitHub Actions test is executed on this branch.
Historical test evidence on `dev` does not establish correctness of these new changes.

## Current evidence

### Windows 0.3.2 missing AirPlay code (2026-10-21, test_4)

- Owner confirms iOS asks for an AirPlay code but PC shows none. Reproduced with one
  loopback POST /pair-pin-start RTSP/1.0 to127.0.0.1:7101: response200, immediate close,
  --diagnose-airplay-pin exit1(no retained code). Same request with socket held3s exits0.
  This isolates MM_AIRPLAY_IDLE clearing pin_ when the challenge socket closes.
  Initial probe used media port7100; it was corrected to the control port7101.
- Original immediate-close request now retains code for the diagnostic observation
  period. Final packaged diagnose-airplay.ps1 -CheckPin exits0 for both PIN-start(200)
  and binary-plist password SETUP(401) followed by immediate close. Reports only
  CODE_AVAILABLE, never digits; no client approved/media received/helper left running.
- scripts/build-windows.ps1 final exit0, Windows0.3.2+8; log
  .toolchains/windows-0.3.2-package-final.log. Flutter analyze exit0/no issues; Dart format
  check0/zero changes. PowerShell AST and diff checks exit0. Archive payload inspected;
  all283 release-manifest digests match. Unsigned owner-local ZIP and checksum provided.
- Read-only review: no remaining Critical/Important/Minor findings. Resolved introduced
  active-stream expiry risk by ignoring unrelated PIN events during active video/approval;
  bounded the scrollable header and reveal code/approval panels to avoid clipping.
- No unit/instrumentation/workflow tests or test files added/run. Only local runtime
  diagnosis, builds, formatting and static analysis. Actual rendered layout/DPI,120s
  expiry timing, physical iOS code entry and media remain unverified; no production or
  hardware-complete claim. Rust, upstream helper patch and dependency locks unchanged.

### Windows 0.3.1 immediate AirPlay shutdown repair (2026-10-21)

- Reproduced packaged `uxplay.exe -h`: exit `-1073741515` / `0xC0000135`, no output.
  Imported `libgstreamer-1.0-0.dll` was incorrectly in the plugin directory. Copying that
  same DLL to `bin` made the same help invocation exit 0. Source fix classifies directories.
- The actual native startup then exited 1: the plugin whitelist omitted `autodetect`,
  required by pinned `audio_renderer.c::check_plugins`. Adding only that plugin made
  the same startup path reach Ready/exit 0. The final packager includes it by default.
- `scripts/build-windows.ps1`: final exit 0; Windows 0.3.1+7, native MSVC compilation,
  locked helper/runtime packaging and portable ZIP. Log `.toolchains/windows-0.3.1-package-final.log`.
- `powershell.exe -NoProfile -File scripts/diagnose-airplay.ps1 -CheckStartup`: final
  exit 0. Packaged loader exits 0; native receiver remains Ready for two seconds, then
  stops. Process inspection confirms no `uxplay` remains. No client was approved or media
  streamed. The earlier shell wrapper checked an unset LASTEXITCODE after .NET process
  invocation; the final explicit PowerShell process invocation provides an actual exit code.
- Dart format: exit 0, zero changes. `flutter analyze`: exit 0, no issues. PowerShell
  syntax checks for all five affected scripts and diff whitespace check: exit 0.
- Final archive inspection verifies the core DLL in bin, required autodetect plugin,
  native helper and support script. 283 file digests match the release manifest. ZIP
  entry separators from Windows Compress-Archive were normalized during inspection;
  an initial forward-slash-only lookup was corrected, with no missing archive payload.
- Read-only source review: no remaining Critical/Important/Minor findings. The reviewer
  caught and resolved premature diagnostic success by requiring post-Ready observation.
- No unit/instrumentation/workflow suites or test files were added or run. This is local
  startup evidence only. iPad/iPhone LAN discovery and media remain HARDWARE_VERIFIED pending;
  no project-wide CODE_COMPLETE or automated media-integration claim is made. Binaries are
  unsigned owner-local previews; public dependency redistribution review remains pending.

### Windows 0.3.0 fitted viewer + native AirPlay preview (2026-10-21)

- `scripts/setup-airplay-toolchain.ps1`: exit 0 against the installed exact package lock.
  Initial dependency download encountered HTTP429; serial retries of official signed
  packages succeeded. No signature policy, Defender or firewall settings were weakened.
- `scripts/build-airplay.ps1`: exit 0; pinned UxPlay and readable integration/hardening patch
  compile with portable x64 CPU flags. Selected plugins/import closure: 141 native binaries.
- `flutter build windows --release`: exit 0; native MSVC `/W4 /WX` compilation passed.
- `scripts/build-windows.ps1`: exit 0; product version 0.3.0+6; EXE, native AirPlay runtime,
  licenses and corresponding helper source included. Packaged helper hash matches compiler
  output. The bundle is unsigned and is an owner-local preview.
- Dart formatting and `flutter analyze`: exit 0, no issues. Initial brace-style findings
  were corrected before the successful analysis.
- Initial AirPlay packaging exited 1 because Windows PowerShell needed an explicit
  System.IO.Compression assembly load; added it and the subsequent package build exited 0.
- Read-only follow-up review confirmed auth-field checks/fail-closed PIN, decoded-buffer
  readiness, transport-independent media-idle, generation guards and PowerShell exit handling.
  No unresolved Critical/Important findings. Both diff and reverse-patch checks exited 0.
- No receiver was started. No unit/instrumentation/workflow test suite or physical AirPlay
  verification was performed. iPhone 15 Pro Max / 2020 base iPad compatibility, OS versions,
  audio/video, layout, DPI and reconnection remain device-verification activities.
- The owner separately confirms PC↔Android works on 0.2.1. Android code was unchanged and
  APKs were not rebuilt for this Windows-only increment. The retained iOS app still needs
  Apple compilation/signing; the native AirPlay adapter does not require that iOS app.
- Modified helper source/licenses are included locally. Complete dependency redistribution
  review remains required before publishing the preview as a public binary release.

### 0.2.1 on test_3 (2026-10-21)

- `cargo check --workspace --lib --locked`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --workspace --lib --locked -- -D warnings`: exit 0.
- Dart formatting and `flutter analyze`: exit 0, no issues.
- `scripts/build-windows.ps1`: exit 0; final reviewed EXE/DLLs/data/ZIP packaged.
  Windows product/file version is 0.2.1+5.
- `scripts/build-android.ps1 -Configuration debug` and `-Configuration release`: exit 0
  for both phone/TV ARM64 variants. APK metadata is versionName 0.2.1, versionCode 5.
  Debug APKs are installable previews; release APKs remain unsigned.
- `:app:lintPhoneDebug :app:lintTvDebug`: exit 0, 0 errors and 77 warnings per flavor.
  First invocation exited 1 because ANDROID_SDK_ROOT was absent from that shell;
  setting the existing SDK path resolved it. No source workaround/suppression was added.
- Initial Clippy/analyzer findings (nested if / Dart braces) were fixed before passing.
- Read-only review found and corrected LAN nonce downgrade across listener restarts,
  bootstrap clock sampling and admission accounting. No unresolved Critical/Important findings.
- No unit, instrumentation or workflow test suite ran. Lint's empty test-source models
  do not constitute a device/integration result. No iOS native build was available.
- Owner reports Android 13 on Redmi Note 5 failing after capture approval. Source startup
  ordering was corrected, but that failure has not been reproduced or confirmed fixed on
  physical devices here. Direct LAN approval/code comparison/media also remain unverified.

See [ADR0058](adr/0058-direct-lan-requests-and-sender-startup.md) for the source diagnosis
and connection authentication design. Detailed local logs are `.toolchains/*0.2.1*.log`.

### 0.2.0 on test_3 (2026-10-21)

- `cargo check --workspace --lib --offline`: exit 0 (Windows host).
- `cargo clippy --workspace --lib --locked -- -D warnings`: exit 0.
- Flutter formatting and `flutter analyze`: exit 0, no issues.
- `scripts/build-windows.ps1`: exit 0, unsigned portable EXE/ZIP packaged.
- `scripts/build-android.ps1 -Configuration debug` and `-Configuration release`: exit 0,
  phone and TV ARM64 JNI/Java/APKs compiled. Release APKs remain unsigned.
- `:app:lintPhoneDebug :app:lintTvDebug`: exit 0; 0 errors, 78 warnings per variant,
  including existing localization/resource/deprecation warnings. No suppression baseline added.
- iOS Rust target, Swift broadcast extension, Ruby configurator, Xcode embedding/signing:
  not executed on Windows; Apple native build and device evidence are pending.
- The owner reports baseline app success; new QR/search/mobile-sender/PC-receiver paths
  have not had physical verification. No test suites or workflows ran.

The first Android build was blocked by a shared Gradle-cache lock from VS Code Java
tooling; an isolated `.toolchains/gradle-test3` cache resolved the lock without stopping
the user process. Two Java compile errors were corrected before the successful builds.

### Historical 0.1.1 checkpoint

- Rust production workspace compilation: passed (`cargo check --workspace --lib --offline`, exit 0).
- Rust production Clippy: `cargo clippy --workspace --lib --locked -- -D warnings`, exit 0.
- Flutter static analysis: `flutter analyze`, no issues, exit 0.
- Windows 0.1.1: `scripts/build-windows.ps1`, portable EXE, dependencies and ZIP packaged, exit 0.
- Android 0.1.1 ARM64 Rust JNI and Java: `scripts/build-android.ps1 -Configuration debug`
  and `-Configuration release`, phone and TV APK assembly and checksum generation, exit 0.
  Debug APKs are locally signed previews; release APKs are unsigned.
- Hardware pairing, capture, transmission, MediaCodec rendering and resource behavior: pending.
- Release signing, Windows installer, Google Play upload and store acceptance: pending.

Evidence above was collected on 2026-10-20. Gradle reports deprecations for a future Gradle 10
upgrade; the pinned Gradle 9.8.1 builds succeeded. No dependencies were upgraded for this fix.
The user reported an invitation/approval failure. No Android device is attached to ADB;
the exact failure message is pending. The new diagnostics and refresh changes do not prove
that the original device failure has been resolved.

## Final user device validation

Use the [user guide](USER_GUIDE.md). On actual devices, confirm invitation expiry, rejection,
explicit approval, selected-source capture, a changing received picture, readable screenshots,
stop from either endpoint, revocation, brief network recovery, retry after failure and app exit.
Observe visible capture indicators, GPU/decoder diagnostics, memory and latency. Record actual
device/OS/GPU versions and failures. These activities have not been performed by this build session.

Keep CODE_COMPLETE, BUILD_VERIFIED and HARDWARE_VERIFIED separate in every release statement.
