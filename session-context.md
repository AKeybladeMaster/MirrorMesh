# MirrorMesh release checkpoint

## Final repository consolidation - 2026-10-22

- Owner authorized preserving test_6, consolidating on pre_release, and merging/pushing
  pre_release into main as the final repository release for now. Earlier stop is revoked.
  Inspect local/remote Git refs for exact publication commits before future changes.
- Safe original workspace is pushed at test_6 / 4052f40; original implementation c22b9a0.
  Consolidated source commit: 65c17bd. All older refs and published history are preserved.
- Current tree: 319 tracked files; minimal committed source ZIP: 304 build/guide files.
  All 17 Rust crates, canonical native Android and desktop/iOS apps, locks and relative
  native build paths retained. Legacy prototypes/reports remain in the checkpoint history.
  Local archive is ignored; private extracted receiver files are untouched and ignored.
- README has product readiness, generalized FAQ and code tree; docs cover architecture,
  protocol/storage, builds/personal releases and actual verification. No private values
  are intentionally included. Source ZIP stages/validates before public output.
- Final consolidation Windows build/package exit0: pre-release-windows-final.log.
  Android phone/TV debug and unsigned optimized builds exit0. Host Rust format/Clippy,
  Dart format/analysis, PowerShell and POSIX syntax checks pass. No test suites/workflows run.
- Source ZIP generated/extracted successfully. Gitless Linux Docker build from 65c17bd
  compiled, analyzed and packaged, exit0: pre-release-linux-source.log. Output imported
  into local releases. Required source inputs and export exclusions checked.
- Windows artifact: 94,086,239bytes, 285 payload digests verified; SHA256
  24ce2e20b6fe74f05a3a116cbb0955789a71f3e01b74c5e2b3645e174be8363b.
  Linux artifact: 23,681,265bytes, 21 payload digests verified; SHA256
  5eda7b78bf37a1eb7bf59ad1bfc94e86908c9f96713a6d00e2f569f227227a1c.
  All four APK sidecars match. Linux has no unresolved reported libraries or world-writable
  tar members. Packages are local outputs; source and public binary checksum metadata are Git-tracked.
- Source ZIP/sidecar are generated locally from the selected committed tree, excluded
  from Git to avoid circular metadata. Regenerate from main after final publication.
- Read-only consolidation and final packaging reviews found no blockers; minor old links
  corrected. Code/lockfiles are unchanged by final evidence updates.
- Hardware limitations: TV/Linux desktop media unverified; no Mac/Xcode means no Apple
  binaries or Apple build/device claims. Native Miracast failure unresolved; Google Cast
  unavailable. Earlier PC/Android sharing and native iOS connections were owner-confirmed.
- Publication sequence: push pre_release, merge it into main without rewriting history,
  push main, verify both remote refs and preserved older refs. No further implementation
  work is queued; await owner instructions/device feedback after publication.
- No credit gauge is exposed. Keep future checkpoints concise and sanitized.
