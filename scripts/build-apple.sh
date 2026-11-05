#!/bin/zsh
# Build twopassword-ffi for all installed Apple targets, regenerate Swift
# bindings, and assemble twopasswordFFI.xcframework.
#
# iOS targets require a rustup-managed toolchain:
#   brew install rustup && rustup-init -y
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Prefer the rustup proxy (has cross-compiled Apple std libs) over the
# Homebrew rust toolchain, which is host-only.
if [[ -d /opt/homebrew/opt/rustup/bin ]]; then
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
elif [[ -d "$HOME/.cargo/bin" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

BINDINGS="apps/apple/Bindings"
FRAMEWORKS="apps/apple/Frameworks"
mkdir -p "$BINDINGS" "$FRAMEWORKS"

have_std() { [[ -d "$(rustc --print target-libdir --target "$1" 2>/dev/null)" ]]; }

TARGETS=(aarch64-apple-darwin)
for t in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  if have_std "$t"; then
    TARGETS+=("$t")
  else
    echo "skip $t (std not installed — see header comment)"
  fi
done

for t in "${TARGETS[@]}"; do
  echo "== cargo build --release --target $t"
  cargo build --release -p twopassword-ffi --target "$t"
done

echo "== generate Swift bindings"
cargo run -p twopassword-ffi --release --example uniffi-bindgen -- \
  generate --library target/aarch64-apple-darwin/release/libtwopassword_ffi.dylib \
  --language swift --out-dir "$BINDINGS"

echo "== stage framework headers"
# Clang only auto-discovers `module.modulemap`; the generated file is named
# <crate>FFI.modulemap, so restage it under the canonical name.
HDRDIR="$FRAMEWORKS/Headers"
rm -rf "$HDRDIR" && mkdir -p "$HDRDIR"
cp "$BINDINGS/twopassword_ffiFFI.h" "$HDRDIR/"
cp "$BINDINGS/twopassword_ffiFFI.modulemap" "$HDRDIR/module.modulemap"

echo "== assemble xcframework"
# Simulator slices (aarch64-apple-ios-sim + x86_64-apple-ios) must be lipo'd
# into a single library — xcodebuild rejects them as duplicate definitions.
SIM_INPUTS=()
for t in aarch64-apple-ios-sim x86_64-apple-ios; do
  if [[ -f "target/$t/release/libtwopassword_ffi.a" ]]; then
    SIM_INPUTS+=("target/$t/release/libtwopassword_ffi.a")
  fi
done

ARGS=()
for t in "${TARGETS[@]}"; do
  case "$t" in
    aarch64-apple-ios-sim|x86_64-apple-ios) ;; # merged via lipo below
    *) ARGS+=(-library "target/$t/release/libtwopassword_ffi.a" -headers "$HDRDIR") ;;
  esac
done
if ((${#SIM_INPUTS[@]})); then
  mkdir -p target/apple-ios-sim
  lipo -create "${SIM_INPUTS[@]}" -output target/apple-ios-sim/libtwopassword_ffi.a
  ARGS+=(-library target/apple-ios-sim/libtwopassword_ffi.a -headers "$HDRDIR")
fi
rm -rf "$FRAMEWORKS/twopasswordFFI.xcframework"
xcodebuild -create-xcframework "${ARGS[@]}" -output "$FRAMEWORKS/twopasswordFFI.xcframework"

echo "done -> $FRAMEWORKS/twopasswordFFI.xcframework"
