# Verification boundaries for the test branch

The user requested builds and source integration without test files or automated test workflows.
No unit, instrumentation, synthetic-media or GitHub Actions test is executed on this branch.
Historical test evidence on `dev` does not establish correctness of these new changes.

## Current evidence

- Rust production workspace compilation: passed (`cargo check --workspace --lib --offline`, exit 0).
- Flutter static analysis: passed after eight brace-style fixes (exit 0).
- Android ARM64 Rust JNI and Java: compiled; phone and TV release/debug APK assembly passed.
- Final packaged Windows EXE, updated Android artifacts and checksum inventory: being finalized.
- Hardware pairing, capture, transmission, MediaCodec rendering and resource behavior: pending.
- Release signing, Windows installer, Google Play upload and store acceptance: pending.

## Final user device validation

Use the [user guide](USER_GUIDE.md). On actual devices, confirm invitation expiry, rejection,
explicit approval, selected-source capture, a changing received picture, readable screenshots,
stop from either endpoint, revocation, brief network recovery, retry after failure and app exit.
Observe visible capture indicators, GPU/decoder diagnostics, memory and latency. Record actual
device/OS/GPU versions and failures. These activities have not been performed by this build session.

Keep CODE_COMPLETE, BUILD_VERIFIED and HARDWARE_VERIFIED separate in every release statement.
