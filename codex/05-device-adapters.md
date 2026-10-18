# TASK 05 — Platform adapter dispatch (parallel workstreams)

**DO NOT implement all platforms in one PR.** This index routes six independent Codex agents to files that own non-overlapping code paths. All tracks start architecture/SDK/mock work at P1 and real media work at P2 once ADR-0002 and shared golden fixtures are ready.

| Branch/track | Task | Acceptance scope |
|---|---|---|
| `WIN` | [05a — Windows](05a-windows.md) | WGC/MF sender and receiver, Windows x64 package |
| `LNX` | [05b — Linux](05b-linux.md) | PipeWire/portal sender and receiver, x64 Linux packages |
| `AND` | [05c — Android phone](05c-android.md) | MediaProjection sender and mobile receiver, ARM64 package |
| `IOS` | [05d — iOS/iPadOS](05d-ios.md) | system picker capture feasibility + receiver, signed IPA/TestFlight |
| `TV` **primary** | [05e — Android TV](05e-android-tv.md) | native receiver APK, remote pairing, hardware decode |
| `WEB` fallback | [05f — TV browser](05f-browser-receiver.md) | secure-origin WebRTC receiver on tested browser, explicit unsupported UX |

Before coding: inspect `AGENTS.md`, `docs/SPEC-001-mirrormesh.md`, `docs/protocol.md`, `docs/implementation.md`, the chosen transport ADR and official API docs. No platform agent changes shared Rust contracts without CORE review.

Every PR produces: changed paths, SDK/codec support matrix, permissions/consent behavior, build log, relevant unit tests, one real hardware recording when available, a `PASS/FAIL/BLOCKED` gate and next integrator handoff. Never claim TV/browser universality. See [07 integration](07-integration-matrix.md).
