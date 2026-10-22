# ADR 0058 — Direct LAN requests and Android sender startup

## Reason

The owner reported Android 13 (Redmi Note 5) → Windows failing after capture consent,
with a generic control-channel error. The owner also requested connection approval on
a discovered receiver instead of manually copying its invitation. Work stays on test_3.

## Android startup

Creating and setting a WebRTC offer is asynchronous. The sender scheduled event polling
immediately; the shared driver returned SessionNotStarted before send_offer had run,
and the service stopped capture/closed signaling. Poll only after successful offer
submission. The shared driver also returns Pending for an idle approved channel.
Show preparation/negotiation/connected/failure stages on Android. Do not claim streaming
merely because the foreground service started. PC diagnostics identify protocol, closed
channel and transport failures without disclosing SDP, ICE, invitations or raw errors.
This source ordering defect is confirmed; the owner's physical failure is not reproduced
on this host, and another device-specific cause may remain.

## Direct request protocol

- Keep QR TLS fingerprint pinning, signed pair.begin, 128-bit single-use nonce, two-minute
  invitation lifetime, source admission, SQLite trust, grants and explicit consent.
- An active receiver can advertise pair=qr,request and a ninth TXT field fp (the full
  public SHA-256 fingerprint). The eight-field pair=qr form remains accepted. Discovery
  is unauthenticated: neither the name, ID nor fingerprint establishes identity.
- Windows connects only to the selected private/link-local numeric endpoint on port 8765,
  with TLS 1.3 Ed25519 key possession and the advertised fingerprint as its provisional pin.
  The existing /v1/control WebSocket remains unavailable to browser Origin requests.
- Before pair.begin, a direct client sends exactly {"v":1,"request":"invitation"}.
  The receiver issues a fresh invitation over that same TLS channel, then expects the
  existing signed pair.begin. No invitation/nonce is published over mDNS. This bootstrap
  has its own bounded per-source attempt limiter and the normal handshake deadline.
- The client checks endpoint, receiver ID, fingerprint and expiry against its selection.
  The receiver binds LAN-issued nonces to this bootstrap connection; a QR-path or other
  socket cannot redeem them. Nonce provenance lives with the shared runtime, so restarting
  a listener cannot downgrade a LAN challenge to QR approval. Only the view grant is
  supported for direct requests.
- Both display the first 64 bits of SHA-256(domain || receiver fingerprint || exact
  UTF-8 signed pair.begin frame), in four groups of four hex characters. Domain is
  `MirrorMesh LAN pairing code v1` followed by a zero byte. The pin binds the code to
  the receiver key; the proof binds the sender identity, nonce, requested grant and name.
  A MITM substituting a key produces a different code. Users must compare every group;
  this is interactive authentication and depends on that comparison.
- The receiver's unchecked permission checkbox explicitly confirms matching codes. The
  Windows sender requires its own matching-code confirmation as well as receiver approval
  before native state becomes Approved or any capture/session command can start. Code
  confirmation expires 120 seconds after receiver approval. Cancellation remains available.
- No automatic acceptance, silent capture, subnet probing, new session engine or cloud
  service is introduced. TLS post-handshake writes now have a two-second timeout, keeping
  signaling sends/closure bounded. Advance monotonic time during a blocking accept so
  nonce verification does not use the clock value from before an idle wait.

## Compatibility and evidence

Upgrade both apps to 0.2.1 for direct requests. Older receivers continue through invitations;
older discovery parsers may omit the extended advertisement. Generic Miracast/AirPlay/
Chromecast receivers remain unsupported. Android/Google TV needs the MirrorMesh TV app
running in receiver mode. The new controls work through the same Rust driver as QR sessions.
No database schema or platform permission changes. Dependency versions stay locked; signaling
reuses the workspace's existing sha2 version. Builds/static analysis are permitted; test
suites and hardware verification remain deferred under the branch contract.
