# ADR-0019: enforce approved grant scope before trusted-session start

- **Status:** Proposed; security and cross-track review pending.
- **Date:** 2026-10-18.
- **Scope:** CORE trusted-session runtime and its composed `MirrorMeshRuntime` facade.

## Context

`TrustedSessionRuntime` previously checked that a peer was active before starting its control transport, but it did not compare the operation's requested permissions with the grants persisted during explicit pairing approval. A trusted peer with a narrower grant set could therefore reach session setup without a runtime-level scope check.

## Decision candidate

- Require each `start_session` caller to provide the `TrustedPeerGrants` needed for that session operation.
- Load the persisted peer record and require it to remain active.
- Start no transport unless the requested grants are a subset of the peer's stored grants.
- Return `TrustedSessionError::GrantNotAuthorized` when the stored approval does not cover the requested scope.
- Apply the same argument and check through `MirrorMeshRuntime::start_session`.

The runtime cannot infer local sender/receiver direction from the current transport API. Callers must request the correct operation scope, and future capture, annotation, and media operations must enforce their own required grants at their respective boundaries. `start_session` continues to open a control transport only; this ADR does not authorize media or capture by itself.

## Consequences and open gates

- A denied scope is rejected before `TransportPort::start`; unit and real-verifier/SQLite composition tests verify that the fake transport remains unopened.
- Existing approved subsets continue to start, and SQLite revocation/reopen behavior remains covered.
- This is a public CORE API and security behavior candidate. Cross-track review must confirm grant meaning and caller responsibilities before protocol or platform integration.
- No protocol encoding, dependency, platform adapter, signaling listener, or media path changes.

## Verification

Focused and workspace Rust commands, toolchain results, and limits are recorded in [verification](../verification.md).
