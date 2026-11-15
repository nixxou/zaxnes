//! Kaiser FDS-conversion boards whose PRG is banked in **2 KiB** pages (lot MAPPERS-N), finer than
//! the 8 KiB windows of [`BankWindows`]: every CPU read of `$6000-$FFFF` is answered here from the
//! page table, and only CHR goes through the window table.
//!
//! Read from Mesen2 (`cleanroom/third_party/mesen2/Core/NES/Mappers/Kaiser/…`, read, not copied).

use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FineRomKind {
    /// 302, `Kaiser7057.h` (KS7057, *Gyruss* FDS conversion).
    M302,
    /// 305, `Kaiser7031.h` (KS7031, *Dracula II* FDS conversion).
    M305,
}

impl FineRomKind {
    /// The kind of an iNES number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        match mapper {
            302 => Some(Self::M302),
            305 => Some(Self::M305),
            _ => None,
        }
    }
}

/// A board of this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FineRom {
    /// CHR only: both boards have one 8 KiB CHR bank.
    banks: BankWindows,
    kind: FineRomKind,
    regs: [u8; 8],
}

impl FineRom {
    pub(crate) fn new(memory: &CartridgeMemory, kind: FineRomKind) -> Self {
        Self {
            banks: windows_for(memory),
            kind,
            regs: [0; 8],
        }
    }

    /// The 2 KiB PRG-ROM page CPU `address` shows, for `$6000-$FFFF`.
    fn page(&self, address: u16) -> Option<i32> {
        if address < 0x6000 {
            return None;
        }
        let r = |i: usize| i32::from(self.regs[i]);
        let slot = usize::from((address - 0x6000) >> 11);
        Some(match self.kind {
            // `Kaiser7057.h` `UpdateState`: `$6000-$7FFF` from R4-R7, `$8000-$9FFF` from R0-R3,
            // then `SelectPrgPage4x(1, 0x34)`, `(2, 0x38)`, `(3, 0x3C)`: 2 KiB pages $34-$3F.
            FineRomKind::M302 => match slot {
                0..=3 => r(4 + slot),
                4..=7 => r(slot - 4),
                _ => 0x34 + i32::try_from(slot - 8).unwrap_or(0),
            },
            // `Kaiser7031.h`: `$6000-$7FFF` from the four registers, `$8000-$FFFF` the pages 15
            // down to 0 (`SelectPrgPage(i, 15 - i)`).
            FineRomKind::M305 => match slot {
                0..=3 => r(slot),
                _ => 15 - i32::try_from(slot - 4).unwrap_or(0),
            },
        })
    }
}

impl Mapper for FineRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        match self.kind {
            FineRomKind::M302 => 302,
            FineRomKind::M305 => 305,
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = [0; 8];
        self.banks.map_chr_8k(0);
        // `Kaiser7031.h` `InitMapper`: "SetMirroringType(MirroringType::Vertical)".
        if self.kind == FineRomKind::M305 {
            memory.set_mirroring(NametableMirroring::Vertical);
        }
    }

    /// `$6000-$7FFF` is PRG-ROM on both boards.
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn rom_windows(&self, _memory: &CartridgeMemory) -> [usize; 4] {
        [NO_ROM_WINDOW; 4]
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        let page = self.page(address)?;
        let rom = memory.rom().prg_rom();
        let count = i32::try_from(rom.len() / 0x800).unwrap_or(1).max(1);
        let base = usize::try_from(page.rem_euclid(count)).unwrap_or(0) * 0x800;
        rom.get(base + usize::from(address & 0x07FF)).copied()
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match self.kind {
            // `Kaiser7057.h` `WriteRegister`: mask `$F002`, A0 picks the nibble.
            FineRomKind::M302 => {
                let index = match address & 0xF002 {
                    0x8000 | 0x8002 | 0x9000 | 0x9002 => {
                        memory.set_mirroring(if value & 0x01 != 0 {
                            NametableMirroring::Vertical
                        } else {
                            NametableMirroring::Horizontal
                        });
                        return;
                    }
                    0xB000 => 0,
                    0xB002 => 1,
                    0xC000 => 2,
                    0xC002 => 3,
                    0xD000 => 4,
                    0xD002 => 5,
                    0xE000 => 6,
                    0xE002 => 7,
                    _ => return,
                };
                let register = &mut self.regs[index];
                *register = if address & 0x01 == 0 {
                    (*register & 0xF0) | (value & 0x0F)
                } else {
                    (*register & 0x0F) | ((value << 4) & 0xF0)
                };
            }
            // `Kaiser7031.h`: "_regs[(addr >> 11) & 0x03] = value".
            FineRomKind::M305 => self.regs[usize::from((address >> 11) & 0x03)] = value,
        }
    }
}
