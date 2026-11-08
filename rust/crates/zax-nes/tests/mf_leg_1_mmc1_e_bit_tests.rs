//! **The extra check lot MF-LEG-1 step 1 owes the owner** (report entry MMC1-1), and the same
//! evidence under lot **MF-LEG-1B**, which carries the change further.
//!
//! Taking the SNROM `E` bit away takes it away from every iNES 1.0 mapper-1 dump with CHR-RAM —
//! which is the whole popular set, *Zelda II*, *Metroid*, *Mega Man 2*, *Final Fantasy* among
//! them. Removing the bit can only ever *re-enable* PRG-RAM that was switched off, so a game that
//! works today keeps working; what the owner asked be proved is the stronger statement, that the
//! change moves **nothing** for those games — i.e. that under the **old** rule the RAM would
//! never have been switched off during a run in the first place.
//!
//! MF-LEG-1 step 1 narrowed `Mmc1::is_snrom` to boards an NES 2.0 header declares; MF-LEG-1B's
//! answer to Q-176 drops the bit outright in `default`. The check is the same either way and
//! holds for both, because what it establishes is about the **old** rule: the games never reach
//! the bit at all.
//!
//! The question reduces to one bit. On a board the old test fired on, the `E` bit *is* bit 4 of
//! the committed CHR bank 0 register ([`Mmc1::chr_bank0_has_the_e_bit`]), so "the old rule would
//! have disabled the RAM at some point" is exactly "that bit was set at some point". The register
//! can only change on a CPU write, so sampling it after **every CPU instruction** is exact: there
//! is no sub-instruction window in which a value could be committed and overwritten unseen.
//!
//! That is why the run below is driven instruction by instruction rather than by
//! `NesCore::run_frame`. The loop is `run_frame`'s own — `begin_frame`, `clear_frame_boundary`,
//! `defer_ppu`, step until the PPU says vblank began, `end_deferred_ppu` — minus the margin
//! decoder, the overclock idle loop and the video callback, none of which any of these five games
//! has with no 3D pack loaded. So the emulation is the ordinary one.
//!
//! Needs the owner's own dumps: set `ZAXNES_ROMS` to the folder that holds them. Without it the
//! test returns early and passes vacuously, which is why the lot's final check runs it with the
//! root set.

// The ROM run is long and its bookkeeping is plain; the test helpers panic on a failed setup.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use zax_nes::cartridges::mappers::Mapper;
use zax_nes::cartridges::Board;
use zax_nes::NesCore;

/// The five dumps the old heuristic read as SNROMs and that the owner names: *Zelda II* in both
/// regions, *Metroid*, *Mega Man 2* and *Final Fantasy*.
const ROMS: [&str; 5] = [
    "Zelda II - The Adventure of Link (USA).nes",
    "Zelda II - The Adventure of Link (Europe).nes",
    "Metroid (USA).nes",
    "Mega Man 2 (USA).nes",
    "Final Fantasy (USA).nes",
];

/// Frames each ROM is played for, the number the lot instruction gives.
const FRAMES: usize = 3600;

/// One frame of `NesCore::run_frame`, stepped instruction by instruction, calling `sample` after
/// each one. Returns the number of instructions the frame took.
fn run_frame_sampled(core: &mut NesCore, mut sample: impl FnMut(&NesCore)) -> u64 {
    core.bus_mut().input_mut().begin_frame(0);
    core.bus_mut().ppu_mut().clear_frame_boundary();
    core.bus_mut().defer_ppu();
    let mut instructions = 0;
    loop {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
        instructions += 1;
        sample(core);
        if core.bus().ppu().vblank_just_started() {
            break;
        }
    }

    core.bus_mut().end_deferred_ppu();
    instructions
}

/// The MMC1 of a loaded mapper-1 game.
fn mmc1(core: &NesCore) -> &zax_nes::cartridges::mappers::Mmc1 {
    match core.cartridge().expect("a cartridge").board() {
        Board::Mmc1(mmc1) => mmc1,
        other => panic!("not an MMC1: {:?}", other.mapper_number()),
    }
}

/// REQ-CORE-100, report entry MMC1-1: none of the five games ever commits the SNROM `E` bit, so
/// neither MF-LEG-1 step 1's narrowed board test nor MF-LEG-1B's dropping the bit changes
/// anything for any of them.
#[test]
#[ignore = "needs the user's ROMs: set ZAXNES_ROMS to the folder holding the five mapper-1 dumps"]
fn mf_leg_1_the_five_mapper_1_games_never_commit_the_snrom_e_bit() {
    let Ok(folder) = std::env::var("ZAXNES_ROMS") else {
        eprintln!("skipped: ZAXNES_ROMS is not set");
        return;
    };

    let mut checked = 0;
    let mut affected = Vec::new();
    for rom in ROMS {
        let path = std::path::Path::new(&folder).join(rom);
        let Ok(image) = std::fs::read(&path) else {
            eprintln!("skipped: {rom} not found in {folder}");
            continue;
        };

        let mut core = NesCore::new();
        // The plain emulation: no built-in patch rewriting the image, no advanced patch.
        core.set_builtin_patches(false);
        core.set_advanced_patches(false);
        let load = core.load_rom(&image, rom);
        assert!(load.success, "{rom}: {:?}", load.error);
        assert_eq!(
            1,
            core.cartridge().unwrap().mapper_number(),
            "{rom} is not mapper 1"
        );

        let mut instructions = 0u64;
        let mut first_frame_with_the_bit = None;
        for frame in 0..FRAMES {
            instructions += run_frame_sampled(&mut core, |core| {
                if first_frame_with_the_bit.is_none() && mmc1(core).chr_bank0_has_the_e_bit() {
                    first_frame_with_the_bit = Some(frame);
                }
            });
        }

        eprintln!("{rom}: {FRAMES} frames, {instructions} instructions");
        if let Some(frame) = first_frame_with_the_bit {
            affected.push(format!("{rom} commits the `E` bit at frame {frame}"));
        }

        checked += 1;
    }

    assert!(checked > 0, "none of the five ROMs was found in {folder}");
    assert!(
        affected.is_empty(),
        "the old rule would have disabled the PRG-RAM of {checked} games checked:\n{}",
        affected.join("\n")
    );
}
