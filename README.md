# MirrorMesh

Local screen sharing from a **Windows x64 PC to a native Android phone, tablet or TV receiver**.
This `test_2` branch contains Android UI/accessibility improvements, based on `test` at
`d5d6053`. The working `test` branch and all other branches remain unchanged.

## Get started

1. [Install and use the apps](docs/USER_GUIDE.md).
2. [Build Windows and Android](docs/build-release.md).
3. [Read the actual implementation and build status](docs/IMPLEMENTATION_STATUS.md).

The implemented path uses an expiring receiver invitation, pinned TLS 1.3 signaling,
Ed25519 sender identity, explicit receiver approval, native WebRTC H.264 capture and
transmission, and Android MediaCodec decoding with fullscreen rendering. Received-frame
screenshots, disconnect, bounded reconnection and receiver revocation are integrated.
It requires both devices to be reachable on the same LAN; no cloud login is needed.

**This is a release candidate, not a hardware-verified final product.** Physical media
delivery, encoder compatibility and performance still require user verification.
Windows binaries are unsigned. Android debug APKs are installable previews; unsigned
release APKs need the owner's signing credentials before distribution.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/` | Shared Rust domain, protocol, identity, trust, sessions and native adapters |
| `apps/mirrormesh_flutter/` | Windows sender UI, native capture/WebRTC bridge; retained platform shells |
| `apps/android-tv/` | Shared native Android receiver with `phone` and `tv` flavors |
| `apps/tv-web/` | Browser receiver foundation; authenticated media integration remains incomplete |
| `scripts/` | Local build and artifact packaging tools |
| `releases/` | Local binaries, bundles and checksums; large generated binaries are Git-ignored |
| `docs/` | Build/use guides, implementation contracts and architecture decisions |

Per the owner's instruction, this branch contains no unit/instrumentation test sources,
fuzz targets or GitHub Actions workflows. Their history remains on `dev`. Local compilation,
formatting and static analysis are used here without executing test suites.

The broader design still includes Linux, Android sending, iOS and browser reception.
Those retained foundations are **not** complete products. See [status](docs/IMPLEMENTATION_STATUS.md)
and [the original specification](docs/SPEC-001-mirrormesh.md).
