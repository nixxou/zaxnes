//! Tests of `zax-engine3d`, ported from `tests/Zax.Engine3D.Tests`. The tests that read the
//! built-in profiles are `#[ignore]`d: set `ZAXNES_PROFILES` and run with `--ignored`.

// Integration-test crate: its helpers (outside `#[test]` functions) fail by unwrapping, which
// `allow-unwrap-in-tests` in clippy.toml only grants inside `#[test]` items.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod cylinder_vector_tests;
mod geometry_vector_tests;
mod instance_and_output_tests;
mod intake_and_frame_tests;
mod registry_tests;
mod scaffold_tests;
mod state_and_mode_tests;
mod support;
mod test_vector_tests;
