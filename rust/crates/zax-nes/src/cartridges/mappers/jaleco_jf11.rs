use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 140, the Jaleco JF-11 and JF-14 of *Bio Senshi Dan* (REQ-CORE-102).
///
/// nesdev ("INES Mapper 140"), one register:
///
/// ```text
/// $6000-7FFF: [...PP CCCC]
///   P = 32 KiB PRG bank at $8000
///   C = 8 KiB CHR bank at $0000
/// ```
///
/// A GNROM with the register moved below $8000, and the page draws the consequence itself:
/// "Regs lie at $6000-7FFF, so there's no SRAM". So the window is the register and not RAM —
/// `registry::board_memory_sizes` gives the board none — and the write is taken by
/// [`Mapper::intercept_cpu_write`], which is the shape [`super::JalecoJ87`] established for
/// mappers 87 and 101. No mirroring register, no IRQ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoJf11 {
    banks: BankWindows,
    register: u8,
}

impl JalecoJf11 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks
            .map_prg_32k(i32::from((self.register >> 4) & 0x03));
        self.banks.map_chr_8k(i32::from(self.register & 0x0F));
    }
}

impl Mapper for JalecoJf11 {
    banked!();

    fn mapper_number(&self) -> i32 {
        140
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        let selected = (0x6000..=0x7FFF).contains(&address);
        if selected {
            self.register = value;
            self.update_banks();
        }

        selected
    }
}
