//! Fidelite+ part A (`docs/design/HALF-CYCLE.md`): outside the timing the edges were measured on
//! (the NTSC 2C02, `ppu/edges.rs`), the Fidelite+ frame loop runs exactly as the standard core.
//! Each ROM of `ZAXNES_TEST_ROMS` and `ZAXNES_ROMS` runs 600 frames both ways in PAL and in Dendy,
//! with a button pattern; every frame's cycles and picture, and the full savestate at the end, must
//! match. On NTSC the mode differs by design; its judge is the netlist corpus (A5) and the
//! campaign. (Step 1 checked all 348 ROMs in NTSC too, before any edge.)

mod common;

use common::machine_with;
use std::path::{Path, PathBuf};
use zax_nes::NesRegion;

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
fn fidplus_loop_matches_the_standard_core_outside_ntsc() {
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
        if image.len() <= 16 || &image[..4] != b"NES\x1A" {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        for region in [NesRegion::Pal, NesRegion::Dendy] {
            let mut runs = Vec::new();
            for fidelity_plus in [false, true] {
                let mut core = machine_with(&image, &name);
                core.set_region(region);
                core.set_fidelity_plus(fidelity_plus);
                let mut frames = Vec::new();
                for frame in 0..600u16 {
                    let input = if frame % 64 < 20 {
                        0x08 | (frame & 0x81)
                    } else {
                        0
                    };
                    let cycles = core.run_frame(input);
                    frames.push((cycles, core.pixel_frame_with_mask().to_vec()));
                }
                runs.push((frames, core.save_state(false).ok()));
            }
            let (standard, fidelity) = (&runs[0], &runs[1]);
            for (frame, (a, b)) in standard.0.iter().zip(&fidelity.0).enumerate() {
                assert_eq!(a.0, b.0, "{name} {region:?}: cycles of frame {frame}");
                assert!(a.1 == b.1, "{name} {region:?}: picture of frame {frame}");
            }
            assert!(
                standard.1 == fidelity.1,
                "{name} {region:?}: savestate after 600 frames"
            );
        }
        checked += 1;
    }
    eprintln!("{checked} ROMs checked");
}
