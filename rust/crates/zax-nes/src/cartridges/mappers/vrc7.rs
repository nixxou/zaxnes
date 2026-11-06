// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The decode of `Vrc7::write_register` — in particular that the two sound ports stay at $9010
// and $9030 whatever the register-select line is, and the order the four $E000-$F010 registers
// come in — is the arrangement of `Konami/VRC7.h:89-111` (`WriteRegister`), which transcribes the
// nesdev register table; see THIRD-PARTY-NOTICES.md.

use super::vrc_irq::VrcIrq;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 85, Konami VRC7 (nesdev, "VRC7"; the number is outside REQ-CORE-100's table,
/// which REQ-CORE-102 allows, and REQ-CORE-075 makes the chip's sound optional).
///
/// Registers are selected by A12-A15 and **one** further address line, with the three PRG
/// registers, the eight CHR registers and the four control registers laid out like this:
///
/// | address | bits | effect |
/// | --- | --- | --- |
/// | $8000 | `..PP PPPP` | 8 KiB PRG bank at $8000-$9FFF |
/// | $8010 / $8008 | `..PP PPPP` | 8 KiB PRG bank at $A000-$BFFF |
/// | $9000 | `..PP PPPP` | 8 KiB PRG bank at $C000-$DFFF |
/// | $9010 | `RRRR RRRR` | sound register select (always here — see below) |
/// | $9030 | `DDDD DDDD` | sound register data |
/// | $A000 … $D010 / $D008 | `BBBB BBBB` | CHR registers R0-R7, one 1 KiB window each |
/// | $E000 | `RS.. ..MM` | WRAM enable, silence, nametable arrangement |
/// | $E010 / $E008 | `LLLL LLLL` | IRQ latch |
/// | $F000 | `.... .MEA` | IRQ control |
/// | $F010 / $F008 | — | IRQ acknowledge |
///
/// $E000-$FFFF is not switchable: the last 8 KiB bank is wired there. $6000-$7FFF is 8 KiB of
/// PRG-RAM, which `R` enables.
///
/// **The two variants are one address line apart** (nesdev, "VRC7" and "NES 2.0 submappers"):
/// "VRC7b uses A3 to select registers ($x008), used in *Tiny Toon Adventures* (**Submapper 1**).
/// VRC7a uses A4 to select registers ($x010), used in *Lagrange Point* (**Submapper 2**)." A dump
/// that names neither — submapper 0, and every iNES 1.0 image — decodes **both**, which is the
/// same OR'd heuristic `Vrc2Vrc4` falls back on for mappers 21, 23 and 25 and the one Mesen
/// applies unconditionally.
///
/// **The two sound ports are at $9010 and $9030 on every submapper**, decoded before the
/// register-select rewrite. Both pages give them at those addresses with no submapper clause, and
/// Mesen exempts exactly `$9010`/`$9030` from its A4-to-A3 fold (`VRC7.h:91-94`). It costs
/// nothing to be wrong about a VRC7b here: the only commercial VRC7b, *Tiny Toon Adventures 2*,
/// has no sound code at all, because the board is missing the ceramic resonator the sound needs
/// — which is **Q-129**, since this build plays a submapper-1 dump's music rather than muting it.
///
/// The IRQ is Konami's, shared with the VRC4 and the VRC6 as [`VrcIrq`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc7 {
    banks: BankWindows,
    /// Whether A3 selects the register (the VRC7b of submapper 1, and submapper 0).
    ///
    /// Two `bool`s and not an enum, so that [`Vrc7::register_index`] is two tests and no branch
    /// on a variant — the shape `Vrc2Vrc4::index_lines` has for the same reason.
    select_a3: bool,
    /// Whether A4 selects the register (the VRC7a of submapper 2, and submapper 0).
    select_a4: bool,
    irq: VrcIrq,
    /// `$8000`, `$8010` and `$9000`, the three switchable 8 KiB banks: six bits each.
    ///
    /// Kept as registers and not only as mapped windows, so that [`Mapper::reset`] puts them
    /// back — `BankWindows` has no memory of what was written into it (AMD-061 §5 exempts
    /// mapper 28 and no other board; the sweep test caught exactly this on the VRC6).
    prg: [u8; 3],
    /// R0-R7, the eight CHR registers, exactly as written.
    chr: [u8; 8],
    /// `$E000`, exactly as written.
    control: u8,
    /// `$E000` bit 7, `R`: precomputed so that `prg_ram_enabled` — asked on the CPU's
    /// $6000-$7FFF decode — is one load and one test, like every other board's.
    prg_ram_on: bool,
    /// `$E000` bit 6, `S`: "Silence expansion sound if set". A field in this step, read by the
    /// sound in step 2 of `docs/progress/MF-5b.md`.
    silenced: bool,
}

impl Vrc7 {
    /// The three switchable PRG registers: `$8000`, `$8010` and `$9000`.
    const PRG_REGISTERS: usize = 3;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let submapper = if header.is_nes20 { header.submapper } else { 0 };
        let mut board = Self {
            banks: windows_for(memory),
            // Submapper 1 is the VRC7b (A3), submapper 2 the VRC7a (A4), and submapper 0 — with
            // every iNES 1.0 dump — is neither, so it answers to both lines.
            select_a3: submapper != 2,
            select_a4: submapper != 1,
            irq: VrcIrq::new(),
            // Nothing on the page gives a power-on value for any PRG register. 0, 1 and 2 are
            // the banks the windows would show with no board logic at all — $8000-$DFFF laid out
            // linearly — which is what Mesen comes up with too (`VRC7.h:34-43` maps only the
            // fixed window and leaves `BaseMapper`'s linear default under it). Every VRC7 game
            // writes all three in its reset handler, so this is not observable.
            prg: [0, 1, 2],
            chr: [0; 8],
            control: 0,
            prg_ram_on: false,
            silenced: false,
        };
        board.update_banks();
        board
    }

    /// The three switchable PRG windows, the wired last one, and the eight CHR windows.
    ///
    /// Written as two indexed loops over constant ranges, and not with `iter().enumerate()`,
    /// because the range is what lets LLVM drop the bounds check: the `enumerate` form made
    /// `BankWindows::map_chr_1k` emit a `panic_bounds_check` of its own and put a fifth one into
    /// `NesBus::write_internal`, which is MF-4d step 2's stop-the-lot test. This is the shape
    /// `vrc6.rs` already uses.
    fn update_banks(&mut self) {
        for slot in 0..Self::PRG_REGISTERS {
            self.banks.map_prg_8k(slot, i32::from(self.prg[slot]));
        }

        // "$E000-$FFFF" is absent from the page's PRG table: the last bank is wired there.
        self.banks.map_prg_8k(3, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }

    /// Bit 0 of the register index: the state of whichever address line this board's variant
    /// uses to select the second register of a $1000 page.
    ///
    /// A dump that names no submapper answers to **both** lines, so `$x008` and `$x010` are the
    /// same register there — the OR'd reading. A VRC7a ignores A3 and a VRC7b ignores A4, so on
    /// either of them the other line's address is a mirror of the page's first register.
    const fn register_index(&self, address: u16) -> usize {
        let a3 = self.select_a3 && address & 0x08 != 0;
        let a4 = self.select_a4 && address & 0x10 != 0;
        (a3 || a4) as usize
    }

    /// "Mirroring (0: vertical; 1: horizontal; 2/3: one-screen)" — `$E000`'s two low bits. The
    /// page's two one-screen rows are the two CIRAM pages, which Mesen names
    /// `ScreenAOnly`/`ScreenBOnly` (`VRC7.h:70-77`).
    const fn arrangement(control: u8) -> NametableMirroring {
        match control & 0x03 {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }

    /// Applies `$E000` in full: the arrangement, the WRAM enable and the silence bit.
    fn apply_control(&mut self, memory: &mut CartridgeMemory) {
        self.prg_ram_on = self.control & 0x80 != 0;
        self.silenced = self.control & 0x40 != 0;
        memory.set_mirroring(Self::arrangement(self.control));
    }
}

impl Mapper for Vrc7 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.control);
        self.irq.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.control = reader.read_u8()?;
        self.irq.load_state(reader)?;

        // The two derived bits come back from `$E000` (REQ-CORE-091). The **mirroring** is not
        // touched: `Cartridge::load_state` has just restored it from the state, and recomputing
        // it here would overwrite the saved one with whatever `$E000` implies — the same value in
        // practice, but the state is authoritative.
        self.prg_ram_on = self.control & 0x80 != 0;
        self.silenced = self.control & 0x40 != 0;
        self.update_banks();
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        85
    }

    /// Nothing on the page gives the power-on value of `$E000`, so it is 0: vertical mirroring,
    /// the sound audible and the PRG-RAM protected. Mesen powers up the same way, and **it is
    /// the one board in this build whose power-on state overrides the header's arrangement**,
    /// because a VRC7 drives CIRAM A10 from `$E000` alone and has no pass-through mode (the
    /// VRC6's mode 0 is not an analogue: that one lets the header's arrangement stand).
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.irq.reset();
        self.prg = [0, 1, 2];
        self.chr = [0; 8];
        self.control = 0;
        self.update_banks();
        self.apply_control(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.irq.asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.prg_ram_on
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        // The two sound ports first, and only then the register-select rewrite: see the type's
        // own note for why they are at $9010 and $9030 on every submapper. A5 is what separates
        // them, so the mask has to be $F030 and not $F010.
        if address & 0xF030 == 0x9010 || address & 0xF030 == 0x9030 {
            // The sound is step 2 of `docs/progress/MF-5b.md`.
            return;
        }

        let index = self.register_index(address);
        match (address & 0xF000, index) {
            // "Select 8 KB PRG ROM bank", six bits, three windows.
            (0x8000, _) => {
                self.prg[index] = value & 0x3F;
                self.banks.map_prg_8k(index, i32::from(self.prg[index]));
            }
            (0x9000, 0) => {
                self.prg[2] = value & 0x3F;
                self.banks.map_prg_8k(2, i32::from(self.prg[2]));
            }
            // R0-R7, two per page from $A000 to $D010. The `& 7` is what makes the index
            // provably in range: the arm already restricts A12-A15 to $A-$D and `index` to 0 or
            // 1, so the mask changes no value, and without it LLVM puts a `panic_bounds_check`
            // into `NesBus::write_internal` (MF-4d step 2's stop-the-lot test).
            (0xA000..=0xD000, _) => {
                let slot = ((usize::from(address >> 12) - 0xA) * 2 + index) & 7;
                self.chr[slot] = value;
                self.banks.map_chr_1k(slot, i32::from(value));
            }
            (0xE000, 0) => {
                self.control = value;
                self.apply_control(memory);
            }
            (0xE000, _) => self.irq.write_latch(value),
            (0xF000, 0) => self.irq.write_control(value),
            (0xF000, _) => self.irq.acknowledge(),
            // What is left is `$9008` on a VRC7b, which the page does not name: it prints the
            // two sound ports at $9010 and $9030 only, so A3 set in page $9000 is nothing at
            // all. Mesen ignores it too (`VRC7.h:99`, whose `0xF038` switch has no `$9008`
            // label), and no commercial VRC7b writes there.
            _ => {}
        }
    }

    /// **Out of line, measured on the VRC6 and the same shape here.** This is one arm of
    /// `Board::on_cpu_cycle`, whose thirty-two arms share one prologue, so an arm that holds a
    /// value across a call makes every board save a register on every CPU cycle — MF-3b rule 3,
    /// and `vrc6.rs`'s own note is the measurement.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.irq.step_cpu_cycle();
    }
}
