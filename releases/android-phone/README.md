# Android phone/tablet receiver

Install `MirrorMesh-phone-arm64-v8a-debug.apk` for a locally signed preview.
The `release-unsigned.apk` requires the owner's signing key before installation.
This native app receives a Windows screen; Android screen sending remains incomplete.

Build both device flavors: `scripts/build-android.ps1 -Configuration debug`.
