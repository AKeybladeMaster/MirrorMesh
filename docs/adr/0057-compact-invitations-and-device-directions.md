# ADR 0057 - Compact invitations and native screen directions

The owner requested shorter links, QR connections in both directions, Windows/TV support
and a Windows device search on `test_3`. Preserve the established native protocol and keys.

## Compact QR v2

Emit `mirrormesh://p/` plus canonical, unpadded URL-safe Base64 of these binary fields:
address family byte (4 or 6), IP octets (4/16), port (2, big endian), device ID (16),
SHA-256 peer fingerprint (32), random one-time nonce (16), Unix expiry (8, big endian).
Payload lengths are exactly 79/91 bytes, encoded as 106/122 characters. Full URI lengths
are 121/137 characters. Reject other lengths, characters and nonzero trailing pad bits.
Numeric endpoint policy, approved port list, 120-second maximum lifetime, receiver-side
monotonic expiry, single-use nonce and explicit approval remain unchanged. V1 links still
parse; old binaries cannot parse v2. Upgrade sender and receiver together. No URL shortener,
six-digit secret, account or cloud lookup is introduced. Base64 here is protocol data;
routine file edits remain plain text and reviewable.

## Shared session integration

Move the existing Windows sender worker into `mm-adapter-signaling::sender_driver`,
injecting the platform identity and local ID. Move the generic Android listener into
`native_service`, keeping Android aliases for existing JNI callers. The shared receiver
registry now serves Windows/iOS metadata commands too. Capture/frames stay in native
WebRTC/MediaProjection/ReplayKit; Dart/FFI receives control metadata only.

Windows can explicitly start a receiver and show its pinned invitation QR. An Android
sender opens a QR app link or pastes it, authenticates the receiver, waits for receiver
approval and separately asks Android for MediaProjection consent. Screen capture uses
a nonexported foreground service, a visible sharing notification and Stop action.
Android requires FOREGROUND_SERVICE and FOREGROUND_SERVICE_MEDIA_PROJECTION permissions;
the existing Android 17 local-network consent also applies to sending. No capture starts
from merely opening a link. The sender has a bounded initial connection deadline.

iOS source uses the same Rust control commands and pinned keychain identity. Full-device
ReplayKit broadcast requires a provisioned App Group and a signed Upload Extension.
An original Swift uploader uses the pinned plugin's local Unix-socket frame format.
It bounds frame size, rate and send duration; a partial/failed write ends broadcast.
Small JPEGs contain a valid ignored APP15 segment to exceed the pinned native reader's
initial 10 KiB read, keeping adjacent stream frames from crossing its message boundary.
The extension serializes socket writes and shutdown. Receiver teardown is idempotent,
owns its listener lifecycle and rejects stale asynchronous negotiation completions.
No frames or identity seeds are stored in the group container. Native iOS compilation,
project configuration, signing and hardware verification require Apple tooling.

## Discovery

Use the existing `_mirrormesh._tcp.local.` schema. Android NSD advertises public device
ID, receive role and codecs only while the user runs a receiver. Windows Search invokes
the bounded Rust mDNS browser on a worker and permits local name filtering. Correct the
browser's normal receive-timeout handling so an empty poll continues the search window.
Results are unauthenticated hints; selecting one still requests the receiver invitation.
Do not publish QR secrets, auto-approve, scan subnets, or claim compatibility with generic
Miracast/AirPlay/Chromecast TVs. Android/Google TV uses the native MirrorMesh TV APK.

## Evidence

Use builds, formatters and static analysis only under this branch's operating contract.
No unit, instrumentation or workflow tests are introduced. Successful compilation does
not establish a new device path, signing readiness or production completeness.
