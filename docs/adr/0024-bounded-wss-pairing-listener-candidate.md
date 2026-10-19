# ADR-0024: bounded native WSS pairing listener candidate

- **Status:** Proposed; requires security and cross-track review
- **Date:** 2026-10-19
- **Scope:** CORE signaling adapter only

## Context

The existing pairing runtime verifies a bounded `pair.begin` envelope and returns a pending approval. ADR-0023 provides a native TLS 1.3 raw-public-key (RPK) identity candidate pinned to the fingerprint carried by the receiver's QR invitation. The application still needs a network boundary that combines those contracts without persisting trust before the owner approves it.

## Decision

Add `mm-adapter-signaling::WssPairingListener` as a synchronous native listener candidate. The caller supplies the bound `SocketAddr` and the TLS server configuration produced by `mm-adapter-tls`; the listener is run on a dedicated worker. It accepts only `GET /v1/control`, rejects queries and `Origin` headers, and accepts the first application message only as a text `pair.begin` envelope. It captures the TCP peer IP before TLS and WebSocket processing and passes that address with the complete text payload to `PairingRuntime::verify_pair_begin_from`.

The runtime's existing attempt limiter reserves the source budget before JSON envelope parsing and signature verification. Because the current runtime API accepts the completed payload, that per-source admission happens after TLS and WebSocket framing. The listener bounds the pre-admission exposure globally: at most eight concurrent accept/handshake/pending slots, an absolute ten-second deadline from accept through the first complete message, and WebSocket frame/message limits equal to the protocol's 256 KiB envelope cap. Ping messages are answered during the initial read; binary, close, wrong-path, wrong-method, query, and browser-origin requests are rejected.

On successful verification, the listener returns the admitted pending approval and the live WSS connection to its caller. The listener does not approve the request, persist trust, start a media session, or define an acknowledgment/status message. The application must show the requested identity and grants, then consume the pending value through `PendingPairingConnection::approve`. That shared caller path invokes `PairingRuntime::approve` and returns the live connection only after trust persistence succeeds. If the approval is denied by expiry or another runtime error, consuming the pending value drops the connection and releases its capacity slot. The browser fallback remains a separate WSS endpoint using ordinary platform-trusted certificates; this native listener intentionally rejects browser `Origin` requests.

## Security and lifecycle notes

- Only pass the TLS 1.3 RPK server configuration from ADR-0023. The public listener constructor accepts Rustls `ServerConfig`, so the caller is responsible for supplying that restricted configuration and a platform-held signing key.
- The peer IP is captured before handshake parsing, but the source attempt limiter is not applied until a complete WebSocket text message is assembled. The connection cap and absolute deadline bound this window; they do not provide per-IP rate limiting before TLS/WebSocket framing.
- Socket read/write timeouts are reset to the remaining time before each I/O operation. This makes the limit one absolute deadline rather than a fresh timeout for every partial read, as peers can otherwise keep incomplete handshakes open by slowly sending bytes. The pinned Rust `TcpStream` docs describe read/write timeouts as applying to blocking calls: [Rust `TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html).
- Pending approval has no new wire response contract. The approval object's original monotonic expiry is enforced by the shared `PendingPairingConnection::approve` caller path before trust is persisted; a stale value is consumed, its connection is dropped, and its capacity slot is released. The listener caps retained connections at eight but does not impose an owner-prompt deadline.
- The listener accepts one request per connection and handles no session offer, ICE, media, trust revocation, or reconnect messages.

## Verification

`crates/mm-adapter-signaling/tests/wss_pair_begin_two_process.rs` launches separate local client and server processes. The sender pins the server identity to the QR fingerprint and sends a real signed `pair.begin`; one scenario confirms that valid explicit approval persists only the view grant, while another advances to the approval deadline and confirms `ApprovalExpired`, no stored trust, and a closed client connection. Unit tests cover route rejection, message/frame limits, capacity release, absolute deadline arithmetic, and slow partial reads. This is a local loopback test only; it does not establish LAN, platform, browser, UI, or physical-device behavior.

The adapter pins `tungstenite 0.30.0` with its handshake feature. Its documented MSRV is Rust 1.85 and its license expression is MIT OR Apache-2.0; the locked transitive package license metadata was reviewed against the repository allowlist. See the versioned [Tungstenite handshake API](https://docs.rs/tungstenite/0.30.0/tungstenite/fn.accept_with_config.html), [`WebSocketConfig`](https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/struct.WebSocketConfig.html), and [crate manifest](https://docs.rs/crate/tungstenite/0.30.0/source/Cargo.toml).

## Consequences

This candidate joins the TLS identity and pairing runtime at a testable native socket boundary. It remains unassembled into Windows/Linux/Android/iOS or Android TV startup and has no approval UI. Task 20 adds a fake-store Windows `LocalTlsIdentityPort` implementation; it has not accessed a real Credential Manager entry. Protocol v1, the RPK profile, attempt policy, and this public adapter boundary remain candidates pending their required reviews.
