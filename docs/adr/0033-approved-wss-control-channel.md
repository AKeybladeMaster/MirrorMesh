# ADR-0033: Approved WSS control channel

- **Status:** Proposed; requires CORE and security review
- **Date:** 2026-10-19
- **Scope:** Rust native signaling adapter API

## Context

The bounded WSS listener authenticates a QR-pinned TLS peer, verifies its `pair.begin` proof,
and returns a connection only after explicit approval. The returned connection still exposed its
raw WebSocket, allowing callers to bypass the shared v1 parser, message-kind boundary, and sequence
checks on the approved channel.

## Proposed decision

- Keep the existing protocol v1 envelopes and pairing listener unchanged on the wire.
- Replace raw public WebSocket access with typed send/receive methods on `PairingConnection`.
- Parse each message with `mm_protocol::parse_message`; reject `device.hello` and pairing setup
  messages after approval. Permit the existing session, annotation, and revocation message kinds.
- Track increasing sequence numbers independently for inbound and outbound directions, per
  approved peer and session. Track `pair.revoke` sequence numbers per direction as well.
- Bound tracked sessions to eight per connection. The caller must call `forget_session` after a
  session is fully closed to release tracker capacity.
- Keep grant enforcement, session-state authorization, and revocation persistence in the existing
  `TrustedSessionRuntime` and application owner. This adapter validates transport envelopes; it
  does not authorize the operation described by a valid envelope.
- Keep blocking reads on a dedicated worker and reject binary WebSocket messages as protocol input.

## Verification and limits

The Rust adapter unit tests cover message-kind filtering, malformed and oversized frames,
independent per-session sequences, replay rejection, and bounded listener capacity. A two-process
loopback test exercises pinned TLS/WSS pairing, explicit approval, then a shared-protocol offer and
answer exchange. No platform app invokes this API yet. It does not establish a WebRTC peer, carry
media, or verify a physical device. The public API remains a candidate pending cross-track and
security review.
