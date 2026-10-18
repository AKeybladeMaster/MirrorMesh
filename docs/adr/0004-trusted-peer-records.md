# ADR-0004 — Trusted peer record and fingerprint contract

- **Decision date:** 2026-10-18
- **Status:** Proposed CORE contract; cross-track review pending

## Context

Pairing needs a stable trust record that can be persisted and revoked without putting private identity keys in an application database. The existing port exposed only a boolean trust check and revocation, which could not return peer identity, grants, or audit timestamps. A fingerprint must have an unambiguous human-readable representation for visual comparison.

## Decision

- Model a trusted peer in `mm-domain` with a canonical device ID, bounded display name, opaque bounded public-key bytes, a 32-byte fingerprint, explicit `view` / `send` / `annotate` grants, pairing time, optional last-seen time, and optional revocation time. These persisted timestamps use UTC Unix milliseconds, not a process monotonic clock.
- Encode a fingerprint as exactly 64 lowercase hexadecimal characters. The 32-byte representation is the SHA-256 fingerprint contract; identity adapters must compute and verify it over the agreed canonical public-key encoding before storage or approval.
- Keep private keys and OS keychain references out of `TrustedPeer` and the database row. Platform identity adapters own private-key generation, use, and deletion.
- The `TrustedPeerPort` contract loads, saves, lists, checks, and revokes records. Pairing application code may save only after explicit receiver approval; revoked peers cannot receive grants through the active-peer checks.
- The database schema remains SQLite as specified in `docs/protocol.md`; this ADR defines only the shared record/port boundary and does not select a Rust SQLite adapter.

## Consequences

- Domain code can enforce grant scope, field bounds, one-way revocation, and monotonic record timestamps without depending on a platform store.
- Identity proof, key algorithm/encoding, OS keychain adapters, SQLite migration execution, and transactional repository behavior remain separate implementation gates.
- Public-key bytes are intentionally opaque until the signing-key algorithm and wire encoding are selected. No unauthenticated first-use trust is permitted by this data contract.

## Security review notes

- A record's 32-byte fingerprint is not itself proof of possession. The pairing flow must verify a signed challenge and compare the presented key's computed fingerprint before calling `save_peer`.
- UI approval must scope grants explicitly; empty grants are rejected. Revocation must stop active sessions and remove any corresponding session credentials in the application layer.
