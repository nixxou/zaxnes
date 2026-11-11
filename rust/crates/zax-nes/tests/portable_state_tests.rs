//! Portable savestates (owner request): a state made with a game's built-in hack pack loads
//! without it, and the other way round, and the game goes on exactly as if it had always run
//! the way it runs now.
//!
//! The measure is the game's own memory. A hack pack leaves the game's logic as it is (its code
//! runs in the extension's memory, the game's RAM is the game's), so from the same inputs a game
//! played with the hack and one played without it hold the same 2 KB of CPU RAM, frame for frame.
//! A state carried across must keep that: loaded in the other mode at frame `SAVE`, the game
//! reaches frame `END` with the RAM of a run made in that mode from the start, and draws the
//! same picture inside the 256 pixels both modes show.
//!
//! First game: `FlappyZaxNes`, the demo game whose sources the project holds. Its ROM is the
//! demo one of zax-app (`crates/zax-app/demo/FlappyZaxNes.nes`, kept out of git); without it the
//! tests return early.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use zax_nes::NesCore;

/// The frame the state is made at, and the frame both runs are compared at.
const SAVE: usize = 420;
const END: usize = 720;

/// Standard pad bits of the input word: A and Start.
const A: u16 = 0x01;
const START: u16 = 0x08;

fn rom() -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zax-app/demo/FlappyZaxNes.nes");
    std::fs::read(path).ok()
}

/// The pad on `frame`: Start once the title shows, then a flap every 18 frames, held 3 frames.
fn input(frame: usize) -> u16 {
    match frame {
        90..=94 => START,
        f if f > 120 && f % 18 < 3 => A,
        _ => 0,
    }
}

/// A core with the built-in patches `on` or off, the widescreen margin a 16:9 window offers.
fn core(image: &[u8], on: bool) -> NesCore {
    let mut core = NesCore::new();
    core.set_builtin_patches(on);
    let loaded = core.load_rom(image, "FlappyZaxNes.nes");
    assert!(loaded.success, "the demo ROM loads");
    core.set_widescreen_margin(if on { 64 } else { 0 });
    core
}

fn run(core: &mut NesCore, frames: std::ops::Range<usize>) {
    for frame in frames {
        core.run_frame(input(frame));
    }
}

/// The 256 x 240 picture of the frame just run, as palette indices.
fn picture(core: &NesCore) -> Vec<u8> {
    let mut out = Vec::new();
    core.pixel_frame_indices_into(&mut out);
    out
}

/// The run of one mode from power-on to `END`, its RAM and picture there, and its state at
/// `SAVE` taken at the game's wait for the next frame.
struct Reference {
    ram: Vec<u8>,
    picture: Vec<u8>,
    state: Vec<u8>,
    at_idle: bool,
}

fn reference(image: &[u8], on: bool) -> Reference {
    let mut core = core(image, on);
    run(&mut core, 0..SAVE);
    let (state, at_idle) = core.save_state_at_idle(true).expect("a state");
    run(&mut core, SAVE..END);
    Reference {
        ram: game_ram(&core),
        picture: picture(&core),
        state,
        at_idle,
    }
}

/// Loads `state` into a core of the mode `on` and plays it on to `END`.
fn carried(image: &[u8], state: &[u8], on: bool) -> (Vec<u8>, Vec<u8>) {
    let mut core = core(image, on);
    core.try_load_state(state, 0).expect("the state loads");
    run(&mut core, SAVE..END);
    (game_ram(&core), picture(&core))
}

/// The page the game builds its sprite list in before the DMA ($4014): `FlappyZaxNes` uses
/// page 2. The hack adds its sprites there (the clouds of the margins), so the list is the
/// hack's as much as the game's; what the player sees of it is the picture, compared apart.
const SPRITE_PAGE: usize = 0x0200;

/// The game's memory of `core`: its RAM, with the part of the stack below its top (left over
/// from calls returned from, read by nothing) and the sprite list set to zero.
fn game_ram(core: &NesCore) -> Vec<u8> {
    let mut ram = core.bus().ram().to_vec();
    let top = 0x0100 + usize::from(core.cpu().s);
    ram[0x0100..=top].fill(0);
    ram[SPRITE_PAGE..SPRITE_PAGE + 0x100].fill(0);
    ram
}

/// The addresses where two RAM images differ, for the failure message.
fn differences(left: &[u8], right: &[u8]) -> Vec<String> {
    left.iter()
        .zip(right)
        .enumerate()
        .filter(|(_, (l, r))| l != r)
        .map(|(address, (l, r))| format!("${address:04X}: {l:02X} vs {r:02X}"))
        .take(24)
        .collect()
}

#[test]
fn the_hack_leaves_the_game_memory_as_it_is() {
    let Some(image) = rom() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let with = reference(&image, true);
    let without = reference(&image, false);
    assert!(
        with.at_idle,
        "the state with the hack is taken at the game's wait"
    );
    assert_eq!(
        differences(&with.ram, &without.ram),
        Vec::<String>::new(),
        "the game's RAM with and without the hack"
    );
}

#[test]
fn a_state_of_the_hack_goes_on_without_it() {
    let Some(image) = rom() else {
        return;
    };
    let with = reference(&image, true);
    let without = reference(&image, false);
    let (ram, picture) = carried(&image, &with.state, false);
    assert_eq!(
        differences(&ram, &without.ram),
        Vec::<String>::new(),
        "RAM at frame {END} of the state made with the hack, played on without it"
    );
    assert!(picture == without.picture, "the picture without the hack");
}

#[test]
fn a_state_without_the_hack_goes_on_with_it() {
    let Some(image) = rom() else {
        return;
    };
    let with = reference(&image, true);
    let without = reference(&image, false);
    let (ram, picture) = carried(&image, &without.state, true);
    assert_eq!(
        differences(&ram, &with.ram),
        Vec::<String>::new(),
        "RAM at frame {END} of the state made without the hack, played on with it"
    );
    assert!(picture == with.picture, "the picture with the hack");
}

/// The pack says its states carry (its states section), and the section leaves the hack's
/// version as it was: the states made with the pack before it keep loading.
#[test]
fn the_pack_says_its_states_carry_and_keeps_its_version() {
    let patch = zax_nes::builtin_patches::PATCHES
        .iter()
        .find(|patch| patch.name == zax_nes::builtin_patches::FLAPPYZAXNES)
        .expect("the FlappyZaxNes patch");
    assert!(patch.states().portable);
    assert_eq!(patch.states().redraw, []);
    assert_eq!(
        patch.version(),
        0x540D_CBDF,
        "the version of the pack before its section"
    );
}
