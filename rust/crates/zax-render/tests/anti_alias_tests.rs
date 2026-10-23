//! The full-screen anti-aliasing SPEC-09 REQ-PERS-017 allows and AMD-032 §2 implements as
//! multisampling: the option to sample-count mapping, the fallback to what the hardware grants,
//! and the effect on a capture (port of `AntiAliasTests.cs`).
//!
//! The mapping and the fallback are arithmetic and need no context; only the last two tests ask
//! for one. The shadow map is deliberately left out of all this (REQ-SCN-030).

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Built, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;
use zax_render::scene::RenderSettings;

/// AMD-032 §2: 0 off, 1 is four samples, 2 is eight.
#[test]
fn the_option_maps_to_a_sample_count() {
    for (option, expected) in [(0, 1), (1, 4), (2, 8)] {
        assert_eq!(expected, geometry::anti_alias_samples(option));
    }
}

/// A value no option carries leaves anti-aliasing off rather than guessing.
#[test]
fn an_option_outside_the_domain_means_off() {
    for option in [-1, 3, 64] {
        assert_eq!(1, geometry::anti_alias_samples(option));
    }
}

/// AMD-032 §2: what is asked for is lowered to the highest power of two the implementation
/// supports, never raised, and never below one sample.
#[test]
fn the_sample_count_falls_back_to_what_is_supported() {
    // asked, supported, granted
    for (asked, supported, granted) in [
        (8, 8, 8),
        (8, 4, 4),
        (8, 2, 2),
        (8, 1, 1),
        (4, 8, 4),
        (1, 8, 1),
        (8, 6, 4),
        (8, 0, 1),
        (0, 8, 1),
    ] {
        assert_eq!(
            granted,
            geometry::clamp_anti_alias_samples(asked, supported)
        );
    }
}

/// The setting never asks the renderer for more than AMD-032 §2 defines.
#[test]
fn the_scene_setting_is_clamped_to_the_defined_range() {
    let mut settings = RenderSettings::default();
    assert_eq!(1, settings.anti_alias_samples());

    settings.set_anti_alias_samples(64);
    assert_eq!(
        geometry::MAX_ANTI_ALIAS_SAMPLES,
        settings.anti_alias_samples()
    );

    settings.set_anti_alias_samples(0);
    assert_eq!(1, settings.anti_alias_samples());
}

/// AMD-032 §2: the renderer reports the sample count it was actually granted, and a plain
/// desktop GL 3.3 driver grants the four and the eight samples the two levels ask for.
#[test]
#[ignore = "needs a driver granting 8 samples: Mesa llvmpipe caps GL_MAX_SAMPLES at 4 (to check on a real machine)"]
fn the_renderer_reports_the_sample_count_it_was_granted() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    for (option, expected) in [(0, 1), (1, 4), (2, 8)] {
        let mut built = edges();
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        gl.render_built(&mut renderer, &built);

        assert_eq!(expected, renderer.statistics().anti_alias_samples);
    }
}

/// The effect the user asked for: a slanted silhouette drawn with no anti-aliasing has two
/// colours along its edge, and multisampling fills it with intermediate ones. Applying the
/// setting takes no restart: the three captures come from one renderer and one context, the
/// sample count changing between two frames (AMD-032 §2).
#[test]
fn anti_aliasing_adds_intermediate_colours_along_the_edges() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    let mut colors = [0; 3];
    for (option, count) in colors.iter_mut().enumerate() {
        let mut built = edges();
        let option = i32::try_from(option).expect("small option");
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        *count = gl
            .render_built(&mut renderer, &built)
            .distinct_colors()
            .len();
    }

    assert!(
        colors[1] > colors[0],
        "MSAA 4x produced {} colours, no more than the {} of no anti-aliasing.",
        colors[1],
        colors[0]
    );
    assert!(
        colors[2] >= colors[1],
        "MSAA 8x produced {} colours, fewer than the {} of MSAA 4x.",
        colors[2],
        colors[1]
    );
}

/// A white block over the background, seen from the default angles: its silhouette is slanted
/// in screen space, so it is all edge.
fn edges() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 24, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 12, 4, 4)
        .build()
}
