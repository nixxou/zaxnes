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
use std::sync::atomic::{AtomicUsize, Ordering};

use zax_goldens::read_render;
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
///
/// The folder name carries a counter as well as the label and the process: the tests of this file
/// run in parallel and two of them may ask for the same label and backend, in which case one
/// would delete the folder the other is still reading.
fn capture(label: &str, backend: Option<&str>, extra: &[&str]) -> Option<PathBuf> {
    capture_with(label, backend, extra, false)
}

/// The same with the backdrop model of AMD-049 REQ-SCN-075 **on**, which is the desktop default
/// (`lit`). Every other test of this file turns it off so that one variable moves at a time; the
/// backdrop is a subject of its own, below.
fn capture_lit(label: &str, backend: Option<&str>, extra: &[&str]) -> Option<PathBuf> {
    capture_with(label, backend, extra, true)
}

fn capture_with(label: &str, backend: Option<&str>, extra: &[&str], lit: bool) -> Option<PathBuf> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let output = std::env::temp_dir().join(format!(
        "zax-parity-{label}-{}-{}-{}",
        backend.unwrap_or("gl"),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut arguments = vec![
        "--out".to_owned(),
        output.display().to_string(),
        "--anti-alias".to_owned(),
        "0".to_owned(),
    ];
    if !lit {
        arguments.push("--no-backdrop-lighting".to_owned());
    }
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
    parity_with(label, extra, false, &SCENES)
}

/// The same with the backdrop model of AMD-049 REQ-SCN-075 on, over the scenes that have a
/// backdrop — which [`SCENES`] alone does not reach, since the one scene built for it is not in
/// that list.
fn parity_lit(label: &str, extra: &[&str]) -> Option<(f64, u8)> {
    parity_with(label, extra, true, &BACKDROP_SCENES)
}

fn parity_with(label: &str, extra: &[&str], lit: bool, scenes: &[&str]) -> Option<(f64, u8)> {
    let reference =
        capture_with(label, None, extra, lit).expect("the OpenGL backend always renders");
    let mut worst_share = 1.0_f64;
    let mut worst_channel = 0_u8;
    let mut any = false;
    for backend in backends() {
        let Some(capture) = capture_with(label, Some(backend), extra, lit) else {
            continue;
        };
        any = true;
        for &scene in scenes {
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

// ------------------------------------------------------------------ AMD-055, the shader effects

/// AMD-055 REQ-SCN-081: the voxel grid modulates the albedo, on its own — REQ-SCN-079 switches it
/// and the pixel shadows separately, so each is measured without the other.
#[test]
fn req_scn_081_the_voxel_grid_agrees_between_the_backends() {
    let Some((share, channel)) = parity(
        "voxel-grid",
        &["--style", "enhanced", "--style-param", "spl.voxel_grid=3"],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the voxel grid agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "voxel grid: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// AMD-055 REQ-SCN-082: the shadow read at the centre of the fragment's voxel, on its own.
#[test]
fn req_scn_082_the_pixel_shadows_agree_between_the_backends() {
    let Some((share, channel)) = parity(
        "pixel-shadow",
        &["--style", "enhanced", "--style-param", "spl.pixel_shadow=1"],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the pixel shadows agree on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "pixel shadows: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// The two together, and with the curvature on top: REQ-SCN-081 and REQ-SCN-082 both evaluate in
/// the mesh's local frame **before** the curvature, and the pixel shadows then place the snapped
/// point through it (REQ-SCN-082 point 3, REQ-SCN-068). That ordering is the one thing this
/// configuration checks and the three above cannot.
#[test]
fn req_scn_082_the_two_effects_and_the_curvature_agree_between_the_backends() {
    let Some((share, _)) = parity(
        "grid-shadow-bend",
        &[
            "--style",
            "enhanced",
            "--style-param",
            "spl.voxel_grid=3,spl.pixel_shadow=1",
            "--bend",
            "60,0,0",
        ],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the two effects under curvature agree on only {:.4} % of pixels",
        share * 100.0
    );
}

/// Each of the two effects has to **do** something on the `wgpu` side, and the same something —
/// the half of the check a share cannot make, since two pictures without the effect agree
/// perfectly. And neither touches the HUD (REQ-SCN-081, REQ-SCN-082).
///
/// Each effect is read on the scene that shows it, and the floor is the share that effect moves
/// on the **OpenGL** backend, measured here rather than assumed. The two are four orders of
/// magnitude apart, and the reason is the fixtures rather than the port:
///
/// - the grid modulates the albedo of every face, so it moves **32.25 %** of `framing`;
/// - the pixel shadows move the point a shadow is *read* at, and the synthetic set's shadows
///   cover a sliver of the frame — **0.0091 %** of `shadow`, about 28 pixels of 307 200, and
///   nothing at all of `framing`, which has no cast shadow for them to move.
///
/// So this test is weak evidence for REQ-SCN-082 and strong evidence for REQ-SCN-081, and the
/// floor is set just under each measured value rather than at a round number that would flatter
/// one of them. What carries REQ-SCN-082 on this side is the agreement figure of
/// `req_scn_082_the_pixel_shadows_agree_between_the_backends` together with the OpenGL unit tests
/// of `amd055_style_tests.rs`; a fixture with a large cast shadow would be the thing to add.
#[test]
fn req_scn_079_each_shader_effect_moves_as_much_of_the_frame_on_each_backend() {
    let effects = [
        ("voxel-grid", "spl.voxel_grid=3", "framing", 0.15),
        ("pixel-shadow", "spl.pixel_shadow=1", "shadow", 0.00005),
    ];
    for (label, parameter, scene, least) in effects {
        let classic = capture("moves-classic", None, &[]).expect("OpenGL always renders");
        let enhanced = capture(
            label,
            None,
            &["--style", "enhanced", "--style-param", parameter],
        )
        .expect("OpenGL always renders");
        let reference = figures(&load(&classic, scene), &load(&enhanced, scene)).differing;
        assert!(
            reference > least,
            "{label} moved only {:.4} % of `{scene}` on OpenGL",
            reference * 100.0
        );

        for backend in backends() {
            let Some(flat) = capture("moves-classic", Some(backend), &[]) else {
                continue;
            };
            let on = capture(
                label,
                Some(backend),
                &["--style", "enhanced", "--style-param", parameter],
            )
            .unwrap();
            let moved = figures(&load(&flat, scene), &load(&on, scene)).differing;
            println!(
                "{label} on {backend}: moves {:.4} % of `{scene}`, OpenGL {:.4} %",
                moved * 100.0,
                reference * 100.0
            );
            assert!(
                (moved - reference).abs() < 0.01,
                "{label} on {backend} moved {:.4} % against OpenGL's {:.4} %",
                moved * 100.0,
                reference * 100.0
            );

            let hud = figures(&load(&flat, "hud"), &load(&on, "hud"));
            assert_eq!(
                hud.max_channel, 0,
                "{label} on {backend} changed the HUD by up to {} levels",
                hud.max_channel
            );
            fs::remove_dir_all(&flat).unwrap();
            fs::remove_dir_all(&on).unwrap();
        }
        fs::remove_dir_all(&classic).unwrap();
        fs::remove_dir_all(&enhanced).unwrap();
    }
}

// ------------------------------------------------------ AMD-055, the occlusion of REQ-SCN-083

/// The settings that turn the hemisphere occlusion of REQ-SCN-083 on: `spl.ao` = 2, the mode
/// whose estimator is reconstructed from the frame's depth alone.
const DEPTH_OCCLUSION: [&str; 4] = ["--style", "enhanced", "--style-param", "spl.ao=2"];

/// AMD-055 REQ-SCN-083, mode `spl.ao` = 2: the whole chain — the depth reconstruction, the
/// hemisphere kernel, the depth-aware blur and the multiply composite — agrees with OpenGL.
///
/// This is the first effect of the lot that is a **screen pass** rather than a scene variant, so
/// it is also where the two conventions `screen_depth.wgsl` undoes are read: a port that kept the
/// GLSL's `ndc = uv * 2 - 1` would mirror view space in y, and a port that got the stored depth
/// wrong would darken the wrong distances.
#[test]
fn req_scn_083_the_depth_occlusion_agrees_between_the_backends() {
    let Some((share, channel)) = parity("ao-depth", &DEPTH_OCCLUSION) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the Depth occlusion agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "ao depth: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// REQ-SCN-083 at `ao.quality` High, where the estimator runs at the frame's **own** resolution
/// rather than half of each dimension — a different target size, a different `uTexel`, and the
/// composite's upsample out of the picture. The divisor is the one number of this pass a port can
/// get right at one quality and wrong at the other.
#[test]
fn req_scn_083_the_full_resolution_occlusion_agrees_between_the_backends() {
    let Some((share, _)) = parity(
        "ao-high",
        &[
            "--style",
            "enhanced",
            "--style-param",
            "spl.ao=2,ao.quality=2",
        ],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the full-resolution occlusion agrees on only {:.4} % of pixels",
        share * 100.0
    );
}

/// The occlusion has to **do** something on the `wgpu` side, and the same something — the half of
/// the check a share cannot make. And it must not touch the HUD, which REQ-SCN-080 draws *after*
/// the pass (so the exclusion is the frame order rather than a test in the shader).
///
/// **The scene is `shadow`, and which scene it is was measured rather than chosen.** On the
/// OpenGL backend the Depth occlusion leaves `framing`, `heavy`, `clip`, `nudge`, `alpha`, `sky`
/// and `hud` **byte-identical** and moves 1.76 % of `shadow` (and of `no-shadow` and
/// `layer-shadow-off`) and 2.54 % of `backdrop`. That is REQ-SCN-083 working as written, not a
/// defect: `O` is null "là où aucune géométrie occultante ne se trouve à moins de 2r", and the
/// dense tile-box scenes of the set are flat facades with no concavity inside the radius, while
/// the three shadow scenes and the backdrop have a box standing on a floor. It is also why the
/// `ao-depth` rows above sit exactly on the Classic control for `framing` and `heavy` — on those
/// two the pass runs and changes nothing, on either backend.
#[test]
fn req_scn_083_the_occlusion_darkens_as_much_of_the_frame_on_each_backend() {
    let classic = capture("darkens-classic", None, &[]).expect("OpenGL always renders");
    let occluded = capture("darkens-ao", None, &DEPTH_OCCLUSION).expect("OpenGL always renders");
    let reference = figures(&load(&classic, "shadow"), &load(&occluded, "shadow")).differing;
    assert!(
        reference > 0.015,
        "the OpenGL occlusion darkened only {:.2} % of `shadow`",
        reference * 100.0
    );
    // And it leaves the scenes with no occluder within the radius exactly alone.
    for scene in ["framing", "heavy"] {
        let untouched = figures(&load(&classic, scene), &load(&occluded, scene));
        assert_eq!(
            untouched.max_channel, 0,
            "REQ-SCN-083 moved `{scene}` on OpenGL, so this file's reading of the set is stale"
        );
    }

    for backend in backends() {
        let Some(flat) = capture("darkens-classic", Some(backend), &[]) else {
            continue;
        };
        let on = capture("darkens-ao", Some(backend), &DEPTH_OCCLUSION).unwrap();
        let moved = figures(&load(&flat, "shadow"), &load(&on, "shadow")).differing;
        println!(
            "ao depth on {backend}: darkens {:.4} % of `shadow`, OpenGL {:.4} %",
            moved * 100.0,
            reference * 100.0
        );
        assert!(
            (moved - reference).abs() < 0.004,
            "the occlusion on {backend} darkened {:.4} % against OpenGL's {:.4} %",
            moved * 100.0,
            reference * 100.0
        );

        let hud = figures(&load(&flat, "hud"), &load(&on, "hud"));
        assert_eq!(
            hud.max_channel, 0,
            "the occlusion on {backend} changed the HUD by up to {} levels",
            hud.max_channel
        );
        fs::remove_dir_all(&flat).unwrap();
        fs::remove_dir_all(&on).unwrap();
    }
    fs::remove_dir_all(&classic).unwrap();
    fs::remove_dir_all(&occluded).unwrap();
}

/// AMD-055 REQ-SCN-080 under anti-aliasing, which is the one path with no OpenGL counterpart: the
/// passes read the **resolved** depth, and `wgpu` has no depth resolve, so this frame submits the
/// extra pass of `shaders/depth_resolve.wgsl` and then draws the transparent objects and the HUD
/// into the single-sample target.
///
/// A port that read the multisampled depth would not compile; one that read an **unresolved**
/// single-sample depth — never written — would darken nothing at all, which is what the
/// comparison with OpenGL's own multisampled capture catches.
#[test]
fn req_scn_080_the_occlusion_agrees_under_anti_aliasing() {
    let mut arguments = vec!["--anti-alias", "1"];
    arguments.extend_from_slice(&DEPTH_OCCLUSION);
    let Some((share, channel)) = parity("ao-depth-aa", &arguments) else {
        return;
    };
    println!(
        "ao depth, MSAA 4x: worst {:.4} % within 2 of OpenGL, max {channel} levels",
        share * 100.0
    );
    assert!(
        share >= WITHIN_TWO_MULTISAMPLED,
        "the occlusion under MSAA 4x agrees with OpenGL on only {:.4} % of pixels",
        share * 100.0
    );
}

/// What the occlusion asked of each backend, read off `render.json`: the share figures say the two
/// pictures match, and these say the two renderers did the same amount of work to get there.
///
/// The one number that differs by construction is the **pass count under anti-aliasing**: the
/// depth resolve of REQ-SCN-080's last bullet is one `glBlitFramebuffer` on OpenGL, which is not a
/// pass, and a render pass of its own here. So `wgpu` submits exactly one more, and the test says
/// so rather than relaxing to an inequality.
#[test]
fn req_scn_080_the_occlusion_costs_the_same_passes_and_bytes_on_each_backend() {
    for (level, extra) in [("0", 0_u32), ("1", 1)] {
        let mut arguments = vec!["--anti-alias", level];
        arguments.extend_from_slice(&DEPTH_OCCLUSION);
        let reference = capture("ao-counts", None, &arguments).expect("OpenGL always renders");
        let counters = |folder: &PathBuf| {
            let records = read_render(&folder.join("render.json")).unwrap();
            let scene = records
                .iter()
                .find(|record| record.scene == "framing")
                .expect("the synthetic set has a `framing` scene");
            (
                scene.screen_effect_passes.expect("Enhanced writes the key"),
                scene.screen_effect_bytes.expect("Enhanced writes the key"),
            )
        };
        let (gl_passes, gl_bytes) = counters(&reference);
        for backend in backends() {
            let Some(capture) = capture("ao-counts", Some(backend), &arguments) else {
                continue;
            };
            let (passes, bytes) = counters(&capture);
            println!(
                "ao depth, anti_alias {level} on {backend}: {passes} passes and {bytes} bytes, \
                 OpenGL {gl_passes} and {gl_bytes}"
            );
            assert_eq!(
                passes,
                gl_passes + extra,
                "anti_alias {level} on {backend} submitted {passes} screen passes, \
                 OpenGL {gl_passes} plus the {extra} this API needs for the depth resolve"
            );
            assert_eq!(
                bytes, gl_bytes,
                "anti_alias {level} on {backend} allocated {bytes} bytes of screen targets, \
                 OpenGL {gl_bytes}"
            );
            fs::remove_dir_all(&capture).unwrap();
        }
        fs::remove_dir_all(&reference).unwrap();
    }
}

// --------------------------------- AMD-055, the multi-scale estimator of REQ-SCN-083 *Mode MSVO*

/// The settings that turn the multi-scale occlusion on: `spl.ao` = 1, the four-level pyramid RU-6
/// ported from Microsoft's `MiniEngine`.
const MULTI_SCALE: [&str; 4] = ["--style", "enhanced", "--style-param", "spl.ao=1"];

/// AMD-055 REQ-SCN-083 *Mode MSVO*: the whole chain of nineteen passes — the linear pyramid, the
/// sphere-penetration test per level, the `SmartBlur` over each of them and the bilateral upsample
/// — agrees with OpenGL.
///
/// Two details of this chain read a **row index**, and both had to be mirrored into OpenGL's
/// convention rather than transcribed: which texel of each 2 × 2 block the decimation keeps
/// (`msvo_downsample.wgsl`) and which pixel coordinate the per-pixel rotation hashes
/// (`msvo_occlude.wgsl`). A port that transcribed either would still be admissible — the
/// requirement leaves the decimation and the pattern free — and would read far worse here, which
/// is what this test is for.
#[test]
fn req_scn_083_the_multi_scale_occlusion_agrees_between_the_backends() {
    let Some((share, channel)) = parity("ao-msvo", &MULTI_SCALE) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the multi-scale occlusion agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "ao msvo: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// The same at `ao.quality` High, where every level of the pyramid doubles in each dimension: the
/// focal length of each level, the sphere's pixel radius and the blur's step all change with it.
#[test]
fn req_scn_083_the_full_resolution_pyramid_agrees_between_the_backends() {
    let Some((share, _)) = parity(
        "ao-msvo-high",
        &[
            "--style",
            "enhanced",
            "--style-param",
            "spl.ao=1,ao.quality=2",
        ],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the full-resolution pyramid agrees on only {:.4} % of pixels",
        share * 100.0
    );
}

/// What the multi-scale chain asked of each backend: **nineteen** passes and the composite, the
/// four levels of `style::MSVO_LEVELS`, and the same pyramid bytes — the strongest statement that
/// the two renderers ran the same chain rather than two chains that happen to look alike.
#[test]
fn req_scn_083_the_multi_scale_chain_costs_the_same_passes_and_bytes_on_each_backend() {
    let reference = capture("msvo-counts", None, &MULTI_SCALE).expect("OpenGL always renders");
    let counters = |folder: &PathBuf| {
        let records = read_render(&folder.join("render.json")).unwrap();
        let scene = records
            .iter()
            .find(|record| record.scene == "framing")
            .expect("the synthetic set has a `framing` scene");
        (
            scene.screen_effect_passes.expect("Enhanced writes the key"),
            scene.screen_effect_bytes.expect("Enhanced writes the key"),
            scene
                .style_programs_resident
                .expect("Enhanced writes the key"),
        )
    };
    let (gl_passes, gl_bytes, gl_programs) = counters(&reference);
    assert_eq!(
        gl_passes, 20,
        "the nineteen passes of the chain and the composite"
    );
    for backend in backends() {
        let Some(capture) = capture("msvo-counts", Some(backend), &MULTI_SCALE) else {
            continue;
        };
        let (passes, bytes, programs) = counters(&capture);
        println!(
            "ao msvo on {backend}: {passes} passes, {bytes} bytes and {programs} programs; \
             OpenGL {gl_passes}, {gl_bytes} and {gl_programs}"
        );
        assert_eq!(passes, gl_passes, "the same nineteen passes and composite");
        assert_eq!(bytes, gl_bytes, "the same pyramid");
        assert_eq!(
            programs, gl_programs,
            "the five of the chain and the composite"
        );
        fs::remove_dir_all(&capture).unwrap();
    }
    fs::remove_dir_all(&reference).unwrap();
}

// ----------------------------------------------------- AMD-055, the reflections of REQ-SCN-084

/// The settings that turn the floor reflections of REQ-SCN-084 on.
const REFLECTIONS: [&str; 4] = ["--style", "enhanced", "--style-param", "spl.reflections=1"];

/// AMD-055 REQ-SCN-084 over the synthetic set, which has **no floor at all**: the floor mask comes
/// out empty, the trace leaves every pixel at zero and the composite blends nothing — so the
/// picture must be the Classic one on both backends, and the two must agree as the Classic frame
/// does.
///
/// That the set has no floor is measured rather than assumed, below, and it is why the figures for
/// what the reflections actually *do* are in `zax-render/tests/amd055_parity_tests.rs`, on the
/// close-up R of §2.5. What this test covers that the other cannot is the plumbing at the frame's
/// own scale: the mask pass opened over 1 926 opaque runs' worth of frame, the trace reading a
/// mask of 307 200 zero texels, and the three passes submitted for it.
#[test]
fn req_scn_084_the_reflections_agree_between_the_backends() {
    let Some((share, channel)) = parity("reflections", &REFLECTIONS) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the reflections agree on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "reflections: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// And the measurement that says why: on **either** backend, turning the reflections on leaves the
/// whole synthetic set untouched, because not one instance of it is a floor surface — a floor needs
/// a bounding box of at least `6 × échelle de scène` in both dimensions of the floor plane *and* a
/// face whose scene normal is `+Y ≥ 0,9`, and the backdrop, the widest flat thing in the set, is
/// 256 × 240 × **1** (Q-093 (a) records exactly that reading).
///
/// So this is a **negative** parity test, and it is worth having: a port whose mask marked a face
/// it should not, or whose trace returned something where the mask is 0, would reflect a wall or
/// tint the frame here. That is not hypothetical — the first version of this step did mark an extra
/// face, from a depth bias written with its two arguments crossed over.
#[test]
fn req_scn_084_nothing_of_the_synthetic_set_is_a_floor() {
    let classic = capture("floorless-classic", None, &[]).expect("OpenGL always renders");
    let reflected = capture("floorless-ssr", None, &REFLECTIONS).expect("OpenGL always renders");
    for scene in SCENES {
        let figures = figures(&load(&classic, scene), &load(&reflected, scene));
        assert_eq!(
            figures.max_channel, 0,
            "REQ-SCN-084 moved `{scene}` on OpenGL by up to {} levels, so the set now has a floor \
             and this file's reading of it is stale",
            figures.max_channel
        );
    }

    for backend in backends() {
        let Some(flat) = capture("floorless-classic", Some(backend), &[]) else {
            continue;
        };
        let on = capture("floorless-ssr", Some(backend), &REFLECTIONS).unwrap();
        for scene in SCENES {
            let figures = figures(&load(&flat, scene), &load(&on, scene));
            println!(
                "reflections on {backend}, `{scene}`: max {} levels, {:.4} % of pixels differ",
                figures.max_channel,
                figures.differing * 100.0
            );
            assert!(
                figures.differing < 0.0002,
                "the reflections moved {:.4} % of `{scene}` on {backend}, where there is no floor",
                figures.differing * 100.0
            );
        }
        fs::remove_dir_all(&flat).unwrap();
        fs::remove_dir_all(&on).unwrap();
    }
    fs::remove_dir_all(&classic).unwrap();
    fs::remove_dir_all(&reflected).unwrap();
}

/// What the reflections asked of each backend, read off `render.json`: three passes — the mask,
/// the trace and the composite — and the same target bytes, with the one extra pass this API needs
/// for the depth resolve when the frame is multisampled.
#[test]
fn req_scn_084_the_reflections_cost_the_same_passes_and_bytes_on_each_backend() {
    for (level, extra) in [("0", 0_u32), ("1", 1)] {
        let mut arguments = vec!["--anti-alias", level];
        arguments.extend_from_slice(&REFLECTIONS);
        let reference = capture("ssr-counts", None, &arguments).expect("OpenGL always renders");
        let counters = |folder: &PathBuf| {
            let records = read_render(&folder.join("render.json")).unwrap();
            let scene = records
                .iter()
                .find(|record| record.scene == "framing")
                .expect("the synthetic set has a `framing` scene");
            (
                scene.screen_effect_passes.expect("Enhanced writes the key"),
                scene.screen_effect_bytes.expect("Enhanced writes the key"),
            )
        };
        let (gl_passes, gl_bytes) = counters(&reference);
        assert_eq!(gl_passes, 3, "the mask, the trace and the composite");
        for backend in backends() {
            let Some(capture) = capture("ssr-counts", Some(backend), &arguments) else {
                continue;
            };
            let (passes, bytes) = counters(&capture);
            println!(
                "reflections, anti_alias {level} on {backend}: {passes} passes and {bytes} bytes, \
                 OpenGL {gl_passes} and {gl_bytes}"
            );
            assert_eq!(
                passes,
                gl_passes + extra,
                "anti_alias {level} on {backend} submitted {passes} screen passes, OpenGL \
                 {gl_passes} plus the {extra} this API needs for the depth resolve"
            );
            assert_eq!(
                bytes, gl_bytes,
                "anti_alias {level} on {backend} allocated {bytes} bytes of screen targets, \
                 OpenGL {gl_bytes}"
            );
            fs::remove_dir_all(&capture).unwrap();
        }
        fs::remove_dir_all(&reference).unwrap();
    }
}

// ------------------------------------------- AMD-049, the backdrop lighting of REQ-SCN-075

/// The scenes the backdrop model is read on: the one built for it, and three it must leave
/// **untouched**.
///
/// Measured rather than assumed, as the occlusion's own scene list was: on the OpenGL backend
/// `render.backdrop_lighting` moves **41.07 %** of `backdrop` and **0.0000 %** of every other
/// capture of the set — including `shadow`, `no-shadow` and `layer-shadow-off`, which draw no
/// backdrop of their own, and `hud`, which has none either. So the test below reads the first for
/// the effect and the rest as a control, which is what pins "no run but the backdrop's".
const BACKDROP_SCENES: [&str; 4] = ["backdrop", "shadow", "heavy", "hud"];

/// AMD-049 REQ-SCN-075: the almost-matt reflection model of the backdrop, which is **on by
/// default** on the desktop and which this backend did not render at all before this step.
///
/// Step 1 of this lot measured it and recorded it as out of scope: rendered with the model on, the
/// two backends agreed on **58.930 %** of the `backdrop` scene. The owner put it into the lot
/// between the two sessions, and this is the test that holds the result.
#[test]
fn req_scn_075_the_backdrop_lighting_agrees_between_the_backends() {
    let Some((share, channel)) = parity_lit("backdrop-lit", &[]) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the lit backdrop agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "backdrop lighting: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// The same with the curvature and the voxel grid, which are the backdrop variant's two axes
/// (REQ-SCN-068 bends the box, and AMD-055 §3 point 5 puts the grid on its faces) — so this is
/// the configuration that exercises `backdrop_bend_grid`, the fourth and last of them.
#[test]
fn req_scn_075_the_lit_backdrop_agrees_under_curvature_and_the_grid() {
    let Some((share, _)) = parity_lit(
        "backdrop-bend-grid",
        &[
            "--bend",
            "45,0,0",
            "--style",
            "enhanced",
            "--style-param",
            "spl.voxel_grid=3",
        ],
    ) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the lit backdrop under curvature and the grid agrees on only {:.4} % of pixels",
        share * 100.0
    );
}

/// And the model has to **do** something on the `wgpu` side, and the same something: the scenes
/// that have a backdrop move with it, by as much as they do on OpenGL, and the ones that have none
/// do not move at all.
///
/// This is the half of the check the share cannot make, and the half that would have caught the
/// gap step 1 found: a backend that ignored `render.backdrop_lighting` would read 100 % here on
/// every scene and would have been wrong on five of them.
#[test]
fn req_scn_075_the_backdrop_lighting_moves_as_much_of_the_frame_on_each_backend() {
    let plain = capture("backdrop-plain", None, &[]).expect("OpenGL always renders");
    let lit = capture_lit("backdrop-lit-moves", None, &[]).expect("OpenGL always renders");
    let reference: Vec<f64> = BACKDROP_SCENES
        .iter()
        .map(|scene| figures(&load(&plain, scene), &load(&lit, scene)).differing)
        .collect();
    for (scene, moved) in BACKDROP_SCENES.iter().zip(&reference) {
        println!(
            "the OpenGL backdrop model moves {:.4} % of `{scene}`",
            moved * 100.0
        );
    }
    assert!(
        reference[0] > 0.10,
        "the OpenGL model moved only {:.4} % of `backdrop`",
        reference[0] * 100.0
    );
    for (scene, moved) in BACKDROP_SCENES.iter().zip(&reference).skip(1) {
        assert_eq!(
            0.0, *moved,
            "`{scene}` draws no backdrop, so REQ-SCN-075 must leave it alone on OpenGL too"
        );
    }

    for backend in backends() {
        let Some(flat) = capture("backdrop-plain", Some(backend), &[]) else {
            continue;
        };
        let on = capture_lit("backdrop-lit-moves", Some(backend), &[]).unwrap();
        for (scene, theirs) in BACKDROP_SCENES.iter().zip(&reference) {
            let moved = figures(&load(&flat, scene), &load(&on, scene)).differing;
            println!(
                "the model on {backend} moves {:.4} % of `{scene}`, OpenGL {:.4} %",
                moved * 100.0,
                theirs * 100.0
            );
            assert!(
                (moved - theirs).abs() < 0.01,
                "the model on {backend} moved {:.4} % of `{scene}` against OpenGL's {:.4} %",
                moved * 100.0,
                theirs * 100.0
            );
        }
        fs::remove_dir_all(&flat).unwrap();
        fs::remove_dir_all(&on).unwrap();
    }
    fs::remove_dir_all(&plain).unwrap();
    fs::remove_dir_all(&lit).unwrap();
}

// ------------------------------------------------------------- AMD-032, the anti-aliasing levels

/// AMD-032 §2 and AMD-052 REQ-PERS-089: `video.anti_alias` 0, 1 and 2 mean no anti-aliasing,
/// MSAA 4x and MSAA 8x, and the `wgpu` backend grants exactly what the OpenGL one grants — which
/// is the normative half of the brief's point 5, and the half that is an equality rather than a
/// share.
#[test]
fn amd032_each_anti_alias_level_grants_the_same_samples_on_every_backend() {
    for (level, wanted) in [("0", 1_u32), ("1", 4), ("2", 8)] {
        let reference =
            capture("aa", None, &["--anti-alias", level]).expect("OpenGL always renders");
        let granted = |folder: &PathBuf| {
            let records = read_render(&folder.join("render.json")).unwrap();
            let samples: Vec<u32> = records.iter().map(|r| r.anti_alias_samples).collect();
            assert!(
                samples.iter().all(|&s| s == samples[0]),
                "the level is one per run, not one per scene"
            );
            samples[0]
        };
        let opengl = granted(&reference);
        assert_eq!(
            opengl, wanted,
            "video.anti_alias = {level} on OpenGL granted {opengl} samples"
        );
        for backend in backends() {
            let Some(capture) = capture("aa", Some(backend), &["--anti-alias", level]) else {
                continue;
            };
            let mine = granted(&capture);
            println!("video.anti_alias = {level}: {backend} granted {mine}, OpenGL {opengl}");
            assert_eq!(
                mine, opengl,
                "video.anti_alias = {level} on {backend} granted {mine}, OpenGL {opengl}"
            );
            fs::remove_dir_all(&capture).unwrap();
        }
        fs::remove_dir_all(&reference).unwrap();
    }
}

/// What the two multisampled levels are held to against OpenGL, and why it is not [`WITHIN_TWO`].
///
/// **MSAA sample positions are implementation-defined in every one of the three APIs.** Neither
/// OpenGL nor Vulkan nor DirectX 12 fixes where inside a pixel the 4 or 8 samples sit, so two
/// correct implementations resolve an edge pixel to different colours. The measurement that
/// settles it is in this file's own output: under MSAA 4x on `heavy` — 1926 overlapping tile
/// boxes, so silhouette edges everywhere — **Vulkan and DX12 agree with each other on 100.000 %
/// of the pixels** and both differ from OpenGL on 0.892 %. One pattern on the two `wgpu` APIs,
/// another in the OpenGL driver, and nothing in the renderer to choose between them.
///
/// Measured here, worst scene of four: **99.1084 %** at MSAA 4x and **98.9489 %** at MSAA 8x,
/// both on `heavy`, and the more samples the further the two patterns can diverge. The floor is
/// set just under the worse of the two rather than at a round number.
///
/// So the multisampled rows are held to this against OpenGL and to [`AGREE`] against each other,
/// which is the stronger of the two assertions and the one a port can actually break.
const WITHIN_TWO_MULTISAMPLED: f64 = 0.985;

/// What the two `wgpu` backends are held to against **each other**: they run the same WGSL
/// through the same naga, so a difference between them is a bug in one driver or in the port's
/// use of a feature one of them reads differently — which is how step 3 found `fwidth`.
const AGREE: f64 = 0.999;

/// AMD-032: each level's picture, compared with OpenGL's and between the two `wgpu` backends.
/// AMD-058 REQ-SCN-065 is in this too — the centroid variant exists only for the multisampled
/// levels, so this is where it is read.
#[test]
fn amd032_the_anti_alias_levels_agree_between_the_backends() {
    for (level, floor) in [
        ("0", WITHIN_TWO),
        ("1", WITHIN_TWO_MULTISAMPLED),
        ("2", WITHIN_TWO_MULTISAMPLED),
    ] {
        let Some((share, channel)) = parity(
            &format!("aa-{level}"),
            &["--anti-alias", level, "--style", "enhanced"],
        ) else {
            return;
        };
        println!(
            "video.anti_alias = {level}: worst {:.4} % within 2 of OpenGL, max {channel} levels",
            share * 100.0
        );
        assert!(
            share >= floor,
            "video.anti_alias = {level} agrees with OpenGL on only {:.4} % of pixels",
            share * 100.0
        );
    }
}

/// And the two `wgpu` backends against each other, at every level: the same WGSL through the same
/// naga, so this is the assertion that holds tightest and the one that caught the `fwidth` of
/// step 3.
///
/// The occlusion of REQ-SCN-083 rides along, which makes this the strongest check the screen
/// passes get: at level 1 and 2 it exercises the depth resolve of REQ-SCN-080 — the pass with no
/// OpenGL counterpart — and a difference between the two APIs there would be a driver's or the
/// port's, not a sample pattern's.
#[cfg(windows)]
#[test]
fn ru_7_vulkan_and_dx12_agree_with_each_other_at_every_level() {
    for level in ["0", "1", "2"] {
        let arguments = [
            "--anti-alias",
            level,
            "--style",
            "enhanced",
            "--style-param",
            "spl.ao=2",
        ];
        let Some(vulkan) = capture("pair", Some("vulkan"), &arguments) else {
            return;
        };
        let Some(dx12) = capture("pair", Some("dx12"), &arguments) else {
            return;
        };
        for scene in SCENES {
            let figures = figures(&load(&vulkan, scene), &load(&dx12, scene));
            println!(
                "anti_alias {level} {scene:<10} vulkan vs dx12: max {:>3} levels, {:>8.4} % within 2",
                figures.max_channel,
                figures.within_two * 100.0
            );
            assert!(
                figures.within_two >= AGREE,
                "anti_alias {level}, {scene}: the two wgpu backends agree on only {:.4} %",
                figures.within_two * 100.0
            );
        }
        fs::remove_dir_all(&vulkan).unwrap();
        fs::remove_dir_all(&dx12).unwrap();
    }
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
