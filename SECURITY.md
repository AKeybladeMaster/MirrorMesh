# Security policy

MirrorMesh is an early prototype and is not suitable for sensitive or production screen sharing. No authenticated streaming path is implemented.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting for this repository if it is enabled. If it is unavailable, do not publish exploit details in a public issue; contact a repository maintainer through a private channel first.

Include the affected revision, component, reproduction steps, and impact. Do not attach real screen recordings, access tokens, private keys, or third-party application binaries.

## Security scope

See [the project threat model and privacy rules](docs/security-legal.md). Pairing, TLS identity verification, capture permission, stream teardown, dependency audits, and signed release artifacts are required before any release can claim a secure media session.
