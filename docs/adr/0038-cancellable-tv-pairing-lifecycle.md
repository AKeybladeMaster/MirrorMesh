# ADR-0038 - Cancellable Android TV pairing lifecycle

- **Date:** 2026-10-19
- **Status:** Proposed; Android target build and activity integration remain open

## Decision

The shared WSS pairing listener polls a nonblocking TCP listener and accepts an optional
cancellation token. The Android TV pairing service owns a stop flag, and its JNI bridge exposes a
shutdown operation that stops accepting, drops an undecided request, and closes every approved
control channel. JNI serializes request handoff and approval with shutdown so a request or channel
cannot be registered after shutdown drains service state.

The existing listener API remains available and retains its wait-until-request behavior. A
cancellable overload is used by the Android TV service. An accepted TLS/WebSocket handshake keeps
the existing absolute ten-second deadline; shutdown during that handshake is bounded but may take
until that deadline to return.

## Consequences and limits

Idle accept workers can now exit without closing a process or leaving the listener service
registered. Dropped pending requests close with their owned connection; approved channels receive
best-effort WebSocket close frames and are removed from the registry. Host tests cover cancellation
and the shared pairing flow. The Android-only JNI entrypoint is not target-build verified because
the local Android NDK is unavailable. The activity still needs to call shutdown from its lifecycle.
