# TASK 06 — CI, security, benchmarking and cross-platform releases

**Skills:** verification-before-completion, requesting-code-review, systematic-debugging (failures), codebase-design

## Release workflow tasks

1. Launch **concurrent** GitHub Actions workflows for Rust/Flutter, Windows x64, Linux x64, Android phone ARM64, **separate Android TV receiver**, iOS ARM64 (macOS/Xcode), TV web bundle, scheduled fuzzing; require compile and unit tests on each supported runner.
2. `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo audit`, dependency/license review (`cargo deny` optional), Flutter analyze/test, formatting, protocol schema compatibility checks.
3. Protect tags/release branches; sign Windows MSIX/installer, Android APK/AAB, Apple IPA; Linux package signing where appropriate. Signing material from protected secrets, never repo files.
4. Generate SBOM (CycloneDX/SPDX), checksums, third-party notice, reproducibility notes and exact SDK version manifest per release.
5. Test Windows clean installer/upgrade/uninstall; Linux Wayland/X11 dependencies; Android **phone sender/receiver** and **native TV receiver APK** separately; iOS capture-availability matrix, iOS receiver and TestFlight. TV browser WebRTC over HTTPS is a separately documented fallback, not a replacement for native-TV testing.
6. Browser TV tested matrix includes manufacturer/browser/version and mode; mark unsupported variants explicitly. Do not label any TV as supported without tests.
7. Run external high-speed camera latency fixture, 720p fallback, 60-minute GPU/memory soak, disconnect/revoke and TCP/UDP firewall failure tests; record p50/p95, screenshot and annotations.
8. Only when gates pass, create GitHub Release with changelog, SHA256, SBOM, screenshots, demo, privacy/security docs and known issues.
9. Publish portfolio case study with measured results, decisions, alternatives, device limits and genuine original screenshots.
10. Repeat security threat review and dependency license audit whenever media/casting adapter changes.

## Minimum release assets

`MirrorMesh-<tag>-windows-x64.msix` (or installer), `MirrorMesh-<tag>-linux-x64.deb`/AppImage, `MirrorMesh-<tag>-android-phone-arm64.apk`/AAB, **`MirrorMesh-<tag>-android-tv-arm64.apk`/AAB (distinct)**, and iOS signed IPA/TestFlight via approved Apple channels; TV web HTTPS receiver build/deploy only after compatibility proof. Actual naming/tools can differ; each artifact only published once build/install runs pass.

## No-greenwashing rule

All four application pipelines start in parallel, but a release artifact is published only if **that platform** passes its independent install, permission, security, media and hardware gates. Android TV is the primary TV feature: a passing TV-browser demo cannot replace it. CI passing `cargo test` does not imply native ScreenCaptureKit/MediaProjection support or physical TV compatibility. Link manual device results with exact model. If a platform cannot be tested, publish no "supported" badge for it. Leave it `planned`/`experimental`.
