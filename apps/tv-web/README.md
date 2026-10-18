# MirrorMesh TV browser shell

This static shell is written in TypeScript and compiles to browser JavaScript with the pinned local compiler. It checks whether the browser has a secure origin, WebRTC, and an exposed H.264 receive capability. It also explains when autoplay is unverified or blocked. It deliberately keeps **Connect disabled** because there is no authenticated signaling service, pairing flow, or WebRTC session implementation in this shell. TypeScript is a build-time dependency only; the page has no runtime package dependencies.

Use Android TV native as the primary television receiver. Treat this page as a fallback compatibility probe only. A physical television must use an HTTPS origin with a trusted certificate; the localhost exception is for local development only. Never accept certificate warnings to make the receiver usable. The page does not request camera or microphone access, and it does not capture or receive media. Autoplay cannot be verified until a media track exists; the TV remote may need to start playback after an approved session.

The page uses native buttons and links for focus semantics. On TV browsers, use the remote arrows to move focus, Select to activate a control, and Back/Escape to exit full screen. The Connect control remains unavailable until all browser checks and authenticated signaling are ready.

## Local preview

From this directory, run `npm ci`, then `npm test`, then `python -m http.server 8080` and open `http://localhost:8080`. Localhost is a browser secure context for capability development. A headless Microsoft Edge 155 run confirmed secure-origin, WebRTC, and H.264 detection and the disabled Connect state. This does not simulate a remote TV deployment or prove that any TV browser supports H.264/WebRTC; the captured desktop preview is in `../../docs/assets/tv-web-shell-edge-155.png`.

## Checks

Run `npm ci` and `npm test` from this directory. `npm test` compiles the TypeScript sources and runs the Node capability tests against the generated JavaScript.
