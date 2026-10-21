# ADR 0059 — Native AirPlay adapter and bounded receiver view

## Request and scope

The owner confirmed PC↔Android works and requested a whole-image PC viewer plus iOS
Control Center Screen Mirroring → PC. This authorizes a distinct AirPlay interoperability
adapter. It cannot use the app-owned WebRTC signaling protocol. Work remains on test_3.

## Presentation

The connected WebRTC receiver uses a bounded Column/Expanded native RTCVideoView with
contain fitting. It is no longer a fixed 16:9 rectangle inside a scrolling page. Setup
remains scrollable; the actual picture and Stop control remain within the viewing window.
Windows full-screen preserves/restores the previous monitor/window placement. F11/Escape
and the button control the same window. Existing capture/identity/media flows are retained.

## AirPlay boundary

- Use pinned UxPlay 15a6b87172eca38b0df4e615fbf1bba108734dde as an isolated GPL companion
  executable. No GPL library is linked into Flutter/Rust. No claim about the final combined
  distribution's licensing is made; the local preview includes notices and modified source.
- A Windows MethodChannel carries status, consent and native pixel bounds only. GStreamer
  D3D11/FFmpeg decoding and WASAPI audio remain native. The helper owns its video HWND under
  an app-owned native container; this avoids subclassing a foreign-process video window.
  Per-monitor DPI context and native resize updates keep the image bounded.
- Start only after explicit user action. Fixed arguments require an upstream random PIN on
  each connection, ephemeral AirPlay keys, no external config, recording or HLS. A second
  PC Allow/Reject decision gates SETUP before media starts. It times out/rejects in 30s.
  This is AirPlay authentication; it does not downgrade QR/TLS checks for MirrorMesh sessions.
- The helper receives a private status pipe plus consent stdin. Its arbitrary upstream logs
  go to NUL, preventing client labels containing line breaks from forging UI status/events.
  Only fixed metadata, four PIN digits and request IDs cross that pipe. No SDP or raw media.
- Spawn suspended, assign to a kill-on-close Windows Job, then resume. Inherit only the
  three necessary handles. Stop/page/app exit terminates the process tree; this is lifetime
  containment, not a security sandbox. No service, privilege elevation or firewall rule.
- Decoded sink buffers announce video readiness. Video reset/RTP shutdown clear video state
  independently of unrelated open sockets. Native pipeline errors close the helper. Flutter
  generation guards reject stale status replies after stop/disposal.
- Harden the new upstream password boundary: reject missing/oversized device identifiers,
  malformed/missing Digest fields and nonce bounds, and fail closed if PIN randomness or
  allocation fails. Never fall back to upstream's predictable 1234 PIN. No custom crypto.

## Builds and compatibility

### 0.3.1 startup repair

Packaging must classify plugins by their source directory; matching filenames misplaced
the imported core `libgstreamer-1.0-0.dll`. Include the pinned helper's required `autodetect`
plugin even though the app specifies fixed sinks. Verify imports in the final runtime
layout and run help with a restricted PATH to detect loader failures before packaging.

Explicit `--diagnose-airplay` uses the same native host and private status/approval pipes.
It briefly advertises on the user's LAN, requires two seconds of stable Ready within 15
seconds, approves no clients and stops its Job. The packaged support script requires
`-CheckStartup` to opt into network initialization; its default checks files/loader only.
No arbitrary helper log, PIN, client identity or media is printed. No firewall rule or
privilege change is introduced. This diagnostic establishes local startup only.

Toolchain bootstrap and packages are pinned by SHA-256; pacman signature verification
remains required. The build rejects installed version drift. Disable host-specific CPU
instructions. Package selected plugins/import closure, licenses, modified helper source,
patch, lock and setup/build scripts. Full dependency source redistribution review remains
a public-release prerequisite; locally generated binaries are not committed to Git.

No MirrorMesh Rust protocol/dependency/permission changes are required. Android 0.2.1 is
still compatible with Windows 0.3.0 for native sessions. iOS app/ReplayKit source remains
available, with Apple builds pending. Native AirPlay needs no installed iOS app.

Compile/static evidence is tracked separately from hardware. No unit/instrumentation or
workflow test is authorized. No runtime, physical AirPlay interoperability, screenshot,
DRM, AWDL or multi-room support claim follows from successful compilation.
