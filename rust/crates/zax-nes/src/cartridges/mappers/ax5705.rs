//! NES 2.0 mapper 530, the AX5705 *Super Mario Bros.* pirate board (lot MAPPERS-N): a VRC4 whose
//! A12 comes from CPU A3 and whose data lines are crossed on the PRG and CHR-high registers.
//!
//! puNES `mapper_530.c` (read, not copied): a VRC4 with A0/A1 on CPU A0/A1 (the VRC4f wiring),
//! the address OR'd with `(A & 8) << 9`, and on `$8000`/`$A000` (address `& $F001`, A0 either)
//! the value's bits 1 and 3 swapped to 3 and 1 — `((v & 2) << 2) | ((v & 8) >> 2) | (v & 5)` —
//! and on the high nibbles `$B001-$E001` bits 1 and 2 swapped — `((v & 4) >> 1) | ((v & 2) << 1)
//! | (v & 9)`. Mesen2's `Ax5705.h` decodes the same registers and the same crossings but leaves
//! out the rest of the VRC4: no IRQ, only `$8008` for the mirroring (FIDELITY-LEADS J29). This
//! board takes puNES's whole VRC4, through [`Vrc2Vrc4`] untouched.
use super::{Mapper, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Mapper 530.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ax5705 {
    vrc: Vrc2Vrc4,
}

impl Ax5705 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            vrc: Vrc2Vrc4::new_vrc4f(memory, 530),
        }
    }

    /// The address and value the chip sees for a CPU write of `value` at `address`.
    const fn unscramble(address: u16, value: u8) -> (u16, u8) {
        let address = address | ((address & 0x0008) << 9);
        let value = match address & 0xF001 {
            0x8000 | 0x8001 | 0xA000 | 0xA001 => {
                ((value & 0x02) << 2) | ((value & 0x08) >> 2) | (value & 0x05)
            }
            0xB001 | 0xC001 | 0xD001 | 0xE001 => {
                ((value & 0x04) >> 1) | ((value & 0x02) << 1) | (value & 0x09)
            }
            _ => value,
        };
        (address, value)
    }
}

impl Mapper for Ax5705 {
    fn banks(&self) -> Option<&BankWindows> {
        self.vrc.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.vrc.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        530
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.vrc.save_registers(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.vrc.load_registers(reader)
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.reset(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.vrc.irq_asserted()
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.on_cpu_cycle(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        let (address, value) = Self::unscramble(address, value);
        self.vrc.write_register(memory, address, value);
    }
}
