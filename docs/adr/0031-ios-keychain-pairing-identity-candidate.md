# ADR-0031: iOS Keychain pairing identity and explicit runtime assembly

**Status:** Proposed implementation; Apple-platform build, runtime, and cross-track/security review pending

## Context

Task 25 assembles Android pairing identity and trusted-peer storage only after an explicit user action. iOS needs the same lifetime and consent boundary while keeping the private Ed25519 seed out of SQLite and Flutter messages. The iOS runner already builds the Rust `mm-ffi` static library for device targets, but shared FFI crate changes are owned by CORE.

## Decision

- Add the iOS-owned `mm-adapter-ios` crate. It implements `LocalPairingIdentityPort`, `LocalTlsIdentityPort`, and `PairingIdentityPort` using the same canonical proof and TLS CertificateVerify input checks as the Android candidate.
- Store the 32-byte Ed25519 seed as a non-synchronizable generic-password Keychain item. Its service and account identify the MirrorMesh pairing identity. New items use `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`; duplicate writes update the existing item for identity rotation, and deleting a missing item succeeds.
- Keep the Keychain calls in the Swift iOS runner, reached through narrow C ABI callbacks. The Rust adapter copies a valid seed into zeroizing Rust buffers for signing and fails closed for missing, malformed, or unavailable Keychain data. Keychain operations run on a background queue.
- Flutter exposes `Prepare pairing support` on the iOS Devices page. The native method creates the app's Application Support directory, excludes it from backup, then asks Rust to open the Keychain identity and SQLite trusted-peer store. Rust retains one `PairingRuntime` for the process. The action does not start discovery, a listener, trust approval, capture, or streaming. The simulator build keeps the Rust device bridge out of the link and reports local pairing support unavailable.
- Derive the public local device ID from the first 16 fingerprint bytes with UUID version 8 and RFC variant bits. The ID is public implementation-specific metadata; it changes when the identity rotates.

## Security and privacy limits

Keychain encrypts the stored item and `ThisDeviceOnly` prevents migration to a different device, but the seed is exportable into the app process for Rust Ed25519 signing. This candidate does not use Secure Enclave keys and does not claim hardware backing or resistance to a compromised process. Foundation/Security may retain temporary copies returned by Keychain; Rust-owned copies are zeroized, while Swift cannot guarantee immediate wiping of every framework-managed copy. No secret is written to SQLite or returned through Flutter.

The app must call this initializer only after explicit user action and off the UI thread. A Keychain or SQLite failure returns a generic unavailable result and does not create an ephemeral identity. If identity rotation or deletion is exposed later, the process pairing runtime must be dropped and recreated so its captured fingerprint stays aligned with the active identity.

## Verification and remaining work

Host fake-store tests cover identity persistence, malformed seed rejection, storage errors, canonical pairing/TLS signing, rotation, and deletion. They do not call Security.framework or verify item accessibility. The iOS target check and Xcode archive/link build require Apple SDK tooling; actual Keychain behavior and device startup require an iPhone/iPad. Pairing UI, discovery, listener startup, protocol review, media capture, and device verification remain separate work.

## References

- [Apple: Generic password item class](https://developer.apple.com/documentation/security/ksecclassgenericpassword)
- [Apple: `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`](https://developer.apple.com/documentation/security/ksecattraccessiblewhenunlockedthisdeviceonly)
- [Apple: Keychain item classes and values](https://developer.apple.com/documentation/security/item-class-keys-and-values)
- [Apple: `SecItemAdd`](https://developer.apple.com/documentation/security/secitemadd%28_%3A_%3A%29)
- [ADR-0007: Ed25519 identity candidate](0007-ed25519-identity-candidate.md)
- [ADR-0010: Pairing runtime composition](0010-pairing-runtime-composition.md)
