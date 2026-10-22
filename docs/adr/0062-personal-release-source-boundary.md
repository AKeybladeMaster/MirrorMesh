# ADR0062 — Consolidated personal release source

Date: 2026-10-22. Owner-authorized consolidation through `pre_release` into `main`.

## Decision

Preserve the safe pre-cleanup workspace at `test_6` / `4052f40`. Archive the obsolete
Flutter Android shell, unfinished browser prototype, old candidate ADRs and reports
outside the current release tree. Retain canonical native Android and desktop/iOS
apps, all Rust workspace crates, locked dependencies and native relative build paths.
No media protocol, permissions or trust rules change.

Build scripts no longer require historical test branch names. The native builds work
from a source archive without Git metadata; the Docker wrapper inventories explicit
source roots in that case and excludes caches/private local configuration.

Source ZIPs use committed files plus `.gitattributes` exclusions. Generated releases,
local SDKs, session notes and personal receiver state are excluded. Common private-key
and token patterns are checked in ignored staging before publishing the package; the check is
not exhaustive. A private runtime key found in a personal extracted bundle was excluded
before checkpoint publication. That local app/state is left untouched and ignored.

Binary bundles keep required notices/licenses/matching native companion source; this
cleanup does not strip those distribution inputs. Source-only Apple targets and
unverified TV/Linux media/Miracast behavior remain explicitly distinguished from builds.
No tests, workflows, signing secrets, driver changes or protection bypasses are added.
