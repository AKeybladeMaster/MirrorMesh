# End-to-end test entry point

There is no executable end-to-end harness yet. The first required sender-to-native-Android-TV protocol exchange has not been implemented, ADR-0002 still defers transport selection, and the TV app remains a local-only UI shell.

Do not add fake signaling or label the local demos as E2E. Begin this directory's test harness after the [P3 integration readiness gate](../../docs/spikes/integration-readiness.md) passes. Each run must record both build hashes, protocol/fixture revision, device and OS details, network fixture, user-consent and receiver-approval behavior, negative cases, metrics, and direct authorized artifact links.

The current pathway statuses and exact blockers are maintained in the [device matrix](../../docs/device-matrix.md) and [verification ledger](../../docs/verification.md).
