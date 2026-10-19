# ADR-0016: source-aware pairing runtime admission candidate

- **Status:** Proposed; security and cross-track API review pending.
- **Date:** 2026-10-18.
- **Scope:** Require per-source admission at the reusable pairing runtime boundary.

## Context

ADR-0015 added a bounded in-memory limiter, but a standalone primitive does not ensure callers run it before signature verification or preserve the source address through owner approval. The production signaling listener does not exist, so this candidate strengthens the Rust runtime boundary without claiming network integration.

## Decision candidate

- `PairingRuntime` owns a `PairingAttemptLimiter` by default and accepts an injected limiter for explicit policy configuration.
- Replace source-agnostic runtime proof verification with `verify_proof_from(source_ip, proof, now_monotonic_ms)`. It reserves an attempt before invoking the coordinator/verifier and returns cooldown or capacity errors without doing proof verification.
- Return `AdmittedPendingPairingApproval`, which carries the admitted source IP with the coordinator's pending approval. `approve` accepts this wrapper, persists only the explicitly approved grant subset, then clears that source's limiter history.
- Callers must use the actual remote socket IP and one monotonic clock domain. Keep the runtime alive for the listener lifetime; recreating it resets the in-memory source table. Do not expose an unthrottled runtime proof method to a network listener.
- If persistence succeeds but the limiter lock is poisoned during cleanup, report `AdmissionCleanup`; the peer has already been saved and later source admission remains fail-closed.

## Consequences and remaining evidence

The reusable runtime now places admission before proof verification and ties limiter reset to the source whose proof was approved. This is still an API candidate pending security and cross-track review. The listener, socket-IP extraction, TLS/WSS authentication, global limits, platform assembly, and owned-LAN tests remain open. Per-source limits can group legitimate peers behind NAT and do not prevent source-address rotation.

## Verification

Runtime tests cover rejection before signature verification and source-budget reset after explicit approval, alongside the existing fake-backed and real-verifier/SQLite tests. Commands and final results are recorded in [verification](../verification.md).
