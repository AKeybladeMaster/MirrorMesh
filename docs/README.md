# MirrorMesh design documentation index

Start with [SPEC-001](SPEC-001-mirrormesh.md) and [device matrix](device-matrix.md), then review [protocol](protocol.md) and [security](security-legal.md). Follow the dependency graph and concurrent workstream ownership in `../codex/README.md`; task numbers are not a strict serial schedule.

## Design documents

| Topic | Path |
|---|---|
| Product background / MoSCoW / Method / Milestones / Results | `SPEC-001-mirrormesh.md` |
| REA authorized research SOP and blank evidence report | `research/airmirroring-rea.md`, `research/airmirroring-evidence-template.md` |
| Agent skills and how to load them | `skills.md` |
| OS/device/TV protocols and limits | `device-matrix.md` |
| App GUI and full feature list | `features-ui.md` |
| Rust control protocol and SQLite schema | `protocol.md` |
| Shared protocol/security terminology | `GLOSSARY.md` |
| Architecture ADR/transport decision | `adr/0001-rust-flutter-webRTC.md`, `adr/0002-transport-spike.md` (libwebrtc selected; engine integration and interoperability pending), `adr/0003-parallel-targets-android-tv-primary.md` |
| Parallel platform tracks, dependencies and milestones | `implementation.md` |
| Release packaging and README guidance | `build-release.md` |
| Threat model and clean-room/legal workflow | `security-legal.md` |
| Verification and benchmark targets | `verification.md` |
| Official sources, current package hints | `sources.md` |
| PlantUML component and pairing diagrams | `diagrams/architecture.puml`, `diagrams/pairing.puml` |

Architecture/design and REA research are planning artifacts. No media implementation, tested installer or verified feature inventory is included.
