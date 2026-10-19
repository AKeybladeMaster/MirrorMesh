# ADR-0014: endpoint-preserving discovery application boundary

- **Status:** Proposed; cross-track and security review pending.
- **Date:** 2026-10-18.
- **Scope:** CORE discovery candidate port and application filtering.

## Context

The mDNS adapter parses unauthenticated service records into a `Device` hint and resolved addresses. The existing `DiscoveryPort::discover` returns only devices, so a caller that needs to begin pairing cannot retain the endpoint selected from discovery. The app does not yet assemble discovery or stable local identity.

## Decision candidate

- Add `DiscoveryCandidate` to `mm-ports` with a `Device` hint and resolved `SocketAddr` endpoints. `DiscoveryCandidatesPort` extends the existing device-only `DiscoveryPort` so existing callers remain source compatible.
- The mDNS adapter implements the endpoint-preserving method, fixes each endpoint to signaling port 8765, sorts and deduplicates socket addresses, and carries the mDNS IPv6 interface index into `SocketAddrV6::scope_id`.
- `mm-application::DiscoveryCoordinator` filters the local device ID and endpointless candidates. If multiple services in one scan claim the same non-local ID, it omits all of them as ambiguous. Remaining candidates and endpoints are returned in stable sorted order.
- Discovery results remain untrusted. This use case does not verify a key, save trust, authorize pairing, or start a connection. The existing pairing proof and explicit approval flow remains required.
- The Flutter bridge and user interface are unchanged. App assembly awaits stable local identity and cross-track FFI review.

## Consequences and remaining evidence

The shared Rust caller can retain resolved endpoints through the application boundary, including IPv6 scope information, without introducing a network or runtime dependency. The additive port remains a proposal pending cross-track/security review. Tests use fakes and constructed mDNS values; they do not open sockets or establish LAN behavior. Platform startup, UI selection, native Android/iOS discovery, owned-LAN tests, and authenticated signaling remain open.

## Verification

Three application tests cover self filtering, deterministic ordering, removal of endpointless and duplicate-ID hints, cancellation, and error propagation. Two adapter tests cover normalized IPv4/IPv6 socket endpoint conversion and interface-scope preservation. The full workspace Rust gates and Linux cross-target checks are recorded in [verification](../verification.md). No performance measurement is claimed.
