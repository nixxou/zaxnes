use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 64, Tengen RAMBO-1: MMC3-like with a third PRG register, 1 KiB CHR mode and a
/// scanline counter that can also count CPU cycles (REQ-CORE-100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rambo1 {
    banks: BankWindows,
    a12: A12Watcher,
    bank: [u8; 16],
    select: u8,
    irq_latch: u8,
    irq_counter: i32,
    cycle_divider: i32,
    cycle_mode: bool,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Rambo1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            a12: A12Watcher::new(),
            bank: [0; 16],
            select: 0,
            irq_latch: 0,
            irq_counter: 0,
            cycle_divider: 0,
            cycle_mode: false,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    fn update_banks(&mut self) {
        let b = |i: usize| i32::from(self.bank[i]);
        if self.select & 0x40 != 0 {
            self.banks.map_prg_8k(0, b(15));
            self.banks.map_prg_8k(1, b(6));
            self.banks.map_prg_8k(2, b(7));
        } else {
            self.banks.map_prg_8k(0, b(6));
            self.banks.map_prg_8k(1, b(7));
            self.banks.map_prg_8k(2, b(15));
        }

        self.banks.map_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        if self.select & 0x20 != 0 {
            // 1 KiB mode: registers 8 and 9 fill the odd kilobytes of the first two windows.
            self.banks.map_chr_1k(high, b(0));
            self.banks.map_chr_1k(high + 1, b(8));
            self.banks.map_chr_1k(high + 2, b(1));
            self.banks.map_chr_1k(high + 3, b(9));
        } else {
            self.banks.map_chr_2k(high, b(0) >> 1);
            self.banks.map_chr_2k(high + 2, b(1) >> 1);
        }

        for i in 0..4 {
            self.banks.map_chr_1k(low + i, b(2 + i));
        }
    }

    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }
}

impl Mapper for Rambo1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_i32(self.cycle_divider);
        writer.write_bool(self.cycle_mode);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.cycle_divider = reader.read_i32()?;
        self.cycle_mode = reader.read_bool()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        64
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 16];
        self.select = 0;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.cycle_divider = 0;
        self.cycle_mode = false;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.a12.reset();
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xE001 {
            0x8000 => {
                self.select = value;
                self.update_banks();
            }
            0x8001 => {
                self.bank[usize::from(self.select & 0x0F)] = value;
                self.update_banks();
            }
            0xA000 => memory.set_mirroring(if value & 0x01 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            0xC000 => self.irq_latch = value,
            0xC001 => {
                self.cycle_mode = value & 0x01 != 0;
                self.cycle_divider = 0;
                self.irq_reload = true;
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            0xE001 => self.irq_enabled = true,
            _ => {}
        }
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if !self.cycle_mode && self.a12.rose(address, clock) {
            self.clock_irq_counter();
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.cycle_mode {
            return;
        }

        self.cycle_divider += 1;
        if self.cycle_divider < 4 {
            return;
        }

        self.cycle_divider = 0;
        self.clock_irq_counter();
    }
}
