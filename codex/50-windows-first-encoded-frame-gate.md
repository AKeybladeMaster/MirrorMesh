# Task 50 — Gate Windows session start on its first encoded video frame

**Owner:** WIN. **Scope:** Windows Flutter sender controller/UI and Flutter CI. **Requirements supported:** M-02 (sender-side H.264 flow), S-05 (sender diagnostics); this task completes neither end-to-end requirement.

## Patch plan

- Distinguish WebRTC transport connectivity from the sender's local encoded-video report.
- Send the existing v1 `session.start` only after the connection is ready and outbound WebRTC stats report at least one encoded frame.
- Wait at most 15 seconds after transport/session readiness for that first frame. On timeout, stop local capture and close the peer and pairing channel without sending `session.start`.
- Cover delayed first-frame stats and timeout cleanup in `windows_sender_session_test.dart`; keep video-frame data in native WebRTC.
- Run the Flutter checks on `task_*` pushes as well as `main` so each task branch receives CI evidence.
- Do not change the protocol schema, Rust FFI, signaling, or Android TV implementation.

## Verification

Commands from `apps/mirrormesh_flutter/`:

```powershell
dart format --output=none --set-exit-if-changed lib/windows_sender_session.dart lib/app.dart test/windows_sender_session_test.dart
flutter analyze
flutter test test/windows_sender_session_test.dart
```

Expected evidence is formatted/analyzed Dart source and passing controller tests showing that transport connection alone does not send `session.start`, the first encoded-frame report sends it once, and the bounded timeout releases capture/peer resources and cancels pairing. The Flutter workflow additionally builds its Windows/Linux/Android/iOS shell matrix on task branches.

## Risk flags and limits

- WebRTC outbound statistics may omit `framesEncoded`; this task deliberately fails closed after the timeout in that case.
- The stats are sender-side evidence only. They do not establish RTP delivery, Android TV decode/render, WGC capture, hardware encoder selection, or hardware behavior.
- Flutter/Dart are unavailable on the current workstation. The source tests are not considered executed until the hosted workflow result is inspected.
- The 15-second timeout begins only after the peer is connected and the session ID exists; it does not time out user pairing or initial offer/answer negotiation.

## Status

Source is implemented. `BUILD_VERIFIED`, `AUTOMATED_INTEGRATION_VERIFIED`, and `HARDWARE_VERIFIED` remain pending. See [verification ledger](../docs/verification.md#task-50-verification--windows-first-encoded-frame-gate) and [ADR-0047](../docs/adr/0047-windows-session-start-control.md).
