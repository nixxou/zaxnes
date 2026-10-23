//! Port of the application-layer part of `DisplayPacingTests.cs`. FIX-1, the two defects the user
//! reported: "Synchronisation verticale" did nothing because the setting never reached the
//! window, and every launch overwrote the persisted settings with the command-line defaults.
//!
//! Covers SPEC-06 REQ-SCN-061 and SPEC-09 `video.vsync` / `video.frame_rate`, REQ-PERS-020 /
//! REQ-PERS-021 with Q-066 (an option overrides a persisted setting only when it is explicitly
//! passed, and the override is never written back). The swap-interval arithmetic itself and the
//! window settings are tested in `zax-platform`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use common::{LoopHarness, SyntheticRom, TemporaryRoot};
use time::OffsetDateTime;
use zax_app::app_options::AppOptions;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::video_pacing::VideoPacing;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::persistence::command_line_settings::{self, OVERRIDABLE_KEYS};
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::user_data_paths::UserDataPaths;
use zax_platform::pacing::DisplaySurface;
use zax_settings::catalogue::SettingId;
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::values::SettingValue;

/// A window that records what the pacing asks of it.
#[derive(Default)]
struct FakeDisplaySurface {
    swap_interval: i32,
    frame_cap: u32,
}

impl DisplaySurface for FakeDisplaySurface {
    fn set_vsync(&mut self, enabled: bool) {
        self.swap_interval = i32::from(enabled);
    }

    fn set_frame_cap(&mut self, frames_per_second: u32) {
        self.frame_cap = frames_per_second;
    }

    fn try_set_swap_interval(&mut self, interval: i32) -> bool {
        self.swap_interval = interval;
        true
    }
}

/// What the host loop does with the changes the model reports: the pacing follows them.
fn follow(pacing: &mut VideoPacing, surface: &mut FakeDisplaySurface, model: &mut SettingsModel) {
    let ids: Vec<SettingId> = model.drain_changes().map(|change| change.id).collect();
    for id in ids {
        pacing.on_setting_changed(surface, model, id);
    }
}

fn epoch() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

fn paths_of(root: &TemporaryRoot) -> UserDataPaths {
    UserDataPaths::resolve(Some(root.path()), None, None)
}

fn options_of(configure: impl FnOnce(&mut AppOptions)) -> AppOptions {
    let mut options = AppOptions::default();
    configure(&mut options);
    options
}

/// FIX-1 (a): changing `video.vsync` at run time reaches the window's swap interval, and changing
/// `video.frame_rate` reaches its frame cap — which is what the options window of AMD-010
/// REQ-UI-202 does when the user ticks "Synchronisation verticale".
#[test]
fn req_scn_061_a_runtime_vsync_change_reaches_the_window() {
    let mut surface = FakeDisplaySurface::default();
    let mut model = SettingsModel::new();
    let mut pacing = VideoPacing::new(None, None);

    pacing.apply(&mut surface, &model);
    assert_eq!(surface.swap_interval, 0);
    assert_eq!(surface.frame_cap, 90);

    model.set_vsync(1);
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.swap_interval, 1);
    assert_eq!(surface.frame_cap, 0);

    model.set_vsync(2);
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.swap_interval, 2);

    model.set_vsync(0);
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.swap_interval, 0);
    assert_eq!(surface.frame_cap, 90);

    model.set_frame_rate(60);
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.frame_cap, 60);

    // REQ-PERS-009: writing the value already in force changes nothing and applies nothing.
    let applications = pacing.applications();
    model.set_frame_rate(60);
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(pacing.applications(), applications);
}

/// A script's write on `video.vsync` is a session value (REQ-PERS-010) and it takes effect at
/// once like any other change; the end of the game brings the preference back to the window.
#[test]
fn req_pers_010_a_script_write_on_vsync_reaches_the_window_and_is_undone() {
    let mut surface = FakeDisplaySurface::default();
    let mut model = SettingsModel::new();
    let mut pacing = VideoPacing::new(None, None);

    model.set_vsync(1);
    follow(&mut pacing, &mut surface, &mut model);
    model.set_layer(
        SettingId::VideoVSync,
        SettingValue::Enum(0),
        SettingLayer::Session,
    );
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.swap_interval, 0);

    model.end_of_game();
    follow(&mut pacing, &mut surface, &mut model);
    assert_eq!(surface.swap_interval, 1);
}

/// FIX-1 (b): a launch with no flag at all keeps every persisted setting the command line could
/// have overridden (SPEC-09 REQ-PERS-020, Q-066).
#[test]
fn q_066_a_launch_with_no_flag_keeps_every_persisted_setting() {
    let root = TemporaryRoot::new("zax-pacing");
    let paths = paths_of(&root);

    // What the user chose in the options window during a previous run.
    let mut chosen = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut chosen, epoch());
    chosen.set_fov(55.0);
    chosen.set_shadow(false);
    chosen.set_skybox(2);
    chosen.set_skip_frame(false);
    chosen.set_frame_rate(120);
    chosen.set_vsync(2);
    store.save(&chosen);

    // The next launch, with nothing on the command line.
    let options = AppOptions::default();
    let mut settings = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut settings, epoch());
    command_line_settings::apply(&options, &mut settings);

    assert_eq!(settings.fov(), 55.0);
    assert!(!settings.shadow());
    assert_eq!(settings.skybox(), 2);
    assert!(!settings.skip_frame());
    assert_eq!(settings.frame_rate(), 120);
    assert_eq!(settings.vsync(), 2);

    // No key was overridden, so the model holds no command-line value at all.
    for key in OVERRIDABLE_KEYS {
        assert!(!settings.has_command_line_value(key), "{key:?}");
    }
}

/// FIX-1 (c): an explicitly passed option overrides the persisted setting for this run and is not
/// written back; a change the user then makes takes over and is persisted (Q-066).
#[test]
fn q_066_an_explicit_flag_overrides_this_run_only() {
    let root = TemporaryRoot::new("zax-pacing");
    let paths = paths_of(&root);

    let mut chosen = SettingsModel::new();
    let mut first = SettingsStore::new(&paths);
    first.load(&mut chosen, epoch());
    chosen.set_vsync(2);
    chosen.set_fov(55.0);
    first.save(&chosen);

    // A launch that asks for no synchronisation and a 95 degree field of view.
    let options = options_of(|options| {
        options.vsync = Some(0);
        options.fov = Some(95.0);
    });
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut settings, epoch());
    command_line_settings::apply(&options, &mut settings);

    assert_eq!(settings.vsync(), 0);
    assert_eq!(settings.fov(), 95.0);
    assert!(settings.has_command_line_value(SettingId::VideoVSync));

    // REQ-PERS-021: an override is not a change by the user, so it schedules no write...
    let ids: Vec<SettingId> = settings.drain_changes().map(|change| change.id).collect();
    for id in ids {
        store.on_setting_changed(&settings, id);
    }
    assert!(!store.has_pending_write());

    // ...and the exit write of REQ-LOOP-052 keeps the user's own choices.
    store.save(&settings);
    let mut reread = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut reread, epoch());
    assert_eq!(reread.vsync(), 2);
    assert_eq!(reread.fov(), 55.0);

    // The user ticks the option during this run: that value is a preference again.
    settings.set_vsync(1);
    assert_eq!(settings.vsync(), 1);
    assert!(!settings.has_command_line_value(SettingId::VideoVSync));
    let ids: Vec<SettingId> = settings.drain_changes().map(|change| change.id).collect();
    for id in ids {
        store.on_setting_changed(&settings, id);
    }
    assert!(store.has_pending_write());

    store.save(&settings);
    let mut after_change = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut after_change, epoch());
    assert_eq!(after_change.vsync(), 1);

    // The field of view was never touched by the user, so the file still holds 55.
    assert_eq!(after_change.fov(), 55.0);
}

/// Q-066: the "Valeurs par defaut" button of REQ-UI-202 also ends an override — it is a choice by
/// the user like any other.
#[test]
fn q_066_a_reset_to_the_default_ends_the_override() {
    let mut settings = SettingsModel::new();
    command_line_settings::apply(
        &options_of(|options| options.frame_rate = Some(60)),
        &mut settings,
    );
    assert_eq!(settings.frame_rate(), 60);

    settings.reset(SettingId::VideoFrameRate);

    assert_eq!(settings.frame_rate(), 90);
    assert!(!settings.has_command_line_value(SettingId::VideoFrameRate));
}

/// FIX-1 (d): the 2D/3D toggle of REQ-UI-200 changes nothing of the pacing, and the full screen
/// of REQ-UI-209 keeps the swap interval the user asked for — the host re-applies it after the
/// window-state change, which is where the tearing in full screen came from.
#[test]
fn req_ui_209_the_2d_toggle_and_full_screen_keep_the_swap_interval() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "a.nes");
    harness.catalog().add("a", Some(&rom));

    let mut surface = FakeDisplaySurface::default();
    let mut pacing = VideoPacing::new(None, None);
    harness.host.update_settings(|model| model.set_vsync(2));
    pacing.apply(&mut surface, harness.host.settings());
    assert_eq!(surface.swap_interval, 2);

    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    harness.run(Command::ToggleTwoDimensional);
    assert!(harness.host.showing_two_dimensional());
    pacing.apply(&mut surface, harness.host.settings());
    assert_eq!(surface.swap_interval, 2);
    assert_eq!(surface.frame_cap, 0);

    // REQ-UI-209: the command flips ui.fullscreen and the host re-applies the pacing to the
    // window it has just moved.
    harness.run(Command::ToggleFullscreen);
    assert!(harness.host.settings().fullscreen());
    pacing.apply(&mut surface, harness.host.settings());

    assert_eq!(surface.swap_interval, 2);
    assert_eq!(surface.frame_cap, 0);
    assert_eq!(harness.scene.borrow().settings.vsync(), 2);
}
