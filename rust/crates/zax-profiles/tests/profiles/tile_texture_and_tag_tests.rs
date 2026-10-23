use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use zax_profiles::enums::{TextureFormat, TileFlip};
use zax_profiles::format::{self};
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tags;
use zax_profiles::tile::ProfileTile;
use zax_profiles::values::{Mat4F, Vec3F};
use zax_profiles::variant::{PatternVariant, TexturePixels, VariantTexture};

use super::common::neutral_assets;

fn hash_of(tile: &ProfileTile) -> u64 {
    let mut hasher = DefaultHasher::new();
    tile.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn req_prof_031_the_flip_bits_mirror_the_displayed_pixels() {
    // A single pixel at the top-left corner: bit y*8 + (7 - x) = bit 7 of the low plane.
    const TOP_LEFT: u64 = 1 << 7;
    let plain = ProfileTile::new(TOP_LEFT, 0, TileFlip::NONE);
    let mirrored_x = ProfileTile::new(TOP_LEFT, 0, TileFlip::HORIZONTAL);
    let mirrored_y = ProfileTile::new(TOP_LEFT, 0, TileFlip::VERTICAL);
    let mirrored_both = ProfileTile::new(TOP_LEFT, 0, TileFlip::HORIZONTAL | TileFlip::VERTICAL);

    assert_eq!(Some(1), plain.color_at(0, 0));
    assert_eq!(Some(1), mirrored_x.color_at(7, 0));
    assert_eq!(Some(0), mirrored_x.color_at(0, 0));
    assert_eq!(Some(1), mirrored_y.color_at(0, 7));
    assert_eq!(Some(1), mirrored_both.color_at(7, 7));

    assert!(mirrored_x.flip_x());
    assert!(!mirrored_x.flip_y());
    assert!(mirrored_y.flip_y());
}

#[test]
fn req_prof_031_the_colour_of_a_pixel_is_two_times_the_high_bit_plus_the_low_bit() {
    const BIT: u64 = 1 << 7;
    assert_eq!(
        Some(0),
        ProfileTile::new(0, 0, TileFlip::NONE).color_at(0, 0)
    );
    assert_eq!(
        Some(1),
        ProfileTile::new(BIT, 0, TileFlip::NONE).color_at(0, 0)
    );
    assert_eq!(
        Some(2),
        ProfileTile::new(0, BIT, TileFlip::NONE).color_at(0, 0)
    );
    assert_eq!(
        Some(3),
        ProfileTile::new(BIT, BIT, TileFlip::NONE).color_at(0, 0)
    );
}

#[test]
fn req_prof_032_two_tiles_are_the_same_tile_when_planes_and_flip_agree() {
    let left = Arc::new(ProfileTile::new(0x1234, 0x5678, TileFlip::HORIZONTAL));
    let right = Arc::new(ProfileTile::new(0x1234, 0x5678, TileFlip::HORIZONTAL));
    let other_flip = ProfileTile::new(0x1234, 0x5678, TileFlip::VERTICAL);

    assert_eq!(left, right);
    assert_eq!(hash_of(&left), hash_of(&right));
    assert_ne!(*left, other_flip);
    assert!(!Arc::ptr_eq(&left, &right));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_032_cells_carried_by_different_envelopes_may_still_be_the_same_tile() {
    // Across a whole profile the same tile value turns up under several keys: value equality
    // and object sharing are two different things.
    let document = neutral_assets::load_profile("Super Mario Bros.bytes");
    let objects: Vec<&Arc<ProfileTile>> = document.profile.distinct_tile_objects().collect();
    let values: HashSet<ProfileTile> = objects.iter().map(|tile| ***tile).collect();

    assert_eq!(783, objects.len());
    assert!(values.len() <= objects.len());
}

#[test]
fn req_prof_035_the_null_marker_and_an_explicit_texture_are_told_apart() {
    let marker = VariantTexture::null_marker();
    assert!(marker.is_null_marker());
    assert!(marker.has_consistent_length());

    let indexed = VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(
            TextureFormat::Indexed,
            vec![0; 4],
            vec![],
        )),
    );
    assert!(!indexed.is_null_marker());
    assert!(indexed.has_consistent_length());

    let too_short = VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(
            TextureFormat::Indexed,
            vec![0; 3],
            vec![],
        )),
    );
    assert!(!too_short.has_consistent_length());

    let direct = VariantTexture::new(
        2,
        3,
        Some(TexturePixels::new(
            TextureFormat::Direct,
            vec![0; 6],
            vec![],
        )),
    );
    assert!(direct.has_consistent_length());
}

#[test]
fn req_prof_035_an_indexed_payload_needs_one_word_per_sixteen_pixels() {
    let indexed = TexturePixels::new(TextureFormat::Indexed, vec![], vec![]);
    assert_eq!(1, indexed.expected_word_count(1, 1));
    assert_eq!(1, indexed.expected_word_count(4, 4));
    assert_eq!(2, indexed.expected_word_count(4, 5));
    assert_eq!(4, indexed.expected_word_count(8, 8));

    let direct = TexturePixels::new(TextureFormat::Direct, vec![], vec![]);
    assert_eq!(64, direct.expected_word_count(8, 8));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_035_every_explicit_texture_of_the_corpus_has_the_stated_length() {
    let mut explicit_textures = 0;
    for path in neutral_assets::profile_files() {
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let profile = neutral_assets::load_profile(&file_name).profile;
        for pattern in profile.all_patterns() {
            for variant in &pattern.variants {
                let Some(texture) = variant
                    .texture
                    .as_ref()
                    .filter(|texture| !texture.is_null_marker())
                else {
                    continue;
                };

                explicit_textures += 1;
                assert!(
                    texture.has_consistent_length(),
                    "{file_name}: a {}x{} texture has {:?} words",
                    texture.width,
                    texture.height,
                    texture.pixels.as_ref().map(|pixels| pixels.data.len())
                );
            }
        }
    }

    // 13 539 indexed plus 142 direct textures (§5.5).
    assert_eq!(13681, explicit_textures);
}

#[test]
fn req_prof_036_a_tag_string_is_split_trimmed_and_emptied_of_blanks() {
    let variant = PatternVariant {
        tag_string: Some(" char, ,!cast ".to_owned()),
        ..PatternVariant::default()
    };

    assert_eq!(vec!["char", "!cast"], variant.tags());
    assert!(variant.has_tag("char"));
    assert!(variant.has_tag("!cast"));
    assert!(!variant.has_tag("Char"));
}

#[test]
fn req_prof_036_the_special_variant_tags_are_the_five_of_the_spec() {
    assert_eq!(
        ["!cast", "!receiv", "!recv", "!static", "_gaxis"],
        tags::SPECIAL_VARIANT_TAGS
    );
}

#[test]
fn req_prof_036_an_absent_tag_string_gives_no_tag() {
    assert_eq!(tags::parse(None), [] as [&str; 0]);
    assert_eq!(tags::parse(Some("")), [] as [&str; 0]);
    assert_eq!(tags::parse(Some(" , , ")), [] as [&str; 0]);
}

#[rstest::rstest]
#[case(0, 0)]
#[case(4, 4)]
#[case(-1, 4)]
#[case(-5, 0)]
#[case(-2, 3)]
fn req_prof_037_a_negative_empty_colour_is_brought_back_into_0_to_4(
    #[case] stored: i32,
    #[case] expected: i32,
) {
    let variant = PatternVariant {
        stored_empty_color: stored,
        ..PatternVariant::default()
    };

    assert_eq!(expected, variant.empty_color());
    assert_eq!(stored, variant.stored_empty_color);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_037_no_built_in_profile_stores_a_negative_empty_colour() {
    for path in neutral_assets::profile_files() {
        let profile =
            neutral_assets::load_profile(&path.file_name().unwrap().to_string_lossy()).profile;
        for pattern in profile.all_patterns() {
            for variant in &pattern.variants {
                assert!((0..=4).contains(&variant.stored_empty_color));
            }
        }
    }
}

#[test]
fn req_prof_034_the_custom_data_of_a_variant_comes_from_the_last_row_of_the_two_matrices() {
    let speed: [f32; 16] = std::array::from_fn(|index| f32::from(u8::try_from(index).unwrap()));
    let offset: [f32; 16] =
        std::array::from_fn(|index| 100.0 + f32::from(u8::try_from(index).unwrap()));

    let variant = PatternVariant {
        deform_speed: Some(Mat4F::new(speed)),
        deform_offset: Some(Mat4F::new(offset)),
        ..PatternVariant::default()
    };

    // Row 3 of deltaSpeed is CData[0..3], row 3 of deltaOffset is CData[4..7].
    assert_eq!(Some(12.0), variant.custom_data(0));
    assert_eq!(Some(15.0), variant.custom_data(3));
    assert_eq!(Some(112.0), variant.custom_data(4));
    assert_eq!(Some(115.0), variant.custom_data(7));
    assert_eq!(None, variant.custom_data(8));

    let empty = PatternVariant::default();
    assert_eq!(Some(0.0), empty.custom_data(0));
}

#[test]
fn req_prof_002_float_bits_survive_a_round_trip_including_negative_zero() {
    let mut profile = GameProfile::create_empty();
    let pattern = ProfilePattern {
        variants: vec![Arc::new(PatternVariant {
            alpha: -0.0,
            scale: Some(Vec3F::new(f32::NAN, -0.0, f32::NEG_INFINITY)),
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    let result = format::load(&format::save(&profile, FieldPresence::All), None, None).unwrap();
    let restored = &result.profile.permanent_patterns[0].variants[0];
    let scale = restored.scale.unwrap();

    assert_eq!((-0.0_f32).to_bits(), restored.alpha.to_bits());
    assert_eq!(f32::NAN.to_bits(), scale.x.to_bits());
    assert_eq!((-0.0_f32).to_bits(), scale.y.to_bits());
    assert_eq!(f32::NEG_INFINITY.to_bits(), scale.z.to_bits());

    // Bitwise equality: -0 is not 0 here, which is what keeps a rewrite byte-identical.
    assert_ne!(Vec3F::new(0.0, 0.0, 0.0), Vec3F::new(-0.0, 0.0, 0.0));
}
