# Android phone/tablet receiver

Install `MirrorMesh-phone-arm64-v8a-debug.apk` for a locally signed preview.
The `release-unsigned.apk` requires the owner's signing key before installation.
Version 0.4.0 receives a Windows screen and sends an Android screen using explicit
MediaProjection consent. Upgrade Windows too for signed device facts. New dialogs and
automatic network selection remain device verification pending.

Build both device flavors: `scripts/build-android.ps1 -Configuration debug`.
