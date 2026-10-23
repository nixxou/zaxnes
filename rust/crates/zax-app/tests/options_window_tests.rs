//! The options window as data and as a drawn frame (`OptionsWindowTests.cs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::collections::HashSet;

use ui_harness::{UiHarness, UiLanguageScope};
use zax_app::audio::{AudioMixer, TrackRandom};
use zax_app::ui::options::{self, OptionsModel, OptionsRow, OptionsTab};
use zax_app::ui::strings::{self, UiLanguage};
use zax_platform::WaveAudio;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::{SettingScope, SettingTheme};

fn keys(model: &OptionsModel, tab: OptionsTab) -> Vec<SettingId> {
    model
        .rows(tab)
        .iter()
        .map(|row| row.descriptor.id())
        .collect()
}

fn row(model: &OptionsModel, id: SettingId) -> OptionsRow {
    let rows: Vec<&OptionsRow> = model
        .all_rows()
        .filter(|row| row.descriptor.id() == id)
        .collect();
    assert_eq!(1, rows.len());
    *rows[0]
}

#[test]
fn req_ui_202_every_user_preference_is_shown() {
    let model = OptionsModel::new();
    let shown: HashSet<SettingId> = model.all_rows().map(|row| row.descriptor.id()).collect();

    let missing: Vec<String> = catalogue::of_scope(SettingScope::User)
        .filter(|descriptor| {
            !shown.contains(&descriptor.id())
                && !options::EXCLUDED.contains(&descriptor.id())
                && !matches!(
                    descriptor.theme(),
                    SettingTheme::Vr
                        | SettingTheme::Editor
                        | SettingTheme::Movie
                        | SettingTheme::Internal
                )
        })
        .map(|descriptor| descriptor.key().into_owned())
        .collect();

    assert_eq!(Vec::<String>::new(), missing);
}

#[test]
fn req_ui_202_the_settings_spec10_showed_are_all_there() {
    let model = OptionsModel::new();
    let shown: HashSet<SettingId> = model.all_rows().map(|row| row.descriptor.id()).collect();

    // The rows of the table of REQ-UI-061, in its order. "Musique de menu" (audio.bg_music) left
    // the window with the menu music itself, which AMD-024 REQ-PUB-005 removed on 2026-10-21;
    // the setting stays in the model, without effect.
    let visible = [
        SettingId::LightIntensity,
        SettingId::AudioVolume,
        SettingId::RenderShadow,
        SettingId::VideoFrameRate,
        SettingId::VideoSkipFrame,
        SettingId::EmuGameSpeed,
        SettingId::CameraFov,
        SettingId::EmuTvMode,
        SettingId::SceneSkybox,
        SettingId::UiFullscreen,
    ];
    for id in visible {
        assert!(shown.contains(&id), "{}", id.key());
    }

    // REQ-UI-063 and REQ-UI-090: the custom background colour goes with the skybox list.
    assert!(shown.contains(&SettingId::SceneCustomBgColor));
}

#[test]
fn req_ui_202_the_six_tabs_are_the_ones_the_amendment_names() {
    let _language = UiLanguageScope::keep();
    strings::set_language(UiLanguage::English);

    let titles = |tabs: &[OptionsTab]| -> Vec<String> {
        tabs.iter().map(|tab| options::title_of(*tab)).collect()
    };
    assert_eq!(
        vec![
            "Video",
            "Camera",
            "Light and shadows",
            "Audio",
            "Emulation",
            "Interface"
        ],
        titles(&options::TABS)
    );

    strings::set_language(UiLanguage::French);
    assert_eq!(
        vec![
            "Vidéo",
            "Caméra",
            "Lumière et ombres",
            "Audio",
            "Émulation",
            "Interface"
        ],
        titles(&options::TABS)
    );

    strings::set_language(UiLanguage::English);

    let model = OptionsModel::new();
    for tab in options::TABS {
        assert!(!model.rows(tab).is_empty());
        for row in model.rows(tab) {
            assert_ne!(row.label(), "");
        }
    }

    // The field of view belongs to the camera tab, the shadows to the light one, and the scene
    // decor to the video one.
    assert!(keys(&model, OptionsTab::Camera).contains(&SettingId::CameraFov));
    assert!(keys(&model, OptionsTab::Light).contains(&SettingId::RenderShadow));
    assert!(keys(&model, OptionsTab::Light).contains(&SettingId::LightIntensity));
    assert!(keys(&model, OptionsTab::Video).contains(&SettingId::SceneSkybox));
    assert!(keys(&model, OptionsTab::Audio).contains(&SettingId::AudioVolume));
    assert!(keys(&model, OptionsTab::Emulation).contains(&SettingId::EmuGameSpeed));
    assert!(keys(&model, OptionsTab::Interface).contains(&SettingId::UiShowFps));
}

#[test]
fn req_ui_202_the_vr_settings_are_absent() {
    let model = OptionsModel::new();
    let shown: HashSet<SettingId> = model.all_rows().map(|row| row.descriptor.id()).collect();

    for descriptor in catalogue::of_theme(SettingTheme::Vr) {
        assert!(!shown.contains(&descriptor.id()), "{}", descriptor.key());
    }
    assert!(!shown.contains(&SettingId::UiLastBrowseDir));
    assert!(!shown.contains(&SettingId::UiWindowRect));
}

#[test]
fn req_ui_202_the_widgets_use_the_interface_bounds() {
    let model = OptionsModel::new();

    let fov = row(&model, SettingId::CameraFov);
    assert!((fov.minimum - 45.0).abs() < f64::EPSILON);
    assert!((fov.maximum - 95.0).abs() < f64::EPSILON);
    assert!((fov.step - 5.0).abs() < f64::EPSILON);

    let light = row(&model, SettingId::LightIntensity);
    assert!((light.minimum - 0.5).abs() < 1e-9);
    assert!((light.maximum - 1.5).abs() < 1e-9);

    let speed = row(&model, SettingId::EmuGameSpeed);
    assert!((speed.minimum + 8.0).abs() < f64::EPSILON);
    assert!((speed.maximum - 8.0).abs() < f64::EPSILON);
}

#[test]
fn req_ui_202_a_change_takes_effect_and_the_reset_restores_the_defaults() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let model = OptionsModel::new();

    harness.host.update_settings(|settings| {
        settings.set(SettingId::CameraFov, 85.0_f32);
        settings.set(SettingId::RenderShadow, false);
    });

    // REQ-PERS-009: the change reaches the scene at once.
    assert!((harness.host.scene().settings.fov - 85.0).abs() < 1e-4);
    assert!(!harness.host.scene().settings.shadows);
    assert!(harness
        .host
        .settings_store()
        .expect("a settings store")
        .has_pending_write());

    harness
        .host
        .update_settings(|settings| model.reset_tab(settings, OptionsTab::Camera));
    assert!((harness.host.settings().get(SettingId::CameraFov).as_float() - 70.0).abs() < 1e-4);

    // The shadows are on the light tab, so the camera reset leaves them alone.
    assert!(!harness
        .host
        .settings()
        .get(SettingId::RenderShadow)
        .as_bool());

    harness
        .host
        .update_settings(|settings| model.reset_tab(settings, OptionsTab::Light));
    assert!(harness
        .host
        .settings()
        .get(SettingId::RenderShadow)
        .as_bool());
}

#[test]
fn req_ui_202_the_window_draws() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.ui.state.options = true;
    harness.draw_frames(3);

    assert!(harness.ui.state.options);
}

#[test]
fn amd_021_r2_013_speed_and_fov_are_the_only_silent_settings() {
    assert_eq!(
        [SettingId::EmuGameSpeed, SettingId::CameraFov],
        options::SILENT
    );

    assert!(!options::plays_validation_sound(SettingId::EmuGameSpeed));
    assert!(!options::plays_validation_sound(SettingId::CameraFov));

    // Every other row of every tab, the drop-downs, the check boxes and the colour picker
    // included: the validation sound.
    let model = OptionsModel::new();
    for row in model.all_rows() {
        let id = row.descriptor.id();
        let silent = id == SettingId::EmuGameSpeed || id == SettingId::CameraFov;
        assert_eq!(!silent, options::plays_validation_sound(id));
    }

    // The named rows of the table of REQ-AUD-041, now extended to the whole page.
    for id in [
        SettingId::AudioVolume,
        SettingId::LightIntensity,
        SettingId::RenderShadow,
        SettingId::SceneSkybox,
        SettingId::VideoFrameRate,
        SettingId::UiFullscreen,
        SettingId::SceneCustomBgColor,
    ] {
        assert!(options::plays_validation_sound(id), "{}", id.key());
    }
}

/// The audio model: exactly one sound per change of value.
#[test]
fn amd_021_r2_013_a_change_plays_the_validation_sound_exactly_once() {
    // The validation clip of REQ-AUD-041; the harness mixer carries no assets, so the clip is
    // supplied directly. REQ-AUD-040 has a single interface voice, which is why the count and
    // not `sound_playing` is what tells one sound from two.
    let mut mixer = AudioMixer::new(48_000, None, TrackRandom::new(1));
    let validation = WaveAudio {
        samples: vec![0.0; 480],
        channels: 1,
        sample_rate: 48_000,
    };

    let mut play = |id: SettingId| {
        let before = mixer.sounds_played;
        if options::plays_validation_sound(id) {
            mixer.play_clip(Some(validation.clone()));
        }
        mixer.sounds_played - before
    };

    // Three changes of the shadow check box: three sounds, one each.
    assert_eq!(1, play(SettingId::RenderShadow));
    assert_eq!(1, play(SettingId::RenderShadow));
    assert_eq!(1, play(SettingId::AudioVolume));

    // Speed and FOV: none.
    assert_eq!(0, play(SettingId::EmuGameSpeed));
    assert_eq!(0, play(SettingId::CameraFov));
}
