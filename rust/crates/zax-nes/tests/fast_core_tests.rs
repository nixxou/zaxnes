//! Lot FAST-1 (`docs/design/FAST-CORE.md`): the fast core is the same machine as the standard
//! core. Each ROM of `ZAXNES_TEST_ROMS` and `ZAXNES_ROMS` on a board with a fast path runs both
//! ways with a button pattern: every frame's cycles, picture, tile records and sound, and the
//! savestate at the end, must match; a savestate taken on either core continues on the other as
//! on itself; the widescreen extension (64 pixels a side, advanced patches) and run-ahead give
//! the same frames on both.

mod common;

use common::machine_with;
use std::path::{Path, PathBuf};
use zax_nes::NesCore;

/// The mappers with a fast path (`with_fast_board_kind!`).
const FAST: [u8; 6] = [0, 1, 2, 3, 4, 7];

fn roms(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            roms(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("nes"))
        {
            out.push(path);
        }
    }
}

fn mapper(image: &[u8]) -> Option<u8> {
    (image.len() > 16 && &image[..4] == b"NES\x1A").then(|| (image[6] >> 4) | (image[7] & 0xF0))
}

/// The ROMs on a fast board, with their names.
fn fast_roms() -> Vec<(String, Vec<u8>)> {
    let mut paths = Vec::new();
    for var in ["ZAXNES_TEST_ROMS", "ZAXNES_ROMS"] {
        if let Ok(folder) = std::env::var(var) {
            roms(Path::new(&folder), &mut paths);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let image = std::fs::read(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();
            mapper(&image)
                .is_some_and(|m| FAST.contains(&m))
                .then_some((name, image))
        })
        .collect()
}

fn input(frame: u32) -> u16 {
    #[allow(clippy::cast_possible_truncation)] // Below 0x100.
    let low = (frame & 0x81) as u16;
    if frame % 64 < 20 {
        0x08 | low
    } else {
        0
    }
}

/// What one frame leaves behind.
#[derive(PartialEq)]
struct Frame {
    cycles: i64,
    picture: Vec<u8>,
    wide: Vec<u8>,
    tiles: String,
    sound: Vec<f32>,
}

fn record(core: &NesCore, cycles: i64) -> Frame {
    let mut wide = Vec::new();
    core.wide_picture_into(&mut wide);
    let mut sound = vec![0f32; 8192];
    let read = core.read_audio(&mut sound, false).unwrap_or(0);
    sound.truncate(read);
    Frame {
        cycles,
        picture: core.pixel_frame_with_mask().to_vec(),
        wide,
        tiles: format!("{:?}", core.tile_records()),
        sound,
    }
}

fn run(core: &mut NesCore, from: u32, frames: u32, ahead: u8) -> Vec<Frame> {
    (from..from + frames)
        .map(|frame| {
            let cycles = if ahead == 0 {
                core.run_frame(input(frame))
            } else {
                core.run_frame_ahead(input(frame), ahead)
            };
            record(core, cycles)
        })
        .collect()
}

fn same(name: &str, what: &str, a: &[Frame], b: &[Frame]) {
    assert_eq!(a.len(), b.len());
    for (frame, (a, b)) in a.iter().zip(b).enumerate() {
        assert_eq!(a.cycles, b.cycles, "{name} {what}: cycles of frame {frame}");
        assert!(
            a.picture == b.picture,
            "{name} {what}: picture of frame {frame}"
        );
        assert!(
            a.wide == b.wide,
            "{name} {what}: wide picture of frame {frame}"
        );
        assert!(a.tiles == b.tiles, "{name} {what}: tiles of frame {frame}");
        assert!(a.sound == b.sound, "{name} {what}: sound of frame {frame}");
    }
}

fn core(image: &[u8], name: &str, fast: bool) -> NesCore {
    let mut core = machine_with(image, name);
    core.set_fast_core(fast);
    core
}

#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn fast_1_the_fast_core_is_the_standard_core() {
    let mut checked = 0;
    for (name, image) in fast_roms() {
        let mut standard = core(&image, &name, false);
        let mut fast = core(&image, &name, true);
        same(
            &name,
            "play",
            &run(&mut standard, 0, 600, 0),
            &run(&mut fast, 0, 600, 0),
        );
        assert!(
            standard.save_state(false).ok() == fast.save_state(false).ok(),
            "{name}: savestate after 600 frames"
        );
        checked += 1;
    }
    eprintln!("{checked} ROMs on fast boards checked");
}

#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn fast_1_savestates_cross_both_ways() {
    for (name, image) in fast_roms() {
        for first_fast in [false, true] {
            let mut first = core(&image, &name, first_fast);
            let _ = run(&mut first, 0, 300, 0);
            let state = first.save_state(false).expect("a ROM is loaded");
            // The reference is the state loaded on a fresh core of the same kind: what a
            // savestate does not hold (the sound already resampled, and on
            // RasterChromaLuma.NES the first picture after a load, standard core included:
            // docs/review/FIDELITY-LEADS.md) restarts the same way.
            let mut same_kind = core(&image, &name, first_fast);
            same_kind
                .try_load_state(&state, 0)
                .expect("the state loads");
            let mut second = core(&image, &name, !first_fast);
            second.try_load_state(&state, 0).expect("the state loads");
            let crossed = run(&mut second, 300, 300, 0);
            let reloaded = run(&mut same_kind, 300, 300, 0);
            same(
                &name,
                &format!("crossed from fast={first_fast}"),
                &reloaded,
                &crossed,
            );
            assert!(
                same_kind.save_state(false).ok() == second.save_state(false).ok(),
                "{name}: savestate after the crossing from fast={first_fast}"
            );
        }
    }
}

#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn fast_1_widescreen_and_run_ahead_match() {
    for (name, image) in fast_roms() {
        let wide = |fast: bool| {
            let mut core = NesCore::new();
            core.set_power_on_top(false);
            core.set_ppu_reset_line(false);
            core.set_advanced_patches(true);
            core.set_fast_core(fast);
            assert!(core.load_rom(&image, &name).success);
            core.set_widescreen_margin(64);
            core.set_widescreen_vertical_margin(16);
            core
        };
        let (mut standard, mut fast) = (wide(false), wide(true));
        same(
            &name,
            "widescreen",
            &run(&mut standard, 0, 300, 0),
            &run(&mut fast, 0, 300, 0),
        );
        let (mut standard, mut fast) = (core(&image, &name, false), core(&image, &name, true));
        same(
            &name,
            "run-ahead 2",
            &run(&mut standard, 0, 300, 2),
            &run(&mut fast, 0, 300, 2),
        );
    }
}

/// Real-time slices (the host's default loop): about 2.4 slices a frame, as at 144 Hz, cut
/// anywhere in the frame; the fast core must stop on the same instructions and give the same
/// machine as the standard core.
#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn fast_1_real_time_slices_match() {
    let sliced = |core: &mut NesCore| {
        let step = 29_780 * 10 / 24;
        let mut until = 0i64;
        let mut frames = Vec::new();
        for frame in 0..600u32 {
            loop {
                until += step;
                if let Some(cycles) = core.run_frame_slice(input(frame), until) {
                    frames.push(record(core, cycles));
                    break;
                }
            }
        }
        frames
    };
    for (name, image) in fast_roms() {
        let mut standard = core(&image, &name, false);
        let mut fast = core(&image, &name, true);
        same(&name, "slices", &sliced(&mut standard), &sliced(&mut fast));
        assert!(
            standard.save_state(false).ok() == fast.save_state(false).ok(),
            "{name}: savestate after 600 sliced frames"
        );
    }
}
