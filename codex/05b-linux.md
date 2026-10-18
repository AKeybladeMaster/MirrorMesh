# TASK 05b — LNX: Linux x64 sender AND receiver (P2-LNX)

**Owner:** LNX; paths `crates/mm-adapter-linux/`, `apps/mirrormesh_flutter/linux/`, Linux packages. Start portal + synthetic-media spike immediately; **real transport gate:** CORE fixture + ADR-0002.

1. Build Linux x64 crate with feature gates; Wayland XDG Desktop Portal screen capture through PipeWire consent is default.
2. Probe PipeWire DMA-BUF for GPU-owned frame path; note compositor/driver limitations and explicitly measure copies.
3. Choose VA-API/V4L2/GStreamer as transport ADR permits. Fallback to software encode must declare CPU load/resolution limit.
4. Implement native video receive/decode and Flutter Linux texture/surface ownership; use protocol test vectors for session states.
5. Address X11 separately with a gated capture mechanism and a permission explanation; no invisible privileged capture.
6. Run real `LNX→Android TV` and `LNX→Windows` sender tests, `WIN→Linux` receiver test; test revoked portal permission, monitor unplug, 60-minute soak.
7. CI package `.deb` and AppImage on pinned distro, test clean GNOME/KDE Wayland and one X11 fixture; document shared library/codecs licensing.

**Acceptance:** sender/receiver hardware proof, controllable permission prompt, tested Linux x64 binary. Mark unsupported desktop environment/driver combinations clearly.
