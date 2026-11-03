//! `render --explain` and `render --bench` on the synthetic scenes (the offscreen context is
//! whatever EGL driver the machine has, Mesa llvmpipe in CI).

// A test: a failed render or a malformed report should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use zax_engine3d::settings::RenderAlgorithm;
use zax_harness::render::{self, scenes, Options};
use zax_render::style::{AmbientOcclusion, StyleSettings};

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

/// AMD-049 REQ-SCN-075: `--no-backdrop-lighting` is the flag that reproduces the look of before
/// the amendment, which is what the goldens were captured in and what R0 compares against. It is
/// on by default, as the desktop setting is.
#[test]
fn backdrop_lighting_is_on_unless_the_flag_turns_it_off() {
    let arguments = |values: &[&str]| -> Vec<String> {
        ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&v| v.to_owned())
            .collect()
    };
    assert!(
        Options::parse(&arguments(&[])).unwrap().backdrop_lighting,
        "on by default"
    );
    assert!(
        !Options::parse(&arguments(&["--no-backdrop-lighting"]))
            .unwrap()
            .backdrop_lighting
    );

    // And it reaches the picture: the backdrop scene of REQ-SCN-048 has a black box, which the
    // diffuse model of REQ-SCN-028 draws black and the model of REQ-SCN-075 draws sRGB 26. The
    // colour count of that scene is the cheapest way to see the difference in the report.
    let (_, lit) = run("backdrop-lit", &["--frames", "synthetic:backdrop"]);
    let (_, plain) = run(
        "backdrop-plain",
        &["--frames", "synthetic:backdrop", "--no-backdrop-lighting"],
    );
    let colours = |scenes: &[Value]| -> u64 {
        scenes[0]["distinctColors"]
            .as_u64()
            .expect("a colour count")
    };
    assert!(
        colours(&lit) > colours(&plain),
        "the model adds colours to the backdrop scene: {} against {}",
        colours(&lit),
        colours(&plain)
    );
}

/// AMD-055 REQ-SCN-078: `--style` is Classic by default — the setting's own default, so a capture
/// taken without it is the capture of before the amendment — and takes both names and the three
/// numbers the requirement admits, 1 among them.
#[test]
fn req_scn_078_the_style_flag_is_classic_by_default_and_takes_both_names() {
    let parse = |values: &[&str]| -> Options {
        let arguments: Vec<String> = ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&value| value.to_owned())
            .collect();
        Options::parse(&arguments).unwrap()
    };

    assert!(
        !parse(&[]).style_settings().enhanced,
        "Classic by default, as the setting is"
    );
    assert!(!parse(&["--style", "classic"]).style_settings().enhanced);
    assert!(!parse(&["--style", "0"]).style_settings().enhanced);
    for value in ["enhanced", "Enhanced", "ENHANCED", "4", "1"] {
        let style = parse(&["--style", value]).style_settings();
        assert!(style.enhanced, "--style {value}");
        // REQ-SCN-078: the accepted 1 reads back as 4.
        assert_eq!(4, style.identifier(), "--style {value}");
    }

    // A misspelt flag is an error, not the silent Classic the setting falls back to: a capture
    // campaign must not quietly produce the wrong picture.
    for value in ["", "Enhance", "2", "7", "-1"] {
        let arguments = ["--out", "x", "--style", value].map(str::to_owned);
        assert!(Options::parse(&arguments).is_err(), "--style {value:?}");
    }
}

/// AMD-055 REQ-SCN-079: `--style-param` names the twelve parameters by the Lua identifiers the
/// amendment makes normative, and each value goes through the setting's **own** normalisation —
/// the criterion's own examples, `ao.strength` 9 → 5, `spl.voxel_grid` 4 → 3, `spl.ao` 1,4 → 1.
#[test]
fn req_scn_079_the_style_parameters_are_named_and_normalised() {
    let parse = |values: &[&str]| -> Options {
        let arguments: Vec<String> = ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&value| value.to_owned())
            .collect();
        Options::parse(&arguments).unwrap()
    };

    let style = parse(&[
        "--style",
        "enhanced",
        "--style-param",
        "spl.ao=1.4,ao.strength=9,ao.radius=1,ao.thickness=5,ao.quality=2",
        "--style-param",
        "spl.pixel_shadow=1,spl.voxel_grid=4,spl.reflections=1",
        "--style-param",
        "ssr.shine=5,ssr.clarity=1,ssr.length=0,ssr.quality=3",
    ])
    .style_settings();

    assert_eq!(AmbientOcclusion::Msvo, style.ao, "1,4 snaps to 1");
    assert_eq!(5, style.ao_strength, "9 is bounded to 5");
    assert_eq!(1, style.ao_radius);
    assert_eq!(5, style.ao_thickness);
    assert_eq!(2, style.ao_quality);
    assert!(style.pixel_shadow);
    assert_eq!(3, style.voxel_grid, "4 is bounded to 3");
    assert!(style.reflections);
    assert_eq!(5, style.ssr_shine);
    assert_eq!(1, style.ssr_clarity);
    assert_eq!(1, style.ssr_length, "0 is bounded to 1");
    assert_eq!(3, style.ssr_quality);

    // Without the flag every parameter keeps the default of the amendment.
    assert_eq!(StyleSettings::default(), parse(&[]).style_settings());

    // A parameter set without `--style` is kept and normalised, but the style stays Classic, so
    // nothing of it acts: the style is what selects, as REQ-SCN-078 says.
    let classic = parse(&["--style-param", "spl.voxel_grid=2"]).style_settings();
    assert!(!classic.enhanced);
    assert_eq!(2, classic.voxel_grid);
    assert!(!classic.voxel_grid_active());

    let bad = |values: &[&str]| {
        let arguments: Vec<String> = ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&value| value.to_owned())
            .collect();
        Options::parse(&arguments)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default()
    };
    assert!(bad(&["--style-param", "ao.power=3"]).contains("unknown parameter"));
    assert!(bad(&["--style-param", "spl.ao"]).contains("<id>=<value>"));
    assert!(bad(&["--style-param", "spl.ao=lots"]).contains("not a number"));
    // The four curvature parameters of AMD-043 are known identifiers this flag cannot serve:
    // the curvature is not part of the style (REQ-SCN-078) and reaches the scene another way.
    for name in ["spl.bend_zx", "spl.bend_zy", "spl.arc_yx", "spl.arc_center"] {
        assert!(
            bad(&["--style-param", &format!("{name}=30")]).contains("--bend"),
            "{name}"
        );
    }
}

/// The brief's points 1 and 2 as a capture campaign reads them: at `--style enhanced` with every
/// effect off, the three counters of `render.json` are 0 and the capture is the Classic one **byte
/// for byte**; each effect turned on moves its own counter and no other.
///
/// This is the flag pair R0 captures each effect with, so the test is also the demonstration that
/// it works.
#[test]
fn req_scn_080_the_harness_captures_each_effect_separately() {
    let png = |name: &str| fs::read(output(name).join("framing.png")).unwrap();
    let counter = |entries: &[Value], key: &str| -> u64 {
        entries[0]
            .get(key)
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("{key} is in the report"))
    };

    // The Classic capture, and the report of before the amendment: not one of the four keys.
    let (_, classic) = run("style-classic", &["--frames", "synthetic:framing"]);
    for key in [
        "visualStyle",
        "styleProgramsResident",
        "screenEffectBytes",
        "screenEffectPasses",
    ] {
        assert!(
            classic[0].get(key).is_none(),
            "{key} is absent from a Classic report"
        );
    }

    // Enhanced with every effect off: the same picture, to the byte, and three zeros.
    let (_, off) = run(
        "style-enhanced-off",
        &["--frames", "synthetic:framing", "--style", "enhanced"],
    );
    assert_eq!(4, counter(&off, "visualStyle"));
    assert_eq!(0, counter(&off, "styleProgramsResident"));
    assert_eq!(0, counter(&off, "screenEffectBytes"));
    assert_eq!(0, counter(&off, "screenEffectPasses"));
    assert_eq!(
        png("style-classic"),
        png("style-enhanced-off"),
        "Enhanced with every effect off is the Classic capture, byte for byte"
    );

    // The voxel grid is shader variants: programs, and still no screen pass.
    let (_, grid) = run(
        "style-grid",
        &[
            "--frames",
            "synthetic:framing",
            "--style",
            "enhanced",
            "--style-param",
            "spl.voxel_grid=2",
        ],
    );
    assert!(counter(&grid, "styleProgramsResident") > 0);
    assert_eq!(0, counter(&grid, "screenEffectBytes"));
    assert_eq!(0, counter(&grid, "screenEffectPasses"));
    assert_ne!(
        png("style-classic"),
        png("style-grid"),
        "the grid changes the picture"
    );

    // The occlusion is a pass: its own programs, its own targets, and three passes.
    let (_, occlusion) = run(
        "style-ao",
        &[
            "--frames",
            "synthetic:framing",
            "--style",
            "enhanced",
            "--style-param",
            "spl.ao=2",
        ],
    );
    assert_eq!(3, counter(&occlusion, "styleProgramsResident"));
    assert!(counter(&occlusion, "screenEffectBytes") > 0);
    assert_eq!(3, counter(&occlusion, "screenEffectPasses"));
}

/// RO-5 part 2: `--bend`'s fourth part is the arc centre, and it now takes any fraction from 0 to
/// 1 — the five rungs the setting offers and the values between them, so that a capture campaign
/// can compare pivots the menu does not offer, as it already could with the angles.
///
/// The two values AMD-043 REQ-SCN-066 names are unchanged, and so is the default when the part is
/// left out.
#[test]
fn ro_5_the_bend_centre_takes_any_fraction() {
    let parse = |values: &[&str]| {
        let arguments: Vec<String> = ["--out", "x"]
            .iter()
            .chain(values)
            .map(|&value| value.to_owned())
            .collect();
        Options::parse(&arguments).expect("the arguments parse")
    };
    let pivot = |centre: &str| {
        parse(&["--bend", &format!("0,0,60,{centre}")])
            .bend
            .arc_pivot()
    };

    // The amendment's two, and the default.
    assert_eq!(0.0, pivot("0"));
    assert_eq!(zax_engine3d::space_bend::EDGE_PIVOT_OFFSET, pivot("1"));
    assert_eq!(0.0, parse(&["--bend", "0,0,60"]).bend.arc_pivot());

    // The three rungs RO-5 adds, and a value off the grid, which this parser takes as it is.
    for (centre, expected) in [
        ("0.25", -30.0),
        ("0.5", -60.0),
        ("0.75", -90.0),
        ("0.1", -12.0),
    ] {
        assert!(
            (pivot(centre) - expected).abs() < 1e-4,
            "{centre} gave {}",
            pivot(centre)
        );
    }

    let bad = |centre: &str| {
        let arguments: Vec<String> = ["--out", "x", "--bend", &format!("0,0,60,{centre}")]
            .iter()
            .map(|&value| value.to_owned())
            .collect();
        Options::parse(&arguments)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default()
    };
    for centre in ["1.5", "-0.1", "edge", ""] {
        assert!(
            bad(centre).contains("a number from 0 to 1"),
            "{centre}: {}",
            bad(centre)
        );
    }
}
