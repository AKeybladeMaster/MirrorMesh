# ADR-0036 — Android TV receiver pairing service

- **Date:** 2026-10-19
- **Status:** Proposed; Android app integration and target review remain open

## Decision

Add `AndroidTvPairingService` to `mm-adapter-android` as a host-testable owner for the native TV
pairing listener. It reuses `PairingRuntime`, `AndroidPairingIdentity`, the QR parser/formatter,
TLS 1.3 raw-public-key pinning, the bounded WSS listener, and its typed approved control channel.
The caller selects the local bind address and the numeric advertised endpoint. The listener port
is the only allowed invitation port.

The app must call the service only after an explicit receiver-pairing action. A verified request
exposes sender ID, display name, fingerprint, source IP, and requested grants, but does not persist
trust. The TV user must approve an accepted subset before the service saves the peer and returns
the live control channel. Rejection sends `pair.reject` and stores no trust. Binding fails if the
pairing runtime, QR identity, and TLS identity do not use the same key.

## Consequences and limits

`accept_request` is blocking and belongs on a worker thread. This slice accepts one request per
call; the native UI/bridge must sequence accept, decision, and the next accept. Cancellation,
displayed QR, user-facing grant controls, and the Android JNI/Gradle bridge remain follow-up work.
The service and its tests do not create a WebRTC peer, transmit video, or verify an Android build.
