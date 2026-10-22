# ADR 0055 — Release integration on the isolated test branch

The user requested a release-focused branch without test sources or GitHub Actions.
`test` starts at `dev` commit 5f3c957. Other branches retain their source and history.

Windows uses the existing Flutter native WebRTC sender, Rust authenticated WSS client,
Credential Manager identity and explicit source selection. Android phone and TV flavors
share one native receive-only H.264 renderer, Rust pairing runtime and trust database.
No account provider or Internet relay is required for the local-network path.

The existing v1 `pair.revoke` message names the peer whose access the author revokes.
It is accepted only on an authenticated channel and only when addressed to the local
device ID. The receiver persists revocation of the authenticated channel's remote peer,
then closes that channel. Storage failures are reported; media still closes.
`session.stop` is sent by either endpoint before local teardown when possible.
Neither message permits modification of an unrelated peer's trust record.

APK signing credentials come from local environment variables. No private signing
material is committed. Unsigned releases are distinct from locally installable previews.
Compilation and static analysis provide build evidence; physical decoding, capture,
performance and store acceptance remain separate, unverified release activities.
