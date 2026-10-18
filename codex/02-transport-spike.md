# TASK 02 — Real H.264 WebRTC spike + ADR-0002

**Status (2026-10-18): IN_PROGRESS — software control path implemented; engine integration underway.** Hardware and interoperability testing are deferred and do not block source work. ADR-0002 selects the libwebrtc path for implementation, with local builds and runtime evidence still pending.

### Separate implementation and verification status

- **Software implementation:** Rust transport lifecycle, bounded SDP/ICE handling, reconnect policy, 90 kHz timestamp clock, bounded encoded-frame queue, synthetic timestamped I420 color-bar source, and an async `WebRtcEnginePort` control/metadata boundary are implemented in `crates/mm-transport/` and `crates/mm-ports/`.
- **Local compilation:** Rust workspace builds and its full tests/Clippy pass on the Windows host. `flutter_webrtc` has not yet been added to the Flutter package graph or compiled; it is absent from the local pub cache.
- **Automated tests:** Rust transport tests cover limits, state transitions, reconnect, timestamp rollover and queue recovery. They do not encode or decode H.264 and do not exercise libwebrtc.
- **Hardware verification:** `DEFERRED_HARDWARE_TEST`. No Android TV decoder, named TV browser, authenticated signaling peer, media interoperability, soak or latency run has occurred.
- **Next code work:** integrate the pinned libwebrtc engine into platform adapters, authenticated signaling with Task 03, real H.264 encoder output, and native rendering. Continue without waiting for physical devices.

See [ADR-0002](../docs/adr/0002-transport-spike.md), the [implementation status ledger](../docs/IMPLEMENTATION_STATUS.md), and the [transport implementation and deferred-test report](../docs/spikes/transport-experiment-blocker.md).

**Skills:** rust-best-practices, systematic-debugging, codebase-design, verification-before-completion

## Goal

Prove H.264 WebRTC with a **synthetic known source→Android TV native receiver (primary)** and a modern desktop browser fixture; optionally add an authorized Windows display/window sender early. Benchmark candidate stacks against shipping criteria including **Windows, Linux, Android and iOS** packaging. This spike runs **in parallel with** platform SDK/mock work; choose one transport implementation before merging any real-media adapters.

## Implementation steps

1. Create a platform-neutral timestamped fake H.264 source and a Windows WGC sample if hardware exists; provide all six tracks the same synthetic stream and decoder golden fixtures.
2. Inspect `str0m` and GStreamer `webrtcbin` currently pinned versions/public API, codec setup, ICE requirements, license and build manifests. Build minimum sample for each if feasible; third candidate native libwebrtc only if both fail.
3. H.264 encode via native Media Foundation hardware encoder, low-delay profile (no B frames, small VBV) with fallback tested. If initial spike needs software encoder, mark performance proof **provisional** and schedule hardware integration test before ADR approval.
4. Implement secure signaling spike with authenticated development device identity; never expose raw unprotected internet sockets. Use SDP/ICE/DTLS-SRTP through vetted transport. Log sanitized state transitions and pcap summary showing encrypted media.
5. **Primary TV integration first:** Android TV `MediaCodec` native receiver decodes the known test stream after explicit approval. **Fallback browser spike:** `apps/tv-web` uses `<video autoplay playsinline>` (subject to browser policies) on HTTPS secure origin, with feature/codec detection and Stop; test on at least one named TV-browser device before claims.
6. Record measured capture→encode spans, actual frame arrival/render time and external camera latency on test fixture; run for 15 minutes and 20 reconnects.
7. Verify H264 baseline/profiles/packetization, MTU, NAT single subnet and denied-source behavior. No custom RTP crypto.
8. Complete ADR-0002 with results for `str0m`, GStreamer/libwebrtc as investigated, and Windows/Linux/Android/iOS/Android TV/web binary/runtime packaging risks; CORE alone updates protocol traits through reviewed versioned PR.
9. Add one end-to-end smoke test invocation in `scripts/test-web-rtc.sh` (or `.ps1`) with explicit manual hardware steps plus automated signaling tests.

## Acceptance

Primary Android TV hardware or explicitly BLOCKED native-TV decoder spike, authenticated browser fixture with result and explicit TV model compatibility; TLS/signaling authenticated; browser console no codec/media errors; 720p30 baseline and 1080p60 attempt marked pass/fail, no growing frame buffers, physical measurements documented. If cannot reach goal, mark task `BLOCKED`, propose alternate stack; do not advance to GUI and pretend transport exists.
