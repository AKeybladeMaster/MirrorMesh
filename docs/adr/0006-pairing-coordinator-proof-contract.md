# ADR-0006 — Pairing coordinator and signed proof contract

- **Decision date:** 2026-10-18
- **Status:** Proposed CORE application contract; security and cross-track review pending

## Context

Task 03 needs one shared application flow between the QR parser, one-time nonce registry, identity provider, explicit owner consent, and trusted-peer store. The existing primitives had no orchestration contract, so platform UIs and future network adapters could otherwise make different decisions about replay, proof contents, and when trust is persisted.

## Decision

- Add `mm-application::PairingCoordinator` as an OS-independent coordinator over `PairingIdentityPort` and `TrustedPeerPort`. It parses only numeric endpoints on caller-approved ports and creates a canonical QR with a 120-second wall-clock expiry and the registry's monotonic deadline.
- A `PairingProof` carries the receiver ID, one-time nonce, sender ID, bounded display name, public key, requested non-empty grants, and a bounded signature. The signed bytes are `MirrorMesh.PairingProof.v1\0`, followed by receiver ID bytes, nonce bytes, sender ID bytes, a big-endian `u32` UTF-8 name byte length, name bytes, and one grant-bits byte (`view=bit 0`, `send=bit 1`, `annotate=bit 2`).
- The coordinator rejects wrong receiver IDs, self-pairing, and unknown/expired nonces before calling the identity provider. It verifies signature possession, computes the public-key fingerprint through the same identity port, and consumes the nonce exactly once before returning a pending approval. That pending approval is bound to the receiver coordinator that verified it and cannot be persisted through a different receiver instance.
- Returning a pending approval does not persist trust. The owner-facing UI must explicitly approve a non-empty subset of requested grants while the original monotonic nonce deadline remains live. Only then does `approve` create and persist a `TrustedPeer` record.
- The proof byte encoding is an application contract, not a frozen network schema. The `pair.begin` payload shape and canonical candidate encodings are recorded in proposed [ADR-0020](0020-pair-begin-wire-encoding-candidate.md); its algorithm profile, authenticated TLS behavior, and cross-track compatibility must be reviewed before protocol v1 is frozen.

## Consequences

- Platform UI and network code can share one deny-by-default consent sequence without depending on an OS SDK.
- Unit tests use fake identity and storage ports. A stateless Ed25519 verifier candidate exists in `mm-adapter-identity`, but it is not wired into the coordinator. There is no QR UI, transport/listener, rate limiter, or durable trusted-peer adapter; these tests do not prove end-to-end pairing.
- The coordinator consumes a nonce only after successful signature and fingerprint operations. A failed proof leaves it available until expiry; a network adapter must rate-limit attempts without making arbitrary remote requests able to silently approve or persist trust.
- Wall-clock QR expiry and process-local monotonic approval expiry are separate checks. A host restart invalidates outstanding in-memory invitations.
- Proposed ADR-0007 selects raw Ed25519 public-key bytes for the 32-byte SHA-256 fingerprint, but the encoding and strict-verification profile are not frozen until cross-track/security review.

## Verification

`mm-application` tests cover QR round-trip, domain-separated and scope-bound proof bytes, signature rejection, nonce replay/unknown nonce, the explicit approval persistence boundary, scope reduction, original-deadline expiry, proof size bounds, and receiver-bound pending approval. These are contract tests with fakes only.
