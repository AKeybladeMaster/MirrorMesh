# Flutter native surface and snapshot contract spike

**Status:** PROPOSAL ONLY — no native video surface or snapshot implementation exists.
**Scope:** define behavior to review across CORE, GUI, WIN, LNX, AND, and IOS before creating a shared render API. This note does not freeze an FFI shape or change `docs/protocol.md`.

## Current evidence

- [The protocol contract](../protocol.md) contains Rust pseudocode for `RenderPort::attach_surface` and `RenderPort::snapshot`, with `SurfaceHandle` and `EncodedPng` left opaque until an implementation spike.
- [The current ports crate](../../crates/mm-ports/src/lib.rs) implements capture, codec, transport, preferences, trusted-peer, and pairing identity ports. It has `CaptureHandle`, but no `SurfaceHandle`, `RenderPort`, or snapshot port. `CaptureHandle` is a capture resource and must not be reused as a render surface handle by assumption.
- [The FFI crate](../../crates/mm-ffi/src/lib.rs) documents a control-plane-only bridge: session state events cross FFI; video and audio buffers do not.
- Flutter's [annotation painter](../../apps/mirrormesh_flutter/lib/annotation_overlay.dart) receives normalized vector points and layout metadata. The [demo preview](../../apps/mirrormesh_flutter/lib/app.dart) paints a synthetic placeholder, and its screenshot control is disabled because no received frame exists.
- The platform adapter crates and native decoder-to-texture paths are not present. No real-media resize, surface-loss, screenshot, or latency measurement has run.

## Proposed contract semantics for review

These are behavioral requirements, not a proposed Rust struct layout.

### Surface handle and ownership

1. A surface handle is an opaque, local reference to one platform-owned render target for one session and one lifetime generation. Its concrete representation is decided by the adapter and CORE; it is not a raw pointer, OS handle, or pixel buffer exposed to Dart.
2. One platform adapter is responsible for creating and releasing the decoder output and its Flutter texture registration. The OS/Flutter engine's reference ownership is documented per target. Flutter borrows a renderable handle and may request attach, detach, or layout changes; it cannot free the native resource directly.
3. Attach and detach are idempotent. A surface can report `ready`, `resized`, `lost`, or `released`. After loss or release, the old generation is invalid; reconnecting creates a fresh handle. No callback may update a disposed session.
4. Size, rotation, pixel aspect, and color-space changes travel as bounded metadata. The adapter publishes them before Flutter maps pointer coordinates or paints an overlay for that layout.
5. Session stop, peer revocation, capture stop, window teardown, and application shutdown release the surface and subscriptions. Repeated start/stop and loss/recreate cycles must not leak listeners or retain frame buffers.

### Snapshot behavior

1. A snapshot starts only from an explicit user action. The file picker or destination choice is user-visible; canceling it creates no file. There is no timer, background export, gallery upload, or automatic retry that silently saves.
2. The request names its composition: `raw` means the presented video image without annotations; `annotated` means the same presented image with the current local vector strokes composited using the same normalized coordinates, orientation, crop, and viewport transform as the on-screen overlay. Failure to compose must return an error; it must not silently save a raw image instead.
3. The native service captures the latest presented frame at request time. Any GPU readback is bounded to that request. The live decoder/texture path remains the owner of streaming frames; the screenshot path does not add a long-lived Dart or FFI frame clone.
4. Encode and write the image locally through the selected platform path. An internal native port may hold encoded output as the protocol pseudocode describes; the Flutter bridge returns a receipt (success/cancel/error, saved destination, dimensions, composition mode, and capture time), not raw frame bytes. Do not include peer addresses, pairing secrets, or tokens in image metadata.
5. A missing, lost, or unsupported surface produces a visible unavailable/error result and no file. Screenshot work must not pause, reconfigure, or disconnect the active stream as a side effect.

## Review and implementation gates

- CORE reviews whether the existing `RenderPort` pseudocode should become a shared port, how cancellation and ownership are represented, and whether any local control metadata belongs in FFI. Any public port/FFI change follows the CORE cross-track review rule and gets an ADR if it changes a public API.
- Each native track documents who creates, presents, resizes, loses, and destroys its texture/surface, plus the opaque handle lifetime and UI-thread/actor boundary. Do not assume Windows Direct3D semantics apply to Linux, Android, or iOS.
- The GUI track keeps annotation points as vectors and tests the same transform for input and rendering. Composition must not require sending decoded frame pixels through Dart or FFI.
- Before native implementation is accepted, add fake-backed lifecycle tests for attach/detach/loss/recreate, invalid-generation callbacks, resize/orientation, cancel/error, and repeated teardown. Add one synthetic known-color frame fixture for raw and annotated output; cross-track fixture changes require CORE review.
- On each available device, verify portrait/landscape and resize behavior, inspect saved dimensions/orientation and metadata, compare raw versus annotated output, and measure snapshot time and stream side effects. Record platform, device, OS, resolution, and whether readback occurred.

## Unresolved decisions

- Whether Flutter receives a platform texture identifier directly or a narrower adapter-owned view abstraction; keep the concrete value platform-local until native feasibility is demonstrated.
- Which actor owns snapshot coordination and how the selected frame is synchronized with the annotation layer.
- Whether native adapters can compose the shared vector annotation model on every target without a GPU readback outside the explicit snapshot action.
- File picker and save receipt boundary for Windows, Linux, Android, and iOS, including platform-specific permission or destination behavior.

**Next action:** CORE and one native track review the behavioral contract and provide a buildable fake lifecycle before any shared `SurfaceHandle` or FFI code is introduced. Current local evidence is insufficient to claim the proposal has been validated.

## Local documentation validation (2026-10-18)

- PowerShell relative-link check: exit 0; all 8 referenced source/document paths exist.
- This change is documentation-only. No runtime tests, platform build, or native surface behavior were exercised.
