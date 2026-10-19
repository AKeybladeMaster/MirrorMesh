# Codex execution program — parallel platforms, gated interfaces

**Decision MM-PLAN-001 (2026-10-18):** Windows, Linux, Android and iOS launch engineering runs in parallel. Android TV native receiver is the **primary TV path**; TV browser WebRTC receiver is fallback on tested engines. The repository now contains a verified Rust scaffold and FFI integration tests, generated Flutter bridge bindings and a local-only demo UI, a capability-gated web shell, native Android TV standby source checks, CI workflows, and a deferred transport ADR. Windows Flutter shell compilation passed; Android TV and mobile/Linux platform builds remain unverified. No product streaming path has passed integration or hardware validation.

## What "parallel" means

Do **00, 01 and the contract/transport sections of 02** as a coordinated bootstrap. Once protocol fixtures and FFI/capture contracts are reviewed, run **05a–05f concurrently** in distinct branches/working directories/agent contexts. Work on mock/native SDK scaffolds may start in parallel *even before* ADR-0002 selects a WebRTC implementation; integrating real media must wait for its decision. Tasks 03 (pairing), 04 (UI) and 06 (CI) can be owned by dedicated integrators alongside native tracks. Task 07 performs continuous matrix integration.

**Do not interpret numerical task labels as a serial schedule.** Each task file remains **one contained PR or short PR series**, with explicit reviewer approval between changes. An agent may never merge/ship other agents' incomplete work without review. Never claim unsupported hardware was tested.

## Dependency and ownership board

| Task | Workstream | Earliest start | Blocking prerequisite | Milestone |
|---|---|---|---|---|
| [00 REA](00-rea-investigation.md) | research | day 1 | lawful authorization for binary | P0 |
| [01 Core](01-bootstrap-core.md) | CORE | day 1 | none | P1 |
| [02 Transport](02-transport-spike.md) | CORE/media | day 1 | codec/browser fixtures for ADR; native integration after choice | P1 |
| [03 Discovery/pairing](03-discovery-pairing.md) | CORE/security | after v1 envelope freeze | identity/consent and test cert contracts | P2/P3 |
| [04 GUI/screenshots](04-ui-annotations.md) | GUI | after fake session events | native texture integration needs adapter handles | P2/P3 |
| [05a Windows](05a-windows.md) | WIN | day 1 SDK/mock; real media after 01/02 | signed v1 API and media choice | **P2-WIN** |
| [05b Linux](05b-linux.md) | LNX | day 1 SDK/mock; real media after 01/02 | same | **P2-LNX** |
| [05c Android](05c-android.md) | AND | day 1 SDK/mock; real media after 01/02 | same | **P2-AND** |
| [05d iOS](05d-ios.md) | IOS | day 1 SDK/mock; real media after 01/02 | same + Mac/Xcode/device | **P2-IOS** |
| [05e Android TV](05e-android-tv.md) | TV | day 1 TV UI/mock; real media after 01/02 | same + TV/device | **P2-TV primary** |
| [05f TV web](05f-browser-receiver.md) | WEB | day 1 static UI/mock; real media after 01/02 | same + secure origin/browser | **P2-WEB fallback** |
| [07 Cross-OS integration](07-integration-matrix.md) | integration | after first sender+TV receive | fixture version and test hardware | P3 |
| [06 CI/packaging](06-quality-release.md) | release/QA | day 1 CI; signing after native builds | per-platform build and hardware verification | P4/P5 |

Legacy [05](05-device-adapters.md) is a **directory/index** of the new independent platform tasks, not a giant PR.

## Codex coordinator kickoff prompt

```text
You are MirrorMesh's architecture/integration agent. Read AGENTS.md, docs/SPEC-001-mirrormesh.md,
docs/protocol.md, docs/implementation.md and codex/README.md.
User-confirmed decision MM-PLAN-001: parallel Windows/Linux/Android/iOS engineering;
primary native Android TV receiver; capability-gated TV browser fallback.

Start P0/P1 coordination ONLY. Propose independently owned worktrees/PRs for CORE, WIN,
LNX, AND, IOS, TV and WEB; do not make claims of implementation or binary examination.
Review authorization before running task 00 on proprietary materials. Define protocol v1
golden fixtures, media ownership/FFI contracts and ADR-0002 decision gate.
Then produce a status board showing each workstream's PASS/FAIL/BLOCKED and next PR.
Do not edit another workstream's owned paths without review.
```

## Per-track agent prompt (run in separate workspace after shared fixtures)

```text
You are the <TRACK> agent implementing codex/05<letter>-<platform>.md.
Read AGENTS.md, SPEC-001, protocol.md, implementation.md, ADR-0002, and your task brief.
Respect CORE ownership; do not modify protocol or shared traits without an RFC/PR review.
Begin with native API feasibility + fake-media CI build; use explicit user consent.
Add real media only when common transport decision is approved. Provide compiled artifact
when SDK available, exact test commands, hardware proof or an explicit BLOCKED report.
Update device-matrix.md and verification.md with truthful statuses only.
```

## Merge cadence and acceptance

- Require `CODEOWNERS` approval from **CORE** for protocol/trait/FFI changes, **security owner** for identity/signaling, and track owner for platform patches. CI runs on every PR.
- Pin golden fixture commit SHA; bump protocol minor versions only additively and verify compatibility, otherwise version/ADR migration required.
- Every 1–2 days integrate the newest compatible sender into Android TV **and** one desktop; the TV browser track tests identical offers on its actual browser fixture.
- Each agent posts `RESULT: PASS | FAIL | BLOCKED`, paths, changed APIs, commands/exit codes, fixture, security/permission proof, measurements, and device matrix row.
- If no Mac/iOS27 device, mark iOS capture hardware gate `BLOCKED`, **continue** Swift bindings and native receiver work. Do not claim iOS mirroring is released.
- Release only passing targets. Priority does not override objective validation.

## Recent functional integration tasks

| Task | Scope | Brief |
|---|---|---|
| 49 | Android TV synthetic H.264 loopback | [49-tv-synthetic-h264-loopback.md](49-tv-synthetic-h264-loopback.md) |
| 50 | Windows first-encoded-frame gate | [50-windows-first-encoded-frame-gate.md](50-windows-first-encoded-frame-gate.md) |
| 51 | Windows peer reconnect | [51-windows-peer-reconnect.md](51-windows-peer-reconnect.md) |
| 52 | Android TV production signaling adapter loopback | [52-android-tv-production-signaling-loopback.md](52-android-tv-production-signaling-loopback.md) |
| 53 | Android TV signaling lifecycle tests | [53-android-tv-signaling-lifecycle-tests.md](53-android-tv-signaling-lifecycle-tests.md) |
