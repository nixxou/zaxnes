//! The demo game's 3D scene, measured: run natively and on the web target (`tools/web/test.sh`),
//! the two must find the same draw list (the web build once drew only the sky).

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};

const ROM: &[u8] = include_bytes!("../demo/FlappyZaxNes.nes");
const PROFILE: &[u8] = include_bytes!("../demo/flappyzaxnes.3dn");

#[test]
fn the_demo_game_builds_a_3d_draw_list() {
    let profile = zax_profiles::format::load(PROFILE, None, Some("flappyzaxnes.3dn"))
        .unwrap()
        .profile;
    let mut harness = LoopHarness::create();
    let rom_path = harness.write_rom(ROM, "FlappyZaxNes.nes");
    harness.catalog().add_with(
        "game",
        Some(&rom_path),
        Some(profile),
        false,
        Some("FlappyZaxNes"),
    );
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    harness.run_frames(120);
    let session = harness.host.session_mut().expect("a session");
    let list = session.build_draw_list();
    let tiles = list.tiles().len();
    let meshes = list
        .tiles()
        .iter()
        .filter(|tile| tile.mesh.is_some())
        .count();
    println!("draw tiles {tiles} meshes {meshes}");
    assert!(tiles > 0, "the demo game draws something in 3D");
}
