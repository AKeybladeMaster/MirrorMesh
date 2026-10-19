# MirrorMesh Android TV receiver shell

This is a separate native Android TV application target. It declares the Leanback launcher, touchscreen-free navigation, app icon, and TV banner. The main activity lists active local IPv4 addresses, starts the Rust-owned pairing listener, displays a short-lived one-time QR invitation, waits for a verified sender request off the UI thread, and presents the sender fingerprint and requested grants. All permission boxes begin unchecked; approval and rejection require separate remote-control actions. A refresh action cancels the current idle wait and issues a fresh invitation; rejection and channel close also refresh the one-time code. It also contains a native receive-only WebRTC adapter and fullscreen `SurfaceViewRenderer` activity. The adapter accepts only an approved signaling channel, bounds SDP and ICE inputs, configures a receive-only transceiver with H.264-only codec preferences, and uses `HardwareVideoDecoderFactory` without a software decoder fallback.

The pairing screen is connected to the Rust JNI bridge, but the approved control channel is not yet connected to the TV WebRTC adapter or a Windows sender. Pairing success does not start a video session. Only the ordinary `INTERNET` permission is requested; cleartext traffic is disabled and no local screen, camera, or microphone capture is requested. ZXing Core 3.5.4 (Apache-2.0) encodes the invitation QR. Native Android TV is the primary TV path; the browser receiver remains a compatibility fallback.

## Build

Install JDK 17 or newer, Android SDK platform 37, Build Tools 36.0.0 or newer, NDK 28.2.13676358, Rust 1.99.1 with the `aarch64-linux-android` target, and Gradle 9.6.0. From this directory run `gradle :app:assembleDebug`. Gradle builds and packages the shared `mm_ffi` Rust library for ARM64. The output is `app/build/outputs/apk/debug/app-debug.apk`. An emulator build is only a compile/UI smoke check; it does not prove hardware decoding or a real TV session. The native JNI pairing bridge is implemented, but the current activity has not been wired to it yet.

GitHub Actions runs the Java state checks separately from an Ubuntu native APK build. The build job installs Android platform 37 and Build Tools 36.0.0, assembles the debug APK, and prints its SHA-256 digest. It does not publish or sign the APK.

## Local behavior checks

From the repository root run:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\test-demo-state.ps1
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\verify-shell.ps1
```

The JDK-only tests cover explicit pairing state, bounded H.264 SDP selection, and receiver lifecycle. XML/source checks verify bridge wiring, QR dependency, explicit grant selection, INTERNET-only permission, and hardware decode policy. The Android app has no build evidence on this workstation because Gradle and the Android SDK/NDK are unavailable. No actual QR scan, sender connection, or media playback is claimed. Final hardware testing must install the APK on an Android TV, pair to a Windows sender, explicitly approve, receive H.264, verify fullscreen hardware decode, and exercise disconnect, reconnect, rejection, and revocation. Emulator compilation is build evidence only, not TV hardware evidence.
