# ADR-0013: opt-in mDNS service publication candidate

- **Status:** Proposed; cross-track and security review pending.
- **Date:** 2026-10-18.
- **Scope:** CORE mDNS service-record construction and publisher lifecycle.

## Context

ADR-0012 defines a candidate DNS-SD record and a bounded browser. The adapter also needs to construct the same record when a receiver explicitly becomes discoverable. The current app does not yet assemble a stable local pairing identity or discovery runtime, so the publisher must remain an explicit adapter API and must not run during app startup.

## Decision candidate

- `MdnsServicePublisher::publish` accepts a caller-supplied `Device`, `.local.` hostname, and explicit IP addresses. It does not generate or persist a device identity.
- Build exactly the candidate v1 TXT fields from ADR-0012: protocol version, device ID, roles, QR/TLS requirements, H.264/audio support, and maximum dimensions/frame rate. The signaling port is fixed at `8765`. No keys, fingerprints, credentials, SDP, or IP values are added to TXT data.
- Accept only non-loopback, non-multicast private or link-local IPv4 and unique-local or link-local IPv6 addresses. Sort, deduplicate, and cap the address list at 16. Names and H.264 capabilities are validated before creating a daemon.
- Publishing is opt-in through an explicit call. `stop()` requests graceful daemon shutdown and waits up to two seconds; dropping the publisher requests shutdown without blocking.
- A successful `publish` return means the daemon accepted the registration command. It does not prove that the OS opened multicast sockets or that a peer can see the service. Authenticated TLS/WSS signaling, QR proof, and explicit receiver approval remain required before pairing or capture.

## Consequences and remaining evidence

Twenty-one host unit tests now cover strict parsing, service-record construction, and endpoint conversion, including local-address restrictions, capability bounds, address limits, and inactive-stop idempotence. They do not start the mDNS daemon or send packets. The publisher is not wired into app startup or a user-facing discoverability control; stable identity/runtime assembly and cross-track FFI review are still required. Android and iOS need separate native discovery/permission adapters. Owned-LAN interoperability remains unverified.

## Sources

- [`mdns-sd 0.21.5` crate metadata](https://docs.rs/crate/mdns-sd/0.21.5)
- [`ServiceInfo` API](https://docs.rs/mdns-sd/0.21.5/mdns_sd/struct.ServiceInfo.html)
- [`ServiceDaemon` API](https://docs.rs/mdns-sd/0.21.5/mdns_sd/struct.ServiceDaemon.html)
