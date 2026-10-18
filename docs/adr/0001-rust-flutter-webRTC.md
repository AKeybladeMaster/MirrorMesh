# ADR-0001 — Rust core, Flutter GUI and interoperable WebRTC

- **Decision date:** 2026-10-18
- **Status:** Proposed; awaiting approval and transport spike

## Context

Require Windows x64/Linux/Android/iOS distribution, minimal input lag, screen interactions, and TV compatibility. No single platform-native UI or media framework completely solves this. Rust offers memory safety and efficient cross-platform session logic, but OS APIs still own capture/codec/GPU surfaces. Flutter offers a cohesive UI across four targets.

## Decision

Use:
- Rust 2024 edition workspace for domain/session/discovery/security/protocol with trait-based adapters and explicit concurrency ownership.
- Flutter for navigation, pairing screens, settings, annotation vectors and presentation surfaces.
- `flutter_rust_bridge` v2 stable, **2.13.0 seen in upstream releases** as of research date; pin an independently verified stable version in lockfiles. Use event streams for control/state only.
- Native code: C++/WinRT or Rust `windows` bindings (Windows); PipeWire/portal/GStreamer or platform FFIs (Linux); Kotlin (Android); Swift (iOS).
- WebRTC H.264 with transport candidates `str0m`, GStreamer `webrtcbin`, or native libwebrtc. **ADR-0002 must select after measurable browser/native tests.**
- App-owned secure signaling and pairing via mDNS+authenticated WSS; optional online broker only for browser TV / future TURN, not mandatory for local native-to-native.

## Rejected alternatives / why

- **Rust + egui only:** efficient desktop UI, but mobile iOS/Android packaging + integration needs more custom work.
- **Tauri v2:** attractive Rust/WebView stack, but native video texture integration for strict low-latency across mobile requires an additional multimedia pipeline; not impossible, only higher risk for this scope.
- **Electron:** larger resident overhead, mobile parity lacking; not appropriate for app targets.
- **Python end-to-end:** convenient prototyping, but not appropriate for native HW pipeline / 60 fps engine.
- **Custom TCP video:** LAN-only and proprietary receivers, no standard browser decoder/ICE/STUN/security interoperability.
- **One universal protocol adapter for TV:** not compatible with distinct Cast/Miracast/AirPlay ecosystems.

## Key risk / migration

`str0m` is sans-I/O; implementing packetization/timer/network and mobile packaging is more work than using a prebuilt full WebRTC stack. GStreamer `webrtcbin` gives media pipeline integration but native redistribution on Windows/mobile and binary size require a spike. Reject any transport that fails real-device 1080p testing, crash-resilience, interoperable H.264, or signed app delivery. Encapsulation behind `TransportPort` makes migration possible without GUI rewrite.

## Official sources

- https://docs.flutter.dev/reference/supported-platforms
- https://github.com/fzyzcjy/flutter_rust_bridge
- https://github.com/fzyzcjy/flutter_rust_bridge/releases
- https://github.com/algesten/str0m
- https://gstreamer.freedesktop.org/documentation/webrtc/
- https://docs.rs/gstreamer/latest/gstreamer/
