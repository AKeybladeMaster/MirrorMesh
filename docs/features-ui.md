# Feature inventory, UI and extension backlog

**MM-PLAN-001:** Deliver Flutter native control UX concurrently on Windows/Linux/Android/iOS. The Flutter shell has Connect, Session, Devices, and Settings pages. Live Connect is disabled. The local-only demo flow shows one built-in mock receiver; scan, pairing-start and end update the Rust state machine through named FFI operations. The annotation preview still uses a no-frame placeholder. Screenshot export stays disabled and diagnostics show no measured values. The demo does not contact a device, verify identity, request capture, or stream media. App startup safely initializes the generated Rust state bridge when its native library is available. The Windows CMake build packages `mm_ffi.dll`; Linux CMake and CI are configured to package/test `libmm_ffi.so`, awaiting Linux runner evidence. Android CI is configured to package the ARM64 `.so` in its APK, awaiting Android runner evidence. iOS Xcode device linking generates an export list for Dart's FRB process lookup, but awaits macOS runner evidence; the simulator bridge remains unavailable. Android TV now has a D-pad-focused local request/Accept/Reject/Disconnect demonstration backed by a tested state model; it has no network, media APIs, or permissions and has not been built. The TV browser provides secure-origin, WebRTC, H.264, signaling, and autoplay status with remote-arrow focus navigation; authenticated receiving remains disabled. No receiver path streams video.

The demo annotation overlay now uses a reusable `AnnotationCoordinateMapper` and `AnnotationOverlayPainter` to store points in normalized, unrotated video coordinates and paint them through `BoxFit` scaling/cropping and quarter-turn rotation. It ignores touches in contain letterboxing, clips strokes to the viewport, renders a tap as a dot, and offers labeled color and width choices captured separately for each stroke. A golden test checks the painter over a synthetic known-color frame; a widget test verifies that changing the next stroke style does not alter existing strokes. The current demo remains a fixed 16:9 placeholder; native frame orientation, live rendering, screenshot composition and device performance are not verified.

## Current local mock flow

- `Scan local demo devices`, `Start demo pairing`, and `End demo pairing` call named Rust operations. The native integration test verifies the sequence `Idle → Discovered → Pairing → Stopped` and confirms the rendered UI reports Pairing, not Connected. This does not use network discovery or establish a real peer.
- The session page renders a preview placeholder. `Draw locally`, `Undo`, and `Clear` operate on normalized in-memory strokes over that placeholder only. Teal/amber/coral/white colors and fine/medium/bold widths are selected locally and stored per stroke.
- Screenshot export remains unavailable while there is no received frame; no file is created. FPS, bitrate, and latency remain blank/unmeasured.
- The Settings page displays the same Rust session state used by the demo controls. The FFI has no API for forging identity, granting capture, or starting video.
- The live pairing code and Connect control stay disabled. The demo does not request OS capture permission or transmit media.

## UI shell (desktop)

```
┌─────────────────────────────────────────────────────────────────┐
│ MirrorMesh       Devices  Receive  Send  Sessions  Settings    │
├──────────────────────┬──────────────────────────────────────────┤
│ Available devices    │ Stream: "Living Room TV"         ● LIVE  │
│ ○ My Windows PC      │ ┌──────────────────────────────────────┐ │
│ ○ Android TV         │ │ Decoded video shown on native GPU   │ │
│ ○ Enter address      │ │ texture; Flutter overlay layer     │ │
│                     │ │ canvas; letterboxing respected     │ │
│ [Refresh] [Pair]    │ └──────────────────────────────────────┘ │
│                     │ [Disconnect] [Freeze] [Screenshot]       │
│ History/Trust        │ [Doodle] [Undo] [Clear] [Fullscreen]     │
│                     │ 1080p 60 fps · 12 Mb/s · 47 ms*           │
└──────────────────────┴──────────────────────────────────────────┘
* Example layout only; actual latency value must name its measurement method.
```

Mobile variant: tabs `Send`, `Receive`, `Devices`, `Settings`; explicit system permission action, prominent red Stop sharing, keep-screen-awake only while receiver is active and user approves.

## Features with acceptance criteria

| ID | Priority | Feature | Acceptance |
|---|---|---|---|
| F-01 | Must | Device discovery | show device name, authenticated/trusted status, type, IP masked |
| F-02 | Must | Manual connection | input host + optional port; no unauthenticated streaming |
| F-03 | Must | Pair/unpair | QR expiry, verified fingerprint, revocation stops active streams |
| F-04 | Must | Capture selection | choose monitor/window; OS consent; source-change re-negotiates |
| F-05 | Must | Connect/disconnect | UI states match session machine, no stuck spinner |
| F-06 | Must | Screenshot now | PNG has correct video orientation, optional annotation composite |
| F-07 | Must | Doodle | pressure optional, color/width, undo/clear, keyboard shortcuts, no visible lag |
| F-08 | Must | Freeze/unfreeze | freezes receiver display but sender may continue until requested stop |
| F-09 | Must | Metrics | FPS, bitrate, dropped, estimated component delay with label |
| F-10 | Should | Audio toggle | separate explicit permission, A/V sync test |
| F-11 | Should | Quality profiles | Auto / 720p30 / 1080p60, encoder caps negotiated |
| F-12 | Should | Pin / trust management | revoke, peer rename, last seen, scoped grants |
| F-13 | Should | Region capture | capture monitor/window/region with OS/API limitations |
| F-14 | Should | Presentation pointer | sender's cursor indicator shown on receiver overlay |
| F-15 | Should | Receiver screenshot export | choose PNG / JPEG and safe save path |
| F-16 | Could | Recording | consent, visible REC indicator, MP4 hardware mux, max duration |
| F-17 | Could | Rolling 15s buffer | memory/disk bounded, export only with action |
| F-18 | Could | Collaborative annotation | data channel sync only with `annotate` grants |
| F-19 | Could | Multi-receiver casting | bounded per-receiver bandwidth/encoder usage; SFU later |
| F-20 | Could | LAN party gaming mode | max quality capped for lower input lag |
| F-21 | Could | Stream to virtual camera/OBS | native OS integration, separate permission |
| F-22 | Could | Accessibility | semantic labels for pen controls, keyboard navigation, high contrast |
| F-23 | Could | Video playback controls | rotate, crop/fit, aspect, zoom, soft keyboard |
| F-24 | Research | Remote input/control | separate granular scope, revoke at any time; OS restrictions |
| F-25 | Research | Streaming from PSP | only explicit compatible homebrew/hardware bridge |
| F-26 | Research | TV screen as sender | manufacturer/API availability, not universal |

## GUI pages and states

- **Onboarding:** capture/privacy explanation; choose "send" vs "receive"; OS permissions are requested only when needed.
- **Devices:** discovered/trusted tabs, QR scan/show, manual IP, protocol/codec availability badge.
- **Session:** video, toolbar, pairing/stream state, time/fps/bitrate/jitter; local-only annotation overlay.
- **Annotations:** pointer/pen/highlighter/eraser, customizable width and palette, shape tools later; undo/clear.
- **Settings:** 720/1080 profiles, codec policy, discovery, audio, data collection (off), identity/trust list, diagnostic export.
- **Error banners:** permission denied, codec unavailable, peer version mismatch, firewall, TV browser unsupported, receiver full, TLS mismatch, signaling timeout, reconnect exhausted, network isolation.

## Doodle pipeline

Do not modify the sender's display: the receiver overlays vector strokes over the displayed texture. Snapshot option A: raw decoded frame; option B: composited frame + visible annotation. The mapper in `apps/mirrormesh_flutter/lib/annotation_geometry.dart` maps widget touches to 0..1 coordinates in the original, unrotated video and back through contain/cover fitting and quarter-turn rotation. `AnnotationOverlayPainter` draws the vector overlay without reading or copying frame pixels. Unit tests cover 0°, 90°, 180°, 270°, letterbox rejection, cover crop, invalid geometry, and proportional viewport resize; a golden test covers line and tap rendering on a synthetic frame. These tests do not validate a native video surface. Snapshot option A/B and the ≤1-frame visual response target remain unverified.

## Portfolio showcase

Include: 12–30s demo GIF/video, architecture diagram, one latency benchmark with methodology and specs, supported device matrix with named models, honest feature badges, pipeline visualization, screenshots with private data redacted, build/release commands, lessons learned. Avoid claiming third-party reverse engineering proves source architecture.
