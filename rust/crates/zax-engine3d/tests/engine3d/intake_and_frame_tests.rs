//! Intake of a batch (SPEC-04 §3.1), image attributes (§3.2) and the frame model (§3.3).

use std::collections::HashSet;

use rstest::rstest;
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::grid;
use zax_engine3d::image::{ImageColors, ImageDensity, ImageRegistry, ImageShape2D, TileImageKey};
use zax_engine3d::settings::ConversionSettings;
use zax_engine3d::vectors::{Cell, Vec3};
use zax_profiles::values::Vec4F;

use super::support::{cells, img, permanent, Batch, PatternOptions, ProfileBuilder};

#[test]
fn req_e3d_001_a_batch_produces_one_frame_whose_counter_is_the_batch_counter() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);

    let frame = batch.convert(&mut engine, 42);
    assert_eq!(42, frame.counter());
    let converted = std::ptr::from_ref(frame);

    assert!(std::ptr::eq(converted, engine.converting_frame()));
}

#[rstest]
#[case(-8, 100, false)]
#[case(-7, 100, true)]
#[case(247, 100, true)]
#[case(248, 100, false)]
#[case(100, 239, true)]
#[case(100, 240, false)]
#[case(100, -7, true)]
fn req_e3d_002_the_clip_area_keeps_a_tile_or_not(#[case] x: i32, #[case] y: i32, #[case] kept: bool) {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch.bg(img::a(), x, y, 0);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(usize::from(kept), frame.background_tile_count());
}

#[test]
fn req_e3d_002_the_clip_margins_come_from_the_settings() {
    let mut engine = ProfileBuilder::empty_engine();
    let clipping = engine.settings().clipping;
    let default = ConversionSettings::DEFAULT_CLIPPING;
    assert_eq!(
        (default.x, default.y, default.z, default.w),
        (clipping.x, clipping.y, clipping.z, clipping.w)
    );

    // Widening the left margin to 16 drops a tile that the default margins keep.
    engine.settings_mut().clipping = Vec4F::new(16.0, 8.1, 8.1, 0.1);
    let mut batch = Batch::new();
    batch.bg(img::a(), 8, 100, 0);
    assert_eq!(0, batch.convert(&mut engine, 0).background_tile_count());
}

#[test]
fn req_e3d_003_a_sprite_at_240_or_empty_is_dropped() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .sprite(img::a(), 100, 240)
        .sprite(img::EMPTY, 100, 100)
        .sprite(img::a(), 100, 100);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.sprites().len());
    assert_eq!(100, frame.sprites()[0].y());
}

#[test]
fn req_e3d_004_an_empty_background_tile_depends_on_the_setting() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .bg_cell(img::a(), 10, 10)
        .bg_cell(img::EMPTY, 11, 10)
        .bg_cell(img::a(), 12, 10);

    let dropped = batch.convert(&mut engine, 0);
    assert_eq!(2, dropped.background_tile_count());

    // REQ-E3D-041: the missing cell separates the two regions.
    assert_eq!(2, dropped.shapes().len());

    engine.settings_mut().empty_tile = true;
    let kept = batch.convert(&mut engine, 1);
    assert_eq!(3, kept.background_tile_count());
}

#[test]
fn req_e3d_004_an_empty_image_can_join_a_shape_when_a_pattern_holds_it() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::EMPTY, img::a()]))
        .load();
    engine.settings_mut().empty_tile = true;

    let mut batch = Batch::new();
    batch.bg_row(10, 10, &[img::a(), img::EMPTY, img::a()]);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(3, frame.shape(frame.shapes()[0]).tile_count());
}

#[rstest]
#[case(-3, 13, 1, 3)]
#[case(5, 16, 2, 3)]
fn req_e3d_005_a_background_tile_lands_in_its_cell(#[case] x: i32, #[case] y: i32, #[case] cx: i32, #[case] cy: i32) {
    assert_eq!(Cell::new(cx, cy), grid::background_cell(x, y));

    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch.bg(img::a(), x, y, 0);
    let frame = batch.convert(&mut engine, 0);

    let tile = frame.background_at_cell(Cell::new(cx, cy)).unwrap();
    assert_eq!(x, tile.x());
    assert_eq!(y, tile.y());
}

#[test]
fn req_e3d_005_a_sprite_lands_in_its_cell_without_margin() {
    assert_eq!(Cell::new(1, 2), grid::sprite_cell(13, 20));

    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch.sprite_full(img::a(), 13, 20, 2, false);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(Cell::new(1, 2), frame.sprites()[0].position());
    assert_eq!(2, frame.sprites()[0].palette());
}

fn first_image_of_first_shape(engine: &mut ConversionEngine, batch: &Batch, counter: i32) -> TileImageKey {
    let frame = batch.convert(engine, counter);
    frame.shape(frame.shapes()[0]).images()[0].key()
}

#[test]
fn req_e3d_006_the_germination_order_follows_the_scan_direction() {
    // Two separate one-tile shapes: the one that germinates first is added first.
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a()]))
        .row(&cells(&[img::b()]))
        .load();

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 20, 2).bg_cell(img::b(), 2, 20);

    assert_eq!(2, batch.convert(&mut engine, 0).shapes().len());
    assert_eq!(img::a(), first_image_of_first_shape(&mut engine, &batch, 0));

    engine.settings_mut().scan_direction = 1;
    assert_eq!(img::b(), first_image_of_first_shape(&mut engine, &batch, 1));
}

#[test]
fn req_e3d_006_sprites_germinate_in_emission_order() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a()]))
        .row(&cells(&[img::b()]))
        .load();
    let mut batch = Batch::new();
    batch.sprite(img::b(), 200, 100).sprite(img::a(), 10, 100);

    assert_eq!(2, batch.convert(&mut engine, 0).shapes().len());
    assert_eq!(img::b(), first_image_of_first_shape(&mut engine, &batch, 0));
}

#[test]
fn req_e3d_007_an_image_keeps_its_identity_for_the_session() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let pattern = permanent(&engine, 0);

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);
    let frame = batch.convert(&mut engine, 1);
    let image = frame.shape(frame.shapes()[0]).images()[0];

    for counter in 2..=1000 {
        let frame = batch.convert(&mut engine, counter);
        let shape = frame.shape(frame.shapes()[0]);
        assert_eq!(image, shape.images()[0]);
        assert_eq!(Some(pattern), shape.reference_pattern());
    }

    assert_eq!(1, engine.images().count());
}

#[test]
fn req_e3d_008_the_learned_state_survives_a_relaunch_and_dies_with_the_game() {
    let profile_a = ProfileBuilder::new().row(&cells(&[img::a()])).build();
    let mut engine = ConversionEngine::new();
    engine.load_profile(&profile_a, true).unwrap();

    let mut learning = Batch::new();
    learning.bg_cell(img::u(), 10, 10);
    learning.convert(&mut engine, 1);
    assert_eq!(2, engine.patterns().permanent().len());

    // Relaunching the same game without reloading the profile keeps what was learned, and the
    // learned pattern is active from the very first frame.
    engine.restart();
    assert_eq!(2, engine.patterns().permanent().len());
    let learned = permanent(&engine, 1);
    let first = learning.convert(&mut engine, 1);
    assert_eq!(1, first.shapes().len());
    assert_eq!(Some(learned), first.shape(first.shapes()[0]).reference_pattern());

    // Loading another game clears everything.
    engine
        .load_profile(&ProfileBuilder::new().row(&cells(&[img::b()])).build(), true)
        .unwrap();
    assert_eq!(1, engine.patterns().permanent().len());
    assert_eq!(1, engine.images().count());
}

#[test]
fn req_e3d_020_image_attributes() {
    let mut registry = ImageRegistry::new();

    let two = registry.intern(TileImageKey::new(
        img::pixel(2, 3, false, false).low | img::pixel(5, 6, false, false).low,
        0,
        false,
        false,
    ));
    assert_eq!(2, two.pixel_count());
    assert_eq!(ImageColors::COLOR1, two.colors());
    assert_eq!(Cell::new(2, 3), two.pixel_start());
    assert_eq!(Cell::new(5, 6), two.pixel_end());

    let empty = registry.intern(img::EMPTY);
    assert_eq!(0, empty.pixel_count());
    assert_eq!(Cell::new(7, 7), empty.pixel_start());
    assert_eq!(Cell::new(0, 0), empty.pixel_end());
}

#[test]
fn req_e3d_021_density_and_2d_shape_classes() {
    let mut registry = ImageRegistry::new();

    // A full tile of one colour: SOLID and FULL_RECT.
    let solid = registry.intern(TileImageKey::new(u64::MAX, 0, false, false));
    assert_eq!(ImageDensity::Solid, solid.density());
    assert_eq!(ImageShape2D::FullRect, solid.shape_2d());

    // A monochrome glyph of 20 pixels: AVERAGE and CHAR.
    let plane = (0..20).fold(0u64, |plane, i| plane | (1 << i));
    let glyph = registry.intern(TileImageKey::new(plane, 0, false, false));
    assert_eq!(20, glyph.pixel_count());
    assert_eq!(ImageDensity::Average, glyph.density());
    assert_eq!(ImageShape2D::Char, glyph.shape_2d());
}

#[test]
fn req_e3d_040_the_frame_lists_background_shapes_first() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a()]))
        .row(&cells(&[img::b()]))
        .load();
    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10).sprite(img::b(), 100, 100);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(2, frame.shapes().len());
    assert_eq!(1, frame.background_shape_count());
    for &shape in &frame.shapes()[..frame.background_shape_count()] {
        assert!(frame.shape(shape).background());
    }

    assert!(!frame.shape(frame.shapes()[frame.background_shape_count()]).background());
}

#[test]
fn req_e3d_042_every_tile_belongs_to_at_most_one_shape() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .load();
    let batch = Batch::full_frame(4);

    let frame = batch.convert(&mut engine, 0);

    let tiles_in_shapes: usize = frame.shape_list().map(|shape| shape.tile_count()).sum();
    assert!(tiles_in_shapes <= frame.background_tile_count() + frame.sprites().len());

    let mut seen = HashSet::new();
    for shape in frame.shape_list() {
        for &tile in shape.tiles() {
            assert!(seen.insert(tile));
        }
    }
}

#[test]
fn req_e3d_043_the_frame_time_is_the_counter_over_60_whatever_the_region() {
    let mut engine = ProfileBuilder::empty_engine();
    let frame = Batch::new().convert(&mut engine, 600);

    assert_eq!(600, frame.counter());
    assert!((frame.time() - 10.0).abs() < 1e-5);
}

#[test]
fn req_e3d_044_shape_boxes_and_centre() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0).bg(img::a(), 24, 32, 0);

    let frame = batch.convert(&mut engine, 0);
    let shape = frame.shape(frame.shapes()[0]);

    assert_eq!(2, shape.tile_count());
    assert_eq!(Cell::new(16, 32), shape.pixel_start());
    assert_eq!(Cell::new(31, 39), shape.pixel_end());
    assert_eq!(Vec3::new(24.0, 36.0, 0.0), shape.center());
}

#[test]
fn req_e3d_045_inside_at_border_big_and_finished() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();

    let mut border = Batch::new();
    border.mask(8, 0).bg(img::a(), 8, 100, 0);
    let frame = border.convert(&mut engine, 0);
    let at_border = frame.shape(frame.shapes()[0]);
    assert_eq!(8, at_border.pixel_start().x);
    assert!(at_border.at_border());
    assert!(!at_border.inside());

    let mut inside = Batch::new();
    inside.mask(8, 0);
    for x in (9..=193).step_by(8) {
        inside.bg(img::a(), x, 1, 0);
    }

    for y in (9..=225).step_by(8) {
        inside.bg(img::a(), 9, y, 0);
    }

    let frame = inside.convert(&mut engine, 1);
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(9, shape.pixel_start().x);
    assert_eq!(200, shape.pixel_end().x);
    assert_eq!(1, shape.pixel_start().y);
    assert_eq!(232, shape.pixel_end().y);
    assert!(shape.inside());
    assert!(shape.is_finished());
}

#[test]
fn req_e3d_045_a_shape_28_cells_wide_is_big() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let mut batch = Batch::new();
    for column in 0..28 {
        batch.bg_cell(img::a(), column, 10);
    }

    let frame = batch.convert(&mut engine, 0);
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(28, shape.tile_size().x);
    assert!(shape.is_big());
    assert!(shape.is_finished());
}

#[test]
fn req_e3d_046_completed_req_e3d_063_shape_key() {
    let mut engine = ProfileBuilder::new()
        .row_with(
            &cells(&[img::a(), img::b(), img::c()]),
            PatternOptions {
                self_connected: false,
                ..PatternOptions::default()
            },
        )
        .load();
    let mut batch = Batch::new();
    batch.bg_row(10, 10, &[img::a(), img::b()]);

    let frame = batch.convert(&mut engine, 0);
    let shape = frame.shape(frame.shapes()[0]);
    let reference = shape.reference_pattern().unwrap();

    assert_eq!(2, shape.images().len());
    assert!(!shape.completed());
    assert_eq!(3, engine.patterns().get(reference).unwrap().images().len());
}

#[test]
fn req_e3d_047_the_shape_palette_is_the_germ_attribute_plus_four_for_a_sprite() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();

    let mut sprite = Batch::new();
    sprite.sprite_full(img::a(), 100, 100, 2, false);
    let frame = sprite.convert(&mut engine, 0);
    assert_eq!(6, frame.shape(frame.shapes()[0]).palette());

    let mut background = Batch::new();
    background.bg_cell_pal(img::a(), 10, 10, 2);
    let frame = background.convert(&mut engine, 1);
    assert_eq!(2, frame.shape(frame.shapes()[0]).palette());
}

#[test]
fn req_e3d_048_hidden_is_the_or_of_the_sprite_flags() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .sprite(img::a(), 100, 100)
        .sprite(img::b(), 108, 100)
        .sprite_full(img::c(), 116, 100, 0, true)
        .sprite(img::d(), 124, 100);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(4, shape.tile_count());
    assert!(shape.hidden());
}

#[test]
fn req_e3d_048_a_background_shape_is_never_hidden() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);

    let frame = batch.convert(&mut engine, 0);
    assert!(!frame.shape(frame.shapes()[0]).hidden());
}
