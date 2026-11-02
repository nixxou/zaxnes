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

/// AMD-041 REQ-ARCH-012 amended: `bench-core` runs at **N = 0** unless asked otherwise, so a
/// figure taken before this lot and one taken after it measure the same work. `--run-ahead` sets
/// it, the header reports it, and the restore of step 5 has its own line.
#[test]
fn req_arch_012_the_bench_runs_without_run_ahead_unless_asked() {
    let options = Options::parse(&arguments(&["--frames", "5", "--warmup", "2"])).unwrap();
    let report = bench_core::run(&options).unwrap();
    assert!(report.contains("run-ahead 0"), "{report}");
    assert!(report.contains("run-ahead restore"), "{report}");

    for frames in ["1", "2", "3"] {
        let options =
            Options::parse(&arguments(&["--frames", "5", "--run-ahead", frames])).unwrap();
        let report = bench_core::run(&options).unwrap();
        assert!(
            report.contains(&format!("run-ahead {frames}")),
            "--run-ahead {frames}: {report}"
        );
    }

    // Out of the domain of REQ-LOOP-064.
    assert!(matches!(
        Options::parse(&arguments(&["--run-ahead", "4"])),
        Err(HarnessError::Usage(_))
    ));
    assert!(matches!(
        Options::parse(&arguments(&["--run-ahead", "many"])),
        Err(HarnessError::Usage(_))
    ));
}

/// The emulated work really does grow with N: four frames per real frame at N = 3 take plainly
/// longer than one at N = 0. Measured, not an equality — it is a wall clock.
#[test]
fn the_core_runs_n_plus_one_frames_per_real_frame() {
    let of = |ahead: &str| -> f64 {
        let options =
            Options::parse(&arguments(&["--frames", "120", "--run-ahead", ahead])).unwrap();
        let report = bench_core::run(&options).unwrap();
        report
            .lines()
            .find_map(|line| line.trim().strip_prefix("core run_frame"))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse::<f64>().ok())
            .expect("the core line carries a figure")
    };
    let zero = of("0");
    let three = of("3");
    assert!(
        three > zero * 2.0,
        "N = 3 took {three} ms against {zero} ms at N = 0"
    );
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
