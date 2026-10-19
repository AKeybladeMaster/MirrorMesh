# ADR-0035: QR-pinned WSS pairing client

- **Status:** Proposed; requires CORE and security review
- **Date:** 2026-10-19
- **Scope:** Rust sender-side signaling adapter

## Context

Tasks 31–32 implemented the receiver's approved WSS control channel and explicit pairing decision,
but the sender side still constructed TLS, WebSocket, proof, and control frames manually in the
integration test. Windows had no shared client entry point to consume a receiver QR invitation.

## Proposed decision

- Add `WssPairingClient::begin_pairing` in `mm-adapter-signaling`. It strictly parses the QR with
  caller-provided current UTC time and approved port list, uses only its numeric IP endpoint, pins
  native TLS 1.3 raw-public-key authentication to the QR fingerprint, and applies the same absolute
  ten-second connection/initial-handshake deadline.
- Create the existing signed `pair.begin` through `PairingProof::create_signed` and
  `mm-protocol::encode_pair_begin`, with an OS-random message ID.
- Return a pending client that accepts only one sessionless v1 `pair.approve` or `pair.reject`.
  Verify the approval fingerprint against the QR and ensure accepted grants are a subset of what
  the sender requested before returning the shared typed `PairingConnection`.
- Keep trusted-peer persistence, UI approval prompts, capture consent, session/grant authorization,
  media, and app lifecycle in their existing application/runtime owners. This adapter does not
  persist the receiver in a sender's trust store.

## Verification and limits

A two-process localhost test now runs both ends through their shared pairing/control APIs and
covers approval, rejection, expiry, and offer/answer control exchange. Unit tests reject QR-pin
mismatch and grant escalation. The implementation has host Rust evidence only; the Windows Flutter
app and native Android TV app are not wired to it, and no WebRTC or media flow is claimed.
