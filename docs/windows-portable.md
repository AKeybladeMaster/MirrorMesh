# MirrorMesh Windows portable app — 0.4.1

Extract the complete ZIP into a new folder and open `MirrorMesh.exe`. Keep its DLLs,
`data/`, and the complete `airplay/` companion folder together. This preview is unsigned.

Use **Receive a device's screen** for MirrorMesh Android/iOS app senders, or select a
receiver to send the PC screen. Review device facts and explicitly allow/approve each
connection. Native iPhone/iPad Screen Mirroring uses **Receive iPhone / iPad with AirPlay**;
its PIN appears after an Apple device requests authentication. See `airplay.md`.

Android Wireless display/Miracast opens the separate Windows receiver and reports OS
listening state. Actual Miracast connection failure is unresolved. Google Cast is unavailable.

See `USER_GUIDE.md`, `connections.md` and `install-android-tv.md` for use/install instructions.
`diagnose-airplay.ps1` and `diagnose-miracast.ps1` provide explicit diagnostics. Do not share
invitations, PINs, private keys or personal receiver state in reports.

Keep required notices/licenses and matching companion source in this bundle. New-device
behavior and signing remain separate from compilation; this is not a production-complete claim.
