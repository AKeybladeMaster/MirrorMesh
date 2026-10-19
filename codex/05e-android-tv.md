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

The local approval shell has five D-pad-focusable controls, explicit accept/reject/disconnect wiring, INTERNET-only permission, disabled cleartext, and a polite accessibility live region. Task 30 adds an Android native receive-only libwebrtc adapter with a hardware-only H.264 decoder, H.264-only receive codec preferences, bounded SDP/ICE handling, a fullscreen native surface, diagnostics, and cleanup. The approved-channel contract is not yet implemented or connected to the Rust pairing runtime or standby approval UI, so the receiver is dormant and no sender-to-TV path is claimed. JDK policy/state tests and source-boundary checks pass. `gradle :app:assembleDebug` cannot run on this host because Gradle is absent; no APK or physical TV evidence exists.

## Next implementation slice

Implement the Windows synthetic H.264 sender against the shared Rust-backed authenticated signaling/session contract, then wire explicit TV approval into `TvApprovedSessionLauncher`. Keep WGC capture and hardware encoding behind their native adapters, and add a received-frame screenshot path after the native surface is integrated.
