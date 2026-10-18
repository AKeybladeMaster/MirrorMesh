# ADR-0020 — `pair.begin` wire encoding candidate

- **Decision date:** 2026-10-19
- **Status:** Proposed CORE protocol candidate; security and cross-track review pending

## Context

The strict `mm-protocol` parser previously accepted an ephemeral-key-shaped DTO that could not be passed to the existing `mm-application::PairingProof`. That left a gap between a validated JSON frame and the source-aware runtime pairing path. Protocol v1 is still a candidate, so the parser and runtime need one explicit, testable encoding without implying that other platform tracks have approved it.

## Decision

- Define the `pair.begin` payload with exactly these fields: `receiver_id`, `nonce`, `peer_id`, `display_name`, `public_key`, `requested_grants`, and `signature`. Unknown fields are rejected.
- Encode both device IDs as canonical lowercase UUID strings. Encode the 16-byte pairing nonce as 32 lowercase hexadecimal digits.
- Encode the candidate Ed25519 public key as exactly 32 raw bytes in 64 lowercase hexadecimal digits, and its signature as exactly 64 raw bytes in 128 lowercase hexadecimal digits. Algorithm/profile approval remains under ADR-0007.
- Limit `display_name` to 1–80 Unicode scalar values and reject control characters. Require one or more unique grants from `view`, `send`, and `annotate`.
- Keep the message inside the common bounded v1 envelope, without a session ID. The signature covers the existing canonical `PairingProof::signing_message` bytes from ADR-0006; it does not sign the JSON serialization.
- `PairingRuntime::verify_pair_begin_from` reserves the source-IP attempt before parsing. It parses the bounded frame, converts the validated payload to a `PairingProof`, verifies through the coordinator, and returns only a pending approval. Trust is persisted only after explicit approval.
- Proposed [ADR-0021](0021-pair-begin-outbound-encoding-candidate.md) records the typed outbound encoder and read-only application-proof accessors that use the same candidate schema.

## Consequences

- The parser golden fixture and runtime integration tests now cover the same field names and encodings that the application proof accepts.
- This closes only the Rust parser-to-runtime boundary. It does not add a listener, TLS, peer authentication, platform startup, QR UI, or automatic approval. A listener must still provide the actual remote socket address and a consistent monotonic clock.
- Malformed and wrong-signature frames consume the source attempt budget. The parser caps frames at 256 KiB; global connection and message limits remain the listener's responsibility.
- Cross-track and security review must approve this schema and the Ed25519 verification profile before protocol v1 is frozen.

## Verification

The protocol tests cover the golden payload, strict fields, canonical identifiers and lowercase hex lengths, grant vocabulary, malformed encodings, and outbound encoding round-trips. Runtime/SQLite integration tests use a real test-only Ed25519 signature to verify persistence remains behind explicit approval, tampered signatures do not persist, and malformed requests consume the source attempt budget. Commands and platform limitations are recorded in [verification](../verification.md).
