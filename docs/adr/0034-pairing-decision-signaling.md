# ADR-0034: Pairing decision signaling

- **Status:** Proposed; requires CORE and security review
- **Date:** 2026-10-19
- **Scope:** Protocol v1 approval and rejection responses

## Context

The receiver's WSS pairing listener verified a signed `pair.begin` and waited for an explicit
local decision. Approval persisted trust, but the remote sender received no typed approval
response; rejection only closed the connection. That prevented the sender from learning the
accepted grants or reliably distinguishing rejection from network failure.

## Proposed decision

- Use the existing sessionless `pair.approve` and `pair.reject` message kinds; do not add a new
  protocol version or transport.
- Define `pair.approve.payload.peer_identity_proof` as the receiver's canonical lowercase
  SHA-256 fingerprint hex. The sender compares it to the fingerprint in the validated pairing QR.
  The pinned TLS 1.3 raw-public-key handshake authenticates the same key on the live channel.
- Require `accepted_grants` to be a non-empty unique subset of requested grants, as already
  enforced by `PairingRuntime::approve` before the response is built.
- Send `pair.approve` only after explicit approval has been persisted. The connection becomes
  available for post-approval control messages only after the response is sent successfully.
- Send a bounded `pair.reject` reason and close without writing a trusted-peer record when the
  user rejects the pending request.
- Generate response message IDs from the operating-system random source. Use independent sequence
  tracking for the future sender/receiver directions.

## Failure behavior and verification

If trust persistence succeeds but the approval response cannot be delivered, approval returns a
signaling error and drops the connection; the persisted trust remains because the user approved
it. The UI must surface that delivery failed and offer disconnect/revoke. A two-process local test
covers pinned WSS approval plus v1 `pair.approve`, rejection plus no persistence, expiry plus no
persistence, and post-approval offer/answer exchange. No platform app uses these responses yet, and
no WebRTC or hardware behavior is claimed.
