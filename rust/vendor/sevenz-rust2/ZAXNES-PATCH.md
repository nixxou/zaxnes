# sevenz-rust2 0.25.0, patched for Zaxnes

Unchanged from crates.io but for one thing: its wasm-bindgen and js-sys dependencies, and the
code using them, apply to `wasm32-unknown-unknown` only (`target_os = "unknown"`) instead of every
`wasm32` target. Zaxnes's web build is `wasm32-unknown-emscripten`, which cannot link
wasm-bindgen (its imports are not provided by Emscripten). Files changed: `Cargo.toml` (the two
target dependency tables), `src/lib.rs` (lines 37 and 77), `src/util.rs` (line 6).

Licence: Apache-2.0 (`LICENSE`). Used through `[patch.crates-io]` in `rust/Cargo.toml`.
