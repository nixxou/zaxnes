//! Lot NESHAWK-HACKS: the built-in patches, the hack packs and the widescreen extension on the
//! NesHawk engine. The ROM tests need `ZAXNES_ROMS` (a directory of `.nes` files, such as
//! `G:\3DSen\demo\roms`) and are `#[ignore]`d like every ROM test of the crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::path::PathBuf;

use zax_core_api::Engine;
use zax_neshawk::NesHawkEngine;

fn roms() -> Vec<(String, Vec<u8>)> {
    let Some(dir) = std::env::var_os("ZAXNES_ROMS").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("ZAXNES_ROMS").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("nes") {
            continue;
        }
        if let Ok(image) = std::fs::read(&path) {
            out.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                image,
            ));
        }
    }
    out.sort();
    out
}

/// A scripted few presses of Start and A, the same for every run.
fn input(frame: u32) -> u16 {
    match frame % 120 {
        30..=33 => 0x08,
        90..=93 => 0x01,
        _ => 0,
    }
}

fn run(engine: &mut NesHawkEngine, frames: u32) -> Vec<u8> {
    let mut trace = Vec::new();
    for frame in 0..frames {
        engine.run_frame(input(frame));
        for address in 0..0x800u16 {
            trace.push(engine.read_memory(address));
        }
    }
    trace
}

/// With the patches off (the default) and no margin, no extension is attached at all: the
/// machine is the C#'s.
#[test]
#[ignore = "needs ZAXNES_ROMS"]
fn by_default_nothing_is_attached() {
    for (name, image) in roms() {
        let mut engine = NesHawkEngine::new();
        if !engine.load_rom(&image, &name).success {
            continue;
        }
        assert!(!engine.has_wide_extension(), "{name}");
        assert_eq!(engine.builtin_patch(), None, "{name}");
        run(&mut engine, 30);
        assert!(!engine.has_wide_extension(), "{name}");
    }
}

/// A margin offered to a game without a patch attaches a locked extension, which changes
/// nothing: the same work RAM frame after frame as without it, and its savestates restore.
#[test]
#[ignore = "needs ZAXNES_ROMS"]
fn a_locked_extension_changes_nothing() {
    for (name, image) in roms() {
        let mut plain = NesHawkEngine::new();
        if !plain.load_rom(&image, &name).success {
            continue;
        }
        let mut wide = NesHawkEngine::new();
        assert!(wide.load_rom(&image, &name).success);
        wide.set_widescreen_margin(64);
        wide.power();
        assert!(wide.has_wide_extension(), "{name}");
        if wide
            .nes()
            .is_some_and(|nes| nes.b.wide.as_ref().unwrap().ext.unlocked())
        {
            continue; // a ROM made for the extension unlocks it
        }
        assert_eq!(run(&mut plain, 600), run(&mut wide, 600), "{name}");
        let mut state = Vec::new();
        wide.save_state_to(true, &mut state).expect("state");
        let more = run(&mut wide, 60);
        wide.try_load_state(&state, 0).expect("restore");
        assert_eq!(run(&mut wide, 60), more, "{name}: restored state");
    }
}

/// With the patches on, a game a built-in patch is made for runs it: the patch is applied, the
/// extension booted (zone or window) or unlocked by the ROM within the first seconds, and a
/// savestate taken in play restores to the same frames.
#[test]
#[ignore = "needs ZAXNES_ROMS"]
fn the_built_in_patches_run_on_neshawk() {
    let mut patched = 0;
    for (name, image) in roms() {
        let mut engine = NesHawkEngine::new();
        engine.set_builtin_patches(true);
        engine.set_advanced_patches(true);
        engine.set_widescreen_margin(128);
        if !engine.load_rom(&image, &name).success {
            continue;
        }
        let Some(patch) = engine.builtin_patch() else {
            continue;
        };
        patched += 1;
        run(&mut engine, 900);
        let unlocked = engine
            .nes()
            .and_then(|nes| nes.b.wide.as_ref())
            .is_some_and(|wide| wide.ext.unlocked());
        assert!(unlocked, "{name} ({patch}): the extension never unlocked");
        let mut state = Vec::new();
        engine.save_state_to(true, &mut state).expect("state");
        let more = run(&mut engine, 120);
        engine.try_load_state(&state, 0).expect("restore");
        assert_eq!(run(&mut engine, 120), more, "{name}: restored state");
        println!(
            "{name}: {patch}, components {:?}",
            engine.applied_components()
        );
    }
    assert!(
        patched > 0 || roms().is_empty(),
        "no ROM of ZAXNES_ROMS has a built-in patch"
    );
}
