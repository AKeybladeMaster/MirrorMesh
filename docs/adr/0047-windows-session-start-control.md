# ADR-0047 — Queue session start after the Windows sender encodes video

- **Decision date:** 2026-10-19
- **Status:** Proposed; cross-track review pending

## Context

Protocol v1 already defines `session.start` as sender-to-receiver control after capture authorization and DTLS connection. The Windows sender created the offer and exchanged ICE but never sent this message. The Android TV Rust bridge parsed the message but discarded its source metadata, so the TV UI could not reflect the sender's capture-start report. Task 50 tightens the sender-side signal so that a connected transport alone does not imply that capture produced video.

## Decision

- The Windows controller queues `session.start` once, only after the WebRTC peer reports `connected` and outbound WebRTC statistics report at least one encoded video frame; pending local ICE sends are drained first.
- After transport connection, the controller waits up to 15 seconds for the first encoded frame. If none appears, it closes local capture and the peer, cancels the approved pairing channel, and reports a startup failure without sending `session.start`.
- The sender's `streaming` UI state means that local WebRTC statistics report encoded output. It does not mean that RTP reached Android TV or that the TV decoded or displayed a frame.
- The control message carries the generic `desktop-capture` source identifier. The local native capture ID and window/display name remain inside Windows.
- Rust validates the 1–128 byte, control-free source ID, requires an approved pairing and an active session, enforces one start per session, and assigns the next outbound sequence after offer/ICE.
- Android TV verifies the start belongs to the approved session and passes bounded metadata to its receiver status listener. It reports that the sender says capture started; this is not proof of frame receipt.
- The approved WSS and WebRTC adapters continue to own signaling and media. No video bytes enter Dart, JNI, or the FFI control DTOs.

## Consequences

The existing control message now has a Windows send path and a TV status path. Rust, protocol, and Java tests cover bounded encoding and the offer/answer/start control exchange. Task 50 adds sender-side state and timeout handling without changing the control schema or shared Rust/FFI interfaces. Flutter tests and the app bundle remain unverified in this environment. Cross-track review remains required for the shared protocol encoder and FFI surface introduced by Task 47.

`transportConnected` reports WebRTC connectivity while waiting for local encoded output. `streaming` reports local encoded output only. RTP delivery, TV decoding/rendering, WGC capture, hardware encoder use, and hardware behavior remain unverified.
