// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The OR'd heuristic `Vrc2Vrc4::new` falls back on when the NES 2.0 submapper is absent is
// Mesen's `TranslateAddress` (`Core/NES/Mappers/Konami/VRC2_4.h`); see THIRD-PARTY-NOTICES.md.

use super::vrc_irq::VrcIrq;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// One of the nine wirings of the VRC2/VRC4 silicon (nesdev, "VRC2 and VRC4", the variant
/// table), plus the pirate board of mapper 27. The dump's mapper number and NES 2.0 submapper
/// name it; without a submapper, mappers 21, 23 and 25 each allow several and the board falls
/// back on the OR'd heuristic instead (MR-21-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VrcVariant {
    Vrc2a,
    Vrc2b,
    Vrc2c,
    Vrc4a,
    Vrc4b,
    Vrc4c,
    Vrc4d,
    Vrc4e,
    Vrc4f,
    Vrc427,
}

impl VrcVariant {
    /// The variant of `mapper_number` with `submapper`, or `None` when the dump does not say
    /// and the number allows more than one (nesdev, "NES 2.0 submappers"). Mapper 22 is always
    /// VRC2a and mapper 27 always the pirate board, so neither is ever unknown.
    fn resolve(mapper_number: i32, is_nes20: bool, submapper: i32) -> Option<Self> {
        let submapper = if is_nes20 { submapper } else { 0 };
        match (mapper_number, submapper) {
            (22, _) => Some(Self::Vrc2a),
            (27, _) => Some(Self::Vrc427),
            (21, 1) => Some(Self::Vrc4a),
            (21, 2) => Some(Self::Vrc4c),
            (23, 1) => Some(Self::Vrc4f),
            (23, 2) => Some(Self::Vrc4e),
            (23, 3) => Some(Self::Vrc2b),
            (25, 1) => Some(Self::Vrc4b),
            (25, 2) => Some(Self::Vrc4d),
            (25, 3) => Some(Self::Vrc2c),
            _ => None,
        }
    }

    /// The two address lines that supply bits 0 and 1 of the register index, as the variant
    /// table prints them: `[line for bit 0, line for bit 1]`. VRC4c's A6 and A7 are the pair
    /// no OR of A0-A3 can reach (MR-21-3).
    const fn index_lines(self) -> [u32; 2] {
        match self {
            Self::Vrc2a | Self::Vrc2c | Self::Vrc4b => [1, 0],
            Self::Vrc2b | Self::Vrc4f | Self::Vrc427 => [0, 1],
            Self::Vrc4a => [1, 2],
            Self::Vrc4c => [6, 7],
            Self::Vrc4d => [3, 2],
            Self::Vrc4e => [2, 3],
        }
    }

    /// A VRC2 has no IRQ counter and no PRG swap mode, and all of $9000-$9003 is its
    /// nametable-arrangement register (MR-21-2).
    const fn is_vrc2(self) -> bool {
        matches!(self, Self::Vrc2a | Self::Vrc2b | Self::Vrc2c)
    }
}

/// iNES mappers 21, 22, 23, 25 and 27, Konami VRC2 and VRC4 (REQ-CORE-100): the same silicon
/// with the register address lines wired differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc2Vrc4 {
    banks: BankWindows,
    mapper_number: i32,
    chr: [i32; 8],
    /// The address lines of the register index, and a second pair OR'd into them. For a known
    /// variant the two pairs are the same and the OR is a no-op; for submapper 0 of mappers
    /// 21, 23 and 25 they are the two candidate wirings. Load-time, so `register_index` has
    /// no branch at all.
    index_lines: [u32; 2],
    index_or_lines: [u32; 2],
    is_vrc2: bool,
    chr_shift: bool,
    chr_high_mask: i32,
    prg0: u8,
    prg1: u8,
    swap_prg: bool,
    /// Konami's IRQ counter, now [`VrcIrq`] and shared with the VRC6 (lot MF-5a step 1). The
    /// seven fields it replaces were written to the save state in exactly the order
    /// [`VrcIrq::save_state`] writes them, so the block is unchanged.
    irq: VrcIrq,
}

impl Vrc2Vrc4 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let header = memory.rom().header();
        let variant = VrcVariant::resolve(mapper_number, header.is_nes20, header.submapper);
        let (index_lines, index_or_lines) = match variant {
            Some(variant) => (variant.index_lines(), variant.index_lines()),
            // Submapper 0 of mapper 21, 23 or 25: the candidate wirings are OR'd together,
            // the heuristic Mesen documents with "ORing both values should make most games
            // work" (`VRC2_4.h:208-247`). The third candidate of each number shares its lines
            // with one of these two, so a pair covers it.
            None => match mapper_number {
                21 => (
                    VrcVariant::Vrc4a.index_lines(),
                    VrcVariant::Vrc4c.index_lines(),
                ),
                23 => (
                    VrcVariant::Vrc2b.index_lines(),
                    VrcVariant::Vrc4e.index_lines(),
                ),
                _ => (
                    VrcVariant::Vrc4b.index_lines(),
                    VrcVariant::Vrc4d.index_lines(),
                ),
            },
        };
        Self {
            banks: windows_for(memory),
            mapper_number,
            chr: [0; 8],
            index_lines,
            index_or_lines,
            // An unknown variant keeps the VRC4 features: a VRC4 game needs them, and a dump
            // that really is a VRC2b or VRC2c says so with submapper 3.
            is_vrc2: variant.is_some_and(VrcVariant::is_vrc2),
            // "On VRC2a (mapper 22), the low bit is ignored (right shift value by 1)" — the one
            // variant with 2 KiB CHR granularity.
            chr_shift: variant == Some(VrcVariant::Vrc2a),
            // The pirate board of mapper 27 widens the high half of each CHR register from
            // five bits to eight (AMD-021 R2-004).
            chr_high_mask: if mapper_number == 27 { 0xFF } else { 0x1F },
            prg0: 0,
            prg1: 1,
            swap_prg: false,
            irq: VrcIrq::new(),
        }
    }

    /// Mapper 116's VRC2 mode: nesdev ("INES Mapper 116") gives it as "A0/A1, i.e. VRC2b",
    /// which is exactly [`VrcVariant::Vrc2b`] — Mesen's scrambled-looking
    /// `((((addr & 0x02) | (addr >> 10)) >> 1) + 2) & 0x07` resolves to A1 picking the
    /// half-register and A0 the nibble, which is what this build's mapper-23 arm computes with
    /// `index = addr & 3`. A mapper-116 image declares no VRC submapper, so the variant is asked
    /// for directly instead of being resolved from the header.
    #[cfg(feature = "board-116")]
    pub(crate) fn new_vrc2b(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let lines = VrcVariant::Vrc2b.index_lines();
        Self {
            index_lines: lines,
            index_or_lines: lines,
            is_vrc2: true,
            chr_shift: false,
            chr_high_mask: 0x1F,
            mapper_number,
            ..Self::new(memory, 23)
        }
    }

    /// Recomputes the windows from the registers (mapper 116); see
    /// [`super::Mmc3Board::refresh_banks`].
    #[cfg(feature = "board-116")]
    pub(crate) fn refresh_banks(&mut self) {
        self.update_banks();
    }

    /// Sets all eight CHR registers without a register write.
    ///
    /// nesdev ("INES Mapper 116") has the VRC2's CHR registers powering up to **`$FF`** on that
    /// board, and "one version of *Somari the Adventurer* relies on this". A **method and not a
    /// field**, for the reason MF-6b step 1 measured: one byte of displacement on
    /// `VrcIrq::pending` — which `Board::irq_asserted` reads for this very board — re-laid out
    /// `Cpu6502::branch` and cost eleven of lot PR-1's thirteen ROMs +0.009 % to +0.027 % with
    /// no code added at all. The same shape as [`super::Mmc1::force_chr0_register`].
    #[cfg(feature = "board-116")]
    pub(crate) fn force_chr_registers(&mut self, value: i32) {
        self.chr = [value; 8];
        self.update_banks();
    }

    /// MR-21-1 and MR-21-3: the register index comes from the two address lines the variant
    /// table gives this chip, not from the OR'd heuristic of its mapper number. The OR is
    /// still here and still free, because a known variant has both pairs equal.
    fn register_index(&self, address: u16) -> i32 {
        let a = i32::from(address);
        let bit = |lines: [u32; 2], which: usize| {
            ((a >> lines[which]) | (a >> self.index_or_lines[which])) & 1
        };
        bit(self.index_lines, 0) | (bit(self.index_lines, 1) << 1)
    }

    /// The four registers of $F000-$FFFF, reached through the variant's two index lines: the
    /// latch's low nibble, its high nibble, the control register and the acknowledge.
    fn write_irq_register(&mut self, index: i32, value: u8) {
        match index {
            0 => self.irq.write_latch_nibble(value, false),
            1 => self.irq.write_latch_nibble(value, true),
            2 => self.irq.write_control(value),
            _ => self.irq.acknowledge(),
        }
    }

    fn update_banks(&mut self) {
        if self.swap_prg {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.prg0));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.prg0));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.prg1));
        self.banks.map_prg_8k(3, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            let bank = if self.chr_shift {
                self.chr[slot] >> 1
            } else {
                self.chr[slot]
            };
            self.banks.map_chr_1k(slot, bank);
        }
    }
}

impl Mapper for Vrc2Vrc4 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        for value in self.chr {
            writer.write_i32(value);
        }
        writer.write_u8(self.prg0);
        writer.write_u8(self.prg1);
        writer.write_bool(self.swap_prg);
        self.irq.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for value in &mut self.chr {
            *value = reader.read_i32()?;
        }
        self.prg0 = reader.read_u8()?;
        self.prg1 = reader.read_u8()?;
        self.swap_prg = reader.read_bool()?;
        self.irq.load_state(reader)?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn irq_asserted(&self) -> bool {
        self.irq.asserted()
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg0 = 0;
        self.prg1 = 1;
        self.swap_prg = false;
        self.irq.reset();
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let index = self.register_index(address);
        match address & 0xF000 {
            0x8000 => {
                self.prg0 = value & 0x1F;
                self.update_banks();
            }
            0x9000 => {
                if self.is_vrc2 {
                    // MR-21-2, nesdev ("VRC2 and VRC4"): "VRC2 does not have a Swap Mode and
                    // the bank this register will switch is always the $8000-$9FFF bank", so
                    // all four of $9000-$9003 are the nametable arrangement. AMD-021 R2-004
                    // records the one-bit mask as an accepted divergence.
                    memory.set_mirroring(if value & 0x01 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                } else if index < 2 {
                    memory.set_mirroring(match value & 0x03 {
                        0 => NametableMirroring::Vertical,
                        1 => NametableMirroring::Horizontal,
                        2 => NametableMirroring::SingleScreenA,
                        _ => NametableMirroring::SingleScreenB,
                    });
                } else {
                    self.swap_prg = value & 0x02 != 0;
                    self.update_banks();
                }
            }
            0xA000 => {
                self.prg1 = value & 0x1F;
                self.update_banks();
            }
            0xB000..=0xE000 => {
                let group = i32::from((address & 0xF000) - 0xB000) >> 12;
                let slot = usize::try_from(group * 2 + (index >> 1)).unwrap_or(0);
                if index & 1 == 0 {
                    self.chr[slot] = (self.chr[slot] & !0x0F) | i32::from(value & 0x0F);
                } else {
                    self.chr[slot] =
                        (self.chr[slot] & 0x0F) | ((i32::from(value) & self.chr_high_mask) << 4);
                }

                self.update_banks();
            }
            _ => {
                if !self.is_vrc2 {
                    self.write_irq_register(index, value);
                }
            }
        }
    }

    /// "A VRC2 has no IRQ counter" (MR-21-2), so the whole arm is behind that load-time flag.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.is_vrc2 {
            return;
        }

        self.irq.step_cpu_cycle();
    }
}
