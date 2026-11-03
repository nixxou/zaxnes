//! AMD-055 REQ-SCN-078 and REQ-SCN-080: what the Enhanced style costs when nothing of it is on.
//!
//! The brief of this lot has two performance requirements this file is the home of:
//!
//! 1. **Classic costs exactly what it costs today** — no extra pass, framebuffer, texture or
//!    buffer, no extra uniform upload, no per-fragment branch;
//! 2. **each effect pays only when it is on** — with `spl.ao`, `spl.pixel_shadow`,
//!    `spl.voxel_grid` and `spl.reflections` all 0, which are the defaults of REQ-SCN-079,
//!    Enhanced must cost what Classic costs.
//!
//! Both are measured rather than claimed, through the three counters of `RenderStatistics` that
//! exist for it (`style_programs`, `screen_effect_bytes`, `screen_effect_passes`) and through the
//! capture itself. The requirement's own wording for the picture is REQ-SCN-080: "Tous les effets
//! désactivés, une capture Enhanced est identique à une capture Classic."
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::style::{AmbientOcclusion, StyleSettings};

/// A lit scene with shadows: a block over a floor, the shape every criterion of §2.3 and §2.4
/// measures on.
fn scene() -> common::Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(2, 0x21)
        .pattern(solid(1, 1), 8, ProfileLayer::L1)
        .pattern(solid(2, 2), 1, ProfileLayer::L2)
        .rect(solid(2, 2), 0, 0, 32, 30)
        .rect(solid(1, 1), 14, 12, 4, 4)
        .build()
}

/// REQ-SCN-080: Enhanced with every effect off is the Classic picture. Asserted **at tolerance
/// 0** — the requirement allows ±2 sRGB levels per channel, and the design has to do better than
/// that, because what draws the frame is the very same program with the very same uniforms.
#[test]
fn req_scn_080_enhanced_with_every_effect_off_is_the_classic_picture() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let mut built = scene();

    assert!(!built.scene.settings.style.enhanced, "Classic by default");
    let classic = fixture.render_built(&mut renderer, &built);

    built.scene.settings.style.enhanced = true;
    let enhanced = fixture.render_built(&mut renderer, &built);

    assert_eq!(
        classic.pixels, enhanced.pixels,
        "REQ-SCN-080: every effect off, Enhanced is Classic to the pixel"
    );

    // And back again, so that the style is not a one-way door.
    built.scene.settings.style.enhanced = false;
    let again = fixture.render_built(&mut renderer, &built);
    assert_eq!(classic.pixels, again.pixels);
}

/// Point 1 and point 2 of the brief, as counters: nothing of AMD-055 is compiled, allocated or
/// submitted in Classic, nor in Enhanced while the four switches are at their defaults.
#[test]
fn req_scn_080_enhanced_with_every_effect_off_allocates_and_submits_nothing() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let mut built = scene();

    fixture.render_built(&mut renderer, &built);
    let classic = renderer.statistics().clone();
    assert_eq!(0, classic.style_programs, "Classic compiles no variant");
    assert_eq!(0, classic.screen_effect_bytes, "and allocates no target");
    assert_eq!(0, classic.screen_effect_passes, "and submits no pass");

    built.scene.settings.style.enhanced = true;
    fixture.render_built(&mut renderer, &built);
    let enhanced = renderer.statistics().clone();
    assert_eq!(0, enhanced.style_programs);
    assert_eq!(0, enhanced.screen_effect_bytes);
    assert_eq!(0, enhanced.screen_effect_passes);

    // The frame is also the same work: the same draws, the same triangles, the same meshes.
    assert_eq!(classic.draw_calls, enhanced.draw_calls);
    assert_eq!(classic.batches, enhanced.batches);
    assert_eq!(classic.triangles, enhanced.triangles);
    assert_eq!(classic.resident_meshes, enhanced.resident_meshes);
    assert_eq!(classic.mesh_uploads, enhanced.mesh_uploads);
    assert_eq!(classic.shadow_casters, enhanced.shadow_casters);
}

/// REQ-SCN-078: in Classic the four parameters cannot act, whatever they hold. A player who goes
/// back to Classic with every effect turned up keeps his settings and pays nothing — which is
/// what makes the style the single gate (§3 point 4).
#[test]
fn req_scn_078_classic_with_every_parameter_turned_up_is_the_classic_picture() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let mut built = scene();

    let plain = fixture.render_built(&mut renderer, &built);

    built.scene.settings.style = StyleSettings {
        enhanced: false,
        ao: AmbientOcclusion::Depth,
        ao_strength: 5,
        ao_radius: 5,
        pixel_shadow: true,
        voxel_grid: 3,
        reflections: true,
        ssr_shine: 5,
        ssr_quality: 3,
        ..StyleSettings::default()
    };
    let loaded = fixture.render_built(&mut renderer, &built);

    assert_eq!(plain.pixels, loaded.pixels);
    let statistics = renderer.statistics();
    assert_eq!(0, statistics.style_programs);
    assert_eq!(0, statistics.screen_effect_bytes);
    assert_eq!(0, statistics.screen_effect_passes);
}

/// AMD-055 §3 point 4 on the GPU: the curvature acts in Enhanced exactly as in Classic, so a
/// change of style moves no vertex. The two captures are compared at tolerance 0 in both styles.
#[test]
fn req_scn_078_the_curvature_acts_in_both_styles() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let mut built = scene();
    built.scene.bend = zax_engine3d::space_bend::SpaceBend::new(60.0, 0.0, 0.0, 0.0, 1.0);

    let classic = fixture.render_built(&mut renderer, &built);
    built.scene.settings.style.enhanced = true;
    let enhanced = fixture.render_built(&mut renderer, &built);

    assert_eq!(
        classic.pixels, enhanced.pixels,
        "a bent scene draws the same in both styles"
    );

    // And the curvature really is in effect, so the comparison is worth something.
    built.scene.bend = zax_engine3d::space_bend::SpaceBend::NONE;
    let flat = fixture.render_built(&mut renderer, &built);
    assert_ne!(classic.pixels, flat.pixels, "60° changes the picture");
}

/// Q-104, answered (RU-7): **every effect of the Enhanced style of AMD-055 is in the `wgpu`
/// backend**, down to the multi-scale estimator of REQ-SCN-083's *Mode MSVO* and its fallback.
///
/// This test used to assert that not one of them was in the WGSL — the known gap, so that a
/// half-done parity failed loudly rather than passing unnoticed. Step 3 ported the voxel grid of
/// REQ-SCN-081 and the pixel shadows of REQ-SCN-082; step 4b the occlusion of REQ-SCN-083 and the
/// architecture its passes live in; step 4c the reflections of REQ-SCN-084 and the floor mask; step
/// 4d the multi-scale chain. The diagnostic line of `wgpu_backend/renderer.rs` went with 4c, as
/// this test's own message had instructed, and the list of what must *not* exist is empty — so what
/// is left here is the **positive** half: each effect's own constants, named one by one, so that a
/// change which quietly drops one of them fails.
///
/// Whether the WGSL compiles and whether the two backends agree is not a question this side can
/// answer — naga validates at module creation. That is
/// `zax-harness/tests/parity_wgpu_tests.rs` and `amd055_parity_tests.rs`.
#[cfg(feature = "wgpu")]
#[test]
// One assertion per constant of four amendments; a shorter body would say less.
#[allow(clippy::too_many_lines)]
fn q_104_the_wgpu_backend_carries_every_effect_of_the_enhanced_style() {
    const COMMON: &str = include_str!("../src/wgpu_backend/shaders/common.wgsl");
    const SCENE: &str = include_str!("../src/wgpu_backend/shaders/scene.wgsl");
    const SHADOW: &str = include_str!("../src/wgpu_backend/shaders/shadow.wgsl");
    const RESOLVE: &str = include_str!("../src/wgpu_backend/shaders/resolve.wgsl");
    const VOXEL_GRID: &str = include_str!("../src/wgpu_backend/shaders/voxel_grid.wgsl");
    const PIXEL_SHADOW: &str = include_str!("../src/wgpu_backend/shaders/pixel_shadow.wgsl");
    const SCREEN_DEPTH: &str = include_str!("../src/wgpu_backend/shaders/screen_depth.wgsl");
    const AO: &str = include_str!("../src/wgpu_backend/shaders/ao.wgsl");
    const AO_BLUR: &str = include_str!("../src/wgpu_backend/shaders/ao_blur.wgsl");
    const AO_COMPOSITE: &str = include_str!("../src/wgpu_backend/shaders/ao_composite.wgsl");
    const DEPTH_RESOLVE: &str = include_str!("../src/wgpu_backend/shaders/depth_resolve.wgsl");
    const SSR: &str = include_str!("../src/wgpu_backend/shaders/ssr.wgsl");
    const SSR_COMPOSITE: &str = include_str!("../src/wgpu_backend/shaders/ssr_composite.wgsl");
    const MSVO_LINEARIZE: &str = include_str!("../src/wgpu_backend/shaders/msvo_linearize.wgsl");
    const MSVO_DOWNSAMPLE: &str = include_str!("../src/wgpu_backend/shaders/msvo_downsample.wgsl");
    const MSVO_OCCLUDE: &str = include_str!("../src/wgpu_backend/shaders/msvo_occlude.wgsl");
    const MSVO_BLUR: &str = include_str!("../src/wgpu_backend/shaders/msvo_blur.wgsl");
    const MSVO_UPSAMPLE: &str = include_str!("../src/wgpu_backend/shaders/msvo_upsample.wgsl");

    // REQ-SCN-081: the grooves, with the five constants of the requirement.
    assert!(VOXEL_GRID.contains("fn zax_voxel_grid("));
    for number in ["0.38 + 0.24 * t", "0.20 - 0.10 * t", "0.16 - d", "0.035"] {
        assert!(VOXEL_GRID.contains(number), "REQ-SCN-081: {number}");
    }
    // REQ-SCN-082: the voxel centre, and the point placed like a vertex.
    assert!(PIXEL_SHADOW.contains("fn zax_voxel_center("));
    assert!(PIXEL_SHADOW.contains("fn zax_voxel_shadow_point("));
    // Both reach the scene pass, each behind its own marker (REQ-SCN-079 switches them apart).
    assert!(SCENE.contains("//#ifdef ZAX_VOXEL_GRID"));
    assert!(SCENE.contains("//#ifdef ZAX_PIXEL_SHADOW"));
    assert!(SCENE.contains("albedo_linear *= zax_voxel_grid("));
    assert!(SCENE.contains("zax_voxel_shadow_point("));

    // REQ-SCN-080: the depth reconstruction both screen passes share, with the two conventions
    // this backend has to undo — the y-mirrored NDC map, and the clamp of GL's CLAMP_TO_EDGE.
    for name in [
        "fn zax_view_depth(",
        "fn zax_view_sample(",
        "fn zax_view_normal(",
        "fn zax_project_view(",
        "fn zax_dither(",
    ] {
        assert!(SCREEN_DEPTH.contains(name), "REQ-SCN-080: {name}");
    }
    assert!(SCREEN_DEPTH.contains("1.0 - (uv.y * 2.0)"), "the y mirror");
    // REQ-SCN-083: the hemisphere kernel, its ceiling, the depth-aware blur and the `1 - O`
    // factor the multiply blend applies in place.
    assert!(AO.contains("fn zax_kernel("));
    assert!(AO.contains("mix(0.1, 1.0, fraction * fraction)"));
    assert!(AO.contains("screen.ao.y * (occlusion / count)"));
    assert!(AO_BLUR.contains("ZAX_AO_BLUR_SHARPNESS: f32 = 20.0"));
    assert!(AO_COMPOSITE.contains("1.0 - occlusion"));
    // And the pass REQ-SCN-080's last bullet needs here and nowhere else.
    assert!(DEPTH_RESOLVE.contains("texture_depth_multisampled_2d"));
    assert!(DEPTH_RESOLVE.contains("@builtin(frag_depth)"));

    // REQ-SCN-084: the march, the bisection, the thickness test, the two attenuations and the
    // premultiplied result the composite's blend relies on.
    assert!(SSR.contains("fn zax_blurred_colour("));
    for name in [
        "ZAX_SSR_SURFACE_BIAS: f32 = 0.05",
        "ZAX_SSR_THICKNESS: f32 = 2.0",
        "ZAX_SSR_FADE_FROM: f32 = 0.45",
        "ZAX_SSR_EDGE: f32 = 0.06",
    ] {
        assert!(SSR.contains(name), "REQ-SCN-084: {name}");
    }
    assert!(SSR.contains("reflect(normalize(p), normal)"));
    assert!(SSR.contains("vec4<f32>(colour * weight, weight)"));
    assert!(SSR_COMPOSITE.contains("reflection * mask"));
    // And the floor mask, whose fragment stage is the `+Y >= 0,9` test of the floor definition and
    // which reads the scene normal **before** the curvature.
    assert!(SCENE.contains("//#ifdef ZAX_FLOOR_MASK"));
    assert!(SCENE.contains("ZAX_FLOOR_NORMAL_THRESHOLD: f32 = 0.9"));
    assert!(SCENE.contains("scene_normal: vec3<f32>"));
    assert!(SCENE.contains("out.scene_normal = out.normal;"));

    // REQ-SCN-083 *Mode MSVO*: the five passes of the chain RU-6 ported from `MiniEngine`, each
    // with the constant or the departure that would be missing if the pass had been dropped.
    assert!(MSVO_LINEARIZE.contains("-zax_view_sample(in.uv).z"));
    // The decimation's mirrored row, which is the first of this chain's two row-order traps.
    assert!(MSVO_DOWNSAMPLE.contains("at.y * 2 + 1"));
    assert!(MSVO_OCCLUDE.contains("ZAX_MSVO_PATTERN"));
    for entry in [
        "0.9165151",
        "0.9591663",
        "0.8246211",
        "0.7745967",
        "0.4472136",
    ] {
        assert!(
            MSVO_OCCLUDE.contains(entry),
            "the original's table: {entry}"
        );
    }
    // The per-pixel rotation, and the second row-order trap: the hash reads OpenGL's coordinate.
    assert!(MSVO_OCCLUDE.contains("fract(sin(dot(at, vec2<f32>(12.9898, 78.233))) * 43758.5453)"));
    assert!(MSVO_OCCLUDE.contains("floor(zax_pixel(in.position.xy))"));
    // The access headroom, which §2.4's first invariant is what found the need for.
    assert!(MSVO_OCCLUDE.contains("clamp(access, 0.0, headroom) / headroom"));
    // The `SmartBlur` and its tent, and the bilateral upsample with its ceiling and its far test.
    assert!(MSVO_BLUR.contains("select(1.0, 0.5, abs(tap) == 2)"));
    assert!(MSVO_UPSAMPLE.contains("(0.01 * fine) + 1e-3"));
    assert!(MSVO_UPSAMPLE.contains("ceiling * occlusion"));
    assert!(MSVO_UPSAMPLE.contains("fine >= far_distance"));

    // And no WGSL source of this backend carries a `const_assert`, which is how
    // `wgpu_backend/preprocess.rs` reports a variant marker it could not read: the markers are
    // expanded at module creation, so a malformed one is a module that does not compile — and the
    // sources themselves must never carry one.
    for source in [
        COMMON,
        SCENE,
        SHADOW,
        RESOLVE,
        VOXEL_GRID,
        PIXEL_SHADOW,
        SCREEN_DEPTH,
        AO,
        AO_BLUR,
        AO_COMPOSITE,
        DEPTH_RESOLVE,
        SSR,
        SSR_COMPOSITE,
        MSVO_LINEARIZE,
        MSVO_DOWNSAMPLE,
        MSVO_OCCLUDE,
        MSVO_BLUR,
        MSVO_UPSAMPLE,
    ] {
        assert!(!source.contains("const_assert"), "a source carries one");
    }
}
