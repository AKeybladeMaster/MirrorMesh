# MirrorMesh protocol v1 — implementation contract (candidate draft)

The Rust parser is implemented in `crates/mm-protocol` and is covered by the `session.offer` and candidate `pair.begin` golden fixtures plus malformed-input tests. `mm-runtime` can parse a bounded `pair.begin` frame, reserve source admission, convert it to the application proof, and verify it through the coordinator. `mm-security` provides strict manual IP parsing and a bounded, single-use 128-bit pairing nonce registry. `mm-domain` defines a validated trusted-peer record, `mm-ports` exposes storage and identity contracts, and `mm-application` coordinates QR invitation, signed-proof verification, nonce use, and explicit approval. No production listener, TLS channel, QR UI, or media transport is implemented. The candidate still needs cross-track review and CI before protocol v1 is frozen.

This document defines the **app-owned control protocol**; WebRTC media conforms to negotiated SDP/ICE/DTLS-SRTP RFC behavior, not a homegrown packet protocol. No claim of interoperability with AirPlay, Cast or Miracast.

## Endpoints

MVP receiver local signaling server listens at `wss://<receiver-host>:8765/v1/control` behind TLS, after device pairing approval. Discovery browses mDNS `_mirrormesh._tcp.local`. The proposed v1 DNS-SD record has exactly these TXT keys: `v=1`, `id=<UUID>`, `roles=send|recv|send,recv`, `pair=qr`, `tls=1`, `video=h264`, `audio=opus|none`, and `max=<width>x<height>@<fps>`. The service instance label is the user-selected display name; SRV/address records provide the signaling endpoint, and the current adapter accepts only port `8765`. The endpoint-preserving `DiscoveryCandidatesPort` carries resolved socket addresses, including IPv6 interface scope, through `mm-application::DiscoveryCoordinator`; that coordinator removes the local ID, endpointless results, and ambiguous duplicate IDs and returns a stable order. Non-default ports require a separate explicit approval path. Do not publish IP addresses, secrets, credentials, SDP, or other private data in TXT records. mDNS is unauthenticated: all fields, including the device ID, role, capabilities, name, and endpoint, are untrusted hints until QR proof and explicit approval complete. The current parser rejects advertisements with unknown or missing v1 fields. Provide manual address fallback.

The current manual-address parser accepts only numeric `IPv4:port` and bracketed `[IPv6]:port`; it rejects URL schemes, paths, credentials, query/fragment data, hostnames, IPv6 zone identifiers, and malformed or oversized input. The caller must pass the ports explicitly approved by the user; port 8765 is the documented default. The parser does not resolve DNS. DNS-name fallback remains deferred until a connection adapter can resolve once, pin the chosen socket address for the connection, reject redirects, and verify the peer's pinned identity.

First-time QR flow: sender obtains `host,port,device,fingerprint,nonce,expires`; it must verify the receiver identity against the QR fingerprint when establishing the eventual authenticated connection, then send a one-time signed proof. `parse_pairing_qr` accepts only the exact `mirrormesh://pair` scheme, the seven v1 fields, literal IPs on explicitly approved ports, canonical UUID/fingerprint/nonce encodings, and Unix expiry within 120 seconds. The nonce helper generates 16 random bytes using the operating-system random source, encodes exactly 32 lowercase hexadecimal digits, and tracks up to 1,024 outstanding values for single use. Its 120-second deadline uses the issuing process's monotonic clock; the QR's Unix expiry is a separate wall-clock value. `PairingCoordinator` rejects wrong receiver IDs, self-pairing, and unknown/expired nonces before signature verification; it then verifies the sender proof, computes the presented key fingerprint, consumes the nonce once, and returns a pending approval. Trust is saved only after explicit approval of a subset of requested grants and before the original monotonic deadline. The signed proof bytes are specified in [ADR-0006](adr/0006-pairing-coordinator-proof-contract.md). Ed25519 raw 32-byte public keys, raw 64-byte signatures, and SHA-256 fingerprints over the raw key bytes are proposed in [ADR-0007](adr/0007-ed25519-identity-candidate.md); a stateless verifier candidate is implemented in `mm-adapter-identity`, but its acceptance profile and cross-track/security review remain pending. `mm-runtime::PairingRuntime` composes the coordinator with identity and trusted-peer ports; integration tests use real test-only signatures and SQLite persistence. Proposed [ADR-0018](adr/0018-pairing-session-runtime-composition-candidate.md) records `MirrorMeshRuntime`, which composes pairing and trusted-session operations over one identity and peer store; its integration test uses a fake transport. Proposed [ADR-0019](adr/0019-trusted-session-grant-scope-candidate.md) requires each session-start caller to supply an operation grant scope and checks it against persisted approval before transport start; the current API cannot infer direction, and media/capture boundaries must enforce their own required scopes. There is no platform app assembly, local signer/key-lifecycle UI, QR UI, signaling listener, or TLS pinning implementation. `mm-security::PairingAttemptLimiter` provides a proposed attempt-admission candidate. Both `verify_proof_from` and `verify_pair_begin_from` reserve a per-source attempt before proof verification; the raw-frame path also counts malformed frames, then carries an admitted source address through pending approval. No listener uses these paths yet. See proposed [ADR-0015](adr/0015-pairing-attempt-limiter-candidate.md) and [ADR-0016](adr/0016-pairing-runtime-admission-candidate.md). The candidate `pair.begin` payload shape and encodings are recorded in [ADR-0020](adr/0020-pair-begin-wire-encoding-candidate.md); they remain unfrozen. A **6-digit pairing code requires an audited PAKE protocol**; do not implement plain SHA(code) or a six-digit shared secret in JSON. See [ADR-0004](adr/0004-trusted-peer-records.md), [ADR-0006](adr/0006-pairing-coordinator-proof-contract.md), and [ADR-0007](adr/0007-ed25519-identity-candidate.md).

### Sender proof construction

After parsing the QR and applying its expiry/receiver-fingerprint policy, sender code can use `PairingProof::create_signed` with the receiver ID and nonce, the local device ID, the sender display name, requested grants, and its protected local signing identity. The helper signs the canonical application proof bytes and returns a validated `PairingProof`; `mm-protocol::encode_pair_begin` then serializes those accessors into the candidate wire envelope. The proof helper does not validate the QR or authenticate TLS, and the wire encoder does not sign JSON. See proposed [ADR-0022](adr/0022-sender-proof-creation-candidate.md).

Control message JSON common envelope (after authenticated TLS handshake):

```json
{
  "v": 1,
  "type": "session.offer",
  "message_id": "b8298043-8b3b-4a86-86e3-79c0b863879b",
  "session_id": "8b3d8788-1476-4aae-a287-334a428b5b6f",
  "seq": 7,
  "sent_at_ms": 1792316000000,
  "payload": {
    "role": "sender",
    "capabilities": {
      "video": ["H264"],
      "audio": ["opus"],
      "max_width": 1920,
      "max_height": 1080,
      "max_fps": 60
    },
    "sdp": "<WebRTC SDP offer>"
  }
}
```

- `type` one of documented enum; `v` exact integer 1; unknown major version → `error.version_mismatch`.
- `message_id` UUID dedup key. `session_id` UUIDv4; required except in `device.hello`, `pair.*`.
- `seq` monotonic `u64` scoped to session+sender; stale/replay must be rejected.
- `sent_at_ms` for logging only; no latency comparisons between clocks without sync.
- JSON frame max **256 KiB**; SDP max **128 KiB**, candidates max **4 KiB**, annotation control max **32 KiB**; reject negative/out-of-range numeric fields and Unicode control characters in labels; peer name max 80 grapheme clusters.
- Max one pending unacknowledged critical operation per session; 5s request deadline for local handshake; 15s total ICE handshake configurable; reconnect within 10s with exponential backoff up to 3 attempts.
- Error payload: `{ "code": "permission_denied", "message": "Human-readable localized key", "retryable": false }`; no raw OS secrets.

## Message vocabulary

### `pair.begin` wire candidate

The sessionless common envelope carries a strict payload with exactly `receiver_id`, `nonce`, `peer_id`, `display_name`, `public_key`, `requested_grants`, and `signature`. IDs must be canonical lowercase UUIDs; the nonce is 16 bytes encoded as 32 lowercase hex digits; the proposed Ed25519 public key and signature are respectively 32 and 64 raw bytes encoded as 64 and 128 lowercase hex digits. The display name is limited to 80 Unicode scalar values with no controls, and requested grants are unique, non-empty values from `view`, `send`, and `annotate`. The signature covers the canonical proof bytes in ADR-0006, not JSON text. Sender code can create that signature through `PairingProof::create_signed` and the protected local identity port, then map the validated proof through read-only accessors to `mm-protocol::encode_pair_begin`, which uses the same validator as the inbound parser. The application helper does not validate QR policy or authenticate transport. See proposed [ADR-0020](adr/0020-pair-begin-wire-encoding-candidate.md), [ADR-0021](adr/0021-pair-begin-outbound-encoding-candidate.md), and [ADR-0022](adr/0022-sender-proof-creation-candidate.md); this schema is not frozen for cross-platform use.

| Type | Payload | Direction | Invariants |
|---|---|---|---|
| `device.hello` | `device_id, protocol_versions, capabilities` | both | no sensitive data; version check |
| `pair.begin` | `receiver_id, nonce, peer_id, display_name, public_key, requested_grants, signature` | sender→receiver | only an unexpired QR; strict candidate encoding and domain-separated proof are checked before an approval prompt; explicit approval is still required |
| `pair.approve` | `accepted_grants, peer_identity_proof` | receiver→sender | identity/fingerprint verified |
| `pair.reject` | `reason` | receiver→sender | no stream setup |
| `pair.revoke` | `peer_id` | either | closes active sessions scoped to peer |
| `session.offer` | `role, capabilities, sdp` | sender→receiver | after grant and capture authorization |
| `session.answer` | `sdp, selected_codec` | receiver→sender | H264 baseline preference |
| `session.ice` | `candidate, sdp_mid, mline_index` | both | bounded ICE candidates |
| `session.start` | `source_id` | sender→receiver | only after capture permit + DTLS connection |
| `session.stop` | `reason` | either | idempotent teardown; removes transient grants |
| `session.pause` | `pause=true` | either | freeze receiver or pause media per requested mode |
| `session.resume` | `{}` | either | must still have capture permission |
| `session.quality` | `max_width,max_height,target_fps,video_kbps` | both | clamp to negotiated caps |
| `session.stats` | `fps,loss_percent,rtt_ms,bitrate_kbps,queue_depth` | both | opt-in telemetry to peer |
| `session.error` | `code,message,retryable` | both | sanitized message |
| `annotation.*` | stroke messages below | both | only annotate grant and bounded rate |

**SDP** and ICE candidates are only transported inside authenticated signaling; media encryption remains WebRTC native. Use trickle ICE. TURN credentials if introduced later must be scoped, expire rapidly, never be in mDNS TXT.

## Annotation messages

Reliable, ordered data channel named `annotations-v1`; optional for local-only MVP (render annotations locally without network).

```json
{"v":1,"type":"annotation.stroke.start","session_id":"8b3d8788-1476-4aae-a287-334a428b5b6f","seq":8,"payload":{"stroke_id":"f3ad0fc8-e586-41f3-a73a-0b15aec8e212","color_rgba":"#17B2E6FF","width_px":4,"transform_rev":2}}
{"v":1,"type":"annotation.stroke.points","session_id":"8b3d8788-1476-4aae-a287-334a428b5b6f","seq":9,"payload":{"stroke_id":"f3ad0fc8-e586-41f3-a73a-0b15aec8e212","points":[{"u":0.25,"v":0.5,"dt_ms":0},{"u":0.3,"v":0.6,"dt_ms":8}]}}
{"v":1,"type":"annotation.stroke.end","session_id":"8b3d8788-1476-4aae-a287-334a428b5b6f","seq":10,"payload":{"stroke_id":"f3ad0fc8-e586-41f3-a73a-0b15aec8e212"}}
```

Bounds: `u,v ∈ [0,1]`, max 256 points per message and 240 per second per peer, `width_px` 1..64, max 1000 active strokes, max 64KiB local history default. `annotation.undo` carries `stroke_id`; `annotation.clear` purges a session overlay. If remote annotation is not granted, close annotation channel or ignore/reject inbound messages.

## Rust crate/module layout

```text
Cargo.toml                              # edition 2024, workspace + lockfile
rust-toolchain.toml                     # pin stable toolchain after CI compatibility
crates/
  mm-domain/src/{device,session,capabilities,annotation}.rs
  mm-application/src/{discover,pair,connect,disconnect,snapshot}.rs
  mm-ports/src/{capture,codec,transport,signaling,discovery,store,clock}.rs
  mm-protocol/src/{message,limits,errors,validation}.rs
  mm-security/src/{identity,qr,authorization,tls}.rs
  mm-store/src/{sqlite,migrations}.rs
  mm-runtime/src/{orchestrator,metrics,reconnect}.rs
  mm-adapter-win/src/{capture,codec,render}.rs
  mm-adapter-linux/src/{pipewire,codec,render}.rs
  mm-adapter-android/src/{jni,capture,codec,render}.rs
  mm-adapter-ios/src/{ffi,capture,codec,render}.rs
  mm-ffi/src/{api,events}.rs
apps/
  mirrormesh_flutter/lib/{app,features,widgets,bridge}/
  tv-web/{src,tests}/
  android-tv/{app,tests}/
tests/
  protocol-golden/, fixtures/, e2e/
```

The transport crate chosen in ADR-0002 can be `mm-transport-webrtc`. Favor `cargo features` / conditional compilation for platform adapters; never compile native OS bindings on unsupported host.

### Ports (Rust pseudocode; refine after proof spike)

```rust
pub trait DiscoveryPort: Send + Sync {
    fn start(&self) -> Result<(), DiscoveryError>;
    fn stop(&self);
    fn snapshots(&self) -> PeerSnapshotStream;
}
pub trait IdentityStorePort: Send + Sync {
    fn get_local_identity(&self) -> Result<LocalIdentityRef, StoreError>;
    fn get_peer(&self, id: DeviceId) -> Result<Option<TrustedPeer>, StoreError>;
    fn revoke_peer(&self, id: DeviceId) -> Result<(), StoreError>;
}
pub trait TransportPort: Send {
    fn make_offer(&mut self, session: SessionId, config: TransportConfig)
      -> Result<String, TransportError>;
    fn receive_signaling(&mut self, msg: SignalingMessage) -> Result<(), TransportError>;
    fn stop(&mut self);
}
pub trait RenderPort: Send {
    fn attach_surface(&mut self, target: SurfaceHandle) -> Result<(), RenderError>;
    fn snapshot(&self, annotations: Option<&AnnotationLayer>) -> Result<EncodedPng, RenderError>;
}
```

`PeerSnapshotStream`, `LocalIdentityRef`, `SurfaceHandle` are opaque types defined in `mm-domain`/`mm-ports` at implementation time; do not invent FFIs without platform spikes. The trait contracts additionally require cancellation and explicit lifecycle semantics. `mm-runtime` owns session actors and channels, backpressure and teardown.

## Session algorithm

```
select(peer):
  if peer not trusted: verify QR fingerprint -> pair UI -> save scoped grant
  check peer capability intersection and OS capture permission
  create_session(uuid) -> Negotiating
  generate offer(H264/Opus); send via verified WSS
  exchange answer and ICE; require DTLS connected
  request user-approved source; start native capture/encode
  send encoded AccessUnits/packets to transport (no UI raw frames)
  await first decoded present ACK -> Streaming
on_disconnect(reason):
  atomically set Stopping, cancel source, flush encoder, close SRTP/ICE,
  remove data channel subscriptions, release GPU handles, -> Stopped
on_network_loss():
  cap encoded queue, stop sending stale video,
  attempt ICE restart with backoff if device is still trusted/consent valid
  after budget exceeded -> Failed -> Stopped; display error
```

## SQLite migration v1 (run in a transaction)

```sql
PRAGMA foreign_keys=ON;
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL
);
CREATE TABLE local_identity (
    device_id TEXT PRIMARY KEY, public_key BLOB NOT NULL,
    key_ref TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE trusted_peers (
    device_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
    public_key BLOB NOT NULL, fingerprint TEXT NOT NULL UNIQUE,
    grants_json TEXT NOT NULL, paired_at_ms INTEGER NOT NULL,
    last_seen_at_ms INTEGER, revoked_at_ms INTEGER
);
CREATE TABLE session_history (
    id TEXT PRIMARY KEY, peer_id TEXT NOT NULL,
    local_role TEXT NOT NULL CHECK(local_role IN ('sender','receiver')),
    started_at TEXT NOT NULL, ended_at TEXT,
    result TEXT, stats_json TEXT,
    FOREIGN KEY(peer_id) REFERENCES trusted_peers(device_id)
);
CREATE INDEX idx_history_peer_started ON session_history(peer_id,started_at);
CREATE TABLE preferences (
    key TEXT PRIMARY KEY, value_json TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE annotation_exports (
    id TEXT PRIMARY KEY, session_id TEXT, path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    include_annotations INTEGER NOT NULL CHECK(include_annotations IN (0,1))
);
INSERT INTO schema_migrations(version,applied_at)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
```

**Note:** local_identity private key never stored in DB. Use OS keychain. `session_history` is diagnostic metadata only; add user setting for auto-purge after 30 days. SQLite schema on iOS/Android uses sandbox paths managed by the app.

## End-to-end acceptance examples

- `invalid fingerprint → no `pair.approve`, no network media`.
- `revoked grant midstream → no further encoded frame, socket shut down`.
- `unknown message type → sanitized error, no panic`.
- `disconnect repeated 50 times → idempotent, no leaked listeners`.
- `video 90° rotated / letterbox → annotation normalized mapping accurate`.
- `codec missing → explicit capability negotiation failure, not blank screen`.
