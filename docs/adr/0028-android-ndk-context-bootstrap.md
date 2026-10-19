# ADR-0028: Initialize Android application context for the Rust bridge

**Status:** Proposed implementation; Android runner and cross-track review pending

**Date:** 2026-10-19
**Requirements:** M-01, M-06, M-07, M-08

## Context

Task 22 added an Android Keystore-backed identity adapter whose JNI operations require a valid Java VM and Android `Context`. Flutter loads `libmm_ffi.so` through Dart's dynamic-library loader, so loading the library does not itself provide the Rust `ndk-context` crate with an Android context. The adapter must remain lazy: app startup may initialize JNI access, but must not create or read a pairing identity until a user begins pairing.

`ndk-context` requires valid VM and context pointers and exactly one initialization. A process-lifetime JNI global reference can keep the application context alive while Rust code uses it. The Flutter Rust Bridge guide documents explicit context handoff from Android host code as an initialization method.

## Decision

- Android `MainActivity` attempts to load the packaged `mm_ffi` native library through `System.loadLibrary` before configuring the Flutter engine. If the optional library is absent, the existing Flutter Rust-unavailable fallback remains available.
- Before Flutter code can call Rust, `MainActivity` passes `applicationContext` to a narrow JNI entrypoint in `mm-ffi`.
- The Rust entrypoint creates a global JNI reference, obtains the Java VM, and calls `ndk_context::initialize_android_context` once. A `OnceLock` retains the global reference for the process lifetime and prevents a second initialization.
- When the library loaded successfully, JNI setup failure returns `JNI_FALSE`; the Android host fails startup instead of allowing later key-store access with an uninitialized context.
- This bootstrap does not create an identity, open the trusted-peer database, or assemble a pairing runtime. Those operations remain behind a future explicit pairing action and runtime-lifecycle task.

## Consequences and limits

- Android Rust functionality depends on the CI-packaged `libmm_ffi.so`. The existing Android workflow stages the ARM64 library into `jniLibs/arm64-v8a` before building its APK. If it is missing, the shell keeps its existing unavailable-state behavior.
- The application context is available to Rust process code but is not exposed through the Dart FFI surface.
- The JNI symbol, Android loader order, Android SDK/NDK compile, APK startup, and device behavior remain unverified on this Windows host. The host JNI type-check is not Android runtime evidence.
- Pairing, trust persistence, capture permission, and media remain unchanged. The Android identity still must be opened only after a user starts pairing.

## Verification

`ndk_context::initialize_android_context` and the JNI entrypoint are type-checked against the pinned `jni 0.21.1` and `ndk-context 0.1.1` APIs in a temporary host compile harness. The host FFI tests do not execute the Android-only entrypoint. The Android target check is blocked locally because `dart-sys` cannot find NDK clang; no APK or device test has run.

## References

- [Flutter Rust Bridge Android NDK initialization](https://cjycode.com/flutter_rust_bridge/guides/how-to/ndk-init)
- [`ndk_context::initialize_android_context` 0.1.1](https://docs.rs/ndk-context/0.1.1/ndk_context/fn.initialize_android_context.html)
- [Android NDK JNI tips](https://developer.android.com/ndk/guides/jni-tips)
