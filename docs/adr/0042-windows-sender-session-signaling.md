# ADR-0042 — Windows sender session signaling bridge

- **Status:** Proposed
- **Date:** 2026-10-19
- **Owners:** WIN + CORE
- **Requirements:** M-02, M-06, M-07, S-03, S-06

## Context

Task 40 connects the Windows Devices flow to the existing QR-pinned WSS pairing client and retains the approved `PairingConnection`. Android TV already consumes the shared `session.offer` and `session.ice` messages through its Rust/JNI signaling bridge. The Windows pairing worker did not yet use that control connection for session negotiation.

## Decision

Keep the existing v1 control schema and add a shared `mm-protocol::encode_session_offer` helper. The Windows pairing worker continues to own the approved WSS connection on a background thread. Bounded commands carry an SDP offer or trickled ICE metadata to that worker; bounded events carry an H.264 answer, remote ICE, stop, or revoke notification back to Flutter. The polling C ABI reads only from an in-process event queue and performs no socket I/O.

`WindowsSenderSessionController` configures the pinned `flutter_webrtc` engine with a send-only video transceiver, creates and applies a local offer, sends it over the authenticated WSS connection, applies the TV answer, and exchanges ICE metadata. Local and remote candidate queues cap at 128 candidates and 64 KiB. Native video frames remain in the WebRTC/native renderer path and never cross Dart FFI.

This slice does not attach a video track, capture the Windows desktop, send RTP media, render a received frame, or claim that a mirror stream works. The send-only transceiver exists to exercise signaling and peer negotiation before the native synthetic H.264 source is integrated.

## Consequences

- Sender and TV use the existing authenticated peer identity, `view` grant, WSS connection, session identifiers, sequence tracking and v1 message validators.
- The Windows C ABI exposes only bounded SDP/ICE strings and event metadata. Socket reads and writes stay on the worker thread.
- Flutter can manage the WebRTC offer/answer and peer lifecycle without owning WSS protocol serialization.
- The Android TV answer and ICE path now has a matching Windows source bridge, but there is no app-to-app interoperability evidence yet.
- A later task must attach a real synthetic H.264 track and verify ICE/DTLS-SRTP, then replace the synthetic source with Windows Graphics Capture and hardware encoding.

## Verification and limits

Protocol encoder tests, a two-process Rust WSS offer/answer test, fake-backed Flutter signaling tests, Windows-target Rust checks, and a direct MSVC compile of the Windows runner cover this source slice. No native Windows app bundle, Android TV APK, actual `RTCPeerConnection` exchange, ICE connectivity, RTP frame, screenshot or hardware scenario is verified by this ADR.
