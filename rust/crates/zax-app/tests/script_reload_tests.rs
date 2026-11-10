//! The script reload of EDITOR-1 (`tools.reload_script`): the profile is read again and the game
//! stays where it is; fast keeps the core and starts the new scripts there, exact replays the
//! movie from its start with them; scripts that change the core force exact.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

/// The vector ROM of REQ-LOOP-065.
#[path = "../../zax-nes/tests/common/run_ahead_rom.rs"]
mod run_ahead_rom;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};

use run_ahead_rom as rom;

/// Counts its `Update` calls by `step` into `PData[0]`, and writes `mark` into `PData[1]`.
fn counting(step: u32, mark: u32) -> String {
    format!("function Update()\n  n = (n or 0) + {step}\n  SetPData(0, n)\n  SetPData(1, {mark})\nend\n")
}

fn pdata(harness: &mut LoopHarness, index: i64) -> f64 {
    harness
        .host
        .session_mut()
        .expect("a session")
        .read_persistent_data(index + 1)
        .expect("a runtime")
}

fn ram(harness: &LoopHarness) -> (i64, Vec<u8>) {
    let core = harness.host.session().expect("a session").core();
    (
        core.frame_count(),
        (0..0x0800u16)
            .map(|address| core.read_memory(address))
            .collect(),
    )
}

/// One movie file per test: the tests of this file run in parallel.
static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The vector ROM with `source`, playing a 600-frame `.fm2`, 300 frames in.
fn playing(source: &str) -> (LoopHarness, std::path::PathBuf) {
    let mut harness = LoopHarness::create();
    harness.launch_rom_with_script("vector", &rom::build(), Some(source));
    let mut text = String::from("version 3\nport0 1\nport1 0\nport2 0\n");
    for frame in 0..600 {
        text.push_str(if (frame / 5) % 2 == 0 {
            "|0|.......A|||\n"
        } else {
            "|0|........|||\n"
        });
    }
    let path = std::env::temp_dir().join(format!(
        "zaxnes-reload-{}-{}.fm2",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("the movie is written");
    harness.host.enqueue(
        Command::PlayMovie,
        CommandArgs::of_text(&path.to_string_lossy()),
    );
    for _ in 0..300 {
        harness.tick_frame();
    }
    (harness, path)
}

fn reload(harness: &mut LoopHarness, exact: bool) {
    harness.host.enqueue(
        Command::ReloadScript,
        CommandArgs::of_number(i32::from(exact)),
    );
    harness.tick_frame();
    for _ in 0..400 {
        if harness.host.movie_seek_target().is_none() {
            return;
        }
        harness.tick_frame();
    }
    panic!("the exact reload did not arrive");
}

/// A **fast** reload during a movie, which is the Zaxnes movie path's own case.
///
/// Compiled only without `fceux-core`, and for a reason rather than to go green: a movie the
/// engine plays itself is loaded again by the reload (FCEUX's state block is the machine, not the
/// movie mode), so its clock goes to 0 and the frame on screen is reachable only by replaying to
/// it — the reload there is **always exact**, which lot FCEUX-48 step 4 records and
/// `fceux_movie_app_tests::reloading_the_profile_mid_movie_keeps_the_movie_and_the_engine` holds.
/// `a_reload_without_a_movie_is_fast` below still covers the fast path in both builds.
#[test]
#[cfg(not(feature = "fceux-core"))]
fn a_fast_reload_keeps_the_frame_and_starts_the_new_scripts_there() {
    let (mut harness, path) = playing(&counting(1, 1));
    harness.run(Command::Pause);
    assert!(pdata(&mut harness, 0) > 250.0);
    let before = ram(&harness);

    harness.catalog().set_script("vector", &counting(2, 7));
    reload(&mut harness, false);
    let _ = std::fs::remove_file(&path);
    assert_eq!(ram(&harness), before, "the core moved");
    // `movie_mode` rather than `session.movie()`: lot FCEUX-48 step 4 gave the application a
    // second source of movie frames — FCEUX's own movie mode, which an `.fm2` goes to in a build
    // with `fceux-core` and which keeps no `MovieRun` — and "the movie was lost" is the question
    // either way.
    assert!(harness.host.movie_mode().is_some(), "the movie was lost");

    harness.run(Command::Resume);
    harness.tick_frame();
    assert_eq!(pdata(&mut harness, 1), 7.0, "the new script does not run");
    // Its own count started over, two by two.
    assert!(pdata(&mut harness, 0) <= 4.0);
}

#[test]
fn an_exact_reload_replays_the_movie_with_the_new_scripts() {
    let (mut harness, path) = playing(&counting(1, 1));
    harness.run(Command::Pause);
    let before = ram(&harness);
    let calls = pdata(&mut harness, 0);

    harness.catalog().set_script("vector", &counting(2, 7));
    reload(&mut harness, true);
    let _ = std::fs::remove_file(&path);
    assert_eq!(ram(&harness), before, "the exact reload landed elsewhere");
    assert_eq!(pdata(&mut harness, 1), 7.0);
    // As if the new script had always been there: two per frame of the movie.
    let replayed = pdata(&mut harness, 0);
    assert!(
        (replayed - 2.0 * calls).abs() <= 4.0,
        "{replayed} after the exact reload, {calls} calls before"
    );
}

#[test]
fn scripts_that_change_the_core_force_an_exact_reload() {
    let writer = "function Update()\n  n = (n or 0) + 1\n  SetPData(0, n)\n  Script:WriteMem(0x0300, 1)\nend\n";
    let (mut harness, path) = playing(writer);
    harness.run(Command::Pause);
    assert!(harness.host.scripts_drive_the_core());
    let calls = pdata(&mut harness, 0);

    harness.catalog().set_script("vector", writer);
    reload(&mut harness, false);
    let _ = std::fs::remove_file(&path);
    // A fast reload would have started the count over: it was replayed instead.
    let replayed = pdata(&mut harness, 0);
    assert!(
        (replayed - calls).abs() <= 2.0,
        "{replayed} after the reload, {calls} before: not replayed"
    );
}

#[test]
fn a_reload_without_a_movie_is_fast() {
    let mut harness = LoopHarness::create();
    harness.launch_rom_with_script("vector", &rom::build(), Some(&counting(1, 1)));
    for _ in 0..60 {
        harness.tick_frame();
    }
    harness.run(Command::Pause);
    let before = ram(&harness);
    harness.catalog().set_script("vector", &counting(1, 9));
    reload(&mut harness, true);
    assert_eq!(ram(&harness), before);
    harness.run(Command::Resume);
    harness.tick_frame();
    assert_eq!(pdata(&mut harness, 1), 9.0);
}
