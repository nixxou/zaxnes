//! Lot FIDPLUS-2: two `$2007` writes on consecutive CPU cycles (`INC $2007`), measured on the
//! transistor netlist (`docs/review/FIDELITY-LEADS.md` A5).

mod common;

use common::{machine_with, minimal_rom};

/// Waits for two vblanks, `$2000 = 0`, fills `$2000-$2007` with 0..7, points `v` at `$2002`,
/// `INC $2007`, then loops. The bench program `mkrmw.py` without its read-back.
const PROGRAM: [u8; 53] = [
    0x78, 0xD8, // SEI, CLD
    0x2C, 0x02, 0x20, 0x10, 0xFB, // BIT $2002, BPL
    0x2C, 0x02, 0x20, 0x10, 0xFB, // BIT $2002, BPL
    0xA9, 0x00, 0x8D, 0x00, 0x20, // $2000 = 0
    0xA9, 0x20, 0x8D, 0x06, 0x20, 0xA9, 0x00, 0x8D, 0x06, 0x20, // $2006 = $2000
    0xA2, 0x00, // LDX #0
    0x8E, 0x07, 0x20, 0xE8, 0xE0, 0x08, 0xD0, 0xF8, // STX $2007, INX, CPX #8, BNE
    0xA9, 0x20, 0x8D, 0x06, 0x20, 0xA9, 0x02, 0x8D, 0x06, 0x20, // $2006 = $2002
    0xEE, 0x07, 0x20, // INC $2007
    0x4C, 0x32, 0x80, // JMP *
];

#[test]
fn fidplus2_inc_2007_stores_the_final_value_at_both_addresses() {
    inc_2007(false);
}

/// Fidélité+ part A step 2: the same result from the data latch and the delayed store, with no
/// rule written for it (`ppu/edges.rs`).
#[test]
fn fidplus_a2_inc_2007_follows_from_the_delayed_store() {
    inc_2007(true);
}

fn inc_2007(fidelity_plus: bool) {
    // The read at $2002 returns the buffer and moves v to $2003; the dummy write of the old value
    // and the write of the new one follow on the next two cycles. The netlist stores the second
    // value at both addresses in all four alignments.
    for alignment in 0..4 {
        let image = minimal_rom(0x00, Some(&PROGRAM), None);
        let mut machine = machine_with(&image, "rmw.nes");
        machine.set_fidelity_plus(fidelity_plus);
        machine.set_cpu_ppu_alignment(alignment);
        machine.reset();
        for _ in 0..4 {
            machine.run_frame(0);
        }
        let vram = machine.ppu().vram();
        assert_eq!([0, 1, 2], vram[0..3], "alignment {alignment}: the fill");
        assert_eq!(
            [1, 1],
            vram[3..5],
            "alignment {alignment}: {:02X?}",
            &vram[..8]
        );
        assert_eq!([5, 6, 7], vram[5..8], "alignment {alignment}");
    }
}
