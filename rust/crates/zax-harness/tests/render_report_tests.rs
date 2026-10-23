//! `render --explain` and `render --bench` on the synthetic scenes (the offscreen context is
//! whatever EGL driver the machine has, Mesa llvmpipe in CI).

// A test: a failed render or a malformed report should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use zax_engine3d::settings::RenderAlgorithm;
use zax_harness::render::{self, scenes, Options};

fn output(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("zax-harness-{name}-{}", std::process::id()))
}

fn run(name: &str, values: &[&str]) -> (String, Vec<Value>) {
    let out = output(name);
    let mut arguments = vec!["--out".to_owned(), out.display().to_string()];
    arguments.extend(values.iter().map(|&value| value.to_owned()));
    let text = render::run(&Options::parse(&arguments).unwrap()).unwrap();
    let json = fs::read_to_string(out.join("render.json")).unwrap();
    (text, serde_json::from_str(&json).unwrap())
}

#[test]
fn explain_prints_the_camera_and_the_draw_list_groups_of_a_synthetic_scene() {
    let (text, _) = run("explain", &["--frames", "synthetic:framing", "--explain"]);

    let tiles = scenes::build("framing", RenderAlgorithm::Greedy)
        .unwrap()
        .draw_list
        .tiles()
        .len();
    assert!(tiles > 0);
    assert!(text.starts_with("camera: fov 70.0  d "), "{text}");
    assert!(text.contains("clipping: ("), "{text}");
    assert!(text.contains(&format!("\n{tiles} tile(s), ")), "{text}");

    // Each group has its key line, with the texture and the colours, then its placement line.
    let key_lines: Vec<&str> = text
        .lines()
        .filter(|line| line.trim_start().starts_with('p') && line.contains(" tiles  tex "))
        .collect();
    assert!(!key_lines.is_empty(), "{text}");
    for line in key_lines {
        assert!(line.contains(" bg=") && line.contains(" pal="), "{line}");
        assert!(line.contains("mat=") && line.contains("layer="), "{line}");
    }
    assert!(
        text.contains("depth ") && text.contains("drawn z ["),
        "{text}"
    );

    // The groups are listed from the largest down.
    let counts: Vec<usize> = text
        .lines()
        .filter_map(|line| line.split(" tiles  tex ").next()?.split_whitespace().last())
        .filter_map(|count| count.parse().ok())
        .collect();
    assert!(
        counts.windows(2).all(|pair| pair[0] >= pair[1]),
        "{counts:?}"
    );
}

#[test]
fn explain_is_off_by_default() {
    let (text, _) = run("explain-off", &["--frames", "synthetic:framing"]);
    assert!(
        !text.contains("camera:") && !text.contains("group(s)"),
        "{text}"
    );
}

#[test]
fn bench_reports_the_frame_times_of_each_scene() {
    let (text, entries) = run("bench", &["--frames", "synthetic:framing", "--bench", "5"]);

    assert!(text.contains(" ms median, "), "{text}");
    assert_eq!(entries.len(), 1);
    let median = entries[0]["medianFrameMs"].as_f64().unwrap();
    let p99 = entries[0]["p99FrameMs"].as_f64().unwrap();
    assert!(median > 0.0 && p99 >= median, "{median} {p99}");
}

#[test]
fn no_bench_leaves_the_report_without_times() {
    let (text, entries) = run("no-bench", &["--frames", "synthetic:framing"]);
    assert!(!text.contains(" ms median"), "{text}");
    assert!(entries[0].get("medianFrameMs").is_none());
}

#[test]
fn bench_takes_a_count() {
    let arguments = |values: &[&str]| -> Vec<String> {
        ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&v| v.to_owned())
            .collect()
    };
    assert_eq!(
        Options::parse(&arguments(&["--bench", "30"]))
            .unwrap()
            .bench,
        30
    );
    assert!(Options::parse(&arguments(&["--bench", "many"])).is_err());
    assert!(Options::parse(&arguments(&["--explain"])).unwrap().explain);
}
