#!/bin/sh
set -eu

# The iOS simulator shell can run without the device-only Rust bridge.
if [ "${PLATFORM_NAME:-}" != "iphoneos" ]; then
  exit 0
fi

repo_root="$(cd "$SRCROOT/../../.." && pwd)"
if command -v cargo >/dev/null 2>&1; then
  cargo_bin="$(command -v cargo)"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
else
  echo "Cargo is required to build the MirrorMesh iOS Rust bridge." >&2
  exit 1
fi

"$cargo_bin" build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package mm-ffi \
  --target aarch64-apple-ios \
  --release --locked

"$cargo_bin" build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package mm-adapter-ios \
  --target aarch64-apple-ios \
  --release --locked

bridge_archive="$repo_root/target/aarch64-apple-ios/release/libmm_ffi.a"
ios_adapter_archive="$repo_root/target/aarch64-apple-ios/release/libmm_adapter_ios.a"
exported_symbols="$repo_root/target/aarch64-apple-ios/release/frb-exported-symbols.txt"
test -f "$bridge_archive"
test -f "$ios_adapter_archive"

# Dart resolves flutter_rust_bridge exports with DynamicLibrary.process().
# Export only those bridge symbols from the app executable.
nm -gU "$bridge_archive" | awk '
  $NF ~ /^_(frb_|frbgen_|store_dart_post_cobject$|mirrormesh_(connection_command|alloc_command|release_command|free_json|discover_devices)$)/ { print $NF }
' | sort -u > "$exported_symbols"

for required_symbol in \
  _frb_get_rust_content_hash \
  _frb_pde_ffi_dispatcher_primary \
  _frb_pde_ffi_dispatcher_sync \
  _store_dart_post_cobject \
  _frbgen_mirrormesh_flutter_rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerBridgeController \
  _frbgen_mirrormesh_flutter_rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerBridgeController
do
  if ! grep -qx "$required_symbol" "$exported_symbols"; then
    echo "Missing required Rust bridge export: $required_symbol" >&2
    exit 1
  fi
done
