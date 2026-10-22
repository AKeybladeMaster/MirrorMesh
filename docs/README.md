# Documentation

| Guide | Contents |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Workspace tree, crate map, platform code and connection lifecycle |
| [Build and personal releases](build-release.md) | Pinned tools, native builds, packaging, signing boundaries and debug commands |
| [User guide](USER_GUIDE.md) | Installation, consent, screen sharing, disconnect and screenshots |
| [Connection directions](connections.md) | MirrorMesh, native AirPlay and system Miracast flows |
| [Android TV installation](install-android-tv.md) | USB, phone-transfer limitations, ADB and ABI compatibility |
| [AirPlay](airplay.md) | Windows native receiver, helper/runtime setup and troubleshooting |
| [Readiness](IMPLEMENTATION_STATUS.md) | Source vs build vs hardware evidence; known incomplete products |
| [Verification](verification.md) | Actual compiler/static/artifact checks and pending device verification |
| [Protocol](protocol.md) | Shared authenticated control-plane contract |
| [Security and storage](security-legal.md) | Identity, permission, privacy and distribution boundaries |

The root [README FAQ](../README.md#faq--connection-and-playback) summarizes encountered bugs
without exposing private machine/network information. Current ADRs are in `adr/`.
[The original specification](SPEC-001-mirrormesh.md) is broader design scope, not a completion report.
Old incremental reports and candidate ADRs remain in the `test_6` checkpoint's Git history.
