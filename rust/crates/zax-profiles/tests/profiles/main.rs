//! Tests of `zax-profiles`, ported from `tests/Zax.Profiles.Tests`. The tests that read the
//! built-in profiles are `#[ignore]`d: set `ZAXNES_PROFILES` and run with `--ignored`.

mod common;
mod lz4_block_tests;
mod profile_export_tests;
mod profile_validation_tests;
mod script_order_tests;
mod setting_application_tests;
mod tile_texture_and_tag_tests;
mod unity_bundle_reader_tests;
