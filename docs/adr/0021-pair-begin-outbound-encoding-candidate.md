# ADR-0021 — outbound `pair.begin` encoding candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed CORE protocol API; security and cross-track review pending

## Context

Task 15 connected a strict inbound `pair.begin` frame to `PairingRuntime`, but platform senders still had to duplicate the JSON envelope and canonical field encoding. `PairingProof` kept its validated fields private, so a caller could not build the same signed message consistently without reaching into implementation details.

## Decision

- Expose read-only accessors from `mm-application::PairingProof` for its receiver ID, nonce, peer ID, display name, public key, requested grants, and signature bytes. Keep its fields private and preserve construction validation.
- Add `mm-protocol::encode_pair_begin` with a private serializable view of the protocol `PairBegin` DTO. The public DTO cannot bypass validation through a derived serializer. The encoder accepts a typed message ID, a non-zero sequence, sent-at timestamp, and payload; it emits a v1 `pair.begin` envelope without a session ID.
- Run the same `validate_pair_begin` checks for both inbound parsing and outbound encoding. Reject zero sequence and frames over `MAX_ENVELOPE_BYTES`.
- Keep signing in the application/identity boundary. The encoder serializes a caller-provided signature and does not sign JSON bytes or claim to authenticate a TLS connection.

## Consequences

- Senders can construct the same candidate schema used by the inbound runtime without hand-formatting JSON or sending arbitrary-size key/signature values.
- JSON field encoding and Ed25519 profile remain proposed under [ADR-0020](0020-pair-begin-wire-encoding-candidate.md); cross-track/security review is still required before v1 freeze.
- This is a serialization boundary only. It adds no network listener, TLS, connection identity, QR UI, or trust persistence behavior.

## Verification

Protocol tests encode and parse a sessionless frame, and reject non-canonical payloads and sequence zero. Runtime/SQLite integration constructs a signed `PairingProof`, maps its read-only accessors to the protocol DTO, encodes it, and verifies the resulting bytes through the receiver path. See [verification](../verification.md).
