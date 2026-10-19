# ADR-0046 — Best-effort Windows sender video diagnostics

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

The Windows sender can report a connected WebRTC transport before any video
frame has been encoded. That state alone does not show that media is leaving
the sender, and it says nothing about TV decoding or presentation.

## Decision

The Windows sender queries its video `RTCRtpSender` statistics once per second
and reports the native WebRTC stack's outbound frame and byte counters. When
present, the UI also displays the reported codec MIME type and encoder
implementation string. Missing or malformed counters stay unknown. Stats
errors do not stop an otherwise active session.

Only bounded metadata crosses into Dart. Video frame bytes remain in the
native WebRTC pipeline. The UI labels these values as sender-side reports and
keeps TV decode/presentation unconfirmed. An encoder implementation string is
displayed as reported; it is not treated as proof of hardware use.

## Consequences

This is a Windows UI/adapter diagnostic only. It adds no signaling messages,
dependencies, capture permissions, or changes to shared protocol contracts.
Other platform senders may add equivalent local diagnostics independently.

## Verification

Parser tests cover video selection, audio exclusion, malformed counters, and
optional codec/encoder metadata. A successful Windows app build is required
for `BUILD_VERIFIED`; a real receiver frame is required for
`AUTOMATED_INTEGRATION_VERIFIED`. Hardware encoder and TV display behavior
remain separate checks.
