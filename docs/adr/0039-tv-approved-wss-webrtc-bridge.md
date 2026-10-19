# ADR-0039: Android TV approved WSS to WebRTC bridge

**Status:** Proposed
**Date:** 2026-10-19
**Scope:** Android TV receiver signaling integration

## Decision

Keep pairing identity, trust, protocol validation, message sequence checks, and control-channel ownership in Rust. After explicit approval with the `view` grant, the TV JNI bridge polls the existing `PairingConnection` for typed `session.offer`, `session.ice`, `session.start`, `session.stop`, and `pair.revoke` messages. Rust emits only the bounded SDP/candidate metadata needed by the existing Android WebRTC adapter; media frames remain inside libwebrtc and the native renderer.

The shared WSS adapter uses a 100 ms post-handshake read timeout and exposes `try_receive_control_message`. Blocking callers retain the existing `receive_control_message` API, which waits across bounded polls. This allows Android JNI reads, sends, and explicit close to share the connection registry without holding its global lock during an unbounded socket read.

Rust encodes and validates TV `session.answer` and `session.ice` envelopes. The Android side starts the existing receive-only H.264 WebRTC adapter only after an approved sender offer arrives. Early ICE candidates are buffered in a bounded queue until the video receiver attaches.

## Consequences

- The shared v1 schema is additive; no pairing message or platform-specific session engine is introduced.
- Host Rust tests can verify envelope encoding and WSS polling. They cannot verify Android JNI ABI, Gradle packaging, WebRTC interop, MediaCodec decode, or TV rendering.
- The Android target build remains required to verify JNI and Java integration; an app-level synthetic sender test is still needed before this becomes a functioning reference path.
- Trust revocation persistence, received-frame screenshots, Windows sender wiring, capture, and encoding remain separate incomplete work.

## Verification

See [implementation status](../IMPLEMENTATION_STATUS.md) and [verification](../verification.md) for the Task 38 check results and explicit limits.
