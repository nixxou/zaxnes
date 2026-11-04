// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The OR'd heuristic `Vrc2Vrc4::new` falls back on when the NES 2.0 submapper is absent is
// Mesen's `TranslateAddress` (`Core/NES/Mappers/Konami/VRC2_4.h`); see THIRD-PARTY-NOTICES.md.

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
    irq_latch: u8,
    irq_counter: i32,
    prescaler: i32,
    irq_enabled: bool,
    irq_enable_after_ack: bool,
    irq_cycle_mode: bool,
    irq_pending: bool,
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
            irq_latch: 0,
            irq_counter: 0,
            prescaler: 341,
            irq_enabled: false,
            irq_enable_after_ack: false,
            irq_cycle_mode: false,
            irq_pending: false,
        }
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

    fn write_irq_register(&mut self, index: i32, value: u8) {
        match index {
            0 => self.irq_latch = (self.irq_latch & 0xF0) | (value & 0x0F),
            1 => self.irq_latch = (self.irq_latch & 0x0F) | ((value & 0x0F) << 4),
            2 => {
                self.irq_enable_after_ack = value & 0x01 != 0;
                self.irq_enabled = value & 0x02 != 0;
                self.irq_cycle_mode = value & 0x04 != 0;
                if self.irq_enabled {
                    self.irq_counter = i32::from(self.irq_latch);
                    self.prescaler = 341;
                }

                self.irq_pending = false;
            }
            _ => {
                self.irq_pending = false;
                self.irq_enabled = self.irq_enable_after_ack;
            }
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

    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_pending = true;
            return;
        }

        self.irq_counter += 1;
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
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_i32(self.prescaler);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_enable_after_ack);
        writer.write_bool(self.irq_cycle_mode);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for value in &mut self.chr {
            *value = reader.read_i32()?;
        }
        self.prg0 = reader.read_u8()?;
        self.prg1 = reader.read_u8()?;
        self.swap_prg = reader.read_bool()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.prescaler = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_enable_after_ack = reader.read_bool()?;
        self.irq_cycle_mode = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg0 = 0;
        self.prg1 = 1;
        self.swap_prg = false;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.prescaler = 341;
        self.irq_enabled = false;
        self.irq_enable_after_ack = false;
        self.irq_cycle_mode = false;
        self.irq_pending = false;
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

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.is_vrc2 || !self.irq_enabled {
            return;
        }

        if self.irq_cycle_mode {
            self.clock_irq_counter();
            return;
        }

        self.prescaler -= 3;
        if self.prescaler > 0 {
            return;
        }

        self.prescaler += 341;
        self.clock_irq_counter();
    }
}
