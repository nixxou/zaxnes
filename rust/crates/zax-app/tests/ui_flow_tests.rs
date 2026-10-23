//! The flows of the interface: opening a ROM, the welcome screen, full screen, the OSD and the
//! about block (`UiFlowTests.cs`).

mod common;
mod ui_harness;

use std::path::Path;

use ui_harness::{UiHarness, UiLanguageScope};
use zax_platform::input::Key;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
use zax_app::emulation::rom_file::INVALID_ROM_MESSAGE;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::ui::menu_shortcuts::ShortcutModifiers;
use zax_app::ui::strings::UiLanguage;
use zax_app::ui::{file_browser, text};
use zax_app::emulation::game_catalog::GameCatalog as _;
use zax_profiles::catalogue::ProfileImportOutcome;

fn full_path(path: &str) -> String {
    std::path::absolute(path)
        .expect("an absolute path")
        .to_string_lossy()
        .into_owned()
}

fn profile_for(harness: &UiHarness, rom: &str) -> Option<String> {
    harness
        .host
        .settings_store()
        .expect("a settings store")
        .games()
        .profile_for(&full_path(rom))
        .map(str::to_owned)
}

fn set_profile_for(harness: &mut UiHarness, rom: &str, game_id: &str) {
    harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut()
        .set_profile_for(&full_path(rom), game_id);
}

#[test]
fn req_ui_201_the_first_open_asks_for_a_profile_and_the_second_does_not() {
    let mut harness = UiHarness::new(&["Arkanoid", "Balloon Fight"]);
    let rom = harness.write_rom("game.nes");

    // First opening: no profile is known for the file, so the chooser opens and nothing starts.
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));

    assert!(harness.ui.state.profile_chooser);
    assert_eq!(Some(rom.as_str()), harness.ui.chooser.rom_path.as_deref());
    assert!(harness.host.current_game_id().is_none());

    // The chooser offers the catalogue, the user profiles and "2D Mode", first.
    assert_eq!(text::two_dimensional_mode(), harness.ui.chooser.all()[0].title);
    assert!(harness
        .ui
        .chooser
        .all()
        .iter()
        .any(|choice| choice.title == "Arkanoid"));

    // Choosing "Arkanoid" starts the game and remembers the pair for the file.
    harness.host.launch_rom_with_profile(&rom, "g0");
    harness.settle();
    harness.ui.state.profile_chooser = false;

    assert_eq!(Some("g0"), harness.host.current_game_id());
    assert_eq!(Some("g0".to_owned()), profile_for(&harness, &rom));

    // Second opening: direct, with no chooser.
    harness.run(Command::Stop);
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));

    assert!(!harness.ui.state.profile_chooser);
    assert_eq!(Some("g0"), harness.host.current_game_id());
}

#[test]
fn req_ui_201_the_chooser_searches_by_title() {
    let mut harness = UiHarness::new(&["Arkanoid", "Balloon Fight", "Batman"]);
    let chooser = &mut harness.ui.chooser;

    chooser.set_search("  BAT ");
    let titles: Vec<&str> = chooser.filtered().map(|choice| choice.title.as_str()).collect();
    assert_eq!(vec!["Batman"], titles);

    chooser.set_search("");
    assert_eq!(4, chooser.filtered().count());
}

#[test]
fn req_ui_201_a_forgotten_profile_reopens_the_chooser() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let rom = harness.write_rom("game.nes");

    set_profile_for(&mut harness, &rom, "gone");
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));

    assert!(harness.ui.state.profile_chooser);
}

#[test]
fn req_ui_201_the_menu_entry_opens_the_file_dialog() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let root = harness.paths.root.to_string_lossy().into_owned();
    harness.host.update_settings(|model| model.set_last_browse_directory(&root));

    harness.run(Command::OpenRom);

    assert!(harness.ui.state.file_browser);
    assert_eq!(text::open_rom_title(), harness.ui.browser.title());
    assert_eq!(Some(full_path(&root).as_str()), harness.ui.browser.directory());

    // The filters of SPEC-10 §4 plus "every file", as REQ-UI-201 asks.
    assert_eq!(["nes", "zip"], harness.ui.browser.extensions());
    assert!(harness
        .ui
        .browser
        .filters()
        .iter()
        .any(|filter| filter.label == "All files"));
}

#[test]
fn req_ui_200_the_recent_roms_are_remembered_and_pruned() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let rom = harness.write_rom("game.nes");
    set_profile_for(&mut harness, &rom, "g0");

    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    let recent = |harness: &UiHarness| -> Vec<String> {
        harness
            .host
            .settings_store()
            .expect("a settings store")
            .games()
            .recent_roms()
            .to_vec()
    };
    assert_eq!(vec![full_path(&rom)], recent(&harness));

    // The file disappears: the entry leaves the list and the user is told.
    harness.run(Command::Stop);
    std::fs::remove_file(&rom).expect("the ROM can be deleted");
    harness.run_with(Command::OpenRecentRom, CommandArgs::of_number(0));

    assert!(recent(&harness).is_empty());
    let shown = harness.host.notifications.current().expect("a notification");
    assert_eq!(INVALID_ROM_MESSAGE, shown.text);
    assert_eq!(NotificationSeverity::Error, shown.severity);
}

// Q-037
#[test]
fn req_ui_200_the_2d_toggle_is_a_hot_switch_of_the_rendering() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let rom = harness.write_rom("game.nes");
    set_profile_for(&mut harness, &rom, "g0");

    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(Some("g0"), harness.host.current_game_id());
    assert!(!harness.host.two_dimensional_view());

    // A frame is run so that the session has state the toggle must not lose.
    harness.tick(1.0 / 60.0);
    let frames = harness.host.emulated_frames();
    let session = harness.host.session().map(std::ptr::from_ref);

    harness.run(Command::ToggleTwoDimensional);
    assert!(harness.host.two_dimensional_view());
    assert!(harness.host.showing_two_dimensional());

    // No relaunch: no loading indicator, the same session object, the same current game and the
    // frames already run are still there.
    assert!(!harness.host.is_loading());
    assert_eq!(session, harness.host.session().map(std::ptr::from_ref));
    assert_eq!(Some("g0"), harness.host.current_game_id());
    assert_eq!(frames, harness.host.emulated_frames());

    // The way back is the same entry, with nothing reloaded.
    harness.run(Command::ToggleTwoDimensional);
    assert!(!harness.host.two_dimensional_view());
    assert_eq!(Some("g0"), harness.host.current_game_id());
    assert_eq!(session, harness.host.session().map(std::ptr::from_ref));
}

// Q-037
#[test]
fn req_scn_055_the_2d_mode_entry_shows_the_pixel_frame_whatever_the_toggle() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let rom = harness.write_rom("game.nes");
    set_profile_for(&mut harness, &rom, UiHarness::two_dimensional_mode_id());

    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(
        Some(UiHarness::two_dimensional_mode_id()),
        harness.host.current_game_id()
    );

    assert!(!harness.host.two_dimensional_view());
    assert!(harness.host.showing_two_dimensional());
}

#[test]
fn req_ui_207_the_welcome_screen_shows_while_no_game_is_loaded() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.draw();
    assert!(harness.ui.state.welcome_visible());

    harness.launch("g0");
    harness.draw();
    assert!(!harness.ui.state.welcome_visible());

    harness.run(Command::Stop);
    harness.draw();
    assert!(harness.ui.state.welcome_visible());

    // Q-073: full screen with no game keeps the welcome screen, which is then the only thing on
    // screen that names the way out (AMD-032 §6).
    harness.host.update_settings(|model| model.set_fullscreen(true));
    harness.draw();
    assert!(harness.ui.state.welcome_visible());
    assert!(!harness.ui.state.menu_bar_visible());

    harness.launch("g0");
    harness.draw();
    assert!(!harness.ui.state.welcome_visible());
}

#[test]
fn req_ui_209_full_screen_hides_the_menu_bar_and_keeps_the_shortcuts() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.launch("g0");
    harness.draw();

    assert!(harness.ui.state.menu_bar_visible());
    assert!(harness.ui.chrome_height > 0.0);

    harness.run(Command::ToggleFullscreen);
    harness.draw();

    assert!(harness.host.settings().fullscreen());
    assert!(!harness.ui.state.menu_bar_visible());
    assert!(harness.ui.chrome_height.abs() < f32::EPSILON);

    // The shortcuts keep working: pause, then the options window over the view.
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::P], ShortcutModifiers::NONE);
    harness.host.apply_commands();
    assert_eq!(EmulationState::Paused, harness.host.state());

    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Comma], ShortcutModifiers::CONTROL);
    harness.host.apply_commands();
    harness.pump();
    harness.draw();
    assert!(harness.ui.state.options);

    // Alt+Entrée comes back out, and the bar is visible again.
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Enter], ShortcutModifiers::ALT);
    harness.host.apply_commands();
    harness.pump();
    harness.draw();

    assert!(!harness.host.settings().fullscreen());
    assert!(harness.ui.state.menu_bar_visible());
    assert!(harness.ui.chrome_height > 0.0);
}

#[test]
fn req_ui_206_the_osd_shows_the_notifications_of_spec10() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.launch("g0");
    harness.run(Command::QuickSave);

    let shown = harness.host.notifications.current().expect("a notification");
    assert_eq!("Quick Save", shown.text);
    assert!((shown.seconds - DEFAULT_SECONDS).abs() < f64::EPSILON);
    assert_eq!(NotificationSeverity::Normal, shown.severity);

    // The OSD draws in both states and in full screen (REQ-UI-209).
    harness.draw();
    harness.host.update_settings(|model| model.set_fullscreen(true));
    harness.draw();

    // REQ-UI-081: a normal notification cannot replace a visible error.
    harness.host.notifications.show_error("Invalid rom file.");
    assert!(!harness.host.notifications.show_normal("Quick Save"));
    assert_eq!(
        NotificationSeverity::Error,
        harness.host.notifications.current().expect("a notification").severity
    );
}

#[test]
fn req_ui_095_the_frame_counter_is_off_by_default() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    assert!(!harness.host.settings().show_fps());
    assert!(!harness.ui.state.debug_overlay);

    harness.run(Command::ToggleFpsCounter);
    assert!(harness.host.settings().show_fps());

    harness.draw_frames(2);
}

#[test]
fn dec_10_q3_the_about_block_has_its_six_lines() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let lines = text::about_lines();
    assert_eq!(6, lines.len());
    assert_eq!("Zaxnes", lines[0]);
    assert_eq!(format!("Version {}", text::version()), lines[1]);
    let numbers: Vec<&str> = text::version().split('.').collect();
    assert!(numbers.len() >= 3 && numbers.iter().all(|n| n.chars().next().is_some_and(|c| c.is_ascii_digit())));
    assert_eq!("GPL v3", lines[2]);

    // AMD-035: English by default, the translation of the three attribution lines.
    assert_eq!("Independent project inspired by 3dSen (Geod Studio);", lines[3]);
    assert_eq!(
        "contains code adapted from 3DNes_Engine (Geod Studio, GPL v3).",
        lines[4]
    );
    assert_eq!("Not affiliated with Geod Studio or Nintendo.", lines[5]);

    // AMD-029 §3 writes the sentence in French, word for word, split over the three lines.
    {
        let _french = UiLanguageScope::new(UiLanguage::French);
        let lines = text::about_lines();
        assert_eq!("Projet indépendant inspiré de 3dSen (Geod Studio) ;", lines[3]);
        assert_eq!(
            "contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3).",
            lines[4]
        );
        assert_eq!("Sans lien avec Geod Studio ni Nintendo.", lines[5]);
    }

    harness.run(Command::ShowAbout);
    assert!(harness.ui.state.about);

    harness.draw_frames(2);
}

#[test]
fn req_ui_209_a_window_opened_by_shortcut_survives_full_screen() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.host.update_settings(|model| model.set_fullscreen(true));
    harness.run(Command::ShowKeyConfig);

    // The state follows the setting at the next drawn frame (REQ-PERS-067).
    harness.draw();

    assert!(harness.ui.state.key_config);
    assert!(harness.ui.state.any_window_open());
    assert!(!harness.ui.state.menu_bar_visible());
    assert!(!harness.ui.state.debug_overlay_visible());
}

#[test]
fn req_ui_200_the_whole_interface_draws_in_one_frame() {
    let mut harness = UiHarness::new(&["Arkanoid", "Balloon Fight"]);

    harness.run(Command::OpenRom);
    harness.run(Command::ShowOptions);
    harness.run(Command::ShowKeyConfig);
    harness.run(Command::ShowAbout);
    harness.ui.state.profile_chooser = true;
    harness.ui.state.debug_overlay = true;
    harness.host.update_settings(|model| model.set_show_fps(true));
    harness.host.notifications.show_normal("Quick Save");

    harness.draw_frames(4);

    assert!(harness.ui.state.any_window_open());
}

#[test]
fn req_ui_084_a_bad_profile_import_shows_the_exact_message() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let bogus = harness.paths.root.join("not-a-profile.3dn");
    std::fs::write(&bogus, "this is plain text, not a profile").expect("the file can be written");

    let result = harness.host.games_mut().import_profile(Path::new(&bogus));

    assert_eq!(ProfileImportOutcome::Rejected, result.outcome);
    assert!(!result.message.is_empty());

    harness.host.notifications.show_normal(&result.message);
    assert_eq!(
        NotificationSeverity::Normal,
        harness.host.notifications.current().expect("a notification").severity
    );
}
