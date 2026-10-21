# Implementation status — test release integration

Branch baseline: `test_2` at `ef05d95`. Current work belongs exclusively to `test_3`.
This document supersedes historical increment counts for this release candidate.

## Readiness labels

- **Source implemented:** integrated code exists; this alone does not prove runtime behavior.
- **Build verified:** an actual compiler/package build succeeded for the stated target.
- **Automated integration verified:** not claimed for this branch; user requested no test suites.
- **Hardware verified:** the user reports successful Windows-to-Android-phone screen sharing
  on the 0.1.1 baseline. Device/OS/GPU details, TV behavior and new UI behavior remain unverified.
- **CODE_COMPLETE:** the complete agreed product requirements are implemented; not yet achieved project-wide.

## Current reference pipeline

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
