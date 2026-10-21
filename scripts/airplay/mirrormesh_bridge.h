// MirrorMesh UxPlay integration. SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#ifdef _WIN32
bool mm_bridge_enabled();
void mm_bridge_init();
void mm_bridge_ready();
void mm_bridge_pin(const char *pin);
bool mm_bridge_approve(const char *name, const char *model, const char *source_ip);
void mm_bridge_idle();
void mm_bridge_video_idle();
#else
inline bool mm_bridge_enabled() { return false; }
inline void mm_bridge_init() {}
inline void mm_bridge_ready() {}
inline void mm_bridge_pin(const char *) {}
inline bool mm_bridge_approve(const char *, const char *, const char *) { return true; }
inline void mm_bridge_idle() {}
inline void mm_bridge_video_idle() {}
#endif
