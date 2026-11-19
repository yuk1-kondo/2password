#!/bin/zsh
# Install the 2pw-nmh native-messaging host manifest for Chrome/Firefox.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXTENSION_ID="${1:-}"
if [[ ! "$EXTENSION_ID" =~ ^[a-p]{32}$ ]]; then
  echo "Usage: $0 <Chrome extension ID (32 letters a-p)>" >&2
  echo "Load apps/extension at chrome://extensions and copy its ID." >&2
  exit 1
fi
BIN="$ROOT/target/release/2pw-nmh"
cargo build --release -p twopassword-nmh
[[ -x "$BIN" ]] || { echo "host binary missing: $BIN"; exit 1; }

write_manifest() {
  local dir="$1" extra="$2"
  mkdir -p "$dir"
  cat > "$dir/dev.twopassword.native.json" <<JSON
{
  "name": "dev.twopassword.native",
  "description": "2Password native messaging host",
  "path": "$BIN",
  "type": "stdio",
$extra
}
JSON
  echo "installed -> $dir/dev.twopassword.native.json"
}

# Chrome / Chromium / Edge (macOS)
for base in \
  "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts" \
  "$HOME/Library/Application Support/Chromium/NativeMessagingHosts" \
  "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts"; do
  write_manifest "$base" "  \"allowed_origins\": [\"chrome-extension://$EXTENSION_ID/\"]"
done

# Firefox
write_manifest \
  "$HOME/Library/Application Support/Mozilla/NativeMessagingHosts" \
  '  "allowed_extensions": ["2password@localhost"]'
