# MirrorMesh glossary

- **Adapter:** platform-specific code that implements a shared port using Windows, Linux, Android, iOS, or browser APIs.
- **Capture consent:** explicit operating-system approval for a user-selected screen or window source. It is separate from pairing approval.
- **Capability:** a reported feature or limit, such as H.264 support or maximum dimensions. A capability does not grant permission.
- **Control message:** bounded JSON used for discovery, pairing, session setup, and diagnostics. It does not contain video or audio frames.
- **FFI:** the narrow native boundary between the Rust control plane and Flutter. The current contract carries session state; raw media buffers must not cross it.
- **Grant:** a peer permission scoped to one or more of `view`, `send`, and `annotate`, with expiry and revocation behavior.
- **ICE:** Interactive Connectivity Establishment, the candidate negotiation used by WebRTC to find a media path.
- **Native texture/surface:** a platform-owned GPU or codec output handle used to render media without copying frames through Dart.
- **Pairing approval:** a user decision to trust a peer identity and requested grant set. It does not itself start capture.
- **Port:** a platform-neutral interface for capture, codecs, transport, discovery, time, or storage.
- **Session:** one authorized connection between peers, with explicit negotiation, streaming, stop, and cleanup states.
- **Synthetic fixture:** deterministic test-only input used to exercise state, timing, or UI without accessing a user's display or microphone.
- **DTLS-SRTP:** WebRTC's standard key agreement and encrypted media transport. MirrorMesh does not define custom media encryption.
- **WSS:** WebSocket signaling protected by TLS. The server and peer identity still require authentication and authorization.
