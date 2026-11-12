#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "$(uname -sm)" != "Linux x86_64" ]]; then
  echo 'Build on x86_64 Linux, matching the Hostinger VPS.' >&2
  exit 1
fi

export CARGO_TARGET_DIR="$root/target"
bundle="$(mktemp -d "$root/deployment/.bundle-XXXXXX")"
trap 'rm -rf "$bundle"' EXIT

cargo build --manifest-path "$root/Cargo.toml" --release --locked -p eins_server
(
  cd "$root/eins_web"
  trunk build --release --locked --dist "$bundle/public"
)
install -m 755 "$CARGO_TARGET_DIR/release/eins_server" "$bundle/eins_server"

# Replace the previous generated bundle only after both builds succeed.
rm -rf "$root/deployment/bundle"
mv "$bundle" "$root/deployment/bundle"
echo "Built $root/deployment/bundle (no deployment performed)."
