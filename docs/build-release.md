# Build desktop and Android apps

## Choose your release product

| Need | Source | Personal-use output |
| --- | --- | --- |
| Windows PC | Flutter Windows host + shared Rust + native AirPlay bridge | Complete portable ZIP, unsigned |
| Android phone/tablet | Native `apps/android-tv`, phone flavor | Locally signed debug APK or owner-signed optimized APK |
| Android TV / Google TV | Same source, TV flavor | TV APK; ARM64 OS required |
| Linux x64 | Flutter GTK host + shared Rust | Ubuntu 24.04-compatible complete tarball |
| macOS desktop | Flutter Swift host + Apple Rust adapter | Intended universal app; Mac build/sign verification still pending |
| iOS app | Swift host + ReplayKit extension + Apple adapter | Intended provisioned app; Mac build/sign verification still pending |

Keep native source directories in place: CMake, Gradle and Xcode resolve the Rust root
relative to their project folders. The canonical Android APK is not a Flutter Android build.

All commands below are local; no GitHub Actions or unit tests are invoked. Run from the
repository root. Build scripts support `main`, release checkouts and extracted source archives;
they do not switch branches, commit or push. Pin dependencies rather than upgrading while building.

## Toolchain

| Tool | Version/configuration used |
| --- | --- |
| Rust | 1.99.1; locked Cargo dependencies; Windows MSVC and Android ARM64 targets |
| Flutter / Dart | 3.47.7 / 3.13.5 |
| Visual Studio | 2022 Build Tools, Desktop development with C++, Windows SDK, CMake |
| Java | JDK 21 (Gradle/AGP require at least 17) |
| Gradle / Android plugin | Wrapper 9.8.1 with SHA-256 pin / AGP 9.4.0 |
| Android SDK | Platform `platforms;android-37.0`, Build Tools 36.0.0 |
| Android NDK | 28.2.13676358 |

Enable Windows Developer Mode for Flutter plugin symlinks. Install the required tools
before building and set your own SDK locations (examples contain no private machine paths):

```powershell
$env:FLUTTER_ROOT = "$env:LOCALAPPDATA\Programs\MirrorMeshFlutter\flutter"
$env:JAVA_HOME = 'C:\path\to\your\jdk'
$env:ANDROID_SDK_ROOT = "$PWD\.toolchains\android-sdk"
```

For a new machine, obtain command-line tools from the [official Android download page](https://developer.android.com/studio),
place them under `<SDK>/cmdline-tools/latest`, then install:

```powershell
& "$env:ANDROID_SDK_ROOT\cmdline-tools\latest\bin\sdkmanager.bat" `
  "--sdk_root=$env:ANDROID_SDK_ROOT" `
  'platform-tools' 'platforms;android-37.0' 'build-tools;36.0.0' 'ndk;28.2.13676358'
rustup target add aarch64-linux-android
```

Review the SDK license prompts during installation. No SDK, private keys or build cache
belongs in Git. [AGP compatibility](https://developer.android.com/build/releases/agp-9-4-0-release-notes)
documents the supported Gradle/JDK/NDK versions.

## Windows portable release

```powershell
.\scripts\build-windows.ps1
```

The script enforces the Flutter lockfile, builds Rust plus the native Flutter runner,
and builds/packages the local AirPlay companion by default. First run
`scripts/setup-airplay-toolchain.ps1`; see [AirPlay prerequisites and lock](airplay.md).
`-WithoutAirPlay` omits that companion for a developer-only native MirrorMesh build.
The bundle includes the EXE, DLLs, Flutter assets and available Visual C++ runtime DLLs:

- `releases/windows-x64/MirrorMesh.exe`
- `releases/windows-x64/SHA256SUMS.txt`
- `releases/MirrorMesh-windows-x64.zip`

Override the SDK path with `-FlutterRoot C:\path\to\flutter`. If no installed Visual C++
runtime redistributable files are found, pass `-VcRuntimeDirectory` or install Microsoft's
VC++ x64 redistributable on the destination PC. The EXE needs its full bundle. Authenticode
signing and an installer/MSIX are separate release operations requiring your certificate.

Direct build/debug commands:

```powershell
Set-Location apps\mirrormesh_flutter
& "$env:FLUTTER_ROOT\bin\flutter.bat" build windows --release
& "$env:FLUTTER_ROOT\bin\flutter.bat" run -d windows
```

## Android phone and TV

```powershell
# Installable, locally debug-signed previews; no test suite is executed.
.\scripts\build-android.ps1 -Configuration debug

# Optimized release APKs, unsigned unless signing variables are supplied.
.\scripts\build-android.ps1 -Configuration release
```

The default ABI is ARM64. `-Abi x86_64` builds that ABI separately. Outputs go to
`releases/android-phone` and `releases/android-tv`; each APK has a `.sha256` sidecar.
Both flavors use the same JNI library, identity/pairing engine and native decoder.
The phone APK includes a receiver and a user-consented MediaProjection sender. Android/TV
sender and new Windows receiver behavior still require device validation.

To sign distribution builds, set these environment variables privately before the release
command: `MM_ANDROID_KEYSTORE` (absolute path), `MM_ANDROID_STORE_PASSWORD`,
`MM_ANDROID_KEY_ALIAS`, and `MM_ANDROID_KEY_PASSWORD`. Keep the same key for updates.
Do not commit passwords or keystores. The build deliberately leaves release APKs unsigned
when no key is provided. Google Play enrollment, upload signing and store review are not
performed by the build script.

Direct Gradle commands (from `apps/android-tv`):

```powershell
.\gradlew.bat :app:assemblePhoneRelease :app:assembleTvRelease --console=plain --no-daemon
# AAB generation for an owner-signed store submission:
.\gradlew.bat :app:bundlePhoneRelease :app:bundleTvRelease --console=plain --no-daemon
```

The `preBuild` dependency compiles `mm-ffi` with the NDK and stages `libmm_ffi.so` automatically.
Do not manually copy an old JNI library into an APK.

## Optional local static checks

```powershell
cargo fmt --all -- --check
cargo check --workspace --lib --locked
cargo clippy --workspace --lib --locked -- -D warnings
# From apps/mirrormesh_flutter:
flutter analyze
```

Compilation cannot establish that real devices render media, meet latency targets or recover
correctly on a specific network. See [verification status](verification.md).

## iOS source build (macOS required)

The new iOS paths have not been compiled on this Windows host. Install Xcode, CocoaPods
(including the xcodeproj Ruby gem), Flutter and the Rust `aarch64-apple-ios` target. From
`apps/mirrormesh_flutter`, run `flutter pub get --enforce-lockfile`, then:

```sh
cd ios
MM_IOS_APP_GROUP=group.your.provisioned.group \
MM_IOS_BROADCAST_ID=your.unique.app.broadcast ruby scripts/configure-broadcast.rb
cd ..
flutter build ios --release
```

In Xcode select your development team, use an extension identifier prefixed with the
Runner app identifier, and provision the same App Group for Runner and MirrorMeshBroadcast.
The configurator creates/embeds the ReplayKit Upload Extension and configures entitlements.
It changes the local Xcode project; review and commit those generated project settings only
on your development branch. Credentials and provisioning profiles stay outside Git. Full-device sharing
appears only when the extension is present; app-only ReplayKit capture remains available.
Check `docs/connections.md` for directions and verification boundaries.

## Linux x64 from Windows / Docker Desktop

Docker Desktop must be running with its Linux engine (WSL2 backend). No extra Ubuntu WSL
distribution is needed. The wrapper builds the pinned Ubuntu 24.04/Flutter 3.47.7/Rust 1.99.1
toolchain, snapshots source roots, then builds in a separate container filesystem:

```powershell
.\scripts\build-linux-docker.ps1
# Reuse an already built toolchain image:
.\scripts\build-linux-docker.ps1 -SkipToolchainBuild
```

Native Linux build, after installing that same Flutter/Rust toolchain plus clang, CMake,
Ninja, pkg-config, GTK3, OpenSSL and Secret Service development libraries:

```sh
sh scripts/build-linux.sh
```

Outputs: `releases/MirrorMesh-0.4.1-linux-x64.tar.gz`, its `.sha256` sidecar, and the complete
`releases/linux-x64/MirrorMesh/` folder. Extract the archive into a new directory and run:

```sh
tar -xzf MirrorMesh-0.4.1-linux-x64.tar.gz
./MirrorMesh/MirrorMesh
```

Keep `lib/` and `data/` next to the executable. This Ubuntu 24.04 x86_64 build needs a compatible
desktop (glibc 2.39 or newer), GTK3, graphics/audio system libraries and a session D-Bus with an
unlocked Secret Service keyring. Headless Docker supplies compilation evidence only; run the
app as your desktop user. Linux Wayland/PipeWire portal capture and individual GPU codecs
still need desktop/device verification. App sharing remains video-only in this release.

## macOS source build (a Mac is required)

There is no macOS binary produced on this Windows/WSL host. On a Mac with the full Xcode
toolchain supported by Flutter 3.47.7 and Rust 1.99.1, use the release source, then:

```sh
sh scripts/build-macos.sh
```

The source targets macOS 13+ and both Apple Silicon / Intel. The script builds the Rust static
bridge for both architectures, retains runtime FFI exports, and packages `MirrorMesh.app`
as an **unsigned preview ZIP**. SwiftPM is used where supported; CocoaPods is required only
when Flutter generates a fallback Podfile. Do not use `flutter build ios` for the Mac desktop app.

For actual use/distribution, open `apps/mirrormesh_flutter/macos/Runner.xcworkspace` on the Mac,
select your team and a unique bundle identifier, and sign with the network entitlements intact.
Data Protection Keychain access requires an appropriately signed/entitled app; unsigned
compilation alone does not prove secure identity storage. Allow local-network access and
explicit screen-recording permission when macOS asks. Signing/notarization and Keychain,
capture, universal linking and actual media behavior remain unverified without that Mac.

## Miracast diagnosis

In Windows MirrorMesh choose **Android wireless display options → Wireless display / Miracast**.
The popup reads Windows receiver status; the separate Wireless Display window owns approval,
video and Stop. Use **Projection settings** to inspect Windows' settings yourself. If Android
sees the PC but fails to connect, discovery works; obtain the exact Windows receiver error.

```powershell
.\scripts\diagnose-miracast.ps1
```

This read-only report collects capability/listening state without private IPs or driver changes.
The current host reported Wi-Fi support but mixed display-driver support; no actual Miracast
handshake repair is verified. Use the MirrorMesh Android app for the owner-confirmed path.
No Google Cast provider is installed: no compatible permanently free Windows provider has
been verified, and the owner declined paid licenses/trials.

## Personal release procedure

1. Use a fresh checkout of `main` or extract the committed source ZIP. Install the pinned
   tools above; retain Cargo/Flutter/Gradle lock/pin files. Set SDK variables to your own locations.
2. Build only the platform you need. Debug Android APKs are sufficient for personal installation.
   For signed optimized Android builds, provide your private signing environment; never commit it.
3. Keep the complete product folder. On another computer, extract to a new folder rather
   than mixing DLLs/assets from old versions. Do not package a working receiver's private state.
4. Verify archive/APK SHA-256 sidecars and payload `SHA256SUMS.txt` where supplied.
5. Install/run locally, approve the required network/capture prompts, and verify a changing
   image on the receiver. Builds and sender counters are not device/media acceptance.
6. Preserve the same Android signing key for updates. Windows certificate signing, Apple
   provisioning/notarization and store submissions are optional separate owner operations.

Build products are ignored locally under `releases/`; source and public checksum metadata
belong in Git. Windows and Android signing credentials are external private inputs.
AirPlay runtime notices/licenses/matching source materials belong in the complete Windows ZIP.

### Minimal source archive

After committing the desired source in a Git checkout:

```powershell
.\scripts\package-source.ps1
```

This packages the committed tree using `.gitattributes` export exclusions, checks common
private-key/token patterns, and writes `releases/MirrorMesh-0.4.1-source.zip` plus `.sha256`.
It excludes generated release folders, SDKs/caches, personal state and maintenance session notes.
It does not include untracked edits or prove that every possible secret format is absent.
Native builds work without `.git`; the Windows Docker wrapper also supports that source ZIP.

### Local debug commands

```powershell
Set-Location apps\mirrormesh_flutter
flutter run -d windows
# On a Linux desktop: flutter run -d linux
# On a Mac with Xcode: flutter run -d macos
```

For Android, build/install the debug flavor and use the SDK's `adb logcat` for relevant
app messages. Replace private device names, endpoints, paths and invitations before sharing
output. The app's Copy diagnostics action provides counters/errors without an invitation.
Use [the README FAQ](../README.md#faq--connection-and-playback) for known symptoms and fixes.
