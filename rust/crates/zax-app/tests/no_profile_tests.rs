//! "No profile" (owner request): the generic 3D entry starts on the NES picture, its default 3D
//! one switch away; a game with a profile starts in 3D; the retired "2D Mode" plays as "No
//! profile".

mod common;

use common::{LoopHarness, SyntheticRom};
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
