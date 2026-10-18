# TASK 03 — mDNS discovery, QR trust and authenticated signaling

**Skills:** TDD, security-review (after audit), rust-best-practices, requesting-code-review

**Scope:** Shared CORE pairing runs in parallel with platform adapters; Android TV native `Accept/Reject` is the primary TV onboarding UI, web QR fallback is separate and secure-origin dependent. Cross-agent protocol changes need CORE review.

## Current progress (2026-10-18)

The shared CORE contract now has a bounded manual endpoint parser, strict QR parser, OS-random one-time nonce registry, validated trusted-peer record and storage/identity ports, plus a fake-backed `mm-application::PairingCoordinator`. It creates invitations, validates the domain-separated proof, enforces one-time nonce use and explicit grant-subset approval, and persists only after approval. See [ADR-0004](../docs/adr/0004-trusted-peer-records.md), [ADR-0005](../docs/adr/0005-sqlite-trusted-peer-store.md), [ADR-0006](../docs/adr/0006-pairing-coordinator-proof-contract.md), proposed [ADR-0007](../docs/adr/0007-ed25519-identity-candidate.md), proposed [ADR-0010](../docs/adr/0010-pairing-runtime-composition.md), and proposed [ADR-0011](../docs/adr/0011-linux-identity-key-storage.md). The candidate encoding is Ed25519 with 32-byte raw public keys, 64-byte signatures, and SHA-256 fingerprints over the raw public-key bytes. A stateless `mm-adapter-identity` verifier candidate pins `ed25519-dalek 3.0.0`, `curve25519-dalek 5.0.0`, and `sha2 0.11.0`; six RFC 8032 and negative-input tests pass. Canonical-key and low-order checks plus `verify_strict` still need cross-track/security approval. The versioned SQLite trusted-peer adapter in `mm-store` passes ten tests. A generic `mm-runtime::PairingRuntime` composes the coordinator with local identity and trusted-peer ports; nine fake-backed unit tests pass, and two integration tests exercise the real Ed25519 verifier and SQLite store through the consent-gated flow. Concrete OS adapter assembly into platform app startup remains open. The Windows Credential Manager identity candidate passes ten fake-store tests, including canonical proof-message rejection, rotation, deletion, and storage-failure cases. It compiles the Win32 calls but has not accessed a real credential. The Linux Secret Service identity candidate passes eight fake-store tests and compiles for the GNU Linux target on Rust 1.99.1 and 1.85; it has not accessed D-Bus or a user's keyring. Remaining Task 03 work includes cross-track/security approval of the verifier profile and shared signing port, concrete platform app assembly, live keyring validation plus Android/iOS protected key storage, mDNS app integration, authenticated TLS/WSS signaling and rate limiting, platform QR/approval UIs, revocation-driven transport closure, and owned-LAN/device verification. Protocol v1 remains a candidate pending cross-track and security review.

## Deliverables and detailed steps

### Shared mDNS discovery candidate (2026-10-18)

`crates/mm-adapter-discovery/` now contains a `DiscoveryPort` candidate backed by pinned `mdns-sd 0.21.5`. It browses `_mirrormesh._tcp.local.` for at most two seconds, parses the proposed strict TXT v1 fields, filters unusable endpoints, and returns device hints and resolved addresses. Eleven parser tests pass on Rust 1.99.1 and Rust 1.85 without opening sockets. This is not yet assembled into app startup and did not exercise multicast or an owned LAN. Proposed [ADR-0012](../docs/adr/0012-mdns-discovery-candidate.md) records the dependency and schema; its cross-track/security review remains pending. The upstream compatibility statement covers Windows, Linux, and macOS; Android and iOS still need native discovery and permission adapters.

### Shared mDNS service publisher candidate (2026-10-18)

`MdnsServicePublisher` builds the same strict v1 service record from a caller-supplied `Device`, `.local.` hostname, and explicit LAN addresses. It accepts only private/link-local IPv4 or unique-local/link-local IPv6, caps addresses at 16, fixes signaling to port 8765, and supports explicit stop with a bounded daemon shutdown wait. Proposed [ADR-0013](../docs/adr/0013-mdns-service-publication-candidate.md) records the API and limits. Eight new tests cover record construction, metadata and address bounds, invalid input before daemon creation, and idempotent inactive stop without opening sockets. The app does not yet have stable local identity/runtime assembly, so the publisher is not started from its UI or at app startup.

1. Implement `DiscoveryPort` adapter using vetted mDNS/DNS-SD Rust crate, test `_mirrormesh._tcp.local` TXT in real LAN; no BLE dependency in MVP.
2. Add strict manual URI/IP:port parser; reject URL schemes, DNS rebinding/redirection, private-key contents or arbitrary ports not explicitly user-approved.
3. Generate OS keychain private identity and SQLite public peer table from `docs/protocol.md`; signed challenge proves possession and matches QR fingerprint. Security review must evaluate self-signed TLS pinning and replay handling.
4. Implement one-time random 128-bit pairing nonce + two-minute expiry; Windows/Linux/Android/iOS receiver UIs **and Android TV remote UI** show QR/device name and explicitly approve requested roles. Web fallback uses the same security contract, not a simplified token. **No plain numeric PIN flow** until PAKE code, threat review and tests are approved.
5. WSS control server `8765` accepts only authenticated sessions or minimal rate-limited pairing exchange; bound JSON frames to 256KiB and hard-fail unknown versions.
6. Implement session offer/answer/ICE with immutable session ownership and capability validation. Reject unsolicited offer before grants/capture consent.
7. The versioned SQLite migration and trusted-peer revocation store are implemented in `mm-store`; wire revocation to active transport closure and prevent reconnect.
8. Offer "manual IP" only with out-of-band public fingerprint comparison and physical receiver approval; do not trust unverified self-signed server automatically.
9. Test discovery with blocked multicast, Wi-Fi disconnect, changed IP, same-name devices, revoked peer, expired/reused QR, malicious oversized frames, 20 wrong pairing attempts and rate limiting.

## Acceptance

Two devices pair over owned LAN; unpaired device cannot view stream; QR replay/expiry and MITM rejected; disconnection and revoked access reliably close transport; docs include exact certificate/key handling and screenshot of pairing UX.
