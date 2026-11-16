//! Lot FIDPLUS-D2b: which games put colour emphasis (PPUMASK bits 5-7) on screen, and where.
//!
//! A survey, not a judge: every ROM of `ZAXNES_TEST_ROMS` and `ZAXNES_ROMS` runs 1800 frames with
//! a button pattern; the ROMs that show emphasis are printed with the number of frames and the
//! bits seen. Its use is to pick the games whose pictures are checked by eye
//! (`docs/design/ANALOG-OUTPUT.md`). Run with `--nocapture`.

mod common;

use common::machine_with;
use std::path::{Path, PathBuf};

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

#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn fidplus_d2b_emphasis_survey() {
    let mut paths = Vec::new();
    for var in ["ZAXNES_TEST_ROMS", "ZAXNES_ROMS"] {
        if let Ok(folder) = std::env::var(var) {
            roms(Path::new(&folder), &mut paths);
        }
    }
    paths.sort();
    let mut plane = Vec::new();
    let mut surveyed = 0;
    for path in paths {
        let Ok(image) = std::fs::read(&path) else {
            continue;
        };
        if image.len() <= 16 || &image[..4] != b"NES\x1A" {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut core = machine_with(&image, &name);
        surveyed += 1;
        let mut frames = 0;
        let mut first = None;
        let mut bits = 0u8;
        for frame in 0..1800u16 {
            let input = if frame % 64 < 20 {
                0x08 | (frame & 0x81)
            } else {
                0
            };
            core.run_frame(input);
            if core.wide_emphasis_into(&mut plane) {
                frames += 1;
                first.get_or_insert(frame);
                bits |= plane.iter().fold(0, |all, &pixel| all | (1 << pixel));
            }
        }
        if frames > 0 {
            println!(
                "{name}: {frames} frames with emphasis, first {}, values seen {bits:08b}",
                first.unwrap_or(0)
            );
        }
    }
    println!("{surveyed} ROMs surveyed");
}
