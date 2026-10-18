# TASK 05f — WEB FALLBACK: TV browser receiver (P2-WEB)

**Owner:** WEB; paths `apps/tv-web/`, web deploy and browser test fixtures. Runs concurrently with Android TV native, **not** a replacement. Start UI + feature probes before ADR-0002; real stream uses shared offer/cert model.

**Current scaffold:** a TypeScript capability and remote-focus shell is compiled to JavaScript by the locked local compiler. It keeps Connect disabled until authenticated signaling exists. Seven Node tests pass; no browser/device runtime result exists.

1. Implement browser WebRTC `RTCPeerConnection` receive-only using H.264 capability detection; confirm browsers expose WebRTC and media decode. No assumption that every Smart TV browser has it.
2. Use HTTPS-hosted origin, authenticated WSS broker/session handshake and short-lived device pairing QR. Explicitly document LAN/media topology, HTTPS certificate trust, same-origin/CORS and TV engine quirks.
3. Never instruct users to accept invalid self-signed TLS warnings; any offline-LAN secure-origin variant must be separately evaluated before shipping.
4. Design 10-foot full-screen interface and TV remote focus controls; show precise unsupported message for missing WebRTC/H264, media autoplay restrictions or inaccessible signaling.
5. Test Chrome/Edge browser fixture plus at least one **named physical Smart TV/browser version**. Compare connected sessions with Android TV: time-to-first-frame, buffering, stop/disconnect and snapshot if browser API permits.
6. Deployment package must not contain private keys, passwordless media endpoints or a silent microphone permission request.

**Acceptance:** authenticated `WIN→supported TV browser` fixture with measured codec/latency + clear fallback compatibility documentation. Non-WebRTC TV browsers stay **unsupported**, not automatically handled by Google Cast.
