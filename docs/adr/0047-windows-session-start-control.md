# ADR-0047 — Queue session start after the Windows peer connects

- **Decision date:** 2026-10-19
- **Status:** Proposed; cross-track review pending

## Context

Protocol v1 already defines `session.start` as sender-to-receiver control after capture authorization and DTLS connection. The Windows sender created the offer and exchanged ICE but never sent this message. The Android TV Rust bridge parsed the message but discarded its source metadata, so the TV UI could not reflect the sender's capture-start report.

## Decision

- The Windows controller queues `session.start` once, only after the WebRTC peer reports `connected`; pending local ICE sends are drained first.
- The control message carries the generic `desktop-capture` source identifier. The local native capture ID and window/display name remain inside Windows.
- Rust validates the 1–128 byte, control-free source ID, requires an approved pairing and an active session, enforces one start per session, and assigns the next outbound sequence after offer/ICE.
- Android TV verifies the start belongs to the approved session and passes bounded metadata to its receiver status listener. It reports that the sender says capture started; this is not proof of frame receipt.
- The approved WSS and WebRTC adapters continue to own signaling and media. No video bytes enter Dart, JNI, or the FFI control DTOs.

## Consequences

The existing control message now has a Windows send path and a TV status path. Rust, protocol, and Java tests cover bounded encoding and the offer/answer/start control exchange. Flutter tests and the app bundle remain unverified in this environment. The change adds no wire schema or protocol version; cross-track review remains required because the shared protocol encoder and FFI surface changed.

`transportConnected` still reports WebRTC connectivity, not encoded video, RTP delivery, TV decoding, rendering, or hardware encoder use. Those gates remain open.
