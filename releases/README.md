# Personal build outputs

Generated payloads stay local/ignored; source, folder descriptions and public checksum
metadata are kept in Git. Version 0.4.1 is the current personal release checkpoint.

| Output | Producer |
| --- | --- |
| `windows-x64/`, `MirrorMesh-windows-x64.zip` | `scripts/build-windows.ps1` |
| `android-phone/*.apk`, `android-tv/*.apk` | `scripts/build-android.ps1` |
| `linux-x64/`, `MirrorMesh-0.4.1-linux-x64.tar.gz` | `scripts/build-linux-docker.ps1` or `build-linux.sh` |
| `MirrorMesh-0.4.1-source.zip` | `scripts/package-source.ps1` from committed source |
| Intended `macos-universal/` app | `scripts/build-macos.sh` on a Mac; unbuilt here |

Keep complete app folders; the Windows EXE and Linux executable are not standalone.
Debug APKs are locally signed/installable. Optimized unsigned APKs need your private key
before installation/distribution. Windows is unsigned; no Mac/iOS binary is supplied.

Build from clean source, never from an extracted app's private runtime state. Keep licenses,
notices and matching AirPlay source with the generated complete Windows bundle.
Older personal extracted folders remain ignored and untouched.

See [builds](../docs/build-release.md), [TV installation](../docs/install-android-tv.md)
and [actual readiness](../docs/IMPLEMENTATION_STATUS.md).
