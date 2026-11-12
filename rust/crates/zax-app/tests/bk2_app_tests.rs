//! A BizHawk `.bk2` opened in the application (owner request, 2026-11-12): it plays on the NesHawk
//! port, the engine it was recorded on, and the work RAM follows BizHawk's own frame by frame.
//!
//! `#[ignore]`d: it needs the *Hoshi no Kirby* ROM, its `.bk2` and BizHawk's per-frame reference,
//! which are not in the repository (`ZAXNES_MOVIES`, default `G:/3DSen/test-movies`).

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::PathBuf;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_core_api::EngineKind;

const ROM: &str = "roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes";
const MOVIE: &str = "bk2/keyword-kirbysadventure-noabilities.bk2";
const REFERENCE: &str = "bizhawk-runs/kirby-jp-step1.txt";
/// Frames compared; the reset the movie presses near frame 246 is inside them.
const FRAMES: usize = 400;

fn movies() -> PathBuf {
    std::env::var("ZAXNES_MOVIES")
        .map_or_else(|_| PathBuf::from("G:/3DSen/test-movies"), PathBuf::from)
}

/// BizHawk's work RAM after each frame, by frame number.
fn reference() -> Option<Vec<(usize, Vec<u8>)>> {
    let text = std::fs::read_to_string(movies().join(REFERENCE)).ok()?;
    let mut frames = Vec::new();
    for line in text.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let Some(frame) = fields.next().and_then(|field| field.parse::<usize>().ok()) else {
            continue;
        };
        let Some(ram) = fields.find_map(|field| field.strip_prefix("ram=")) else {
            continue;
        };
        let bytes: Vec<u8> = (0..ram.len() / 2)
            .filter_map(|index| u8::from_str_radix(&ram[index * 2..index * 2 + 2], 16).ok())
            .collect();
        frames.push((frame, bytes));
    }
    Some(frames)
}

fn work_ram(harness: &LoopHarness) -> Vec<u8> {
    let core = harness.host.session().expect("a session").core();
    (0..0x0800u16)
        .map(|address| core.read_memory(address))
        .collect()
}

#[test]
#[ignore = "needs the Hoshi no Kirby ROM, its .bk2 and BizHawk's reference"]
fn a_neshawk_bk2_plays_on_the_port_in_step_with_bizhawk() {
    let (Ok(image), Some(reference)) = (std::fs::read(movies().join(ROM)), reference()) else {
        eprintln!("skipped: the movie set is not at {:?}", movies());
        return;
    };
    let mut harness = LoopHarness::create();
    let path = harness.write_rom(&image, "kirby.nes");
    harness
        .catalog()
        .add_with("game", Some(&path), None, false, Some("Kirby"));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Zaxnes));

    let movie = movies().join(MOVIE);
    harness.run_with(
        Command::OpenMovie,
        CommandArgs::of_text(&movie.to_string_lossy()),
    );
    harness.settle();
    assert_eq!(
        harness.host.running_engine(),
        Some(EngineKind::NesHawk),
        "a movie recorded on NesHawk must play on the port"
    );
    assert_eq!(
        harness.host.movie_position().map(|(frame, _)| frame),
        Some(0)
    );

    let mut compared = 0;
    for (frame, expected) in reference.iter().take_while(|(frame, _)| *frame <= FRAMES) {
        let at = harness
            .host
            .movie_position()
            .map_or(0, |(position, _)| position as usize);
        if *frame > at {
            harness.run_exact_frames(*frame - at, &[]);
        }
        let ram = work_ram(&harness);
        let first = ram
            .iter()
            .zip(expected)
            .position(|(ours, theirs)| ours != theirs);
        assert_eq!(
            first,
            None,
            "frame {frame}: the work RAM parts at ${:04X}",
            first.unwrap_or(0)
        );
        compared += 1;
    }
    assert!(compared >= FRAMES - 2, "{compared} frames compared");
}
