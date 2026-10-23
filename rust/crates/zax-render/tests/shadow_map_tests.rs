//! AMD-034 §2: the shadow map is 4096 texels of side by default, falls back to the largest size
//! the hardware accepts (2048, then 1024), and the depth bias is one texel of the map in use,
//! never under 0.02 unit (port of `ShadowMapTests.cs`).
//!
//! The size chain and the bias are arithmetic and need no context; only the last two tests ask
//! for one.

mod common;

use common::{assert_close, solid, Built, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;
use zax_render::scene::RenderSettings;
use zax_render::SceneRenderer;

/// The floor in shadow: NES 0x30 seen from the front (SPEC-06 §5.2).
const SHADOWED_FLOOR: (i32, i32, i32) = (146, 144, 146);

/// AMD-034 §2, replacing the 1024 texels of DEC-R2-D4.
#[test] // REQ-SCN-030, AMD-034
fn the_default_shadow_map_is_four_thousand_ninety_six_texels() {
    assert_eq!(4096, geometry::SHADOW_MAP_SIZE);
    assert_eq!(4096, RenderSettings::default().shadow_map_size());
}

/// The sizes tried, largest first. OpenGL 3.3 only guarantees a 1024-texel texture, so the chain
/// the amendment names — 4096, 2048, 1024 — is the one a default map walks down.
#[test] // REQ-SCN-030, AMD-034
fn the_fallback_chain_halves_the_requested_size() {
    assert_eq!(
        vec![4096, 2048, 1024, 512, 256, 128, 64],
        geometry::shadow_map_sizes(4096, 16384).collect::<Vec<_>>()
    );
}

/// A hardware limit below what was asked for removes the sizes the driver already says it
/// cannot hold, so the first size tried is the largest possible one (AMD-034 §2).
#[test] // REQ-SCN-030, AMD-034
fn the_largest_size_tried_is_capped_by_the_hardware() {
    // requested, GL_MAX_TEXTURE_SIZE, largest size tried
    for (requested, max, first) in [
        (4096, 16384, 4096),
        (4096, 4096, 4096),
        (4096, 2048, 2048),
        (4096, 1024, 1024),
        (2048, 16384, 2048),
    ] {
        assert_eq!(Some(first), geometry::shadow_map_sizes(requested, max).next());
    }
}

/// A driver that answers nonsense, or a map asked for below the floor, still yields one size to
/// try: the renderer must never end up with no shadow map at all.
#[test] // REQ-SCN-030, AMD-034
fn a_nonsense_size_or_limit_still_leaves_the_floor_to_try() {
    for (requested, max) in [(4096, 0), (4096, -1), (0, 16384), (-8, 16384)] {
        assert_eq!(
            vec![geometry::MIN_SHADOW_MAP_SIZE],
            geometry::shadow_map_sizes(requested, max).collect::<Vec<_>>()
        );
    }
}

/// AMD-034 §2 lowers the bias of Q-032 from 1.5 texels to 1 texel, and keeps the 0.02 unit of
/// REQ-SCN-031 as a floor it may never go under.
#[test] // REQ-SCN-031, AMD-034
fn the_bias_is_one_texel_and_never_under_two_hundredths() {
    assert_close(1.0, f64::from(geometry::SHADOW_BIAS_TEXELS), 0.0);
    assert_close(0.02, f64::from(geometry::SHADOW_DEPTH_BIAS), 1e-9);

    // Above the floor the bias is exactly the texel.
    assert_close(0.5, f64::from(geometry::shadow_bias(0.5)), 1e-6);
    assert_close(0.1, f64::from(geometry::shadow_bias(0.1)), 1e-6);

    // Below it, and at the degenerate sizes, the floor wins.
    assert_close(0.02, f64::from(geometry::shadow_bias(0.019)), 1e-6);
    assert_close(0.02, f64::from(geometry::shadow_bias(0.0)), 1e-6);
}

/// The point of the amendment, as arithmetic: on the same covered area a 4096-texel map has a
/// texel four times finer than a 1024-texel one, and the bias that is subtracted from every
/// shadow shrinks with it — six times less at 1 texel than at the 1.5 texels of Q-032.
#[test] // REQ-SCN-031, AMD-034
fn a_finer_map_shortens_the_bias_the_shadows_lose() {
    // The extent of the SMB3 world map across the light, about 410 units.
    let extent = 410.0_f32;
    let before = (1.5 * (extent / 1024.0)).max(0.02);
    let after = geometry::shadow_bias(extent / 4096.0);

    assert!(
        after < before / 5.0,
        "bias {after} is not six times under the {before} of Q-032"
    );
}

/// With a context: the renderer reports the size it was granted, which on a desktop GL 3.3
/// driver is the 4096 texels asked for, and a changed setting is honoured on the next frame.
#[test] // REQ-SCN-030, AMD-034
fn the_renderer_reports_the_shadow_map_size_it_was_granted() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    for asked in [4096, 2048, 1024, 4096] {
        let mut built = thin_strokes();
        built.scene.settings.set_shadow_map_size(asked);
        gl.render_built(&mut renderer, &built);

        let statistics = renderer.statistics();
        assert!(statistics.shadow_pass);
        assert_eq!(asked, statistics.requested_shadow_map_size);
        assert_eq!(asked, statistics.shadow_map_size);
    }
}

/// What the user reported: on the same scene, a finer map casts more shadow. The glyphs of
/// AMD-034 §1 are one-unit strokes standing 8 units in front of a panel; here single-tile
/// columns standing in front of a wall stand in for them, and the shadowed pixels of the wall
/// are counted.
#[test] // REQ-SCN-030, AMD-034
fn a_finer_map_casts_more_shadow_under_a_thin_stroke() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let coarse = shadowed_pixels(&gl, &mut renderer, 1024);
    let fine = shadowed_pixels(&gl, &mut renderer, geometry::SHADOW_MAP_SIZE);

    assert!(coarse > 0, "the coarse map casts no shadow at all; the scene is wrong");
    assert!(
        fine > coarse,
        "the {}-texel map shadows {fine} pixels, no more than the {coarse} of a 1024-texel one",
        geometry::SHADOW_MAP_SIZE
    );
}

fn shadowed_pixels(gl: &GlFixture, renderer: &mut SceneRenderer, map_size: i32) -> usize {
    let mut built = thin_strokes();
    built.scene.settings.set_shadow_map_size(map_size);
    let capture = gl.render_built(renderer, &built);

    let mut count = 0;
    for y in 0..capture.height {
        for x in 0..capture.width {
            let (r, g, b) = capture.at(x, y);
            if (r - SHADOWED_FLOOR.0).abs() <= 4
                && (g - SHADOWED_FLOOR.1).abs() <= 4
                && (b - SHADOWED_FLOOR.2).abs() <= 4
            {
                count += 1;
            }
        }
    }

    count
}

/// A flat white wall with one-tile-wide columns standing 32 units in front of it: at 1024
/// texels a texel is wider than a column, so their shadows break into dots (AMD-034 §1).
fn thin_strokes() -> Built {
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern(solid(2, 0), 32, ProfileLayer::L1)
        .rect(solid(1, 0), 6, 8, 20, 14);

    for column in (8..24).step_by(2) {
        fixture = fixture.rect(solid(2, 0), column, 10, 1, 10);
    }

    fixture.build()
}
