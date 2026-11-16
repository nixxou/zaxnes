//! NES 2.0 mapper 303, Kaiser KS7017, an FDS-conversion board (lot MAPPERS-N): a 16 KiB PRG bank
//! chosen by the **address** of a write to `$4A00-$4AFF`, a 16-bit CPU-cycle IRQ counter at
//! `$4020`/`$4021` and its status at `$4030`.
//!
//! Mesen2 `Kaiser/Kaiser7017.h` and puNES `mapper_303.c` agree (read, not copied):
//!
//! * a write to `$4A00-$4AFF` latches `((A >> 2) & 3) | ((A >> 4) & 4)`, and a write to
//!   `$5100-$51FF` maps it at `$8000`; `$C000` is bank 2;
//! * `$4020` writes the counter's low byte, `$4021` its high byte and enables it, both
//!   acknowledging the IRQ; the counter counts down every CPU cycle and, reaching 0, raises the
//!   IRQ and stops;
//! * `$4025` bit 3 is the mirroring (1 horizontal);
//! * a read of `$4030` answers 1 while the IRQ is pending, and acknowledges it (Mesen2's
//!   `ReadRegister`; puNES computes the same value but then returns the work-RAM read, which this
//!   board does not follow, FIDELITY-LEADS J25).
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Mapper 303.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kaiser7017 {
    banks: BankWindows,
    latch: u8,
    prg: u8,
    counter: u16,
    enabled: bool,
    pending: bool,
}

impl Kaiser7017 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            latch: 0,
            prg: 0,
            counter: 0,
            enabled: false,
            pending: false,
        };
        board.refresh();
        board
    }

    fn refresh(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, 2);
        self.banks.map_chr_8k(0);
    }
}

impl Mapper for Kaiser7017 {
    banked!();

    fn mapper_number(&self) -> i32 {
        303
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.latch);
        writer.write_u8(self.prg);
        writer.write_u16(self.counter);
        writer.write_bool(self.enabled);
        writer.write_bool(self.pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.latch = reader.read_u8()?;
        self.prg = reader.read_u8()?;
        self.counter = reader.read_u16()?;
        self.enabled = reader.read_bool()?;
        self.pending = reader.read_bool()?;
        self.refresh();
        Ok(())
    }

    /// `InitMapper`: everything 0 and vertical mirroring.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.latch = 0;
        self.prg = 0;
        self.counter = 0;
        self.enabled = false;
        self.pending = false;
        self.refresh();
        memory.set_mirroring(NametableMirroring::Vertical);
    }

    fn irq_asserted(&self) -> bool {
        self.pending
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.enabled && self.counter != 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.enabled = false;
                self.pending = true;
            }
        }
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        let value = self.cpu_peek_override(memory, address);
        if value.is_some() {
            self.pending = false;
        }
        value
    }

    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        (address == 0x4030).then_some(Some(u8::from(self.pending)))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address {
            0x4A00..=0x4AFF => {
                self.latch =
                    (address.to_le_bytes()[0] >> 2) & 0x03 | (address.to_le_bytes()[0] >> 4) & 0x04;
            }
            0x5100..=0x51FF => {
                self.prg = self.latch;
                self.refresh();
            }
            0x4020 => {
                self.pending = false;
                self.counter = (self.counter & 0xFF00) | u16::from(value);
            }
            0x4021 => {
                self.pending = false;
                self.counter = (self.counter & 0x00FF) | (u16::from(value) << 8);
                self.enabled = true;
            }
            0x4025 => memory.set_mirroring(if value & 0x08 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            _ => {}
        }
    }
}
