//! `.fm2` movies played through the loop (EDITOR-1): the game is powered on, the movie's pads
//! reach the core frame by frame, the run-ahead is held at 0 while it plays and comes back after.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

/// The vector ROM of REQ-LOOP-065: it stores the A button of each frame at `$12`.
#[path = "../../zax-nes/tests/common/run_ahead_rom.rs"]
mod run_ahead_rom;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

use run_ahead_rom as rom;

/// An `.fm2` of 60 frames, A held on frames 20 to 39.
fn movie_text() -> String {
    let mut text =
        String::from("version 3\nemuVersion 22020\npalFlag 0\nport0 1\nport1 0\nport2 0\n");
    for frame in 0..60 {
        let pad = if (20..40).contains(&frame) {
            ".......A"
        } else {
            "........"
        };
        text.push_str("|0|");
        text.push_str(pad);
        text.push_str("|||\n");
    }
    text
}

fn button_a(harness: &LoopHarness) -> u8 {
    harness
        .host
        .session()
        .expect("a session")
        .core()
        .read_memory(0x12)
}

#[test]
fn an_fm2_plays_from_power_on_at_run_ahead_zero() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(1)));
    harness.launch_rom_with_script("vector", &rom::build(), None);
    for _ in 0..100 {
        harness.tick_frame();
    }
    assert_eq!(1, harness.host.session().expect("a session").run_ahead());

    let path = std::env::temp_dir().join(format!("zaxnes-fm2-test-{}.fm2", std::process::id()));
    std::fs::write(&path, movie_text()).expect("the movie is written");
    harness.host.enqueue(
        Command::PlayMovie,
        CommandArgs::of_text(&path.to_string_lossy()),
    );
    harness.tick_frame();
    let session = harness.host.session().expect("a session");
    assert!(session.movie().is_some(), "the movie does not play");
    assert!(
        session.core().frame_count() < 10,
        "the game was not powered on"
    );

    let mut seen_a = Vec::new();
    for _ in 0..45 {
        harness.tick_frame();
        let session = harness.host.session().expect("a session");
        assert_eq!(0, session.run_ahead(), "run-ahead while a movie plays");
        seen_a.push(button_a(&harness));
    }
    let _ = std::fs::remove_file(&path);
    // A shows from about frame 20 to about frame 40, released around it.
    assert_eq!(0, seen_a[10]);
    assert_eq!(1, seen_a[30]);
    assert_eq!(0, seen_a[44]);

    // Played out: the game goes back to the player, and the run-ahead to the setting.
    for _ in 0..30 {
        harness.tick_frame();
    }
    let session = harness.host.session().expect("a session");
    assert!(session.movie().is_none(), "the movie did not end");
    assert_eq!(1, session.run_ahead(), "the run-ahead did not come back");
}

#[test]
fn a_refused_fm2_leaves_the_game_alone() {
    let mut harness = LoopHarness::create();
    harness.launch_rom_with_script("vector", &rom::build(), None);
    for _ in 0..10 {
        harness.tick_frame();
    }
    let before = harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_count();
    let path = std::env::temp_dir().join(format!("zaxnes-fm2-bad-{}.fm2", std::process::id()));
    std::fs::write(&path, "version 3\nbinary 1\n").expect("the movie is written");
    harness.host.enqueue(
        Command::PlayMovie,
        CommandArgs::of_text(&path.to_string_lossy()),
    );
    harness.tick_frame();
    let _ = std::fs::remove_file(&path);
    let session = harness.host.session().expect("a session");
    assert!(session.movie().is_none());
    assert!(session.core().frame_count() > before, "the game was reset");
}
