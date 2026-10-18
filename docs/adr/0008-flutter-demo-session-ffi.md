# ADR-0008 — Narrow Flutter demo session transitions

- **Date:** 2026-10-18
- **Status:** Implemented locally; cross-track review pending
- **Owners:** CORE + GUI
- **Requirements:** Master prompt Step C, M-07, M-09

## Context

Before this decision, the Flutter UI's local demo actions changed only Dart state. The Rust bridge created a session and reported its state/events, but it did not expose the safe discovery and pairing-start transitions needed by the mock flow. Step C requires available GUI actions to drive domain state transitions.

The state machine intentionally requires verified identity and explicit approval before `Paired`, and authenticated peer proof, OS capture permission, and a presented frame before `Streaming`. The demo has none of those facts and must not invent them.

## Decision

Expose three named FFI operations for the local demo: discover a demo receiver, begin demo pairing, and disconnect the demo receiver. They dispatch only the corresponding safe events through the Rust session manager. Rediscovery after a stopped demo resets that demo session before recording discovery. Disconnect performs stop and cleanup. The Rust, generated bridge, and Flutter UI changes are implemented and pass local tests; cross-track review is still pending.

The Flutter UI displays the demo as **Pairing** while the Rust domain is in `Pairing`. This pathway never marks a peer paired, starts negotiation, or enters `Streaming`. It uses no network, identity provider, capture API, capture permission, or media frame. If the Rust bridge is unavailable, the page remains an explicitly UI-local mock.

The Dart API remains narrow and named. It does not expose arbitrary `SessionEvent` dispatch or authorization booleans. The Android, Linux, iOS, and Windows platform teams can continue using the shared state machine; this FFI addition does not change the wire protocol.

## Consequences

- The mock flow's discovery, pairing-start, and disconnect actions can be verified against Rust state transitions.
- A visible `Pairing` state is not evidence of an authenticated peer or a stream. UI copy must preserve that distinction.
- FFI bindings have been regenerated and Rust plus native Flutter integration tests pass.
- This public FFI surface remains a candidate until cross-track review. Real pairing and media integration still require the frozen contract and security/permission gates.

## Alternatives rejected

- **Expose generic event dispatch:** lets Dart submit arbitrary state-machine events and weakens the trust boundary.
- **Pass `fingerprint_verified`, `user_approved`, or `capture_permitted` flags from Dart:** fabricates security/OS facts and is rejected.
- **Keep all demo state in Dart:** preserves a mock that cannot prove the Rust domain integration required by Step C.
