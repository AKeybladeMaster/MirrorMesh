# ADR-0003 — Concurrent OS implementation; native Android TV first, browser fallback

- **Decision ID:** MM-PLAN-001
- **Decision date:** 2026-10-18
- **Status:** Accepted by requester, implementation unstarted

## Context

MirrorMesh targets Windows x64, Linux x64, Android phone/tablet ARM64 and iOS/iPadOS ARM64 installations. A Windows-first rollout would delay feedback on mobile OS permission APIs, codec packaging, Android TV receiver deployment, and Apple's version-dependent system capture. Television support cannot rely on a generic open TCP socket: native Android TV, ordinary TV browsers, Cast, Miracast, and AirPlay have different constraints.

## Decision

1. Start six development workstreams at P1: WIN, LNX, AND, IOS, TV (native Android TV), WEB (receiver browser), plus CORE integrator. Tasks 05a–05f run in separate owned branches with independent CI/SDK mocks.
2. Shared protocol v1, capture/encoder/renderer ports, identity/consent and golden fixtures are frozen before *real-media* integration; a WebRTC implementation is selected after ADR-0002 test evidence.
3. **Android TV native receiver APK is primary** for verified PC/phone→TV sessions. TV shell is optimized for D-pad approval/disconnect, native H.264 decoding and explicit user consent.
4. **WebRTC browser receiver is optional fallback** when an individual TV browser passes secure-origin, authentication, H.264 codec and performance tests. No assumption of universal TV WebRTC.
5. Each platform has separate release gates. A missing API, hardware or Apple signing requirement results in `BLOCKED` instead of fabricated support. Receiver-only iOS may be demonstrated with explicit labeling when sender capture is unavailable.
6. Cast/Miracast/AirPlay are later separate protocol adapters, not alternate names for native Android TV or browser receiving.

## Trade-offs

**Advantages:** mobile/Linux restrictions discovered early; fewer late integration surprises; faster evidence for the true TV destination; incremental release artifacts per platform.

**Costs:** more simultaneous review/integration demand; real Mac/Xcode/iOS/TV test fixtures required; cross-platform native media ABI details may constrain the chosen WebRTC stack; greater CI/signing maintenance.

**Mitigation:** CORE API ownership, versioned protocol fixtures, daily merge cadence, synthetic media, per-platform feature flags and no raw frames over UI bridge.

## Verification

- Protocol/FFI fixture gates before real media.
- Matrix: each shipping sender→native Android TV and at least one native desktop receiver, and each native receiver←a known sender.
- Native Android TV acceptance and browser fallback acceptance are **independent**.
- Physical hardware evidence, signed artifacts and capability-specific README labels.

See [SPEC-001](../SPEC-001-mirrormesh.md), [implementation](../implementation.md), [Codex tasks](../../codex/README.md) and [verification ledger](../verification.md).
