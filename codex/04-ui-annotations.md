# TASK 04 — Shared Flutter GUI, native Android TV UX and browser fallback controls (P2/P3)

The annotation UI is implemented as a local placeholder. Demo scan, pairing-start and end buttons call narrow Rust session operations and native bridge tests verify the safe state path. The demo still has no real peer, media frame or network activity; cross-track FFI review remains pending in [ADR-0008](../docs/adr/0008-flutter-demo-session-ffi.md).

**MM-PLAN-001:** Flutter screen models cover Windows, Linux, Android and iOS concurrently. Android TV has a dedicated native D-pad receiver UX; TV browser fallback has its own capable/unsupported states. The annotations/snapshot *model* is shared; render adapters are platform-specific.

**Skills:** frontend-design, TDD, requesting-code-review, verification-before-completion

## Current progress (2026-10-18)

The Flutter demo now uses `AnnotationCoordinateMapper` and the reusable `AnnotationOverlayPainter`. The mapper converts local touches to normalized, unrotated video coordinates through `BoxFit` and quarter-turn transforms. Five unit tests cover rotation, contain letterboxing, cover crop, proportional viewport resize, and invalid geometry. The painter clips to bounds, draws single taps as dots, and has a golden test over a known-color synthetic frame. The demo offers labeled color and width choices, captured per stroke; widget tests verify per-stroke style, color/width semantics, and the session-only `P` / `Ctrl+Z` / `⌘+Z` shortcuts. Flutter format/analyze and widget tests pass; the Windows release build passed from the writable validation mirror with only temporary-directory MSBuild warnings. These are Flutter widget-level key events on this host. No native frame, snapshot composition, cross-platform video surface, full-app screenshot/GIF, or hardware interaction has been verified. Remaining Task 04 steps and acceptance criteria are open.

An additional widget check verifies compact bottom navigation at 400 logical pixels and the wide navigation rail at 800 logical pixels with 2x device-pixel ratio. This verifies the existing logical breakpoint in Flutter's simulated view; physical DPI, rotation, and device input remain open.

## Steps

1. Create screens/widgets: Onboarding, Devices, PairingModal, SessionPage, StreamView, SessionToolbar, OverlayCanvas, Settings, Errors, TrustManagement.
2. Model UI state as `Disconnected | Discovering | Pairing | Negotiating | Connected | Paused | Reconnecting | Error` derived from Rust events. Widget tests assert controls only enabled in legal states.
3. Define platform-independent `SurfaceHandle`, snapshot composition semantics and lifecycle first; have WIN/LNX/AND/IOS agents implement **their own** native decoder-to-texture adapters. Windows Direct3D texture is only one adapter. Document each device/surface's ownership, resize and loss semantics. **Never send frame pixels over FFI**. Current behavioral proposal and blockers: [surface/snapshot spike](../docs/spikes/flutter-surface-snapshot.md).
4. Put annotations in Flutter `CustomPainter` overlay for MVP, mapped to normalized video rect and independent of media frames; screen capture is composed through a native snapshot service (or native GPU readback **only when user explicitly takes screenshot**). Measure screenshot time and side effects.
5. Toolbar actions: disconnect, freeze/unfreeze, screenshot, pen toggle, undo, clear, fullscreen, audio toggle (if supported), quality selection.
6. Implement image save path picker and PNG metadata, filename `MirrorMesh-YYYYMMDD-HHMMSS.png`, no silent gallery upload. Show toast with local save destination.
7. Add high DPI, portrait/landscape, letterboxing, fullscreen, accessibility labels and keyboard shortcuts across native Flutter shells; Android TV agent implements receiver-native 10-foot D-pad focus, QR approval and Stop; browser agent implements 10-foot focus + unsupported-browser UX.
8. Add screenshot golden tests with known RGB test frames and annotations; video draw pointer test at 0/90/180/270°, window resize, crop/fit.
9. Record short demo GIF/screenshot from **our original app**, replace README placeholder only once authentic, redact private names.
10. Update `docs/features-ui.md` checkboxes and actual release status.

## Acceptance

Doodle drawn locally on each verified native receiver with no perceptible UI slowdown independent of stream rate; TV native remote stop and browser capability errors tested; screenshot raw vs composite verified; disconnect removes active tracks and overlay; no unauthorized recording.
