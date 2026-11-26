#![cfg(feature = "board-vs-system")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! A real VS. System dump through the three frame loops (fix/vs-paths): the shown palette must be
//! the same (the RP2C04 remap applied), and the slice loop must give the standard frame's
//! pictures and cycles exactly. `ZAXNES_VS_ROM=<file.nes> cargo test --test vs_rom_loop_tests --
//! --ignored`.

use zax_nes::NesCore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameLoop {
    Standard,
    FidelityPlus,
    Slices,
}

type Run = (Vec<(i64, Vec<u8>)>, Vec<[u8; 32]>);

fn run(image: &[u8], frame_loop: FrameLoop) -> Run {
    let mut core = NesCore::new();
    assert!(core.load_rom(image, "vs.nes").success);
    assert!(core.vs_system().is_some(), "not seen as a VS. System image");
    core.set_fidelity_plus(frame_loop == FrameLoop::FidelityPlus);
    let (mut frames, mut palettes) = (Vec::new(), Vec::new());
    let count: u16 = std::env::var("ZAXNES_VS_FRAMES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(600);
    for frame in 0..count {
        // coin and start now and then, so the game leaves its first screen
        let input = if frame % 120 < 10 { 0x08 } else { 0 };
        let cycles = match frame_loop {
            FrameLoop::Standard | FrameLoop::FidelityPlus => core.run_frame(input),
            FrameLoop::Slices => {
                let mut until = core.cycle_count();
                loop {
                    until += 1500;
                    if let Some(cycles) = core.run_frame_slice(input, until) {
                        break cycles;
                    }
                }
            }
        };
        frames.push((cycles, core.pixel_frame_with_mask().to_vec()));
        palettes.push(core.ppu().snapshot_palette());
    }
    (frames, palettes)
}

#[test]
#[ignore = "needs ZAXNES_VS_ROM, a VS. System dump"]
fn a_vs_game_runs_alike_on_every_frame_loop() {
    let path = std::env::var_os("ZAXNES_VS_ROM").expect("set ZAXNES_VS_ROM");
    let image = std::fs::read(path).unwrap();
    let standard = run(&image, FrameLoop::Standard);
    let slices = run(&image, FrameLoop::Slices);
    let fidelity = run(&image, FrameLoop::FidelityPlus);
    let differing = |other: &Run| {
        standard
            .1
            .iter()
            .zip(&other.1)
            .filter(|(a, b)| a != b)
            .count()
    };
    let distinct = |run: &Run| {
        let last = &run.0.last().unwrap().1;
        let mut seen = std::collections::BTreeSet::new();
        seen.extend(last.iter().copied());
        seen.len()
    };
    eprintln!(
        "values in the last picture: standard {}, slices {}",
        distinct(&standard),
        distinct(&slices)
    );
    let pictures = standard
        .0
        .iter()
        .zip(&slices.0)
        .filter(|(a, b)| a != b)
        .count();
    eprintln!(
        "palettes differing from the standard frame: slices {}, Fidelite+ {}; slice pictures differing: {pictures}",
        differing(&slices),
        differing(&fidelity)
    );
    assert_eq!(differing(&slices), 0, "slices: palette");
    assert_eq!(pictures, 0, "slices: pictures or cycles");
    assert_eq!(differing(&fidelity), 0, "Fidelite+: palette");
}
