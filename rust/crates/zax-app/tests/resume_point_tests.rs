//! The resume point of AMD-053 REQ-LOOP-076, the favourites of REQ-UI-210 and the search of
//! REQ-UI-211 as the host offers them.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::path::Path;

use common::{LoopHarness, SyntheticRom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::persistence::resume_point;

/// The core's state block, which tells whether the resume point restored exactly what the exit
/// captured (REQ-LOOP-060).
fn core_state(harness: &LoopHarness) -> Vec<u8> {
    harness
        .host
        .session()
        .unwrap()
        .core()
        .save_state(false)
        .unwrap()
}

/// Puts the game `g1` back in the library of the next launch, before the host is built: the order
/// of the real application, where the library is read first.
fn with_g1(options: &mut common::HostOptions, root: &Path) {
    let rom = root.join("g1.nes");
    std::fs::write(&rom, SyntheticRom::counter()).unwrap();
    options.games.add("g1", Some(&rom.to_string_lossy()));
}

/// REQ-LOOP-076: quitting during a game writes the resume point, and the next launch offers it;
/// loading it restores the state the exit captured.
#[test]
fn req_loop_076_quitting_during_a_game_offers_the_resume_point_at_the_next_launch() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(90);
    let before = core_state(&harness);

    harness.host.shutdown();

    assert!(harness.paths.resume_state_file().is_file());
    let point = resume_point::peek(&harness.paths).expect("a readable resume point");
    assert_eq!("g1", point.game_id);
    assert_eq!(harness.now(), point.saved_at);

    let mut next = harness.restart_with(with_g1);

    assert!(next.host.resume_available());
    assert_eq!(Some("g1".to_owned()), next.host.resume_game_title());
    assert!(next
        .host
        .can_execute(Command::ResumeLast, &CommandArgs::default()));

    next.run(Command::ResumeLast);
    next.settle();

    assert_eq!(Some("g1"), next.host.current_game_id());
    assert_eq!(before, core_state(&next));
    // Erasure rule 2: loading it does not erase it; it stands until the next exit.
    assert!(next.host.resume_available());
}

/// REQ-LOOP-076, erasure rule 1: quitting from Stopped empties the point, its file is gone, and
/// the next launch does not offer the entry.
#[test]
fn req_loop_076_quitting_from_stopped_clears_the_resume_point() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.host.shutdown();
    assert!(harness.paths.resume_state_file().is_file());

    // A second run that never plays: rule 1 wins over rule 2, so ignoring the entry and quitting
    // does clear the point.
    let mut next = harness.restart_with(with_g1);
    assert!(next.host.resume_available());
    assert_eq!(EmulationState::Stopped, next.host.state());

    next.host.shutdown();

    assert!(!next.paths.resume_state_file().is_file());
    assert!(!next.paths.resume_thumbnail_file().is_file());
    let third = next.restart_with(with_g1);
    assert!(!third.host.resume_available());
}

/// REQ-LOOP-076, erasure rule 3: a truncated resume file is not offered, it is deleted, no
/// notification is shown, and the start-up is normal.
#[test]
fn req_loop_076_a_truncated_resume_point_is_discarded_without_a_notification() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.host.shutdown();
    let file = harness.paths.resume_state_file();
    let whole = std::fs::read(&file).unwrap();
    std::fs::write(&file, &whole[..whole.len() / 2]).unwrap();

    let next = harness.restart_with(with_g1);

    assert!(!next.host.resume_available());
    assert!(!file.is_file());
    assert!(next.host.notifications.current().is_none());
    assert_eq!(EmulationState::Stopped, next.host.state());
    assert!(next
        .log
        .borrow()
        .iter()
        .any(|line| line.contains("resume point was discarded")));
}

/// REQ-LOOP-076, erasure rule 3 with REQ-PERS-055: a resume point whose game left the library is
/// discarded at start-up.
#[test]
fn req_loop_076_a_resume_point_whose_game_is_gone_is_discarded() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.host.shutdown();

    // The next launch never puts "g1" back in the library.
    let next = harness.restart_with(|_, _| {});

    assert!(!next.host.resume_available());
    assert!(!next.paths.resume_state_file().is_file());
    assert!(next
        .log
        .borrow()
        .iter()
        .any(|line| line.contains("no longer in the library")));
}

/// REQ-LOOP-076: exactly one `Save` hook per active script before the capture, then exactly one
/// `End` hook per active script, in that order.
#[test]
fn req_loop_076_the_exit_runs_one_save_hook_then_one_end_hook_per_script() {
    let mut harness = LoopHarness::create();
    harness.launch_with_script(
        "g1",
        "function Save() print('save') end\nfunction End() print('end') end\n",
    );
    harness.run_frames(30);
    harness.clear_script_lines();

    harness.host.shutdown();

    assert_eq!(
        vec!["save".to_owned(), "end".to_owned()],
        harness.script_lines()
    );
}

/// Q-088, confirmed by R1: `-noresume` suspends the resume point whole — it is neither read, nor
/// written, nor erased.
#[test]
fn req_ui_213_noresume_neither_reads_nor_writes_nor_erases_the_resume_point() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.host.shutdown();
    let written = std::fs::read(harness.paths.resume_state_file()).unwrap();

    // A run with -noresume: the entry is not offered and the file is left exactly as it was.
    let mut next = harness.restart_with(|options, root| {
        options.suspend_resume = true;
        with_g1(options, root);
    });
    assert!(!next.host.resume_available());
    assert!(!next
        .host
        .can_execute(Command::ResumeLast, &CommandArgs::default()));

    // Quitting from Stopped would erase it without the option (rule 1); with it, nothing moves.
    next.host.shutdown();
    assert_eq!(
        written,
        std::fs::read(next.paths.resume_state_file()).unwrap()
    );

    // And quitting mid-game writes nothing either: the option is symmetric.
    let mut playing = next.restart_with(|options, root| {
        options.suspend_resume = true;
        with_g1(options, root);
    });
    playing.run_with(Command::LaunchGame, CommandArgs::of_text("g1"));
    playing.settle();
    playing.run_frames(120);
    playing.host.shutdown();
    assert_eq!(
        written,
        std::fs::read(playing.paths.resume_state_file()).unwrap()
    );
}

/// REQ-UI-210: `favorite.toggle` on the entry named, or on the one being played, and the filter
/// the enumeration reads.
#[test]
fn req_ui_210_favorite_toggle_and_the_filter_are_commands_of_the_model() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.catalog().add("g2", None);

    assert!(!harness.host.is_favorite("g1"));
    harness.run_with(Command::ToggleFavorite, CommandArgs::of_text("g1"));
    assert!(harness.host.is_favorite("g1"));
    assert!(!harness.host.is_favorite("g2"));
    // Without a text it is the game being played.
    harness.run(Command::ToggleFavorite);
    assert!(!harness.host.is_favorite("g1"));

    assert!(!harness.host.favorites_only());
    assert!(!harness.host.is_checked(Command::FilterFavorites));
    harness.run(Command::FilterFavorites);
    assert!(harness.host.favorites_only());
    assert!(harness.host.is_checked(Command::FilterFavorites));
    harness.run(Command::FilterFavorites);
    assert!(!harness.host.favorites_only());
}

/// REQ-UI-211: `library.search` keeps the typed text, blank included.
#[test]
fn req_ui_211_library_search_keeps_the_typed_text() {
    let mut harness = LoopHarness::create();

    assert_eq!("", harness.host.library_search());
    harness.run_with(Command::SearchLibrary, CommandArgs::of_text("mario"));
    assert_eq!("mario", harness.host.library_search());
    harness.run_with(Command::SearchLibrary, CommandArgs::of_text(""));
    assert_eq!("", harness.host.library_search());
}

/// REQ-UI-213: `-emustats` writes one line per emulated frame, with its emulation time, through
/// the sink the application points at the log file. NTSC, 10 s → 600 ± 1 lines.
#[test]
fn req_ui_213_emustats_writes_one_line_per_emulated_frame() {
    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let sink = std::sync::Arc::clone(&lines);
    let mut harness = LoopHarness::create_with(move |options| {
        options.emu_stats = Some(Box::new(move |line| {
            if let Ok(mut lines) = sink.lock() {
                lines.push(line.to_owned());
            }
        }));
    });
    harness.launch("g1");
    lines.lock().unwrap().clear();

    // Ten seconds of NTSC: 600 emulated frames, ± 1 for the frame the tick boundary rounds.
    let frames = harness.run_frames(600);

    let written = lines.lock().unwrap().clone();
    assert!((599..=601).contains(&frames), "{frames} frames ran");
    assert_eq!(usize::try_from(frames).unwrap(), written.len());
    // In English (AMD-035 §2), one line per frame, with the figure.
    assert!(written[0].starts_with("emustats frame "), "{}", written[0]);
    assert!(written[0].ends_with(" ms"), "{}", written[0]);
}

/// With no `-emustats` the sink is absent and nothing is written: the option costs nothing when
/// it is off, which is the default.
#[test]
fn req_ui_213_without_emustats_nothing_is_written() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    let before = harness.log.borrow().len();

    harness.run_frames(120);

    assert!(!harness
        .log
        .borrow()
        .iter()
        .skip(before)
        .any(|line| line.contains("emustats")));
}
