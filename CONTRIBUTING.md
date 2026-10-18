# Contributing

MirrorMesh is an early implementation. Please keep each change within one workstream and use the corresponding task brief in `codex/`.

## Before changing code

- Read `AGENTS.md`, the product spec, protocol/security docs, and the current task brief.
- Start with a short patch plan that names owned paths, commands, expected evidence, and risks.
- Treat shared Rust ports, FFI, protocol schemas, and golden fixtures as CORE-owned. Propose shared API changes for review before editing them from a platform task.
- Check current SDK documentation and dependency licenses before adding packages. Keep lockfiles committed.
- Keep platform capture, permissions, codec, and rendering in native adapters. Do not pass pixel or audio buffers through Dart events.

## Pull requests

Use one platform or task scope per pull request. Include requirement IDs, changed paths, exact commands and exit codes, security/permission effects, and blockers. Do not describe a mock, compile, or static check as a hardware or integration test. UI changes need an original screenshot or a note explaining why a screenshot could not be captured.

Run the relevant repository checks before requesting review. Keep real device and store signing evidence separate from CI compile results. Never commit credentials, private keys, proprietary binaries, or personal screen captures.

## Current local checks

Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.

Flutter: from `apps/mirrormesh_flutter`, run `flutter pub get`, `dart format --output=none --set-exit-if-changed lib test`, `flutter analyze`, and `flutter test`.

TV web: from `apps/tv-web`, run `npm ci` and `npm test` to compile the TypeScript source and run the capability tests.

Android TV: its source shell currently has a PowerShell XML/permission check at `apps/android-tv/scripts/verify-shell.ps1`; the Android SDK build gate is separate.
