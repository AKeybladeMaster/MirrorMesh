# TASK 07 — Multi-platform integration, compatibility and GUI proof (P3)

**Owner:** integration; owned paths `tests/e2e/`, `docs/verification.md`, `docs/device-matrix.md`, QA scripts and test environment docs. Begin once first known sender and native Android TV receiver can exchange protocol v1. Run in short continuous cycles, not only after every platform is done.

## Build fixtures

- Design a synthetic timestamped H264 video/golden annotation clip shared by Windows, Linux, Android, iOS, Android TV and browsers.
- Record build SHA + protocol version + transport ADR + device model/OS/GPU + router/network + H264 profile and hardware support.
- Test positive session path (discovery→pair→capture approval→decode→snapshot/doodle→stop) and negative cases (bad QR fingerprint, reject, revoked trust, packet loss, ICE failure, codec mismatch, missing capture permission).

## Mandatory pathway grid (conditional on actual OS API support)

| Sender | Native receiver | TV primary | Web fallback |
|---|---|---|---|
| Windows | Windows + smoke to Linux/Android phone/iOS receivers | Android TV hardware | at least one named TV browser |
| Linux | Windows native | Android TV hardware | optional supplemental |
| Android phone | Windows native | Android TV hardware | optional supplemental |
| iOS/iPadOS | Windows native **if SDK/system picker permits capture** | Android TV hardware same gate | optional supplemental |

Additional receiver proofs: Windows native sender→Linux native receiver, Android native phone receiver, and iOS native receiver, including release-mode decode.

## Merge gates

1. Protocol golden vectors identical; no platform-private copy of pairing/state machine. Validate feature flags and exact version rejection.
2. Consent appears on every sender with no capture before user action; TV receiver approval deny-by-default; browser session authenticated.
3. No raw frame transfer through Dart; queues bounded and teardown closes captures/encoders/sockets/decoder within resource budgets.
4. Per tested pair: 100 measured interaction→photons samples where possible, FPS, p95 latency, drop rate, 60-minute soak and at least one annotated screenshot.
5. Maintain per-path status `PLANNED | MOCK | BUILDS | E2E_PASS | BLOCKED` with date and direct artifact link. Never extrapolate a TV engine or iOS API to untested devices.
6. If a track is blocked, other tracks continue; the release dashboard and README indicate unsupported features explicitly.

**End of task:** post validated matrix plus five representative video/screenshot evidence artifacts from owned test devices; exact CI commands and hardware test log links. Do not manufacture evidence.
