# ADR-0029: Owned pairing runtime state

**Status:** Proposed implementation; cross-track and security review pending

**Date:** 2026-10-19
**Requirements:** M-06, M-07, M-10

## Context

`PairingRuntime` held references to the identity and trusted-peer ports. That made it awkward for an application owner to construct a runtime in one scope and retain it for a listener's lifetime. Rebuilding the runtime for each callback would also recreate its in-memory invitation nonce registry and per-source attempt limiter. `MirrorMeshRuntime` additionally needs pairing and trusted-session operations to address the same trusted-peer store.

## Decision

- `PairingCoordinator`, `PairingRuntime`, and `TrustedSessionRuntime` own their ports through `Arc` handles instead of borrowing them.
- `MirrorMeshRuntime` receives one identity and peer-store `Arc`; it clones the peer-store handle so pairing and trusted-session operations use the same store instance.
- An application owner keeps one runtime alive across listener callbacks and serializes its mutating operations. Recreate it after local identity rotation or deletion.
- `Arc` provides shared ownership only. It does not add `Send`/`Sync` bounds or make an underlying port thread-safe; callers must still follow each adapter's concurrency contract.
- Identity creation, rotation, and deletion remain explicit operations. Constructing a runtime reads the public identity key but does not create or rotate an identity.

## Consequences and limits

- A runtime can outlive the function that assembled it, which preserves nonce and admission state for its lifetime.
- The constructor signatures are a public Rust API change: callers now pass `Arc<I>` and `Arc<P>`, and runtime type annotations no longer carry a borrow lifetime.
- Callers that need to inspect a store can retain an `Arc::clone`. Reconstructing a runtime with a different store handle does not share in-memory limiter or nonce state, even if the two adapters point at the same database.
- Process restart still clears in-memory nonces and admission history. Persisted trusted peers and revocations continue to come from the trusted-peer adapter.
- This does not assemble a platform runtime, start a listener, or validate concurrency on Android or another device.

## Verification

The new owned-runtime regression test creates the runtime through a helper that returns it after its constructor scope ends, issues an invitation, verifies a signed proof, confirms trust is absent before approval, then confirms the same shared store contains the peer after approval. Existing SQLite and WSS integration tests exercise the migrated constructors.

## Review

Review the public API change and the state-lifetime assumptions with CORE, Android, and signaling owners. Confirm each app owner retains one runtime instance while the listener is active and serializes mutations; confirm identity rotation tears down and recreates dependent runtime state.
