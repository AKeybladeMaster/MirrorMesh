# ADR-0032: Windows pairing runtime assembly candidate

- **Status:** Proposed; requires Windows, security, and cross-track review
- **Date:** 2026-10-19
- **Scope:** Windows pairing bootstrap only

## Context

`mm-adapter-win` provides a fake-tested Windows Credential Manager identity, and CORE provides `PairingRuntime` plus the SQLite trusted-peer store. Windows Flutter had no application entry point that assembled those pieces. The Devices page must keep identity creation behind an explicit local user action; application startup must not open credentials, create the trusted-peer database, discover devices, or start network services.

## Proposed decision

- Add `mm-adapter-win::initialize_pairing_runtime`, retaining one `PairingRuntime<SqliteTrustedPeers, WindowsPairingIdentity>` in a process-wide `OnceLock` after the caller asks for pairing support.
- Store the public trusted-peer database at `%LOCALAPPDATA%\MirrorMesh\trusted-peers-v1.sqlite`. Create the directory only during explicit initialization. The existing Windows identity adapter continues to store the Ed25519 seed in Windows Credential Manager; the seed is exportable to the app process for signing and is not hardware-backed.
- Derive the local `DeviceId` as UUIDv8-shaped bytes from the public-key fingerprint. Return only those 16 public bytes across the `mm_ffi.dll` C ABI; no key, signature, peer record, or video data crosses it.
- Register `com.mirrormesh/windows_pairing` in the Windows Flutter runner. `preparePairingRuntime` resolves the exported initializer from the already loaded `mm_ffi.dll` and is called only by the Devices-page button. Failures return a generic unavailable error.
- Do not start the WSS listener, discovery, trust approval, capture, or streaming as part of bootstrap. Those remain separate consent and integration steps.

## Security and lifecycle

The C ABI rejects a null or undersized output buffer and catches Rust panics before returning to C++. The runtime mutex prevents duplicate initialization. A failed initialization leaves the process slot empty so a later explicit attempt can retry. The fixed SQLite filename is joined to the OS-provided LocalAppData path; no caller-controlled path is accepted. Credential Manager and SQLite application runtime behavior have not been exercised against a real Windows user profile.

## Verification

Rust tests cover stable UUIDv8 derivation, one-time process retention, retry after initialization failure, the fixed LocalAppData database path, and C ABI buffer rejection. Flutter tests cover the Windows method channel and the Devices action; the widget test confirms no channel call occurs until the user taps. The Windows Rust DLL target builds and exports `mirrormesh_windows_prepare_pairing_runtime`; the runner translation unit compiles with MSVC `/W4 /WX`. The full Flutter Windows build currently stops at CMake generation because the validation mirror has no WebRTC/JNI plugin symlink directories. No real Credential Manager call, app startup with Windows pairing, network, capture, or device test ran.

No external dependency or wire-protocol change is introduced. The bridge addition is a proposed platform-specific application boundary and needs cross-track review before approval.
