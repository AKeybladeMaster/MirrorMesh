# ADR-0051 — Bound Windows sender peer reconnection

- **Decision date:** 2026-10-19
- **Status:** Proposed; Windows/CORE review pending

## Context

The Windows sender controller distinguished transport connection from local encoding, but ignored a WebRTC `disconnected` event after connection. That could leave the UI reporting `streaming` while the peer was interrupted, and it provided no bound for recovery or receiver cleanup.

## Decision

- After a peer that was connected reports `disconnected`, the sender enters `reconnecting` and gives the existing libwebrtc peer up to 10 seconds to recover.
- Keep the same peer and user-approved capture active during this bounded interval. Let libwebrtc perform its normal recovery; do not silently recreate the peer or restart capture.
- If the peer reconnects, restore `transportConnected` or sender-reported `streaming` according to the existing local encoded-frame statistic. Do not resend `session.start` for the same session.
- If the peer does not recover, stop local capture and close the peer, attempt the existing bounded `session.stop` while the approved signaling channel is open, then cancel that channel and report failure.
- Failure cleanup uses bounded generic reasons (`transport-failed`, `media-startup-failed`, or `sender-failed`). It does not carry private source names or media bytes.

## Consequences

The Windows UI can distinguish a temporary transport interruption from local encoding and terminal failure. No protocol schema, Rust FFI, permissions, or capture API changes. The timeout and event transitions have mock-backed controller tests; Flutter/Dart tools are unavailable on the current workstation, so `BUILD_VERIFIED` remains pending. Reconnection tests do not demonstrate RTP delivery or Android TV recovery.
