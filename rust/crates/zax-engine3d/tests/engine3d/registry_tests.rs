//! The pattern registry, its keys and its relations (SPEC-04 §3.4), and the parts of the
//! segmentation that the test vectors do not cover (§3.5, §3.6).

use zax_engine3d::image::{ImageRegistry, TileImageKey};
use zax_engine3d::instance::PatternInstance;
use zax_engine3d::pattern::{EnginePattern, GrowDirection};
use zax_engine3d::registry::PatternRegistry;
use zax_engine3d::settings::ZLayer;
use zax_engine3d::vectors::{Cell, Vec2};
use zax_profiles::enums::{PatternColorFlags, PatternDensity, ProfileLayer};

use super::support::{cells, img, instances, permanent, Batch, PatternOptions, ProfileBuilder};

fn build(
    registry: &mut ImageRegistry,
    width: u16,
    height: u16,
    keys: &[Option<TileImageKey>],
    self_connected: bool,
    permanent: bool,
) -> EnginePattern {
    let images: Vec<_> = keys
        .iter()
        .map(|key| key.map(|key| registry.intern(key)))
        .collect();
    let mut pattern = EnginePattern::new(width, height, images, permanent);
    pattern.set_self_connected(self_connected);
    pattern.add_instance(PatternInstance::new());
    pattern
}

fn closed(
    registry: &mut ImageRegistry,
    width: u16,
    height: u16,
    keys: &[TileImageKey],
) -> EnginePattern {
    build(registry, width, height, &cells(keys), true, true)
}

#[test]
fn req_e3d_061_the_relations_of_a_one_row_pattern() {
    let mut registry = ImageRegistry::new();
    let a = registry.intern(img::a());
    let b = registry.intern(img::b());

    let open = build(
        &mut registry,
        2,
        1,
        &cells(&[img::a(), img::b()]),
        false,
        true,
    );
    assert_eq!(1, open.horizontal_pair_count());
    assert!(open.has_horizontal_pair(&a, &b));
    assert_eq!(0, open.vertical_pair_count());

    let closed = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    assert_eq!(2, closed.horizontal_pair_count());
    assert!(closed.has_horizontal_pair(&a, &b));
    assert!(closed.has_horizontal_pair(&b, &a));
    assert_eq!(2, closed.vertical_pair_count());
    assert!(closed.has_vertical_pair(&a, &a));
    assert!(closed.has_vertical_pair(&b, &b));
}

#[test]
fn req_e3d_061_a_single_cell_pattern_connected_to_itself() {
    let mut registry = ImageRegistry::new();
    let a = registry.intern(img::a());
    let pattern = closed(&mut registry, 1, 1, &[img::a()]);

    assert_eq!(1, pattern.horizontal_pair_count());
    assert_eq!(1, pattern.vertical_pair_count());
    assert!(pattern.has_horizontal_pair(&a, &a));
    assert!(pattern.has_vertical_pair(&a, &a));
}

#[test]
fn req_e3d_061_empty_cells_produce_no_pair() {
    let mut registry = ImageRegistry::new();
    let pattern = build(&mut registry, 2, 1, &[Some(img::a()), None], false, true);

    assert_eq!(0, pattern.horizontal_pair_count());
    assert_eq!(0, pattern.vertical_pair_count());
    assert_eq!(1, pattern.images().len());
}

#[test]
fn req_e3d_062_skip_geo_ignores_the_direction() {
    let mut registry = ImageRegistry::new();
    let a = registry.intern(img::a());
    let b = registry.intern(img::b());
    let mut pattern = build(
        &mut registry,
        2,
        1,
        &cells(&[img::a(), img::b()]),
        false,
        true,
    );
    pattern.skip_geometry = true;

    for direction in GrowDirection::ORDER {
        assert!(pattern.allows_direction(&a, &b, direction));
        assert!(pattern.allows_direction(&b, &a, direction));
    }

    let c = registry.intern(img::c());
    assert!(!pattern.allows_direction(&a, &c, GrowDirection::Right));
}

#[test]
fn req_e3d_062_the_size_constraint() {
    let mut registry = ImageRegistry::new();
    let mut pattern = closed(&mut registry, 1, 1, &[img::a()]);
    assert!(pattern.fits_constraint(99, 99));
    assert!(!pattern.fits_constraint(100, 1));

    pattern.constraint_size = Cell::new(2, 99);
    assert!(pattern.fits_constraint(2, 40));
    assert!(!pattern.fits_constraint(3, 1));
}

#[test]
fn req_e3d_063_two_patterns_with_the_same_images_are_equal() {
    let mut registry = ImageRegistry::new();
    let wide = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    let tall = closed(&mut registry, 1, 3, &[img::b(), img::a(), img::b()]);

    assert_eq!(wide.key(), tall.key());

    let mut patterns = PatternRegistry::new();
    let tall_key = tall.key().clone();
    let id = patterns.add(wide, &mut registry);
    assert_eq!(Some(id), patterns.find_permanent(tall_key.ids()));
}

#[test]
fn req_e3d_064_the_pattern_pixel_box_and_its_corners() {
    let mut registry = ImageRegistry::new();
    let pattern = closed(&mut registry, 1, 1, &[img::a()]);

    assert_eq!(Cell::new(0, 0), pattern.pixel_start());
    assert_eq!(Cell::new(7, 7), pattern.pixel_end());
    assert_eq!(Vec2::new(0.0, 0.0), pattern.bottom_left());

    // TopRight uses the grid width on the y axis: the kept quirk of DEC-04-Q04.
    let wide = closed(&mut registry, 3, 1, &[img::a(), img::b(), img::c()]);
    assert_eq!(Vec2::new(23.0, f32::from((8u8 * 3) - 1)), wide.top_right());
}

#[test]
fn req_e3d_065_a_learned_permanent_pattern_swallows_the_temporary_subsets() {
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();

    let temporary = build(&mut registry, 1, 1, &cells(&[img::a()]), true, false);
    patterns.add(temporary, &mut registry);
    assert_eq!(1, patterns.temporary().len());

    let permanent = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    patterns.add(permanent, &mut registry);

    assert!(patterns.temporary().is_empty());
    assert_eq!(1, patterns.permanent().len());
}

#[test]
fn req_e3d_065_a_new_temporary_pattern_replaces_the_ones_that_contain_it() {
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();

    let big = build(
        &mut registry,
        2,
        1,
        &cells(&[img::a(), img::b()]),
        true,
        false,
    );
    patterns.add(big, &mut registry);
    let small = build(&mut registry, 1, 1, &cells(&[img::a()]), true, false);
    let small = patterns.add(small, &mut registry);

    assert_eq!([small], patterns.temporary());
}

#[test]
fn req_e3d_065_a_permanent_pattern_replaces_the_one_of_the_same_key() {
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();
    let first = closed(&mut registry, 1, 1, &[img::a()]);
    let first = patterns.add(first, &mut registry);
    let second = closed(&mut registry, 1, 1, &[img::a()]);
    let second = patterns.add(second, &mut registry);

    assert_eq!([second], patterns.permanent());
    let a = registry.intern(img::a());
    assert!(!registry.permanent_patterns(&a).contains(&first));
    assert!(registry.permanent_patterns(&a).contains(&second));
}

#[test]
fn req_e3d_091_candidates_keep_the_registration_order() {
    // Two patterns of equal size holding A: the first registered wins.
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .row(&cells(&[img::a(), img::c()]))
        .load();

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);

    let first = permanent(&engine, 0);
    let frame = batch.convert(&mut engine, 0);
    assert_eq!(
        Some(first),
        frame.shape(frame.shapes()[0]).reference_pattern()
    );
}

#[test]
fn amd_021_r2_008_a_newcomer_takes_the_place_of_the_last_pattern_removed() {
    // AMD-021 R2-008 criterion: P1 then P2 both hold image A; P1 is removed and P3, which
    // also holds A, is registered — the candidates of A are then P3, P2, so the shape of a
    // lone A refers to P3 and not to P2.
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();
    let a = registry.intern(img::a());

    let p1 = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    let p2 = closed(&mut registry, 2, 1, &[img::a(), img::c()]);
    let p1 = patterns.register(p1, &mut registry);
    let p2 = patterns.register(p2, &mut registry);
    assert_eq!([p1, p2], registry.permanent_patterns(&a));

    patterns.remove(p1, &mut registry);
    let p3 = closed(&mut registry, 2, 1, &[img::a(), img::d()]);
    let p3 = patterns.register(p3, &mut registry);

    assert_eq!([p3, p2], registry.permanent_patterns(&a));

    // The place is used up: a fourth pattern goes to the end.
    let p4 = closed(&mut registry, 2, 1, &[img::a(), img::u()]);
    let p4 = patterns.register(p4, &mut registry);
    assert_eq!([p3, p2, p4], registry.permanent_patterns(&a));
}

#[test]
fn amd_021_r2_008_without_a_removal_a_newcomer_goes_last() {
    // Second half of the criterion: with no removal beforehand, P3 comes after P2.
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();
    let a = registry.intern(img::a());

    let p1 = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    let p2 = closed(&mut registry, 2, 1, &[img::a(), img::c()]);
    let p3 = closed(&mut registry, 2, 1, &[img::a(), img::d()]);
    let p1 = patterns.register(p1, &mut registry);
    let p2 = patterns.register(p2, &mut registry);
    let p3 = patterns.register(p3, &mut registry);

    assert_eq!([p1, p2, p3], registry.permanent_patterns(&a));
}

#[test]
fn amd_021_r2_008_the_place_is_kept_per_image() {
    // Removing a pattern frees a place in the candidate list of each of its images, at the
    // index it occupied there — which need not be the same index in both.
    let mut registry = ImageRegistry::new();
    let mut patterns = PatternRegistry::new();
    let a = registry.intern(img::a());
    let b = registry.intern(img::b());

    let only_b = closed(&mut registry, 1, 1, &[img::b()]);
    let both = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    let only_a = closed(&mut registry, 1, 1, &[img::a()]);
    let only_b = patterns.register(only_b, &mut registry);
    let both = patterns.register(both, &mut registry);
    let only_a = patterns.register(only_a, &mut registry);

    assert_eq!([both, only_a], registry.permanent_patterns(&a));
    assert_eq!([only_b, both], registry.permanent_patterns(&b));

    // "both" sits at index 0 of A and at index 1 of B.
    patterns.remove(both, &mut registry);
    let newcomer = closed(&mut registry, 2, 1, &[img::a(), img::b()]);
    let newcomer = patterns.register(newcomer, &mut registry);

    assert_eq!([newcomer, only_a], registry.permanent_patterns(&a));
    assert_eq!([only_b, newcomer], registry.permanent_patterns(&b));
}

#[test]
fn amd_021_r2_008_the_mesh_of_a_tile_follows_the_new_candidate_order() {
    // The observable the review asked for: which pattern the shape of a lone A refers to.
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .row(&cells(&[img::a(), img::c()]))
        .load();

    let p1 = permanent(&engine, 0);
    let p2 = permanent(&engine, 1);

    let mut before = Batch::new();
    before.bg_cell(img::a(), 10, 10);
    let frame = before.convert(&mut engine, 0);
    assert_eq!(Some(p1), frame.shape(frame.shapes()[0]).reference_pattern());

    let (patterns, images) = engine.registries_mut();
    patterns.remove(p1, images);
    let cells = [Some(images.intern(img::a())), Some(images.intern(img::d()))];
    let mut p3 = EnginePattern::new(2, 1, cells, true);
    p3.set_self_connected(true);
    p3.add_instance(PatternInstance::new());
    let p3 = patterns.register(p3, images);

    let mut after = Batch::new();
    after.bg_cell(img::a(), 10, 10);
    let frame = after.convert(&mut engine, 0);
    assert_eq!(Some(p3), frame.shape(frame.shapes()[0]).reference_pattern());
    let frame = after.convert(&mut engine, 0);
    assert_ne!(Some(p2), frame.shape(frame.shapes()[0]).reference_pattern());
}

#[test]
fn req_e3d_093_a_tile_whose_patterns_are_all_disabled_is_not_displayed() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let pattern = permanent(&engine, 0);

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);
    assert_eq!(1, batch.convert(&mut engine, 1).shapes().len());

    engine.pattern_mut(pattern).unwrap().disabled = true;
    let frame = batch.convert(&mut engine, 2);
    assert!(frame.shapes().is_empty());

    // The image stays known, so pass 2 does not learn a pattern for it.
    assert_eq!(1, engine.patterns().permanent().len());
    assert!(engine.patterns().temporary().is_empty());

    engine.pattern_mut(pattern).unwrap().disabled = false;
    assert_eq!(1, batch.convert(&mut engine, 3).shapes().len());
}

#[test]
fn req_e3d_093_a_disabled_pattern_does_not_stop_another_germ_from_taking_the_tile() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a()]))
        .row_with(
            &cells(&[img::b(), img::a()]),
            PatternOptions {
                self_connected: false,
                ..PatternOptions::default()
            },
        )
        .load();
    let first = permanent(&engine, 0);
    let second = permanent(&engine, 1);
    engine.pattern_mut(first).unwrap().disabled = true;

    let mut batch = Batch::new();
    batch.bg_row(10, 10, &[img::b(), img::a()]);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(2, shape.tile_count());
    assert_eq!(Some(second), shape.reference_pattern());
}

#[test]
fn req_e3d_094_a_ring_is_not_full_a_u_and_a_line_are() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();

    // A 3x3 ring of 8 tiles, centre missing: the centre is surrounded, so not full.
    let mut ring = Batch::new();
    for dy in 0..3 {
        for dx in 0..3 {
            if dx != 1 || dy != 1 {
                ring.bg_cell(img::a(), 10 + dx, 10 + dy);
            }
        }
    }

    let frame = ring.convert(&mut engine, 1);
    assert!(!frame.is_full(frame.shapes()[0]));

    // A U shape: nothing strictly inside the box, so full.
    let mut u = Batch::new();
    u.bg_cell(img::a(), 10, 10)
        .bg_cell(img::a(), 10, 11)
        .bg_cell(img::a(), 11, 11)
        .bg_cell(img::a(), 12, 11)
        .bg_cell(img::a(), 12, 10);
    let frame = u.convert(&mut engine, 2);
    assert!(frame.is_full(frame.shapes()[0]));

    // A single line.
    let mut line = Batch::new();
    line.bg_row(10, 10, &[img::a(), img::a(), img::a()]);
    let frame = line.convert(&mut engine, 3);
    assert!(frame.is_full(frame.shapes()[0]));
}

#[test]
fn req_e3d_090_pass_1_keeps_a_full_region_of_eight_tiles_and_two_images() {
    let mut engine = ProfileBuilder::new()
        .pattern(2, 2, &cells(&[img::a(), img::b(), img::b(), img::a()]))
        .load();

    // A 4x2 chequerboard: full, 8 tiles, 2 images, so pass 1 keeps it.
    let mut batch = Batch::new();
    batch.bg_row(10, 10, &[img::a(), img::b(), img::a(), img::b()]);
    batch.bg_row(11, 10, &[img::b(), img::a(), img::b(), img::a()]);

    let frame = batch.convert(&mut engine, 0);
    assert_eq!(1, frame.shapes().len());
    assert_eq!(8, frame.shape(frame.shapes()[0]).tile_count());
}

#[test]
fn req_e3d_092_a_shape_rejected_by_pass_1_keeps_its_tiles_until_the_pass_ends() {
    // [A] alone would grow one tile at a time; a 7-tile region of one image is rejected by
    // pass 1 (fewer than 8 tiles, one image) and recovered whole by pass 3.
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let mut batch = Batch::new();
    for column in 0..7 {
        batch.bg_cell(img::a(), 10 + column, 10);
    }

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(7, frame.shape(frame.shapes()[0]).tile_count());
}

#[test]
fn req_e3d_132_a_sprite_cell_is_occupied_for_the_rest_of_the_frame() {
    let mut engine = ProfileBuilder::empty_engine();

    // Two unknown sprites of the same palette in the same cell, and a third adjacent to the
    // first: pass S1 cannot attach the second through the shared cell.
    let mut batch = Batch::new();
    batch
        .sprite(img::a(), 100, 50)
        .sprite(img::b(), 100, 50)
        .sprite(img::c(), 108, 50);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(2, frame.shapes().len());
    assert_eq!(2, frame.shape(frame.shapes()[0]).tile_count());
    let second = frame.shape(frame.shapes()[1]);
    assert_eq!(1, second.tile_count());
    assert_eq!(img::b(), second.images()[0].key());
}

#[test]
fn req_e3d_131_pass_s1_absorbs_a_known_neighbour() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::b()])).load();

    let mut batch = Batch::new();
    batch.sprite(img::u(), 100, 50).sprite(img::b(), 108, 50);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(2, frame.shape(frame.shapes()[0]).tile_count());
}

#[test]
fn req_e3d_131_a_germ_without_a_candidate_stays_free_in_pass_s2() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let pattern = permanent(&engine, 0);
    engine.pattern_mut(pattern).unwrap().disabled = true;

    let mut batch = Batch::new();
    batch.sprite(img::a(), 100, 50);
    let frame = batch.convert(&mut engine, 0);

    assert!(frame.shapes().is_empty());
    assert_eq!(1, frame.sprites().len());
    assert!(frame.sprites()[0].is_free());
}

#[test]
fn req_e3d_090_same_background_palette_stops_the_growth() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    engine.settings_mut().same_background_palette = true;

    let mut batch = Batch::new();
    batch
        .bg_cell(img::a(), 10, 10)
        .bg_cell_pal(img::a(), 11, 10, 1);
    assert_eq!(2, batch.convert(&mut engine, 0).shapes().len());

    engine.settings_mut().same_background_palette = false;
    assert_eq!(1, batch.convert(&mut engine, 1).shapes().len());
}

#[test]
fn req_e3d_253_each_tile_keeps_its_own_palette_attribute() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let mut batch = Batch::new();
    batch
        .bg_cell_pal(img::a(), 10, 10, 1)
        .bg_cell_pal(img::a(), 11, 10, 2);

    let frame = batch.convert(&mut engine, 0);
    let id = frame.shapes()[0];
    let shape = frame.shape(id);

    assert_eq!(2, shape.tile_count());
    assert_eq!(1, shape.palette());
    let tiles: Vec<_> = frame.shape_tiles(id).collect();
    assert_eq!(1, tiles[0].palette());
    assert_eq!(2, tiles[1].palette());
}

#[test]
fn req_e3d_341_the_stored_densities_are_kept_as_read() {
    let mut builder = ProfileBuilder::new();
    builder.row(&cells(&[img::a()]));
    let pattern = builder.permanent_pattern(0);
    pattern.pixel_count = 123;
    pattern.tile_density = PatternDensity::from_raw(6);
    pattern.pixel_density = PatternDensity::High;
    pattern.color_flags = PatternColorFlags::COLOR2;

    let engine = builder.load();

    let registered = engine.patterns().get(permanent(&engine, 0)).unwrap();
    assert_eq!(123, registered.stored_pixel_count);
    assert_eq!(6, registered.stored_tile_density);
    assert_eq!(4, registered.stored_pixel_density);
    assert_eq!(4, registered.stored_color_flags);
}

#[test]
fn req_e3d_342_the_automatic_layer_setting_has_no_effect() {
    let mut engine = ProfileBuilder::new()
        .pattern_with(
            1,
            1,
            &cells(&[img::a()]),
            PatternOptions {
                layer: ProfileLayer::Unidentified,
                ..PatternOptions::default()
            },
        )
        .load();

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);
    let before = instances(batch.convert(&mut engine, 1))[0].layer;

    engine.settings_mut().auto_layer = true;
    assert_eq!(before, instances(batch.convert(&mut engine, 2))[0].layer);
    assert_eq!(ZLayer::Unidentified, before);
}
