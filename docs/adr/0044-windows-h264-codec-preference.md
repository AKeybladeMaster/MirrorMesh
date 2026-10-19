# ADR-0044 — Require H.264 preferences for the Windows sender

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

The native Android TV receiver accepts H.264 video and configures its hardware
decoder for H.264 only. Task 42 attaches a selected desktop track to the
Windows WebRTC peer, but leaving codec selection to defaults could create an
offer that cannot interoperate with that receiver.

## Decision

Before opening the selected source, the Windows Flutter WebRTC adapter queries
local video sender capabilities and requires at least one `video/H264` codec
at 90 kHz. Once the selected silent track is attached, the adapter finds its
sender transceiver and applies all locally advertised H.264 codec variants as
preferences before the session creates the SDP offer. If H.264 is not
advertised, capture does not start. Other video codecs are not offered for
this sender transceiver.

This uses the pinned `flutter_webrtc` capability and transceiver APIs. It does
not add dependencies, change the MirrorMesh signaling schema, or pass media
frames over Dart/FFI.

## Limits

The capability list is not proof that a hardware encoder was selected or that
the selected codec can encode the chosen resolution and frame rate. A Windows
target build, generated SDP inspection, encoder diagnostics, RTP transmission,
Android TV decode, and physical hardware validation remain required. The
separate WGC capture and OS-owned source picker work also remains open.

## Verification gate

`SOURCE_IMPLEMENTED` requires an early no-H.264 failure and H.264-only
transceiver preference setup before offer creation. `BUILD_VERIFIED` requires
a successful Windows Flutter build. `AUTOMATED_INTEGRATION_VERIFIED` requires
a sender-to-TV offer/answer and decoded synthetic frame. `HARDWARE_VERIFIED`
requires final physical-device testing.
