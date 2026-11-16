//! NES 2.0 mapper 529, the T-230 / YY0807 bootleg boards of *Datach Dragon Ball Z* and *Dragon
//! Ball Z Gaiden* (lot MAPPERS-N): a VRC4e clone whose `$A00x` register selects one 16 KiB bank at
//! `$8000`, the last 16 KiB fixed at `$C000`.
//!
//! nesdev ("NES 2.0 Mapper 529"): "a VRC4 clone (A2/A3, VRC4e)", "$800x register has no effect",
//! "$A00x register selects a 16 KiB PRG-ROM bank"; puNES `mapper_529.c` (read, not copied) maps
//! it at `$8000` and the last bank at `$C000`, the rest of the chip a plain VRC4. Mesen2's
//! `T230.h` is another reading (an outer bank for CHR-RAM boards, an `$A000` pair in 8 KiB), which
//! this board does not follow (FIDELITY-LEADS J30).
//!
//! **The 93C56 EEPROM some of these carts save to is not emulated**: nesdev's `$8800` writes reach
//! the VRC4 like any other, and `$5000-$5FFF` answers 1 on D0, puNES's answer when the image
//! declares no EEPROM (J30).
use super::{Mapper, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Mapper 529.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct T230 {
    vrc: Vrc2Vrc4,
    /// The 16 KiB bank `$A00x` selected.
    prg: u8,
}

impl T230 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            vrc: Vrc2Vrc4::new_vrc4e(memory, 529),
            prg: 0,
        }
    }

    /// puNES `prg_fix_vrc2and4_529`: the bank at `$8000`, the last one at `$C000`.
    fn map_prg(&mut self) {
        let bank = i32::from(self.prg);
        if let Some(banks) = self.vrc.banks_mut() {
            banks.map_prg_16k(0, bank);
            banks.map_prg_16k(2, -1);
        }
    }
}

impl Mapper for T230 {
    fn banks(&self) -> Option<&BankWindows> {
        self.vrc.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.vrc.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        529
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.vrc.save_registers(writer);
        writer.write_u8(self.prg);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.vrc.load_registers(reader)?;
        self.prg = reader.read_u8()?;
        self.map_prg();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.prg = 0;
        self.vrc.reset(memory);
        self.map_prg();
    }

    fn irq_asserted(&self) -> bool {
        self.vrc.irq_asserted()
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.on_cpu_cycle(memory);
    }

    /// `$5000-$5FFF`: the EEPROM's data out on D0, 1 with no EEPROM (puNES).
    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        (0x5000..0x6000).contains(&address).then_some(Some(0x01))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 => return,
            0xA000 => self.prg = value & 0x1F,
            _ if address < 0x8000 => return,
            _ => {}
        }
        self.vrc.write_register(memory, address, value);
        self.map_prg();
    }
}
