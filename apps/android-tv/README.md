# MirrorMesh Android TV receiver shell

This is a separate native Android TV application target. It declares the Leanback launcher, touchscreen-free navigation, app icon, and TV banner. Its D-pad-friendly local demo lets a tester show a request from a fictional sender, explicitly Accept or Reject it, and Disconnect. The request and session labels are local UI state only; they do not discover a device or pair with a sender.

No network, capture, microphone, or media permissions are requested. The shell does not implement QR pairing, authenticated signaling, WebRTC, MediaCodec, or a video surface; those remain gated on the shared v1 contract and ADR-0002. Accepting the local demo does not start a stream. Native Android TV is the primary TV path; the browser receiver is a compatibility fallback.

## Build

Install JDK 17 or newer, Android SDK platform 37, Build Tools 36.0.0 or newer, and Gradle 9.6.0. From this directory run `gradle :app:assembleDebug`. The output, when the required SDK is installed, is `app/build/outputs/apk/debug/app-debug.apk`. An emulator build is only a compile/UI smoke check; it does not prove hardware decoding or a real TV session.

GitHub Actions runs the Java state checks separately from an Ubuntu native APK build. The build job installs Android platform 37 and Build Tools 36.0.0, assembles the debug APK, and prints its SHA-256 digest. It does not publish or sign the APK.

## Local behavior checks

From the repository root run:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\test-demo-state.ps1
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\verify-shell.ps1
```

On an Android TV device, install the separately built APK and use the remote to show a local request, confirm the state stays pending, press Reject, show another request, press Accept, then Disconnect. Verify the screen always labels this as a local demonstration and no media appears. These steps verify only the mock approval UX; actual sender discovery, pairing, capture, and video require separate implementation and hardware evidence.
