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
    built.scene.bend = zax_engine3d::space_bend::SpaceBend::new(60.0, 0.0, 0.0, false, 1.0);

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

/// Q-104, extended to AMD-055 by point 6 of this lot's brief: the Enhanced style landed in the
/// **OpenGL** backend only — the one SPEC-06 REQ-SCN-056 requires, the one of the default build,
/// of R0's captures and of the two portable targets. Under `wgpu` the style is read back as set
/// and the scene is drawn in Classic, with one log line.
///
/// This asserts the **known gap**, so that a half-done parity fails loudly rather than passing
/// unnoticed: not one of the four effects is in the WGSL. When that backend gains them, this test
/// and the note in `wgpu_backend/renderer.rs` are what have to go. The log line itself is the
/// observable half and needs a device, which no test on this side is guaranteed one.
#[cfg(feature = "wgpu")]
#[test]
fn q_104_the_wgpu_backend_carries_nothing_of_the_enhanced_style_yet() {
    const SOURCES: [&str; 4] = [
        include_str!("../src/wgpu_backend/shaders/common.wgsl"),
        include_str!("../src/wgpu_backend/shaders/scene.wgsl"),
        include_str!("../src/wgpu_backend/shaders/shadow.wgsl"),
        include_str!("../src/wgpu_backend/shaders/resolve.wgsl"),
    ];

    for source in SOURCES {
        for name in ["voxel", "pixel_shadow", "occlusion", "reflect", "ssr"] {
            assert!(
                !source.to_ascii_lowercase().contains(name),
                "the wgpu shaders mention '{name}': the Enhanced style is being ported, so \
                 Q-104, this test and the note in wgpu_backend/renderer.rs go together"
            );
        }
    }
}
