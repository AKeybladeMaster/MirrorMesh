# Build Windows and Android

All commands below are local; no GitHub Actions or unit tests are invoked. Run from the
repository root on the authorized release branch: finish the AirPlay startup repair on
`test_3`, then use `test_4` after the requested handoff. Preserve every other branch.

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
before building. The current machine's SDKs are at:

```powershell
$env:FLUTTER_ROOT = "$env:LOCALAPPDATA\Programs\MirrorMeshFlutter\flutter"
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-21.0.10'
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
on `test_3`. Credentials and provisioning profiles stay outside Git. Full-device sharing
appears only when the extension is present; app-only ReplayKit capture remains available.
Check `docs/connections.md` for directions and verification boundaries.
