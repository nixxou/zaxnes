//! A survey of every built-in pack for portable states: for each ROM of `ZAXNES_PACK_ROMS`
//! (folders, `;`-separated) a built-in patch is made for, a state made at the game's idle loop
//! with the hack is played on without it, and the other way round, against the run that never
//! changed mode. Prints a table; asserts nothing (the per-game round trips do).
//!
//! `cargo test -p zax-nes --release --test portable_pack_survey_tests -- --ignored --nocapture`

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use zax_nes::builtin_patches::{patch_for, BuiltinPatch};
use zax_nes::NesCore;

/// Frames the states are made at, and how far each run goes on after.
const SAVES: [usize; 3] = [900, 1800, 3000];
const AFTER: usize = 600;

const A: u16 = 0x01;
const B: u16 = 0x02;
const START: u16 = 0x08;
const RIGHT: u16 = 0x80;

/// A pad that gets most games past their title and moving: Start now and then, Right held, B
/// held (run), A every half second.
fn input(frame: usize) -> u16 {
    let start = matches!(frame % 300, 60..=64) && frame < 1500;
    let mut pad = if start { START } else { 0 };
    if frame > 400 {
        pad |= RIGHT | B;
        if frame % 30 < 8 {
            pad |= A;
        }
    }
    pad
}

fn roms() -> Vec<(PathBuf, Vec<u8>, &'static BuiltinPatch)> {
    let Some(folders) = std::env::var_os("ZAXNES_PACK_ROMS") else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for folder in std::env::split_paths(&folders) {
        let Ok(entries) = walk(&folder) else { continue };
        for path in entries {
            let Ok(image) = std::fs::read(&path) else {
                continue;
            };
            if let Some(patch) = patch_for(&image).filter(|patch| seen.insert(patch.name)) {
                out.push((path, image, patch));
            }
        }
    }
    out
}

fn walk(folder: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(walk(&path).unwrap_or_default());
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
        {
            out.push(path);
        }
    }
    Ok(out)
}

fn core(image: &[u8], name: &str, on: bool) -> Option<NesCore> {
    let mut core = NesCore::new();
    core.set_builtin_patches(on);
    core.set_advanced_patches(on);
    core.load_rom(image, name).success.then_some(())?;
    core.set_widescreen_margin(if on { 64 } else { 0 });
    Some(core)
}

fn game_ram(core: &NesCore) -> Vec<u8> {
    let mut ram = core.bus().ram().to_vec();
    let top = 0x0100 + usize::from(core.cpu().s);
    ram[0x0100..=top.min(0x01FF)].fill(0);
    ram
}

fn picture(core: &NesCore) -> Vec<u8> {
    let mut out = Vec::new();
    core.pixel_frame_indices_into(&mut out);
    out
}

fn diff(left: &[u8], right: &[u8]) -> usize {
    left.iter().zip(right).filter(|(l, r)| l != r).count()
}

/// The RAM and picture after 1, 60 and `AFTER` frames from `from`.
fn trace(core: &mut NesCore, from: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    for frame in from..from + AFTER {
        core.run_frame(input(frame));
        let done = frame + 1 - from;
        if done == 1 || done == 60 || done == AFTER {
            out.push((game_ram(core), picture(core)));
        }
    }
    out
}

#[test]
#[ignore = "needs ZAXNES_PACK_ROMS"]
fn survey_of_the_packs() {
    let roms = roms();
    println!("{} packs with a ROM", roms.len());
    // ZAXNES_SURVEY_ONLY: a part of the patch names to keep; ZAXNES_SURVEY_DUMP: a folder for
    // the pictures (palette indices, 256 x 240) of both runs at each point.
    let only = std::env::var("ZAXNES_SURVEY_ONLY").unwrap_or_default();
    let dump = std::env::var_os("ZAXNES_SURVEY_DUMP").map(PathBuf::from);
    for (path, image, patch) in roms {
        if !patch.name.contains(only.as_str()) {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for save in SAVES {
            let mut line = format!("{:<52} @{save:>4}", patch.name);
            for on in [true, false] {
                let Some(mut source) = core(&image, &name, on) else {
                    line += " load-fail";
                    continue;
                };
                for frame in 0..save {
                    source.run_frame(input(frame));
                }
                let Ok((state, at_idle)) = source.save_state_at_idle(true) else {
                    line += " save-fail";
                    continue;
                };
                let reference = trace(&mut source, save);
                let Some(mut other) = core(&image, &name, !on) else {
                    continue;
                };
                if other.try_load_state(&state, 0).is_err() {
                    let _ = write!(
                        line,
                        " | {}: load refused",
                        if on { "on>off" } else { "off>on" }
                    );
                    continue;
                }
                let carried = trace(&mut other, save);
                if let Some(folder) = &dump {
                    let way = if on { "on-off" } else { "off-on" };
                    for (index, (r, c)) in reference.iter().zip(&carried).enumerate() {
                        let stem = format!("{}-{save}-{way}-{index}", patch.name.replace(' ', "_"));
                        let _ = std::fs::write(folder.join(format!("{stem}-ref.bin")), &r.1);
                        let _ = std::fs::write(folder.join(format!("{stem}-car.bin")), &c.1);
                    }
                }
                let ram: Vec<String> = reference
                    .iter()
                    .zip(&carried)
                    .map(|(r, c)| diff(&r.0, &c.0).to_string())
                    .collect();
                let pix: Vec<String> = reference
                    .iter()
                    .zip(&carried)
                    .map(|(r, c)| diff(&r.1, &c.1).to_string())
                    .collect();
                let _ = write!(
                    line,
                    " | {} idle={} ram {} pix {}",
                    if on { "on>off" } else { "off>on" },
                    u8::from(at_idle),
                    ram.join("/"),
                    pix.join("/")
                );
            }
            println!("{line}");
        }
    }
}

/// Where the CPU is at the end of frames of the games whose states are rarely made at the wait.
#[test]
#[ignore = "needs ZAXNES_PACK_ROMS"]
fn where_the_idle_loop_is_missed() {
    let only = std::env::var("ZAXNES_SURVEY_ONLY").unwrap_or_default();
    for (path, image, patch) in roms() {
        if !patch.name.contains(only.as_str()) {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(mut core) = core(&image, &name, true) else {
            continue;
        };
        for frame in 0..1800 {
            core.run_frame(input(frame));
        }
        let mut pcs = Vec::new();
        for frame in 1800..1806 {
            core.run_frame(input(frame));
            pcs.push(format!(
                "{:04X}@{}",
                core.cpu().pc,
                core.bus().ppu().scanline()
            ));
        }
        println!(
            "{:<44} idle {:04X?}  ends {}",
            patch.name,
            patch.idle_loop,
            pcs.join(" ")
        );
    }
}
