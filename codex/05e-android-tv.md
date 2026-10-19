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

The native Android TV app now has D-pad QR pairing and explicit grant approval through its Rust JNI bridge. Task 38 source-wires the approved WSS connection to the existing receive-only libwebrtc adapter: Rust validates offers/ICE and encodes answers/ICE, JNI exposes bounded polling, and the first H.264 offer opens the fullscreen TV activity. The receive adapter uses a hardware-only decoder and H.264-only receive policy. Host Rust protocol/loopback tests and JDK policy/state plus Android XML/source checks pass. Android-only JNI compilation and `gradle :app:assembleDebug` remain unavailable because Android NDK/Clang and Gradle are absent; no APK, app-to-app WebRTC run, synthetic media, or physical TV evidence exists.

## Next implementation slice

Implement a synthetic H.264 sender against the shared Rust-backed authenticated signaling contract and verify sender-to-TV negotiation on an Android build runner. Keep WGC capture and hardware encoding behind their native adapters; add received-frame screenshot capture and persisted trusted-peer revocation after the media path runs.
