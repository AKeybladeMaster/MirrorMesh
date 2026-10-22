# MirrorMesh release maintenance contract

The owner explicitly authorized preserving `test_6`, consolidating on `pre_release`,
then merging and pushing `pre_release` into `main` as the final repository checkpoint
for now. That authorization replaces earlier test_6-only branch restrictions.
Preserve `test_6` checkpoint `4052f40`, `pre_release_v1` and all older refs. Future work
must respect the owner's next branch instructions; do not rewrite published history.

Read README.md, docs/ARCHITECTURE.md, docs/build-release.md,
docs/IMPLEMENTATION_STATUS.md, docs/protocol.md, docs/security-legal.md and
session-context.md before changes.

1. Reuse the shared Rust identity, TLS, pairing, signaling, trust and session engine.
   No raw media frames through Dart/FFI, silent capture or automatic approval.
2. Do not add/run unit or instrumentation suites, fuzz targets or GitHub Actions.
   Builds, formatters, artifact integrity checks and static analysis are permitted.
3. Preserve TLS/QR authentication and platform consent. Never disable Defender,
   bypass platform security, embed signing secrets or scan arbitrary networks.
4. Use transparent patches and scoped changes. Report exact blocked commands.
   Never hide edits in Base64 or alter technique solely to evade protection.
5. Keep dependencies locked. Record meaningful protocol/security/permission changes
   in an ADR. Avoid code moves that break native relative paths or build generation.
6. Distinguish CODE_COMPLETE, BUILD_VERIFIED and HARDWARE_VERIFIED. Do not claim
   native Cast/Miracast success, Apple binaries or production completion without evidence.
7. Keep keys, invitations, credentials, private paths and device/network identities
   out of public documentation and source archives. Runtime-generated receiver state
   is not a release input. Do not touch an owner's running extracted app.
8. Save concise sanitized resumable progress in session-context.md periodically.
   No credit gauge is exposed; do not claim to observe a usage percentage.

Historical reports, prototype sources and old ADRs remain in the test_6 checkpoint.
Local archived material is ignored under .toolchains/release-consolidation-archive.
