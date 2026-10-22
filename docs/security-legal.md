# Identity, permissions and private release inputs

## Identity and trust storage

| Platform | Private identity storage |
| --- | --- |
| Windows | Windows Credential Manager adapter |
| Android | Keystore AES-GCM wrapping key; encrypted seed in app-private no-backup storage |
| Linux | Secret Service on the user's desktop session; fail closed if unavailable/locked |
| iOS / macOS source | Non-synchronizable unlocked-device Keychain item; desktop-private service namespace |

The seed can enter the app process for signing; this is not a claim of non-exportable
hardware attestation. The SQLite peer database holds public trust/grants and revocation,
not a plaintext private identity seed. Apple storage/signing behavior remains unverified
until the source is built and exercised on appropriately signed Apple targets.

## Consent and transport boundaries

- Invitation/QR identity pinning, signed pairing proofs, expiry/nonces, bounded parsing
  and source admission remain enabled. Discovery results alone are not trusted.
- Incoming requests require explicit grants/approval. OS screen-capture prompts remain
  required; no silent capture, automatic approval or protected-content bypass is added.
- Native WebRTC carries encrypted media. No raw media buffers cross Dart/FFI.
- Stop/reject/revoke and bounded failures release owned resources. Revocation persistence
  failures are reported instead of presented as success.
- Reported device facts are not attestation. Observed incoming source addresses may appear
  in confirmation, but local endpoints are hidden during ordinary setup.
- No cloud account, automatic upload, arbitrary subnet scanning or TURN relay is configured.

## Keep out of Git and source archives

Private keys, pairing invitations/PINs, runtime receiver identity files, app trust databases,
keystores, provisioning profiles, passwords, local SDK paths and diagnostic personal data
are not build inputs. An extracted receiver can create private runtime state; never copy
its whole working directory into public source or use it as a clean packaging input.

The release consolidation found a private runtime key in a personal extracted bundle.
It was excluded before the checkpoint push and left untouched locally. The final release
tree ignores that personal bundle entirely. No key contents or private machine identifiers
are published in the guides or FAQ. Existing Git history is preserved, not rewritten.

Signing uses owner-supplied environment variables/OS tooling; credentials remain outside
the repository. The generated source ZIP uses committed source with release/state folders
excluded and a private-key/token check. This check is a targeted safeguard, not proof that
all possible secret formats can be detected.

## Native receiver and distribution boundary

Windows AirPlay is a separately built pinned UxPlay/GStreamer companion. Keep its notices,
licenses and matching source materials in the complete binary bundle; the packaging
scripts assemble them. Retain Rust/Flutter third-party notices as well. Removing SDK caches
from Git does not remove required redistribution materials from the built product.

Miracast remains Microsoft's OS-owned receiver; MirrorMesh does not terminate or replace
its consent engine. No licensed Google Cast provider is installed or redistributed.
Do not disable Defender/firewalls, add exclusions, obfuscate edits or bypass platform
signing/authentication to work around a failed build or connection. Report exact blocked
commands and inspect partial writes before applying an ordinary readable patch.

Review each bundled component's included license/source terms before wider distribution.
Personal signing, Apple notarization, store enrollment and hardware acceptance are separate
from a successful compiler/package build.
