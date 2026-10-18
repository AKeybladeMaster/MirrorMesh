# TASK 05a — WIN: Windows x64 sender AND receiver (P2-WIN)

**Owner:** WIN; owned paths `crates/mm-adapter-win/`, `apps/mirrormesh_flutter/windows/`, Windows package scripts. **Can start immediately:** SDK setup, demo permission/capture picker and synthetic stream. **Real WebRTC gate:** CORE fixture + ADR-0002.

1. Create conditional adapter crate, CI compile target `x86_64-pc-windows-msvc`; explicitly state supported Windows version/SDK.
2. Source selection and consent with Windows Graphics Capture (WGC). Implement window/display lifecycle and device-loss/restart test harness.
3. Send hardware H.264 via Media Foundation and Direct3D native textures with bounded queue and no frame bytes over Dart; test fallback codec errors.
4. Receive/decoder native GPU surface and Flutter external texture binding; attach/detach, resize and GPU-device reset tests.
5. Use shared protocol/identity and verified WSS to send to Android TV; receive known synthetic and native Windows sender.
6. Include user-confirmed disconnect, frame snapshot and Flutter overlay alignment, test 100 start/stops.
7. Prototype MSIX or signed installer packaging; run on physical Intel/AMD/NVIDIA available GPU as fixture. Hardware profile MUST be logged.

**Acceptance:** `WIN→Android TV`, `WIN→WIN` on real hardware, capture consent, stop/revoke and Windows x64 clean install. Failing/unavailable GPU feature means a documented unsupported state, not silently switching to raw TCP.
