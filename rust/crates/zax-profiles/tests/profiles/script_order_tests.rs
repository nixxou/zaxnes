use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::script_order::compare_names;
use zax_profiles::variant::PatternVariant;
use zax_profiles::wire;

use super::common::neutral_assets;

fn sort(names: &[&str]) -> Vec<String> {
    let mut sorted: Vec<&str> = names.to_vec();
    sorted.sort_by(|left, right| compare_names(Some(left), Some(right)));
    sorted.into_iter().map(str::to_owned).collect()
}

fn script_names(profile: &GameProfile) -> Vec<Option<&str>> {
    profile
        .scripts
        .iter()
        .map(|script| script.name.as_deref())
        .collect()
}

#[test]
fn req_prof_024_punctuation_sorts_before_digits_which_sort_before_letters() {
    // The Excitebike example: the underscore comes before the digit 7, which an ordinal sort
    // would get the other way round and would change that profile's hook order.
    assert_eq!(
        [
            "bike",
            "camera",
            "ground",
            "ramp_rotation",
            "ramp7",
            "scene"
        ],
        sort(&[
            "scene",
            "ramp7",
            "ramp_rotation",
            "camera",
            "bike",
            "ground"
        ])
        .as_slice()
    );
}

#[test]
fn req_prof_024_case_only_decides_at_the_last_level_lowercase_first() {
    assert_eq!(["a", "b", "B"], sort(&["B", "a", "b"]).as_slice());
}

#[test]
fn req_prof_024_a_shorter_name_comes_first_when_the_primary_keys_agree() {
    assert_eq!(["bg", "bg'", "bg1"], sort(&["bg1", "bg'", "bg"]).as_slice());
}

#[test]
fn req_prof_024_the_comparison_is_a_total_order() {
    let names = [
        "bg", "BG", "bg1", "bg'", "b g", "bg_", "Bg", "a", "A9", "a_",
    ];
    let sorted = sort(&names);

    for pair in sorted.windows(2) {
        assert!(
            compare_names(Some(&pair[0]), Some(&pair[1])).is_le(),
            "{} should not come after {}",
            pair[0],
            pair[1]
        );
    }

    // Comparing a name with itself is zero, and the comparison is antisymmetric.
    for left in names {
        assert!(compare_names(Some(left), Some(left)).is_eq());
        for right in names {
            let forward = compare_names(Some(left), Some(right));
            let backward = compare_names(Some(right), Some(left));
            assert_eq!(forward, backward.reverse());
        }
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_024_the_order_in_every_built_in_file_is_already_the_sorted_order() {
    let mut checked_profiles = 0;
    for path in neutral_assets::profile_files() {
        // Decoded without the loader, so the scripts are in file order.
        let document = wire::decode(&std::fs::read(&path).unwrap()).unwrap();
        if document.profile.scripts.is_empty() {
            continue;
        }

        checked_profiles += 1;
        let file_order = script_names(&document.profile);
        let mut sorted = file_order.clone();
        sorted.sort_by(|left, right| compare_names(*left, *right));

        assert_eq!(file_order, sorted);
    }

    assert_eq!(89, checked_profiles);
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_024_excitebike_keeps_the_documented_order_after_loading() {
    let profile = neutral_assets::load_profile("Excitebike.bytes").profile;

    assert_eq!(
        vec![
            Some("bike"),
            Some("camera"),
            Some("ground"),
            Some("ramp_rotation"),
            Some("ramp7"),
            Some("scene")
        ],
        script_names(&profile)
    );
}

#[test]
fn req_prof_024_loading_sorts_the_scripts() {
    // The loader sorts, so a file whose scripts were out of order would still expose the order
    // of REQ-LUA-004.
    let mut profile = GameProfile::default();
    for name in ["scene", "bike", "ramp7", "ramp_rotation"] {
        profile
            .scripts
            .push(ProfileScript::new(Some(name), true, None, None));
    }
    profile.settings = GameProfile::create_empty().settings;

    let result = format::load(&format::save(&profile, FieldPresence::All), None, None).unwrap();

    assert_eq!(
        vec![
            Some("bike"),
            Some("ramp_rotation"),
            Some("ramp7"),
            Some("scene")
        ],
        script_names(&result.profile)
    );
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_036_script_tags_bind_to_the_variants_that_share_one() {
    // Mega Man 2: the script "gear" carries the tags "gear,gear1".
    let profile = neutral_assets::load_profile("Mega Man 2.bytes").profile;
    let gear = profile
        .scripts
        .iter()
        .find(|script| script.name.as_deref() == Some("gear"))
        .unwrap();

    assert_eq!(vec!["gear", "gear1"], gear.tags());
    assert!(!gear.binds_to_every_variant());

    let tagged = PatternVariant {
        tag_string: Some("gear1".to_owned()),
        ..PatternVariant::default()
    };
    let other = PatternVariant {
        tag_string: Some("hud".to_owned()),
        ..PatternVariant::default()
    };
    assert!(gear.binds(&tagged));
    assert!(!gear.binds(&other));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_036_a_star_in_the_tag_string_binds_a_script_to_everything() {
    let profile = neutral_assets::load_profile("empty.bytes").profile;
    let build = &profile.scripts[0];

    assert_eq!(Some("*"), build.tag_string.as_deref());
    assert!(build.binds_to_every_variant());
    assert!(build.binds(&PatternVariant {
        tag_string: None,
        ..PatternVariant::default()
    }));
}
