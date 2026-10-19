# ADR-0048 — Explicit Windows-to-TV session stop

- **Status:** Proposed; CORE, Windows, and Android TV review pending
- **Date:** 2026-10-19
- **Requirements:** M-02, M-06, M-07, S-04

## Context

The Windows sender closes local capture and its approved WSS pairing channel when the user disconnects. The receiver currently treats a remote `session.stop` as a transport failure. Closing the channel does eventually release TV resources, but it does not give the TV an explicit normal-stop transition.

## Decision

Use the existing protocol-v1 `session.stop` message for an explicit Windows sender disconnect. The protocol encoder validates a non-empty, control-free reason of at most 128 UTF-8 bytes. The Windows worker remains the sole owner of the approved WSS connection, uses the active session identifier and next outbound sequence, sends the stop, and reports whether the write succeeded within a two-second bound. The Flutter controller stops its native WebRTC track and peer first, sends the stop while the approved control channel remains open, then closes pairing. A signaling failure never prevents local capture and peer cleanup.

Android TV treats this message as a normal disconnect: transition the active receiver state to `STOPPED`, show a sender-disconnected status, and close the peer, renderer, signaling listener, and decoder resources. Reconnection requires a new session negotiation.

## Consequences

- No protocol version or message schema changes; `session.stop` was already part of v1.
- No pixel or encoded-frame data crosses Dart or FFI.
- The worker acknowledgement confirms a local signaling write, not remote receipt. TV disconnection and cleanup still depend on the receiver processing the message or detecting channel closure.
- The two-process WSS test verifies envelope/session/sequence handling. It does not run either application, WebRTC, RTP, media decode, or hardware.

## Verification

Rust protocol, Windows adapter/FFI, and two-process signaling tests plus direct JDK TV lifecycle tests are required. The Windows Flutter bundle and Android TV APK need their respective platform SDKs; hardware verification is deferred to final user testing.
