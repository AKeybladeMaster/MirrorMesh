# Local release artifacts

Build with `scripts/build-windows.ps1` and `scripts/build-android.ps1`.
The scripts create Windows portable files and Android phone/TV APKs here, with checksums.
Generated binary payloads and SDK caches are excluded from Git; source, build instructions,
folder descriptions and checksums are tracked on `test_6`. Stable Windows/Android: 0.4.0;
new desktop/Android preview: 0.4.1. The preserved stable branch is `pre_release_v1`.
Upgrade both apps together for signed device details. iOS app builds need macOS/Xcode;
no IPA is available. See `docs/connections.md` for native protocol availability.

Debug APKs are locally signed installable previews. Release APKs are unsigned until the
owner supplies signing credentials. Windows is unsigned. No device-validation or store
release claim is implied by these artifacts.

Linux packaging: `scripts/build-linux-docker.ps1` on Windows with Docker Desktop, or
`sh scripts/build-linux.sh` on Linux. Outputs a complete x64 tarball and checksums.
macOS source build: `sh scripts/build-macos.sh` on a Mac; no macOS binary exists here.
See [build instructions](../docs/build-release.md) and [TV installation](../docs/install-android-tv.md).
