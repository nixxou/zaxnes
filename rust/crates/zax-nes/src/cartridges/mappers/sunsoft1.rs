use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 184, Sunsoft-1 — *Atlantis no Nazo* and *The Wing of Madoola* (REQ-CORE-102).
///
/// nesdev ("INES Mapper 184"), one register at **$6000-$7FFF** and no PRG banking at all:
///
/// ```text
/// 7  bit  0
/// .1HH .LLL
///  |||  |||
///  |||  +++- Select 4k CHR bank from $0000-0FFF
///  +++------ Select 4k CHR bank from $1000-1FFF; only banks 4-7 selectable
/// ```
///
/// **Bit 6 is tied high in hardware**, which is what "only banks 4-7 selectable" means, so the
/// upper window's bank is `4 | HH` and not the three bits as written. The page adds that
/// "existing software is well behaved and writes to this with the $40s bit set", so a real image
/// cannot tell this reading from one that took all three bits — Mesen 2 takes all three
/// (`Sunsoft184.h:21`) and agrees on every image that exists.
///
/// The page's other statement is the one §4.6 of `docs/review/MF-6-inventory.md` asked for:
/// "This mapper precludes iNES-default RAM because the register is mapped into $6000-7FFF", so
/// `registry::board_memory_sizes` gives the board **no PRG-RAM** and the window is the register.
/// No mirroring register, no IRQ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sunsoft1 {
    banks: BankWindows,
    register: u8,
}

impl Sunsoft1 {
    /// The bank of the upper 4 KiB window: `HH` with bit 2 forced, i.e. banks 4 to 7.
    const UPPER_BASE: u8 = 0x04;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(0);
        self.banks.map_chr_4k(0, i32::from(self.register & 0x07));
        self.banks.map_chr_4k(
            4,
            i32::from(Self::UPPER_BASE | ((self.register >> 4) & 0x03)),
        );
    }
}

impl Mapper for Sunsoft1 {
    banked!();

    fn mapper_number(&self) -> i32 {
        184
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
