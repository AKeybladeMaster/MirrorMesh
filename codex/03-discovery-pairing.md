# TASK 03 — mDNS discovery, QR trust and authenticated signaling

**Skills:** TDD, security-review (after audit), rust-best-practices, requesting-code-review

**Scope:** Shared CORE pairing runs in parallel with platform adapters; Android TV native `Accept/Reject` is the primary TV onboarding UI, web QR fallback is separate and secure-origin dependent. Cross-agent protocol changes need CORE review.

## Current progress (2026-10-18)

The shared CORE contract now has a bounded manual endpoint parser, strict QR parser, OS-random one-time nonce registry, validated trusted-peer record and storage/identity ports, plus a fake-backed `mm-application::PairingCoordinator`. It creates invitations, validates the domain-separated proof, enforces one-time nonce use and explicit grant-subset approval, and persists only after approval. See [ADR-0004](../docs/adr/0004-trusted-peer-records.md), [ADR-0005](../docs/adr/0005-sqlite-trusted-peer-store.md), [ADR-0006](../docs/adr/0006-pairing-coordinator-proof-contract.md), and proposed [ADR-0007](../docs/adr/0007-ed25519-identity-candidate.md). The candidate encoding is Ed25519 with 32-byte raw public keys, 64-byte signatures, and SHA-256 fingerprints over the raw public-key bytes. The previously named `ed25519-compact 2.6.0` implementation version is unverified: accessible docs.rs views differed and the exact page/package index was inaccessible; no crypto dependency or provider was added. Remaining Task 03 work includes confirming a crate version, auditing and implementing a production provider and OS key storage, defining local signing/key-lifecycle APIs, durable SQLite storage, mDNS, authenticated TLS/WSS signaling and rate limiting, platform QR/approval UIs, revocation-driven transport closure, and owned-LAN/device verification. Protocol v1 remains a candidate pending cross-track and security review.

## Deliverables and detailed steps

1. Implement `DiscoveryPort` adapter using vetted mDNS/DNS-SD Rust crate, test `_mirrormesh._tcp.local` TXT in real LAN; no BLE dependency in MVP.
2. Add strict manual URI/IP:port parser; reject URL schemes, DNS rebinding/redirection, private-key contents or arbitrary ports not explicitly user-approved.
3. Generate OS keychain private identity and SQLite public peer table from `docs/protocol.md`; signed challenge proves possession and matches QR fingerprint. Security review must evaluate self-signed TLS pinning and replay handling.
4. Implement one-time random 128-bit pairing nonce + two-minute expiry; Windows/Linux/Android/iOS receiver UIs **and Android TV remote UI** show QR/device name and explicitly approve requested roles. Web fallback uses the same security contract, not a simplified token. **No plain numeric PIN flow** until PAKE code, threat review and tests are approved.
5. WSS control server `8765` accepts only authenticated sessions or minimal rate-limited pairing exchange; bound JSON frames to 256KiB and hard-fail unknown versions.
6. Implement session offer/answer/ICE with immutable session ownership and capability validation. Reject unsolicited offer before grants/capture consent.
7. Add `trusted_peers` SQLite migrations and revocation logic; revocation closes active stream and prevents reconnect.
8. Offer "manual IP" only with out-of-band public fingerprint comparison and physical receiver approval; do not trust unverified self-signed server automatically.
9. Test discovery with blocked multicast, Wi-Fi disconnect, changed IP, same-name devices, revoked peer, expired/reused QR, malicious oversized frames, 20 wrong pairing attempts and rate limiting.

## Acceptance

Two devices pair over owned LAN; unpaired device cannot view stream; QR replay/expiry and MITM rejected; disconnection and revoked access reliably close transport; docs include exact certificate/key handling and screenshot of pairing UX.
