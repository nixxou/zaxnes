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

use zax_fceux::{Fceux, RamInit};
use zax_nes::NesCore;

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
