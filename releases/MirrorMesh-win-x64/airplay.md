# iPhone and iPad → Windows with native Screen Mirroring

Windows 0.4.0 shows the retained AirPlay code and reported device name/model in popups,
with unchecked permission before approval. It includes the 0.3.1 startup packaging repairs.
The locally built receiver preview is separate from the
MirrorMesh iOS app and does not require an IPA, ReplayKit extension or Xcode signing.
The owner confirmed PC/Android sharing and native iPhone/iPad AirPlay connections before
0.4.0. The new dialogs need physical verification. Devices: iPhone 15 Pro Max and base iPad from 2020;
their iOS/iPadOS versions have not been recorded.

## Connect

1. Extract the **complete** Windows ZIP, including its `airplay` folder. Open MirrorMesh.
2. Choose **Receive iPhone / iPad with AirPlay**, then **Start iPhone / iPad receiver**.
3. Put the PC and Apple device on the same local network. Open Control Center on the
   iPhone/iPad and choose **Screen Mirroring → MirrorMesh PC**.
4. When iOS asks for a code, enter the four digits displayed in the PC's **AirPlay code**
   popup. The code appears after a device requests authentication, not merely at Start.
   On the PC, review the reported device name/model and observed source address, check
   permission and choose **Approve**. Missing OS/manufacturer facts are not invented.
   Unexpected requests should be rejected; consent expires
   after 30 seconds. A fresh authentication code is required for another connection.
   A pending code remains visible across brief connection gaps for up to two minutes.
   If it expires, the PC stops that receiver; start it again and reselect MirrorMesh PC.
5. The received picture fits the available window without cropping. Resize/maximize the
   window or use **F11** for full screen; **Escape** exits full screen. Stop receiving
   closes the receiver and its media processes. Closing the page or app also stops it.

No network listener starts merely by opening the AirPlay page. Starting reception makes
the PC discoverable on the LAN. No media recording, metadata file export or persistent
AirPlay trust/key file is enabled. Standard MirrorMesh QR connections retain their own
identity, approval and TLS fingerprint checks.

## If the PC is absent or video fails

If iOS asks for a code but the 0.3.1 PC app shows none, use the complete 0.3.2 bundle.
The old host cleared the code when iOS closed its short authentication connection.
0.3.2 retains it until authentication advances or the code expires, without granting
approval automatically. Stop the old receiver before starting the updated app.

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
# Diagnose code delivery with your iOS device; it never prints the code or approves it.
.\diagnose-airplay.ps1 -CheckPin
```

From the source checkout the command is `scripts/diagnose-airplay.ps1`. The second mode
briefly advertises on this LAN, uses the same PIN/approval and process ownership as the
app, requires Ready to persist for two seconds within a 15-second window, then closes
all receiver processes. This also observes initial asynchronous media errors. No client is
approved and no PIN, client identity or media is logged. It checks native initialization
and service registration, not iPad discovery or media compatibility.
`-CheckPin` additionally waits for you to select MirrorMesh PC within 15 seconds and
requires code availability to persist for two seconds. It reports only CODE_AVAILABLE
or failure, then stops the receiver. Use the normal app to enter a code and connect.

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
