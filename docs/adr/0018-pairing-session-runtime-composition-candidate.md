# ADR-0018: pairing and trusted-session runtime composition candidate

- **Status:** Proposed; security and cross-track review pending.
- **Date:** 2026-10-18.
- **Scope:** Provide one CORE runtime facade over pairing and trusted-session lifecycle.

## Context

`PairingRuntime` and `TrustedSessionRuntime` were independently composable, but platform startup could accidentally construct them with different trusted-peer stores or identity lifetimes. That would allow explicit pairing approval to persist in one store while session startup checked another.

## Decision candidate

- Add `MirrorMeshRuntime`, which constructs both components over the same `LocalPairingIdentityPort`, `PairingIdentityPort`, `TrustedPeerPort`, and one `TransportPort` instance.
- Expose the consent-gated invitation, source-aware proof verification, explicit grant approval, trusted transport start/stop, and peer revoke operations through the facade.
- Recreate the facade after local identity rotation or deletion. Serialize mutating calls through one application runtime owner.
- `start_session` starts a trusted peer's control transport only and requires a caller-supplied operation grant set, checked against persisted approval as described in proposed [ADR-0019](0019-trusted-session-grant-scope-candidate.md). It does not create an offer, authorize capture, or establish media; future signaling, capture, annotation, and media boundaries must apply their operation-specific grants and capture consent.

## Consequences and open gates

- A real Ed25519 proof + SQLite integration test covers proof verification, explicit approval, control-transport start, revocation/close, and denial after runtime recreation.
- The transport in that test is fake. No platform app, signaling listener, network connection, session offer, capture, codec, renderer, or hardware path uses the facade.
- The facade is a public CORE API candidate pending security/cross-track review. It does not freeze the pairing wire encoding or TLS/WSS behavior.

## Verification

Commands and results are recorded in [verification](../verification.md). The focused test combines `Ed25519PairingIdentity`, `SqliteTrustedPeers`, and a fake `TransportPort`.
