# Desktop and iOS application

Flutter UI with native Windows/Linux/macOS/iOS hosts and a shared Rust control plane.
Native WebRTC handles capture/rendering; raw media never crosses Dart/FFI.

- `lib/`: send/receive/search/approval, session lifecycle and native metadata commands.
- `windows/`: native runner, AirPlay host, device context and OS Miracast handoff/status.
- `linux/`: GTK runner, public device context and shared Rust initialization.
- `macos/`: Swift host and Apple Keychain/static-Rust integration (unbuilt here).
- `ios/`: Swift host, native receive and ReplayKit extension source (unbuilt here).
- `lib/src/rust/` and Rust generated bridge: pinned generated control bindings; retain them.

The canonical Android APKs are built from `../android-tv`, not from a Flutter Android shell.
For products, pinned dependencies, debug commands and signing, use the root
[build guide](../../docs/build-release.md). See [architecture](../../docs/ARCHITECTURE.md)
and [actual readiness](../../docs/IMPLEMENTATION_STATUS.md).

Optional local checks: `flutter analyze`, `dart format --output=none --set-exit-if-changed lib`.
No test suites or GitHub workflows are included/run under the release policy.
