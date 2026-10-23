//! Tests of `zax-scripting`, ported from `tests/Zax.Scripting.Tests`. The tests that read the
//! built-in profiles are `#[ignore]`d: set `ZAXNES_PROFILES` and run with `--ignored`.

// Integration-test crate: its helpers (outside `#[test]` functions) fail by unwrapping, which
// `allow-unwrap-in-tests` in clippy.toml only grants inside `#[test]` items.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The vectors of SPEC-05 compare floats exactly, as the C# tests do.
#![allow(clippy::float_cmp)]

mod api_behaviour_tests;
mod api_coverage_tests;
mod api_engine_tests;
mod api_value_tests;
mod corpus_tests;
mod lua_access_resolver;
mod lua_runtime_tests;
mod scaffold_tests;
mod smoke_tests;
mod support;
mod test_vector_tests;
