# ADR-0012: mDNS discovery adapter and DNS-SD record candidate

- **Status:** Proposed; cross-track and security review pending.
- **Date:** 2026-10-18.
- **Scope:** CORE discovery adapter and proposed MirrorMesh DNS-SD v1 record.

## Context

`DiscoveryPort` is a synchronous shared port. The protocol describes `_mirrormesh._tcp.local` discovery but did not define the capability keys needed to construct the current domain `Device` model. Discovery data is unauthenticated and must not grant access or establish peer trust.

The reviewed implementation candidate is [`mdns-sd 0.21.5`](https://docs.rs/crate/mdns-sd/0.21.5), whose upstream documents safe Rust DNS-SD/mDNS with IPv4/IPv6 and Windows, Linux, and macOS support, and no async-runtime dependency. The upstream README labels the crate beta and dual-licenses it under Apache-2.0 or MIT ([upstream project](https://github.com/keepsimple1/mdns-sd)). The adapter pins the version and disables default features. The current locked dependency graph resolves `mio 1.2.4`, which requires libc 0.2.183 or newer; the workspace libc pin was advanced from 0.2.177 to 0.2.183, the lowest version accepted by that dependency. The graph is checked against the declared Rust 1.85 MSRV.

## Decision candidate

- Browse only `_mirrormesh._tcp.local.` for a bounded two-second window. `stop()` shuts down the active daemon; adapter destruction also shuts it down.
- Accept only exact v1 TXT fields: `v`, `id`, `roles`, `pair`, `tls`, `video`, `audio`, and `max`. Current accepted values include `v=1`, UUID device ID, `pair=qr`, `tls=1`, `video=h264`, `audio=opus|none`, and `max=<width>x<height>@<fps>`. Roles are `send`, `recv`, or canonical `send,recv`. Unknown or missing keys, unknown versions, invalid values, and out-of-range capabilities reject the advertisement.
- Use the DNS-SD instance label as the displayed name and SRV/address data as an endpoint. Accept only the specified signaling port `8765`; non-default ports require a separate explicit approval flow. Drop unspecified, loopback, multicast, and broadcast addresses; retain IPv6 scope information; cap endpoint addresses and discovered services; deduplicate device IDs.
- `DiscoveryPort::discover` remains the device-only compatibility method. `DiscoveryCandidatesPort::discover_candidates` returns `DiscoveryCandidate` values containing the `Device` hint and resolved `SocketAddr` endpoints, preserving IPv6 interface scope. The current app does not assemble either path.
- An advertisement never marks a device trusted. QR proof, verified identity, authenticated signaling, and explicit owner approval remain mandatory. Malformed advertisements are ignored while browsing; port/daemon failures map to `PortError::Unavailable`.

## Consequences and remaining evidence

The DNS-SD record, endpoint-preserving port, and adapter are candidates, not frozen cross-track contracts. The application coordinator filters local IDs, endpointless candidates, and duplicate claimed IDs, then returns candidates ordered by device ID; this does not verify identity or persist trust. The Flutter FFI is unchanged. A matching explicit publisher candidate is described in [ADR-0013](0013-mdns-service-publication-candidate.md). The exact capability fields and shared port contract must receive cross-track/security review before protocol v1 is declared frozen.

Twenty-one parser, service-record, and endpoint-conversion tests exercise valid records, DNS case-insensitivity, invalid fields, bounds, and endpoint rules without opening sockets; two tests cover socket endpoint conversion and IPv6 interface-scope preservation. Three application tests cover local-device filtering, deterministic ordering, endpointless/ambiguous candidates, stop, and adapter-error propagation. Host tests do not establish multicast visibility or LAN interoperability. The current upstream support statement covers Windows, Linux, and macOS; Android and iOS require separate native discovery and permission validation before being represented as supported. Publisher startup is not wired into the app; application startup assembly, Android/iOS adapters, cancellation under a live LAN browse, and owned-LAN testing remain open.

## Sources

- [mdns-sd 0.21.5 crate metadata and release](https://docs.rs/crate/mdns-sd/0.21.5)
- [mdns-sd upstream documentation and compatibility notes](https://github.com/keepsimple1/mdns-sd)
- [ServiceDaemon API](https://docs.rs/mdns-sd/0.21.5/mdns_sd/struct.ServiceDaemon.html)
- [ResolvedService API](https://docs.rs/mdns-sd/0.21.5/mdns_sd/struct.ResolvedService.html)
