//! AMD-051 REQ-E3D-644: a 3D object whose texture cannot be allocated is left out of the frame
//! instead of taking the frame — and the rest of the scene is drawn, the next frame is produced,
//! and one diagnostic line per pattern says so.
//!
//! The budget of `SceneRenderer::set_texture_budget` is what the harness option `--atlas-budget`
//! sets: the only way to reach this path on a desktop GPU, which refuses nothing an NES profile
//! can ask of it.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;

/// Two patterns of one tile each, side by side on the background grid: two 3D objects with two
/// textures, so a budget can fit one of them and refuse the other.
fn two_objects() -> common::Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(5, 0x16)
        .pattern(solid(1, 1), 8, ProfileLayer::L1)
        .pattern(solid(2, 2), 8, ProfileLayer::L1)
        .tile(solid(1, 1), 96, 112, 0)
        .tile(solid(2, 2), 136, 112, 1)
        .build()
}

#[test]
fn req_e3d_644_a_texture_that_cannot_be_allocated_leaves_its_tiles_out_and_draws_the_rest() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let built = two_objects();

    // What the whole scene looks like, so that "the rest is drawn" has something to mean.
    let whole = fixture.render_built(&mut renderer, &built);
    let drawn = whole.count_other(whole.packed(0, 0));
    assert!(drawn > 0, "the scene draws something at all");
    let texels = renderer.statistics().texture_texels;
    assert!(texels > 0, "two object textures are resident");

    // One texel short of what the two textures need: the second one cannot be allocated.
    let mut renderer = fixture.renderer();
    renderer.set_texture_budget(Some(texels - 1));
    let partial = fixture.render_built(&mut renderer, &built);
    let statistics = renderer.statistics();
    assert_eq!(
        1, statistics.patterns_without_texture,
        "exactly one pattern was refused"
    );
    assert!(
        statistics.opaque_tiles > 0,
        "the pattern that fits is still drawn: {} opaque tiles",
        statistics.opaque_tiles
    );
    let partial_drawn = partial.count_other(partial.packed(0, 0));
    assert!(
        partial_drawn > 0 && partial_drawn < drawn,
        "some of the scene is drawn and some is missing: {partial_drawn} of {drawn}"
    );

    // REQ-E3D-644: "la frame suivante est produite" — no error was raised and the renderer is
    // still usable, with the same outcome.
    let again = fixture.render_built(&mut renderer, &built);
    assert_eq!(
        partial_drawn,
        again.count_other(again.packed(0, 0)),
        "the next frame is produced, and draws the same thing"
    );
    assert_eq!(
        1,
        renderer.statistics().patterns_without_texture,
        "the running total counts patterns, not frames"
    );
}

#[test]
fn req_e3d_644_the_diagnostic_is_one_line_per_pattern_and_nothing_when_all_is_well() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let built = two_objects();

    fixture.render_built(&mut renderer, &built);
    assert!(
        renderer.take_diagnostics().is_empty(),
        "a frame that drew everything says nothing"
    );

    // No room for any texture at all: both patterns are refused, each exactly once.
    let mut renderer = fixture.renderer();
    renderer.set_texture_budget(Some(0));
    fixture.render_built(&mut renderer, &built);
    let lines = renderer.take_diagnostics();
    assert_eq!(2, lines.len(), "one line per pattern: {lines:?}");
    for line in &lines {
        assert!(line.contains("REQ-E3D-644"), "the line names it: {line}");
    }

    // Three more frames of the same failure add nothing: the rule is per pattern, not per frame.
    for _ in 0..3 {
        fixture.render_built(&mut renderer, &built);
        assert!(
            renderer.take_diagnostics().is_empty(),
            "a pattern is reported once"
        );
    }
    assert_eq!(2, renderer.statistics().patterns_without_texture);
}

#[test]
fn req_e3d_644_a_budget_that_fits_everything_changes_nothing() {
    let fixture = GlFixture::new();
    let mut reference = fixture.renderer();
    let built = two_objects();
    let whole = fixture.render_built(&mut reference, &built);
    let texels = reference.statistics().texture_texels;

    let mut renderer = fixture.renderer();
    renderer.set_texture_budget(Some(texels));
    let capture = fixture.render_built(&mut renderer, &built);
    assert_eq!(
        0,
        renderer.statistics().patterns_without_texture,
        "nothing is refused when the budget is exactly enough"
    );
    for y in 0..fixture.height() {
        for x in 0..fixture.width() {
            assert_eq!(
                whole.packed(x, y),
                capture.packed(x, y),
                "the picture is the same at ({x}, {y})"
            );
        }
    }
}

#[test]
fn req_e3d_646_a_late_object_is_reported_once_through_the_shared_line_budget() {
    let fixture = GlFixture::new();
    let mut renderer = fixture.renderer();
    let built = two_objects();
    let id = *built
        .instances(|_| true)
        .first()
        .expect("the scene has an instance");
    let pattern = built.instance(id).reference().pattern;

    renderer.report_late_object(pattern, 0, 600);
    renderer.report_late_object(pattern, 0, 600);
    let lines = renderer.take_diagnostics();
    assert_eq!(1, lines.len(), "at most once per object: {lines:?}");
    assert!(lines[0].contains("REQ-E3D-646"), "{}", lines[0]);
    assert!(lines[0].contains("600"), "{}", lines[0]);

    // Another object of the same pattern is its own subject.
    renderer.report_late_object(pattern, 1, 600);
    assert_eq!(1, renderer.take_diagnostics().len());
}
