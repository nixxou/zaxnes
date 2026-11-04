//! AMD-060 §2 — the domain of `video.anti_alias` goes from 0..2 to **0..3**, and nothing else
//! about the line moves.
//!
//! The amendment is precise about what changes and what does not, and each clause is one test
//! here: the fourth value exists and has a label of its own; the three existing labels, the
//! default of AMD-052 REQ-PERS-089, the profile field 32 and the Lua path are untouched; a
//! settings file carrying the fourth value is read back as it stands ("relu tel quel"); and a
//! value outside 0..3 is brought back the way one outside 0..2 was.
//!
//! The libretro value list is **not** extended: a core option's value strings are persisted by the
//! host, AMD-032 §7 lists the libretro table among what does not change, and AMD-060 §2 names only
//! the options window. That is asserted here too, so the choice is a checked one rather than an
//! omission.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use zax_settings::catalogue::{self, SettingId};
use zax_settings::core_options;
use zax_settings::model::SettingsModel;
use zax_settings::persistence::SettingsFile;
use zax_settings::store::MemoryStorage;
use zax_settings::values::SettingValue;

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::UNIX_EPOCH
}

/// REQ-SCN-085 as the setting's domain: four rungs, the fourth being 3 with a label of its own.
#[test]
fn req_scn_085_the_domain_carries_a_fourth_value() {
    let descriptor = catalogue::descriptor(SettingId::VideoAntiAlias);
    let options = descriptor.domain().options().expect("a choice domain");

    assert_eq!(4, options.len());
    assert_eq!(3.0, options[3].value);
    assert_eq!("SMAA_DEPTH", options[3].label);

    // And the three of AMD-032 §2 are where they were, in the order they were in.
    for (index, (value, label)) in [(0, "None"), (1, "FFAA"), (2, "SMAA")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(f64::from(value), options[index].value, "{label}");
        assert_eq!(label, options[index].label);
    }
}

/// "Le défaut (1, AMD-052 REQ-PERS-089) **ne change pas**", and neither does the profile field nor
/// the Lua path.
#[test]
fn req_scn_086_the_default_the_profile_field_and_the_lua_path_do_not_move() {
    let descriptor = catalogue::descriptor(SettingId::VideoAntiAlias);
    assert_eq!(1, descriptor.default_value().as_int());
    assert_eq!(Some("32"), descriptor.profile_field());
    assert_eq!(Some("AntiAlias"), descriptor.lua_path());
}

/// "Un fichier de réglages […] qui la porte est relu tel quel par Zaxnes": the fourth value
/// survives a save and a load, by the label the file holds.
#[test]
fn req_scn_085_a_settings_file_carrying_the_fourth_value_reads_it_back() {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoAntiAlias, SettingValue::Enum(3));
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&model).expect("the store accepts the write");
    let written = file
        .storage()
        .text(SettingsFile::<MemoryStorage>::DEFAULT_NAME)
        .expect("the file was written");
    assert!(written.contains("\"SMAA_DEPTH\""), "{written}");

    let mut storage = MemoryStorage::new();
    storage.put(SettingsFile::<MemoryStorage>::DEFAULT_NAME, &written);
    let mut read = SettingsModel::new();
    SettingsFile::new(storage).load(&mut read, now());
    assert_eq!(3, read.get(SettingId::VideoAntiAlias).as_int());
}

/// "Une valeur hors 0..3 est ramenée comme aujourd'hui une valeur hors 0..2" — which for a choice
/// domain is `Domain::coerce`'s rule: a number no label covers takes the setting's default.
///
/// 3 used to be such a number, which is what makes this test the one that would have failed before
/// the amendment and must keep passing after it.
#[test]
fn req_scn_085_a_value_outside_the_domain_is_brought_back_as_before() {
    for (held, expected) in [(3, 3), (4, 1), (-1, 1), (99, 1)] {
        let mut storage = MemoryStorage::new();
        storage.put(
            SettingsFile::<MemoryStorage>::DEFAULT_NAME,
            &format!("{{\"version\":1,\"values\":{{\"video.anti_alias\":{held}}}}}"),
        );
        let mut read = SettingsModel::new();
        SettingsFile::new(storage).load(&mut read, now());
        assert_eq!(
            expected,
            read.get(SettingId::VideoAntiAlias).as_int(),
            "a file holding {held}"
        );
    }
}

/// The libretro enumeration keeps the three strings of AMD-032 §2, and the three still apply.
#[test]
fn req_scn_086_the_libretro_enumeration_is_not_extended() {
    let option = core_options::find("zaxnes_video_anti_alias").expect("the anti-aliasing option");
    assert_eq!(["None", "FFAA", "SMAA"], option.values.as_slice());
    assert_eq!("FFAA", option.default_value);

    let mut model = SettingsModel::new();
    for (value, label) in [(0, "None"), (1, "FFAA"), (2, "SMAA")] {
        assert!(core_options::try_apply(
            &mut model,
            "zaxnes_video_anti_alias",
            label
        ));
        assert_eq!(value, model.get(SettingId::VideoAntiAlias).as_int());
    }
    assert!(!core_options::try_apply(
        &mut model,
        "zaxnes_video_anti_alias",
        "SMAA_DEPTH"
    ));
}
