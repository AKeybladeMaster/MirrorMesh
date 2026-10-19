# ADR-0022 — sender-side pairing proof creation candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed CORE application API; security and cross-track review pending

## Context

Tasks 15–16 connected the strict inbound `pair.begin` parser, the outbound encoder, and receiver-side verification. A sender still had to manually construct the application `PairingProof`, compute its canonical signing bytes, and call the local signing port in the correct order.

## Decision

- Add `PairingProof::create_signed` using `LocalPairingIdentityPort`. It obtains the current public key, validates proof fields, signs `PairingProof::signing_message`, and returns a validated proof containing the key and signature.
- Reject self-pairing before invoking the identity provider. Return an explicit error for signer failures or invalid proof data; never return a partially signed proof.
- Keep QR parsing, receiver fingerprint comparison, QR expiry enforcement, device-ID ownership, wire encoding, and transport authentication outside this helper. Callers must supply values from the validated QR and current local device identity.
- Do not persist trust or show an approval result on the sender. Receiver trust remains behind its explicit approval operation.

## Consequences

- Application code can create a possession proof without duplicating canonical signing-byte construction or exporting key material.
- The real-verifier/SQLite integration can exercise sender creation → shared `pair.begin` encoding → source-aware receiver verification.
- A key rotation between public-key retrieval and signing may make the proof fail verification; retry with the current identity. This fails closed and does not persist trust.
- This API does not authenticate the receiver or the signaling transport. QR fingerprint verification and authenticated TLS remain separate required work.

## Verification

Fake-identity tests verify the signer receives the exact canonical message, self-pairing is rejected before signer access, and signing errors return no proof. SQLite integration uses a real deterministic test-only Ed25519 identity to create and encode a proof, then verifies it through the receiver runtime; persistence still requires explicit approval. See [verification](../verification.md).
