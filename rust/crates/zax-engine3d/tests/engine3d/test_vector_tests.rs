//! The fourteen test vectors of SPEC-04 §5 (TV-E3D-01 to TV-E3D-14).

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::frame::ConversionFrame;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::{Geo3DType, ZLayer};
use zax_engine3d::vectors::{Cell, Vec2, Vec3};

use super::support::{
    assert_close, cells, img, instances, permanent, Batch, PatternOptions, ProfileBuilder,
    RecordingScriptHost,
};

fn not_connected() -> PatternOptions {
    PatternOptions {
        self_connected: false,
        ..PatternOptions::default()
    }
}

fn tile_counts(frame: &ConversionFrame) -> Vec<usize> {
    frame
        .shape_list()
        .map(zax_engine3d::frame::FrameShape::tile_count)
        .collect()
}

#[test]
fn tv_e3d_01_region_of_a_periodic_pattern_req_e3d_090_095() {
    // Profile [A B] sc true. Row 5: A B A B A B at columns 10 to 15.
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .load();
    let first = permanent(&engine, 0);
    let mut batch = Batch::new();
    batch.bg_row(
        5,
        10,
        &[img::a(), img::b(), img::a(), img::b(), img::a(), img::b()],
    );

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert!(shape.background());
    assert_eq!(6, shape.tile_count());
    assert_eq!(Some(first), shape.reference_pattern());
    assert!(shape.completed());
}

#[test]
fn tv_e3d_01_eight_tiles_are_kept_by_pass_1_req_e3d_092() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .load();
    let mut batch = Batch::new();
    batch.bg_row(
        5,
        8,
        &[
            img::a(),
            img::b(),
            img::a(),
            img::b(),
            img::a(),
            img::b(),
            img::a(),
            img::b(),
        ],
    );

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(8, frame.shape(frame.shapes()[0]).tile_count());

    // A row of 8 tiles is full, has 8 tiles and 2 distinct images, so pass 1 keeps it.
    assert!(frame.is_full(frame.shapes()[0]));
}

#[test]
fn tv_e3d_02_self_connected_false_req_e3d_061() {
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b()]), not_connected())
        .load();
    let mut batch = Batch::new();
    batch.bg_row(
        5,
        10,
        &[img::a(), img::b(), img::a(), img::b(), img::a(), img::b()],
    );

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(3, frame.shapes().len());
    for shape in frame.shape_list() {
        assert_eq!(2, shape.tile_count());
        assert_eq!(2, shape.images().len());
    }

    // Left to right.
    let starts: Vec<i32> = frame
        .shape_list()
        .map(|shape| shape.tile_start().x)
        .collect();
    assert_eq!(vec![11, 13, 15], starts);
}

#[test]
fn tv_e3d_02_the_pair_b_a_does_not_exist_req_e3d_061() {
    let mut not_connected_engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b()]), not_connected())
        .load();
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::b(), img::a()]);
    assert_eq!(
        2,
        batch.convert(&mut not_connected_engine, 0).shapes().len()
    );

    let mut connected = ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .load();
    assert_eq!(1, batch.convert(&mut connected, 0).shapes().len());
}

#[test]
fn tv_e3d_03_constraint_size_req_e3d_062() {
    let mut engine = ProfileBuilder::new()
        .row_with(
            &cells(&[img::a()]),
            PatternOptions {
                constraint_width: 2,
                constraint_height: 99,
                ..PatternOptions::default()
            },
        )
        .load();
    let first = permanent(&engine, 0);
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::a(); 5]);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(3, frame.shapes().len());
    assert_eq!(vec![2, 2, 1], tile_counts(frame));
    assert!(frame
        .shape_list()
        .all(|shape| shape.reference_pattern() == Some(first)));
}

#[test]
fn tv_e3d_03_constraint_on_the_other_axis_req_e3d_062() {
    let mut engine = ProfileBuilder::new()
        .row_with(
            &cells(&[img::a()]),
            PatternOptions {
                constraint_width: 99,
                constraint_height: 1,
                ..PatternOptions::default()
            },
        )
        .load();
    let mut batch = Batch::new();
    batch
        .bg_cell(img::a(), 10, 5)
        .bg_cell(img::a(), 10, 6)
        .bg_cell(img::a(), 10, 7);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(3, frame.shapes().len());
    assert!(frame.shape_list().all(|shape| shape.tile_count() == 1));
}

#[test]
fn tv_e3d_04_skip_geo_req_e3d_062() {
    let mut engine = ProfileBuilder::new()
        .row_with(
            &cells(&[img::a(), img::b()]),
            PatternOptions {
                self_connected: false,
                skip_geometry: true,
                ..PatternOptions::default()
            },
        )
        .load();
    let mut batch = Batch::new();
    batch.bg(img::a(), 80, 40, 0).bg(img::b(), 80, 48, 0);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(2, frame.shape(frame.shapes()[0]).tile_count());
}

#[test]
fn tv_e3d_04_without_skip_geo_the_vertical_pair_is_missing_req_e3d_062() {
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b()]), not_connected())
        .load();
    let mut batch = Batch::new();
    batch.bg(img::a(), 80, 40, 0).bg(img::b(), 80, 48, 0);

    assert_eq!(2, batch.convert(&mut engine, 0).shapes().len());
}

#[test]
fn tv_e3d_05_candidate_shrinking_and_smallest_pattern_req_e3d_090() {
    // P1 = [A B], P2 = [A B C], both sc false, in this order.
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b()]), not_connected())
        .row_with(&cells(&[img::a(), img::b(), img::c()]), not_connected())
        .load();
    let p1 = permanent(&engine, 0);
    let p2 = permanent(&engine, 1);

    let mut ab = Batch::new();
    ab.bg_row(5, 10, &[img::a(), img::b()]);
    let frame = ab.convert(&mut engine, 0);
    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(Some(p1), shape.reference_pattern());
    assert!(shape.completed());

    let mut abc = Batch::new();
    abc.bg_row(5, 10, &[img::a(), img::b(), img::c()]);
    let frame = abc.convert(&mut engine, 0);
    assert_eq!(1, frame.shapes().len());
    assert_eq!(Some(p2), frame.shape(frame.shapes()[0]).reference_pattern());

    let mut cab = Batch::new();
    cab.bg_row(5, 10, &[img::c(), img::a(), img::b()]);
    let frame = cab.convert(&mut engine, 0);
    assert_eq!(2, frame.shapes().len());
    let first = frame.shape(frame.shapes()[0]);
    assert_eq!(1, first.tile_count());
    assert_eq!(Some(p2), first.reference_pattern());
    assert!(!first.completed());
    let second = frame.shape(frame.shapes()[1]);
    assert_eq!(2, second.tile_count());
    assert_eq!(Some(p1), second.reference_pattern());
}

fn passes_engine() -> ConversionEngine {
    ProfileBuilder::new()
        .pattern(2, 2, &cells(&[img::a(), img::b(), img::b(), img::a()]))
        .row(&cells(&[img::d()]))
        .load()
}

fn passes_batch() -> Batch {
    let mut batch = Batch::new();
    batch.bg_row(2, 4, &[img::a(), img::b(), img::a(), img::b()]);
    batch.bg_row(3, 4, &[img::b(), img::a(), img::b(), img::a()]);
    batch.bg(img::u(), 160, 0, 0);
    batch.bg(img::d(), 160, 120, 0);
    batch
}

#[test]
fn tv_e3d_06_order_of_the_passes_req_e3d_092() {
    // Q = [A B; B A] sc true, and [D] sc true.
    let mut engine = passes_engine();
    let q = permanent(&engine, 0);
    let d = permanent(&engine, 1);

    let frame = passes_batch().convert(&mut engine, 0);

    assert_eq!(3, frame.shapes().len());

    // (1) the chequerboard, 8 tiles, pattern Q, pass 1.
    let first = frame.shape(frame.shapes()[0]);
    assert_eq!(8, first.tile_count());
    assert_eq!(Some(q), first.reference_pattern());

    // (2) the unknown tile U at the top border, pass 2: a temporary pattern is learned.
    let second = frame.shape(frame.shapes()[1]);
    assert_eq!(1, second.tile_count());
    assert!(second.at_border());
    let second_pattern = second.reference_pattern();

    // (3) the D tile, pass 3.
    let third = frame.shape(frame.shapes()[2]);
    assert_eq!(1, third.tile_count());
    assert_eq!(Some(d), third.reference_pattern());

    assert_eq!(1, engine.patterns().temporary().len());
    assert_eq!(Some(engine.patterns().temporary()[0]), second_pattern);
}

#[test]
fn tv_e3d_06_the_passes_win_over_the_germination_order_req_e3d_092() {
    let mut engine = passes_engine();
    let d = permanent(&engine, 1);
    engine.settings_mut().scan_direction = 1;

    let frame = passes_batch().convert(&mut engine, 0);

    assert_eq!(3, frame.shapes().len());
    assert_eq!(8, frame.shape(frame.shapes()[0]).tile_count());
    let second = frame.shape(frame.shapes()[1]).reference_pattern();
    assert_eq!(Some(d), frame.shape(frame.shapes()[2]).reference_pattern());
    assert_eq!(Some(engine.patterns().temporary()[0]), second);
}

#[test]
fn tv_e3d_07_learning_req_e3d_160_161() {
    let mut engine = ProfileBuilder::empty_engine();

    let mut batch = Batch::new();
    batch.bg_row(10, 10, &[img::u(), img::u(), img::u()]);
    let frame = batch.convert(&mut engine, 1);

    assert_eq!(2, frame.shapes().len());

    // Pass 2 learns a permanent 1x1 pattern from the first U.
    let first = frame.shape(frame.shapes()[0]);
    assert_eq!(1, first.tile_count());
    let first_pattern = first.reference_pattern();

    // The two remaining U are known by then, so pass 3 groups them.
    let second = frame.shape(frame.shapes()[1]);
    assert_eq!(2, second.tile_count());
    let second_pattern = second.reference_pattern();

    assert_eq!(1, engine.patterns().permanent().len());
    let learned = permanent(&engine, 0);
    assert_eq!(Some(learned), first_pattern);
    assert_eq!(Some(learned), second_pattern);
    let pattern = engine.patterns().get(learned).unwrap();
    assert_eq!(1, pattern.width());
    assert_eq!(1, pattern.height());
    assert!(pattern.self_connected());
    assert!(!pattern.skip_geometry);
    assert_eq!(Cell::new(99, 99), pattern.constraint_size);
    assert_eq!(1, pattern.instances().len());
    assert_eq!(3, pattern.instances()[0].depth);
    assert_eq!(ZLayer::L1, pattern.instances()[0].layer);
    assert_eq!(Geo3DType::Default, pattern.instances()[0].geometry);

    let frame = batch.convert(&mut engine, 2);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(3, shape.tile_count());
    assert_eq!(Some(learned), shape.reference_pattern());
}

#[test]
fn tv_e3d_07_a_border_tile_only_learns_a_temporary_pattern_req_e3d_160() {
    let mut engine = ProfileBuilder::empty_engine();

    for counter in 1..=3 {
        let mut batch = Batch::new();
        batch.mask(8, 0).bg_cell(img::u(), 0, 10);
        let frame = batch.convert(&mut engine, counter);

        assert_eq!(1, frame.shapes().len());
        let shape = frame.shape(frame.shapes()[0]);
        assert!(shape.at_border());
        let reference = shape.reference_pattern();
        let image = shape.images()[0];
        assert_eq!(engine.patterns().permanent(), []);
        assert_eq!(1, engine.patterns().temporary().len());
        assert_eq!(Some(engine.patterns().temporary()[0]), reference);
        assert!(!engine.images().is_known(&image));
    }
}

#[test]
fn tv_e3d_08_sprites_with_an_empty_profile_req_e3d_131() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .sprite(img::a(), 100, 50)
        .sprite(img::b(), 108, 50)
        .sprite(img::c(), 116, 52);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(2, frame.shapes().len());
    assert_eq!(0, frame.background_shape_count());
    assert_eq!(vec![2, 1], tile_counts(frame));
    assert!(frame.shape_list().all(|shape| shape.palette() == 4));

    assert_eq!(2, engine.patterns().permanent().len());
    let first = engine.patterns().get(permanent(&engine, 0)).unwrap();
    assert_eq!(2, first.width());
    assert_eq!(1, first.height());
    assert_eq!(
        1,
        engine
            .patterns()
            .get(permanent(&engine, 1))
            .unwrap()
            .width()
    );
}

fn abc_batch(b_palette: u8) -> Batch {
    let mut batch = Batch::new();
    batch
        .sprite(img::a(), 100, 50)
        .sprite_full(img::b(), 108, 50, b_palette, false)
        .sprite(img::c(), 116, 52);
    batch
}

#[test]
fn tv_e3d_08_sprite_tolerance_req_e3d_130_131() {
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b(), img::c()]), not_connected())
        .load();
    let first = permanent(&engine, 0);
    engine.settings_mut().sprite_tolerance = 2;

    let frame = abc_batch(0).convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    assert_eq!(3, frame.shape(frame.shapes()[0]).tile_count());

    engine.settings_mut().sprite_tolerance = 0;
    let tight = abc_batch(0).convert(&mut engine, 1);
    assert_eq!(2, tight.shapes().len());
    assert!(tight
        .shape_list()
        .all(|shape| shape.reference_pattern() == Some(first)));
}

#[test]
fn tv_e3d_08_same_sprite_palette_req_e3d_131() {
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a(), img::b(), img::c()]), not_connected())
        .load();
    engine.settings_mut().sprite_tolerance = 2;
    engine.settings_mut().same_sprite_palette = true;

    assert_eq!(3, abc_batch(1).convert(&mut engine, 0).shapes().len());

    engine.settings_mut().same_sprite_palette = false;
    let merged = abc_batch(1).convert(&mut engine, 1);
    assert_eq!(1, merged.shapes().len());
    assert_eq!(4, merged.shape(merged.shapes()[0]).palette());
}

#[test]
fn tv_e3d_09_frame_palette_req_e3d_250_251() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .palette(0, 0x0F)
        .palette(1, 0x30)
        .palette(4, 0x12)
        .palette(5, 0x16)
        .palette(17, 0x27);

    let frame = batch.convert(&mut engine, 0);
    let palette = frame.palette();

    assert_eq!(Some(0x0F), palette.get(0));
    assert_eq!(Some(0x30), palette.get(1));
    assert_eq!(Some(0x0F), palette.get(4));
    assert_eq!(Some(0x16), palette.get(5));
    assert_eq!(Some(0x0F), palette.get(16));
    assert_eq!(Some(0x27), palette.get(17));
    assert_eq!(Some(0x16), palette.get(37));
    assert_eq!(Some(0x27), palette.get(49));

    assert_eq!(Some(Rgb24::new(0xFC, 0x38, 0x00)), palette.color(5));
    assert_eq!(Some(Rgb24::new(0xFC, 0xA0, 0x48)), palette.color(17));
}

#[test]
fn tv_e3d_10_image_identity_and_flips_req_e3d_020() {
    let x = img::pixel(1, 2, false, false);
    let mut engine = ProfileBuilder::new().row(&[Some(x)]).load();
    let original = permanent(&engine, 0);

    let mut batch = Batch::new();
    batch.sprite(img::flipped(x, true, false), 100, 50);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    let flipped = shape.images()[0];
    assert_eq!(1, flipped.pixel_count());
    assert_eq!(Cell::new(6, 2), flipped.pixel_start());
    assert_eq!(Cell::new(6, 2), flipped.pixel_end());
    let reference = shape.reference_pattern();

    // X' is unknown: the sprite made a new pattern and is not bound to [X].
    assert_eq!(2, engine.patterns().permanent().len());
    assert_ne!(Some(original), reference);
    assert_eq!(Some(permanent(&engine, 1)), reference);
}

#[test]
fn tv_e3d_11_clipping_req_e3d_002_003_005() {
    let mut engine = ProfileBuilder::empty_engine();
    let mut batch = Batch::new();
    batch
        .bg(img::a(), -8, 100, 0)
        .bg(img::b(), 248, 100, 0)
        .sprite(img::c(), 100, 240)
        .bg(img::d(), -7, -7, 0);

    let frame = batch.convert(&mut engine, 0);

    assert_eq!(frame.sprites(), []);
    let kept = frame.background_at_cell(Cell::new(1, 1)).unwrap();
    assert_eq!(-7, kept.x());
    assert_eq!(-7, kept.y());
    assert_eq!(1, frame.background_tile_count());
}

#[test]
fn tv_e3d_12_tracker_req_e3d_300() {
    // The vector is expressed in shape centres, so the frames are built so that the sprite
    // shapes have the required ones: a one-tile shape at (x, y) has centre (x + 4, y + 4).
    let mut engine = ProfileBuilder::empty_engine();

    let mut start = Batch::new();
    start.sprite(img::a(), 96, 96);
    let frame = start.convert(&mut engine, 1);
    let tracked = frame.shapes()[0];
    assert_eq!(Vec3::new(100.0, 100.0, 0.0), frame.shape(tracked).center());
    let pattern_t = frame.shape(tracked).reference_pattern().unwrap();

    engine.start_tracker(FrameRole::Converting, tracked);

    // Frame 2: X (another pattern) at centre (104, 100) then Y (pattern of T) at (140, 100).
    let mut second = Batch::new();
    second.sprite(img::b(), 100, 96).sprite(img::a(), 136, 96);
    let frame = second.convert(&mut engine, 2);
    assert_eq!(2, frame.shapes().len());
    let y = frame.shapes()[1];
    let pattern_x = frame.shape(frame.shapes()[0]).reference_pattern().unwrap();

    let found = engine.track(FrameRole::Converting);
    assert_eq!(Some(y), found);
    assert_eq!(
        Some(pattern_t),
        engine.converting_frame().shape(y).reference_pattern()
    );
    assert!(engine.tracker().excluded().contains(&pattern_x));
    assert_eq!(Vec3::new(140.0, 100.0, 0.0), engine.tracker().position());

    // Frame 3: X alone at (141, 100): its pattern is excluded, nothing is found.
    let mut third = Batch::new();
    third.sprite(img::b(), 137, 96);
    third.convert(&mut engine, 3);
    assert_eq!(None, engine.track(FrameRole::Converting));
    assert_eq!(Vec3::new(140.0, 100.0, 0.0), engine.tracker().position());

    // Frame 4: Z at centre (145, 103), 5.83 pixels away: found by proximity.
    let mut fourth = Batch::new();
    fourth.sprite(img::c(), 141, 99);
    let frame = fourth.convert(&mut engine, 4);
    let expected = frame.shapes()[0];
    let z = engine.track(FrameRole::Converting).unwrap();
    assert_eq!(expected, z);
    let z_pattern = engine
        .converting_frame()
        .shape(z)
        .reference_pattern()
        .unwrap();
    assert!(engine.tracker().included().contains(&z_pattern));
    assert_eq!(Vec3::new(145.0, 103.0, 0.0), engine.tracker().position());
}

#[test]
fn tv_e3d_13_a_frame_instance_is_reset_every_frame_req_e3d_190() {
    let tagged = PatternOptions {
        tags: Some("t".to_owned()),
        ..PatternOptions::default()
    };
    let mut engine = ProfileBuilder::new()
        .row_with(&cells(&[img::a()]), tagged.clone())
        .load();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update_shape: Some(Box::new(|instance| {
            instance.offset.x += 5.0;
            Ok(())
        })),
        ..RecordingScriptHost::default()
    }));

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 5);
    let positions: Vec<f32> = (1..=3)
        .map(|counter| instances(batch.convert(&mut engine, counter))[0].pos.x)
        .collect();

    assert_eq!(3, positions.len());
    assert_eq!(positions[0], positions[1]);
    assert_eq!(positions[1], positions[2]);

    // Without the script the anchor would be at the centre of the tile box.
    let mut plain = ProfileBuilder::new()
        .row_with(&cells(&[img::a()]), tagged)
        .load();
    let expected = instances(batch.convert(&mut plain, 0))[0].pos.x;
    assert_close(expected + 5.0, positions[0], 3);
}

#[test]
fn tv_e3d_14_injected_shape_req_e3d_230() {
    let mut engine = ProfileBuilder::new().row(&cells(&[img::a()])).load();
    let pattern = permanent(&engine, 0);

    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            state
                .create_new_shape(pattern, Vec3::new(10.5, 20.25, 3.0), 5)
                .unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let frame = Batch::new().convert(&mut engine, 1);

    assert_eq!(1, frame.shapes().len());
    let id = frame.shapes()[0];
    let shape = frame.shape(id);
    assert!(!shape.background());
    assert!(!shape.scriptable());
    assert_eq!(frame.scriptable_instances(), []);

    let tile = frame.shape_tiles(id).next().unwrap();
    assert_eq!(10, tile.x());
    assert_eq!(221, tile.y());
    assert_eq!(1, tile.palette());
    assert!(!tile.hidden());

    assert_eq!(Cell::new(10, 221), shape.pixel_start());
    assert_eq!(Cell::new(17, 228), shape.pixel_end());
    assert_eq!(Vec2::new(10.0, 19.0), shape.bottom_left());

    let instance = &shape.instances()[0];
    assert_close(0.5, instance.offset.x, 4);
    assert_close(-0.25, instance.offset.y, 4);
    assert_close(3.0, instance.offset.z, 4);
    assert_close(14.5, instance.pos.x, 4);
    assert_close(22.75, instance.pos.y, 4);
    assert_close(4.5, instance.pos.z, 4);
}
