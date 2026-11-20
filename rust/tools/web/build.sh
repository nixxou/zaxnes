#!/usr/bin/env bash
# Builds the web version into target-web/site (index.html, zaxnes.js, zaxnes.wasm).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/env.sh"
cd "$here/../.."
cargo build -p zax-app --bin zaxnes --release --target wasm32-unknown-emscripten --no-default-features
out="$CARGO_TARGET_DIR/wasm32-unknown-emscripten/release"
site="$CARGO_TARGET_DIR/site"
mkdir -p "$site"
cp web/index.html "$out/zaxnes.js" "$out/zaxnes.wasm" "$site/"
mkdir -p "$site/fonts"
cp web/fonts/* "$site/fonts/"
echo "site: $site  (serve with: python -m http.server -d \"$site\" 8080)"
