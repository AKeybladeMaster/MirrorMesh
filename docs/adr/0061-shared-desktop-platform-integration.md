# ADR0061 — Shared Linux/macOS desktop integration and native receiver status

Status: implemented in source on `test_6`; per-platform build evidence is recorded in
`docs/IMPLEMENTATION_STATUS.md`. macOS compilation and signing require a Mac and remain pending.

## Decision

Retain the existing Rust protocol, identity proof, raw-public-key TLS pinning, explicit
approval, trusted-peer SQLite storage and WebRTC session coordinator. Linux and macOS
desktop UIs use the same bounded metadata command API already used by mobile receivers.
No media frames cross Dart/FFI. Windows retains its existing native sender method channel.

Linux composes the existing Secret Service identity and private-database initializer with
the shared native pairing listener. Revocation uses the same public peer store. Failure
to unlock Secret Service fails closed; no plaintext seed fallback is added. Native GTK
provides sanitized public device facts and selects one active physical LAN interface,
preferring its default route. Tunnel/virtual interfaces are excluded.

macOS reuses the Apple identity adapter and C ABI callbacks with a separate desktop
Keychain service namespace. Items are non-synchronizable and available only when unlocked;
the seed remains exportable inside the app process, not a Secure Enclave key. Native
network path/device facts, local-network purpose strings, screen-capture purpose string,
and sandbox network client/server entitlements are included. Static Rust exports used by
Dart are retained explicitly. Keychain entitlement/signing behavior is unverified until
an appropriately signed build runs on a Mac.

The approved signaling command API preserves specific remote stop/revoke events and
local stop reasons; wire protocol version, grants, nonce rules and trust decisions do
not change. Local capture still requires explicit source selection and OS permission.

## Native casting boundary

Windows Miracast continues to use Microsoft's separate Wireless Display app. A bounded
worker queries OS capability/listening status; a popup distinguishes listening, pending,
connected, disabled and policy states. Only an explicit button opens projection settings.
The query neither approves a device nor starts media. OS status does not establish decoded
frames or repair driver/handshake failures. No driver, firewall or policy is changed.

No compatible permanently free Google Cast Windows provider has been verified. The owner
declined paid providers and authorized Miracast/MirrorMesh as the available choices.
The unavailable Google Cast GUI option is removed; no provider is installed or bundled.

## Build boundary

Linux builds run in a native Linux environment. The Windows Docker wrapper checks test_6,
snapshots explicit source roots and compiles inside a private container filesystem, so it
does not overwrite Windows Flutter/Cargo outputs. The pinned Flutter/Rust versions and
locked project dependencies are retained. The Linux archive contains a complete bundle,
library dependency report and SHA-256 manifests; it still requires compatible system GTK,
graphics/audio libraries and an unlocked desktop Secret Service.

macOS source includes a universal ARM64/x86_64 build command. Windows/WSL cannot supply
Apple's Xcode SDK, signing or notarization; no macOS product is claimed without that build.
Builds/static analysis are permitted; test suites/workflows remain excluded by branch policy.
