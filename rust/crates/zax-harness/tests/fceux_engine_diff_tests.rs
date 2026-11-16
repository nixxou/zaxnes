//! Where the two engines' pictures actually differ, pixel by pixel — the measurement behind the
//! comparison table of lot FCEUX-1 step 5.
//!
//! `fceux-compare` can only say *whether* two digests match. That left step 5 with a result it
//! could not interpret: **no** ROM's picture matched at frame 60, yet over half matched at frame
//! 300. A hash cannot tell a one-pixel timing difference from a wholly different screen, so this
//! runs both engines side by side in one process and counts.
//!
//! `#[ignore]`d: it needs the `fceux-core` feature, a built fork and the public ROMs, and its job
//! is to be run deliberately and have its numbers read.

#![cfg(feature = "fceux-core")]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use zax_fceux::{Fceux, RamInit};
use zax_nes::NesCore;

/// Held for the length of every test here.
///
/// **Not belt-and-braces.** There is one FCEUX per process, and with both tests of this file
/// `--include-ignored` they ran on two threads and took the process down with an access violation
/// — `Fceux::new` refuses a second *handle*, but two threads racing to create and drop one are
/// still two threads in the library. This is what makes the file safe to run as a whole.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The frames the comparison table uses, plus frame 1.
const CHECKPOINTS: [u32; 4] = [1, 60, 300, 1200];

/// A few ROMs that between them cover a still screen, a scrolling one and a test harness.
const ROMS: [&str; 6] = [
    "other/nestest.nes",
    "other/new-game.nes",
    "blargg_ppu_tests_2005.09.15b/palette_ram.nes",
    "instr_test-v5/official_only.nes",
    "full_palette/full_palette.nes",
    "scrolltest/scroll.nes",
];

fn rom_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms");
    if root.is_dir() {
        return Some(root);
    }

    eprintln!("skipped: rust/test-roms is missing");
    None
}

#[test]
#[ignore = "a measurement, run it deliberately: cargo test -- --ignored where_the_two"]
fn where_the_two_engines_pictures_differ() {
    let _serial = lock();
    let Some(root) = rom_root() else { return };

    // One FCEUX for the whole test: there is one per process.
    let mut fceux = Fceux::new().expect("an FCEUX instance");
    fceux.set_sample_rate(48_000).expect("48 kHz");

    println!(
        "{:<48} {:>6} {:>10} {:>10} {:>10}",
        "ROM", "frame", "differ", "of 61440", "rows hit"
    );

    for name in ROMS {
        let file = root.join(name);
        let Ok(image) = std::fs::read(&file) else {
            eprintln!("{name}: not present, skipped");
            continue;
        };
        let base = file
            .file_name()
            .map_or_else(|| name.to_owned(), |b| b.to_string_lossy().into_owned());

        // Zaxnes, exactly as `golden-core` drives it: the file stem as the name, then a reset.
        let stem = Path::new(name)
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let mut zaxnes = NesCore::new();
        if !zaxnes.load_rom(&image, &stem).success {
            eprintln!("{name}: Zaxnes refused it, skipped");
            continue;
        }
        zaxnes.reset();

        // FCEUX, exactly as `fceux-run` drives it.
        fceux.set_ram_init(RamInit::Default).expect("the RAM fill");
        if fceux.load_rom(&image, &base).is_err() {
            eprintln!("{name}: FCEUX refused it, skipped");
            continue;
        }
        fceux.reset().expect("a reset");

        let mut left = Vec::new();
        let mut right = Vec::new();
        let last = CHECKPOINTS.iter().copied().max().unwrap_or(1);
        for frame in 1..=last {
            zaxnes.run_frame(0);
            if fceux
                .set_input(0, 0)
                .and_then(|()| fceux.run_frame())
                .is_err()
            {
                eprintln!("{name}: FCEUX stopped at frame {frame}");
                break;
            }

            if !CHECKPOINTS.contains(&frame) {
                continue;
            }

            zaxnes.pixel_frame_indices_into(&mut left);
            fceux.frame_indices_into(&mut right).expect("the picture");
            assert_eq!(left.len(), right.len());

            let differ = left.iter().zip(&right).filter(|(a, b)| a != b).count();
            let rows_hit = (0..240)
                .filter(|row| {
                    let span = row * 256..(row + 1) * 256;
                    left[span.clone()] != right[span]
                })
                .count();
            println!(
                "{name:<48} {frame:>6} {differ:>10} {:>10} {rows_hit:>10}",
                left.len()
            );
        }
    }
}

/// How close FCEUX's **tile records** are to Zaxnes's — the prototype of lot FCEUX-1 step 6.
///
/// The sharpest check available, and much sharper than looking at a 3D picture: the two engines
/// are asked for the same frame's records and compared field by field, in order. A record that
/// matches matches on all ten fields, including both 64-bit pattern planes.
#[test]
#[ignore = "a measurement, run it deliberately: cargo test -- --ignored how_close_the_tile"]
fn how_close_the_tile_records_are() {
    let _serial = lock();
    let Some(root) = rom_root() else { return };

    let mut fceux = Fceux::new().expect("an FCEUX instance");
    fceux.set_tile_capture(true);
    assert!(fceux.tile_capture_enabled());

    println!(
        "{:<44} {:>6} {:>7} {:>7} {:>8} {:>8} {:>8}",
        "ROM", "frame", "zaxnes", "fceux", "same", "bg same", "spr same"
    );

    for name in ROMS {
        let file = root.join(name);
        let Ok(image) = std::fs::read(&file) else {
            continue;
        };
        let stem = Path::new(name)
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let base = file
            .file_name()
            .map_or_else(|| name.to_owned(), |b| b.to_string_lossy().into_owned());

        let mut zaxnes = NesCore::new();
        if !zaxnes.load_rom(&image, &stem).success {
            continue;
        }
        zaxnes.reset();
        fceux.set_ram_init(RamInit::Default).expect("the RAM fill");
        if fceux.load_rom(&image, &base).is_err() {
            continue;
        }
        fceux.reset().expect("a reset");

        for frame in 1..=300u32 {
            zaxnes.run_frame(0);
            if fceux
                .set_input(0, 0)
                .and_then(|()| fceux.run_frame())
                .is_err()
            {
                break;
            }
            if !matches!(frame, 60 | 300) {
                continue;
            }

            let Some(capture) = fceux.take_tiles() else {
                println!("{name:<44} {frame:>6}   no capture");
                continue;
            };
            let theirs = zaxnes.tile_records();

            // Compare as multisets keyed on every field, which is what matters to the renderer:
            // a record present on both sides with the same planes, position, palette and flags is
            // the same tile, whatever order the two engines happened to emit it in.
            let key = |t: (u64, u64, i16, i16, i32, bool, bool, bool, u8, bool)| t;
            let mine: std::collections::HashSet<_> = capture
                .records
                .iter()
                .map(|r| {
                    key((
                        r.plane0,
                        r.plane1,
                        r.x,
                        r.y,
                        r.index,
                        r.mirror_h,
                        r.mirror_v,
                        r.hidden,
                        r.palette,
                        r.background,
                    ))
                })
                .collect();
            let other: std::collections::HashSet<_> = theirs
                .iter()
                .map(|r| {
                    key((
                        r.plane0,
                        r.plane1,
                        r.x,
                        r.y,
                        r.index,
                        r.mirror_h,
                        r.mirror_v,
                        r.hidden,
                        r.palette,
                        r.background,
                    ))
                })
                .collect();
            let same = mine.intersection(&other).count();
            let bg_same = mine.intersection(&other).filter(|k| k.9).count();
            let spr_same = same - bg_same;

            println!(
                "{name:<44} {frame:>6} {:>7} {:>7} {same:>8} {bg_same:>8} {spr_same:>8}",
                theirs.len(),
                capture.records.len()
            );
        }
    }
}
