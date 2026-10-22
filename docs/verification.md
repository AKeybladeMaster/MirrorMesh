# Release verification boundaries

Build success, source completion and hardware behavior are separate. No unit/instrumentation
suites, fuzz targets or GitHub Actions are added or run under the owner's release policy.

## Consolidation checks (2026-10-22)

| Check | Actual result |
| --- | --- |
| Windows x64 complete portable build/package | Exit 0; `.toolchains/pre-release-windows-final.log` |
| Android phone/TV ARM64 debug build/package | Exit 0; `.toolchains/pre-release-android-debug.log` |
| Android phone/TV ARM64 optimized unsigned build/package | Exit 0; `.toolchains/pre-release-android-release.log` |
| Host Rust format and locked workspace/lib Clippy | Exit 0 |
| Dart format check | Exit 0, no changes |
| Flutter analysis | Exit 0, no issues |
| PowerShell build/package script parsing | Exit 0 |
| Local Markdown link targets and targeted source credential scan | Passed; no missing targets or matched credentials |
| Read-only consolidation/packaging review | No blocking findings; minor obsolete document links fixed |
| Linux build from committed source ZIP without `.git` | Pending final archive verification before publication |

All 17 Rust crates, dependency locks, native media/protocol implementations and relative
build roots are retained. Archived prototypes/reports remain in `test_6` / `4052f40`.
Local logs may contain private SDK paths and are ignored, not public release inputs.

## Artifact checks

Each platform build writes SHA-256 metadata. Verify the complete Windows ZIP payload manifest,
Linux tarball manifest/permissions/dependencies, APK sidecars/version metadata and source ZIP
exports. A source archive contains committed build inputs; generated releases and personal
state are excluded. Targeted secret-pattern checks are useful but not exhaustive proof.
The source ZIP is staged privately until its checks succeed, then published locally with a sidecar.

## Device evidence and known limits

- The owner previously confirmed PC/Android app sharing and native iPhone/iPad AirPlay connections.
- TV installation, remote control/navigation, native decoding and TV capture remain unverified.
- Linux desktop keyring, local discovery, capture/Wayland portals, codecs and media remain unverified.
- macOS/iOS source has no Mac compiler/signing/device evidence here; no Apple binary is supplied.
- Native Miracast is unresolved: the phone can remain Connecting without a Windows request.
  Read-only OS status previously reported supported Wi-Fi and inactive NotListening state;
  this does not establish the historical failure's root cause or a repair.
- Google Cast has no compatible provider integrated. No paid provider/trial is installed.
- Latest dialog, cleanup, reconnect, accessibility and performance behavior need device verification.

No new physical-device media run or project-wide CODE_COMPLETE claim is made by consolidation.
Use [readiness](IMPLEMENTATION_STATUS.md), [build commands](build-release.md) and
[the README FAQ](../README.md#faq--connection-and-playback) for current use and limitations.
