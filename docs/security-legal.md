# Security, threat model, privacy and lawful interoperability

## Trust boundaries and assets

Assets: captured screen (often highly sensitive), audio, pairing credentials, identity keys, trusted-peer grants, local exported images, sessions and telemetry.

Threat actors: another device on LAN, malicious QR/IP, compromised TV/browser, relay, malicious skill, stolen pairing code, actor with app-store binaries, third-party dependency.

## Rust dependency policy

The separate fuzz workspace lockfile is included in the same advisory, license, and source policy checks. Its libFuzzer dependency is isolated from shipped application packages.

libfuzzer-sys 0.4.13 declares the NCSA license in addition to its MIT/Apache option. deny.toml permits NCSA only for that exact crate version. The Open Source Initiative's license grants use, modification, redistribution, sublicensing, and sale, subject to retaining notices and disclaimers and avoiding endorsement claims; see the [official NCSA license text](https://opensource.org/license/ncsa). This exception applies only to fuzz tooling and does not add NCSA to the global allow-list.

A pinned cargo-about workflow generates a reviewable Rust-only license-text preview from the root Cargo graph. It deliberately excludes development-only dependencies and includes build and transitive dependencies. It is not a complete package notice: Flutter plugins, Android/Apple SDK components, codecs, and other platform libraries require separate inventory and review.

The current generated Rust notice is checked in at docs/THIRD-PARTY-NOTICES-rust.txt, and CI compares future regeneration against it. Local generation with cargo-about 0.9.2 was repeated after adding the Linux Secret Service dependency graph; see [verification](verification.md). The 272,600-byte snapshot has SHA-256 `10BF5672DDDDD50CDAAA61E797F973A1DA6ED39993D7D95E2043D47983DF5131`. cargo-about warned that allo-isolate 0.1.27 has no license field; the generated Apache-2.0 text was present and is separately confirmed by its packaged license file. The hosted workflow result has not been reviewed.

A pinned Flutter workflow collects root-level LICENSE/COPYING/NOTICE text for the locked runtime package graph into docs/THIRD-PARTY-NOTICES-flutter.txt and checks for drift. The current local snapshot covers 45 Flutter runtime packages, including Flutter SDK packages and the pinned WebRTC plugin graph. It is a package-source inventory only; it does not establish legal completeness for the final APK, IPA, desktop bundle, native SDKs, or codec libraries. Review the hosted workflow result separately.

CI is configured to check the locked Rust dependency graph with `cargo audit` and `cargo deny`. The reviewed license allow-list is Apache-2.0, MIT, Unicode-3.0, 0BSD, Unlicense, Zlib, and BSD-3-Clause. BSD-3-Clause was added for the exact `curve25519-dalek 5.0.0` and `ed25519-dalek 3.0.0` verifier candidate packages after reviewing their package metadata and license text. Linux adds `secret-service 5.1.0` (MIT OR Apache-2.0) and the existing allow-list covers both declared license choices. `allo-isolate@0.1.27` has a license file identifying Apache-2.0 despite not declaring a machine-readable `license` value. CORE now uses exact `getrandom` 0.4.3 for pairing nonce bytes; its `MIT OR Apache-2.0` license is already covered by the allow-list, and its [official docs](https://docs.rs/getrandom/0.4.3/getrandom/) describe the operating-system random-source API. The updated lockfile passed local cargo-deny advisory, license, ban, and source checks without exceptions. `cargo audit --no-fetch` loaded the cached advisory database and found no vulnerable dependencies, while reporting informational unmaintained notices for `zbus_names 4.3.1`, `zvariant 5.9.2`, and `zvariant_derive 5.9.2` (RUSTSEC-2026-0371/0370/0372), all introduced by the Secret Service dependency graph. `deny.toml` contains no advisory exceptions, rejects yanked versions and unknown registries or Git sources, and checks development dependencies. Review any newly reported license against the exact crate and version before changing the allow-list. The scheduled workflow is intended to detect newly published advisories; it has not run yet, and these checks do not prove a dependency is safe from malicious behavior.

The source SBOM workflow uses Syft to create a CycloneDX JSON inventory from the repository manifests and lockfiles and retains it for seven days as a review artifact. It prints the file digest and has no release-upload permission. A release SBOM must be generated from each exact built package and published with that package; the source inventory is not a substitute.

The Android TV debug workflow pins Syft 1.50.0 to inspect the exact APK file and emits a CycloneDX preview alongside the APK digest and an observed runner/SDK manifest. The debug APK, SBOM, and build manifest are retained as a seven-day CI artifact only. The hosted workflow result has not been reviewed, and its debug evidence is not a release package or a hardware-support result.

| Threat | Mitigation | Verification |
|---|---|---|
| Unauthorized stream startup | OS consent on sender and explicit paired receiver acceptance | test no stream before permission |
| LAN spoofing, MITM | cryptographic peer identity + pinned fingerprint from QR; DTLS/SRTP media; TLS signaling | fake cert/identity blocked |
| Replayed QR code | short expiry (e.g. 120s), one-time nonce and replay cache | reusing code rejected |
| Brute force numeric PIN | only authenticated PAKE + throttling + lockout | wrong-code budget/rate test |
| Cross-session abuse | per-session access tokens/scopes, nonce, session-id binding | message from wrong session rejected |
| Malicious oversized packet | strict schema validation, frame/annotation max sizes, bounded queues | fuzz JSON parser and annotation points |
| Public-exposed local socket | bind LAN only by user intent, firewall prompt, no UPnP and no unauthenticated endpoints | external network negative test |
| Device revoked mid-stream | tear down tracks, keys/tokens, peer state | immediate stop test |
| Capture private notifications | user-select window where possible, obvious sharing indicator; privacy guidance | OS permission flow review |
| Sensitive data in logs | redact IP/private names/tokens/video; debug logs opt-in | grep/log snapshot check |
| Compromised skill | vendor/review pinned SKILL.md, least privileges/no keys | installation checklist |
| Media codec exploitable parser | patched libraries, fuzzing, sandbox OS, no own H.264 bitstream parser | dependency & fuzz gate |
| Artifact compromise | signed release, checksums, SBOM, provenance | independent archive hash |

## Pairing design (MVP)

QR encodes `mirrormesh://pair?v=1&host=<numeric-IP>&port=8765&device=<uuid>&fp=<64-lowercase-hex-SHA256-public-key-fingerprint>&nonce=<32-lowercase-hex-digits>&expires=<unix-seconds>`; ensure QR is displayed in owner-controlled receiver UI. Never transmit credentials to an unknown URL from arbitrary QR text. `parse_pairing_qr` enforces this narrow shape, approved numeric endpoints, canonical fields, and a future expiry no more than 120 seconds away. CORE has a bounded helper that fills 128-bit nonces from the OS random source and permits one use before a 120-second local monotonic deadline. The QR Unix expiry is a separate wall-clock check. ADR-0004 defines a 32-byte SHA-256 fingerprint over the canonical presented public-key encoding. The proposed [ADR-0007](adr/0007-ed25519-identity-candidate.md) specifies raw Ed25519 public-key bytes (32 bytes), signatures (64 bytes), and hashing those raw key bytes for the fingerprint. A stateless verifier candidate is implemented in `mm-adapter-identity` and passes RFC 8032, malformed-key, and invalid-signature tests; its `verify_strict` acceptance profile remains subject to cross-track/security review. The domain record validates bounded public-key bytes, scoped grants, and timestamps, and enforces one-way revocation; `TrustedPeerPort` defines the persistence boundary. `mm-application::PairingCoordinator` composes the invitation parser, nonce registry, `PairingIdentityPort`, and trusted-peer storage contract. It verifies the domain-separated sender proof, then creates a pending approval; only explicit approval of a requested-grant subset persists trust. The coordinator unit tests use fakes; two `mm-runtime` integration tests now exercise the flow with the real Ed25519 verifier and SQLite trusted-peer adapter, confirming that trust is saved only after explicit approval and modified signatures do not persist. `mm-store::SqliteTrustedPeers` stores public peer data only. Proposed [ADR-0009](adr/0009-windows-identity-key-storage.md) describes a Windows Credential Manager candidate; proposed [ADR-0011](adr/0011-linux-identity-key-storage.md) describes a Linux Secret Service candidate. Both keep the private seed out of SQLite and fail closed when storage is unavailable, but neither has accessed the real OS key store or been assembled into platform startup. Secret Service item attributes contain application/purpose metadata, not key material; same-user callers may still request the exportable seed from the active service. Android Keystore and iOS Keychain candidates, QR UI, authenticated listener/TLS pinning, and rate limiting remain open. The signing proof contract is recorded in proposed [ADR-0006](adr/0006-pairing-coordinator-proof-contract.md). For first-time manual PIN without QR, implement audited PAKE; until then fallback requires visual fingerprint comparison and a user-approved local connection (no silent TOFU).

The current manual-address parser accepts numeric IPv4 and bracketed IPv6 only, and only when the caller supplies the port in its explicitly user-approved list. It never resolves a hostname. Do not add hostname-based manual connections until the network adapter pins a single resolution result for the connection, rejects redirects, and verifies the expected peer fingerprint; the parser alone does not authenticate or connect to a peer.

**Cloud TV pairing (optional):** a narrowly scoped HTTPS broker maps short-lived room codes to signaling messages, authenticates both ends, and never handles video plaintext. The TV browser uses HTTPS origin; ICE tries direct local UDP. TURN is optional and relay bandwidth incurs costs. Explain cloud dependence in UI and privacy policy.

## External data / user controls

Default: offline/local, no analytics, no auto-upload and no crash dumps unless opted in. Private identity candidates use Windows Credential Manager and Linux Secret Service; Android Keystore and iOS Keychain remain planned. Linux currently fails closed if Secret Service, its session bus, authorization, or a safe runtime lock location is unavailable; no passphrase fallback is implemented. No keys in `trusted_peers` aside from public keys.

Exports user initiated, locally saved, and deletable. UI must show **active capture indicator**, session recipient, permission state and immediate Stop/Disconnect. Recording/casting remote screens requires permission from content owners; protected/DRM windows may be blanked by OS—do not circumvent.

## REA and open-source legal boundary

- Verify law applicable to binary examination/contract, purpose, interoperability exceptions and redistribution. No universal assurance that decompiling Store packages is permitted.
- No bypass of signing, encryption, DRM, paywalls, license checks or access controls.
- No copying binaries, decompiled source, proprietary assets or trademarks into product code/README.
- Keep independent team/code and evidence-only summaries; use public protocol specifications and official APIs.
- Check libraries for patent, redistribution, GPL/LGPL/Apache obligations including dynamically linked GStreamer/codec plugins.
- Treat AirPlay/Google Cast/Miracast brand/protocol claims separately from generic WebRTC. Do not claim official compatibility without validation.
- Choose repository license only after SBOM license review.

## Go-live security gates

Threat model approved; pair/no-pair/revoke integration tests pass; no unauthenticated stream; packet bounds; fuzz parser and malicious data channel input; `cargo audit` and SBOM outputs archived; privacy policy/app-store consent strings reviewed; all release artifacts signed when tooling allows.
