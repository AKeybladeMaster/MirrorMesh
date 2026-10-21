# Windows x64 portable bundle

Open `MirrorMesh.exe` in this folder. Keep its DLLs and `data` directory together.
Windows 0.3.1 includes the repaired AirPlay runtime in `airplay`; keep that whole folder.
`airplay.md` explains native iPhone/iPad Screen Mirroring. For startup diagnostics, run
`powershell -NoProfile -File .\diagnose-airplay.ps1 -CheckStartup` from this folder.
`USER_GUIDE.md` explains pairing, capture, stopping and diagnostics. This is an unsigned
release candidate. Physical sender-to-receiver media verification is still pending.

Rebuild from the repository root: `scripts/build-windows.ps1`.
