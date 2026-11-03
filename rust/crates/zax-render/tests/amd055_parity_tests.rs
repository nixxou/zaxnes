//! RU-7: the Enhanced style of AMD-055 drawn through **both** backends on one scene, where the
//! harness's synthetic set cannot reach it.
//!
//! `zax-harness/tests/parity_wgpu_tests.rs` is the parity file of this lot and it covers every
//! effect the synthetic set exercises. The reflections of REQ-SCN-084 are not one of them, and the
//! measurement is in this file's own output: on the OpenGL backend, `spl.reflections` = 1 against
//! Classic leaves **all eleven synthetic captures byte-identical**. That is the floor definition
//! working as written — a floor surface needs a bounding box of at least `6 × échelle de scène` in
//! both dimensions of the floor plane *and* a face whose scene normal is `+Y ≥ 0,9`, and no
//! instance of the set has both (the backdrop is 256 × 240 × **1**, which Q-093 (a) already
//! records as not a floor).
//!
//! So the reflections need the close-up R of §2.5, which is a fixture this crate's own tests build
//! (`common::close_up_r`) and the harness cannot. Hence this file: the same `Built` scene rendered
//! by [`common::GlFixture`] and by [`common::WgpuFixture`], and the two pictures compared.
//!
//! A machine with no Vulkan and no DirectX 12 adapter skips, with the reason printed, as every
//! `wgpu` test of this workspace does.
#![cfg(feature = "wgpu")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{close_up_r, Built, Capture, GlFixture, WgpuFixture};
use zax_render::style::{AmbientOcclusion, StyleSettings};
use zax_render::wgpu_backend::headless::GpuApi;

/// The bar a configuration **without** the reflections is held to: the bar of
/// `zax-harness/tests/render_wgpu_tests.rs`, which is the one the flat Classic frame already meets
/// on this hardware.
const WITHIN_TWO: f64 = 0.995;

/// What a configuration **with** the reflections is held to against OpenGL, and why it is looser
/// than [`WITHIN_TWO`].
///
/// The reflections are the first effect of this lot whose per-pixel result is **discontinuous in
/// the depth buffer**. Every other one — the curvature, the voxel grid, the pixel shadows, even
/// the occlusion — is a smooth function of the frame, so two correct implementations differ by
/// their compilers' rounding. A screen-space march is not: the hit is decided by `>` comparisons
/// on reconstructed view depths, and one step of difference in where the bisection lands makes the
/// pixel reflect a *different texel of the frame*. On this scene that texel is often across the
/// boundary between the red cube and the grey floor, so the colour jump is large even though the
/// march agreed almost everywhere.
///
/// Measured here, and the measurement is in
/// [`req_scn_084_the_two_backends_reflect_the_same_pixels`], which is the assertion that actually
/// constrains the port: **5 259 pixels are reflected by both backends, 204 by OpenGL alone and
/// 241 by Vulkan alone** — a symmetric fringe of 4 % — and inside the common set 86 % agree to
/// within 2 levels with the worst at 35, which is well under the 118 the *unreflected* control
/// already carries on this scene. Whole-frame shares: **99.5999 %** at `ssr.quality` High (half
/// resolution) and **99.8493 %** at Ultra, the difference being the half-resolution trace sampled
/// back up.
const WITHIN_TWO_REFLECTED: f64 = 0.990;

/// What two captures of one scene differ by.
#[derive(Debug, Clone, Copy)]
struct Figures {
    max_channel: u8,
    differing: f64,
    within_two: f64,
}

fn figures(a: &Capture, b: &Capture) -> Figures {
    assert_eq!(a.pixels.len(), b.pixels.len(), "captures differ in size");
    let mut max_channel = 0_u8;
    let mut differing = 0_u64;
    let mut within = 0_u64;
    let mut pixels = 0_u64;
    for (left, right) in a.pixels.chunks_exact(4).zip(b.pixels.chunks_exact(4)) {
        pixels += 1;
        let mut worst = 0_u8;
        for channel in 0..3 {
            worst = worst.max(left[channel].abs_diff(right[channel]));
        }
        max_channel = max_channel.max(worst);
        differing += u64::from(worst > 0);
        within += u64::from(worst <= 2);
    }
    #[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^53.
    Figures {
        max_channel,
        differing: differing as f64 / pixels as f64,
        within_two: within as f64 / pixels as f64,
    }
}

/// The APIs to compare against OpenGL: Vulkan, and DirectX 12 on Windows.
fn apis() -> Vec<(&'static str, GpuApi)> {
    #[cfg(windows)]
    {
        vec![("vulkan", GpuApi::Vulkan), ("dx12", GpuApi::Dx12)]
    }
    #[cfg(not(windows))]
    {
        vec![("vulkan", GpuApi::Vulkan)]
    }
}

/// Renders `built` on the OpenGL backend and on each reachable `wgpu` one, prints the figures and
/// hands back the worst `within_two` and the largest `max_channel`. `None` when no adapter
/// answers here.
///
/// The OpenGL capture is taken first and once: it is the reference, and the lot's rule is that it
/// is also the definition of the behaviour.
fn parity(label: &str, built: &Built) -> Option<(f64, u8)> {
    let gl = GlFixture::new();
    let reference = gl.render_built(&mut gl.renderer(), built);

    let mut worst_share = 1.0_f64;
    let mut worst_channel = 0_u8;
    let mut any = false;
    for (name, api) in apis() {
        let Some(mut fixture) = WgpuFixture::new(api) else {
            continue;
        };
        any = true;
        let capture = fixture.render_built(built);
        let figures = figures(&capture, &reference);
        println!(
            "{label:24} {name:7} max {:>3} levels, {:>7.4} % of pixels differ, {:>8.4} % within 2",
            figures.max_channel,
            figures.differing * 100.0,
            figures.within_two * 100.0
        );
        worst_share = worst_share.min(figures.within_two);
        worst_channel = worst_channel.max(figures.max_channel);
    }
    any.then_some((worst_share, worst_channel))
}

/// The two `wgpu` APIs against **each other** on `built`: they run the same WGSL through the same
/// naga, so a difference between them is a driver's or the port's use of a feature one of them
/// reads differently. The tightest assertion this lot has, and the one that caught `fwidth` in
/// step 3.
#[cfg(windows)]
fn pair(label: &str, built: &Built) -> Option<Figures> {
    let mut vulkan = WgpuFixture::new(GpuApi::Vulkan)?;
    let one = vulkan.render_built(built);
    drop(vulkan);
    let mut dx12 = WgpuFixture::new(GpuApi::Dx12)?;
    let two = dx12.render_built(built);
    let figures = figures(&one, &two);
    println!(
        "{label:24} vulkan vs dx12: max {:>3} levels, {:>8.4} % within 2",
        figures.max_channel,
        figures.within_two * 100.0
    );
    Some(figures)
}

/// Enhanced with the reflections on and every other effect at the default of REQ-SCN-079.
const REFLECTIONS: StyleSettings = StyleSettings {
    enhanced: true,
    reflections: true,
    ao: AmbientOcclusion::Off,
    ao_strength: 3,
    ao_radius: 3,
    ao_thickness: 3,
    ao_quality: 1,
    pixel_shadow: false,
    voxel_grid: 0,
    ssr_shine: 3,
    ssr_clarity: 3,
    ssr_length: 3,
    ssr_quality: 2,
};

/// The close-up R with the reflections off, as the control every row below is read against.
#[test]
fn ru_7_the_close_up_r_agrees_between_the_backends_with_the_reflections_off() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = StyleSettings {
        reflections: false,
        ..REFLECTIONS
    };
    let Some((share, channel)) = parity("control, no reflection", &built) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO,
        "the control agrees on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "control: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// AMD-055 REQ-SCN-084: the whole chain — the floor mask laid down by the scene's own geometry,
/// the march, the bisection, the thickness test, the blur and the two attenuations — agrees with
/// OpenGL on the one frame of this workspace it acts on.
#[test]
fn req_scn_084_the_reflections_agree_between_the_backends() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = REFLECTIONS;
    let Some((share, channel)) = parity("reflections", &built) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO_REFLECTED,
        "the reflections agree on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "reflections: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// REQ-SCN-084 at `ssr.quality` Ultra, where the trace runs at the frame's **own** resolution and
/// the composite's upsample is out of the picture — the divisor a port can get right at one
/// quality and wrong at the other, as `ao.quality` was for the occlusion.
#[test]
fn req_scn_084_the_full_resolution_trace_agrees_between_the_backends() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = StyleSettings {
        ssr_quality: 3,
        ..REFLECTIONS
    };
    let Some((share, _)) = parity("reflections, Ultra", &built) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO_REFLECTED,
        "the Ultra trace agrees on only {:.4} % of pixels",
        share * 100.0
    );
}

/// The reflections have to **do** something on the `wgpu` side, and the same something — the half
/// of the check a share cannot make, and the half the synthetic set could not make at all.
///
/// Measured: the OpenGL backend moves **1.78 %** of the capture — 5 463 pixels of the floor at the
/// foot of the cube and of the cube's own top face, which the extent test also admits (8 units in
/// both dimensions of the floor plane, over the 6 the requirement asks). The floor of this
/// assertion is set under that, and the two backends are held to within a point of each other.
#[test]
fn req_scn_084_the_reflections_change_as_much_of_the_frame_on_each_backend() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = StyleSettings {
        reflections: false,
        ..REFLECTIONS
    };
    let off = gl.render_built(&mut renderer, &built);
    built.scene.settings.style = REFLECTIONS;
    let on = gl.render_built(&mut renderer, &built);
    let reference = figures(&off, &on).differing;
    println!(
        "the OpenGL reflections move {:.4} % of the close-up R",
        reference * 100.0
    );
    assert!(
        reference > 0.01,
        "the OpenGL reflections moved only {:.4} % of the close-up R, so this fixture no longer \
         carries the effect and the figures below mean nothing",
        reference * 100.0
    );

    for (name, api) in apis() {
        let Some(mut fixture) = WgpuFixture::new(api) else {
            continue;
        };
        built.scene.settings.style = StyleSettings {
            reflections: false,
            ..REFLECTIONS
        };
        let off = fixture.render_built(&built);
        built.scene.settings.style = REFLECTIONS;
        let on = fixture.render_built(&built);
        let moved = figures(&off, &on).differing;
        println!(
            "{name}: the reflections move {:.4} % of the close-up R, OpenGL {:.4} %",
            moved * 100.0,
            reference * 100.0
        );
        assert!(
            (moved - reference).abs() < 0.01,
            "the reflections on {name} moved {:.4} % against OpenGL's {:.4} %",
            moved * 100.0,
            reference * 100.0
        );
    }
}

/// **The assertion that actually constrains the port**, and the one the whole-frame share cannot
/// make: the two backends must reflect the **same pixels**, and agree on their colour inside that
/// set.
///
/// A reflection is a discontinuous function of the depth buffer, so a whole-frame share mixes two
/// very different things — a march that found a different surface, and a march that found the same
/// one and read a neighbouring texel. This separates them. Measured here:
///
/// | | pixels |
/// |---|---|
/// | reflected by **both** backends | 5 259 |
/// | reflected by OpenGL alone | 204 |
/// | reflected by Vulkan alone | 241 |
/// | of the common set, differing by more than 2 levels | 740 (worst 35) |
/// | outside every reflection yet differing by more than 2 | 44 — the control's own silhouette noise |
///
/// The fringe is **symmetric** (204 against 241), which is what says it is the march's hit/miss
/// boundary rather than one backend reflecting something the other does not see: a mask that
/// marked an extra face, or a ray sent the wrong way by the `y` mirror of `screen_depth.wgsl`,
/// would put the whole of that face on one side of the table.
///
/// The worst difference inside the common set, 35 levels, is far under the **118** the *unreflected*
/// control already carries on this scene — so the reflections are not where this frame's largest
/// disagreement is.
#[test]
fn req_scn_084_the_two_backends_reflect_the_same_pixels() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let off = StyleSettings {
        reflections: false,
        ..REFLECTIONS
    };
    built.scene.settings.style = off;
    let gl_off = gl.render_built(&mut renderer, &built);
    built.scene.settings.style = REFLECTIONS;
    let gl_on = gl.render_built(&mut renderer, &built);

    for (name, api) in apis() {
        let Some(mut fixture) = WgpuFixture::new(api) else {
            continue;
        };
        built.scene.settings.style = off;
        let mine_off = fixture.render_built(&built);
        built.scene.settings.style = REFLECTIONS;
        let mine_on = fixture.render_built(&built);

        let moved = |a: &Capture, b: &Capture, x: i32, y: i32| {
            let p = a.at(x, y);
            let q = b.at(x, y);
            (p.0 - q.0)
                .abs()
                .max((p.1 - q.1).abs())
                .max((p.2 - q.2).abs())
        };
        let (mut both, mut gl_only, mut mine_only) = (0_u64, 0_u64, 0_u64);
        let (mut apart, mut worst) = (0_u64, 0_i32);
        for y in 0..gl_off.height {
            for x in 0..gl_off.width {
                let theirs = moved(&gl_off, &gl_on, x, y) > 0;
                let ours = moved(&mine_off, &mine_on, x, y) > 0;
                match (theirs, ours) {
                    (true, true) => {
                        both += 1;
                        let delta = moved(&gl_on, &mine_on, x, y);
                        if delta > 2 {
                            apart += 1;
                            worst = worst.max(delta);
                        }
                    }
                    (true, false) => gl_only += 1,
                    (false, true) => mine_only += 1,
                    (false, false) => {}
                }
            }
        }
        println!(
            "{name}: {both} reflected by both, {gl_only} by OpenGL alone, {mine_only} by {name} alone"
        );
        println!("    {apart} of the common set differ by more than 2 levels, worst {worst}");

        assert!(both > 4_000, "only {both} pixels reflect on both backends");
        // The fringe either way, as a share of the common set. A face marked on one backend and
        // not the other, or a ray sent the wrong way, would blow one of these two past the other.
        #[allow(clippy::cast_precision_loss)] // Counts far below 2^53.
        let (theirs, ours) = (gl_only as f64 / both as f64, mine_only as f64 / both as f64);
        println!(
            "    fringe: OpenGL {:.2} %, {name} {:.2} %",
            theirs * 100.0,
            ours * 100.0
        );
        assert!(
            theirs < 0.08 && ours < 0.08,
            "the hit/miss fringe is {:.2} % on OpenGL and {:.2} % on {name}, which is no longer a              fringe",
            theirs * 100.0,
            ours * 100.0
        );
        assert!(
            (theirs - ours).abs() < 0.03,
            "the fringe is {:.2} % on OpenGL against {:.2} % on {name}: one backend reflects              something the other does not see, which is a mask or a ray direction and not a              boundary case",
            theirs * 100.0,
            ours * 100.0
        );
        #[allow(clippy::cast_precision_loss)]
        let apart_share = apart as f64 / both as f64;
        assert!(
            apart_share < 0.25,
            "{:.2} % of the commonly reflected pixels differ by more than 2 levels on {name}",
            apart_share * 100.0
        );
    }
}

/// REQ-SCN-080 with the reflections **and** the occlusion, which is the only configuration where
/// the frame order the requirement fixes is observable: the reflections run first, on the scene
/// the occlusion has not yet darkened, and the occlusion then darkens the composited result.
///
/// A port that ran them the other way round would darken the surface, reflect the darkened
/// surface, and come out lighter at the contact — which is where this scene has its reflection.
#[test]
fn req_scn_080_the_reflections_run_before_the_occlusion_on_both_backends() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = StyleSettings {
        ao: AmbientOcclusion::Depth,
        ..REFLECTIONS
    };
    let Some((share, channel)) = parity("reflections + occlusion", &built) else {
        return;
    };
    assert!(
        share >= WITHIN_TWO_REFLECTED,
        "the two effects together agree on only {:.4} % of pixels",
        share * 100.0
    );
    println!(
        "reflections + occlusion: worst {:.4} % within 2, max {channel} levels",
        share * 100.0
    );
}

/// And the two `wgpu` APIs against each other, with the reflections on: the same WGSL through the
/// same naga, so this is the assertion that holds tightest.
#[cfg(windows)]
#[test]
fn ru_7_vulkan_and_dx12_agree_on_the_reflections() {
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.style = REFLECTIONS;
    let Some(figures) = pair("reflections", &built) else {
        return;
    };
    assert!(
        figures.within_two >= 0.999,
        "the two wgpu APIs agree on only {:.4} % of the reflected frame",
        figures.within_two * 100.0
    );
}
