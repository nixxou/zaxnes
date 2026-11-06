use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 75, Konami VRC1 (REQ-CORE-102).
///
/// nesdev ("VRC1"), five registers, each decoded on `address & $F000`:
///
/// | address | what it does |
/// | --- | --- |
/// | `$8000-$8FFF` | bits 3-0: 8 KiB PRG bank at $8000 |
/// | `$9000-$9FFF` | bit 0: mirroring, `0 = Vertical`, `1 = Horizontal`; bit 1: **high bit** of the 4 KiB CHR bank at PPU $0000; bit 2: the same for PPU $1000 |
/// | `$A000-$AFFF` | bits 3-0: 8 KiB PRG bank at $A000 |
/// | `$C000-$CFFF` | bits 3-0: 8 KiB PRG bank at $C000 |
/// | `$E000-$EFFF` | bits 3-0: **low four bits** of the 4 KiB CHR bank at PPU $0000 |
/// | `$F000-$FFFF` | bits 3-0: the same for PPU $1000 |
///
/// $E000-$FFFF is wired to the last PRG bank. The chip has **no IRQ, no PRG-RAM and no bus
/// conflicts**, which is the whole of why mapper 75 is in this lot and the VRC2/VRC4 of
/// [`super::Vrc2Vrc4`] is not a relative of it in this code.
///
/// The one condition worth spelling out is the page's: "the mirroring bit is ignored if the
/// cartridge is wired for 4-screen VRAM, as is typical for Vs. System games using the VRC1" —
/// so a four-screen header keeps its layout whatever `$9000` says. Mapper 151 is this board on a
/// VS System board and §6 of `docs/review/MF-6-inventory.md` sets the VS System aside, so 151 is
/// not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc1 {
    banks: BankWindows,
    prg: [u8; 3],
    /// The two 4 KiB CHR banks, five bits each: `$E000`/`$F000` hold the low four and `$9000`
    /// bits 1 and 2 the high one.
    chr: [u8; 2],
    /// A four-screen header, which makes `$9000` bit 0 a no-op. Settled at load.
    four_screen: bool,
}

impl Vrc1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 1, 2],
            chr: [0; 2],
            four_screen: memory.rom().header().mirroring == NametableMirroring::FourScreen,
        }
    }

    fn update_banks(&mut self) {
        for (slot, bank) in self.prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, i32::from(*bank));
        }

        self.banks.map_prg_8k(3, -1);
        self.banks.map_chr_4k(0, i32::from(self.chr[0]));
        self.banks.map_chr_4k(4, i32::from(self.chr[1]));
    }
}

impl Mapper for Vrc1 {
    banked!();

    fn mapper_number(&self) -> i32 {
        75
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0, 1, 2];
        self.chr = [0; 2];
        self.update_banks();
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 => self.prg[0] = value & 0x0F,
            0xA000 => self.prg[1] = value & 0x0F,
            0xC000 => self.prg[2] = value & 0x0F,
            0x9000 => {
                if !self.four_screen {
                    memory.set_mirroring(if value & 0x01 == 0 {
                        NametableMirroring::Vertical
                    } else {
                        NametableMirroring::Horizontal
                    });
                }

                // Bit 1 is CHR A16 of the first window and bit 2 the same of the second, i.e.
                // bit 4 of each 4 KiB bank number.
                self.chr[0] = (self.chr[0] & 0x0F) | ((value & 0x02) << 3);
                self.chr[1] = (self.chr[1] & 0x0F) | ((value & 0x04) << 2);
            }
            0xE000 => self.chr[0] = (self.chr[0] & 0x10) | (value & 0x0F),
            0xF000 => self.chr[1] = (self.chr[1] & 0x10) | (value & 0x0F),
            _ => return,
        }

        self.update_banks();
    }
}
