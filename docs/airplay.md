# iPhone and iPad → Windows with native Screen Mirroring

Windows 0.3.1 repairs the startup packaging of the locally built AirPlay receiver preview.
It is separate from the
MirrorMesh iOS app and does not require an IPA, ReplayKit extension or Xcode signing.
The owner has confirmed PC ↔ Android on 0.2.1; this new AirPlay path has not been tried
on physical devices. Intended devices: iPhone 15 Pro Max and base iPad from 2020;
their iOS/iPadOS versions have not been recorded.

## Connect

1. Extract the **complete** Windows ZIP, including its `airplay` folder. Open MirrorMesh.
2. Choose **Receive iPhone / iPad with AirPlay**, then **Start iPhone / iPad receiver**.
3. Put the PC and Apple device on the same local network. Open Control Center on the
   iPhone/iPad and choose **Screen Mirroring → MirrorMesh PC**.
4. Enter the four-digit code displayed on the PC. On the PC, choose **Allow** for the
   device you just selected. Unexpected requests should be rejected; consent expires
   after 30 seconds. A fresh authentication code is required for another connection.
5. The received picture fits the available window without cropping. Resize/maximize the
   window or use **F11** for full screen; **Escape** exits full screen. Stop receiving
   closes the receiver and its media processes. Closing the page or app also stops it.

No network listener starts merely by opening the AirPlay page. Starting reception makes
the PC discoverable on the LAN. No media recording, metadata file export or persistent
AirPlay trust/key file is enabled. Standard MirrorMesh QR connections retain their own
identity, approval and TLS fingerprint checks.

## If the PC is absent or video fails

If 0.3.0 stops immediately, replace it with the complete 0.3.1 ZIP, extracted into a
new folder. The previous packager put the core GStreamer DLL in the plugin directory;
Windows could not load the helper (exit `0xC0000135`). This occurs before discovery or
network access, so changing firewall settings cannot fix that packaging error.
The 0.3.1 bundle also includes `autodetect`, required by UxPlay's startup plugin check.

Keep the receiver page open and wait until it says Ready. Check that both devices use
the same reachable LAN and that guest-network/client isolation is off for that network.
The receiver uses mDNS UDP 5353 and TCP/UDP 7100–7102. If Windows asks, permit the
receiver on your trusted **private** network. The app does not edit firewall settings
or disable Windows Defender. Missing components require the complete bundle, not an
EXE copied by itself. Stop and start the receiver to clear a failed attempt.

For local diagnostics, run from the extracted Windows folder:

```powershell
# Verify bundled files and the Windows loader; no listener starts.
.\diagnose-airplay.ps1
# Opt in to a brief real receiver startup, then automatic shutdown.
.\diagnose-airplay.ps1 -CheckStartup
```

From the source checkout the command is `scripts/diagnose-airplay.ps1`. The second mode
briefly advertises on this LAN, uses the same PIN/approval and process ownership as the
app, requires Ready to persist for two seconds within a 15-second window, then closes
all receiver processes. This also observes initial asynchronous media errors. No client is
approved and no PIN, client identity or media is logged. It checks native initialization
and service registration, not iPad discovery or media compatibility.

Protected video may be blank. This preview does not bypass protected-content restrictions,
and does not provide certified AirPlay, AWDL peer-to-peer discovery, AirPlay multi-room
audio, HLS streaming, AirPlay sending from PC, or an Apple TV receiver application.
Screenshots through this new AirPlay path are not yet integrated. Native decoding/audio,
orientation, DPI, latency and reconnect still require device validation.

## Build

From the repository root on the authorized release branch (`test_3` for this repair;
`test_4` after the requested handoff):

```powershell
.\scripts\setup-airplay-toolchain.ps1
.\scripts\build-airplay.ps1
.\scripts\build-windows.ps1
```

Setup installs a workspace-local MSYS2 UCRT64 toolchain using the SHA-256-pinned official
base archive, exact package versions/hashes and required trusted package signatures.
It does not modify a system-wide MSYS2 install. Build checks the package lock and UxPlay
revision, applies the readable patch, builds generic x64 code (`NO_MARCH_NATIVE=ON`), and
collects the selected GStreamer plugins and their native import dependencies. No tests
or receiver are started by these scripts. `-WithoutAirPlay` produces a developer build
of the app without adding an AirPlay runtime. For Dart debugging, stage the generated
`.toolchains/airplay-runtime` directory as `airplay` next to the debug executable too.

UxPlay revision `15a6b87172eca38b0df4e615fbf1bba108734dde` is GPL-3.0-or-later. The local
preview contains its modified corresponding source, patch, build scripts, package inventory
and licenses. Dependency redistribution/source obligations still need review before a
public binary release. See `airplay/NOTICE.md` inside the generated bundle.

Primary references: [UxPlay source and Windows instructions](https://github.com/FDH2/UxPlay),
[GStreamer D3D11 renderer/decoder](https://gstreamer.freedesktop.org/documentation/d3d11/index.html),
[MSYS2 package sources](https://www.msys2.org/dev/package-sources/).
