# Windows x64 portable bundle

Open `MirrorMesh.exe` in this folder. Keep its DLLs and `data` directory together.
Windows 0.4.0 retains the AirPlay code during iOS entry and includes the repaired runtime
in `airplay`; keep that whole folder. A code is displayed after iOS requests one.
`airplay.md` explains native iPhone/iPad Screen Mirroring. For startup diagnostics, run
`powershell -NoProfile -File .\diagnose-airplay.ps1 -CheckStartup` from this folder.
`USER_GUIDE.md` explains pairing, capture, stopping and diagnostics. This is an unsigned
release candidate. Upgrade the Android app to 0.4.0 too. Incoming approval shows reported
device facts and requires permission. Addresses are selected automatically and hidden
outside approval. New dialog/layout behavior still needs physical verification.
Miracast opens the installed Windows Wireless Display app in its own window; native
Google Cast reception is unavailable without a compatible Windows provider.

Rebuild from the repository root: `scripts/build-windows.ps1`.
