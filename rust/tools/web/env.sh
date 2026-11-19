# Source this file from Git Bash before building the web version.
EMSDK_DIR="${EMSDK_DIR:-/g/3DSen/ext-tools/emsdk}"
export EMSDK="$EMSDK_DIR"
export EM_CONFIG="$EMSDK_DIR/.emscripten"
export PATH="$EMSDK_DIR/upstream/emscripten:$EMSDK_DIR/upstream/bin:$EMSDK_DIR/node/24.19.0_64bit/bin:$EMSDK_DIR/python/3.13.3_64bit:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target-web}"
export CC_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\emcc.exe'
export CXX_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\em++.exe'
export AR_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\emar.exe'
