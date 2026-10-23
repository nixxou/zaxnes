use std::sync::Arc;

use zax_profiles::enums::TileFlip;
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;

use super::common::model_comparer;
use super::common::neutral_assets;

fn describe_first_difference(expected: &[u8], actual: &[u8]) -> String {
    for (index, (expected_byte, actual_byte)) in expected.iter().zip(actual).enumerate() {
        if expected_byte != actual_byte {
            return format!(
                "byte {index} is 0x{actual_byte:02X}, expected 0x{expected_byte:02X} (lengths {} and {})",
                actual.len(),
                expected.len()
            );
        }
    }
    format!("length {}, expected {}", actual.len(), expected.len())
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_001_every_built_in_profile_decodes_with_no_unknown_field() {
    let mut failures = Vec::new();
    let mut unknown_fields = 0;
    let mut count = 0;
    for path in neutral_assets::profile_files() {
        count += 1;
        match format::load(
            &std::fs::read(&path).unwrap(),
            None,
            Some(&path.to_string_lossy()),
        ) {
            Ok(document) => unknown_fields += document.unknown_field_count,
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(101, count);
    assert_eq!(0, unknown_fields);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_051_the_canonical_writer_reproduces_every_built_in_profile_byte_for_byte() {
    let mut failures = Vec::new();
    let mut identical = 0;
    for path in neutral_assets::profile_files() {
        let original = std::fs::read(&path).unwrap();
        let document = match format::load(&original, None, Some(&path.to_string_lossy())) {
            Ok(document) => document,
            Err(error) => {
                failures.push(format!("{}: {error}", path.display()));
                continue;
            }
        };

        let rewritten = document.save();
        if rewritten == original {
            identical += 1;
        } else {
            failures.push(format!(
                "{}: {}",
                path.display(),
                describe_first_difference(&original, &rewritten)
            ));
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(101, identical);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_050_decoding_a_rewritten_profile_gives_the_same_model() {
    for path in neutral_assets::profile_files() {
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let first = neutral_assets::load_profile(&file_name);
        let rewritten = first.save();
        let second = format::load(&rewritten, None, Some(&file_name))
            .unwrap_or_else(|error| panic!("{file_name}: {error}"));
        model_comparer::assert_equal(&first.profile, &second.profile, &file_name);
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_011_shared_tiles_are_resolved_to_the_same_object() {
    let document = neutral_assets::load_profile("BalloonFight.bytes");
    let filled_cells: usize = document
        .profile
        .all_patterns()
        .map(|pattern| {
            pattern
                .grid
                .as_ref()
                .map_or(0, PatternGrid::filled_cell_count)
        })
        .sum();

    let distinct_objects = document.profile.distinct_tile_objects().count();

    // 516 tile objects carry 1 085 filled cells: the rest are shared references
    // (REQ-PROF-011, §5.3).
    assert_eq!(516, distinct_objects);
    assert!(
        filled_cells > distinct_objects,
        "tiles are expected to be shared"
    );
}

#[test]
fn req_prof_004_a_profile_built_in_memory_writes_every_scalar() {
    // Without recorded fields the writer follows REQ-PROF-051 literally: every scalar written,
    // nulls included. Decoding it back gives a model with the same values.
    let mut profile = GameProfile::create_empty();
    let pattern = ProfilePattern {
        tile_size: Some(Vec2I::new(1, 1)),
        permanent: true,
        skip_geometry: false,
        variants: vec![Arc::new(PatternVariant {
            depth: 3,
            alpha: 1.0,
            ..PatternVariant::default()
        })],
        grid: Some(PatternGrid::new(
            1,
            1,
            vec![Some(Arc::new(ProfileTile::new(
                0x0020_2018_0000_0000,
                0x103A_3C18_0000_0000,
                TileFlip::NONE,
            )))],
        )),
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    let bytes = format::save(&profile, FieldPresence::All);
    let reloaded = format::load(&bytes, None, None).unwrap();

    assert_eq!(1, reloaded.profile.permanent_patterns.len());
    let restored = &reloaded.profile.permanent_patterns[0];
    assert_eq!(Some(Vec2I::new(1, 1)), restored.tile_size);
    assert!(restored.permanent);
    assert_eq!(Vec2I::UNCONSTRAINED, restored.constraint_size);
    assert_eq!(1, restored.variants.len());
    assert_eq!(3, restored.variants[0].depth);
    assert_eq!(
        0x103A_3C18_0000_0000,
        restored.grid.as_ref().unwrap().cells()[0]
            .as_ref()
            .unwrap()
            .high_plane
    );

    // All twelve pattern fields were written, so the layout records no deviation.
    assert_eq!(None, restored.recorded_fields);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_051_the_recorded_field_sets_are_only_the_ones_the_files_leave_out() {
    let empty = neutral_assets::load_profile("empty.bytes");
    let settings_fields = empty.profile.settings.as_ref().unwrap().recorded_fields;

    // empty.bytes stores settings fields 1 to 23 only (§5.4).
    assert_eq!(
        (1..=23).collect::<Vec<u32>>(),
        settings_fields.unwrap().fields()
    );
}
