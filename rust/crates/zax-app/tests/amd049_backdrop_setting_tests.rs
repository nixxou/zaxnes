//! `render.backdrop_lighting` from the model to the scene: the switch AMD-049 REQ-SCN-075 needs
//! on a small machine, read **when it changes** and never per frame.
//!
//! The setting is Zaxnes's own (divergence **(D)**): the original program always lights the
//! backdrop with the almost-matt model, so it has no such setting, no Lua access to one, and no
//! `Setting` field. The priority clause T-1 of AMD-049 is what keeps the other position valid —
//! with the switch off, REQ-SCN-048's own diffuse term applies.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::TemporaryRoot;
use time::OffsetDateTime;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::scene_script_bridge;
use zax_app::ui::handheld;
use zax_app::ui::options;
use zax_app::user_data_paths::UserDataPaths;
use zax_render::scene::SceneState;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::{SettingEffect, SettingKind, SettingScope};
use zax_settings::model::SettingsModel;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

/// The desktop default follows the spec: the model of REQ-SCN-075 is on, which is what the
/// original does (AMD-049 §1).
#[test]
fn req_scn_075_the_desktop_default_is_the_model_of_the_amendment() {
    let model = SettingsModel::new();
    assert!(model.get(SettingId::RenderBackdropLighting).as_bool());

    let scene = SceneState::default();
    assert!(
        scene.settings.backdrop_lighting,
        "and so is the renderer's own default"
    );
}

/// The setting reaches the scene when it changes, through the same bridge every other scene
/// setting uses — so nothing reads it per frame (REQ-PERS-007: a change applies at the next
/// rendered frame).
#[test]
fn req_scn_075_the_setting_reaches_the_scene_when_it_changes() {
    let mut model = SettingsModel::new();
    let mut scene = SceneState::default();

    model.set(SettingId::RenderBackdropLighting, false);
    scene_script_bridge::apply_setting(SettingId::RenderBackdropLighting, &model, &mut scene);
    assert!(!scene.settings.backdrop_lighting);

    model.set(SettingId::RenderBackdropLighting, true);
    scene_script_bridge::apply_setting(SettingId::RenderBackdropLighting, &model, &mut scene);
    assert!(scene.settings.backdrop_lighting);

    // And the wholesale push, which a profile load and a start-up take.
    let mut fresh = SceneState::default();
    model.set(SettingId::RenderBackdropLighting, false);
    scene_script_bridge::push_settings(&model, &mut fresh);
    assert!(!fresh.settings.backdrop_lighting);
}

/// It is persisted (SPEC-09 §4.1): the value survives a restart.
#[test]
fn req_scn_075_the_setting_survives_a_restart() {
    let folder = TemporaryRoot::new("zax-backdrop-lighting-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);

    let mut model = SettingsModel::new();
    model.set(SettingId::RenderBackdropLighting, false);
    SettingsStore::new(&paths).save(&model);

    let mut restarted = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut restarted, at(0));
    assert!(!restarted.get(SettingId::RenderBackdropLighting).as_bool());
}

/// The shape of the descriptor: a user switch that applies immediately, with **no Lua access**
/// and no `Setting` field — the original program has neither.
#[test]
fn req_scn_075_the_setting_has_no_lua_access() {
    let descriptor = catalogue::descriptor(SettingId::RenderBackdropLighting);
    assert_eq!(SettingKind::Bool, descriptor.kind());
    assert_eq!(SettingScope::User, descriptor.scope());
    assert_eq!(SettingEffect::Immediate, descriptor.effect());
    assert_eq!(None, descriptor.lua_path());
    assert_eq!(None, descriptor.profile_field());
    for style in [0, 1, 4, 7] {
        assert_eq!(
            None,
            zax_settings::styles::parameter(style, "spl.backdrop_lighting")
        );
    }
}

/// The desktop options window shows it, in the tab REQ-UI-202 keeps for the light and the
/// shadows — it chooses the lighting model of one surface, so it belongs beside the light it
/// uses rather than among the video settings, which is where its Render theme would put it.
#[test]
fn req_scn_075_the_options_window_shows_it_with_the_light() {
    let descriptor = catalogue::descriptor(SettingId::RenderBackdropLighting);
    assert!(options::is_shown(descriptor));
    assert!(!options::EXCLUDED.contains(&SettingId::RenderBackdropLighting));
    assert_eq!(
        Some(options::OptionsTab::Light),
        options::tab_of(SettingId::RenderBackdropLighting)
    );

    let model = options::OptionsModel::new();
    assert!(
        model
            .rows(options::OptionsTab::Light)
            .iter()
            .any(|row| row.descriptor.id() == SettingId::RenderBackdropLighting),
        "a row of the Light tab"
    );

    // And it carries a help sentence, which matters because the switch shows nothing at all on a
    // profile without a backdrop (REQ-SCN-048: Lua-driven, off by default).
    let help = options::help_of(SettingId::RenderBackdropLighting).expect("a help sentence");
    assert_ne!("", help);
}

/// The handheld offers it as an **advanced** option, which is where the pad reaches the settings
/// of the desktop window a console does not otherwise show.
#[test]
fn req_scn_075_the_handheld_offers_it_as_an_advanced_option() {
    assert!(
        handheld::advanced::rows().contains(&handheld::options::OptionRow::Setting(
            SettingId::RenderBackdropLighting
        )),
        "a row of the advanced page"
    );
    assert_ne!(
        "",
        handheld::advanced::label(SettingId::RenderBackdropLighting)
    );
}

/// It has a libretro core option, as every switch of the options does (SPEC-09 §4.6).
#[test]
fn req_scn_075_the_libretro_option_is_generated() {
    let option = zax_settings::core_options::find("zaxnes_render_backdrop_lighting")
        .expect("zaxnes_render_backdrop_lighting");
    assert_eq!(SettingId::RenderBackdropLighting, option.setting);
    assert_eq!(["enabled", "disabled"], option.values.as_slice());
    assert_eq!("enabled", option.default_value);
}
