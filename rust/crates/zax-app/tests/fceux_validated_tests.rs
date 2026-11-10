//! The validated-games marker of lot FCEUX-48 step 2 — the vetting half of §7 of
//! `docs/review/FCEUX-ENGINE.md`.
//!
//! Compiled in **both** builds, which is deliberate: the table is a fact about lot FCEUX-6's
//! measurements and not about the `fceux-core` feature, so it is readable and testable whether
//! FCEUX is in the binary or not. Only the *note on the row* is engine-dependent, and the two
//! branches of that are both checked below.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use zax_app::emulation::fceux_validated::{self, Status, Verdict};
use zax_app::ui::{keys, options, strings};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

/// The identity is the **CRC-32 of the ROM data**, which is what the application already uses to
/// recognise a ROM file — the one the built-in patches are keyed by. Never a file name, so the
/// same bytes under any name give the same answer and a renamed file cannot gain a verdict.
#[test]
fn a_rom_is_identified_by_the_crc_of_its_data_and_never_by_its_name() {
    // The image of Mega Man 3 [USA], built as `data_crc` reads one: a 16-byte iNES header that is
    // *not* part of the CRC, then the data. Only the data decides, so an image whose header says
    // something else still answers the same.
    let data = vec![0xA5_u8; 64];
    let crc = zax_nes::builtin_patches::crc32(&data);

    let mut image = vec![b'N', b'E', b'S', 0x1A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    image.extend_from_slice(&data);
    let mut other_header = image.clone();
    other_header[6] = 0x20; // another mapper: the same data, so the same CRC
    assert_eq!(zax_nes::builtin_patches::data_crc(&image), crc);
    assert_eq!(zax_nes::builtin_patches::data_crc(&other_header), crc);

    // And a real entry of the table is found by that CRC and by nothing else.
    let rom = fceux_validated::of_crc(0x603A_AA57).expect("Mega Man 3 [USA] is validated");
    assert_eq!(rom.game, "Mega Man 3");
    assert_eq!(rom.verdict, Verdict::Identical);
}

/// The nine games §6 marks **ship**, 47 dumps, and the five it does not.
#[test]
fn the_table_is_the_sweeps_verdicts_and_nothing_else() {
    assert_eq!(fceux_validated::all().len(), 47);

    // One dump of each of the nine, so that the list cannot shrink unnoticed.
    for (crc, game) in [
        (0x9447_6A70_u32, "Mega Man"),
        (0x0FCF_C04D, "Mega Man 2"),
        (0x603A_AA57, "Mega Man 3"),
        (0x902E_3168, "Ninja Gaiden III"),
        (0x7008_0810, "Metroid"),
        (0x3F0F_D764, "Blaster Master"),
        (0xCEBD_2A31, "Final Fantasy"),
        (0xF8A7_13BE, "Adventure Island"),
        (0xB95E_9E7F, "Punch-Out!!"),
    ] {
        let rom = fceux_validated::of_crc(crc)
            .unwrap_or_else(|| panic!("{game} ({crc:08X}) must be validated"));
        assert_eq!(rom.game, game);
        assert!(fceux_validated::is_validated(crc));
    }

    // And the five §6 does not ship, by one dump each.
    for (crc, game) in [
        (0xD445_F698_u32, "Super Mario Bros."),
        (0x0AC1_AA8F, "Castlevania"),
        (0x49AE_B3A6, "Excitebike"),
        (0xA0B0_B742, "Super Mario Bros. 3"),
        (0xBA32_2865, "Zelda II"),
    ] {
        assert!(
            !fceux_validated::is_validated(crc),
            "{game} ({crc:08X}) is not validated by §6"
        );
        assert_eq!(fceux_validated::status_of(Some(crc)), Status::NotValidated);
    }
}

/// "Not validated" and "nothing is known" are two different answers, and the interface must not
/// print the second as the first: a game that was never looked at gets no claim either way.
#[test]
fn an_unknown_rom_is_not_reported_as_unvalidated() {
    assert_eq!(fceux_validated::status_of(None), Status::Unknown);
    assert!(!fceux_validated::status_of(None).is_validated());
    assert_eq!(
        options::fceux_validation_note(SettingId::EmuEngine, &fceux_model(), Status::Unknown),
        None,
        "with no CRC the row says nothing",
    );
}

/// The marker on the engine row: it says which way it went, in both languages, and it leaves the
/// row **live** — §7's vetting is a marker and not a gate, so the option stays available for every
/// game. That is the owner's choice and this test is where it is pinned.
#[test]
fn the_engine_row_carries_the_marker_without_being_disabled() {
    let model = fceux_model();
    let validated = fceux_validated::status_of(Some(0x603A_AA57));
    let not = fceux_validated::status_of(Some(0xD445_F698));

    for (status, key) in [
        (validated, keys::HINT_FCEUX_VALIDATED),
        (not, keys::HINT_FCEUX_NOT_VALIDATED),
    ] {
        let note = options::fceux_validation_note(SettingId::EmuEngine, &model, status);
        if cfg!(feature = "fceux-core") {
            assert_eq!(
                note.as_deref(),
                Some(strings::get(key).as_str()),
                "{key} on the engine row",
            );
            // A sentence a user can act on, in both languages.
            for language in [strings::UiLanguage::English, strings::UiLanguage::French] {
                let text = strings::lookup(language, key)
                    .unwrap_or_else(|| panic!("{key} is missing in {language:?}"));
                assert!(text.len() > 20, "{key} in {language:?}: {text:?}");
            }
        } else {
            // A build with one engine never chooses FCEUX, so there is nothing to mark.
            assert_eq!(note, None, "{key} without the feature");
        }
    }

    // The row itself is never greyed on this account — that is the difference between this note
    // and `inert_reason`, which is what disables a row.
    assert_eq!(options::inert_reason(SettingId::EmuEngine, &model), None);

    // No other row carries it, and Zaxnes carries nothing.
    assert_eq!(
        options::fceux_validation_note(SettingId::EmuRunAhead, &model, validated),
        None,
    );
    assert_eq!(
        options::fceux_validation_note(SettingId::EmuEngine, &SettingsModel::new(), not),
        None,
        "with Zaxnes chosen the question does not arise",
    );
}

/// Each deviation §6 names travels with the dump it belongs to, so the interface can say *why* a
/// validated game is not pixel-identical rather than only that it is validated.
#[test]
fn a_validated_dump_carries_the_deviation_the_design_note_names() {
    let blemish = |crc: u32| {
        fceux_validated::of_crc(crc)
            .expect("validated")
            .verdict
            .blemish()
    };

    // Metroid and Punch-Out!!: identical until the two cores part at frame 3 000.
    assert_eq!(blemish(0x7008_0810), blemish(0xB95E_9E7F));
    assert!(blemish(0xB95E_9E7F).expect("a deviation").contains("3 000"));
    // Final Fantasy: one palette entry, 0.69 % of the 3D picture.
    assert!(blemish(0xCEBD_2A31).expect("a deviation").contains("$3F0B"));
    // Adventure Island: the OAM rotated by a constant, 99.999 %.
    assert!(blemish(0xF8A7_13BE).expect("a deviation").contains("OAM"));
    // And a dump that matched at every frame carries none.
    assert_eq!(blemish(0x603A_AA57), None);
}

/// A model with FCEUX chosen for the game, which is what the row is drawn from.
fn fceux_model() -> SettingsModel {
    let mut model = SettingsModel::new();
    model.set(SettingId::EmuEngine, SettingValue::Enum(1));
    model
}
