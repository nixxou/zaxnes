//! `bench-core` on the public `nestest.nes` of `rust/test-roms/`.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use zax_harness::bench_core::{self, Options};
use zax_harness::HarnessError;

fn arguments(extra: &[&str]) -> Vec<String> {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");
    let mut arguments = vec![rom.display().to_string()];
    arguments.extend(extra.iter().map(|&value| value.to_owned()));
    arguments
}

#[test]
fn the_report_has_one_line_per_stage_and_the_total() {
    let options = Options::parse(&arguments(&["--frames", "5", "--warmup", "2"])).unwrap();
    let report = bench_core::run(&options).unwrap();
    for stage in [
        "core run_frame",
        "tile-record capture",
        "conversion (no Lua)",
        "Lua frame hooks",
        "publish + settings sync",
        "draw list build",
        "2D picture copy",
        "total",
        "tile records per frame",
    ] {
        assert!(report.contains(stage), "no '{stage}' in:\n{report}");
    }
    assert!(report.contains("5 frames after 2 of warm-up"), "{report}");
}

#[test]
fn the_3d_segmentation_can_be_forced() {
    let options = Options::parse(&arguments(&["--frames", "2", "--3d"])).unwrap();
    assert!(bench_core::run(&options)
        .unwrap()
        .contains("3D segmentation"));
}

#[test]
fn a_bad_count_or_an_unknown_option_is_a_usage_error() {
    assert!(matches!(
        Options::parse(&arguments(&["--frames", "many"])),
        Err(HarnessError::Usage(_))
    ));
    assert!(matches!(
        Options::parse(&arguments(&["--fast"])),
        Err(HarnessError::Usage(_))
    ));
    assert!(matches!(Options::parse(&[]), Err(HarnessError::Usage(_))));
}
