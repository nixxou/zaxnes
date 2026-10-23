//! Tests of `zax-profiles`, ported from `tests/Zax.Profiles.Tests`. The tests that read the
//! built-in profiles are `#[ignore]`d: set `ZAXNES_PROFILES` and run with `--ignored`.

// Integration-test crate: its helpers (outside `#[test]` functions) fail by unwrapping, which
// `allow-unwrap-in-tests` in clippy.toml only grants inside `#[test]` items.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod catalogue_and_library_tests;
mod common;
mod lz4_block_tests;
mod profile_corpus_tests;
mod profile_export_tests;
mod profile_round_trip_tests;
mod profile_validation_tests;
mod profile_wire_vector_tests;
mod program_profiles_folder_tests;
mod script_order_tests;
mod setting_application_tests;
mod three_d_sen_import_tests;
mod three_d_sen_real_install_tests;
mod tile_texture_and_tag_tests;
mod unity_bundle_reader_tests;
