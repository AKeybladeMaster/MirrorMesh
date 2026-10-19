# TASK 05e — TV PRIMARY: native Android TV / Google TV receiver (P2-TV)

**Owner:** TV; paths `apps/android-tv/` and TV-specific native integration. Start Android TV app/remote-navigation scaffold concurrently with all native tracks. Real video requires shared v1 fixtures and ADR-0002.

1. Dedicated installed receiver APK; Kotlin TV-focused 10-foot interface (or clearly justified Flutter TV shell), Android TV manifest/launcher/banner and D-pad/Back accessibility.
2. `MediaCodec` H.264 decoder to native `Surface`/SurfaceView. Native or Rust WebRTC transport integrates the **same authenticated signaling and identity contract** as desktop.
3. User-triggered discoverable receiver / pairing QR, visible sender name, explicit `Accept`, `Reject`, `Disconnect`, revoke and session state. QR must be readable at TV distance; do not auto-accept.
4. Fullscreen rendered media, pause/stop on remote; safe handling of resolution changes and codec unsupported states. Receiver annotation toolbar may be initially controlled via remote or paired phone; must keep permission scope.
5. Prove fixed synthetic sender→TV before coupling to Windows, Linux, Android and iOS senders. Capture on-phone screenshots for demo/portfolio only with consent.
6. Test actual Android TV hardware and Google TV build where available; emulator is compile/UI smoke only. Produce a separately named APK/AAB, not the phone app APK.

**Acceptance:** verified native Android TV `known sender→TV` with remote Accept/Stop, bounded decoder lifecycle, TV launcher and APK install. This path is the primary TV feature; web fallback cannot substitute for missing native-TV proof.

## Current source evidence (2026-10-18)

The native Android TV app now has D-pad QR pairing and explicit grant approval through its Rust JNI bridge. Task 38 source-wires the approved WSS connection to the existing receive-only libwebrtc adapter: Rust validates offers/ICE and encodes answers/ICE, JNI exposes bounded polling, and the first H.264 offer opens the fullscreen TV activity. The receive adapter uses a hardware-only decoder and H.264-only receive policy. Task 44 adds a D-pad screenshot action that captures only the next decoded frame after an explicit tap, bounds the I420-to-NV21 copy, and writes a JPEG to MediaStore or app-specific storage without broad permissions. JDK plane-conversion/state tests pass. Android-only JNI/APK compilation and screenshot writing remain unverified because Android NDK/Clang and Gradle are absent; no app-to-app WebRTC run, synthetic media, or physical TV evidence exists.

Task 47 routes the existing bounded `session.start` source label through the Rust-approved channel. The TV verifies that the notice matches its active session and reports that the sender says capture started. Direct JDK tests cover valid, missing, oversized, and control-bearing source IDs. This does not prove frame delivery or decoding. The shell PowerShell check is blocked by the workstation's execution policy; Android Gradle/APK build remains unavailable.

Task 48 handles the existing `session.stop` as a normal disconnect: the receiver state becomes `STOPPED`, the UI reports sender disconnect, and the native peer, renderer, decoder, and approved channel close. The two-process WSS test checks the stop sequence. Direct JDK lifecycle testing is run outside the blocked PowerShell script; no Android APK, live WebRTC peer, decoded frame, or hardware result is claimed.

Task 49 adds an Android instrumentation loopback test. A test-only native I420 capturer feeds one libwebrtc H.264 sender peer; a fake approved channel exchanges SDP/ICE with the production `TvWebRtcReceiver`, configured through its package-private seam with Android's software-only `MediaCodec` decoder factory. The test asserts ICE connection, decoded-frame callback, and one explicit received-frame JPEG capture. It skips if the pinned libwebrtc lacks a software H.264 encoder or the emulator lacks a software H.264 decoder. It does not use production WSS or a Windows sender, and it does not verify MediaCodec hardware. Gradle, SDK, and an emulator are unavailable on the current workstation, so source is authored but build/integration evidence is pending. CI runs `gradle :app:connectedDebugAndroidTest -PmmAndroidAbi=x86_64` on an API 35 emulator while the installable receiver APK remains ARM64.

Task 52 keeps the same native synthetic media and receiver, but routes test signaling through the production `RustApprovedSignalingChannel`. A test-only native bridge supplies its JNI calls and exercises the Java offer/ICE mapping, answer/ICE writes, decoded frame, and explicit screenshot path. This still does not run Rust authentication or Windows interoperability. The emulator test and ARM64 APK build remain source-only until the Android workflow runs.

Task 53 adds instrumentation tests for remote `session.stop`, pairing revocation, and malformed-control fail-closed cleanup through the same production Java channel. The test-only bridge verifies listener notification, one native close, and that the closed adapter no longer reports an approved connection. It does not verify Rust WSS authentication or Windows interoperability; Gradle/SDK/emulator evidence is pending.

Task 54 bounds receiver-side ICE recovery to ten seconds, cancels the watchdog when the same peer reconnects, restores the connection state only after a decoded frame arrives, and ignores callbacks from replaced peers. Deterministic JDK tests cover timeout and cleanup. This is receiver lifecycle source; it does not establish real network recovery or Windows-to-TV media delivery.

## Next implementation slice

Run the Task 49/52/53 emulator tests and build the ARM64 APK. Then integrate the existing synthetic H.264 fixture with real Rust WSS between the Windows sender and TV receiver; continue with Windows Graphics Capture and hardware encoder integration. Keep physical MediaCodec validation for final user testing.
