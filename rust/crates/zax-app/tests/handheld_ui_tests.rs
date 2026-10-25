//! The handheld interface over a real host: the presses of a gamepad launch games, open the
//! in-game menu, carry out its rows, change the options and quit with the combination.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::path::PathBuf;

use common::SyntheticRom;
use ui_harness::{ImGuiHarness, UiHarness};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::persistence::app_settings::{AppSettings as _, AppSettingsMut as _};
use zax_app::ui::handheld::pad::{PadButton, PadState};
use zax_app::ui::handheld::states::StatesMode;
use zax_app::ui::handheld::{HandheldUi, Screen};
use zax_app::ui::zax_ui::UiFrame;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

/// A harness whose ROM folder holds the given files.
fn start(titles: &[&str], roms: &[&str]) -> (UiHarness, HandheldUi) {
    let harness = UiHarness::new(titles);
    let folder: PathBuf = harness.root.path().join("roms");
    std::fs::create_dir_all(&folder).unwrap();
    for rom in roms {
        std::fs::write(folder.join(rom), SyntheticRom::counter()).unwrap();
    }
    let handheld = HandheldUi::new(&harness.host, folder, 480.0);
    (harness, handheld)
}

/// One frame in which `buttons` go down together, then up.
fn press(harness: &mut UiHarness, handheld: &mut HandheldUi, buttons: &[PadButton]) {
    let mut pad = PadState::new();
    for button in buttons {
        pad.set(*button, true);
    }
    frame(harness, handheld, &pad, 1.0 / 60.0);
}

fn frame(harness: &mut UiHarness, handheld: &mut HandheldUi, pad: &PadState, seconds: f32) {
    let UiHarness {
        host, input, pads, ..
    } = harness;
    handheld.update(
        pad,
        seconds,
        &mut UiFrame {
            host,
            input,
            hub: pads,
            real_seconds: seconds,
            frames_per_second: 60.0,
            profile_line: "",
        },
    );
    harness.host.set_modal_window_open(handheld.is_modal());
    harness.pump();
    harness.settle();
}

fn tap(harness: &mut UiHarness, handheld: &mut HandheldUi, button: PadButton) {
    press(harness, handheld, &[button]);
}

/// Plays the first game of the list and opens the in-game menu.
fn play(harness: &mut UiHarness, handheld: &mut HandheldUi) {
    tap(harness, handheld, PadButton::A);
    // The frame that follows the game, then the menu button.
    press(harness, handheld, &[]);
    tap(harness, handheld, PadButton::Menu);
    assert_eq!(Screen::Overlay, handheld.screen());
}

#[test]
fn the_list_shows_the_roms_in_order_with_the_profile_that_matches_by_name() {
    let (harness, handheld) = start(
        &["Castle Quest"],
        &["Zed.nes", "Castle Quest (U).nes", "x.txt"],
    );
    let entries = handheld.roms().entries();
    let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(["Castle Quest (U)", "Zed"], names.as_slice());
    assert_eq!(
        Some("Castle Quest"),
        entries[0]
            .profile
            .as_ref()
            .map(|profile| profile.title.as_str())
    );
    assert!(entries[1].profile.is_none());
    drop(harness);
}

#[test]
fn a_plays_the_game_in_2d_and_the_screen_gives_way_to_it() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    assert!(handheld.is_modal());
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Some("2d"), harness.host.current_game_id());
    // The next frame sees the game and steps aside.
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());
    assert!(!handheld.is_modal());
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn y_held_shows_the_2d_picture_head_on_and_leaves_the_camera_alone() {
    // The profiles of the harness hold no pattern: the game shows in 2D from the start.
    let (mut harness, mut handheld) = start(&["Castle Quest"], &["Castle Quest.nes"]);
    tap(&mut harness, &mut handheld, PadButton::A);
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());
    assert!(!harness.host.scene().picture_head_on);
    let camera = *harness.host.scene().camera.current();

    // Held for most of a second: 2D, seen head on.
    let mut held = PadState::new();
    held.set(PadButton::Y, true);
    for _ in 0..8 {
        frame(&mut harness, &mut handheld, &held, 0.1);
    }
    press(&mut harness, &mut handheld, &[]);
    assert!(harness.host.showing_two_dimensional());
    assert!(harness.host.scene().picture_head_on);
    assert_eq!(camera, *harness.host.scene().camera.current());

    // Leaving the 2D mode (here, the game) puts the camera back in charge.
    tap(&mut harness, &mut handheld, PadButton::Menu);
    for _ in 0..6 {
        tap(&mut harness, &mut handheld, PadButton::Down);
    }
    tap(&mut harness, &mut handheld, PadButton::A); // quit to the ROM list
    press(&mut harness, &mut handheld, &[]);
    assert!(!harness.host.showing_two_dimensional());
    assert!(!harness.host.scene().picture_head_on);
}

#[test]
fn x_chooses_the_profile_and_a_plays_with_it() {
    let (mut harness, mut handheld) = start(&["Castle Quest"], &["Castle Quest.nes"]);
    tap(&mut harness, &mut handheld, PadButton::X);
    assert_eq!(Screen::Profiles, handheld.screen());
    // The matching profile comes first.
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Some("g0"), harness.host.current_game_id());
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());
}

#[test]
fn b_leaves_the_profiles_without_playing() {
    let (mut harness, mut handheld) = start(&["Castle Quest"], &["Castle Quest.nes"]);
    tap(&mut harness, &mut handheld, PadButton::X);
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::Home, handheld.screen());
    assert_eq!(None, harness.host.current_game_id());
}

#[test]
fn the_menu_button_pauses_and_b_resumes() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    assert_eq!(EmulationState::Paused, harness.host.state());
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn save_state_writes_the_chosen_slot_and_the_game_goes_on() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    tap(&mut harness, &mut handheld, PadButton::Down); // save state
    tap(&mut harness, &mut handheld, PadButton::Right); // slot 2
                                                        // A opens the slots on the one the menu showed; A again saves there.
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Screen::States, handheld.screen());
    assert_eq!(StatesMode::Save, handheld.states().mode());
    assert_eq!(2, handheld.states().slot());
    tap(&mut harness, &mut handheld, PadButton::A);
    harness.settle();
    assert!(harness.host.slots.is_occupied("2"));
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn the_load_screen_loads_a_state_and_leaves_an_empty_slot_alone() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    tap(&mut harness, &mut handheld, PadButton::Down); // save state
    tap(&mut harness, &mut handheld, PadButton::A);
    tap(&mut harness, &mut handheld, PadButton::A); // saves in slot 1
    harness.settle();
    assert!(harness.host.slots.is_occupied("1"));

    tap(&mut harness, &mut handheld, PadButton::Menu);
    tap(&mut harness, &mut handheld, PadButton::Down);
    tap(&mut harness, &mut handheld, PadButton::Down); // load state
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Screen::States, handheld.screen());
    assert_eq!(StatesMode::Load, handheld.states().mode());
    // Slot 2 is empty: A loads nothing and the screen stays.
    tap(&mut harness, &mut handheld, PadButton::Down);
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Screen::States, handheld.screen());
    // B goes back to the menu; the load of slot 1 resumes the game.
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::Overlay, handheld.screen());
    tap(&mut harness, &mut handheld, PadButton::A);
    tap(&mut harness, &mut handheld, PadButton::A);
    harness.settle();
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn quit_to_the_rom_list_stops_the_game() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    // Quit to ROM list is the seventh row (no Rewind row on the handheld).
    for _ in 0..6 {
        tap(&mut harness, &mut handheld, PadButton::Down);
    }
    tap(&mut harness, &mut handheld, PadButton::A);
    harness.settle();
    assert_eq!(Screen::Home, handheld.screen());
    assert_eq!(None, harness.host.current_game_id());
}

#[test]
fn exit_to_the_launcher_quits_the_application() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    tap(&mut harness, &mut handheld, PadButton::Up); // wraps to the last row
    tap(&mut harness, &mut handheld, PadButton::A);
    harness.settle();
    assert!(harness.host.quit_requested());
}

#[test]
fn the_exit_combination_ends_with_the_settings_and_the_slot_index_written() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    harness
        .host
        .update_settings(|model| model.set_show_fps(true));
    harness.settle();
    let mut pad = PadState::new();
    pad.set(PadButton::Select, true);
    pad.set(PadButton::Start, true);
    for _ in 0..12 {
        frame(&mut harness, &mut handheld, &pad, 0.1);
    }
    harness.settle();
    assert!(harness.host.quit_requested());
    // What the application does once the loop sees the quit request (`Running::finish`).
    harness.host.shutdown();
    let written = std::fs::read_to_string(&harness.paths.settings_file).unwrap();
    assert!(written.contains("show_fps"), "{written}");
    assert!(harness.paths.save_index_file.is_file());
}

#[test]
fn select_and_start_held_for_a_second_quit_and_a_tap_does_not() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    let mut pad = PadState::new();
    pad.set(PadButton::Select, true);
    pad.set(PadButton::Start, true);
    frame(&mut harness, &mut handheld, &pad, 0.3);
    pad.set(PadButton::Select, false);
    frame(&mut harness, &mut handheld, &pad, 0.016);
    assert!(!harness.host.quit_requested());

    pad.set(PadButton::Select, true);
    for _ in 0..12 {
        frame(&mut harness, &mut handheld, &pad, 0.1);
    }
    assert!(harness.host.quit_requested());
}

#[test]
fn the_options_change_the_settings_of_the_host() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    tap(&mut harness, &mut handheld, PadButton::Y);
    assert_eq!(Screen::Options, handheld.screen());
    tap(&mut harness, &mut handheld, PadButton::Down); // volume
    tap(&mut harness, &mut handheld, PadButton::Left);
    assert!((harness.host.settings().volume() - 0.9).abs() < 1.0e-6);
    for _ in 0..3 {
        tap(&mut harness, &mut handheld, PadButton::Down); // render scale, shadows, frame cap
    }
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(2, harness.host.settings().vsync());
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::Home, handheld.screen());
}

#[test]
fn the_options_of_a_game_hold_only_what_changes_and_play_with_it() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes", "Other.nes"]);
    let rom = harness.root.path().join("roms").join("Game.nes");
    let rom = rom.to_string_lossy().into_owned();
    let shadows = harness.host.settings().shadow();
    tap(&mut harness, &mut handheld, PadButton::Select);
    assert_eq!(Screen::GameOptions, handheld.screen());
    // Reset row, volume, render scale, shadows.
    for _ in 0..3 {
        tap(&mut harness, &mut handheld, PadButton::Down);
    }
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(
        harness.host.game_options(&rom),
        [(SettingId::RenderShadow, SettingValue::Bool(!shadows))]
    );
    assert_eq!(
        shadows,
        harness.host.settings().shadow(),
        "the global value stays"
    );
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::Home, handheld.screen());

    // The game plays with it, and the end of the game gives the global value back.
    tap(&mut harness, &mut handheld, PadButton::A);
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(!shadows, harness.host.settings().shadow());
    tap(&mut harness, &mut handheld, PadButton::Menu);
    for _ in 0..6 {
        tap(&mut harness, &mut handheld, PadButton::Down);
    }
    tap(&mut harness, &mut handheld, PadButton::A); // quit to the ROM list
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::Home, handheld.screen());
    assert_eq!(shadows, harness.host.settings().shadow());

    // X puts everything back.
    tap(&mut harness, &mut handheld, PadButton::Select);
    tap(&mut harness, &mut handheld, PadButton::X);
    assert_eq!(0, harness.host.game_options(&rom).len());
}

#[test]
fn the_options_of_the_menu_go_back_to_the_menu() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    play(&mut harness, &mut handheld);
    // Options is the sixth row (no Rewind row on the handheld).
    for _ in 0..5 {
        tap(&mut harness, &mut handheld, PadButton::Down);
    }
    tap(&mut harness, &mut handheld, PadButton::A);
    assert_eq!(Screen::Options, handheld.screen());
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::Overlay, handheld.screen());
}

#[test]
fn select_and_start_tapped_open_the_menu_only_when_the_setting_says_so() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    tap(&mut harness, &mut handheld, PadButton::A);
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());
    let tap_combo = |harness: &mut UiHarness, handheld: &mut HandheldUi| {
        let mut pad = PadState::new();
        pad.set(PadButton::Select, true);
        pad.set(PadButton::Start, true);
        frame(harness, handheld, &pad, 0.2);
        frame(harness, handheld, &PadState::new(), 0.016);
    };
    tap_combo(&mut harness, &mut handheld);
    assert_eq!(Screen::InGame, handheld.screen(), "off by default");
    harness.host.update_settings(|settings| {
        settings.set(
            zax_settings::catalogue::SettingId::InputEmulatedMenuButton,
            true,
        );
    });
    tap_combo(&mut harness, &mut handheld);
    assert_eq!(Screen::Overlay, handheld.screen());
}

#[test]
fn every_screen_draws_at_the_three_sizes_of_the_devices() {
    let (mut harness, mut handheld) = start(&["Castle Quest"], &["Castle Quest.nes", "Zed.nes"]);
    let mut imgui = ImGuiHarness::new();
    for size in [[640.0, 480.0], [720.0, 480.0], [720.0, 720.0]] {
        imgui.context_mut().io_mut().display_size = size;
        for screen in [
            Screen::Home,
            Screen::Profiles,
            Screen::Options,
            Screen::GameOptions,
            Screen::Controls,
            Screen::Overlay,
            Screen::InGame,
        ] {
            handheld.arrange_for_capture(screen);
            let UiHarness {
                host, input, pads, ..
            } = &mut harness;
            let frame = UiFrame {
                host,
                input,
                hub: pads,
                real_seconds: 1.0 / 60.0,
                frames_per_second: 60.0,
                profile_line: "",
            };
            imgui.frame(|ui| handheld.draw(ui, &frame));
            assert_eq!(screen, handheld.screen());
        }
    }
}

/// The shortcuts of the desktop (Ctrl+S, Ctrl+L) open the state screens over the game, which
/// pauses; going back resumes it. Leaving the console mode closes a menu left open and the game
/// goes on.
#[test]
fn the_state_screens_open_over_the_game_and_the_desktop_takes_the_game_back() {
    let (mut harness, mut handheld) = start(&[], &["Game.nes"]);
    assert!(
        !handheld.open_states(StatesMode::Load, &mut harness.host),
        "no game"
    );
    tap(&mut harness, &mut handheld, PadButton::A);
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::InGame, handheld.screen());

    assert!(handheld.open_states(StatesMode::Save, &mut harness.host));
    press(&mut harness, &mut handheld, &[]);
    assert_eq!(Screen::States, handheld.screen());
    assert_eq!(StatesMode::Save, handheld.states().mode());
    assert_eq!(EmulationState::Paused, harness.host.state());
    tap(&mut harness, &mut handheld, PadButton::B);
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(EmulationState::Running, harness.host.state());

    tap(&mut harness, &mut handheld, PadButton::Menu);
    assert_eq!(Screen::Overlay, handheld.screen());
    handheld.leave_for_desktop(&mut harness.host);
    harness.pump();
    harness.settle();
    assert_eq!(Screen::InGame, handheld.screen());
    assert_eq!(EmulationState::Running, harness.host.state());
}

/// A ROM folder chosen in the options: the list shows its games from then on.
#[test]
fn another_rom_folder_lists_its_games() {
    let (harness, mut handheld) = start(&[], &["Old.nes"]);
    let other = harness.root.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("New.nes"), SyntheticRom::counter()).unwrap();
    handheld.set_roms_directory(&harness.host, other.clone());
    assert_eq!(other.as_path(), handheld.roms_directory());
    let names: Vec<&str> = handheld
        .roms()
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(["New"], names.as_slice());
}
