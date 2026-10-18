# TASK 03 — mDNS discovery, QR trust and authenticated signaling

**Skills:** TDD, security-review (after audit), rust-best-practices, requesting-code-review

**Scope:** Shared CORE pairing runs in parallel with platform adapters; Android TV native `Accept/Reject` is the primary TV onboarding UI, web QR fallback is separate and secure-origin dependent. Cross-agent protocol changes need CORE review.

## Current progress (2026-10-18)

The shared CORE contract now has a bounded manual endpoint parser, strict QR parser, OS-random one-time nonce registry, validated trusted-peer record and storage/identity ports, plus a fake-backed `mm-application::PairingCoordinator`. It creates invitations, validates the domain-separated proof, enforces one-time nonce use and explicit grant-subset approval, and persists only after approval. See [ADR-0004](../docs/adr/0004-trusted-peer-records.md), [ADR-0005](../docs/adr/0005-sqlite-trusted-peer-store.md), [ADR-0006](../docs/adr/0006-pairing-coordinator-proof-contract.md), and proposed [ADR-0007](../docs/adr/0007-ed25519-identity-candidate.md). The candidate encoding is Ed25519 with 32-byte raw public keys, 64-byte signatures, and SHA-256 fingerprints over the raw public-key bytes. A stateless `mm-adapter-identity` verifier candidate pins `ed25519-dalek 3.0.0`, `curve25519-dalek 5.0.0`, and `sha2 0.11.0`; six RFC 8032 and negative-input tests pass. Canonical-key and low-order checks plus `verify_strict` still need cross-track/security approval. The versioned SQLite trusted-peer adapter in `mm-store` passes ten tests; its application runtime wiring and non-Windows builds remain open. The Windows Credential Manager identity candidate passes ten fake-store tests, including canonical proof-message rejection, rotation, deletion, and storage-failure cases. It compiles the Win32 calls but has not accessed a real credential; runtime wiring and security/cross-track review remain open. Remaining Task 03 work includes cross-track/security approval of the verifier profile and shared signing port, runtime wiring, Linux/Android/iOS protected key storage, mDNS, authenticated TLS/WSS signaling and rate limiting, platform QR/approval UIs, revocation-driven transport closure, and owned-LAN/device verification. Protocol v1 remains a candidate pending cross-track and security review.

## Deliverables and detailed steps

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
