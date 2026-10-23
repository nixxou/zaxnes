//! `render --frames synthetic` against `rust/goldens/render-synthetic/`, captured by the C#
//! harness on a Windows GPU: the counts of `render.json` must be identical, and each capture
//! must match its golden image within 2 levels per channel on at least 99.5 % of its pixels.
//! The captures here come from whatever EGL driver the machine has (Mesa llvmpipe in CI).

// A test: a failed render or a malformed golden should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use zax_goldens::{read_render, RenderScene};
use zax_harness::render::{self, Options};
use zax_render::assets::png_image;

fn goldens() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens/render-synthetic")
}

/// The counts that do not depend on the rasteriser.
fn counts(scene: &RenderScene) -> [u32; 13] {
    [
        scene.width,
        scene.height,
        scene.opaque_tiles,
        scene.transparent_tiles,
        scene.hud_tiles,
        scene.shadow_casters,
        scene.draw_calls,
        scene.triangles,
        scene.resident_meshes,
        scene.anti_alias_samples,
        scene.shadow_map_size,
        scene.requested_shadow_map_size,
        // The background is the clear colour around the picture: geometry, not shading.
        scene.background_pixels,
    ]
}

/// Share of pixels whose three channels are within `tolerance` of the golden's.
#[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^52.
fn matching(capture: &[u8], golden: &[u8], tolerance: u8) -> f64 {
    let close = capture
        .chunks_exact(4)
        .zip(golden.chunks_exact(4))
        .filter(|(a, b)| (0..3).all(|c| a[c].abs_diff(b[c]) <= tolerance))
        .count();
    close as f64 / (golden.len() / 4) as f64
}

#[test]
fn synthetic_captures_match_the_csharp_goldens() {
    let output = std::env::temp_dir().join(format!("zax-harness-render-{}", std::process::id()));
    let arguments = ["--out".to_owned(), output.display().to_string()];
    let options = Options::parse(&arguments).unwrap();
    render::run(&options).unwrap();

    let expected = read_render(&goldens().join("render.json")).unwrap();
    let actual = read_render(&output.join("render.json")).unwrap();
    assert_eq!(expected.len(), actual.len());

    let mut failures = Vec::new();
    for (golden, mine) in expected.iter().zip(&actual) {
        assert_eq!(golden.scene, mine.scene);
        assert_eq!(golden.description, mine.description);
        assert_eq!(counts(golden), counts(mine), "counts of '{}'", golden.scene);

        let file = format!("{}.png", golden.scene);
        let reference = png_image::load(&goldens().join(&file)).unwrap();
        let capture = png_image::load(&output.join(&file)).unwrap();
        assert_eq!(
            (reference.width, reference.height),
            (capture.width, capture.height)
        );

        let share = matching(&capture.pixels, &reference.pixels, 2);
        println!(
            "{:<18} {:>7.3} % of the pixels within 2 levels ({} colours, golden {})",
            golden.scene,
            share * 100.0,
            mine.distinct_colors,
            golden.distinct_colors
        );
        if share < 0.995 {
            failures.push(format!("{} ({:.3} %)", golden.scene, share * 100.0));
        }
    }

    fs::remove_dir_all(&output).unwrap();
    assert!(
        failures.is_empty(),
        "captures off the goldens: {}",
        failures.join(", ")
    );
}

#[test]
fn the_scene_list_is_the_one_of_the_goldens() {
    let options = Options::parse(&["--list-scenes".to_owned()]).unwrap();
    let names = render::run(&options).unwrap();
    let expected: Vec<String> = read_render(&goldens().join("render.json"))
        .unwrap()
        .into_iter()
        .map(|scene| scene.scene)
        .collect();
    assert_eq!(expected, names.lines().collect::<Vec<_>>());
}

#[test]
fn a_frame_file_or_a_rom_that_does_not_exist_is_an_error() {
    let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("render-missing");
    for source in [["--frames", "missing.json"], ["--rom", "missing.nes"]] {
        let arguments = [
            "--out".to_owned(),
            output.to_str().unwrap().to_owned(),
            source[0].to_owned(),
            source[1].to_owned(),
        ];
        let options = Options::parse(&arguments).unwrap();
        assert!(render::run(&options).is_err(), "{source:?}");
    }
}
