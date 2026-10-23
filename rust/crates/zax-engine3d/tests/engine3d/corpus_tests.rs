//! The pipeline run against the 101 built-in profiles (REQ-E3D-060) and the performance
//! budget of REQ-E3D-360. Every test here needs the profiles (`ZAXNES_PROFILES`).
//!
//! No ROM is needed: the batches are synthetic, which is what DEC-R2-T7 asks of an acceptance
//! criterion of the clean side.

use std::fs;
use std::time::Instant;

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::image::TileImageKey;
use zax_engine3d::state_codec;

use super::support::{img, neutral_assets, Batch, RecordingGeometryBuilder};

const SMOKE_PROFILES: [&str; 10] = [
    "empty.bytes",
    "Contra.bytes",
    "Final_Fantasy_USA.bytes",
    "Jackal (U).bytes",
    "Castlevania (U) (PRG 1).bytes",
    "Mega Man 2.bytes",
    "punchout.bytes",
    "Felix the Cat.bytes",
    "Adventures of Lolo (U).bytes",
    "DQ3_DW3.bytes",
];

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn req_e3d_060_every_built_in_profile_registers_all_its_patterns() {
    let mut problems = Vec::new();
    let mut total_patterns = 0;
    let mut total_instances = 0;

    for path in neutral_assets::profile_files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let profile = neutral_assets::load_path(&path);
        let mut engine = ConversionEngine::new();
        engine.load_profile(&profile, true).unwrap();

        let patterns = engine.patterns();
        let expected = profile.permanent_patterns.len() + profile.temporary_patterns.len();
        let registered = patterns.permanent().len() + patterns.temporary().len();
        if registered != expected {
            problems.push(format!(
                "{name}: {registered} registered for {expected} in the file"
            ));
        }

        if patterns.permanent().len() != profile.permanent_patterns.len() {
            problems.push(format!("{name}: two permanent patterns share a key"));
        }

        total_patterns += registered;
        total_instances += patterns
            .all()
            .map(|id| patterns.get(id).unwrap().instances().len())
            .sum::<usize>();
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));

    // SPEC-04B annex A: 67 832 patterns and 74 389 3D objects in the 101 profiles.
    assert_eq!(67832, total_patterns);
    assert_eq!(74389, total_instances);
}

/// Real tile images of the profile, so that the recognition has something to match.
fn profile_images(profile: &zax_profiles::profile::GameProfile) -> Vec<TileImageKey> {
    let mut images = Vec::new();
    for pattern in &profile.permanent_patterns {
        let Some(grid) = pattern.grid.as_ref() else {
            continue;
        };

        for tile in grid.cells().iter().flatten() {
            if images.len() < 64 {
                images.push(TileImageKey::new(
                    tile.low_plane,
                    tile.high_plane,
                    tile.flip_x(),
                    tile.flip_y(),
                ));
            }
        }

        if images.len() >= 64 {
            break;
        }
    }

    if images.is_empty() {
        images.push(img::a());
    }

    images
}

fn full_frame_of(images: &[TileImageKey], start: usize) -> Batch {
    let mut batch = Batch::new();
    let mut n = start;
    for row in 0..31 {
        for column in 0..33 {
            let palette = u8::try_from((row + column) & 3).unwrap();
            batch.bg_cell_pal(images[n % images.len()], column, row, palette);
            n += 1;
        }
    }

    for i in 0..64 {
        let palette = u8::try_from(i & 3).unwrap();
        let image = images[(n + usize::try_from(i).unwrap()) % images.len()];
        batch.sprite_full(image, (i * 3) % 240, 8 + ((i * 5) % 220), palette, false);
    }

    batch
}

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn req_e3d_001_the_pipeline_runs_on_synthetic_frames_with_built_in_profiles() {
    for name in SMOKE_PROFILES {
        let profile = neutral_assets::load(name);
        let mut engine = ConversionEngine::new();
        engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));
        engine.two_dimensional_mode = name == "empty.bytes";
        engine.load_profile(&profile, true).unwrap();

        let images = profile_images(&profile);
        let mut drawn_tiles = 0;
        for counter in 1..=4 {
            let mut batch = full_frame_of(&images, usize::try_from(counter).unwrap());
            batch
                .mask(8, 0)
                .palette(0, 0x0F)
                .palette(1, 0x16)
                .palette(17, 0x27);
            batch.convert(&mut engine, counter);
            let frame = engine.converting_frame();

            assert_eq!(counter, frame.counter());
            assert_eq!(0, frame.script_error_count());
            assert!(frame.background_shape_count() <= frame.shapes().len());
            for shape in frame.shape_list() {
                let pattern = engine
                    .patterns()
                    .get(shape.reference_pattern().unwrap())
                    .unwrap();
                assert_ne!(shape.tiles(), []);
                assert_eq!(pattern.instances().len(), shape.instances().len());
            }

            engine.publish();
            drawn_tiles += engine
                .build_draw_list_of(FrameRole::Published)
                .tiles()
                .len();
        }

        assert!(drawn_tiles > 0, "{name}: nothing was drawn");

        // The learned state of a real profile round trips as well.
        let state = state_codec::write(&engine);
        let mut restored = ConversionEngine::new();
        restored
            .load_profile(&neutral_assets::load(name), true)
            .unwrap();
        state_codec::read(&mut restored, &state).unwrap();
        assert_eq!(state, state_codec::write(&restored));
    }
}

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn req_e3d_060_the_built_in_profiles_carry_usable_3d_parameters() {
    // A sanity check on the hand-over from SPEC-03: an alpha of 0 everywhere would make every
    // object transparent, which would point at a reader problem rather than a pipeline one.
    let profile = neutral_assets::load("Contra.bytes");
    let mut engine = ConversionEngine::new();
    engine.load_profile(&profile, true).unwrap();

    let patterns = engine.patterns();
    let instances: Vec<_> = patterns
        .permanent()
        .iter()
        .flat_map(|&id| patterns.get(id).unwrap().instances())
        .collect();
    let total = instances.len();
    let opaque = instances
        .iter()
        .filter(|instance| instance.alpha >= 1.0)
        .count();

    assert!(total > 0);
    assert!(
        opaque > total / 2,
        "only {opaque} of {total} objects are opaque"
    );
}

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn req_e3d_360_conversion_budget_benchmark_report() {
    // A report, not a gate: the budget is a project target measured on the reference machine
    // (DEC-04-Q06, DEC-R2-12-2). The test only fails if a frame busts the frame period of
    // 16.6 ms, which REQ-E3D-360 states as a MUST.
    const ITERATIONS: usize = 1000;

    let profile = neutral_assets::load("Final_Fantasy_USA.bytes");
    let mut engine = ConversionEngine::new();
    engine.load_profile(&profile, true).unwrap();

    let patterns = engine.patterns().permanent().len();
    let mut images = Vec::new();
    for &id in engine.patterns().permanent() {
        for image in engine.patterns().get(id).unwrap().images() {
            if images.len() < 64 {
                images.push(image.key());
            }
        }

        if images.len() >= 64 {
            break;
        }
    }

    let mut batch = Batch::new();
    batch.mask(8, 0).palette(0, 0x0F);
    let mut n = 0;
    for row in 0..31 {
        for column in 0..33 {
            let palette = u8::try_from((row + column) & 3).unwrap();
            batch.bg_cell_pal(images[n % images.len()], column, row, palette);
            n += 1;
        }
    }

    for i in 0..64 {
        let palette = u8::try_from(i & 3).unwrap();
        batch.sprite_full(
            images[usize::try_from(i).unwrap() % images.len()],
            (i * 3) % 240,
            8 + ((i * 5) % 220),
            palette,
            false,
        );
    }

    // Warm up: the first frames learn patterns and fill the pools.
    for i in 0..50 {
        batch.convert(&mut engine, i);
    }

    let mut samples: Vec<f64> = (0..ITERATIONS)
        .map(|i| {
            let start = Instant::now();
            batch.convert(&mut engine, 100 + i32::try_from(i).unwrap());
            start.elapsed().as_secs_f64() * 1000.0
        })
        .collect();

    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / f64::from(u32::try_from(ITERATIONS).unwrap());
    let p99 = samples[ITERATIONS * 99 / 100];
    let worst = samples[ITERATIONS - 1];

    let report = format!(
        "REQ-E3D-360 on {patterns} permanent patterns, full frame (33x31 background, 64 sprites), \
         {ITERATIONS} conversions: mean {mean:.3} ms, p99 {p99:.3} ms, max {worst:.3} ms \
         (target: mean <= 4 ms, p99 <= 8 ms; hard limit 16.6 ms)."
    );
    println!("{report}");
    fs::write(
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("REQ-E3D-360-benchmark.txt"),
        &report,
    )
    .unwrap();

    assert!(worst < 16.6, "{report}");
}
