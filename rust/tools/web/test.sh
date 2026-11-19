#!/usr/bin/env bash
# Runs `cargo test` for the web target under Node (real filesystem through NODERAWFS).
# Usage: tools/web/test.sh -p zax-nes [cargo test arguments...]
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/env.sh"
cd "$here/../.."
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUNNER="$(cygpath -w "$EMSDK/node/24.19.0_64bit/node.exe")"
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS="$CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS -C link-arg=-sNODERAWFS=1 -C link-arg=-sEXIT_RUNTIME=1"
exec cargo test --release --target wasm32-unknown-emscripten "$@"
