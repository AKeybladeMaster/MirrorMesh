# Flutter mock flow and Rust domain state

**Status:** Safe local path implemented; CORE/GUI interface review pending  
**Date:** 2026-10-18  
**Requirements:** Step C of the master implementation prompt; M-07, M-09

## Finding

The Flutter mock flow now calls three named Rust bridge operations from its scan, pairing-start and end actions. `FfiController` applies only discovery, begin-pairing, stop and cleanup transitions. The generated Dart API does not expose generic event dispatch. The annotation demo remains local to Flutter and paints over a synthetic placeholder.

Relevant sources: [Flutter demo flow](../../apps/mirrormesh_flutter/lib/app.dart), [Flutter Rust status wrapper](../../apps/mirrormesh_flutter/lib/rust_session_status.dart), [Rust bridge API](../../crates/mm-ffi/src/bridge_api.rs), [domain state machine](../../crates/mm-domain/src/session.rs).

## Safety and interface constraints

- The Connect action must not fabricate a verified identity, user-approved pairing, capture permission, or first presented frame.
- `SessionState::Streaming` must remain unreachable until authenticated peer proof, OS capture permission, and a real presented frame are established.
- Dart should receive named, narrow commands rather than arbitrary `SessionEvent` values or authorization booleans.
- Any FFI API change is CORE-owned and requires an ADR and cross-track review before it becomes a shared contract.
- Keep the current UI flow explicitly marked as a local mock; the pairing state does not represent a real peer or connection.

## Implementation and review

ADR-0008 defines three named operations: `discover_demo_receiver`, `begin_demo_pairing`, and `disconnect_demo_receiver`. Discovery supports rediscovery from `Stopped` by resetting the demo session first. Disconnect emits Stop and CleanupComplete and returns `Stopped`. Flutter labels the state Pairing and does not claim that a peer is connected or trusted.

Rust FFI integration tests cover invalid pairing before discovery, `Idle → Discovered → Pairing → Stopped`, rediscovery from `Stopped`, and rejection of unauthorized streaming. The native Flutter integration test clicks the UI actions and reads the corresponding Rust state through the generated bridge. Current verification results are recorded in [verification.md](../verification.md). The FFI surface remains a candidate until CORE/GUI cross-track review. This earlier spike predates Git initialization; the current work is tracked on pushed task branches, while that cross-track review remains pending.

## Current verification

The Flutter suite passes analysis and all 14 widget tests with the built Windows DLL configured, including end-to-end UI-to-Rust state assertions. Rust formatting, Clippy and all workspace tests pass (54 unit tests and 3 FFI integration tests). No network, identity proof, capture permission, or media frame is part of this demo path. Linux, Android, iOS and hardware integration remain unverified.
