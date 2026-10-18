# ADR-0015: bounded pairing attempt limiter candidate

- **Status:** Proposed; security review pending.
- **Date:** 2026-10-18.
- **Scope:** In-memory admission control before pairing proof verification.

## Context

The pairing proof verifier is exposed to untrusted network input once an authenticated signaling listener exists. That listener needs a bounded way to reduce repeated proof work from one source while keeping valid trust decisions in the existing pairing coordinator.

## Decision candidate

- Add `mm-security::PairingAttemptLimiter`, keyed by source IP address. Normalize IPv4-mapped IPv6 addresses to the equivalent IPv4 key.
- The caller supplies monotonic milliseconds from one clock domain and calls `begin_attempt` before proof verification. Admission and attempt counting happen under one mutex, so concurrent requests from one IP cannot all pass the same remaining slot.
- Default policy admits five attempts per source, expires partial history after 60 seconds, starts a 30-second cooldown after the limit, and tracks at most 1,024 source addresses. A successful pairing clears that source's history.
- The attempt that reaches the limit is admitted for verification and starts the cooldown immediately; subsequent attempts receive `CoolingDown` until the deadline. If the source table is full, unknown sources receive `CapacityLimited` until an entry expires or is cleared by success.
- A lock error must be treated as rejection by the caller. The limiter is in-memory only and does not identify a user, verify a signature, persist trust, or replace global listener limits.

## Consequences and remaining evidence

The core now provides an atomic admission primitive before expensive verification and tests 20 attempt admissions from one source across cooldowns. It is not wired into a signaling listener because no listener exists. The default policy and capacity behavior remain a proposal pending security review; table saturation can temporarily deny previously unseen sources. No sockets, network traffic, or device pairing are exercised.

## Verification

Eight new unit tests cover the attempt threshold and exact cooldown boundary, 20 attempts across cooldowns, atomic admission under concurrent requests, address isolation and mapped-address normalization, success reset, fail-closed capacity, invalid policies, and monotonic deadline overflow. Full workspace and Rust 1.85/Linux cross-target evidence is recorded in [verification](../verification.md).
