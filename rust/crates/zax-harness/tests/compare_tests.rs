//! `compare` on small PNGs written by the tests.

// A test: a failed write or an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use zax_harness::compare::{self, Options};
use zax_harness::HarnessError;
use zax_render::assets::png_image;

fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("zax-compare-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

/// A 4 x 4 picture of one grey, with the first `changed` pixels `shift` levels brighter.
fn write(folder: &Path, name: &str, size: u32, shift: u8, changed: usize) {
    let mut pixels = Vec::new();
    for index in 0..(size * size) as usize {
        let level = 100 + if index < changed { shift } else { 0 };
        pixels.extend_from_slice(&[level, level, level, 255]);
    }
    fs::write(
        folder.join(name),
        png_image::encode_rgba(size, size, &pixels).unwrap(),
    )
    .unwrap();
}

fn options(reference: &Path, capture: &Path, extra: &[&str]) -> Options {
    let mut arguments = vec![
        reference.display().to_string(),
        capture.display().to_string(),
    ];
    arguments.extend(extra.iter().map(|&value| value.to_owned()));
    Options::parse(&arguments).unwrap()
}

#[test]
fn matching_share_counts_the_pixels_within_the_tolerance() {
    let reference = [10, 10, 10, 255, 10, 10, 10, 255];
    let capture = [12, 8, 10, 255, 13, 10, 10, 255];
    assert!((compare::matching_share(&capture, &reference, 2) - 0.5).abs() < 1e-12);
    assert!((compare::matching_share(&capture, &reference, 3) - 1.0).abs() < 1e-12);
}

#[test]
fn identical_folders_match_entirely() {
    let (a, b) = (folder("same-a"), folder("same-b"));
    write(&a, "one.png", 4, 0, 0);
    write(&b, "one.png", 4, 0, 0);
    let report = compare::run(&options(&a, &b, &[])).unwrap();
    assert!(report.contains("one.png"), "{report}");
    assert!(report.contains("100.000 % within 2 levels"), "{report}");
    assert!(report.ends_with("1 picture(s), 0 off (tolerance 2, minimum 99.5 %)"));
}

#[test]
fn a_difference_inside_the_tolerance_passes_and_one_outside_fails() {
    let (a, b) = (folder("tol-a"), folder("tol-b"));
    write(&a, "x.png", 4, 0, 0);
    write(&b, "x.png", 4, 2, 16);
    assert!(compare::run(&options(&a, &b, &[])).is_ok());

    write(&b, "x.png", 4, 3, 8);
    let Err(HarnessError::Mismatch(report)) = compare::run(&options(&a, &b, &[])) else {
        panic!("a 50 % match must fail");
    };
    assert!(
        report.contains("50.000 %") && report.contains("FAIL"),
        "{report}"
    );
    assert!(compare::run(&options(&a, &b, &["--min", "50"])).is_ok());
    assert!(compare::run(&options(&a, &b, &["--tolerance", "3"])).is_ok());
}

#[test]
fn a_missing_or_resized_capture_fails() {
    let (a, b) = (folder("miss-a"), folder("miss-b"));
    write(&a, "gone.png", 4, 0, 0);
    write(&a, "big.png", 4, 0, 0);
    write(&b, "big.png", 8, 0, 0);
    let Err(HarnessError::Mismatch(report)) = compare::run(&options(&a, &b, &[])) else {
        panic!("must fail");
    };
    assert!(
        report.contains("MISSING") && report.contains("SIZE DIFFERS"),
        "{report}"
    );
}

#[test]
fn files_compare_directly_and_bad_arguments_are_refused() {
    let (a, b) = (folder("file-a"), folder("file-b"));
    write(&a, "p.png", 4, 0, 0);
    write(&b, "p.png", 4, 0, 0);
    assert!(compare::run(&options(&a.join("p.png"), &b.join("p.png"), &[])).is_ok());

    let parse =
        |values: &[&str]| Options::parse(&values.iter().map(|&v| v.to_owned()).collect::<Vec<_>>());
    assert!(parse(&["only-one"]).is_err());
    assert!(parse(&["a", "b", "--min", "101"]).is_err());
    assert!(parse(&["a", "b", "--tolerance", "x"]).is_err());
    assert!(parse(&["a", "b", "--what"]).is_err());
}
