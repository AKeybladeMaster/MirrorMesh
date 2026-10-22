# Linux x86_64 preview

Build/package verified on Ubuntu 24.04 with pinned Flutter 3.47.7 / Rust 1.99.1.
Use `../MirrorMesh-0.4.1-linux-x64.tar.gz` and its `.sha256`; extract the complete archive
and run `./MirrorMesh/MirrorMesh` as your desktop user. Keep `lib/` and `data/` together.

Requires a compatible GTK3 desktop, graphics/audio system libraries and an unlocked
Secret Service keyring on the session D-Bus. The build uses glibc 2.39. Linux desktop
capture, Wayland portals, decoding and actual device media remain unverified.
No hardware or production-complete claim is made. App sharing is video-only.

Build from Windows: `scripts/build-linux-docker.ps1` with Docker Desktop running.
Build on Linux: `sh scripts/build-linux.sh`.
See [full build instructions](../../docs/build-release.md).
