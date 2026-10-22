# Shared control protocol

MirrorMesh's native apps reuse one Rust control plane. Native WebRTC carries media;
the application does not define a custom raw-frame protocol through Dart/FFI.

## Discovery and invitations

DNS-SD `_mirrormesh._tcp` supplies bounded local hints. Discovery does not authenticate
a peer or grant capture/access. A user explicitly starts a receiver on one automatically
selected physical LAN interface; ordinary setup hides its numeric addresses.

Current compact invitation URIs retain receiver identity/fingerprint, numeric LAN endpoint,
nonce and expiry. The legacy invitation parser is retained. Invitations expire within
the existing 120-second window and nonces are single-use. Copy/QR must preserve all
authentication fields; a generic URL shortener is not part of the protocol.

QR/invitation pairing pins the receiver's Ed25519 identity through TLS 1.3 raw-public-key
authentication and its CertificateVerify checks. The discovered-device request path uses
an explicit code comparison/confirmation flow. Do not treat an mDNS result as a trusted key.

## Pairing and device facts

Rust validates bounded `pair.begin` fields, identity proof, nonce/expiry and source admission
before returning an incoming request. The receiver explicitly chooses grants and approves
or rejects. Only accepted scoped grants are persisted; rejection does not persist trust.

Optional `device_details` carries bounded platform/manufacturer/model/OS fields. Legacy
proofs retain their v1 bytes; facts use a domain-separated v2 canonical signed proof.
Names/models are sender-reported facts, not hardware attestation. New fact-bearing
senders require updated receivers. See [ADR0060](adr/0060-device-facts-and-private-connection-ui.md).

## Approved session messages

The retained authenticated channel exchanges `session.offer`, `session.answer`, ICE,
`session.start`, `session.stop` and revocation controls. Rust enforces expected state,
session/identity/grant binding, ordering and bounded messages. Native WebRTC negotiates
SDP/ICE and encrypted DTLS-SRTP media. Merely pairing does not grant OS capture permission.

Sender startup requires explicit source/OS consent and native capture. Sender encoded-frame
diagnostics are separate from receiver presentation evidence. A short interruption has a
bounded recovery period; terminal failure closes local capture/peer resources and signaling.
Stop and revocation remain distinct: revocation persists the peer's revoked state.

The internal metadata command API returns explicit stop/revoke events; Android, iOS and
desktop callers release their resources for both. Pixel/audio buffers do not cross it.
See [ADR0058](adr/0058-direct-lan-requests-and-sender-startup.md) and
[ADR0061](adr/0061-shared-desktop-platform-integration.md).

## Separate native protocols

AirPlay uses the Windows native helper's PIN and explicit PC approval pipeline; it does
not disable or replace MirrorMesh TLS pinning. Miracast is handed off to the Windows
Wireless Display app, which owns approval, rendering and disconnect. Google Cast is not
implemented. A browser cannot use the native raw-public-key WSS flow as a substitute
for a browser-trusted certificate; no browser receiver product is released here.

Authoritative validators/encoders are in `mm-domain`, `mm-security`, `mm-protocol`,
`mm-application`, `mm-runtime`, `mm-adapter-tls` and `mm-adapter-signaling`.
