# Local release artifacts

Build with `scripts/build-windows.ps1` and `scripts/build-android.ps1`.
The scripts create Windows portable files and Android phone/TV APKs here, with checksums.
Generated binary payloads and SDK caches are excluded from Git; source, build instructions,
folder descriptions and checksums are tracked on `test_5`. Current Windows/Android: 0.4.0.
Upgrade both apps together for signed device details. iOS app builds need macOS/Xcode;
no IPA is available. See `docs/connections.md` for native protocol availability.

Debug APKs are locally signed installable previews. Release APKs are unsigned until the
owner supplies signing credentials. Windows is unsigned. No device-validation or store
release claim is implied by these artifacts.
