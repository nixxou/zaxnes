// The 7z decoder (sevenz-rust2) names the wasm-bindgen crate on every wasm32 target, Emscripten
// included, though its wasm-bindgen code is built only with its `util` feature, which Zaxnes
// leaves off. The crate's own exported helpers still import these: they are never called, and
// are defined here so that the link of the Emscripten build goes through.
addToLibrary({
  __wbindgen_describe: function () {},
  __wbindgen_externref_table_grow: function () { return -1; },
  __wbindgen_externref_table_set_null: function () {},
});
