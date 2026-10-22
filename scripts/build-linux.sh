#!/bin/sh
# Native Linux build; no app launch, test suites or upload.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] || { printf 'Linux x86_64 is required.\n' >&2; exit 1; }
export CI=true FLUTTER_SUPPRESS_ANALYTICS=true RUSTUP_TOOLCHAIN=1.99.1
cd "$root"
cargo clippy --workspace --lib --locked -- -D warnings
cd "$root/apps/mirrormesh_flutter"
flutter pub get --enforce-lockfile
flutter analyze
flutter build linux --release --no-pub
cd "$root"
version=$(sed -n 's/^version: \([^+]*\).*/\1/p' apps/mirrormesh_flutter/pubspec.yaml)
output="${MM_RELEASE_OUTPUT:-$root/releases}/linux-x64"
mkdir -p "$output/MirrorMesh"
cp -R apps/mirrormesh_flutter/build/linux/x64/release/bundle/. "$output/MirrorMesh/"
cp docs/USER_GUIDE.md docs/connections.md docs/install-android-tv.md docs/build-release.md docs/THIRD-PARTY-NOTICES-rust.txt \
  docs/THIRD-PARTY-NOTICES-flutter.txt "$output/MirrorMesh/"
ldd "$output/MirrorMesh/MirrorMesh" > "$output/DEPENDENCIES.txt"
ldd "$output/MirrorMesh/lib/libmm_ffi.so" >> "$output/DEPENDENCIES.txt"
if grep -q 'not found' "$output/DEPENDENCIES.txt"; then cat "$output/DEPENDENCIES.txt" >&2; exit 1; fi
# The bundle must not depend on an absolute Rust build location.
if readelf -d "$output/MirrorMesh/MirrorMesh" | grep NEEDED | grep -q '/'; then
  printf 'The executable contains an absolute library dependency.\n' >&2; exit 1
fi
(cd "$output/MirrorMesh" && find . -type f ! -name SHA256SUMS.txt -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS.txt)
archive="${MM_RELEASE_OUTPUT:-$root/releases}/MirrorMesh-$version-linux-x64.tar.gz"
tar -C "$output" -czf "$archive" MirrorMesh
(cd "$(dirname -- "$archive")" && sha256sum "$(basename -- "$archive")" > "$(basename -- "$archive").sha256")
printf 'Built %s\n' "$archive"
