# P3 integration readiness spike

**Status:** BLOCKED at the Task 07 entry gate  
**Date:** 2026-10-18

## Gate result

Task 07 requires a known sender and native Android TV receiver to exchange the candidate protocol before cross-platform end-to-end testing begins. That pair does not exist yet. ADR-0002 selects WebRTC for Flutter sender targets and a native Android TV receiver integration path. The Flutter sender now has a metadata-only WebRTC adapter, but it has no signaling, capture, attached tracks, or media playback. The Android TV app remains a local-only approval UI with no signaling, WebRTC, decoder, or video surface. Its separate APK build workflow is authored but its hosted result has not been reviewed. Flutter's demo scan, pairing-start and end actions mutate only safe Rust session states; native integration tests verify the path, documented in the [Flutter mock/domain-state spike](flutter-mock-domain-state.md) and [ADR-0008](../adr/0008-flutter-demo-session-ffi.md).

The protocol is a candidate, not frozen v1: Rust has one valid `session.offer` golden fixture and parser/state tests, but no cross-platform exchange or shared fixture review. WebRTC is selected, but no H.264 video fixture or encoded stream has been produced because no native encoder path is integrated. A desktop screenshot of the browser capability shell is available; it shows standby only and is not media evidence.

The native Flutter surface and snapshot boundary is also only a proposal: the protocol document has pseudocode, while the shared Rust ports and platform adapter crates do not provide a render surface or snapshot service. See the [surface/snapshot spike](flutter-surface-snapshot.md) for current source evidence and review gates.

## Pathway status

`BLOCKED` describes the real integration path. `MOCK` describes a local UI or capability check and does not imply a network session.

| Required pathway | Status | Current evidence and blocker |
|---|---|---|
| Windows sender → Android TV native receiver | BLOCKED | Windows shell build and local mock only; no capture/transport adapter, no built TV APK, no TV hardware. |
| Linux sender → Android TV native receiver | BLOCKED | No Linux capture/transport adapter or runner result; no built TV APK or hardware. |
| Android phone sender → Android TV native receiver | BLOCKED | No MediaProjection sender or transport adapter; no built TV APK or hardware. |
| iOS/iPadOS sender → Android TV native receiver | BLOCKED | No capture implementation or supported-device run; no built TV APK or hardware. See the [iOS capture spike](ios-capture-spike.md). |
| Windows → Windows native receiver | BLOCKED | Windows app starts and loads the Rust DLL, but native capture, transport, and decode are absent. |
| Windows → Linux, Android phone, or iOS native receiver | BLOCKED | Receiver adapters have not been implemented or built on their target runners. |
| Windows → TV browser fallback | MOCK | Seven Node capability/remote-focus tests and a headless Edge 155 localhost capability smoke pass; no TV-browser runtime, authenticated signaling, peer connection, or media path exists. |

## Available evidence

- Rust CORE format, lint, unit, and FFI integration checks passed on Windows; these validate local code, not protocol interoperability.
- Android TV Java state and XML/permission checks passed. They validate local approval transitions and source boundaries; the APK has not been built or installed.
- TV web logic has seven passing Node tests; headless Edge 155 localhost confirms browser capability detection and the disabled Connect state. No HTTPS deployment, named TV browser, signaling, or media result exists.
- The Windows Flutter release shell built and loaded `mm_ffi.dll` in a startup smoke check. It has no real capture or receive path.
- No sender-to-receiver build hash, hardware test log, measured latency set, annotated screenshot, or video artifact exists. Although the repository now has Git metadata, no interoperating sender/receiver build exists from which to report a validated end-to-end build SHA.

See the [verification ledger](../verification.md) for command-level local evidence and limitations.

## Preconditions to start cross-platform E2E

1. Review and freeze the shared protocol fixtures, including exact version rejection and pairing/consent semantics.
2. Select a transport through a completed ADR-0002 proof using the same H.264 fixture for a native Android TV receiver and a named browser/desktop peer.
3. Implement a sender and native Android TV receiver that exchange authenticated signaling, enforce capture consent and receiver approval, and render a received frame.
4. Run the Android TV APK build job successfully and install the APK on a named physical TV device.
5. Assign exact sender/receiver models, OS versions, SDKs, network fixture, and an external-camera measurement setup.

## Integration evidence record template

For each run, create a dated record and retain links only to authorized artifacts. Do not include private IP addresses, pairing secrets, user screen content, or raw logs containing personal data.

```text
Run ID / date (UTC):
Sender track / build SHA:
Receiver track / build SHA:
Protocol version / fixture revision:
Transport ADR and implementation revision:
Sender model / OS / SDK / GPU / codec:
Receiver model / OS / SDK / GPU / codec:
H.264 profile / packetization mode:
Network and router fixture (sanitized):
Build commands and exit codes:
Pairing and explicit consent result:
First decoded frame / stop and revoke result:
Negative cases (fingerprint, reject, revoke, loss, ICE, codec, permission):
FPS / drop rate / queue bounds:
Latency sample count / method / p50 / p95 / uncertainty:
Soak duration / resource observations:
Annotated screenshot / video / sanitized log links:
Status: PLANNED | MOCK | BUILDS | E2E_PASS | BLOCKED
Known failures and next action:
```

Until the preconditions pass, report Task 07 as BLOCKED and continue work on independent CI and platform scaffolds. Do not publish a supported-device claim from mock or parser evidence.
