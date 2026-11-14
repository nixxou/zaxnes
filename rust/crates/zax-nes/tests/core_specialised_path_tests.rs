//! Lot CORE-2: a game on a specialised board (`with_board_kind!`) runs exactly as on the path of
//! every board. Each ROM of `ZAXNES_TEST_ROMS` and `ZAXNES_ROMS` whose board is in the table runs
//! 600 frames both ways, with a button pattern; every frame's cycles and picture, and the full
//! savestate at the end, must match.

mod common;

use common::machine_with;
use std::path::{Path, PathBuf};

/// The mappers of the table of specialised boards.
const SPECIALISED: [u8; 7] = [0, 1, 2, 3, 4, 5, 7];

fn roms(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            roms(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("nes")) {
            out.push(path);
        }
    }
}

fn mapper(image: &[u8]) -> Option<u8> {
    (image.len() > 16 && &image[..4] == b"NES\x1A").then(|| (image[6] >> 4) | (image[7] & 0xF0))
}

#[test]
#[ignore = "needs ROMs: set ZAXNES_TEST_ROMS and/or ZAXNES_ROMS"]
fn core_2_specialised_paths_match_the_path_of_every_board() {
    let mut paths = Vec::new();
    for var in ["ZAXNES_TEST_ROMS", "ZAXNES_ROMS"] {
        if let Ok(folder) = std::env::var(var) {
            roms(Path::new(&folder), &mut paths);
        }
    }
    paths.sort();
    let mut checked = 0;
    for path in paths {
        let Ok(image) = std::fs::read(&path) else {
            continue;
        };
        if !mapper(&image).is_some_and(|m| SPECIALISED.contains(&m)) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut runs = Vec::new();
        for generic in [false, true] {
            let mut core = machine_with(&image, &name);
            core.set_generic_path(generic);
            let mut frames = Vec::new();
            for frame in 0..600u32 {
                let input = if frame % 64 < 20 { 0x08 | (frame as u16 & 0x81) } else { 0 };
                let cycles = core.run_frame(input);
                frames.push((cycles, core.pixel_frame_with_mask().to_vec()));
            }
            runs.push((frames, core.save_state(false).ok()));
        }
        let (special, generic) = (&runs[0], &runs[1]);
        for (frame, (a, b)) in special.0.iter().zip(&generic.0).enumerate() {
            assert_eq!(a.0, b.0, "{name}: cycles of frame {frame}");
            assert!(a.1 == b.1, "{name}: picture of frame {frame}");
        }
        assert!(special.1 == generic.1, "{name}: savestate after 600 frames");
        checked += 1;
    }
    eprintln!("{checked} ROMs on specialised boards checked");
}
