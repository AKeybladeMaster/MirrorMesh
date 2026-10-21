# Connect MirrorMesh devices - 0.2.0

Upgrade both ends to 0.2.0 to use compact invitations. An IPv4 invitation now has 121
characters (IPv6: 137), with the same full fingerprint, 128-bit nonce and two-minute expiry.
Older long invitations still work in the new apps; older apps cannot read the short form.

## Windows PC to Android phone or Android/Google TV

Install the native `phone` or `tv` APK on the receiver and choose Start receiver. On Windows,
choose Find devices on this network. Search by name, select the receiver, then copy its
invitation into the Windows app and pair. Select Allow view and approve on Android before
choosing a screen/window on the PC. Discovery is an aid to selection; it does not authenticate
a device or replace the pinned invitation. Empty results may mean multicast is blocked;
manual invitations remain available. Search only lists active MirrorMesh services.

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

## iOS directions (source integrated; Apple build pending)

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
