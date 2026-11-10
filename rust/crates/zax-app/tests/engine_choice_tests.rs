//! The per-game engine choice of lot FCEUX-23 step 5 (§4 of `docs/review/FCEUX-ENGINE.md`).
//!
//! Every test here holds with the `fceux-core` feature **off**, which is the default build, and
//! that is the point of most of them: the setting exists, is read and written without error, is
//! not shown, and is ignored — a `games.json` written by a build with FCEUX has to load in one
//! without it. The handful that need a second engine say so.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use zax_app::emulation::engine_handle::EngineHandle;
use zax_app::persistence::save_state_file;
use zax_app::ui::{keys, options};
use zax_core_api::{Capability, EngineKind};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::{SettingEffect, SettingKind, SettingScope};
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

/// §4: a per-game option with a global default, both `zaxnes`, taking effect on load — and no
/// "auto", because an automatic choice would need a judgement the application cannot make.
#[test]
fn the_setting_is_a_two_valued_choice_that_defaults_to_zaxnes_and_acts_on_load() {
    let descriptor = catalogue::descriptor(SettingId::EmuEngine);

    assert_eq!(descriptor.key(), "emu.engine");
    assert_eq!(descriptor.kind(), SettingKind::Enum);
    // `User` is what makes it storable per game *and* as a global preference: the model layers a
    // game's own value over the preference.
    assert_eq!(descriptor.scope(), SettingScope::User);
    // What makes the options window print "(at the next load)" of its own accord.
    assert_eq!(descriptor.effect(), SettingEffect::NextRomLoad);
    assert_eq!(descriptor.default_value(), &SettingValue::Enum(0));
    // No Lua access: the original program has no such setting.
    assert_eq!(descriptor.lua_path(), None);

    let choices = descriptor.domain().options().expect("two choices");
    assert_eq!(
        choices.len(),
        2,
        "there must be no third choice, and no auto"
    );
    assert_eq!(choices[0].label, "Zaxnes");
    assert_eq!(choices[1].label, "FCEUX");
}

/// The two values are the two engines, with the numbers a `games.json` stores.
#[test]
fn the_two_values_are_the_two_engines() {
    assert_eq!(EngineKind::from_u8(0), Some(EngineKind::Zaxnes));
    assert_eq!(EngineKind::from_u8(1), Some(EngineKind::Fceux));
    assert_eq!(EngineKind::from_u8(2), None);
    assert_eq!(EngineKind::default(), EngineKind::Zaxnes);
    assert_eq!(EngineKind::Zaxnes.as_str(), "zaxnes");
    assert_eq!(EngineKind::Fceux.as_str(), "fceux");
    assert_eq!(EngineKind::parse("  FCEUX "), Some(EngineKind::Fceux));
    assert_eq!(EngineKind::parse("auto"), None);
    assert_eq!(EngineKind::Fceux.to_string(), "FCEUX");
}

/// "Without the feature the setting is ignored, read or written without error, and not shown."
#[test]
fn without_the_feature_the_setting_is_stored_and_ignored_rather_than_refused() {
    let mut model = SettingsModel::new();

    // Written and read back, whatever the build.
    assert_eq!(model.get(SettingId::EmuEngine).as_int(), 0);
    model.set(SettingId::EmuEngine, SettingValue::Enum(1));
    assert_eq!(model.get(SettingId::EmuEngine).as_int(), 1);
    // And as a game's own option, which is how `games.json` carries it.
    model.set_game_values([(SettingId::EmuEngine, SettingValue::Enum(1))]);
    assert_eq!(model.get(SettingId::EmuEngine).as_int(), 1);
    assert!(model.has_game_value(SettingId::EmuEngine));

    let shown = options::is_shown(catalogue::descriptor(SettingId::EmuEngine));
    assert_eq!(
        shown,
        cfg!(feature = "fceux-core"),
        "the row must exist exactly in a build that has a second engine"
    );
}

/// And a build with no second engine never makes one, whatever is stored: it falls back to
/// Zaxnes's core rather than failing to start the game.
#[test]
fn an_engine_the_build_has_not_got_falls_back_to_zaxnes() {
    let handle = EngineHandle::of_kind(EngineKind::Zaxnes);
    assert_eq!(handle.kind(), EngineKind::Zaxnes);

    let handle = EngineHandle::of_kind(EngineKind::Fceux);
    if cfg!(feature = "fceux-core") {
        assert_eq!(handle.kind(), EngineKind::Fceux);
    } else {
        assert_eq!(
            handle.kind(),
            EngineKind::Zaxnes,
            "a build without the engine must fall back rather than refuse the game"
        );
    }
}

/// §4: the four groups of controls FCEUX cannot honour are **shown disabled with the reason**,
/// not hidden — "a hidden control reads as a bug".
#[test]
fn with_fceux_chosen_the_four_controls_are_greyed_with_a_reason() {
    let mut model = SettingsModel::new();
    model.set(SettingId::EmuEngine, SettingValue::Enum(1));

    let expected = [
        (SettingId::VideoWidescreen, keys::HINT_FCEUX_NO_WIDESCREEN),
        (
            SettingId::VideoWidescreenExtra,
            keys::HINT_FCEUX_NO_WIDESCREEN,
        ),
        (
            SettingId::VideoWidescreenAdvanced,
            keys::HINT_FCEUX_NO_WIDESCREEN,
        ),
        (SettingId::EmuBuiltinPatches, keys::HINT_FCEUX_NO_HACKS),
        (SettingId::EmuForcedPatch, keys::HINT_FCEUX_NO_HACKS),
        (
            SettingId::VideoRemoveSpriteLimit,
            keys::HINT_FCEUX_NO_SPRITE_LIMIT,
        ),
        (SettingId::EmuRunAhead, keys::HINT_FCEUX_NO_RUN_AHEAD),
    ];

    for (id, reason) in expected {
        let note = options::inert_reason(id, &model);
        if cfg!(feature = "fceux-core") {
            assert_eq!(note, Some(reason), "{}", id.key());
            // And the reason is a sentence a user can act on, in both languages.
            for language in [
                zax_app::ui::strings::UiLanguage::English,
                zax_app::ui::strings::UiLanguage::French,
            ] {
                let text = zax_app::ui::strings::lookup(language, reason)
                    .unwrap_or_else(|| panic!("{reason} is missing in {language:?}"));
                assert!(text.len() > 20, "{reason} in {language:?}: {text:?}");
            }
        } else {
            // A build with one engine never chooses FCEUX, so nothing is greyed for it.
            assert_eq!(note, None, "{}", id.key());
        }
    }

    // With Zaxnes chosen — the default — none of them is greyed on that account.
    let zaxnes = SettingsModel::new();
    for (id, _) in expected {
        assert_eq!(options::inert_reason(id, &zaxnes), None, "{}", id.key());
    }
}

/// The capability flags the rows above are the interface's half of: FCEUX has none of the four,
/// each with a reason of its own, and it *can* roll back.
#[test]
fn the_capabilities_and_the_rows_say_the_same_thing() {
    let fceux = zax_core_api::Capabilities::FCEUX;

    for capability in [
        Capability::Widescreen,
        Capability::HackPacks,
        Capability::SpriteLimit,
        Capability::RunAhead,
    ] {
        assert!(!fceux.has(capability));
        assert!(fceux.refusal(capability).is_some());
    }
    // Rollback works on either engine, over the serialised state (§3 of the design note).
    assert!(fceux.has(Capability::Rollback));
    // But not through the cheap in-memory snapshot, which is Zaxnes's alone.
    assert!(!fceux.has(Capability::FastCoreSnapshot));
    assert!(zax_core_api::Capabilities::ZAXNES.has(Capability::FastCoreSnapshot));
}

/// A slot records the engine that wrote it, and the message a mismatch is refused with says what
/// to do about it.
#[test]
fn a_slot_records_its_engine_and_the_refusal_says_what_to_do() {
    let zaxnes = EngineHandle::of_kind(EngineKind::Zaxnes);
    assert_eq!(zaxnes.state_core_id(), "Zaxnes");
    assert_eq!(zaxnes.state_core_id(), zax_nes::NesCore::CORE_NAME);
    assert_eq!(zaxnes.state_version(), zax_nes::NesCore::STATE_VERSION);
    assert_eq!(
        zax_core_api::state_core_name(EngineKind::Fceux),
        "FCEUX",
        "the other engine's name has to differ, or a slot cannot tell them apart"
    );

    let message = save_state_file::OTHER_ENGINE_MESSAGE;
    assert!(message.contains("engine"));
    assert_ne!(
        message,
        save_state_file::CORRUPTED_MESSAGE,
        "a state of the other engine is not corrupt, and must not be called so"
    );
}

/// The two engines' blocks name themselves in their first four bytes, so a slot whose header was
/// lost still cannot be mistaken for the other's.
#[test]
fn a_block_names_its_engine_in_its_first_four_bytes() {
    use zax_core_api::{engine_of_state, state_magic};

    assert_ne!(
        state_magic(EngineKind::Zaxnes),
        state_magic(EngineKind::Fceux)
    );
    // Zaxnes's is the value its blocks have always carried ("SEN0"), so nothing written before
    // this lot changes meaning.
    assert_eq!(state_magic(EngineKind::Zaxnes), 0x304E_4553);
    assert_eq!(
        state_magic(EngineKind::Zaxnes),
        zax_nes::NesCore::STATE_MAGIC
    );

    for engine in [EngineKind::Zaxnes, EngineKind::Fceux] {
        let block = state_magic(engine).to_le_bytes();
        assert_eq!(engine_of_state(&block, 0), Some(engine));
    }
    assert_eq!(engine_of_state(&[0, 0, 0, 0], 0), None);
    assert_eq!(engine_of_state(&[1, 2, 3], 0), None);
}
