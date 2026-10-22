# macOS source build — no binary available

No Mac/Xcode builder is available on the current Windows/WSL machine. No macOS app,
ZIP or DMG has been built or verified here.

On a Mac, use the release source and run `sh scripts/build-macos.sh` with the documented tools.
The pipeline targets macOS 13+, ARM64 and Intel. The generated unsigned preview needs
appropriate Apple signing/entitlements for Keychain use and owner signing/notarization
before distribution. Universal linking, capture, Keychain and media remain unverified.

See [macOS build instructions](../../docs/build-release.md).
