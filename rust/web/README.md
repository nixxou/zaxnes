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
- Starts on the fastest preset (owner request; `handheld::defaults::apply_fastest`).
- The user data folder is an IDBFS mount (IndexedDB): settings, saves, states and imported ROMs
  survive a reload; the page syncs every 3 s and when hidden.
- Opening a ROM shows the page's dialogue (bundled ROMs, the player's earlier ROMs, drop, file
  picker), reached by the "Ouvrir une ROM" button or File > Open. Page and program talk through
  files of the in-memory file system (`web_bridge.rs`), so no `unsafe` was needed.
- Touch pad offered. Emscripten's SDL has no virtual joysticks: the pad's buttons are read by
  the application as player one's actions (`TouchPad::take_held`, taps latched for a frame).
  With a real gamepad, the automatic mode keeps the buttons a gamepad lacks (menu, 2D/3D,
  camera lock and recentring, rewind, fast forward).

## Still to do

- Main loop through `emscripten_set_main_loop` instead of Asyncify (smaller, faster; Asyncify
  also warns it does not mix with wasm exceptions), swap paced by requestAnimationFrame.
- Importing 3D profiles from the page (ROMs are done).
- The console list does not rescan the ROM folder after an import (the ROM opens directly).
- Audio unlock on first click, gamepads, fonts with accents, the welcome/keys on the web.
- Achievements through fetch; hide desktop-only options (shaders, MCP, native dialog).
