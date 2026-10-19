# Task 48 — Explicit Windows-to-TV disconnect

**Owner:** CORE + Windows + Android TV; scope is existing protocol-v1 stop handling. No wire schema/version change.

## Patch plan

- Add a bounded shared encoder for the existing `session.stop` payload and a test for sequence and reason validation.
- Add a Windows worker-owned stop command with a bounded send acknowledgement; expose it through FFI, the Windows method channel, and the Dart pairing/session controller.
- Stop local native capture and peer resources even if signaling fails. Send `session.stop` before cancelling the approved pairing channel.
- Treat remote stop as a normal TV `STOPPED` transition and close receiver resources; test lifecycle behavior.
- Update status and verification records. See proposed [ADR-0048](../docs/adr/0048-windows-tv-session-stop.md).

## Verification

Planned local checks: Rust workspace format/tests/Clippy, Windows-target adapter/FFI tests and DLL build, Flutter focused analyze/test if the SDK is available, and direct JDK 17+ TV state tests. No hardware or media result is claimed.
