# Implementation status — test release integration

Branch baseline: `dev` at `5f3c957`. Current work belongs exclusively to `test`.
This document supersedes historical increment counts for this release candidate.

## Readiness labels

- **Source implemented:** integrated code exists; this alone does not prove runtime behavior.
- **Build verified:** an actual compiler/package build succeeded for the stated target.
- **Automated integration verified:** not claimed for this branch; user requested no test suites.
- **Hardware verified:** no physical sender/receiver session has been run in this work.
- **CODE_COMPLETE:** the complete agreed product requirements are implemented; not yet achieved project-wide.

## Current reference pipeline

| Stage | Source | Build evidence | Hardware |
| --- | --- | --- | --- |
| Windows local identity and QR-pinned WSS client | Implemented | Rust workspace compiled | Pending |
| Android identity, TLS listener, approval, SQLite trust | Implemented | ARM64 JNI compiled | Pending |
| Windows real pairing and source-selection UI | Implemented; demo home removed | Flutter analysis passes; final EXE rebuild in progress | Pending |
| Windows native desktop capture, H.264 WebRTC sender | Implemented through pinned plugin | Earlier Windows native build passed; final rebuild in progress | Pending |
| Android receive-only WebRTC, hardware H.264, fullscreen surface | Implemented | Phone and TV APK assembly passed | Pending |
| Receiver screenshots | Implemented using received native frame | APK assembly passed | Pending |
| Stop/reconnect/startup deadlines and cleanup | Implemented | Final compilation in progress | Pending |
| Persisted receiver revocation and authenticated notification | Integrated | Android APK assembly passed | Pending |
| Android 17 local-network consent, phone layout, invitation sharing | Integrated | Android APK assembly passed | Pending |

## Remaining product work

1. Confirm the Windows plugin's capture backend on supported GPUs; dedicated WGC selection
   and guaranteed hardware encoding are not implemented or established by these builds.
2. Native Android sending, Windows receiving, system audio and synchronized annotations
   remain outside the completed reference path.
3. Linux and iOS retain identity/platform foundations but lack a fully integrated native
   capture-to-receiver path. iOS needs Apple tooling and signing for build verification.
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
