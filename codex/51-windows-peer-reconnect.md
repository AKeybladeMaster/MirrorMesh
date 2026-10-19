# Task 51 — Bound Windows sender peer reconnection

**Owner:** WIN. **Scope:** Windows Flutter sender state, cleanup, UI, and focused controller tests. **Requirements supported:** M-02, S-04, and S-05; this task completes none of them end to end.

## Patch plan

- Add a `reconnecting` sender state for a transient WebRTC disconnect after the peer has connected.
- Keep the same peer and consented capture for a bounded 10-second recovery period; let libwebrtc reconnect without recreating the peer, restarting capture, or repeating `session.start`.
- On recovery, return to `transportConnected` until a frame is locally reported encoded, or to sender-reported `streaming` if encoding has already occurred.
- On timeout, close native capture and peer resources, attempt the existing bounded `session.stop`, cancel the approved pairing channel, and report failure. Deduplicate concurrent failure cleanup.
- Cover same-peer recovery, one-time session start, timeout stop ordering, and local resource cleanup with mocked controller tests.
- Record the public Dart state contract in proposed [ADR-0051](../docs/adr/0051-windows-peer-reconnect.md).

## Verification

Commands from `apps/mirrormesh_flutter/`:

```powershell
dart format --output=none --set-exit-if-changed lib/windows_sender_session.dart lib/app.dart test/windows_sender_session_test.dart
flutter analyze
flutter test test/windows_sender_session_test.dart
```

Expected evidence is passing state-transition tests showing that the existing peer recovers within the grace period without a second `session.start`, and that timeout attempts `session.stop` before pairing cancellation while releasing local resources. The tests do not establish network reconnection or Android TV frame recovery.

## Limits

- `disconnected` can be temporary. The 10-second grace period is configurable in the controller and does not trigger an ICE restart or new capture request.
- The current workstation lacks Flutter/Dart, so source tests and analysis are not considered run until a hosted workflow result is available.
- No protocol schema or raw-media API change is made. WGC, hardware encoder use, authenticated app-to-TV RTP, receiver-frame evidence, and physical hardware remain open.

## Status

Source implemented; `BUILD_VERIFIED`, `AUTOMATED_INTEGRATION_VERIFIED`, and `HARDWARE_VERIFIED` remain pending. See the [verification ledger](../docs/verification.md) and [Windows sender workstream](05a-windows.md).
