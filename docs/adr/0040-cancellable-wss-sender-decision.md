# ADR-0040 — Pollable and cancellable sender pairing decisions

- **Status:** Proposed
- **Decision date:** 2026-10-19
- **Owners:** CORE + Windows sender
- **Requirements:** M-06, S-04

## Context

`WssPairingClient::begin_pairing` already performs QR validation, certificate pinning, and signed `pair.begin` submission. The pending sender client previously exposed only `wait_for_decision`, which could wait indefinitely while periodically polling the receiver. A Windows UI needs to keep that wait off its UI thread and let the user cancel it without retaining a socket or treating cancellation as approval.

## Decision

Add `PendingWssPairingClient::try_wait_for_decision` to read at most one bounded socket poll and return `Ok(None)` when no decision is available. Keep `wait_for_decision` as a compatibility wrapper that repeats the poll. Add idempotent `cancel`, which drops the pending socket without waiting for a WebSocket close write and makes further polls return `Closed`.

On `pair.approve`, the client validates the QR-pinned receiver fingerprint and grant subset before transferring the live socket into `WssClientApprovedPairing`. On `pair.reject`, the pending socket is dropped and the method returns the rejection reason. Cancellation does not create or persist trust.

## Consequences and limits

- Callers can implement a cancellable worker without blocking a UI thread or adding an async runtime.
- The existing blocking API keeps its behavior for current callers.
- The change does not alter the v1 wire protocol, TLS identity checks, or approval semantics.
- The two-process loopback test covers idle polling, approval transfer, rejection, expiry, and cancellation. No Windows app invokes this API yet; no WebRTC peer, ICE, media, capture, Android APK, or hardware behavior is verified.
- Cancellation drops the socket immediately and is terminal; callers must treat subsequent polling as closed.

## Follow-up

Wire the poll/cancel API into an explicit Windows sender flow, then integrate the same session offer/answer and ICE contract used by Android TV. Keep capture permission and native media ownership outside the Dart FFI boundary.
