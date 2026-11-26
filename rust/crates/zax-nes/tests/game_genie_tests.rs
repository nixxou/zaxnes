//! Lot CHEATS: the Game Genie's patches on the CPU's reads of `$8000-$FFFF`. A tiny NROM program
//! stores the operand of an `LDA #` in RAM; a patch on that operand changes what is stored, on the
//! standard frame, the Fidelite+ frame, and in `read_memory`.
#![allow(clippy::unwrap_used)]

use zax_core_api::CpuReadPatch;
use zax_nes::NesCore;

/// `$E000: LDA #$11; STA $10; JMP $E004`, vectors at `$E000`.
fn machine() -> NesCore {
    let prg = 32 * 1024;
    let mut image = vec![0u8; 16 + prg + 8 * 1024];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 2;
    image[5] = 1;
    let program = 16 + prg - 8 * 1024;
    image[program..program + 7].copy_from_slice(&[0xA9, 0x11, 0x85, 0x10, 0x4C, 0x04, 0xE0]);
    for vector in 0..3 {
        let at = 16 + prg - 6 + vector * 2;
        image[at..at + 2].copy_from_slice(&[0x00, 0xE0]);
    }
    let mut core = NesCore::new();
    assert!(core.load_rom(&image, "genie.nes").success);
    core
}

fn stored(patches: &[CpuReadPatch], fidelity_plus: bool) -> (u8, u8) {
    let mut core = machine();
    core.set_fidelity_plus(fidelity_plus);
    core.set_cpu_read_patches(patches);
    for _ in 0..3 {
        core.run_frame(0);
    }
    (core.read_memory(0x0010), core.read_memory(0xE001))
}

const fn patch(compare: Option<u8>) -> CpuReadPatch {
    CpuReadPatch { address: 0xE001, value: 0x22, compare }
}

#[test]
fn a_patch_changes_what_the_cpu_reads() {
    for fidelity_plus in [false, true] {
        assert_eq!(stored(&[], fidelity_plus), (0x11, 0x11));
        assert_eq!(stored(&[patch(None)], fidelity_plus), (0x22, 0x22));
        // eight letters: only when the cartridge holds the compare value
        assert_eq!(stored(&[patch(Some(0x11))], fidelity_plus), (0x22, 0x22));
        assert_eq!(stored(&[patch(Some(0x99))], fidelity_plus), (0x11, 0x11));
    }
}

#[test]
fn removing_the_patches_gives_the_cartridge_back() {
    let mut core = machine();
    core.set_cpu_read_patches(&[patch(None)]);
    core.run_frame(0);
    assert_eq!(core.read_memory(0x0010), 0x22);
    core.set_cpu_read_patches(&[]);
    core.write_memory(0x0010, 0);
    core.reset();
    core.run_frame(0);
    assert_eq!(core.read_memory(0x0010), 0x11);
}
