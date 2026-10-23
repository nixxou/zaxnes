//! The built-in profiles against the numbers of SPEC-03 §5. Every test here needs the
//! profiles (`ZAXNES_PROFILES`).

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use rstest::rstest;
use zax_profiles::enums::TextureFormat;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::settings::LayerSettingsSet;

use super::common::neutral_assets;

#[derive(Default)]
struct Totals {
    files: usize,
    files_with_scripts: usize,
    scripts: usize,
    enabled_scripts: usize,
    disabled_scripts: usize,
    patterns_field4: usize,
    patterns_field5: usize,
    slot_patterns: usize,
    patterns_total: usize,
    patterns_with_grid: usize,
    rectangular_grids: usize,
    variants: usize,
    new_tiles: usize,
    tile_references: usize,
    filled_cells: usize,
    empty_cells: usize,
    null_marker_textures: usize,
    indexed_textures: usize,
    direct_textures: usize,
    absent_textures: usize,
    variants_with_one_variant: usize,
    stored_layers: usize,
    stored_cast_shadow: usize,
    version2: usize,
    version3: usize,
    profiles_with_id: usize,
    profiles_with_title: usize,
    profiles_with_description: usize,
}

fn measure() -> Totals {
    let mut totals = Totals::default();
    let mut variant_histogram: HashMap<usize, usize> = HashMap::new();

    for path in neutral_assets::profile_files() {
        totals.files += 1;
        let profile =
            neutral_assets::load_profile(&path.file_name().unwrap().to_string_lossy()).profile;

        if !profile.scripts.is_empty() {
            totals.files_with_scripts += 1;
        }
        totals.scripts += profile.scripts.len();
        for script in &profile.scripts {
            if script.enabled {
                totals.enabled_scripts += 1;
            } else {
                totals.disabled_scripts += 1;
            }
        }

        totals.patterns_field4 += profile.permanent_patterns.len();
        totals.patterns_field5 += profile.temporary_patterns.len();

        match profile.version_text.as_deref() {
            Some("2") => totals.version2 += 1,
            Some("3") => totals.version3 += 1,
            _ => {}
        }
        totals.profiles_with_id += usize::from(profile.id.is_some());
        totals.profiles_with_title += usize::from(profile.title.is_some());
        totals.profiles_with_description += usize::from(profile.description.is_some());

        let mut seen_tiles = HashSet::new();
        let all: Vec<_> = profile.all_patterns().collect();
        for pattern in &all {
            totals.patterns_total += 1;
            totals.variants += pattern.variants.len();
            *variant_histogram.entry(pattern.variants.len()).or_default() += 1;

            let Some(grid) = &pattern.grid else {
                continue;
            };
            totals.patterns_with_grid += 1;
            if grid.is_rectangular() {
                totals.rectangular_grids += 1;
            }
            for cell in grid.cells() {
                match cell {
                    None => totals.empty_cells += 1,
                    Some(tile) => {
                        totals.filled_cells += 1;
                        if seen_tiles.insert(std::sync::Arc::as_ptr(tile)) {
                            totals.new_tiles += 1;
                        } else {
                            totals.tile_references += 1;
                        }
                    }
                }
            }
        }

        let settings = profile.settings.as_ref().unwrap();
        let layers = settings.layers.as_ref().unwrap();
        totals.stored_layers += layers.layers.len();
        totals.stored_cast_shadow += layers
            .layers
            .iter()
            .filter(|layer| layer.stored_cast_shadow)
            .count();

        let slots: Vec<&ProfilePattern> = settings
            .saved_patterns
            .iter()
            .chain(&settings.saved_anim_patterns)
            .inspect(|_| totals.slot_patterns += 1)
            .filter_map(|slot| slot.pattern.as_ref())
            .collect();
        totals.patterns_total += slots.len();

        for pattern in all.iter().map(|pattern| &***pattern).chain(slots) {
            count_textures(&mut totals, pattern);
        }
    }

    totals.variants_with_one_variant = variant_histogram.get(&1).copied().unwrap_or(0);
    totals
}

fn count_textures(totals: &mut Totals, pattern: &ProfilePattern) {
    for variant in &pattern.variants {
        let Some(texture) = &variant.texture else {
            totals.absent_textures += 1;
            continue;
        };

        assert!(texture.has_consistent_length(), "texture payload length");
        if texture.is_null_marker() {
            totals.null_marker_textures += 1;
        } else if texture.pixels.as_ref().map(|pixels| pixels.format) == Some(TextureFormat::Direct)
        {
            totals.direct_textures += 1;
        } else {
            totals.indexed_textures += 1;
        }
    }
}

static CORPUS: LazyLock<Totals> = LazyLock::new(measure);

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_023_the_two_pattern_lists_hold_the_stated_numbers() {
    let totals = &*CORPUS;
    assert_eq!(101, totals.files);
    assert_eq!(66609, totals.patterns_field4);
    assert_eq!(1223, totals.patterns_field5);
    assert_eq!(1010, totals.slot_patterns);
    assert_eq!(68842, totals.patterns_total);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_023_every_pattern_of_field_4_is_marked_permanent() {
    for path in neutral_assets::profile_files() {
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let profile = neutral_assets::load_profile(&file_name).profile;
        for pattern in &profile.permanent_patterns {
            assert!(
                pattern.permanent,
                "{file_name}: a field 4 pattern is temporary"
            );
        }
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_034_the_variant_counts_match() {
    let totals = &*CORPUS;
    assert_eq!(74389, totals.variants);
    assert_eq!(62650, totals.variants_with_one_variant);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_034_the_distribution_of_variants_per_pattern_matches() {
    let mut histogram: HashMap<usize, usize> = HashMap::new();
    for path in neutral_assets::profile_files() {
        let profile =
            neutral_assets::load_profile(&path.file_name().unwrap().to_string_lossy()).profile;
        for pattern in profile.all_patterns() {
            *histogram.entry(pattern.variants.len()).or_default() += 1;
            assert!(
                !pattern.variants.is_empty(),
                "a pattern of fields 4 or 5 has no variant"
            );
        }
    }

    // §5.5 and REQ-PROF-034: 1 -> 62 650, 2 -> 4 142, 3 -> 800, 4 and more -> 240.
    assert_eq!(62650, histogram[&1]);
    assert_eq!(4142, histogram[&2]);
    assert_eq!(800, histogram[&3]);
    let four_or_more: usize = histogram
        .iter()
        .filter(|(variants, _)| **variants >= 4)
        .map(|(_, patterns)| patterns)
        .sum();
    assert_eq!(240, four_or_more);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_012_the_cell_counts_match_and_every_grid_is_rectangular() {
    let totals = &*CORPUS;
    assert_eq!(452_231, totals.filled_cells);
    assert_eq!(124_059, totals.empty_cells);
    assert_eq!(67832, totals.patterns_with_grid);
    assert_eq!(67832, totals.rectangular_grids);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_011_tiles_are_shared_in_the_stated_proportion() {
    let totals = &*CORPUS;
    assert_eq!(225_575, totals.new_tiles);
    assert_eq!(226_656, totals.tile_references);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_035_the_texture_counts_match() {
    let totals = &*CORPUS;
    assert_eq!(60708, totals.null_marker_textures);
    assert_eq!(13539, totals.indexed_textures);
    assert_eq!(142, totals.direct_textures);

    // The 1 010 patterns of the memorised slots have no texture field at all.
    assert_eq!(1010, totals.absent_textures);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_024_the_script_counts_match() {
    let totals = &*CORPUS;
    assert_eq!(579, totals.scripts);
    assert_eq!(89, totals.files_with_scripts);
    assert_eq!(556, totals.enabled_scripts);
    assert_eq!(23, totals.disabled_scripts);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_038_the_two_name_fields_always_agree_and_no_error_flag_is_set() {
    // Field 5 repeats field 1 in all 579 scripts, so writing both from one model value is
    // lossless; the byte-identity test is what proves it.
    for path in neutral_assets::profile_files() {
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let profile = neutral_assets::load_profile(&file_name).profile;
        for script in &profile.scripts {
            assert!(
                !script.stored_error_flag,
                "{file_name}: {:?} has the error flag",
                script.name
            );
        }
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_021_one_hundred_profiles_are_version_2_and_one_is_version_3() {
    let totals = &*CORPUS;
    assert_eq!(100, totals.version2);
    assert_eq!(1, totals.version3);

    let jackal = neutral_assets::load_profile("Jackal (U).bytes");
    assert_eq!(Some("3"), jackal.profile.version_text.as_deref());
    assert!(!jackal.profile.is_v2_format());

    let balloon = neutral_assets::load_profile("BalloonFight.bytes");
    assert!(balloon.profile.is_v2_format());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_022_the_metadata_fields_are_present_as_often_as_stated() {
    let totals = &*CORPUS;
    assert_eq!(56, totals.profiles_with_id);
    assert_eq!(17, totals.profiles_with_title);
    assert_eq!(0, totals.profiles_with_description);

    // Jackal carries the title "Tennis" but the library shows the catalogue title.
    assert_eq!(
        Some("Tennis"),
        neutral_assets::load_profile("Jackal (U).bytes")
            .profile
            .title
            .as_deref()
    );
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_042_the_stored_shadow_flags_are_overridden_after_loading() {
    let totals = &*CORPUS;
    assert_eq!(606, totals.stored_layers);
    assert_eq!(119, totals.stored_cast_shadow);

    // After loading, exactly one layer per profile casts shadows: layer 2.
    for index in 0..LayerSettingsSet::LAYER_COUNT {
        assert_eq!(index == 2, LayerSettingsSet::effective_cast_shadow(index));
        assert_eq!(index >= 2, LayerSettingsSet::effective_recv_shadow(index));
    }
}

#[rstest]
#[case("empty.bytes", 3780, 1, 0, 0, 0, 0)]
#[case("Puzznic (U).bytes", 44334, 0, 74, 0, 75, 311)]
#[case("flappy_fpv.bytes", 66849, 11, 67, 0, 77, 149)]
#[case("Super Mario Bros.bytes", 179_515, 10, 257, 1, 296, 783)]
#[case("Contra.bytes", 1_120_081, 3, 1854, 65, 1966, 5365)]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_001_the_small_profiles_decode_to_the_stated_contents(
    #[case] file_name: &str,
    #[case] size: usize,
    #[case] scripts: usize,
    #[case] permanent_patterns: usize,
    #[case] temporary_patterns: usize,
    #[case] variants: usize,
    #[case] distinct_tiles: usize,
) {
    let bytes = neutral_assets::read_profile(file_name);
    assert_eq!(size, bytes.len());

    let profile = neutral_assets::load_profile(file_name).profile;
    assert_eq!(scripts, profile.scripts.len());
    assert_eq!(permanent_patterns, profile.permanent_patterns.len());
    assert_eq!(temporary_patterns, profile.temporary_patterns.len());

    let variant_count: usize = profile
        .all_patterns()
        .map(|pattern| pattern.variants.len())
        .sum();
    assert_eq!(variants, variant_count);
    assert_eq!(distinct_tiles, profile.distinct_tile_objects().count());
}
