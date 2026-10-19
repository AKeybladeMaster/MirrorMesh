# Device and protocol compatibility

**Legend:** `P2-*` = concurrently developed workstream, `INT` = P3 integration test, `R` = research, `N/A` = not applicable. Product support still requires dated real-device evidence. Current scaffolds: Flutter has a four-tab control shell and one Windows compile; Linux Rust bridge bundling and post-build CI smoke testing, Android ARM64 Rust library/APK packaging, and iOS device static-library linking with an FRB export list are configured but await platform runner results; the iOS simulator bridge remains unavailable. Apple's current iOS screen-capture sample requires iOS 27 or later; the API path is recorded in [the iOS capture spike](spikes/ios-capture-spike.md), with SDK and hardware validation pending. TV browser capability/status and remote-focus logic has seven passing Node tests and a headless Edge 155 localhost capability smoke; no physical TV browser was tested. Android TV has a tested local approval state demo and a source check for polite help-text announcements, but remains SOURCE ONLY (no SDK build or TV install). No streaming target has passed a device test. The proposed CORE discovery candidate port retains IPv4/IPv6 socket endpoints, including IPv6 scope, but no platform app assembles it and multicast compatibility remains unverified.

**Decision MM-PLAN-001:** Windows x64, Linux x64, Android ARM64 and iOS/iPadOS ARM64 **start in parallel**. Primary TV destination is an installable **native Android TV/Google TV receiver**; WebRTC in a TV web browser is an **optional, tested-only fallback**. Neither path equals Google Cast or Miracast.

## P3 integration readiness (2026-10-18)

Task 07 is **BLOCKED at its entry gate**: no implemented sender and native Android TV receiver have exchanged the candidate protocol, ADR-0002 selects a transport but its engine integration is not yet built, and the TV APK workflow has not run. The browser fallback is **MOCK** (seven Node logic tests plus a headless Edge 155 localhost capability smoke; no TV-browser, signaling, or media run). The required pathway-by-pathway status and evidence template are in the [integration readiness spike](spikes/integration-readiness.md). No device is listed as supported.

**Pairing identity compatibility:** The proposed Ed25519 encoding in [ADR-0007](adr/0007-ed25519-identity-candidate.md) has a stateless Rust verifier candidate in `crates/mm-adapter-identity/`. It is not wired into a platform app or pairing coordinator, and no Windows, Linux, Android, iOS, or Android TV build has verified the provider. Keep every platform marked unverified until target builds and owned-device pairing tests pass.

**Android local identity candidate:** Task 22 adds `AndroidPairingIdentity` in `crates/mm-adapter-android/`, using an Android Keystore AES-GCM wrapping key for the shared Ed25519 seed. Task 23 attempts to load the Rust library and, when present, initializes the Android application context before Flutter starts, under proposed [ADR-0028](adr/0028-android-ndk-context-bootstrap.md). Task 25 assembles that identity and `SqliteTrustedPeers` into a process-owned `PairingRuntime` only after the user presses **Prepare pairing support**, under proposed [ADR-0030](adr/0030-android-pairing-runtime-assembly.md). The returned UUIDv8 is fingerprint-derived public metadata; its uniqueness is implementation-specific. Thirteen host adapter tests, three host FFI tests, Rust workspace gates, and Flutter analysis/tests pass. Android `mm-ffi` ARM64 compilation, APK startup, actual Keystore operations, and device pairing remain unverified because NDK clang/SDK is unavailable.

**Session trust compatibility:** The proposed Rust `TrustedSessionRuntime` in [ADR-0017](adr/0017-trusted-session-revocation-runtime-candidate.md) gates starts on active trust and closes tracked `TransportPort` sessions. [ADR-0019](adr/0019-trusted-session-grant-scope-candidate.md) adds a caller-specified required-grant check against stored approval before transport start. Fake-backed unit and real-verifier/SQLite integration tests cover the grant denial and allowed-scope paths; a file-backed SQLite integration confirms revocation survives store reopen. Proposed [ADR-0018](adr/0018-pairing-session-runtime-composition-candidate.md) composes pairing and session lifecycle over one store, but no platform signaling, capture, or codec adapter uses the facade; direction selection and platform disconnect/revoke compatibility remain unverified.

**Pairing wire compatibility:** Proposed [ADR-0020](adr/0020-pair-begin-wire-encoding-candidate.md) specifies the strict `pair.begin` JSON payload; proposed [ADR-0021](adr/0021-pair-begin-outbound-encoding-candidate.md) adds a shared Rust encoder, and proposed [ADR-0022](adr/0022-sender-proof-creation-candidate.md) adds application-side proof creation through the protected signing port. Golden-fixture and real-signature integration tests are Rust-only evidence. No Windows, Linux, Android, iOS, Android TV, or browser track has implemented or approved this candidate schema; cross-track review is required before protocol v1 is frozen.

**TLS/signaling compatibility:** Proposed [ADR-0023](adr/0023-tls13-raw-public-key-pinning-candidate.md) adds a native Rustls TLS 1.3 raw-public-key server config and a QR fingerprint pin verifier. Proposed [ADR-0024](adr/0024-bounded-wss-pairing-listener-candidate.md) adds a bounded native WSS listener; its separate-process loopback exchange runs on Windows and verifies QR-pinned TLS, signed `pair.begin`, and approval-only persistence. Tasks 20 and 21 add Windows Credential Manager and Linux Secret Service `LocalTlsIdentityPort` candidates; Task 22 adds the Android Keystore-wrapped identity/TLS signer candidate. Their host tests use fake stores/ciphers, and no real OS key store or app path has exercised these TLS signers. The browser fallback remains a separate normally trusted HTTPS/WSS endpoint. Every platform remains unverified.

**Annotation compatibility:** normalized-coordinate, `BoxFit`, and quarter-turn transforms pass Flutter unit tests on this Windows host. Flutter widget tests cover per-stroke palette/width selection, semantic control labels, and P / Ctrl+Z / ⌘+Z shortcuts; an overlay golden renders over a synthetic RGB frame. Native keyboard behavior, decoder orientation, texture/surface lifecycle, and input latency have not been validated on any device row.

**Responsive shell:** a Flutter widget test confirms bottom navigation at 400 logical pixels and the navigation rail at 800 logical pixels with a simulated 2x device-pixel ratio. Physical monitor scaling, display rotation, and touch behavior remain unverified.

| Source → destination | Proposed path | Target | Limitation / proof needed |
|---|---|---|---|
| Windows PC → Windows PC | Native Capture/Encoder + encrypted WebRTC + native decode | P2-WIN / INT | GPU compatibility, Windows capture prompt, firewall |
| Windows PC → Android TV / Google TV | WebRTC to installed native Android TV receiver APK (**primary**) | P2-WIN + P2-TV / INT | Need installable TV receiver and decode H.264 |
| Windows PC → TV browser | Secure-origin browser WebRTC receiver (**fallback**) | P2-WEB / INT | TV WebRTC/codec/origin restrictions vary; manual testing |
| Windows PC → Chromecast | Google Cast registered custom receiver/adapt media | P2 | Standard Cast is not automatically a live WebRTC sink |
| Windows PC → Miracast TV | OS-supported Miracast projection delegation / native adapter | R | Wi-Fi Direct + Miracast stack; **not regular TCP-on-LAN** |
| Windows PC → Apple TV | Licensed-compatible AirPlay sender adapter or system delegation | R | Proprietary protocol and legal/compatibility checks |
| Linux desktop → native receivers and Android TV | PipeWire+portal (Wayland); X11 fallback → WebRTC | P2-LNX / INT | Portal consent, compositor/DMABUF/HW encoder mix |
| Android phone/tablet → Windows / native Android TV | MediaProjection + MediaCodec → WebRTC | P2-AND / INT | Active user consent, foreground service restrictions |
| iPhone/iPad → Windows / Android TV | ScreenCaptureKit on verified supported iOS; legacy feasibility separate | P2-IOS / INT | Exact API deployment & broadcast policy; no silent capture |
| Windows → Linux native receiver | Native H264 / GPU texture over authenticated WebRTC | P2-LNX / INT | Linux runtime/codec/Wayland integration |
| Windows → Android phone native receiver | MediaCodec decode/Surface + Flutter overlays | P2-AND / INT | activity/surface lifecycle |
| Windows → iPhone/iPad native receiver | VideoToolbox decode/native texture + Flutter | P2-IOS / INT | test actual iOS hardware independent of send API gate |
| Android TV → desktop | TV as sender is **not MVP** | R | installed Android TV app is receiving-only |

| iOS → Apple TV | OS AirPlay screen mirroring | R | May delegate to system; app-owned stream isn't guaranteed |
| Linux phone → receiver | PipeWire/portal or distro-specific capture adapter | R | Hardware/OS differs (postmarketOS, Mobian etc.) |
| PSP → desktop/TV | PSP homebrew encoder/streamer OR USB capture hardware | R | Stock PSP no general remote-screen OS API, very limited CPU/Wi-Fi |
| Smart TV → PC | Only if TV exposes a screen-capture source / manufacturer APIs | R | Most TVs don't expose full display capture to third-party apps |
| Browser tab → other device | getDisplayMedia on supported browser with user gesture | P2 | Secure-origin/permission policies vary |
| Camera → other device | Optional separate camera capture mode | P2 | Not the same as screen mirroring |

## TV receiver paths — approved priority (MM-PLAN-001)

1. **PRIMARY: Android TV / Google TV app**: Native receiver installed via Play Store or developer sideload for testing; predictable application-owned player, WebRTC engine and overlays. TV must support APK / compatible Android device.
2. **FALLBACK: Browser on TV**: Visit a hosted HTTPS pairing page; negotiate media over WebRTC. Browser support, remote-control input, codec profiles, autoplay/audio and network boundaries vary. A local-only `http://IP` receiver is *experimental*, not guaranteed due to secure context, browser restrictions and local-network requests.
3. **Google Cast**: A separate sender/receiver integration with registered Cast application ID and appropriate media pipeline. A Cast default media receiver plays URLs; it is not a general desktop-mirroring WebRTC endpoint. Plan a server-side repackaging/segmented stream fallback only if latency acceptable; test before promising.
4. **Miracast**: Native OS/driver-specific implementation, typically Wi-Fi Direct; no universal userland socket bridge. Offer an OS delegation button if an appropriate API exists; maintain a separate feasibility ADR.
5. **AirPlay**: Receiver or sender via lawful public implementations and compatibility validation; avoid implying full modern Apple protocol support.

**TV proof matrix** to collect:
`vendor | model | year | OS/browser version | recv app mode | codec/profile | 1080p60 pass? | audio? | median/p95 latency | autoplay? | HTTPS transport | issues`.

## Radio and networking reality

- BLE advertising/scan may help identify device IDs or exchange onboarding hints but cannot be treated as a 1080p screen-video transport.
- Same Wi-Fi is helpful, **same subnet/multicast visibility** is needed for mDNS discovery. Client isolation/VLAN/VPN may prevent this; manual address and TURN can address different cases. The shared Rust browser candidate currently has parser/unit-test evidence only; platform permission behavior and owned-LAN interoperability remain unverified.
- WebRTC uses ICE for candidates, DTLS/SRTP for media, and a separate authenticated signaling path; simply opening a TCP listener does not implement interoperability.
- Tethering/hotspot and USB are optional transport alternatives if both devices permit IP traffic.
- Chromecast, AirPlay, Miracast, DLNA and app-specific WebRTC are **different** protocols with different capabilities and conditions.

## Cross-platform acceptance rule

Tracks P2-WIN, P2-LNX, P2-AND and P2-IOS start at the same time with independent SDK/mock CI jobs; P2-TV and P2-WEB also start concurrently. A shared protocol v1 + transport ADR must be approved before real media code lands. At P3, verify **every shipping sender→Android TV + one desktop receiver** and **Windows→each native receiver**. Browser fallback requires named TV/browser/model + secure-origin/H264 evidence. Track statuses are `PLANNED | MOCK | BUILDS | E2E_PASS | BLOCKED`; do not infer untested interoperability.

## iOS-specific feasibility gates

As of the referenced documentation, Apple positions **ScreenCaptureKit** as the preferred screen streaming and mirroring API across Apple platforms. Its current iOS sample requires **iOS 27+**. An implementation targeting older iOS versions needs a separate availability assessment and legacy ReplayKit fallback (which current reference pages mark deprecated) *if still permitted by OS and App Review*.

**Codex action:** build `ios-capture-spike.md` with iOS version vs compile/runtime support, background behavior, user picker, device orientation, audio capture and App Store review results. Do not advertise a generic iOS sender until it works on physical device with approved system UI.

## Candidate research projects

- **scrcpy:** authorized Android/ADB mirroring and low-latency design reference; not a TV-general architecture.
- **Sunshine + Moonlight:** mature hardware-accelerated gaming-streaming pattern; not drop-in compatibility with AirPlay/Cast.
- **Deskreen:** browser-based second-screen/web receiver design comparison.
- **UxPlay:** Unix AirPlay receiver interoperability reference; verify its licensing before any reuse.

See `docs/sources.md` for official links. Do not merge GPL code into differently licensed core without explicitly selecting a compatible project license.
