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

/// The ROM of Castle Quest, launched once with it, then "No profile" (or the retired 2D Mode)
/// remembered for it at `chosen_at`, Castle Quest's profile dated `added`.
fn remembered_no_profile(chosen_at: Option<i64>, added: i64) -> (UiHarness, String) {
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
    let none = harness
        .host
        .games()
        .default_three_dimensional_id()
        .or_else(|| harness.host.games().two_dimensional_mode_id())
        .expect("a generic entry");
    let games = harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut();
    match chosen_at {
        Some(time) => games.set_profile_for_at(&full, &none, time),
        // A choice of a games.json written before the dates: none recorded.
        None => games.set_profile_for_at(&full, &none, 0),
    }
    harness
        .host
        .games_mut()
        .library_mut()
        .find_mut("g0")
        .expect("Castle Quest")
        .added = added;
    (harness, rom)
}

/// Owner request: "No profile" chosen for a ROM gives way to a profile of the ROM the library
/// got after the choice.
#[test]
fn a_profile_added_after_no_profile_was_chosen_takes_over() {
    let (mut harness, rom) = remembered_no_profile(Some(100), 200);
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(Some("g0"), harness.host.current_game_id());
}

/// ... but "No profile" chosen after the profile was there stays.
#[test]
fn no_profile_chosen_after_the_profile_stays() {
    let (mut harness, rom) = remembered_no_profile(Some(300), 200);
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    let current = harness.host.current_game_id().map(str::to_owned);
    assert!(current.is_some_and(|id| id != "g0"));
}

/// A choice of a file written before the dates existed is older than any dated profile.
#[test]
fn an_undated_no_profile_gives_way_to_a_dated_profile() {
    let (mut harness, rom) = remembered_no_profile(None, 200);
    harness.run_with(Command::OpenRom, CommandArgs::of_text(&rom));
    assert_eq!(Some("g0"), harness.host.current_game_id());
}
