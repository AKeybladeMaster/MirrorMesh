# Release readiness — 0.4.1

This repository checkpoint consolidates the `test_6` implementation for `main` via
`pre_release`. It is the owner's final repository release for now, not a claim that
every original specification requirement or platform has full acceptance.

## Evidence labels

- **Source implemented:** integrated code exists.
- **BUILD_VERIFIED:** the stated compiler/package command completed successfully.
- **HARDWARE_VERIFIED:** stated device behavior was observed/reported; builds alone do not establish it.
- **CODE_COMPLETE:** all agreed product requirements are implemented; not achieved project-wide.

| Target/direction | Implemented source | Compilation/package evidence | Device evidence / limits |
| --- | --- | --- | --- |
| Windows → Android app | Approved pairing, native capture/WebRTC/H.264, native Android rendering | Windows x64 and phone/TV ARM64 packages built | Earlier phone sharing owner-confirmed; TV unverified |
| Android app → Windows | Explicit MediaProjection, authenticated signaling, fitted receive view | Phone/TV APK and Windows builds pass | Earlier phone sharing owner-confirmed; TV capture depends on its OS |
| Native iOS AirPlay → Windows | Native helper, PIN retention, explicit PC approval, fitted rendering | Complete Windows helper/runtime package built | Earlier iPhone/iPad connections owner-confirmed; latest changes need device confirmation |
| Linux desktop | Shared Rust pairing/identity/trust and native desktop WebRTC path | Ubuntu 24.04 x64 tarball built; dependencies resolve | Desktop keyring/capture/Wayland/codec/media behavior unverified |
| macOS desktop | Swift host, shared Apple adapter, static Rust FFI and universal build pipeline | Unbuilt: no Mac/Xcode available | No .app/DMG, Keychain, capture or playback claim |
| iOS app | Native receive mode, ReplayKit app/extension sender source | Unbuilt: no Mac/Xcode/provisioning available | No IPA or device behavior claim |
| Native Android Miracast → Windows | OS Wireless Display handoff and live capability/listening popup | Windows host and diagnostic script built/checked | Owner reports phone stuck Connecting with no Windows request; unresolved |
| Native Google Cast → Windows | No compatible provider integrated | Not available | No free compatible provider verified; no paid trial/license installed |

App screen sharing is video-only. AirPlay has its separate native media pipeline.
No cloud login, TURN relay, automatic approval or arbitrary network scan is configured.
Reported device facts are not hardware attestation.

## Preserved build evidence

Implementation checkpoint `c22b9a0` built Windows 0.4.1+10, Linux x64 and Android phone/TV
0.4.1/code10 debug plus optimized unsigned APKs. Commands exited 0. Windows archive
verified 285 payload digests; Linux verified 19 and no world-writable archive members;
all four APK sidecars/version metadata matched. Those package hashes describe their exact
earlier bytes; repackaging guides changes hashes without proving new device behavior.

The release consolidation preserves native media/protocol code and dependency pins,
removes archived prototypes/reports, and makes build scripts independent of branch names.
Current build results and reproducible commands are in [verification](verification.md)
and [build instructions](build-release.md).

## Deferred activities

- TV installation, D-pad usability, decoding and OS capture compatibility.
- Linux desktop keyring, discovery, capture/portal permissions, codecs and real media.
- Mac/iOS compilation, signing, provisioning, Keychain and real media on Apple hardware.
- Native Miracast listening/handshake/driver interoperability diagnosis.
- Latest UI/cleanup/reconnection/accessibility behavior on actual devices.
- Owner signing, notarization/store submission and wider distribution decisions.

Historical browser prototype and duplicate Flutter Android shell are preserved at
`test_6` / `4052f40`; neither is included as a final product in this release tree.
No tests, fuzz targets or GitHub Actions are included/run under the owner's branch policy.
