# Connect MirrorMesh devices - Windows 0.3.0 / Android 0.2.1

Upgrade both ends to 0.2.0 to use compact invitations. An IPv4 invitation now has 121
characters (IPv6: 137), with the same full fingerprint, 128-bit nonce and two-minute expiry.
Older long invitations still work in the new apps; older apps cannot read the short form.

## Windows PC to Android phone or Android/Google TV

Install the native `phone` or `tv` APK on the receiver and choose Start receiver. On Windows,
choose Find devices on this network. Search by name and select the receiver to request
a connection directly (both ends need 0.2.1). If more than one address appears, choose the
one on the same network as the PC. Compare all four groups of the connection code shown
on both devices. On the PC select The codes match; on Android select the permission checkbox
and Approve. Reject unexpected requests or mismatched codes. Then choose a screen/window
on the PC. Discovery alone never authorizes sharing. Older receivers use invitations;
manual invitations also remain available when multicast is blocked. Search only lists
active MirrorMesh receivers. Keep the receiver screen open while requesting a connection.

Generic TVs without the MirrorMesh Android/Google TV receiver are not supported by this
route. Miracast, AirPlay and Chromecast are separate protocols and are not implemented here.

## Android phone to PC, another Android phone or Android TV

On Windows choose Receive a device's screen / show this PC's QR. Select the PC's LAN
address and Start receiving. On Android receivers use Start receiver as before.

On the Android sender, use a camera/QR reader to open the `mirrormesh://` invitation in
MirrorMesh, or choose Share this device's screen instead and paste the decoded invitation.
Pair with receiver, approve the verified device on the receiver, then choose the screen
through Android's capture dialog. A foreground notification provides Stop sharing.
The receiving PC uses native WebRTC rendering. Use Stop receiving to close its listener
and connection. Capture does not start from merely opening a QR link.

### If Android → PC stops after capture approval

Update both devices to 0.2.1. Android now waits for offer submission before polling the
session. Its sharing page shows preparation, connection and failure stages; it no longer
reports streaming just because the service started. If a failure remains, note that stage
and the PC's MM-CONTROL diagnostic. Do not share the invitation, SDP or full network logs.

## iOS directions (source integrated; Apple build pending)

For **native iOS Control Center Screen Mirroring → PC**, use Windows 0.3.0's
Receive iPhone / iPad with AirPlay page. No MirrorMesh iOS app is needed for that path.
See the [AirPlay guide](airplay.md) for PIN, PC consent, builds and current limitations.
The following describes the separate MirrorMesh iOS app route.

The iOS home offers Share this device and Receive another screen / show QR. Use a receiver
invitation to send, or display the iOS receiver invitation on another sender. Camera app
links are stored for the Share page; pasting the decoded text is also supported.

Full-device iOS sending uses ReplayKit's system broadcast picker and a signed
MirrorMeshBroadcast Upload Extension. See [build instructions](build-release.md). Without
the configured extension the UI offers app-only capture and labels that scope explicitly.
The extension sends bounded frames to native WebRTC through the App Group's Unix socket;
screen pixels never pass through Dart or Rust FFI. Apple signing, native compilation,
background lifecycle and real-device behavior have not been verified on this host.

## Evidence boundaries

The owner reported the existing Windows-to-phone path working. New directions, discovery,
QR opening, TV interoperability, OS consent, stop/revocation and cleanup still need device
verification. A build is not a device result. LAN video remains the target; no cloud account,
TURN relay, system audio or arbitrary-network scanning is added.
