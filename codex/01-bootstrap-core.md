# TASK 01 — Shared contracts and **parallel six-target** scaffolds (P1)

**MM-PLAN-001:** Start Windows, Linux, Android phone, iOS, Android TV and TV web SDK/mock scaffolds concurrently. CORE owns shared contracts; platform worktrees own their respective shells. A shared stable fixture must precede *real-media* integration, not initial platform research.

**Skills:** domain-modeling, codebase-design, rust-best-practices, TDD, verification-before-completion

## Deliverables

- Rust workspace `crates/mm-domain`, `mm-protocol`, `mm-ports`, `mm-security`, `mm-store`, `mm-runtime`, `mm-ffi` with locked deps, rustfmt/clippy and errors.
- `apps/mirrormesh_flutter` Flutter template for Windows/Linux/Android/iOS, with bridges generated from pinned `flutter_rust_bridge` stable release.
- `tests/protocol-golden/*.json`, `docs/GLOSSARY.md`, and basic CI `core.yml`, `flutter.yml` with per-platform matrix smoke targets.
- Independent `apps/android-tv` native TV receiver shell and `apps/tv-web` HTTPS/browser fallback shell; both use synthetic test media and protocol v1, not duplicated identity logic.

## Codex instructions

1. Read spec/protocol/ADR; initialize Rust workspace with `edition=2024`; avoid adding OS-dependent crates to universal domain. Store versions in lockfiles/toolchain.
2. Implement validated domain types `DeviceId`, `SessionId`, `Role`, `SessionState`, `CapabilitySet`, `CaptureConfig` and explicit `Result` error enums. Derive Serde only for public schema where appropriate.
3. Implement `try_transition(current,event) -> Result<next,DomainError>`. Add tables for normal and invalid transitions; transitions must be idempotent for Stop, and cannot reach Streaming before authenticated permission.
4. Define adapter traits (clock, capture, codec, transport, discovery, store) as minimal interfaces. Create `FakeCapture` with synthetic frame sequence and monotonic time; no real video pointer crosses Dart.
5. Implement protocol envelope max size and discriminated enum parser with `#[serde(deny_unknown_fields)]` as applicable; bound string, sequence and annotation limits explicitly after parsing and before allocation where possible.
6. Add test signed device identity with in-memory fake secure keystore, but **do not** implement custom cryptography; choose vetted cryptographic libraries in reviewed subtask.
7. Add Flutter 4-tab app and Rust event stream for session state only. Widget tests: Connect disabled before pairing, Stop while sharing, unsupported OS explicitly indicated.
8. Create parallel platform bootstrap issues/worktrees and mock CI jobs: WIN, LNX, AND, IOS, TV primary, WEB fallback. IOS compile on macOS runner if available; record blocker otherwise. TV has D-pad/QR mock screen and receives synthetic known sender; WEB tests browser secure-origin/WebRTC capability.
9. Generate `README.md` developer Quickstart with exact tested build commands after first successful build.


## Tests and exit gate

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/mirrormesh_flutter && flutter analyze && flutter test
```

Run actual commands and record exit codes/versions. Do not fake success if Flutter/Android/iOS toolchains unavailable. Keep scoped commits, PR linked M-01/M-06/M-07/M-09/M-10. SDK scaffolds proceed in **all six workstreams immediately**; task 02 media-spike happens in parallel and must produce ADR-0002 before real-media integration.
