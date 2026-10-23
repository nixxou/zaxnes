use rstest::rstest;
use zax_profiles::enums::ProfileFormatVersion;
use zax_profiles::format::{self, ProfileDocument, ProfileLoadError, ProfileLoadErrorKind};
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::RecordedPatternSlot;
use zax_profiles::profile::{version_rules, GameProfile};

use super::common::model_comparer;
use super::common::neutral_assets;
use super::common::recording_log::RecordingProfileLog;

fn valid_profile() -> GameProfile {
    let mut profile = GameProfile::create_empty();
    let settings = profile.settings.as_mut().unwrap();
    settings
        .saved_patterns
        .extend(RecordedPatternSlot::default_shape_slots());
    settings
        .saved_anim_patterns
        .extend(RecordedPatternSlot::default_deform_slots());
    profile
}

fn save(profile: &GameProfile) -> Vec<u8> {
    format::save(profile, FieldPresence::All)
}

fn load(profile: &GameProfile) -> Result<ProfileDocument, ProfileLoadError> {
    format::load(&save(profile), None, None)
}

fn kind_of(result: Result<ProfileDocument, ProfileLoadError>) -> ProfileLoadErrorKind {
    result.unwrap_err().kind
}

#[test]
fn req_prof_020_a_profile_whose_signature_is_not_3dn_is_refused() {
    let mut profile = valid_profile();
    profile.signature = Some("3dx".to_owned());

    assert_eq!(
        ProfileLoadErrorKind::WrongSignature,
        kind_of(load(&profile))
    );
}

#[test]
fn req_prof_020_an_absent_signature_is_not_an_error() {
    let mut profile = valid_profile();
    profile.signature = None;

    assert!(load(&profile).is_ok());
}

#[rstest]
#[case(Some("1"), ProfileFormatVersion::Unsupported)]
#[case(Some("v1"), ProfileFormatVersion::Unsupported)]
#[case(Some("2"), ProfileFormatVersion::V2)]
#[case(Some("3"), ProfileFormatVersion::Current)]
#[case(Some("2.0"), ProfileFormatVersion::Current)]
#[case(Some("$3"), ProfileFormatVersion::Current)]
#[case(Some(""), ProfileFormatVersion::Current)]
#[case(None, ProfileFormatVersion::Current)]
fn req_prof_021_the_version_string_is_filtered_then_classified(
    #[case] version_text: Option<&str>,
    #[case] expected: ProfileFormatVersion,
) {
    assert_eq!(expected, version_rules::classify(version_text));
}

#[rstest]
#[case("v1", "1")]
#[case("ver 2.0", "2.0")]
#[case("3$", "3$")]
#[case("a-b", "")]
fn req_prof_021_filtering_keeps_only_digits_a_dot_and_a_dollar(
    #[case] input: &str,
    #[case] expected: &str,
) {
    assert_eq!(expected, version_rules::sanitize(Some(input)));
}

#[test]
fn req_prof_021_a_version_1_profile_is_refused_and_a_version_2_profile_is_flagged() {
    let mut refused = valid_profile();
    refused.version_text = Some("1".to_owned());
    assert_eq!(
        ProfileLoadErrorKind::UnsupportedVersion,
        kind_of(load(&refused))
    );

    let mut v2 = valid_profile();
    v2.version_text = Some("2".to_owned());
    assert!(load(&v2).unwrap().profile.is_v2_format());

    let mut v3 = valid_profile();
    v3.version_text = Some("2.0".to_owned());
    assert!(!load(&v3).unwrap().profile.is_v2_format());
}

#[test]
fn req_prof_025_a_profile_without_settings_is_refused() {
    let mut profile = valid_profile();
    profile.settings = None;

    assert_eq!(
        ProfileLoadErrorKind::MissingSettings,
        kind_of(load(&profile))
    );
}

#[test]
fn req_prof_025_a_profile_whose_settings_lack_the_light_the_clipping_or_the_layers_is_refused() {
    let mut without_light = valid_profile();
    without_light.settings.as_mut().unwrap().light = None;
    assert_eq!(
        ProfileLoadErrorKind::IncompleteSettings,
        kind_of(load(&without_light))
    );

    let mut without_clipping = valid_profile();
    without_clipping.settings.as_mut().unwrap().clipping = None;
    assert_eq!(
        ProfileLoadErrorKind::IncompleteSettings,
        kind_of(load(&without_clipping))
    );

    let mut without_layers = valid_profile();
    without_layers.settings.as_mut().unwrap().layers = None;
    assert_eq!(
        ProfileLoadErrorKind::IncompleteSettings,
        kind_of(load(&without_layers))
    );
}

#[test]
fn req_prof_043_a_profile_that_does_not_hold_six_layers_is_refused() {
    let mut profile = valid_profile();
    profile
        .settings
        .as_mut()
        .unwrap()
        .layers
        .as_mut()
        .unwrap()
        .layers
        .remove(5);

    assert_eq!(
        ProfileLoadErrorKind::WrongLayerCount,
        kind_of(load(&profile))
    );
}

#[test]
fn req_prof_025_a_profile_without_the_memorised_slots_is_accepted_and_the_defaults_rebuilt() {
    // DEC-03-Q03, an accepted divergence: the original refuses such a profile.
    let profile = GameProfile::create_empty();
    assert!(profile.settings.as_ref().unwrap().saved_patterns.is_empty());

    let result = load(&profile).unwrap();

    let settings = result.profile.settings.as_ref().unwrap();
    let names = |slots: &[RecordedPatternSlot]| -> Vec<String> {
        slots
            .iter()
            .map(|slot| slot.name.clone().unwrap())
            .collect()
    };
    assert_eq!(
        ["Config1", "Config2", "Config3", "Config4", "Config5"],
        names(&settings.saved_patterns).as_slice()
    );
    assert_eq!(
        ["Deform1", "Deform2", "Deform3", "Deform4", "Deform5"],
        names(&settings.saved_anim_patterns).as_slice()
    );

    // The rebuilt slots are not written back: the file said nothing about them.
    assert_eq!(save(&profile), result.save());
}

/// REQ-PROF-001: a file with field 99 of any wire type decodes to the same model.
#[rstest]
#[case(0)]
#[case(2)]
#[case(5)]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_001_an_unknown_field_is_skipped_and_the_model_is_unchanged(#[case] wire_type: u8) {
    let original = neutral_assets::read_profile("empty.bytes");
    let reference = neutral_assets::load_profile("empty.bytes");

    let tag = [0x98 | wire_type, 0x06];
    let payload: &[u8] = match wire_type {
        0 => &[0x2a],
        2 => &[0x03, 0x61, 0x62, 0x63],
        _ => &[0x00, 0x00, 0x00, 0x00],
    };

    let patched = [original.as_slice(), &tag, payload].concat();
    let result = format::load(&patched, None, None).unwrap();

    assert_eq!(1, result.unknown_field_count);
    model_comparer::assert_equal(
        &reference.profile,
        &result.profile,
        "empty.bytes + field 99",
    );
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_033_a_profile_truncated_in_half_loads_as_an_empty_profile() {
    let original = neutral_assets::read_profile("BalloonFight.bytes");
    let half = &original[..original.len() / 2];

    let error = format::load(half, None, None).unwrap_err();

    assert_eq!(ProfileLoadErrorKind::Malformed, error.kind);

    let fallback = GameProfile::create_empty();
    assert!(fallback.permanent_patterns.is_empty());
    assert!(fallback.temporary_patterns.is_empty());
    assert!(fallback.scripts.is_empty());
    assert!(fallback.settings.is_some());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_acc_041_no_truncation_of_a_profile_ever_throws() {
    let original = neutral_assets::read_profile("Puzznic (U).bytes");
    for length in (0..=original.len()).step_by(97) {
        // Either it is refused or it decodes; what matters is that it never panics and never
        // leaves a half-built profile behind.
        if let Ok(document) = format::load(&original[..length], None, None) {
            assert!(document.profile.settings.is_some());
        }
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_acc_041_no_single_byte_corruption_of_a_profile_ever_throws() {
    let original = neutral_assets::read_profile("empty.bytes");
    let mut accepted = 0;
    let mut refused = 0;
    for index in (0..original.len()).step_by(7) {
        let mut corrupted = original.clone();
        corrupted[index] ^= 0xFF;

        if format::load(&corrupted, None, None).is_ok() {
            accepted += 1;
        } else {
            refused += 1;
        }
    }

    assert!(accepted + refused > 0);
}

#[test]
fn req_acc_041_random_bytes_are_refused_without_throwing() {
    // A fixed xorshift sequence, so the noise is the same on every run.
    let mut state: u32 = 0x3D5E;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };

    for _ in 0..200 {
        let length = usize::try_from(next() % 512).unwrap();
        let noise: Vec<u8> = (0..length).map(|_| next().to_le_bytes()[0]).collect();

        if let Ok(document) = format::load(&noise, None, None) {
            // Random bytes that happen to parse must still have passed every rule.
            assert!(document.profile.settings.is_some());
        }
    }
}

#[test]
fn req_prof_011_a_reference_to_an_unknown_key_is_refused() {
    // Root field 4 holding an envelope whose field 1 names key 7, which no object has.
    let bytes = [0x0a, 0x03, 0x33, 0x64, 0x6e, 0x22, 0x02, 0x08, 0x07];

    let error = format::load(&bytes, None, None).unwrap_err();

    assert_eq!(ProfileLoadErrorKind::Malformed, error.kind);
    assert!(error.message.contains("unknown key"), "{}", error.message);
}

#[test]
fn req_prof_011_a_key_out_of_sequence_is_refused() {
    // Root field 4 holding an envelope with the new key 4 instead of 1.
    let bytes = [
        0x0a, 0x03, 0x33, 0x64, 0x6e, 0x22, 0x04, 0x10, 0x04, 0x52, 0x00,
    ];

    let error = format::load(&bytes, None, None).unwrap_err();

    assert_eq!(ProfileLoadErrorKind::Malformed, error.kind);
    assert!(
        error.message.contains("out of sequence"),
        "{}",
        error.message
    );
}

#[test]
fn req_prof_033_a_refused_profile_is_logged() {
    let log = RecordingProfileLog::new();
    let _ = format::load(
        &[0x0a, 0x03, 0x33, 0x64, 0x78],
        Some(&log),
        Some("Broken.3dn"),
    );

    assert!(log
        .errors
        .borrow()
        .iter()
        .any(|line| line.starts_with("Broken.3dn:")));
}
