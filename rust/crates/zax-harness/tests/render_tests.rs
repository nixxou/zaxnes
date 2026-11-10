//! `render --frames synthetic` against `rust/goldens/render-synthetic/`, captured by the C#
//! harness on a Windows GPU: the counts of `render.json` must be identical, and each capture
//! must match its golden image within 2 levels per channel on at least 99.5 % of its pixels.
//! The captures here come from whatever EGL driver the machine has (Mesa llvmpipe in CI).

// A test: a failed render or a malformed golden should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use zax_goldens::{read_render, RenderScene};
use zax_harness::compare;
use zax_harness::render::{self, Options};
use zax_render::assets::png_image;

fn goldens() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens/render-synthetic")
}

/// The triangles `alpha` draws once AMD-063 REQ-SCN-087 has taken its inner walls away.
///
/// `alpha` is a 4 × 4 block of **full** transparent tiles, so the requirement removes every
/// lateral face that falls on a joint between two of them: the C# capture's 2 208 triangles
/// become **1 904**, −304. It is the only scene of this golden set whose geometry the amendment
/// moves — the six before it and the four after hold no transparent full tile, and their
/// triangle counts are unchanged.
const ALPHA_TRIANGLES: u32 = 1904;

/// The mesh variants the removal builds for `alpha`: a corner, an edge and an inner tile each
/// need their own version of the mesh the sixteen tiles share, and they are **memoised** so that
/// a variant keeps its identity from frame to frame (which is what lets the renderer keep its
/// vertex buffers, REQ-E3D-602).
///
/// Memoised means resident: the eight stay in the renderer for the rest of the run, so every
/// scene **from `alpha` on** reports eight more `resident_meshes` than the C# capture did —
/// `alpha` 12 → 20, `hud` 13 → 21, and so on. The scenes before it are untouched.
const REQ_SCN_087_VARIANTS: u32 = 8;

/// Index of `triangles` in [`counts`].
const TRIANGLES: usize = 7;
/// Index of `resident_meshes` in [`counts`].
const RESIDENT_MESHES: usize = 8;

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

#[test]
fn synthetic_captures_match_the_csharp_goldens() {
    let output = std::env::temp_dir().join(format!("zax-harness-render-{}", std::process::id()));
    // The goldens were captured with the anti-aliasing off, which was the default of both the
    // setting and the option before AMD-052 REQ-PERS-089 took the setting's — and with it the
    // option's (Q-087 point 1) — to 1. The comparison asks for the state they were made in.
    //
    // Same for the backdrop lighting: they were captured with the diffuse term of REQ-SCN-028,
    // before AMD-049 REQ-SCN-075 took the desktop default to the almost-matt model. The new
    // model is the subject of `zax-render/tests/amd049_backdrop_tests.rs`; here the C# reference
    // capture is kept as what it is — an independent reference of the state it was made in —
    // rather than overwritten with our own output.
    let arguments = [
        "--out".to_owned(),
        output.display().to_string(),
        "--anti-alias".to_owned(),
        "0".to_owned(),
        "--no-backdrop-lighting".to_owned(),
    ];
    let options = Options::parse(&arguments).unwrap();
    render::run(&options).unwrap();

    let expected = read_render(&goldens().join("render.json")).unwrap();
    let actual = read_render(&output.join("render.json")).unwrap();
    assert_eq!(expected.len(), actual.len());

    let mut failures = Vec::new();
    // Set once `alpha` has been drawn: its mesh variants stay resident from then on.
    let mut alpha_seen = false;
    for (golden, mine) in expected.iter().zip(&actual) {
        assert_eq!(golden.scene, mine.scene);
        assert_eq!(golden.description, mine.description);

        // The golden is **not** regenerated for AMD-063: it is the C# harness's own capture, kept
        // as an independent reference of the state it was made in (see the note above), so what
        // REQ-SCN-087 changes is stated as a correction to it and asserted exactly — a *further*
        // change to the removal still fails this test.
        let mut expected = counts(golden);
        if golden.scene == "alpha" {
            expected[TRIANGLES] = ALPHA_TRIANGLES;
        }
        if golden.scene == "alpha" || alpha_seen {
            expected[RESIDENT_MESHES] += REQ_SCN_087_VARIANTS;
        }
        assert_eq!(expected, counts(mine), "counts of '{}'", golden.scene);
        alpha_seen |= golden.scene == "alpha";

        let file = format!("{}.png", golden.scene);
        let reference = png_image::load(&goldens().join(&file)).unwrap();
        let capture = png_image::load(&output.join(&file)).unwrap();
        assert_eq!(
            (reference.width, reference.height),
            (capture.width, capture.height)
        );

        let share = compare::matching_share(&capture.pixels, &reference.pixels, 2);
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
