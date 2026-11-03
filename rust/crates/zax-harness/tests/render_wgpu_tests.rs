//! `render --frames synthetic --backend vulkan|dx12` against `rust/goldens/render-synthetic/`,
//! the same check as `render_tests.rs` for the wgpu renderer: identical counts, and each capture
//! within 2 levels per channel of its golden on at least 99.5 % of its pixels. A machine without
//! the API (no Vulkan driver, DirectX 12 off Windows) skips that backend.
#![cfg(feature = "wgpu")]
// A test: a failed render or a malformed golden should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use zax_goldens::read_render;
use zax_harness::compare;
use zax_harness::render::{self, Options};
use zax_harness::HarnessError;
use zax_render::assets::png_image;

fn goldens() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens/render-synthetic")
}

fn check(backend: &str) {
    let output = std::env::temp_dir().join(format!(
        "zax-harness-render-{backend}-{}",
        std::process::id()
    ));
    // The same two options as `render_tests.rs`, and for the same reasons it states: the goldens
    // were captured with the anti-aliasing off and with the diffuse backdrop term of
    // REQ-SCN-028, before AMD-052 REQ-PERS-089 took the anti-aliasing default to 1 and AMD-049
    // REQ-SCN-075 took the backdrop to the almost-matt model. Without them this test compared a
    // MSAA 4x capture against a no-AA golden, which no backend can match: it read 93.1 % on
    // `heavy` and 58.5 % on `backdrop` on an adapter that in fact renders them both correctly.
    // It went unnoticed because the test skips where no adapter answers, which is every machine
    // the two lots that wrote it ran on (Q-104 §2, §5.2). RU-7 step 1.
    let arguments = [
        "--out".to_owned(),
        output.display().to_string(),
        "--backend".to_owned(),
        backend.to_owned(),
        "--anti-alias".to_owned(),
        "0".to_owned(),
        "--no-backdrop-lighting".to_owned(),
    ];
    let options = Options::parse(&arguments).unwrap();
    let result = render::run(&options);
    if let Err(HarnessError::Gpu(message)) = &result {
        if message.starts_with("no ") {
            eprintln!("skipped: {message}");
            return;
        }
    }
    result.unwrap();

    let expected = read_render(&goldens().join("render.json")).unwrap();
    let actual = read_render(&output.join("render.json")).unwrap();
    assert_eq!(expected.len(), actual.len());

    let mut failures = Vec::new();
    for (golden, mine) in expected.iter().zip(&actual) {
        assert_eq!(golden.scene, mine.scene);
        assert_eq!(
            (
                golden.opaque_tiles,
                golden.transparent_tiles,
                golden.hud_tiles
            ),
            (mine.opaque_tiles, mine.transparent_tiles, mine.hud_tiles),
            "tiles of '{}'",
            golden.scene
        );
        assert_eq!(
            (golden.shadow_casters, golden.draw_calls, golden.triangles),
            (mine.shadow_casters, mine.draw_calls, mine.triangles),
            "draws of '{}'",
            golden.scene
        );

        let file = format!("{}.png", golden.scene);
        let reference = png_image::load(&goldens().join(&file)).unwrap();
        let capture = png_image::load(&output.join(&file)).unwrap();
        let share = compare::matching_share(&capture.pixels, &reference.pixels, 2);
        println!(
            "{backend} {:<18} {:>7.3} % of the pixels within 2 levels",
            golden.scene,
            share * 100.0
        );
        if share < 0.995 {
            failures.push(format!("{} ({:.3} %)", golden.scene, share * 100.0));
        }
    }

    fs::remove_dir_all(&output).unwrap();
    assert!(failures.is_empty(), "{backend}: {failures:?}");
}

#[test]
fn vulkan_captures_match_the_goldens() {
    check("vulkan");
}

#[cfg(windows)]
#[test]
fn dx12_captures_match_the_goldens() {
    check("dx12");
}
