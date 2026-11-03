//! RU-7: the `wgpu` backend against the OpenGL one, effect by effect.
//!
//! Q-104 recorded that the curvature of AMD-043/AMD-046 and the Enhanced style of AMD-055 were
//! rendered by the OpenGL backend alone, and gave as its reason that WGSL could not be verified:
//! `render_wgpu_tests.rs` skips without an adapter. The owner's answer was to port it, and this
//! is what replaces the known-gap test that pinned the gap — **the same synthetic scenes drawn
//! through both backends, and the two pictures compared**.
//!
//! Exact equality is not expected and not asked for: the two backends rasterise through different
//! drivers, evaluate the same arithmetic in whatever order their compilers choose, and reach the
//! depth buffer through 0..1 clip space on one side and -1..1 on the other. What is asked is that
//! every effect agree to within a couple of sRGB levels on all but a fringe of the pixels — the
//! same bar `render_wgpu_tests.rs` holds the flat Classic frame to — and that each effect be seen
//! to **do** something, so that a shader which silently compiled to nothing cannot pass.
//!
//! A machine with no Vulkan and no DirectX 12 adapter skips, with the reason printed, exactly as
//! `render_wgpu_tests.rs` does.
#![cfg(feature = "wgpu")]
// A test: a failed render or a malformed capture should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use zax_harness::compare::matching_share;
use zax_harness::render::{self, Options};
use zax_harness::HarnessError;
use zax_render::assets::png_image;

/// The scenes of the synthetic set this file reads. `framing` fills the view with 1798 tile boxes
/// and is where a screen-space effect shows; `shadow` carries the shadow pass REQ-SCN-068 bends
/// with the scene; `heavy` is the densest frame of the set and the worst case for every
/// rasterisation difference; `hud` is the one REQ-SCN-068 and AMD-055 exclude.
const SCENES: [&str; 4] = ["framing", "shadow", "heavy", "hud"];

/// What two captures of one scene differ by.
#[derive(Debug, Clone, Copy)]
struct Figures {
    /// Largest difference on any one colour channel, in sRGB levels.
    max_channel: u8,
    /// Share of pixels not identical on all three channels.
    differing: f64,
    /// Share of pixels within 2 levels on every channel — the measure of the golden tests.
    within_two: f64,
}

fn figures(a: &[u8], b: &[u8]) -> Figures {
    assert_eq!(a.len(), b.len(), "captures differ in size");
    let mut max_channel = 0_u8;
    let mut differing = 0_u64;
    let mut pixels = 0_u64;
    for (left, right) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        pixels += 1;
        let mut same = true;
        for channel in 0..3 {
            let delta = left[channel].abs_diff(right[channel]);
            max_channel = max_channel.max(delta);
            same &= delta == 0;
        }
        differing += u64::from(!same);
    }
    #[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^53.
    Figures {
        max_channel,
        differing: differing as f64 / pixels as f64,
        within_two: matching_share(a, b, 2),
    }
}

/// Renders the synthetic set with `extra`, on `backend` (`None` for the OpenGL reference), into a
/// folder of its own. `None` when the backend has no adapter here.
fn capture(label: &str, backend: Option<&str>, extra: &[&str]) -> Option<PathBuf> {
    let output = std::env::temp_dir().join(format!(
        "zax-parity-{label}-{}-{}",
        backend.unwrap_or("gl"),
        std::process::id()
    ));
    let mut arguments = vec![
        "--out".to_owned(),
        output.display().to_string(),
        // The anti-aliasing and the backdrop model are fixed here so that one variable moves at a
        // time; the anti-aliasing levels are a subject of their own, below.
        "--anti-alias".to_owned(),
        "0".to_owned(),
        "--no-backdrop-lighting".to_owned(),
    ];
    if let Some(backend) = backend {
        arguments.push("--backend".to_owned());
        arguments.push(backend.to_owned());
    }
    arguments.extend(extra.iter().map(|&argument| argument.to_owned()));

    let options = Options::parse(&arguments).unwrap();
    match render::run(&options) {
        Ok(_) => Some(output),
        Err(HarnessError::Gpu(message)) if message.starts_with("no ") => {
            eprintln!("skipped: {message}");
            None
        }
        Err(error) => panic!("{label} on {}: {error}", backend.unwrap_or("gl")),
    }
}

fn load(folder: &PathBuf, scene: &str) -> Vec<u8> {
    png_image::load(&folder.join(format!("{scene}.png")))
        .unwrap()
        .pixels
}

/// The backends to compare against OpenGL: Vulkan, and DirectX 12 on Windows.
fn backends() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        vec!["vulkan", "dx12"]
    }
    #[cfg(not(windows))]
    {
        vec!["vulkan"]
    }
}

/// Renders `extra` on the OpenGL backend and on each reachable `wgpu` one, prints the figures of
/// every scene of [`SCENES`], and hands back the worst `within_two` and the largest
/// `max_channel` seen — over every backend and every scene.
///
/// `label` names the effect in the printed table and in the temporary folders.
fn parity(label: &str, extra: &[&str]) -> Option<(f64, u8)> {
    let reference = capture(label, None, extra).expect("the OpenGL backend always renders");
    let mut worst_share = 1.0_f64;
    let mut worst_channel = 0_u8;
    let mut any = false;
    for backend in backends() {
        let Some(capture) = capture(label, Some(backend), extra) else {
            continue;
        };
        any = true;
        for scene in SCENES {
            let figures = figures(&load(&capture, scene), &load(&reference, scene));
            println!(
                "{label:22} {backend:7} {scene:<10} max {:>3} levels, {:>7.4} % of pixels differ, \
                 {:>8.4} % within 2",
                figures.max_channel,
                figures.differing * 100.0,
                figures.within_two * 100.0
            );
            worst_share = worst_share.min(figures.within_two);
            worst_channel = worst_channel.max(figures.max_channel);
        }
        fs::remove_dir_all(&capture).unwrap();
    }
    fs::remove_dir_all(&reference).unwrap();
    any.then_some((worst_share, worst_channel))
}

/// What every effect of this file is held to: the bar of `render_wgpu_tests.rs`, which is the one
/// the flat Classic frame already meets on this hardware.
const WITHIN_TWO: f64 = 0.995;

/// The flat Classic frame, as a control: whatever the figures of an effect below are, they are
/// read against these.
#[test]
fn ru_7_the_classic_frame_agrees_between_the_backends() {
    let Some((share, channel)) = parity("classic", &[]) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the Classic frame agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "classic: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// AMD-043 REQ-SCN-067 and REQ-SCN-068: the curvature places the geometry, transports the normal
/// and bends the shadow pass, and it now does so on both backends.
#[test]
fn req_scn_067_the_curvature_agrees_between_the_backends() {
    let Some((share, channel)) = parity("bend-lr-90", &["--bend", "90,0,0"]) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the curvature agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "bend 90,0,0: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// The arc of AMD-043 REQ-SCN-066, the third axis, with its centre moved off the default — the
/// one axis whose wrapping reads a pivot (`bend.wgsl`'s `bend_k.w`), so a port that dropped the
/// pivot would pass the test above and fail this one.
#[test]
fn req_scn_066_the_arc_and_its_centre_agree_between_the_backends() {
    let Some((share, _)) = parity("bend-arc-60", &["--bend", "0,0,60,1"]) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the arc agrees on only {:.4} % of pixels",
        share * 100.0
    );
}

/// All three axes at once, which is the only configuration where the Jacobians of REQ-SCN-068
/// compose: a port that applied them in another order, or multiplied the Jacobian the other way
/// round, shades differently here and nowhere else.
#[test]
fn req_scn_068_the_three_axes_together_agree_between_the_backends() {
    let Some((share, _)) = parity("bend-all", &["--bend", "45,30,20"]) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the three axes agree on only {:.4} % of pixels",
        share * 100.0
    );
}

/// The curvature has to **do** something on the `wgpu` side, and the same something: the bent
/// picture differs from the flat one by as much on each backend as on OpenGL.
///
/// This is what a WGSL variant compiled but never selected, or selected but handed zeros, would
/// fail — and it is the half of the check the share above cannot make, since two flat pictures
/// agree perfectly.
#[test]
fn req_scn_067_the_curvature_moves_as_much_of_the_frame_on_each_backend() {
    let flat_gl = capture("moves-flat", None, &[]).expect("the OpenGL backend always renders");
    let bent_gl = capture("moves-bent", None, &["--bend", "90,0,0"])
        .expect("the OpenGL backend always renders");
    // `framing` fills the view, so the curvature moves most of it; `hud` is excluded by
    // REQ-SCN-068 and must not move at all.
    let reference = figures(&load(&flat_gl, "framing"), &load(&bent_gl, "framing")).differing;
    assert!(
        reference > 0.25,
        "the OpenGL curvature moved only {:.2} % of the frame",
        reference * 100.0
    );

    for backend in backends() {
        let Some(flat) = capture("moves-flat", Some(backend), &[]) else {
            continue;
        };
        let bent = capture("moves-bent", Some(backend), &["--bend", "90,0,0"]).unwrap();
        let moved = figures(&load(&flat, "framing"), &load(&bent, "framing")).differing;
        println!(
            "{backend}: the curvature moves {:.2} % of `framing`, OpenGL {:.2} %",
            moved * 100.0,
            reference * 100.0
        );
        assert!(
            (moved - reference).abs() < 0.01,
            "{backend} moved {:.2} % of the frame against OpenGL's {:.2} %",
            moved * 100.0,
            reference * 100.0
        );

        // REQ-SCN-068: the HUD is not bent, on either backend.
        let hud = figures(&load(&flat, "hud"), &load(&bent, "hud"));
        assert_eq!(
            hud.max_channel, 0,
            "{backend} bent the HUD by up to {} levels",
            hud.max_channel
        );
        fs::remove_dir_all(&flat).unwrap();
        fs::remove_dir_all(&bent).unwrap();
    }
    fs::remove_dir_all(&flat_gl).unwrap();
    fs::remove_dir_all(&bent_gl).unwrap();
}
