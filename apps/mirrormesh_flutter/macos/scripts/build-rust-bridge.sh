#!/bin/sh
set -eu
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN=1.99.1
root=$(CDPATH= cd -- "$PROJECT_DIR/../../.." && pwd)
out="$root/target/macos-universal/release"
mkdir -p "$out"
set --
for architecture in ${ARCHS:-$(uname -m)}; do
  case "$architecture" in
    arm64) target=aarch64-apple-darwin ;;
    x86_64) target=x86_64-apple-darwin ;;
    *) printf 'Unsupported macOS architecture: %s\n' "$architecture" >&2; exit 1 ;;
  esac
  rustup target add "$target"
  # Swift provides the Keychain callbacks at the final app link. Do not also link a
  # standalone cdylib, whose link cannot resolve those app-owned callbacks.
  cargo rustc --lib --crate-type staticlib --manifest-path "$root/Cargo.toml" --package mm-ffi --target "$target" --release --locked
  set -- "$@" "$root/target/$target/release/libmm_ffi.a"
done
if [ "$#" -eq 1 ]; then cp "$1" "$out/libmm_ffi.a"; else lipo -create "$@" -output "$out/libmm_ffi.a"; fi
# Dart resolves these symbols at runtime. Keep and export them despite dead stripping.
nm -gU "$out/libmm_ffi.a" | awk '
  $NF ~ /^_(frb_|frbgen_|store_dart_post_cobject$|mirrormesh_(connection_command|alloc_command|release_command|free_json|discover_devices)$)/ { print $NF }
' | sort -u > "$out/frb-exported-symbols.txt"
for symbol in _mirrormesh_connection_command _mirrormesh_alloc_command \
  _mirrormesh_release_command _mirrormesh_free_json _mirrormesh_discover_devices \
  _frb_get_rust_content_hash _frb_pde_ffi_dispatcher_primary \
  _frb_pde_ffi_dispatcher_sync _store_dart_post_cobject; do
  grep -qx "$symbol" "$out/frb-exported-symbols.txt" || { printf 'Missing Rust export: %s\n' "$symbol" >&2; exit 1; }
done
