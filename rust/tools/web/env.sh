# Source this file from Git Bash before building the web version.
EMSDK_DIR="${EMSDK_DIR:-/g/3DSen/ext-tools/emsdk}"
export EMSDK="$EMSDK_DIR"
export EM_CONFIG="$EMSDK_DIR/.emscripten"
export PATH="$EMSDK_DIR/upstream/emscripten:$EMSDK_DIR/upstream/bin:$EMSDK_DIR/node/24.19.0_64bit:$EMSDK_DIR/python/3.13.3_64bit:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target-web}"
export CC_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\emcc.exe'
export CXX_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\em++.exe'
export AR_wasm32_unknown_emscripten='G:\3DSen\ext-tools\emsdk\upstream\emscripten\emar.exe'
# Link options of the web build (SDL2 and WebGL2 from Emscripten's ports).
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS="-C link-arg=-sUSE_SDL=2 -C link-arg=-sMIN_WEBGL_VERSION=2 -C link-arg=-sMAX_WEBGL_VERSION=2 -C link-arg=-sFULL_ES3=1 -C link-arg=-sALLOW_MEMORY_GROWTH=1 -C link-arg=-sSTACK_SIZE=8388608 -C link-arg=-sFORCE_FILESYSTEM=1 -C link-arg=-lidbfs.js -C link-arg=-sEXPORTED_RUNTIME_METHODS=FS,IDBFS,ENV,addRunDependency,removeRunDependency"
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER='G:\3DSen\ext-tools\emsdk\upstream\emscripten\em++.exe'
# Lua is built as C++ (its errors are exceptions): same exception model as Rust's objects.
export CXXFLAGS_wasm32_unknown_emscripten="-fwasm-exceptions"
export CFLAGS_wasm32_unknown_emscripten="-fwasm-exceptions"
