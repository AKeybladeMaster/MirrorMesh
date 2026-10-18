# ADR-0011 - Linux local pairing identity storage candidate

- **Decision date:** 2026-10-18
- **Status:** Proposed candidate; Linux runtime, security, and cross-track review pending

## Context

Task 03 needs a Linux implementation of `LocalPairingIdentityPort` that stores its Ed25519 seed outside SQLite, protocol messages, FFI events, and application-managed files. A generic fake-backed pairing runtime already composes this port with the coordinator; this ADR records the Linux storage candidate without approving the shared Ed25519 profile.

## Candidate

- Use the freedesktop Secret Service API through `secret-service = 5.1.0` and the blocking API. Store the 32-byte seed as the item secret in the user's default collection, under the application and purpose attributes `org.mirrormesh.MirrorMesh` and `pairing-identity-ed25519-v1`.
- Use `EncryptionType::Dh` for the Secret Service D-Bus session. This protects the secret transfer between the application and the service as defined by the API; it is not a claim of hardware-backed storage or protection against a compromised same-user process.
- Treat Secret Service errors, authorization cancellation, missing session bus, locked/unavailable collection, malformed seed, ambiguous matching items, and failure to acquire the per-user lock as errors. Do not silently generate an in-memory replacement.
- Serialize key lifecycle operations with `flock` on `mirrormesh-pairing-identity.lock` inside `XDG_RUNTIME_DIR`. Require the runtime directory and lock file to be owned by the effective user and inaccessible to group/other users; reject symlinks and unexpected lock-file types.
- Read the stored seed for each sign/public-key operation. Rotate by updating the existing Secret Service item before returning the new public key. Delete removes the item and disables future signing through the current operation path.
- Open or create the identity after a user starts pairing. The blocking API may wait for the desktop keyring's own unlock/authorization prompt and must not run on an async executor thread.

Secret Service lookup attributes are metadata and may be stored without encryption by the implementation. They contain only the application identifier and key purpose, never the seed. The key's at-rest protection is provided by the active Secret Service implementation and user collection configuration.

## Dependency and implementation evidence

- `secret-service 5.1.0` declares MIT OR Apache-2.0 and Rust 1.77. Its newer 5.2.0 release raises its Rust requirement to 1.87, so the candidate pins 5.1.0 for the workspace Rust 1.85 floor. The resolved Linux dependency graph uses `zbus 5.13.2`, `zbus_names 4.3.1`, `zvariant 5.9.2`, and `uuid 1.17.0`; it passes a Rust 1.85 Linux-target check.
- The crate implements the common local signer lifecycle and the Linux Secret Service backend in `crates/mm-adapter-linux/`. Eight fake-store tests cover persistence, signing, rotation, deletion, malformed values, store failures, canonical-message rejection, and a known Ed25519 vector.
- Rust 1.99.1 Linux-target check and strict Clippy pass. Rust 1.85 Linux-target check passes. These checks compile the backend but do not connect to D-Bus or execute Linux tests.

## Consequences and open gates

- Secret Service implementations may differ in unlock prompts, default collection policy, item locking, and persistence. A live GNOME Keyring/KWallet smoke test and cancellation/locked-collection checks are still required.
- The locked Rust 1.85-compatible D-Bus graph triggered informational RustSec unmaintained notices for `zbus_names 4.3.1`, `zvariant 5.9.2`, and `zvariant_derive 5.9.2` (RUSTSEC-2026-0371/0370/0372). Local audit found no vulnerable dependency, but this maintenance signal remains a review item; the selected graph does not add advisory suppressions.
- No Linux Flutter startup path currently assembles `LinuxPairingIdentity` with SQLite and `PairingRuntime`. The adapter has not accessed a user's keyring.
- Same-user processes may be able to request the secret through the session service. The key is exportable through the service API and is not hardware-backed. D-Bus session encryption is not a defense against a malicious local service.
- Cross-track/security review of the Ed25519 encoding and signing port remains pending. The broader mDNS, authenticated signaling, QR UI, revocation closure, and owned-LAN pairing gates remain open.

## References

- [Secret Service API Draft](https://specifications.freedesktop.org/secret-service/latest-single/)
- [`secret-service 5.1.0` package manifest (license and MSRV)](https://raw.githubusercontent.com/open-source-cooperative/secret-service-rs/v5.1.0/Cargo.toml)
- [`secret-service 5.1.0` API documentation](https://docs.rs/secret-service/5.1.0/secret_service/)
- [`secret-service 5.2.0` package manifest (Rust 1.87 MSRV)](https://raw.githubusercontent.com/open-source-cooperative/secret-service-rs/v5.2.0/Cargo.toml)
- [Identity provider spike](../spikes/identity-provider.md)
