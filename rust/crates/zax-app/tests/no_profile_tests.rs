//! "No profile" (owner request): the generic 3D entry starts on the NES picture, its default 3D
//! one switch away; a game with a profile starts in 3D; the retired "2D Mode" plays as "No
//! profile".

mod common;
mod ui_harness;

use common::{LoopHarness, SyntheticRom};
use ui_harness::UiHarness;
use zax_app::emulation::commands::{Command, CommandArgs};

fn harness() -> LoopHarness {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "Game.nes");
    let catalog = harness.catalog();
    catalog.add_with("2d", Some(&rom), None, true, Some("2D Mode"));
    catalog.add("3d", Some(&rom));
    catalog.default_three_dimensional_id = Some("3d".to_owned());
    catalog.add("castle", Some(&rom));
    harness
}

#[test]
fn a_game_without_a_profile_starts_in_2d_and_switches_to_3d() {
    let mut harness = harness();
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("3d"));
    assert_eq!(Some("3d"), harness.host.current_game_id());
    assert!(harness.host.showing_two_dimensional());
    harness.run(Command::ToggleTwoDimensional);
    assert!(!harness.host.showing_two_dimensional());
}

#[test]
fn a_game_with_a_profile_starts_in_3d() {
    let mut harness = harness();
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("3d"));
    assert!(harness.host.showing_two_dimensional());
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("castle"));
    assert_eq!(Some("castle"), harness.host.current_game_id());
    // The switch is back on 3D (the fake profiles are empty, hence flat whatever it says).
    assert!(!harness.host.two_dimensional_view());
}

#[test]
fn the_retired_2d_mode_plays_as_no_profile() {
    let mut harness = harness();
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("2d"));
    assert_eq!(Some("3d"), harness.host.current_game_id());
    assert!(harness.host.showing_two_dimensional());
    harness.run(Command::ToggleTwoDimensional);
    assert!(!harness.host.showing_two_dimensional());
}

/// A ROM remembered with "No profile" plays with its own profile once the library has one.
#[test]
fn a_remembered_no_profile_gives_way_to_a_profile_of_the_rom() {
    use zax_app::emulation::game_catalog::GameCatalog as _;
    let mut harness = UiHarness::new(&["Castle Quest"]);
    let rom = harness.write_rom("Castle.nes");
    let full = std::path::absolute(&rom)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    // A first launch with its profile: the library knows the ROM as Castle Quest's.
    harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut()
        .set_profile_for(&full, "g0");
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(Some("g0"), harness.host.current_game_id());
    // Then a generic entry (No profile, or the retired 2D Mode this library has) remembered for
    // it, as a choice made before the profile existed.
    let none = harness
        .host
        .games()
        .default_three_dimensional_id()
        .or_else(|| harness.host.games().two_dimensional_mode_id())
        .expect("a generic entry");
    harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut()
        .set_profile_for(&full, &none);
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(Some("g0"), harness.host.current_game_id());
}
