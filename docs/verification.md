# Verification boundaries for the test branch

The user requested builds and source integration without test files or automated test workflows.
No unit, instrumentation, synthetic-media or GitHub Actions test is executed on this branch.
Historical test evidence on `dev` does not establish correctness of these new changes.

## Current evidence

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
