//! Lot MAPPERS-P: MMC1 multicarts whose outer register sits beside the chip (`docs/progress/MAPPERS-P.md`).
//!
//! Each board holds an [`Mmc1`] and, after every change, rebuilds the chip's windows
//! ([`Mmc1::refresh_banks`]) and narrows them to its outer bank with
//! [`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`], the way `FaridSlrom`
//! (mapper 323) does: the MMC1 itself stays on lot CORE-2's measured path, untouched. Two boards
//! (297, 550) also have a mode that bypasses the chip and maps fixed-size banks directly.
//!
//! Behaviour read from puNES 0.111 (`mapper_297.c`, `mapper_374.c`, `mapper_404.c`,
//! `mapper_550.c`; read, not copied). puNES's `prg_swap`/`chr_swap` hooks see the MMC1's 16 KiB
//! and 4 KiB bank numbers; the windows here are 8 KiB and 1 KiB, so each outer value is shifted
//! one and two bits further.
use super::{Mapper, Mmc1};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mmc1OuterKind {
    /// 297, `mapper_297.c` (TXC 01-22110-000): `$4100` bit 0 picks the MMC1 (outer bank 1) over
    /// a latch mode with UNROM-like PRG and CNROM-like CHR.
    M297,
    /// 374, `mapper_374.c` (reset-based 4-in-1): the outer 128 KiB bank counts resets.
    M374,
    /// 404, `mapper_404.c` (JY012005): `$6000-$7FFF` the outer bank, a lock in D7, D6 halves
    /// the inner PRG mask.
    M404,
    /// 550, `mapper_550.c` (JY-215): `$7000-$7FFF` the outer bank from A0-A3, a lock in A3;
    /// outer value `x11.` selects the MMC1, anything else a latch at `$8000`.
    M550,
    /// 543, `mapper_543.c` (CH-501 5-in-1): an outer register shifted in one bit at a time
    /// through `$5000-$5FFF` D3, which also banks the work RAM.
    M543,
}

impl Mmc1OuterKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            297 => Self::M297,
            374 => Self::M374,
            404 => Self::M404,
            550 => Self::M550,
            543 => Self::M543,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M297 => 297,
            Self::M374 => 374,
            Self::M404 => 404,
            Self::M550 => 550,
            Self::M543 => 543,
        }
    }
}

/// An MMC1 with an outer register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1Outer {
    mmc1: Mmc1,
    kind: Mmc1OuterKind,
    /// The outer register and, on 297 and 550, the latch of their non-MMC1 mode.
    regs: [u8; 2],
    /// 543's serial shift count (its accumulator is `regs[1]`).
    shift: u8,
}

impl Mmc1Outer {
    pub(crate) fn new(memory: &CartridgeMemory, kind: Mmc1OuterKind) -> Self {
        Self {
            mmc1: Mmc1::new(memory),
            kind,
            regs: [0; 2],
            shift: 0,
        }
    }

    /// Whether the MMC1 drives the windows now (297's `$4100` bit 0, 550's outer `x11.`).
    const fn chip_mode(&self) -> bool {
        match self.kind {
            Mmc1OuterKind::M297 => self.regs[0] & 0x01 != 0,
            Mmc1OuterKind::M550 => self.regs[0] & 0x06 == 0x06,
            Mmc1OuterKind::M374 | Mmc1OuterKind::M404 | Mmc1OuterKind::M543 => true,
        }
    }

    /// Rebuilds the MMC1's windows, then applies the outer bank or the latch mode.
    fn apply(&mut self, memory: &mut CartridgeMemory) {
        self.mmc1.refresh_banks(memory);
        let chip = self.chip_mode();
        let (r0, r1) = (i32::from(self.regs[0]), i32::from(self.regs[1]));
        let Some(b) = self.mmc1.banks_mut() else {
            return;
        };
        match (self.kind, chip) {
            // `prg_swap_mmc1_297`: 16 KiB `$08 | (v & 7)`, 4 KiB `$20 | (v & $1F)`.
            (Mmc1OuterKind::M297, true) => {
                b.remap_prg_banks(0x0F, 0x10);
                b.remap_chr_banks(0x7F, 0x80);
            }
            // `prg_fix_297`/`chr_fix_297`: `$4100` bit 1 the outer 64 KiB, the latch's D4-D5
            // the inner 16 KiB bank and D0-D3 the 8 KiB CHR bank; vertical mirroring.
            (Mmc1OuterKind::M297, false) => {
                let outer = (r0 & 0x02) << 1;
                b.map_prg_16k(0, outer | ((r1 & 0x30) >> 4));
                b.map_prg_16k(2, outer | 0x03);
                b.map_chr_8k(r1 & 0x0F);
                memory.set_mirroring(NametableMirroring::Vertical);
            }
            // `prg_swap_mmc1_374`: 16 KiB `outer << 3 | (v & 7)`, 4 KiB `outer << 5 | (v & $1F)`.
            (Mmc1OuterKind::M374, _) => {
                b.remap_prg_banks(0x0F, r0 << 4);
                b.remap_chr_banks(0x7F, r0 << 7);
            }
            // `prg_swap_mmc1_404`: D6 narrows the inner 16 KiB mask from `$0F` to `$07`.
            (Mmc1OuterKind::M404, _) => {
                let mask_16k = 0x0F >> ((r0 & 0x40) >> 6);
                b.remap_prg_banks((mask_16k << 1) | 0x01, r0 << 4);
                b.remap_chr_banks(0x7F, r0 << 7);
            }
            // `prg_swap_mmc1_550` in the MMC1 mode: 16 KiB `outer << 2 | (v & 7)`, 4 KiB
            // `$18 | (v & 7)`.
            (Mmc1OuterKind::M550, true) => {
                b.remap_prg_banks(0x0F, r0 << 3);
                b.remap_chr_banks(0x1F, 0x60);
            }
            // The latch mode: 32 KiB `outer << 1 | D4-D7`, 8 KiB CHR `(outer << 1 & $0C) | D0-D1`;
            // the MMC1's mirroring stays.
            // `prg_swap_mmc1_543`: 16 KiB `outer << 4 | (v & $0F)`, 4 KiB `v & 7`.
            (Mmc1OuterKind::M543, _) => {
                b.remap_prg_banks(0x1F, r0 << 5);
                b.remap_chr_banks(0x1F, 0);
            }
            (Mmc1OuterKind::M550, false) => {
                b.map_prg_32k((r0 << 1) | (r1 >> 4));
                b.map_chr_8k(((r0 << 1) & 0x0C) | (r1 & 0x03));
            }
        }
    }
}

impl Mapper for Mmc1Outer {
    fn banks(&self) -> Option<&BankWindows> {
        self.mmc1.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc1.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc1.save_registers(writer);
        writer.write_raw(&self.regs);
        writer.write_u8(self.shift);
    }

    /// The windows are in the state block already ([`Mapper::load_state`]).
    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc1.load_registers(reader)?;
        reader.read_into(&mut self.regs)?;
        self.shift = reader.read_u8()? & 0x03;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = [0; 2];
        self.shift = 0;
        self.mmc1.reset(memory);
        // `map_init_543`: the PRG register powers on as `$0E`, so that the menu boots.
        if self.kind == Mmc1OuterKind::M543 {
            self.mmc1.force_prg_register(0x0E);
        }
        self.apply(memory);
    }

    /// `map_init_374`: a press on RESET moves to the next outer bank, and the MMC1 is
    /// re-initialised (`init_MMC1(MMC1A, HARD)` on every reset). The others keep the default.
    fn soft_reset(&mut self, memory: &mut CartridgeMemory) {
        if self.kind == Mmc1OuterKind::M374 {
            self.regs[0] = self.regs[0].wrapping_add(1);
            self.mmc1.reset(memory);
            self.apply(memory);
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc1.prg_ram_enabled()
    }

    /// `wram_fix_mmc1_543`: the outer register's D1 picks a bank from D0/D2 over one from D0
    /// and the MMC1's `$A000` D3.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        if self.kind != Mmc1OuterKind::M543 {
            return self.mmc1.prg_ram_window(memory);
        }
        let r = self.regs[0];
        let bank = if r & 0x02 != 0 {
            0x04 | ((r & 0x04) >> 1) | (r & 0x01)
        } else {
            ((r & 0x01) << 1) | ((self.mmc1.chr0_register() & 0x08) >> 3)
        };
        (
            usize::from(bank) * BankWindows::PRG_WINDOW_SIZE,
            memory.prg_ram().len(),
        )
    }

    /// The outer registers below `$8000`: 297's at `$4100` (A8 set, `$4000-$5FFF`), 404's at
    /// `$6000-$7FFF` and 550's at `$7000-$7FFF`, until their lock bit is set. puNES lets no RAM
    /// take those writes, and neither does this board.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        match self.kind {
            Mmc1OuterKind::M297 if address < 0x6000 && address & 0x0100 != 0 => {
                self.regs[0] = value;
            }
            Mmc1OuterKind::M404 if (0x6000..0x8000).contains(&address) => {
                if self.regs[0] & 0x80 == 0 {
                    self.regs[0] = value;
                }
            }
            // `extcl_cpu_wr_mem_543`: D3 shifted in, four bits make the register.
            Mmc1OuterKind::M543 if (0x5000..0x6000).contains(&address) => {
                self.regs[1] |= ((value & 0x08) >> 3) << self.shift;
                if self.shift < 3 {
                    self.shift += 1;
                    return true;
                }
                self.regs[0] = self.regs[1];
                self.regs[1] = 0;
                self.shift = 0;
            }
            Mmc1OuterKind::M550 if (0x7000..0x8000).contains(&address) => {
                if self.regs[0] & 0x08 == 0 {
                    self.regs[0] = address.to_le_bytes()[0] & 0x0F;
                }
            }
            _ => return false,
        }
        self.apply(memory);
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match self.kind {
            Mmc1OuterKind::M297 if !self.chip_mode() => self.regs[1] = value,
            Mmc1OuterKind::M550 => {
                self.regs[1] = value;
                if self.chip_mode() {
                    self.mmc1.write_register(memory, address, value);
                }
            }
            _ => self.mmc1.write_register(memory, address, value),
        }
        self.apply(memory);
    }
}
