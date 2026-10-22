# Use MirrorMesh

For current 0.4.0 directions, approval popups and native casting availability, see
[Connect MirrorMesh devices](connections.md). Upgrade Windows and Android together.

## Install

**Windows:** extract `releases/MirrorMesh-windows-x64.zip`, then open `MirrorMesh.exe`.
Keep every DLL and the `data` folder beside it. You can also run the executable in
`releases/windows-x64`. The current executable is unsigned.

**Android phone/tablet:** install the `phone` APK in `releases/android-phone`.
**Android TV/Google TV:** install the `tv` APK in `releases/android-tv`.
ARM64 is the default build. The receiver requires an Android device with a hardware
H.264 decoder. The native receiver supports Android 5/API 21 and newer, subject to that
codec requirement. The user reported successful PC-to-phone sharing on 0.1.1;
the 0.1.2 interface changes still need on-device accessibility and layout review.

The `debug` APKs are locally signed previews. The `release-unsigned` APKs cannot be
installed until signed. Local previews and releases have the same application ID;
Android will reject an update signed with a different key. Use your permanent signing
key for distributed releases. Uninstalling clears the receiver identity and stored trust.

With an explicitly authorized ADB device connected, installation can also use:

```powershell
adb install -r .\releases\android-phone\MirrorMesh-phone-arm64-v8a-debug.apk
adb install -r .\releases\android-tv\MirrorMesh-tv-arm64-v8a-debug.apk
```

## Pair and share

1. Put the PC and receiver on the same trusted Wi-Fi or Ethernet LAN. Guest networks
   with client isolation will often prevent direct communication.
2. Open MirrorMesh on Android. Choose **Start receiver**; the active Wi-Fi/Ethernet address is selected automatically.
   On Android 17+, allow the local-network permission when prompted. Denial leaves pairing stopped.
3. Use **Copy invitation** or **Share invitation** to transfer the complete text to the PC.
   On a TV, scan the QR with a phone and transfer the decoded text. A QR text reader is
   sufficient; MirrorMesh does not need an account or a third-party identity connector.
4. Paste the full invitation into the Windows app and choose **Pair with receiver**.
   An invitation expires after two minutes and is single use. Refresh it before retrying.
   Refreshing the code also resumes the receiver's listener; no separate Wait press is needed.
5. On Android, inspect the sender name, LAN address and fingerprint. Select **Allow view**
   and **Approve selected permissions**. Reject unfamiliar requests. Approval alone does not start capture.
6. On Windows choose **Choose what to share**, then select a screen or window.
   The selected source is captured by the native WebRTC plugin and sent as H.264.
7. Android opens the fullscreen renderer. **Save screenshot** stores a received frame as JPEG.
   Android 10+ uses `Pictures/MirrorMesh`; older devices use the app's external-files Pictures folder.

In Android 0.1.2, **Hide controls** expands the viewing area; **Show controls** remains
available to reopen the panel. The panel scrolls when text is enlarged. **Connection details**
contains frame counters and **Revoke access**, which asks for confirmation. **Disconnect**
ends the current session without revoking saved approval. Screenshot results remain visible
instead of being replaced by frame counters. Controls never disappear on a timer.

Windows encoded-frame and byte counters confirm sender activity only. A visible changing
image on Android is needed to confirm reception and presentation.

## Stop, reject, reconnect and revoke

- **Stop sharing** or **Disconnect** closes capture, tracks and the authenticated channel.
- A short network interruption has a ten-second recovery window. If recovery fails, media
  closes. Refresh the receiver invitation and pair again for a new session.
- **Reject** refuses a pairing request without storing the sender as trusted.
- **Revoke access** in the receiver's video screen saves revocation and ends the connection.
  A storage failure is reported rather than presented as successful revocation.
- Closing a session does not revoke stored approval. A new session in this release still
  requires a fresh invitation and explicit approval; there is no silent trusted reconnect.
- Leaving the Android video activity stops its media session. Stopping the receiver closes
  its listener. Capture never begins from merely opening the Windows app.

## Troubleshooting

For TV installation by USB drive, phone transfer or wireless ADB, follow the
[Android TV installation guide](install-android-tv.md). TV hardware has not yet been verified.

| Symptom | Action |
| --- | --- |
| No Android address | Connect to Wi-Fi/Ethernet and retry Start receiver; allow local-network permission |
| Pairing fails or expires | Refresh the invitation; check both clocks and the chosen LAN address |
| No request appears | Start receiver/Wait for sender; avoid guest isolation; check app-specific firewall access |
| Paired but no video | Select a source on Windows; inspect codec and encoder diagnostics; receiver needs hardware H.264 |
| No first encoded frame | The sender stops after its startup deadline; check the GPU driver and source availability |
| Connection cannot recover | Wait for cleanup, refresh the invitation and pair again |
| APK update rejected | Check ABI and signing certificate; use the same key as the installed package |

The control connection uses TCP 8765. WebRTC uses negotiated UDP ports for encrypted
media. Configure app-specific access on a trusted network when required; do not disable
Windows Defender or the firewall. There is no TURN relay, arbitrary subnet scan or
synchronized annotation. App screen sharing is video-only; native AirPlay uses its separate
audio/video pipeline. See the current connection guide for Android sending and PC receiving.

For logs, launch Windows via `flutter run -d windows` from its app directory. Android
diagnostics are visible in the receiver and via `adb logcat`; share only relevant lines,
excluding invitations or private network information. **Copy diagnostics** in Windows
copies media counters and errors, not the invitation.
