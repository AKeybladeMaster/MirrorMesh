# TASK 05d — IOS: iOS/iPadOS ARM64 sender AND receiver (P2-IOS)

**Owner:** IOS; paths `crates/mm-adapter-ios/`, iOS Swift/Flutter plugin, iOS build/signing. Begin Xcode/Swift feature-detection and receive/decode scaffolds immediately. Real media requires CORE contract + ADR-0002 and actual Apple SDK availability.

**Task 26 status:** `mm-adapter-ios` now contains a Keychain-backed Ed25519 identity candidate and process-retained pairing-runtime entry point. The iOS Devices action invokes the Rust initializer on a worker queue and stores the trusted-peer database under Application Support, excluded from backup. Host fake-store tests pass. This Windows host has the Rust iOS target installed but lacks `clang`, `xcrun`, and Xcode, so the Apple target build, Swift compile/link, Keychain operations, and device flow remain unverified; see proposed [ADR-0031](../docs/adr/0031-ios-keychain-pairing-identity-candidate.md).

1. On macOS/Xcode CI verify `ScreenCaptureKit`, deployment target, full-display `SCContentSharingPicker` availability and required entitlements/background modes **against official Apple SDK**. Apple's currently published example needs **iOS 27+**; do not advertise older OS support without tested permitted API.
2. Feature gate sender: if unsupported OS/API, display an explicit sender-unavailable state while iOS **receiver development proceeds**.
3. Capture pipeline uses system picker user approval, `SCStream`/`CMSampleBuffer`, VideoToolbox encoder, GPU buffers; no mock consent or private API.
4. Receiver gets VideoToolbox/native texture integrated with Flutter, pairing/approved source; screenshot/annotation if permitted.
5. ReplayKit older-OS fallback is **C-01 feasibility only**, isolated adapter/ADR and App Review verification, not a shortcut to ship.
6. Verify iOS hardware `IOS→Android TV`, `IOS→Windows` if capture capability exists; independently verify `WIN→iOS` receiver. Test background, audio policy, orientation, picker stop and thermal behavior.
7. Build signed ARM64 IPA/TestFlight candidate using secure Apple credentials on macOS, update privacy disclosures and exact OS/device matrix.

**Acceptance:** verified SDK+physical iOS sender/receiver and TestFlight gate OR explicit `BLOCKED (sender)` with evidence and truthful receiver-only capability. **No iOS universal-screen-mirroring claim before hardware verification.**
