# ADR-0043 — Attach an explicitly selected desktop track to the Windows peer

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

Task 41 connects the approved Windows WSS session to a real `flutter_webrtc`
peer connection, but its video transceiver has no track. The Android TV receiver
already accepts an approved H.264 offer and owns native decoding and fullscreen
rendering. The next useful slice is to connect a selected native desktop track
to the existing peer without sending pixels through Dart, Rust FFI, or signaling.

## Decision

After rechecking that TV pairing is approved, the Windows UI lists screen/window
source metadata and asks the user to select a source. The pinned
`flutter_webrtc` plugin obtains one silent native video track, attaches it to
the existing `RTCPeerConnection`, then the sender creates and signals its SDP
offer. Stop, failure, disconnect, or revocation stops the local track and closes
the peer. Only bounded source identifiers, SDP, ICE, and state metadata cross
the Dart/platform and Rust/FFI boundaries.

The approved Devices page also renders a known color-bar test card. It is a
visual target that can be captured from the MirrorMesh window; it is not a
native synthetic frame generator.

## Limits and migration

This is an integration candidate, not the final Windows capture adapter. The
plugin source picker is an in-app list and the underlying desktop capturer has
not been verified as Windows Graphics Capture or as the OS-owned capture picker.
Do not claim WGC, native synthetic H.264 generation, hardware encoder use,
received frames, or app-to-app interoperability from this code. Replace the
capture provider with a reviewed Windows Graphics Capture adapter and its
system picker while keeping the shared signaling and peer-session lifecycle.
The host has no Android SDK/Gradle, and this task has no app-level WebRTC test.

## Verification gate

`SOURCE_IMPLEMENTED` requires track attachment only after an approved pairing,
silent-video validation, and cleanup coverage. `BUILD_VERIFIED` requires a
successful Windows Flutter target build. `AUTOMATED_INTEGRATION_VERIFIED`
requires an automated Windows sender to Android TV peer flow with decoded-frame
evidence. `HARDWARE_VERIFIED` is deferred to final user testing.
