# ADR-0027: Android Keystore-wrapped pairing identity candidate

**Status:** Proposed; implementation candidate pending cross-track and security review
**Date:** 2026-10-19

## Context

MirrorMesh uses the same raw Ed25519 identity for pairing proofs and the TLS 1.3 server `CertificateVerify` signature. Android's documented `AndroidKeyStore` `KeyPairGenerator` algorithm list does not include Ed25519, while AES keys can be generated for authenticated GCM encryption. Platform hardware protection varies by device, so the implementation must not promise TEE or StrongBox backing.

## Decision

- Keep the shared Ed25519 key and `LocalPairingIdentityPort` / `LocalTlsIdentityPort` behavior in Rust. `AndroidPairingIdentity::open_or_create()` creates the key only when the caller explicitly starts pairing; app startup does not create an identity.
- Generate a 256-bit AES key in `AndroidKeyStore` for encrypt/decrypt use with GCM and no padding. The key material is not exported. The device-selected security level is not asserted or attested.
- Store only a versioned record in the app's `noBackupFilesDir`: magic `MMID\x01`, a 12-byte IV, and the AES-GCM ciphertext plus 128-bit tag for the 32-byte Ed25519 seed. Bind a fixed versioned AAD string. Reject wrong sizes, magic, decryption, or authentication before using the seed.
- Loading a stored record must find the existing wrapping-key alias; only first-time encryption may create it. Do not silently replace a missing wrapping key while trying to decrypt an existing identity.
- Open the record without following symlinks. Write a new owner-only temporary file, sync it, then rename it into place. Delete both the record and the Android Keystore wrapping-key alias; deletion can be retried if key cleanup fails after the file is removed.
- Zeroize Rust seed buffers and JNI plaintext byte arrays after use. The Ed25519 seed is still decrypted into the application process for Rust signing, so a compromised process may use or extract it.
- Initialize `ndk-context` and assemble this adapter into the Android Flutter Rust bridge in a separate CORE-owned task. That shared FFI change is not part of this Android adapter patch.

## Consequences

- The platform candidate reuses the project's Ed25519 key encoding and TLS input validation. Host fake-store tests cover record round trips, tamper rejection, failed rotation, key-cleanup retry, identity deletion/recreation, and signing behavior.
- `jni 0.21.1` and `ndk-context 0.1.1` are pinned as Android-target dependencies; their license expressions fit the current MIT/Apache-2.0 allowlist. Dependency metadata is in `Cargo.lock`.
- No AndroidKeyStore operation, Android app startup, APK build, device operation, or hardware-backed-key attestation has run. Pairing-runtime assembly, capture permission, MediaProjection, MediaCodec, UI, LAN behavior, and physical-device tests remain open.
- If the wrapping key is hardware-backed on a device, that protects the AES key material. It does not prevent an attacker who controls the running app process from invoking decryption or using the decrypted Ed25519 identity.

## References

- [Android Keystore system](https://developer.android.com/privacy-and-security/keystore)
- [Android KeyGenParameterSpec.Builder](https://developer.android.com/reference/android/security/keystore/KeyGenParameterSpec.Builder)
- [Android KeyPairGenerator API](https://developer.android.com/reference/java/security/KeyPairGenerator)
- [Flutter Rust Bridge Android NDK initialization](https://cjycode.com/flutter_rust_bridge/guides/how-to/ndk-init)
- [ndk-context 0.1.1](https://docs.rs/ndk-context/0.1.1/ndk_context/)
- [jni 0.21.1](https://docs.rs/jni/0.21.1/jni/)
