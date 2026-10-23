use std::sync::Arc;

use rstest::rstest;
use zax_profiles::enums::{
    GlobalMeshAlgorithm, LayerAlignment, MeshAlgorithm, PatternColorFlags, PatternDensity,
    ProfileLayer, TileFlip, TvRegionMode, VariantGeometry, VariantRenderStyle,
};
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::setting_application::{applied_keys, preserved_keys, values_to_apply};
use zax_profiles::settings::ProfileSettings;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::{ArgbColor, Mat4F, Vec2I, Vec3F, Vec4F};
use zax_profiles::variant::{PatternVariant, VariantTexture};
use zax_profiles::wire::cipher;
use zax_settings::catalogue::SettingId;

use super::common::neutral_assets;

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn save(profile: &GameProfile) -> Vec<u8> {
    format::save(profile, FieldPresence::All)
}

#[test]
fn req_prof_002_scalars_use_the_stated_encoding() {
    // -1 as an int32 is sign-extended to 64 bits, ten bytes, no zig-zag; 1.0f is four
    // little-endian bytes.
    let profile = GameProfile {
        settings: Some(ProfileSettings {
            custom_background_color: Some(ArgbColor::new(-1)),
            volume: 1.0,
            ..ProfileSettings::default()
        }),
        ..GameProfile::default()
    };

    let text = hex(&save(&profile));
    assert!(text.contains("ff ff ff ff ff ff ff ff ff 01"));
    assert!(text.contains("00 00 80 3f"));
}

#[test]
fn req_prof_020_every_profile_starts_with_the_signature_and_the_version() {
    let profile = GameProfile {
        signature: Some("3dn".to_owned()),
        version_text: Some("2".to_owned()),
        ..GameProfile::default()
    };
    assert_eq!("0a 03 33 64 6e 12 01 32", hex(&save(&profile)));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_020_the_built_in_profiles_all_start_with_the_signature() {
    for path in neutral_assets::profile_files() {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!("0a 03 33 64 6e", hex(&bytes[..5]));
    }
}

#[test]
fn req_prof_038_a_script_is_written_as_the_stated_bytes() {
    let mut profile = GameProfile::default();
    profile
        .scripts
        .push(ProfileScript::new(Some("Build"), true, None, None));

    assert!(hex(&save(&profile)).contains("0a 05 42 75 69 6c 64 10 01 18 00"));
}

#[test]
fn req_prof_002_a_vector_of_ones_in_field_8_is_written_as_the_stated_bytes() {
    let mut profile = GameProfile::default();
    let pattern = ProfilePattern {
        variants: vec![Arc::new(PatternVariant {
            scale: Some(Vec3F::new(1.0, 1.0, 1.0)),
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    assert!(hex(&save(&profile)).contains("42 0f 0d 00 00 80 3f 15 00 00 80 3f 1d 00 00 80 3f"));
}

#[test]
fn req_prof_012_an_empty_cell_is_an_empty_group() {
    let mut profile = GameProfile::default();
    let pattern = ProfilePattern {
        tile_size: Some(Vec2I::new(1, 1)),
        grid: Some(PatternGrid::new(1, 1, vec![None])),
        variants: vec![Arc::new(PatternVariant::default())],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    assert!(hex(&save(&profile)).contains("0b 0c"));
}

#[test]
fn req_prof_011_the_twelfth_key_goes_to_the_twelfth_new_object() {
    // One pattern (key 1), then eleven new tiles (keys 2 to 12), then a cell that names the
    // tile of key 12 again. That reproduces both cell vectors of §5.1 and shows the keys are
    // handed out in prefix order over the whole file.
    let shared = Arc::new(ProfileTile::new(
        0x0020_2018_0000_0000,
        0x103A_3C18_0000_0000,
        TileFlip::NONE,
    ));
    let mut cells: Vec<Option<Arc<ProfileTile>>> = (1..=10)
        .map(|plane| Some(Arc::new(ProfileTile::new(plane, 0, TileFlip::NONE))))
        .collect();
    cells.push(Some(Arc::clone(&shared)));
    cells.push(Some(shared));

    let mut profile = GameProfile::default();
    let pattern = ProfilePattern {
        tile_size: Some(Vec2I::new(12, 1)),
        grid: Some(PatternGrid::new(12, 1, cells)),
        variants: vec![Arc::new(PatternVariant::default())],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    let text = hex(&save(&profile));

    // New tile, key 12, the planes of §5.3.
    assert!(text.contains(
        "0b 0a 1c 10 0c 52 18 0a 16 08 b3 f1 a8 ce 80 d0 a5 89 62 \
         10 b3 f1 b4 fa c0 d0 a5 89 62 18 00 0c"
    ));

    // The cell that names the existing key 12.
    assert!(text.contains("0b 0a 02 08 0c 0c"));
}

#[test]
fn req_prof_013_the_null_texture_marker_is_written_payload_first() {
    let mut profile = GameProfile::default();
    let pattern = ProfilePattern {
        variants: vec![Arc::new(PatternVariant {
            texture: Some(VariantTexture::null_marker()),
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    assert!(hex(&save(&profile)).contains("9a 01 08 52 02 10 00 08 00 10 00"));
}

#[test]
fn req_prof_002_an_opaque_black_colour_is_written_as_the_stated_varint() {
    let profile = GameProfile {
        settings: Some(ProfileSettings {
            custom_background_color: Some(ArgbColor::new(-16_777_216)),
            ..ProfileSettings::default()
        }),
        ..GameProfile::default()
    };

    assert!(hex(&save(&profile)).contains("80 80 80 f8 ff ff ff ff ff 01"));
}

#[rstest]
#[case(
    0x0000_0000_0000_0000,
    0x6212_9680_01C2_3EB3,
    7_066_876_241_827_217_075
)]
#[case(0xFFFF_FFFF_FFFF_FFFF, 0x9DED_697F_FE3D_C14C, -7_066_876_241_827_217_076)]
#[case(
    0x0000_0000_0000_0001,
    0x6212_96C0_01C2_3EB3,
    7_066_876_516_705_124_019
)]
#[case(
    0x8000_0000_0000_0000,
    0x6212_96A0_01C2_3EB3,
    7_066_876_379_266_170_547
)]
#[case(
    0x0000_0000_0000_00FF,
    0x6212_A940_01C2_3EB3,
    7_066_896_857_670_237_875
)]
#[case(
    0x0102_0408_1020_4080,
    0x6A02_B680_4143_3CB7,
    7_638_868_579_940_777_143
)]
#[case(0x071F_3F7F_7FFF_FFFF, 0x9DED_6941_C60D_E16C, -7_066_876_509_057_851_028)]
#[case(
    0xC0F0_F8FC_FCFE_FEFE,
    0x5DAD_2930_3DFC_018C,
    6_750_096_703_673_336_204
)]
#[case(
    0xC381_8100_0081_81C3,
    0x4272_E670_E1A2_7EB3,
    4_788_142_726_344_507_059
)]
fn req_prof_030_the_plane_encoding_matches_the_test_vectors(
    #[case] plane: u64,
    #[case] stored_unsigned: u64,
    #[case] stored_signed: i64,
) {
    assert_eq!(0x0007_08FA_CD88_4A5A, cipher::KEY);
    assert_eq!(26, cipher::ROTATION);

    let stored = cipher::encode(plane);
    assert_eq!(stored_signed, stored);
    assert_eq!(stored_unsigned, u64::from_le_bytes(stored.to_le_bytes()));
    assert_eq!(plane, cipher::decode(stored));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_010_the_first_pattern_envelope_of_balloon_fight_is_the_stated_bytes() {
    let bytes = neutral_assets::read_profile("BalloonFight.bytes");

    // Root field 4, length 611, then: new object of key 1, then field 10 of length 606.
    assert!(hex(&bytes).contains("22 e3 04 10 01 52 de 04"));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_031_the_tile_of_key_12_of_balloon_fight_draws_the_expected_pattern() {
    let profile = neutral_assets::load_profile("BalloonFight.bytes").profile;

    assert_eq!(Some("3dn"), profile.signature.as_deref());
    assert_eq!(Some("2"), profile.version_text.as_deref());
    assert_eq!(Some("Lc89vRjlYrUL"), profile.id.as_deref());
    assert_eq!(None, profile.title);
    assert!(profile.scripts.is_empty());
    assert_eq!(237, profile.permanent_patterns.len());
    assert_eq!(2, profile.temporary_patterns.len());

    let first = &profile.permanent_patterns[0];
    assert_eq!(Some(Vec2I::new(8, 1)), first.tile_size);
    assert_eq!(8, first.grid.as_ref().unwrap().filled_cell_count());

    let second = &profile.permanent_patterns[1];
    assert_eq!(Vec2I::UNCONSTRAINED, second.constraint_size);
    assert_eq!(Some(Vec2I::new(1, 1)), second.tile_size);
    assert!(second.permanent);
    assert_eq!(0, second.pixel_count);
    assert_eq!(
        PatternColorFlags::COLOR2 | PatternColorFlags::COLOR3,
        second.color_flags
    );
    assert_eq!(PatternDensity::Solid, second.tile_density);
    assert_eq!(PatternDensity::Low, second.pixel_density);
    assert!(!second.self_connected);
    assert!(!second.skip_geometry);

    let tile = second.grid.as_ref().unwrap().cells()[0].as_ref().unwrap();
    assert_eq!(0x0020_2018_0000_0000, tile.low_plane);
    assert_eq!(0x103A_3C18_0000_0000, tile.high_plane);
    assert_eq!(TileFlip::NONE, tile.flip);
    assert_eq!(7_066_876_241_961_957_555, cipher::encode(tile.low_plane));
    assert_eq!(7_066_876_259_234_298_035, cipher::encode(tile.high_plane));

    let expected = [
        "00000000", "00000000", "00000000", "00000000", "00033000", "00322200", "00322020",
        "00020000",
    ];
    for (y, expected_row) in (0..8).zip(expected) {
        let row: String = (0..8)
            .map(|x| char::from_digit(tile.color_at(x, y).unwrap(), 10).unwrap())
            .collect();
        assert_eq!(expected_row, row);
    }

    assert_eq!(1, second.variants.len());
    let variant = &second.variants[0];
    assert_eq!(3, variant.depth);
    assert_eq!(VariantGeometry::Default, variant.geometry);
    assert_eq!(ProfileLayer::Unidentified, variant.layer);
    assert_eq!(Some(Vec3F::new(1.0, 1.0, 1.0)), variant.scale);
    assert_eq!(Some(Vec3F::ZERO), variant.rotation);
    assert_eq!(Some(Vec3F::ZERO), variant.offset);
    assert_eq!(Some(Vec3F::new(0.5, 0.5, 0.5)), variant.pivot);
    assert!((variant.alpha - 1.0).abs() < f32::EPSILON);
    assert_eq!(MeshAlgorithm::FromSettings, variant.mesh_algorithm);
    assert_eq!(Some(Vec3F::ZERO), variant.deform_amplitude);
    assert_eq!(Some(Vec3F::new(0.5, 0.5, 0.5)), variant.deform_pivot);
    assert_eq!(VariantRenderStyle::Normal, variant.render_style);
    assert!(variant.texture.as_ref().unwrap().is_null_marker());
    assert_eq!(Some(""), variant.tag_string.as_deref());
    assert_eq!(0, variant.stored_empty_color);

    let quarter_turn = f32::from_bits(0x3FC9_0FDB);
    let speed = variant.deform_speed.unwrap();
    let offset = variant.deform_offset.unwrap();
    for cell in 0..Mat4F::CELL_COUNT {
        assert!(speed.cells()[cell].abs() < f32::EPSILON);
        assert_eq!(quarter_turn.to_bits(), offset.cells()[cell].to_bits());
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_003_empty_bytes_decodes_to_the_values_of_section_5_4() {
    let document = neutral_assets::load_profile("empty.bytes");
    let profile = &document.profile;

    assert_eq!(Some("2"), profile.version_text.as_deref());
    assert_eq!(None, profile.id);
    assert!(profile.permanent_patterns.is_empty());
    assert!(profile.temporary_patterns.is_empty());

    assert_eq!(1, profile.scripts.len());
    let script = &profile.scripts[0];
    assert_eq!(Some("Build"), script.name.as_deref());
    assert_eq!(Some("*"), script.tag_string.as_deref());
    assert!(script.enabled);
    assert!(script
        .source
        .as_deref()
        .unwrap()
        .starts_with("function BuildPattern()"));

    let settings = profile.settings.as_ref().unwrap();
    let light = settings.light.as_ref().unwrap();
    assert_eq!(Some(Vec3F::new(1.0, 0.2, 1.0)), light.direction);
    assert_eq!(Some(ArgbColor::new(-1)), light.color);
    assert!((light.intensity - 0.85911).abs() < 0.00001);
    assert!(settings.use_nes_background_color);
    assert_eq!(
        Some(ArgbColor::new(-16_777_216)),
        settings.custom_background_color
    );
    assert_eq!(Some(Vec4F::new(4.1, 16.1, 4.1, 0.1)), settings.clipping);
    let layers = settings.layers.as_ref().unwrap();
    assert_eq!(ProfileLayer::L1, layers.default_layer);
    assert!(!layers.auto);
    assert_eq!(2, settings.graphic_quality);
    assert_eq!(120, settings.frame_rate);
    assert!((settings.volume - 1.0).abs() < f32::EPSILON);
    assert_eq!(GlobalMeshAlgorithm::Greedy, settings.mesh_algorithm);
    assert_eq!(0, settings.skybox);
    assert_eq!(2, settings.core);
    assert!(settings.mirroring);

    let expected_layers = [
        (0.0, LayerAlignment::Front, false, false),
        (0.0, LayerAlignment::Front, false, false),
        (0.0, LayerAlignment::Back, true, true),
        (0.0, LayerAlignment::Front, false, true),
        (-4.0, LayerAlignment::Front, false, true),
        (-4.0, LayerAlignment::Front, false, true),
    ];
    for (layer, (offset, aligned, cast, recv)) in layers.layers.iter().zip(expected_layers) {
        assert!((layer.offset - offset).abs() < f32::EPSILON);
        assert_eq!(aligned, layer.aligned);
        assert_eq!(cast, layer.stored_cast_shadow);
        assert_eq!(recv, layer.stored_recv_shadow);
    }

    let names = |slots: &[zax_profiles::pattern::RecordedPatternSlot]| -> Vec<String> {
        slots
            .iter()
            .map(|slot| slot.name.clone().unwrap())
            .collect()
    };
    assert_eq!(
        ["Road", "BG", "Top", "Loop", "Body"],
        names(&settings.saved_patterns).as_slice()
    );
    assert_eq!(
        ["Deform1", "Deform2", "Deform3", "Deform4", "Deform5"],
        names(&settings.saved_anim_patterns).as_slice()
    );

    let slot_pattern = settings.saved_patterns[0].pattern.as_ref().unwrap();
    assert_eq!(Vec2I::UNCONSTRAINED, slot_pattern.constraint_size);
    assert_eq!(Some(Vec2I::new(0, 0)), slot_pattern.tile_size);
    assert!(!slot_pattern.permanent);
    assert!(slot_pattern.grid.is_none());
    assert_eq!(1, slot_pattern.variants.len());
    assert!(slot_pattern.self_connected);
    assert!(slot_pattern.variants[0].texture.is_none());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_040_the_settings_absent_from_empty_bytes_take_their_defaults() {
    let document = neutral_assets::load_profile("empty.bytes");
    let settings = document.profile.settings.as_ref().unwrap();

    // empty.bytes stores fields 1 to 23 only, so fields 24 to 38 take the defaults of
    // REQ-PROF-040.
    assert!(!settings.empty_tile);
    assert!(!settings.local_camera);
    assert!((settings.scene_ratio - 1.0).abs() < f32::EPSILON);
    assert!(!settings.emulated_menu_button);
    assert_eq!(0, settings.render_method);
    assert_eq!(0, settings.resolution_scale_index);
    assert!(settings.skip_frame);
    assert!(settings.background_music);
    assert_eq!(0, settings.anti_alias);
    assert!(!settings.auto_tv_mode);
    assert_eq!(TvRegionMode::Ntsc, settings.tv_mode);
    assert_eq!(0, settings.tv_box);
    assert_eq!(0, settings.sort_option);
    assert!(!settings.same_sprite_palette);
    assert!((settings.fov - 70.0).abs() < f32::EPSILON);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_040_the_field_of_view_is_absent_from_every_built_in_profile() {
    for path in neutral_assets::profile_files() {
        let document = neutral_assets::load_profile(&path.file_name().unwrap().to_string_lossy());
        assert!((document.profile.settings.as_ref().unwrap().fov - 70.0).abs() < f32::EPSILON);
    }
}

/// AMD-031 §3, REQ-PROF-051
#[rstest]
#[case(0)]
#[case(1)]
#[case(3)]
fn amd_031_the_tv_box_field_survives_a_round_trip_and_applies_nothing(#[case] stored: i32) {
    let mut written = GameProfile::create_empty();
    written.settings.as_mut().unwrap().tv_box = stored;
    let original = save(&written);

    // Field 35 is really on the wire: its tag, 35 << 3, is the two bytes 98 02.
    assert!(hex(&original).contains(&format!("98 02 {stored:02x}")));

    let document = format::load(&original, None, Some("tv-box")).unwrap();

    assert_eq!(0, document.unknown_field_count);
    assert_eq!(stored, document.profile.settings.as_ref().unwrap().tv_box);
    assert_eq!(original, document.save());

    // No effect: the field is in neither list of REQ-PROF-041 and reaches no setting key.
    let key_is_listed = |ids: Vec<SettingId>| ids.iter().any(|id| id.key() == "scene.tv_box");
    assert!(!key_is_listed(applied_keys()));
    assert!(!key_is_listed(preserved_keys()));
    assert!(SettingId::from_key("scene.tv_box").is_none());
    assert!(
        !values_to_apply(document.profile.settings.as_ref().unwrap())
            .keys()
            .any(|id| id.key() == "scene.tv_box")
    );
}
