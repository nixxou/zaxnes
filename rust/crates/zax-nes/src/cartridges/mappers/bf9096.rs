use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 232, the Camerica BF9096 of the *Quattro* carts (REQ-CORE-102).
///
/// nesdev ("INES Mapper 232"), two registers and a 64 KiB block:
///
/// ```text
/// $8000-$BFFF:  [...B B...]   PRG Block Select
/// $C000-$FFFF:  [.... ..PP]   PRG Page Select
/// ```
///
/// "Pages are taken from the 64k block currently selected by $8000", so the window at $8000 is
/// `(block << 2) | page` in 16 KiB units and the one at $C000 is `(block << 2) | 3` — the last
/// page of the same block, fixed. Four blocks of four pages is the 256 KiB of a *Quattro* cart.
///
/// **The lockout defeat circuit is not emulated**, and the page says it need not be: "this
/// mapper also involves a custom lockout defeat circuit which is mostly unimportant for
/// emulation purposes".
///
/// NES 2.0 submapper 1 is the **Aladdin Deck Enhancer**, whose outer bank bits are swapped. The
/// page's own line is "swap the bits of the outer bank number"; Mesen 2 implements exactly that
/// and adds a doubt — "but this seems to match the Pegasus 4-in-1 behavior? Wiki wrong?"
/// (`BF9096.h:28-33`). The swap is implemented, as a load-time `bool`, and the doubt recorded
/// here rather than resolved: this role has no image of either cartridge to settle it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bf9096 {
    banks: BankWindows,
    block: u8,
    page: u8,
    /// NES 2.0 submapper 1, the Aladdin Deck Enhancer: the two block bits the other way round.
    swap_block_bits: bool,
}

impl Bf9096 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            banks: windows_for(memory),
            block: 0,
            page: 0,
            swap_block_bits: header.is_nes20 && header.submapper == 1,
        }
    }

    fn update_banks(&mut self) {
        let base = i32::from(self.block) << 2;
        self.banks.map_prg_16k(0, base | i32::from(self.page));
        self.banks.map_prg_16k(2, base | 3);
    }
}

impl Mapper for Bf9096 {
    banked!();

    fn mapper_number(&self) -> i32 {
        232
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.block);
        writer.write_u8(self.page);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.block = reader.read_u8()?;
        self.page = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.block = 0;
        self.page = 0;
        self.update_banks();
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address {
            0x8000..=0xBFFF => {
                self.block = if self.swap_block_bits {
                    ((value >> 4) & 0x01) | ((value >> 2) & 0x02)
                } else {
                    (value >> 3) & 0x03
                };
            }
            0xC000..=0xFFFF => self.page = value & 0x03,
            _ => return,
        }

        self.update_banks();
    }
}
