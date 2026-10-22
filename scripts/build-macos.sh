#!/bin/sh
# Run on a Mac with Xcode/CocoaPods. This script cannot cross-build on Windows/WSL.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "$(git -C "$root" branch --show-current)" = test_6 ] || { printf 'Use test_6.\n' >&2; exit 1; }
[ "$(uname -s)" = Darwin ] || { printf 'macOS and Xcode are required.\n' >&2; exit 1; }
export CI=true FLUTTER_SUPPRESS_ANALYTICS=true
cd "$root/apps/mirrormesh_flutter"
flutter pub get --enforce-lockfile
flutter analyze
# Produces an unsigned local preview. Owner signing/notarization is a separate step.
flutter build macos --release --no-pub --config-only
if [ -f macos/Podfile ]; then (cd macos && pod install); fi
xcodebuild -workspace macos/Runner.xcworkspace -scheme Runner -configuration Release \
  -derivedDataPath build/macos-preview ARCHS='arm64 x86_64' ONLY_ACTIVE_ARCH=NO CODE_SIGNING_ALLOWED=NO build
version=$(sed -n 's/^version: \([^+]*\).*/\1/p' pubspec.yaml)
output="$root/releases/macos-universal"
mkdir -p "$output"
ditto build/macos-preview/Build/Products/Release/MirrorMesh.app "$output/MirrorMesh.app"
cp "$root/docs/build-release.md" "$output/BUILDING.md"
archive="$root/releases/MirrorMesh-$version-macos-universal.zip"
ditto -c -k --keepParent "$output/MirrorMesh.app" "$archive"
(cd "$root/releases" && shasum -a 256 "$(basename -- "$archive")" > "$(basename -- "$archive").sha256")
printf 'Built unsigned preview %s\n' "$archive"
