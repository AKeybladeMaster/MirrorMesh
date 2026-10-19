# ADR-0037 — Android TV JNI pairing bridge

- **Date:** 2026-10-19
- **Status:** Proposed; Android target build and activity integration remain open

## Decision

The native Android TV activity will reach pairing through JNI entry points in `mm-ffi`. Those
entry points initialize `ndk-context`, start the existing `mm-adapter-android` receiver service
on an explicitly selected numeric local address at port 8765, and return bounded JSON metadata
for an invitation and a verified pending sender request. Separate explicit methods approve a
requested grant subset or reject the request. Approved WSS connections remain owned by Rust and
are closed by connection handle; no video frames cross JNI.

The Android TV Gradle build compiles the locked workspace `mm-ffi` crate for ARM64 with the pinned
NDK and packages `libmm_ffi.so`. The Java bridge is a narrow method declaration layer and does not
parse protocol frames or perform cryptography.

## Consequences and limits

`waitForPairingRequest` blocks and must run on a worker thread. This slice provides the bridge
source and build wiring, but the current activity does not call it. Listener cancellation, QR
rendering, the approval UI, sending/receiving session control messages, WebRTC setup, and a target
APK build remain open. No hardware behavior is claimed.
