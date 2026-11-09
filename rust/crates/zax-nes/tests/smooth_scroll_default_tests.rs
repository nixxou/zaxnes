//! Lot KIRBY-MERGE step 3: the owner's rule made executable for the experimental smooth
//! scrolling — **a game with no pack is not touched unless the option is turned on**.
//!
//! The branch shipped `emu.smooth_scroll_ahead` on by default; the merge turned it off
//! (`docs/progress/KIRBY-MERGE.md` §2.4, (S4)) because on, with the default run-ahead of 1 and
//! any display above 61 Hz, it shifts the 2D playfield of every game and makes the PPU record a
//! scroll per visible line. These tests hold both halves of that: nothing happens off, and the
//! feature does work on, so the default is the only thing standing between the two.
//!
//! They need a real scrolling game. `ZAXNES_ROMS` is the demo root; without it they pass
//! silently, as `ppu_golden_tests` does.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)] // test code

use zax_nes::NesCore;

/// Super Mario Bros., which scrolls as soon as the attract demo starts, and which has no hack
/// pack aid in play here: the plain game.
fn scrolling_rom() -> Option<(Vec<u8>, String)> {
    let folder = std::env::var_os("ZAXNES_ROMS")?;
    let name = "Super Mario Bros. (World).nes";
    let image = std::fs::read(std::path::Path::new(&folder).join(name)).ok()?;
    Some((image, name.to_owned()))
}

fn core_of(image: &[u8], name: &str) -> NesCore {
    let mut core = NesCore::new();
    assert!(core.load_rom(image, name).success, "{name} was refused");
    core
}

/// Frames enough for the attract demo to be running and scrolling.
const FRAMES: usize = 400;

/// Off — the default — nothing of the smooth scrolling exists: no shift, no motion, and the PPU
/// keeps no per-line scroll, so `end_capture_line` does the work it did before the merge.
#[test]
fn off_by_default_a_game_with_no_pack_is_left_alone() {
    let Some((image, name)) = scrolling_rom() else {
        return;
    };
    let mut core = core_of(&image, &name);
    for _ in 0..FRAMES {
        core.run_frame(0);
        assert_eq!(
            None,
            core.picture_shift(),
            "the playfield must not be shifted"
        );
        assert_eq!(None, core.picture_motion(), "no motion is published");
    }
    assert_eq!(
        None,
        core.bus_mut().ppu_mut().playfield_scroll(),
        "no per-line scroll is kept while nothing asked for it"
    );
}

/// On — and only then — the core finds the band that scrolls and publishes a shift for it. This
/// is what says the option is the gate and not a dead switch.
#[test]
fn turned_on_the_same_game_gets_a_shifted_playfield() {
    let Some((image, name)) = scrolling_rom() else {
        return;
    };
    let mut core = core_of(&image, &name);
    core.set_smooth_scroll_ahead(true);
    let mut shifted = 0;
    let mut band = None;
    for _ in 0..FRAMES {
        core.run_frame(0);
        if core.picture_shift().is_some() {
            shifted += 1;
        }
        if band.is_none() {
            band = core.bus_mut().ppu_mut().playfield_scroll();
        }
    }
    assert!(
        shifted > FRAMES / 2,
        "the scrolling band is found on most frames, not {shifted} of {FRAMES}"
    );
    let (_, from, to) = band.expect("a band of lines that scrolls");
    assert!(
        to - from >= 16,
        "the band runs over at least 16 lines, not {from}..{to}"
    );
}

/// And turning it back off puts the game back where it was, in the same run: the switch is not
/// one-way, and a frame run after it is cleared publishes nothing.
#[test]
fn turning_it_off_again_stops_everything() {
    let Some((image, name)) = scrolling_rom() else {
        return;
    };
    let mut core = core_of(&image, &name);
    core.set_smooth_scroll_ahead(true);
    for _ in 0..FRAMES {
        core.run_frame(0);
    }
    assert!(core.picture_shift().is_some(), "the feature was on");
    core.set_smooth_scroll_ahead(false);
    for _ in 0..4 {
        core.run_frame(0);
        assert_eq!(None, core.picture_shift());
        assert_eq!(None, core.picture_motion());
    }
    assert_eq!(None, core.bus_mut().ppu_mut().playfield_scroll());
}

/// The run-ahead bound the branch raised, at the new edge: AMD-041 says 3, this build says 10
/// (Rust-only, owner request). REQ-LOOP-065's invariant has to hold there too — the **real**
/// frames of a run-ahead sequence are the frames of a plain run, so a game computes the same
/// thing however far ahead the host looks.
#[test]
fn the_run_ahead_bound_is_ten_and_the_real_frames_still_match_a_plain_run() {
    assert_eq!(10, NesCore::MAX_RUN_AHEAD);
    let Some((image, name)) = scrolling_rom() else {
        return;
    };
    let mut plain = core_of(&image, &name);
    let mut ahead = core_of(&image, &name);
    for _ in 0..120 {
        plain.run_frame(0);
        ahead.run_frame_ahead(0, NesCore::MAX_RUN_AHEAD);
        assert!(ahead.restore_real_frame(), "the real frame goes back");
    }
    let plain_ram: Vec<u8> = (0..0x0800).map(|a| plain.read_memory(a)).collect();
    let ahead_ram: Vec<u8> = (0..0x0800).map(|a| ahead.read_memory(a)).collect();
    assert_eq!(
        plain_ram, ahead_ram,
        "run-ahead 10 left the real frame's work RAM where a plain run leaves it"
    );
}
