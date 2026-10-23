//! Determinism (REQ-ARCH-012), the learned state a savestate carries (REQ-E3D-008,
//! DEC-R2-T5), 2D mode (SPEC-04 §3.17) and the default texture of REQ-PROF-052.

use std::fmt::Write as _;

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::texture::{
    default_profile_texture, default_texture, is_default_texture,
};
use zax_engine3d::palette::FramePalette;
use zax_engine3d::state_codec;
use zax_engine3d::vectors::Cell;
use zax_profiles::enums::TextureFormat;
use zax_profiles::profile::GameProfile;
use zax_profiles::variant::{TexturePixels, VariantTexture};

use super::support::{
    cells, host, img, permanent, recorder, Batch, PatternOptions, ProfileBuilder,
    RecordingGeometryBuilder, RecordingScriptHost,
};

/// A textual digest of everything the converting frame exposes, used to compare two runs byte
/// for byte.
fn digest(engine: &mut ConversionEngine) -> String {
    let mut text = String::new();
    let frame = engine.converting_frame();
    writeln!(
        text,
        "frame {} t={} bg={} sp={} shapes={}/{}",
        frame.counter(),
        frame.time(),
        frame.background_tile_count(),
        frame.sprites().len(),
        frame.shapes().len(),
        frame.background_shape_count()
    )
    .unwrap();

    for shape in frame.shape_list() {
        let registration = shape
            .reference_pattern()
            .and_then(|id| engine.patterns().get(id))
            .map(zax_engine3d::pattern::EnginePattern::registration_index);
        writeln!(
            text,
            "  shape bg={} t={}-{} p={}-{} pal={} hidden={} tiles={} pattern={registration:?} completed={}",
            shape.background(),
            shape.tile_start(),
            shape.tile_end(),
            shape.pixel_start(),
            shape.pixel_end(),
            shape.palette(),
            shape.hidden(),
            shape.tile_count(),
            shape.completed()
        )
        .unwrap();
    }

    for &id in frame.instances() {
        let instance = frame.instance(id);
        writeln!(
            text,
            "  instance {} pos={} centre={} layer={:?} depth={} palette={}",
            instance.index(),
            instance.pos,
            instance.center,
            instance.layer,
            instance.depth,
            instance.palette_table()
        )
        .unwrap();
    }

    let mut palette = String::new();
    for entry in 0..FramePalette::ENTRY_COUNT {
        write!(palette, "{:02X}", frame.palette().get(entry).unwrap()).unwrap();
    }

    for tile in engine.build_draw_list_of(FrameRole::Converting).tiles() {
        writeln!(
            text,
            "  draw {},{} anchor={} centre={} mat={:?} pal={} cast={} sprite={}",
            tile.tile.x(),
            tile.tile.y(),
            tile.anchor,
            tile.tile_center,
            tile.material,
            tile.palette_table,
            tile.cast_shadow,
            tile.sprite
        )
        .unwrap();
    }

    text.push_str(&palette);
    text
}

fn sample_profile() -> GameProfile {
    ProfileBuilder::new()
        .row(&cells(&[img::a(), img::b()]))
        .row_with(
            &cells(&[img::c()]),
            PatternOptions {
                self_connected: false,
                ..PatternOptions::default()
            },
        )
        .pattern(2, 2, &cells(&[img::d(), img::a(), img::a(), img::d()]))
        .row_with(
            &cells(&[img::full(10)]),
            PatternOptions {
                permanent: false,
                ..PatternOptions::default()
            },
        )
        .build()
}

fn sample_batches() -> Vec<Batch> {
    (0..6)
        .map(|n| {
            let mut batch = Batch::new();
            batch
                .palette(0, 0x0F + u8::try_from(n).unwrap())
                .palette(5, 0x16)
                .mask(8, 0);
            batch.bg_row(
                5 + n,
                4,
                &[img::a(), img::b(), img::a(), img::b(), img::c()],
            );
            batch.bg_row(6 + n, 4, &[img::d(), img::a(), img::a(), img::d()]);
            batch.bg_cell(img::full(20 + n.unsigned_abs()), 20, 20);
            batch.bg_cell(img::full(10), 0, 10);
            batch
                .sprite(img::a(), 100 + n, 50)
                .sprite(img::b(), 108 + n, 50);
            batch.sprite(img::full(30 + n.unsigned_abs()), 200, 100);
            batch
        })
        .collect()
}

fn loaded(profile: &GameProfile) -> ConversionEngine {
    let mut engine = ConversionEngine::new();
    engine.load_profile(profile, true).unwrap();
    engine
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02X}").unwrap();
        text
    })
}

#[test]
fn req_arch_012_two_runs_of_the_same_batches_give_the_same_frames() {
    fn run() -> String {
        let mut engine = loaded(&sample_profile());
        let mut log = String::new();
        for (counter, batch) in (1..).zip(sample_batches()) {
            batch.convert(&mut engine, counter);
            log.push_str(&digest(&mut engine));
            engine.publish();
        }

        log.push_str(&hex(&state_codec::write(&engine)));
        log
    }

    assert_eq!(run(), run());
}

#[test]
fn req_arch_011_two_engines_in_the_same_process_are_independent() {
    let mut a = loaded(&sample_profile());
    let mut b = loaded(&ProfileBuilder::new().row(&cells(&[img::c()])).build());

    let mut interleaved = String::new();
    for (counter, batch) in (1..).zip(sample_batches()) {
        batch.convert(&mut a, counter);
        interleaved.push_str(&digest(&mut a));
        let mut other = Batch::new();
        other.bg_cell(img::c(), 3, 3);
        other.convert(&mut b, counter);
    }

    let mut alone = loaded(&sample_profile());
    let mut solo = String::new();
    for (counter, batch) in (1..).zip(sample_batches()) {
        batch.convert(&mut alone, counter);
        solo.push_str(&digest(&mut alone));
    }

    assert_eq!(solo, interleaved);
}

fn describe(engine: &ConversionEngine) -> String {
    let mut text = String::new();
    let patterns = engine.patterns();
    for pattern in patterns
        .permanent()
        .iter()
        .map(|&id| patterns.get(id).unwrap())
    {
        writeln!(
            text,
            "perm {}x{} |Img|={} sc={} skip={} c={} ins={} depth={} layer={:?}",
            pattern.width(),
            pattern.height(),
            pattern.key().count(),
            pattern.self_connected(),
            pattern.skip_geometry,
            pattern.constraint_size,
            pattern.instances().len(),
            pattern.instances()[0].depth,
            pattern.instances()[0].layer
        )
        .unwrap();
    }

    for pattern in patterns
        .temporary()
        .iter()
        .map(|&id| patterns.get(id).unwrap())
    {
        writeln!(
            text,
            "temp {}x{} |Img|={}",
            pattern.width(),
            pattern.height(),
            pattern.key().count()
        )
        .unwrap();
    }

    let tracker = engine.tracker();
    writeln!(
        text,
        "tracker running={} sp={} pos={} in={} out={}",
        tracker.running(),
        tracker.sprites(),
        tracker.position(),
        tracker.included().len(),
        tracker.excluded().len()
    )
    .unwrap();
    text
}

#[test]
fn dec_r2_t5_the_learned_state_round_trips() {
    let profile = sample_profile();
    let mut engine = loaded(&profile);

    for (counter, batch) in (1..).zip(sample_batches()) {
        batch.convert(&mut engine, counter);
        engine.publish();
    }

    // Something really was learned.
    let profile_count = usize::try_from(engine.patterns().profile_pattern_count()).unwrap();
    assert!(engine.patterns().permanent().len() > profile_count - 1);
    assert_ne!(engine.patterns().temporary(), []);
    assert!(engine.images().count() > engine.profile_image_count());

    let tracked = engine.published_frame().unwrap().shapes()[0];
    engine.start_tracker(FrameRole::Published, tracked);
    engine.track(FrameRole::Published);

    let state = state_codec::write(&engine);
    assert!(state_codec::fits_reserved_capacity(state.len()));

    // The same state written twice gives the same bytes.
    assert_eq!(state, state_codec::write(&engine));

    let before = describe(&engine);

    // A fresh engine with the same profile, then the state put back.
    let mut restored = loaded(&sample_profile());
    state_codec::read(&mut restored, &state).unwrap();

    assert_eq!(before, describe(&restored));
    assert_eq!(engine.images().count(), restored.images().count());
    assert_eq!(hex(&state), hex(&state_codec::write(&restored)));

    // And the next frame converts the same way on both.
    let mut next = Batch::new();
    next.bg_row(12, 4, &[img::a(), img::b(), img::c()])
        .sprite(img::full(33), 60, 60);
    next.convert(&mut engine, 99);
    next.convert(&mut restored, 99);
    assert_eq!(digest(&mut engine), digest(&mut restored));
}

#[test]
fn dec_r2_t5_the_restored_patterns_get_their_geometry_back() {
    let mut engine = loaded(&sample_profile());
    let mut learning = Batch::new();
    learning.bg_cell(img::u(), 10, 10);
    learning.convert(&mut engine, 1);
    let state = state_codec::write(&engine);

    let mut restored = ConversionEngine::new();
    restored.set_geometry(Box::new(RecordingGeometryBuilder::default()));
    restored.set_script_host(Box::new(RecordingScriptHost::default()));
    restored.load_profile(&sample_profile(), true).unwrap();
    let built_at_load = recorder(&restored).built.len();
    let hooked_at_load = host(&restored).built_patterns.len();

    state_codec::read(&mut restored, &state).unwrap();

    // Two patterns come back: the learned one and the profile's temporary pattern, which
    // belongs to the learned state as well (DEC-R2-T5).
    assert_eq!(built_at_load + 2, recorder(&restored).built.len());
    assert_eq!(hooked_at_load + 2, host(&restored).built_patterns.len());
}

#[test]
fn dec_r2_t5_a_state_written_with_another_profile_is_refused() {
    let mut engine = loaded(&sample_profile());
    let state = state_codec::write(&engine);

    let mut other = loaded(&ProfileBuilder::new().row(&cells(&[img::c()])).build());

    assert!(state_codec::read(&mut other, &state).is_err());
    assert!(state_codec::read(&mut engine, &[0; 8]).is_err());
}

fn two_d_engine() -> ConversionEngine {
    let mut engine = ConversionEngine::new();
    engine.two_dimensional_mode = true;
    engine
        .load_profile(&GameProfile::create_empty(), true)
        .unwrap();
    engine
}

#[test]
fn req_e3d_380_two_dimensional_mode_starts_from_an_empty_registry() {
    let mut engine = two_d_engine();

    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10);
    batch.convert(&mut engine, 1);
    assert_eq!(1, engine.patterns().permanent().len());

    engine.unload();
    assert_eq!(engine.patterns().permanent(), []);
    assert_eq!(0, engine.images().count());

    engine
        .load_profile(&GameProfile::create_empty(), true)
        .unwrap();
    assert_eq!(engine.patterns().permanent(), []);
}

#[test]
fn req_e3d_381_a_two_image_brick_wall_gives_a_two_tile_shape_in_2d_mode() {
    let mut engine = two_d_engine();

    let mut batch = Batch::new();
    batch.bg_row(
        10,
        4,
        &[img::a(), img::b(), img::a(), img::b(), img::a(), img::b()],
    );
    let frame = batch.convert(&mut engine, 1);

    // Pass 2 stops at one tile per image, instead of swallowing the whole wall.
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(2, shape.tile_count());
    assert_eq!(2, shape.images().len());

    // In 3D mode the same batch gives a single-tile shape from pass 2 instead.
    let mut three_d = ProfileBuilder::empty_engine();
    let frame = batch.convert(&mut three_d, 1);
    assert_eq!(1, frame.shape(frame.shapes()[0]).tile_count());
}

#[test]
fn req_e3d_381_a_big_unfilled_shape_is_cut_down_to_its_first_full_band() {
    let mut engine = two_d_engine();

    // Row 10 holds 28 distinct images, row 11 only 10: the shape is big and does not fill its
    // box, so the first run of entirely full rows is kept.
    let mut batch = Batch::new();
    for column in 0i32..28 {
        batch.bg_cell(img::full(column.unsigned_abs()), column, 10);
    }

    for column in 0i32..10 {
        batch.bg_cell(img::full(28 + column.unsigned_abs()), column, 11);
    }

    let frame = batch.convert(&mut engine, 1);

    let id = frame.shapes()[0];
    let band = frame.shape(id);
    assert_eq!(28, band.tile_count());
    assert_eq!(Cell::new(28, 1), band.tile_size());
    assert!(frame.shape_tiles(id).all(|tile| tile.y() == 80));
}

#[test]
fn req_prof_052_the_default_texture_of_a_pattern() {
    let mut builder = ProfileBuilder::new();
    builder.pattern(2, 1, &[Some(img::pixel(0, 0, false, false)), None]);
    let texels = default_profile_texture(&builder.build().permanent_patterns[0]);

    assert_eq!(16 * 8, texels.len());
    assert_eq!(1, texels[0]);
    assert_eq!(0, texels[1]);

    // The empty cell is all zero.
    for y in 0..8 {
        for x in 8..16 {
            assert_eq!(0, texels[x + (y * 16)]);
        }
    }

    let engine = builder.load();
    let pattern = engine.patterns().get(permanent(&engine, 0)).unwrap();
    assert_eq!(texels, default_texture(pattern));
}

#[test]
fn req_prof_052_a_texture_equal_to_the_default_one_is_recognised() {
    let mut builder = ProfileBuilder::new();
    builder.pattern(1, 1, &[Some(img::pixel(0, 0, false, false))]);
    let pattern = builder.build().permanent_patterns[0].clone();

    // No texture and the null marker both mean "use the default".
    assert!(is_default_texture(None, &pattern));
    assert!(is_default_texture(
        Some(&VariantTexture::null_marker()),
        &pattern
    ));

    // An explicit copy of the default texture is recognised as such.
    let texels = default_profile_texture(&pattern);
    let words = texels.len().div_ceil(16);
    let mut data = vec![0; words];
    let mut extra = vec![0; words];
    for (n, &texel) in texels.iter().enumerate() {
        data[n / 16] |= (i32::from(texel) & 3) << (2 * (n % 16));
        extra[n / 16] |= ((i32::from(texel) >> 2) & 1) << (n % 16);
    }

    let mut texture = VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(TextureFormat::Indexed, data, extra)),
    );
    assert!(is_default_texture(Some(&texture), &pattern));

    // One texel apart it is not.
    texture.pixels.as_mut().unwrap().data[0] ^= 3;
    assert!(!is_default_texture(Some(&texture), &pattern));
}
