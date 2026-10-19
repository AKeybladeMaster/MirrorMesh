# TASK 05a — WIN: Windows x64 sender AND receiver (P2-WIN)

**Owner:** WIN; owned paths `crates/mm-adapter-win/`, `apps/mirrormesh_flutter/windows/`, Windows package scripts. **Can start immediately:** SDK setup, demo permission/capture picker and synthetic stream. **Real WebRTC gate:** CORE fixture + ADR-0002.

## Pairing bootstrap candidate (Task 28, 2026-10-19)

The Windows Devices page now prepares one process-retained pairing runtime after an explicit tap. It opens the existing Credential Manager identity and a SQLite public-peer store under `%LOCALAPPDATA%\MirrorMesh`; the C ABI returns only the public UUID. Fake-backed tests cover runtime retention and UUID/path rules, and a Flutter test verifies no initialization call occurs before the tap. Credential Manager runtime access, native WSS pairing, Windows capture, media, and hardware behavior remain unverified. Proposed [ADR-0032](../docs/adr/0032-windows-pairing-runtime-assembly-candidate.md) records the boundary.

Task 39 makes the shared QR-pinned sender client's approval wait pollable and cancellable; proposed [ADR-0040](../docs/adr/0040-cancellable-wss-sender-decision.md) records the API. The Windows app still does not invoke the client. Do not treat the pinned Flutter plugin's current desktop capture path as the requested Windows Graphics Capture adapter; WGC, sender media, and hardware encoding remain separate work.

1. Create conditional adapter crate, CI compile target `x86_64-pc-windows-msvc`; explicitly state supported Windows version/SDK.
2. Source selection and consent with Windows Graphics Capture (WGC). Implement window/display lifecycle and device-loss/restart test harness.
3. Send hardware H.264 via Media Foundation and Direct3D native textures with bounded queue and no frame bytes over Dart; test fallback codec errors.
4. Receive/decoder native GPU surface and Flutter external texture binding; attach/detach, resize and GPU-device reset tests.
5. Use shared protocol/identity and verified WSS to send to Android TV; receive known synthetic and native Windows sender.
6. Include user-confirmed disconnect, frame snapshot and Flutter overlay alignment, test 100 start/stops.
7. Prototype MSIX or signed installer packaging; run on physical Intel/AMD/NVIDIA available GPU as fixture. Hardware profile MUST be logged.

**Acceptance:** `WIN→Android TV`, `WIN→WIN` on real hardware, capture consent, stop/revoke and Windows x64 clean install. Failing/unavailable GPU feature means a documented unsupported state, not silently switching to raw TCP.
