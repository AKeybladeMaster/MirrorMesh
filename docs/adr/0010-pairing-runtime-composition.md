# ADR-0010 - Generic pairing runtime composition

- **Decision date:** 2026-10-18
- **Status:** Proposed CORE runtime composition; security and cross-track review pending

## Context

`mm-application::PairingCoordinator` already defines the consent-gated invitation, proof-verification, and trust-persistence flow, while platform identity and trusted-peer persistence are separate ports. Without a runtime boundary, app composition could construct the coordinator with inconsistent local identity metadata, nonce sources, or approval behavior.

## Proposed decision

- Add `mm-runtime::PairingRuntime` as the OS-independent composition boundary over `LocalPairingIdentityPort`, `PairingIdentityPort`, and `TrustedPeerPort`.
- Initialization reads the current public identity and computes its fingerprint through the identity port. Any failure aborts initialization; there is no generated or in-memory identity fallback.
- Each runtime owns a `PendingPairingNonces` registry backed by the operating-system random source. It exposes invitation creation, source-aware proof admission/verification, and explicit approval. `verify_proof_from` reserves an attempt before proof verification; its pending-approval wrapper retains the source address, and successful trust persistence clears that source's attempt history. Failed signature verification does not persist trust; approval persists only a subset of the sender-requested grants.
- Callers provide monotonic and wall-clock timestamps. Platform application code must use the intended system clocks and must obtain explicit user approval for the numeric endpoint and requested grants before calling the corresponding operations.
- The local fingerprint is captured at runtime construction. Platform lifecycle code must drop and recreate the runtime after local identity rotation or deletion so invitations cannot advertise stale key metadata.
- The runtime delegates proof rules and persistence ordering to `PairingCoordinator`; it does not open sockets, discover peers, display QR codes, or implement a platform UI.

## Consequences and open gates

- CORE has a shared port-based runtime that can be composed with different platform identity and trusted-peer adapters.
- Current tests use fakes for runtime composition. The generic runtime is not yet assembled into Windows, Linux, Android, iOS, Android TV, or browser application startup; the SQLite and Windows identity adapters have not been exercised together through this runtime.
- The Windows Credential Manager candidate has only fake-store coverage. Linux Secret Service, Android Keystore, and iOS Keychain adapters, QR/approval UI, discovery, authenticated signaling/listener integration, revocation-driven connection closure, security review, and cross-track API review remain open. The pairing admission policy is a candidate, not an approved protocol limit.
- This ADR does not freeze the protocol wire representation, Ed25519 verification profile, clock source API, or network authentication behavior. No device-pairing or hardware result is claimed.

## Verification

`mm-runtime` tests use fake identity and trusted-peer ports. They cover local fingerprint derivation, fail-closed identity initialization, signature rejection, no persistence before explicit approval, grant reduction, store failure, limiter rejection before signature verification, and source-budget reset after approved pairing. All eleven runtime tests pass under Rust 1.85 and 1.99.1. These tests verify composition only and do not access platform key stores, a network, or physical devices.
