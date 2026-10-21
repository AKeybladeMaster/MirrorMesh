# MirrorMesh test_2 branch operating contract

This branch was explicitly requested for release integration. Work, commit and push only
on `test_2`, created from `test` at the user's request for Android UI/accessibility work;
never update, merge into, or push `test`, `dev`, `main` or any `task_*` branch.

Read `README.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/build-release.md`,
`docs/protocol.md`, `docs/security-legal.md` and `session-context.md` before changes.
The original specification and ADRs remain design references. Historical task plans and
verification suites remain in the unchanged `dev` history.

1. Preserve the shared Rust architecture and native media pipeline. Reuse the existing
   identity, pairing, signaling, trust and session components.
2. Do not add test files, fuzz targets, GitHub Actions or run unit/instrumentation suites.
   The user explicitly requested this branch policy. Use builds, formatters and static
   analysis; report their actual exit codes. Physical verification remains deferred.
3. No raw media frames through Dart/FFI. No silent capture or automatic approval.
4. Keep TLS authentication and QR pinning intact. Do not disable Defender, bypass platform
   security, embed signing secrets or scan arbitrary networks.
5. Use transparent patches, scoped edits and reviewable diffs. Verify paths before deletion.
   Never use Base64 to hide file edits. Report exact commands if Defender blocks an action.
6. Keep dependencies locked. Document meaningful protocol/security/permission changes in an ADR.
7. Keep `CODE_COMPLETE`, `BUILD_VERIFIED` and `HARDWARE_VERIFIED` distinct. A successful build
   is not a functioning-device claim. Do not mark the whole project production complete.
8. Save resumable progress in `session-context.md` periodically and before credit limits.
   Do not claim access to a credit gauge that the environment does not expose.
