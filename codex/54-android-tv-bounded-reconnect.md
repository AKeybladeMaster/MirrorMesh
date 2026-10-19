# Task 54 — Bound Android TV receiver reconnect

**Owner:** TV. **Scope:** Native Android TV WebRTC receiver lifecycle and its JDK/instrumentation gates. **Requirements supported:** M-02 (receiver session lifecycle), S-04 (bounded recovery and cleanup). This task does not complete either requirement end to end.

## Patch plan

- Give an interrupted TV WebRTC peer a single 10-second recovery window, matching the Windows sender's reconnect limit.
- Cancel the deadline when ICE reconnects and close the watchdog when the receiver closes.
- Move the TV state back to `CONNECTING` after transport recovery and to `STREAMING` only after a decoded frame arrives.
- Ignore callbacks from a peer connection that was replaced during renegotiation.
- Add deterministic JDK tests for timeout, recovery, and disposal; keep all frame data inside native WebRTC.
- Do not change WSS, protocol messages, grants, or pairing behavior.

## Verification

From the repository root:

```powershell
powershell.exe -File .\apps\android-tv\scripts\test-demo-state.ps1
powershell.exe -File .\apps\android-tv\scripts\verify-shell.ps1
cargo fmt --all -- --check
git diff --check
```

From `apps/android-tv/`:

```powershell
gradle :app:assembleDebug
gradle :app:connectedDebugAndroidTest -PmmAndroidAbi=x86_64
```

The JDK tests must prove a timeout fires once, recovery cancels it and permits a later attempt, receiver close cancels the timer, and the state leaves `RECONNECTING` after transport recovery. Android instrumentation/build evidence is still required to validate the adapter against libwebrtc.

## Limits

The timeout is a source-level policy; no emulator, APK, Android TV, or real peer recovery has been verified locally. It does not force an ICE restart. The sender and receiver can still fail to reconnect if the underlying network path cannot recover. Rust-authenticated Windows-to-TV media, WGC, hardware encoding, and physical MediaCodec behavior remain open.

## Status

Source is implemented. `CODE_COMPLETE` applies to this bounded receiver reconnect slice after local tests pass. `BUILD_VERIFIED`, `AUTOMATED_INTEGRATION_VERIFIED`, and `HARDWARE_VERIFIED` remain separate gates. See [verification](../docs/verification.md#task-54-verification--android-tv-bounded-reconnect) and [Android TV workstream](05e-android-tv.md).
