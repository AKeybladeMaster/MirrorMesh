# ADR-0017: trusted-session revocation runtime candidate

- **Status:** Proposed; security and cross-track review pending.
- **Date:** 2026-10-18.
- **Scope:** Gate session transport starts on persisted trust and close tracked transports after peer revocation.

## Context

The trusted-peer store persists revocation, and the generic session manager tracks session IDs, but no runtime connected a peer identity to its active control transports. A store-only revoke therefore could not ask the runtime to close sessions or prevent another session start in the current app process.

## Decision candidate

- `TrustedSessionRuntime` owns a `TrustedPeerPort`, a `TransportPort`, and the mapping from tracked session IDs to peer IDs. Callers serialize its mutating operations on one runtime owner.
- `start_session` checks the stored peer is active before asking the transport to start. It rejects peers already revoked in this runtime and duplicate tracked session IDs.
- If transport startup fails, the runtime attempts close immediately. If cleanup also fails, the session remains tracked so the caller can retry `stop_session`.
- `revoke_peer` caches the revoked ID before storage access, attempts to persist the revocation, then attempts to close every tracked session for that peer in stable session-ID order even if persistence failed. Other peers' sessions remain open. Failed closes stay tracked and are returned by session ID for retry.
- A stored revoked peer is rejected after process restart through the trusted-peer port's active-trust check. The in-memory revoked set also blocks session start if a later store read would otherwise report the peer active.

## Consequences and open gates

- Fake-backed runtime tests cover trusted start gating, peer-scoped closure, reconnection rejection, persistence failure, transport-start cleanup, and retryable close failure.
- The runtime closes only `TransportPort` sessions it started and still tracks. It does not stop OS capture, codecs, renderers, or the native WebRTC engine unless the concrete adapter's `close` implementation owns those resources; no platform adapter is wired to this runtime yet.
- A persistence error still blocks new sessions for the runtime lifetime and closes the tracked transports, but the stored peer may be active after process restart. The caller must surface the failure and retry persistence; no durable revocation is claimed when storage fails.
- `TrustedSessionRuntime` and the public port composition are candidates pending security and cross-track review. No signaling listener, socket, peer, or hardware test ran.

## Verification

The `mm-runtime` focused suite includes tests for successful and failed revocation closure plus explicit retry. Exact commands and results are recorded in [verification](../verification.md).
