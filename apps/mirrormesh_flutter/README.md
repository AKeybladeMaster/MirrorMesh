# MirrorMesh Flutter control shell

This Flutter project targets Windows, Linux, Android, and iOS. It provides Connect, Session, Devices, and Settings pages plus a clearly labeled local-only demo. When the Rust bridge loads, scan, pairing-start, and end call narrow Rust state transitions; when it is unavailable, the UI uses a local fallback. The native integration test verifies `Idle → Discovered → Pairing → Stopped`. Live pairing, network discovery, media, and capture are not connected. Windows packages the state-only Rust library beside the executable; Linux CMake is configured to bundle it in `bundle/lib`; Android CI is configured to package the ARM64 library in the APK; iOS device builds link a Rust static library and export only the FRB symbols used through Dart process lookup. The Settings page displays the same local Rust session. Linux, Android, and iOS packaging await platform runner evidence. The iOS simulator bridge remains unavailable. The demo never verifies a peer, uses the network, requests capture, or streams media.

## Checks

Use Flutter 3.47.7 stable and run from this directory:

```sh
flutter pub get
dart format --output=none --set-exit-if-changed lib test
flutter analyze
flutter test
```

The Windows shell can be compiled with `flutter build windows --release`; Cargo and the pinned Windows Rust target must be available on `PATH`. Its CMake build compiles and packages `mm_ffi.dll`. Linux uses the pinned `x86_64-unknown-linux-gnu` Rust target, and CMake installs `libmm_ffi.so` in `bundle/lib`; this path is configured in CI but has not yet been built on a Linux runner. Android CI cross-compiles `aarch64-linux-android`, stages `libmm_ffi.so` under `jniLibs/arm64-v8a`, and verifies the ARM64 APK entry. iOS Xcode builds the pinned `aarch64-apple-ios` static library for device, derives the FRB export list with `nm`, and links it into Runner with exported global symbols retained; the macOS build is configured but not yet run. No platform build implies real capture or streaming support.

The Windows build and Flutter-to-Rust integration test passed from a writable workspace-shaped temporary mirror on 2026-10-18. The native test requires `MIRRORMESH_FFI_LIBRARY` to point at a built DLL or shared library. See the root [verification ledger](../../docs/verification.md) for exact commands and current platform evidence.

## Rust bridge

Rust bridge API source is in `../../crates/mm-ffi/src/bridge_api.rs`. The checked-in Dart and Rust bindings were generated with `flutter_rust_bridge_codegen` 2.13.0:

```sh
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
```

The bridge exposes session control and state-only events; no pixel or audio buffers cross Dart FFI. App startup safely calls `RustLib.init`; when the library is missing, the app shows an unavailable status and remains usable. Live pairing and capture remain disabled because the bridge does not accept Dart-supplied identity or permission flags.

On Windows, the Flutter CMake build compiles the Rust library into `../../target/x86_64-pc-windows-msvc/release/mm_ffi.dll` and installs a copy next to the app executable. To run the native bridge widget test directly after a Cargo build:

```powershell
cargo build --manifest-path ../../Cargo.toml --package mm-ffi --target x86_64-pc-windows-msvc --release
$env:MIRRORMESH_FFI_LIBRARY = (Resolve-Path ../../target/x86_64-pc-windows-msvc/release/mm_ffi.dll).Path
flutter test --no-pub test/rust_bridge_test.dart
```

For an Android ARM64 debug APK on Linux with Android SDK/NDK installed, build and stage the state bridge before Flutter packages it:

```sh
NDK_HOME="$ANDROID_SDK_ROOT/ndk/28.2.13676358"
NDK_BIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export PATH="$NDK_BIN:$PATH"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android21-clang"
cargo build --manifest-path ../../Cargo.toml --package mm-ffi --target aarch64-linux-android --release
mkdir -p android/app/src/main/jniLibs/arm64-v8a
cp ../../target/aarch64-linux-android/release/libmm_ffi.so android/app/src/main/jniLibs/arm64-v8a/libmm_ffi.so
flutter build apk --debug --no-pub --target-platform android-arm64
```

CI checks that the APK contains `lib/arm64-v8a/libmm_ffi.so`. This is packaging evidence; it does not establish device initialization or media support.
