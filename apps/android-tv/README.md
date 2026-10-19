# Native Android receiver

One Java/WebRTC receiver and Rust JNI runtime produce two flavors:

- `phone`: `com.mirrormesh.receiver`, touch devices, adaptive layout.
- `tv`: `com.mirrormesh.tv`, Leanback launcher and remote navigation.

The receiver authenticates a Windows sender before accepting H.264 media. It uses native
MediaCodec decoding, fullscreen rendering, JPEG snapshots and explicit disconnect/revoke
controls. It does not capture or send the Android screen.

From the repository root, run `scripts/build-android.ps1 -Configuration debug` for locally
installable preview APKs or `-Configuration release` for distribution build output.
See [build instructions](../../docs/build-release.md) and [user instructions](../../docs/USER_GUIDE.md).
No tests or GitHub Actions are included in this branch.
