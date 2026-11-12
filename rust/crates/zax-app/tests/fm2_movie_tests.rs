//! `.fm2` movies played through the loop (EDITOR-1): the game is powered on, the movie's pads
//! reach the core frame by frame, the run-ahead is held at 0 while it plays and comes back after.
//!
//! **This is the `.fm2`-on-Zaxnes path**, which is the one the default build has: Zaxnes's own
//! clean-room importer turns the movie into a `MovieRun` and the loop feeds its words. With
//! `fceux-core` on, the application deliberately sends an `.fm2` to FCEUX's own movie mode
//! instead (lot FCEUX-48 step 4), so three of the four tests below are about a path that build
//! does not take and are compiled only without the feature; `fceux_movie_app_tests.rs` and
//! `fceux_movie_end_to_end_tests.rs` cover the same ground there — the power-on, the pads
//! reaching the core, the seek landing where it was asked to, and the movie holding paused on its
//! last frame.
//!
//! `a_refused_fm2_leaves_the_game_alone` is **not** one of them: it holds in both builds, and
//! deliberately so. It is the contract that a file the application will not play must not cost
//! the player the game they were playing, and step 7 of lot FCEUX-48 found the FCEUX path
//! breaking it — the engine switch re-powers the machine, so the refusal has to be decided
//! before the switch rather than after.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

/// The vector ROM of REQ-LOOP-065: it stores the A button of each frame at `$12`.
#[path = "../../zax-nes/tests/common/run_ahead_rom.rs"]
mod run_ahead_rom;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
#[cfg(not(feature = "fceux-core"))]
use zax_settings::catalogue::SettingId;
#[cfg(not(feature = "fceux-core"))]
use zax_settings::values::SettingValue;

use run_ahead_rom as rom;

/// An `.fm2` of 60 frames, A held on frames 20 to 39.
#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
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

#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
fn button_a(harness: &LoopHarness) -> u8 {
    harness
        .host
        .session()
        .expect("a session")
        .core()
        .read_memory(0x12)
}

#[test]
#[cfg(not(feature = "fceux-core"))] // the Zaxnes importer path; see the module docs
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

    // Played out, the movie waits on its last frame; stopped, the game goes back to the player
    // and the run-ahead to the setting.
    for _ in 0..30 {
        harness.tick_frame();
    }
    harness.run(Command::StopMovie);
    harness.run(Command::Resume);
    harness.tick_frame();
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

/// An `.fm2` of `frames` frames, A held on and off every 7 frames.
#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
fn long_movie_text(frames: u32) -> String {
    let mut text = String::from("version 3\nport0 1\nport1 0\nport2 0\n");
    for frame in 0..frames {
        text.push_str(if (frame / 7) % 2 == 0 {
            "|0|.......A|||\n"
        } else {
            "|0|........|||\n"
        });
    }
    text
}

#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
fn ram(harness: &LoopHarness) -> (i64, Vec<u8>) {
    let core = harness.host.session().expect("a session").core();
    (
        core.frame_count(),
        (0..0x0800u16)
            .map(|address| core.read_memory(address))
            .collect(),
    )
}

#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
fn play_long_movie(harness: &mut LoopHarness, frames: u32) -> std::path::PathBuf {
    harness.launch_rom_with_script("vector", &rom::build(), None);
    for _ in 0..5 {
        harness.tick_frame();
    }
    let path = std::env::temp_dir().join(format!(
        "zaxnes-fm2-seek-{}-{frames}.fm2",
        std::process::id()
    ));
    std::fs::write(&path, long_movie_text(frames)).expect("the movie is written");
    harness.host.enqueue(
        Command::PlayMovie,
        CommandArgs::of_text(&path.to_string_lossy()),
    );
    harness.tick_frame();
    path
}

#[cfg(not(feature = "fceux-core"))] // used by the Zaxnes-path tests; see the module docs
fn seek(harness: &mut LoopHarness, frame: i32) {
    harness
        .host
        .enqueue(Command::SeekMovie, CommandArgs::of_number(frame));
    for _ in 0..200 {
        harness.tick_frame();
        if harness.host.movie_seek_target().is_none() {
            return;
        }
    }
    panic!("the seek to {frame} did not arrive");
}

#[test]
#[cfg(not(feature = "fceux-core"))] // the Zaxnes importer path; see the module docs
fn a_seek_lands_on_the_same_state_as_playing_there() {
    // The reference: the movie played straight, the RAM of every frame kept.
    let mut reference = LoopHarness::create();
    let path = play_long_movie(&mut reference, 900);
    let mut states = std::collections::HashMap::new();
    for _ in 0..900 {
        let (frame, memory) = ram(&reference);
        states.insert(frame, memory);
        reference.tick_frame();
    }
    let _ = std::fs::remove_file(&path);

    let mut harness = LoopHarness::create();
    let path = play_long_movie(&mut harness, 900);
    for _ in 0..300 {
        harness.tick_frame();
    }
    // The explorer ran the movie ahead on its thread; its checkpoints met the session's own.
    for _ in 0..200 {
        if harness.host.movie_explorer_trusted().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        harness.tick_frame();
    }
    assert_eq!(harness.host.movie_explorer_trusted(), Some(true));
    let (position, _) = harness.host.movie_position().expect("a timeline");
    let start = harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_count()
        - i64::from(position);
    // Back, forward within what was played, forward past it, back to the start.
    for target in [50, 250, 700, 0, 899] {
        seek(&mut harness, target);
        let (frame, memory) = ram(&harness);
        assert_eq!(
            frame,
            start + i64::from(target),
            "the seek to {target} landed elsewhere"
        );
        assert_eq!(
            harness.host.movie_position().map(|(position, _)| position),
            Some(u32::try_from(target).unwrap())
        );
        // The reference holds every frame but the start state itself, whose tick already ran
        // the first frame.
        assert!(
            target == 0 || states.get(&frame) == Some(&memory),
            "the RAM after the seek to {target} is not the RAM of that frame"
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
#[cfg(not(feature = "fceux-core"))] // the Zaxnes importer path; see the module docs
fn a_movie_with_a_timeline_stops_paused_on_its_last_frame() {
    let mut harness = LoopHarness::create();
    let path = play_long_movie(&mut harness, 120);
    for _ in 0..200 {
        harness.tick_frame();
    }
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        harness.host.state(),
        zax_app::emulation::emulation_state::EmulationState::Paused
    );
    assert_eq!(harness.host.movie_position(), Some((120, 120)));
    assert!(harness.host.session().expect("a session").movie().is_some());
}
