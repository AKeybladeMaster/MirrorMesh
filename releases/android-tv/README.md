# Android TV / Google TV receiver

Install `MirrorMesh-tv-arm64-v8a-debug.apk` for a locally signed preview.
The `release-unsigned.apk` requires the owner's signing key before installation.
A hardware H.264 decoder is required. Version 0.4.0 also includes the shared Android
sender, subject to the TV OS offering MediaProjection permission. Upgrade Windows too
for signed device facts. Physical TV sending/receiving validation remains pending.

Build both device flavors: `scripts/build-android.ps1 -Configuration debug`.
