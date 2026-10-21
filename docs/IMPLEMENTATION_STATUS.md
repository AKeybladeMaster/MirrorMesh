# Implementation status — test release integration

Branch baseline: `test_2` at `ef05d95`. The AirPlay repair checkpoint is on `test_3`;
after the requested handoff, new development belongs exclusively to `test_4`.
This document supersedes historical increment counts for this release candidate.

## Readiness labels

- **Source implemented:** integrated code exists; this alone does not prove runtime behavior.
- **Build verified:** an actual compiler/package build succeeded for the stated target.
- **Automated integration verified:** not claimed for this branch; user requested no test suites.
- **Hardware verified:** user-reported PC→Android and Android→PC success on 0.2.1
  (2026-10-21). Android is Redmi Note 5 / Android 13; PC GPU details were not supplied.
  This is user evidence, not independent verification. TV and new 0.3.0 behavior are pending.
- **CODE_COMPLETE:** the complete agreed product requirements are implemented; not yet achieved project-wide.

## Current reference pipeline

### Windows 0.3.1 AirPlay startup repair (2026-10-21)

- **Source implemented:** package GStreamer core DLLs in `airplay/bin`, classify plugins
  by their actual source directory and include UxPlay's required `autodetect` plugin.
  Inspect final packaged imports and run a restricted-PATH loader check during packaging.
  Native errors now distinguish missing DLLs, wrong architecture, media errors and timeout.
- **BUILD_VERIFIED:** final Windows 0.3.1+7 build/package and Flutter analysis exited 0;
  Dart format, PowerShell parsing and diff checks exited 0. Versioned owner-local ZIP and
  SHA-256 sidecar are in `releases/`; 283 packaged file digests were verified.
- **Local runtime diagnosis:** packaged helper help exits 0. The real native startup
  diagnostic retains Ready for two seconds within a 15-second window, then exits 0
  with no helper process left. It approves no client and captures/receives no media.
  These are startup checks, not an automated media integration suite.
- **Hardware verification pending:** the owner reported 0.3.0 stopping immediately and
  absent iPad discovery. Both reproduced startup defects are repaired locally; iPad/iPhone
  discovery, PIN/consent, decoding, audio and streaming still need physical confirmation.
  Android remains on compatible 0.2.1 with no source changes or new APK required.
- **Branch handoff:** repair/release checkpoint is committed and pushed on `test_3`, then
  `test_4` is created from it as requested. Inspect Git for the resulting hashes/active branch.

### Windows 0.3.0 fitted viewer and native AirPlay preview (2026-10-21)

- **Source implemented:** received WebRTC video fits the available PC window using contain
  sizing and a bounded video area; Stop stays visible. Windows full screen uses F11/Escape.
- **Source implemented:** native iOS Control Center Screen Mirroring receiver through a
  pinned UxPlay companion, mDNS, per-connection PIN, explicit PC Allow/Reject, native
  D3D11/FFmpeg video + WASAPI audio, bounded window placement and owned-process cleanup.
  This is separate from the unchanged MirrorMesh QR/TLS WebRTC engine and iOS ReplayKit app.
- **BUILD_VERIFIED:** Windows 0.3.0+6 native compilation, locked-toolchain setup check,
  helper compilation and portable bundle packaging all exited 0. The runtime includes 141
  required binaries plus modified UxPlay source/notices. Flutter analysis exited 0 with
  no issues. No receiver has been started here.
- **Automated integration verified:** not claimed; no test suites are authorized.
- **Hardware verification pending:** viewer layout and AirPlay on iPhone 15 Pro Max / 2020
  base iPad. Their exact iOS/iPadOS versions are unknown. No AirPlay screenshots, certified
  compatibility, DRM streaming, AWDL or multi-room claim. Public dependency redistribution
  review is pending; this is a local preview, not a production-complete release.

See [AirPlay guide](airplay.md) and [ADR0059](adr/0059-native-airplay-and-bounded-receiver-view.md).

### 0.2.1 sender startup and direct LAN requests (2026-10-21)

- **Source implemented:** Android sender no longer polls before offer submission; an idle
  approved shared channel returns Pending. Android displays preparation/negotiation/failure
  stages. PC receiver errors distinguish protocol rejection and transport/channel closure.
- **Source implemented:** selecting a compatible discovered receiver sends a direct request;
  both screens show a matching code, receiver grants remain unchecked and the native sender
  cannot reach Approved until both receiver approval and sender confirmation. QR remains
  available. LAN nonce provenance survives listener restarts; source throttling is retained.
- **Build verified:** final 0.2.1 Windows EXE/ZIP and Android ARM64 phone/TV debug and
  unsigned release APKs passed with exit 0. Rust check/Clippy, Flutter analysis and Android
  lint passed with exit 0; lint reports 0 errors and 77 warnings per flavor.
- **Automated integration verified:** not claimed; no test suites are permitted on this branch.
- **Hardware verification pending:** owner reported Redmi Note 5 / Android 13 → PC failing
  after capture approval. A source startup race is fixed, but the physical failure was not
  reproduced here. Direct request prompts, code comparison and actual media still need the
  final user-device phase. No project-wide CODE_COMPLETE claim.

See [ADR0058](adr/0058-direct-lan-requests-and-sender-startup.md).

### 0.2.0 connection checkpoint (2026-10-21)

| Requested integration | Source implemented | Build evidence | Device evidence |
| --- | --- | --- | --- |
| Compact invitations, legacy input compatibility | Shared Rust v2 encoding/parser | Windows and Android builds passed | Pending |
| PC network search and name filtering | Existing Rust mDNS adapter + Flutter UI | Windows build passed | Pending |
| Android/TV discoverable receivers | Public Android NSD advertisements | ARM64 phone/TV APKs passed | Pending |
| PC receiver QR and inbound video | Shared receiver registry + native WebRTC renderer | Windows build passed | Pending |
| Android sending to PC/Android/TV | Shared sender driver + MediaProjection service | ARM64 phone/TV APKs passed | Pending |
| iOS send/receive, QR app links and ReplayKit extension | Integrated source and Mac project configurator | Flutter analysis passed; Xcode native build unavailable | Pending |
| PC to native Android/Google TV receiver | Existing route plus discovery UI | Windows and TV APKs passed | TV pending |

The owner reports the preceding app works, including Windows-to-phone sharing. This is
user-reported baseline evidence; the new directions and discovery are not hardware verified.
Generic Miracast/AirPlay/Chromecast TVs remain unsupported. iOS requires a Mac build,
project configuration, a provisioned App Group and signing before its source can be
considered BUILD_VERIFIED. No project-wide CODE_COMPLETE or production claim is made.

The following table describes the baseline path retained by these integrations.

| Stage | Source | Build evidence | Hardware |
| --- | --- | --- | --- |
| Windows local identity and QR-pinned WSS client | Implemented | Rust workspace compiled | Pending |
| Android identity, TLS listener, approval, SQLite trust | Implemented | ARM64 JNI compiled | Pending |
| Windows real pairing and source-selection UI | Implemented; demo home removed | Flutter analysis and 0.1.1 EXE packaging passed | Pending |
| Windows native desktop capture, H.264 WebRTC sender | Implemented through pinned plugin | 0.1.1 Windows native build passed | Pending |
| Android receive-only WebRTC, hardware H.264, fullscreen surface | Implemented | Phone and TV APK assembly passed | Pending |
| Receiver screenshots | Implemented using received native frame | APK assembly passed | Pending |
| Stop/reconnect/startup deadlines and cleanup | Implemented | Windows and Android builds passed | Pending |
| Persisted receiver revocation and authenticated notification | Integrated | Android APK assembly passed | Pending |
| Android 17 local-network consent, phone layout, invitation sharing | Integrated | Android APK assembly passed | Pending |

## Android 0.1.2 UI checkpoint

The user confirms PC screen sharing works on an Android phone and requested Android
accessibility/visibility improvements on a new `test_2` branch. Source changes reorganize
setup, invitation sharing and approval; use scalable text and minimum 56dp controls;
correct focused-button contrast; and separate video controls from the rendering surface.
Connection details are expandable and revocation requires confirmation. Frame counters
no longer replace screenshot feedback. Transport, identity and media code are unchanged.
Compilation and static analysis of the new UI are in progress; native visual/TalkBack
verification has not been performed. See `docs/android-ui.md` for design and boundaries.

## Historical pairing feedback and 0.1.1 checkpoint (2026-10-20)

The user reports that Windows rejected an invitation or did not reach receiver approval.
The exact error and receiver device model are pending; no device is attached to ADB.
This failure was not reproduced here. The later user report confirms successful PC-to-phone
sharing, without establishing the original failure cause or compatibility with other devices.

Source changes add fixed `MM-PAIR-*` diagnostics for invitation, network, secure-handshake
and approval failures, and bound the approval wait by invitation expiry. Android invitation
refresh resumes listening and ignores callbacks from a cancelled older wait. TLS pinning,
signed identity proof and explicit receiver consent remain enforced (ADR 0056).

Windows x64 portable packaging and Android ARM64 phone/TV debug and unsigned release
assembly succeeded. Installable debug previews and the Windows bundle are in `releases/`;
large binaries are local, Git-ignored artifacts. These are **BUILD_VERIFIED**, not
**HARDWARE_VERIFIED** across devices. The later PC-to-phone success is user-reported evidence.

## Remaining product work

1. Confirm the Windows plugin's capture backend on supported GPUs; dedicated WGC selection
   and guaranteed hardware encoding are not implemented or established by these builds.
2. Native Android sending and Windows receiving are now source/build integrated; new
   direction and cleanup behavior need device validation. System audio and synchronized
   annotations remain incomplete.
3. Linux still lacks its fully integrated media path. iOS has source for send/receive
   and broadcast capture, requiring Apple tooling and signing for build verification.
4. Browser receiver still lacks the authenticated signaling integration required to stream.
5. Internet relay configuration, multi-party sessions, polished installers and store packages
   remain incomplete. The current path is direct LAN video only.

Final physical media validation, performance measurement, device compatibility, owner signing
and release/store operations are still pending. No percentage or project-wide completion is claimed.

## Branch cleanup

Test sources, fuzz targets, historical task briefs, obsolete spike notes and GitHub Actions
were removed from this branch at the owner's request. Production Rust modules, native apps,
platform foundations, locked dependencies and architecture/security documentation remain.
No unit or instrumentation test command was run. The old files remain recoverable from `dev`.
