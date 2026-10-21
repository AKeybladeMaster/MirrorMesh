# ADR 0060: Device facts, automatic LAN selection and connection dialogs

Date: 2026-10-21. Branch: test_5. Status: implemented source; see implementation status for build evidence.

## Decision

Reuse the existing Ed25519 identity and pairing coordinator. Optional `device_details`
contains platform, manufacturer, model and OS version, each bounded to 80 Unicode scalars
with control and bidi formatting characters rejected. No serial, account or hardware ID
is collected. OS device names remain the existing signed display name.

A proof without details retains `MirrorMesh.PairingProof.v1\0` and its exact original
canonical bytes. A proof with details uses `MirrorMesh.PairingProof.v2\0`, the original
fields followed by four UTF-8 fields in the order above. Each field has a u32 big-endian
byte-length prefix. Details are attached before signing and checked before approval.
Removing or changing details invalidates the proof. This is identity binding of reported
facts, not attestation that a name, model or OS is truthful. UI labels reflect that limit.

New receivers accept v1 and v2 proofs. V2 senders require upgraded receivers. Upgrade the
Windows and Android apps together; do not silently retry using an unsigned metadata path.
TLS 1.3 identity pinning, QR nonce/expiry, explicit grants and capture consent are retained.

Windows reads public computer/BIOS/version facts. Android reads the OS device name and
Build fields. iOS uses UIDevice and uname, respecting Apple's generic-name privacy rules.
Profile data is snapshotted for each pairing worker; private-key storage is unchanged.

MirrorMesh chooses one LAN IPv4 address at receiver start. Windows ranks routed physical
Wi-Fi/Ethernet adapters by route/interface metric, with a connected physical LAN fallback.
Android selects the active physical Wi-Fi/Ethernet network; iOS uses the OS path monitor.
No router gateway is bound as the local address. No subnet probing, automatic firewall
rule or security-setting change is performed. No raw local address selector or invitation
text appears in setup. QR/copy/share still transfer the complete pinned invitation.
The observed peer source address appears only in incoming approval.

Dialogs own their routes/windows. Permission begins unchecked; Approve stays disabled
until selected. Direct discovery requests still require matching codes on both devices.
Start/approval/result feedback uses popups. An ongoing sharing/receiving indicator and
Stop remain visible. Native media never travels through Flutter/Dart/FFI.

## Native protocols

AirPlay keeps the isolated pinned helper and its existing PIN/consent gate. Its private
pipe now includes bounded hex-framed UTF-8 name/model/source-address fields, written as
one serialized event. The source address comes from the accepted socket; no device ID or
key is exported. Untrusted fields cannot insert control lines. Arbitrary upstream logs
remain excluded. Unsupported OS/manufacturer facts are shown as not supplied. A native
video overlay gate prevents its child HWND covering approval/result dialogs. PIN retention
and the 120-second expiry remain in effect. AirPlay uses its required native sockets;
the one-listener policy applies to the shared MirrorMesh pairing listener.

Miracast is an explicit handoff to Microsoft's installed Wireless Display package, found
using Windows package-query APIs and launched using IApplicationActivationManager. It
uses its own window, OS approval and cleanup. It is not an embedded MirrorMesh session,
does not inherit MirrorMesh trust, and does not prove this PC's Wi-Fi hardware is suitable.
The app reports a missing feature/launch failure. No optional feature is silently installed.

Google's public receiver SDK targets Web/Android TV receivers on Cast devices, not a
standalone Windows receiver. The owner has no compatible Windows provider/SDK. Native
Google Cast reception remains blocked; its option explains availability and the Android
MirrorMesh route. It does not advertise a Cast service or bypass device authentication.
An embedded Miracast receiver and an approved Google Cast provider remain separate work.

References: [Cast API scope](https://developers.google.com/cast/docs/reference),
[Miracast CoreApplicationView requirement](https://learn.microsoft.com/en-us/uwp/api/windows.media.miracast.miracastreceiver.createsession),
[Windows package queries](https://learn.microsoft.com/en-us/windows/win32/appxpkg/functions),
[Windows app activation](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationactivationmanager-activateapplication).

## Verification limits

Builds, formatting and static analysis are allowed. No unit/instrumentation suite or
workflow is introduced or run. New device flows, D-pad/VoiceOver/TalkBack layouts, large
text, route selection and native casting hardware remain physical-user verification.
iOS app compilation/signing requires macOS/Xcode; no IPA is claimed on Windows.
