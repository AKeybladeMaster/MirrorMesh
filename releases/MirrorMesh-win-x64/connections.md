# Connect MirrorMesh devices - 0.4.0 / test_5

Install the new Windows and Android apps together. Device facts use signed v2 pairing
proofs, which require upgraded receivers. New receivers still accept old v1 senders.
Compact invitations retain the full pinned fingerprint, nonce and two-minute expiry.

## PC to Android phone or Android/Google TV

1. Open MirrorMesh on the phone/TV and choose Start receiver. Wi-Fi or Ethernet is selected
   automatically. Allow local-network access if Android requests it.
2. On Windows choose Find devices on this network and select the receiver. Compare every
   group of the connection code in the popups on both screens. Confirm matching codes on
   the PC; on Android review the device name/model/system, check permission and Approve.
3. Choose what to share on the PC and select a screen or window. Pairing never starts capture.

If multicast discovery is blocked, use Copy/Share invitation on the receiver and paste it
into the PC app. On a TV, scan the QR using your phone and transfer the decoded invitation.
The invitation is not printed in setup; copy/share still transfer its full authenticated text.
Reject unexpected requests or mismatched codes. Device facts are sender-reported hints,
not hardware attestation. Only the incoming approval popup shows the actual peer address.

Keep the receiver page open during pairing. Generic TVs need the MirrorMesh Android TV
app for this route. A browser receiver is not interchangeable with the native TLS listener.

## Android phone or TV to PC

On Windows choose Receive a device's screen / show this PC's QR, then Start receiving.
On Android choose Share this device's screen, open the receiver QR using a camera/QR reader,
or paste its invitation. Pair, approve the Android device on the PC, then choose a screen
using Android's system capture dialog. A foreground notification provides Stop sharing.

The TV APK has the same sender source. The TV OS must offer MediaProjection capture;
protected content may be blank and some TV firmware may not permit screen capture.
TV sending remains hardware verification pending. No capture permission is bypassed.

The PC fits received video inside its window. Stop receiving closes its listener/session.
Capture never begins merely by opening a QR link. Android-to-Android/TV uses the same
receiver invitation and explicit approval flow.

## PC to iOS MirrorMesh app (source integrated; macOS build pending)

In the iOS app choose Receive another screen / show QR, then Start receiving. Copy its
invitation to the Windows app, pair, approve the reported PC in the iOS popup, then select
a screen/window on Windows. The shared Rust engine handles pinned signaling and approval;
native WebRTC renders video. No pixels go through Dart/FFI.

An iOS app binary has not been built on this Windows host. Use the macOS/Xcode instructions
in [build-release.md](build-release.md) for compilation, signing and installation. No IPA
or device-verified iOS app is claimed. The separate Share action uses ReplayKit and needs
its provisioned Broadcast Upload Extension for whole-device capture; app-only capture is
explicitly labelled if the extension is absent.

## Native iPhone/iPad Screen Mirroring to PC

Use Receive iPhone / iPad with AirPlay, then Start. On iPhone/iPad open Control Center,
Screen Mirroring, MirrorMesh PC. Enter the PC popup's code on iOS. Review the reported
name/model and source address in the PC approval popup, check permission and Approve.
OS/manufacturer details unavailable through AirPlay are shown as not supplied. No iOS
MirrorMesh app is needed. The code expires after two minutes; Stop closes the native helper.
See [AirPlay details](airplay.md). The owner confirmed native AirPlay connections before
0.4.0; the new dialogs still need physical verification.

## Native Android wireless display options

On Windows choose Android wireless display / Cast options:

- **Wireless display / Miracast:** opens Microsoft's installed Wireless Display app in its
  own window. Windows handles device approval, rendering and disconnect. Install the
  optional feature in Settings > System > Projecting to this PC if it is missing. A
  compatible Wi-Fi adapter and Android Miracast sender are required. This is an external
  system receiver handoff, not an embedded MirrorMesh session. Check its own support message.
- **Google Cast:** unavailable because there is no compatible Windows receiver SDK/provider.
  Google's public receiver SDK targets Cast devices/Web/Android TV. The option reports this
  limitation and offers the MirrorMesh Android route; it does not pretend to start casting.
- **MirrorMesh app:** opens the PC QR receiver page for the Android app.

Switch modes after ending the current sharing session. Close Wireless Display in its own
window when finished; MirrorMesh does not terminate a system-owned receiver process.

## Stop, privacy and verification

Use Stop sharing/Disconnect to end capture; use Stop receiving to close the listener.
Reject grants no trust. Revoke access in the Android viewer persists revocation and ends
its session. Android screenshots save a received frame and report completion in a popup.
Invitations are single use and expire; refresh before pairing again. Guest/client-isolated
networks can prevent connections. No cloud account, arbitrary subnet scan, TURN relay,
silent capture, automatic approval or firewall-security change is added.

PC/Android sharing and native iOS connections were owner-confirmed before these changes.
Build evidence and source status are separate from new-device behavior. See
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for exact readiness and remaining work.
