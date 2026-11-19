# Web build (WebAssembly)

Zaxnes itself — the same interface, cores, 3D renderer and Lua — compiled for
`wasm32-unknown-emscripten` and run in a page.

## Build and try

    bash tools/web/build.sh                      # -> target-web/site
    python -m http.server -d target-web/site 8080 --bind 127.0.0.1

then open http://127.0.0.1:8080/. A plain file:// open does not work (the browser will not load
the .wasm from disk). `?env=NAME=value` sets environment variables (ZAXNES_PROFILE=1, ...).

`tools/web/test.sh -p <crate> [args]` runs a crate's tests as wasm under Node.

Needs the Rust target (`rustup target add wasm32-unknown-emscripten`) and emsdk in
`ext-tools/emsdk` (or `EMSDK_DIR`); `tools/web/env.sh` sets everything else.

## Web-specific choices

- SDL2, Lua and ImGui come from Emscripten / their C sources; em++ links (Lua is C++ there) and
  the C/C++ objects use wasm exceptions like Rust's.
- OpenGL ES 3.0 = WebGL2 (glow). WebGL differences handled: the instance uniform block is bound
  whole (`renderer.rs`, `bound_instance_bytes`), fence waits only poll (`gl/api.rs`).
- One thread: the emulation thread is refused and the frame runs on the main thread (existing
  fallback). The browser gets its turn after each swap (Asyncify, `window.rs`).
- RetroAchievements offline (no TLS stack; `fetch` to wire in).

## Still to do

- Main loop through `emscripten_set_main_loop` instead of Asyncify (smaller, faster; Asyncify
  also warns it does not mix with wasm exceptions), swap paced by requestAnimationFrame.
- ROMs and profiles from the page (file input, drag and drop); saves and settings persisted in
  IndexedDB (IDBFS mount of the user data folder).
- Audio unlock on first click, gamepads, fonts with accents, the welcome/keys on the web.
- Achievements through fetch; hide desktop-only options (shaders, MCP, native dialog).
