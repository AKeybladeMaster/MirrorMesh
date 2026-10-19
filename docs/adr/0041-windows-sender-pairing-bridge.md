# ADR-0041 — Windows sender pairing bridge

- **Status:** Proposed
- **Decision date:** 2026-10-19
- **Owners:** Windows + CORE
- **Requirements:** M-06, S-04, M-10

## Context

The shared `WssPairingClient` already validates receiver invitations, pins TLS to the QR fingerprint, signs `pair.begin`, and validates the receiver fingerprint and grant subset in `pair.approve`. Task 39 made its decision wait pollable and cancellable. The Windows shell did not own a worker or expose this flow to the Devices page.

## Decision

Add a Windows adapter-owned pairing worker that accepts bounded pasted invitation text, initializes the existing Windows pairing runtime and identity, and calls the shared WSS client on a background thread. It requests only the `view` grant and connects only to the receiver port already allowed by the Android TV invitation contract. The worker exposes a numeric request ID, bounded status snapshots, explicit cancellation, and shutdown cleanup through narrow Windows C ABI functions and the existing method channel.

The Devices page requires the user to paste the TV invitation and press **Pair with TV**. It shows pending, approved, rejected, failed, cancelling, and cancelled states, displays the verified receiver fingerprint after approval, and offers **Disconnect** to drop the retained `PairingConnection`. Closing the Windows runner requests cancellation and drops an approved connection.

The bridge carries pairing status and public identifiers only. It carries no video frames, capture handles, signing key material, or permission claims. Capture and media remain behind native platform adapters and require separate tasks.

## Consequences and limits

- The implementation reuses the shared QR parser, TLS pinning, proof signer, WSS client, protocol, and grant types; it adds no wire messages or external packages.
- One Windows sender pairing attempt may be active at a time. The UI remains responsive while the WSS client connects and waits for the TV's explicit decision.
- Cancellation is observed immediately during approval polling. A cancellation during the existing bounded TCP/TLS/WebSocket connect may wait for that connect attempt's deadline to expire.
- Approval retains the authenticated control connection but does not establish a WebRTC peer, exchange SDP/ICE, or send media. The UI states this explicitly.
- This candidate does not persist receiver trust on Windows or revoke the peer record that the TV may have stored after approval. Disconnect closes only the sender's control connection.
- Rust and Flutter source checks pass. The Windows Flutter app build is not verified because plugin symlink support is unavailable on the current host; no app-to-app pairing or Android TV APK test ran.

## Follow-up

Consume the approved `PairingConnection` in the shared session/WebRTC adapter and establish a synthetic H.264 sender-to-TV flow before adding Windows Graphics Capture or hardware encoding.
