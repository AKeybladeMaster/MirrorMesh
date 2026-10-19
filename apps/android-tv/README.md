# MirrorMesh Android TV receiver shell

This is a separate native Android TV application target. It declares the Leanback launcher, touchscreen-free navigation, app icon, and TV banner. The D-pad-friendly standby screen still demonstrates local Accept/Reject/Disconnect states. The app now also contains a native receive-only WebRTC adapter and fullscreen `SurfaceViewRenderer` activity. The adapter accepts only an approved signaling channel, bounds SDP and ICE inputs, configures a receive-only transceiver with H.264-only codec preferences, and uses `HardwareVideoDecoderFactory` without a software decoder fallback.

The adapter is not yet wired to an Android TV pairing/signaling implementation or a Windows sender, so the current APK does not receive a real session. Only the ordinary `INTERNET` permission is requested; cleartext traffic is disabled and no local screen, camera, or microphone capture is requested. Accepting the local demo still does not start a stream. Native Android TV is the primary TV path; the browser receiver remains a compatibility fallback.

## Build

Install JDK 17 or newer, Android SDK platform 37, Build Tools 36.0.0 or newer, and Gradle 9.6.0. From this directory run `gradle :app:assembleDebug`. The output, when the required SDK is installed, is `app/build/outputs/apk/debug/app-debug.apk`. An emulator build is only a compile/UI smoke check; it does not prove hardware decoding or a real TV session.

GitHub Actions runs the Java state checks separately from an Ubuntu native APK build. The build job installs Android platform 37 and Build Tools 36.0.0, assembles the debug APK, and prints its SHA-256 digest. It does not publish or sign the APK.

## Local behavior checks

From the repository root run:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\test-demo-state.ps1
powershell.exe -ExecutionPolicy Bypass -File .\apps\android-tv\scripts\verify-shell.ps1
```

The JDK-only tests cover bounded H.264 SDP selection, approval/rejection, reconnect and revocation state, and the existing local demo. The native receive adapter has no Android build evidence on this workstation because Gradle and the Android SDK are unavailable. Final hardware testing must install the APK on an Android TV, pair to a trusted Windows sender, explicitly approve, receive H.264, verify fullscreen hardware decode, and exercise Disconnect, reconnect, rejection and revocation. Emulator compilation is only build evidence, not TV hardware evidence.
