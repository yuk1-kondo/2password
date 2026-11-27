#!/bin/zsh
# Build the Rust FFI for Android ABIs + Kotlin bindings, then the APK.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# rustup proxy first (has Android std libs)
if [[ -d /opt/homebrew/opt/rustup/bin ]]; then
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
elif [[ -d "$HOME/.cargo/bin" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

have_target() { rustup target list --installed | grep -qx "$1"; }
TARGETS=(aarch64-linux-android x86_64-linux-android)
for t in "${TARGETS[@]}"; do
  have_target "$t" || { echo "missing target $t — rustup target add $t"; exit 1; }
done

ABIS=(arm64-v8a x86_64)
# zsh arrays are 1-indexed; "${!ARR[@]}" is bash-only.
for i in {1..${#TARGETS[@]}}; do
  echo "== cargo build --release --target ${TARGETS[$i]}"
  cargo build --release -p twopassword-ffi --target "${TARGETS[$i]}"
  mkdir -p "apps/android/app/src/main/jniLibs/${ABIS[$i]}"
  cp "target/${TARGETS[$i]}/release/libtwopassword_ffi.so" \
     "apps/android/app/src/main/jniLibs/${ABIS[$i]}/"
done

echo "== generate Kotlin bindings"
cargo run -p twopassword-ffi --release --example uniffi-bindgen -- \
  generate --library target/aarch64-linux-android/release/libtwopassword_ffi.so \
  --language kotlin --out-dir apps/android-bindings --no-format
mkdir -p apps/android/app/src/main/java/uniffi/twopassword_ffi
cp apps/android-bindings/uniffi/twopassword_ffi/twopassword_ffi.kt \
   apps/android/app/src/main/java/uniffi/twopassword_ffi/

echo "== assembleDebug"
(cd apps/android && ./gradlew assembleDebug)
echo "done -> apps/android/app/build/outputs/apk/debug/app-debug.apk"
