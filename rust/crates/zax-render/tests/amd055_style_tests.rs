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

/// Q-104, being answered (RU-7): the two **shader** effects of the Enhanced style and the
/// **occlusion** of REQ-SCN-083 are in the `wgpu` backend, and the **reflections** of
/// REQ-SCN-084 are not yet.
///
/// This test used to assert that not one of the four effects was in the WGSL — the known gap, so
/// that a half-done parity failed loudly rather than passing unnoticed. Step 3 ported the voxel
/// grid of REQ-SCN-081 and the pixel shadows of REQ-SCN-082, which are variants of the scene
/// module; step 4b ported the occlusion of REQ-SCN-083 and the architecture its passes live in.
/// So the assertion stays **two-sided**, and the second list is what keeps the remaining gap
/// pinned: the day the reflections land, it empties and the diagnostic line of
/// `wgpu_backend/renderer.rs` goes with it.
///
/// Whether the WGSL compiles and whether the two backends agree is not a question this side can
/// answer — naga validates at module creation. That is
/// `zax-harness/tests/parity_wgpu_tests.rs`.
#[cfg(feature = "wgpu")]
#[test]
fn q_104_the_wgpu_backend_carries_the_occlusion_and_not_yet_the_reflections() {
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

    // And the half that is still to come: nothing of the reflections of REQ-SCN-084, nor of the
    // multi-scale estimator of REQ-SCN-083's *Mode MSVO*, is in any WGSL source yet.
    //
    // The names here are **functions** rather than words: the sources above say "MSVO" and
    // "reflections" in their prose, because each explains which requirement it serves and which
    // part of it is still to come, and that prose is the thing a reviewer wants. What must not
    // exist yet is the code.
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
    ] {
        for name in [
            "fn zax_trace",
            "fn zax_reflect",
            "fn zax_floor_mask",
            "fn zax_ssr",
            "fn zax_msvo",
        ] {
            assert!(
                !source.to_ascii_lowercase().contains(name),
                "the wgpu shaders mention '{name}': the reflections of REQ-SCN-084 and the MSVO \
                 estimator of REQ-SCN-083 are being ported, so this second list, the diagnostic \
                 line of wgpu_backend/renderer.rs and the Q-104 notes go together"
            );
        }
    }
}
