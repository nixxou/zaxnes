//! Multicarts and boards with registers below $8000 (REQ-CORE-100).

use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

const fn horizontal_if(bit: bool) -> NametableMirroring {
    if bit {
        NametableMirroring::Horizontal
    } else {
        NametableMirroring::Vertical
    }
}

/// iNES mapper 34 (BNROM and NINA-001): registers at $7FFD-$7FFF and a PRG latch at $8000.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapper034 {
    banks: BankWindows,
    prg_bank: u8,
    chr_low: u8,
    chr_high: u8,
}

impl Mapper034 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg_bank: 0,
            chr_low: 0,
            chr_high: 1,
        }
    }

    fn update_banks(&mut self, memory: &CartridgeMemory) {
        self.banks.map_prg_32k(i32::from(self.prg_bank));
        if memory.rom().chr_rom().is_empty() {
            self.banks.map_chr_8k(0);
        } else {
            self.banks.map_chr_4k(0, i32::from(self.chr_low));
            self.banks.map_chr_4k(4, i32::from(self.chr_high));
        }
    }
}

impl Mapper for Mapper034 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.prg_bank);
        writer.write_u8(self.chr_low);
        writer.write_u8(self.chr_high);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg_bank = reader.read_u8()?;
        self.chr_low = reader.read_u8()?;
        self.chr_high = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        34
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.prg_bank = 0;
        self.chr_low = 0;
        self.chr_high = 1;
        self.update_banks(memory);
    }

    // The NINA-001 registers also write through to the PRG-RAM underneath.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        match address {
            0x7FFD => self.prg_bank = value,
            0x7FFE => self.chr_low = value,
            0x7FFF => self.chr_high = value,
            _ => return false,
        }

        self.update_banks(memory);
        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.prg_bank = value;
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 15, the 100-in-1 Contra Function 16 multicart: the address low bits pick the
/// PRG mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapper015 {
    banks: BankWindows,
    mode: i32,
    value: u8,
}

impl Mapper015 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            mode: 0,
            value: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        let bank = i32::from(self.value & 0x3F);
        let half = i32::from((self.value >> 7) & 0x01);
        match self.mode {
            0 => {
                self.banks.map_prg_16k(0, bank);
                self.banks.map_prg_16k(2, bank + 1);
            }
            1 => {
                self.banks.map_prg_16k(0, bank);
                // -1 counts back from the end: the last 16 KiB bank of the image.
                self.banks.map_prg_16k(2, -1);
            }
            2 => {
                for slot in 0..BankWindows::PRG_WINDOWS {
                    self.banks.map_prg_8k(slot, bank * 2 + half);
                }
            }
            _ => {
                self.banks.map_prg_16k(0, bank);
                self.banks.map_prg_16k(2, bank);
            }
        }

        self.banks.map_chr_8k(0);
        memory.set_mirroring(horizontal_if(self.value & 0x40 != 0));
    }
}

impl Mapper for Mapper015 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.mode);
        writer.write_u8(self.value);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mode = reader.read_i32()?;
        self.value = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        15
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mode = 0;
        self.value = 0;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.mode = i32::from(address & 0x03);
            self.value = value;
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 225, the 64-in-1 style multicart: everything is in the address of the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Multicart225 {
    banks: BankWindows,
    latch: i32,
}

impl Multicart225 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            latch: 0x8000,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        let block = (self.latch >> 14) & 0x01;
        let chr = (self.latch & 0x3F) | (block << 6);
        let prg = ((self.latch >> 6) & 0x3F) | (block << 6);
        if self.latch & 0x1000 != 0 {
            self.banks.map_prg_16k(0, prg);
            self.banks.map_prg_16k(2, prg);
        } else {
            self.banks.map_prg_32k(prg >> 1);
        }

        self.banks.map_chr_8k(chr);
        memory.set_mirroring(horizontal_if(self.latch & 0x2000 != 0));
    }
}

impl Mapper for Multicart225 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.latch);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.latch = reader.read_i32()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        225
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.latch = 0x8000;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, _value: u8) {
        if address >= 0x8000 {
            self.latch = i32::from(address);
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 41, Caltron 6-in-1: outer register at $6000-$67FF, inner CHR at $8000.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caltron6In1 {
    banks: BankWindows,
    outer: u8,
    chr_low: u8,
}

impl Caltron6In1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            outer: 0,
            chr_low: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_32k(i32::from(self.outer & 0x03));
        self.banks
            .map_chr_8k(i32::from((((self.outer >> 3) & 0x03) << 2) | self.chr_low));
        memory.set_mirroring(horizontal_if(self.outer & 0x04 != 0));
    }
}

impl Mapper for Caltron6In1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.outer);
        writer.write_u8(self.chr_low);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.outer = reader.read_u8()?;
        self.chr_low = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        41
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.outer = 0;
        self.chr_low = 0;
        self.update_banks(memory);
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        _value: u8,
    ) -> bool {
        if (0x6000..=0x67FF).contains(&address) {
            self.outer = (address & 0x1F) as u8; // Masked to 5 bits.
            self.update_banks(memory);
            return true;
        }

        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.chr_low = value & 0x03;
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 228, Action 52: everything is in the address of the write, CHR low bits in
/// the data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action52 {
    banks: BankWindows,
    latch: i32,
    value: u8,
}

impl Action52 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            latch: 0x8000,
            value: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        let mut chip = (self.latch >> 11) & 0x03;
        if chip == 3 {
            chip = 2;
        }

        let prg = ((self.latch >> 6) & 0x1F) | (chip << 5);
        if self.latch & 0x20 != 0 {
            self.banks.map_prg_16k(0, prg);
            self.banks.map_prg_16k(2, prg);
        } else {
            self.banks.map_prg_32k(prg >> 1);
        }

        self.banks
            .map_chr_8k(((self.latch & 0x0F) << 2) | i32::from(self.value & 0x03));
        memory.set_mirroring(horizontal_if(self.latch & 0x2000 != 0));
    }
}

impl Mapper for Action52 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.latch);
        writer.write_u8(self.value);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.latch = reader.read_i32()?;
        self.value = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        228
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.latch = 0x8000;
        self.value = 0;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.latch = i32::from(address);
            self.value = value;
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 28, Action 53: register select at $5000, four registers at $8000.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action53 {
    banks: BankWindows,
    select: i32,
    chr_bank: u8,
    inner_prg: u8,
    mode: u8,
    outer_prg: u8,
}

impl Action53 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            select: 0,
            chr_bank: 0,
            inner_prg: 15,
            mode: 0,
            outer_prg: 63,
        }
    }

    fn take_mirroring_bit(&mut self, value: u8) {
        if self.mode & 0x02 == 0 {
            self.mode = (self.mode & !0x01) | ((value >> 4) & 0x01);
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_chr_8k(i32::from(self.chr_bank));
        memory.set_mirroring(match self.mode & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::SingleScreenB,
            2 => NametableMirroring::Vertical,
            _ => NametableMirroring::Horizontal,
        });

        // Bits 4 to 5 say how large the selected game is: the mask is the number of 16 KiB
        // banks inside it minus one. Q-059: a bank number past the end of the image wraps
        // modulo the number of banks, as on every other board.
        let m = (2i32 << ((self.mode >> 4) & 0x03)) - 1;
        let outer = i32::from(self.outer_prg) * 2;
        let inner = i32::from(self.inner_prg);
        match (self.mode >> 2) & 0x03 {
            0 | 1 => {
                let bank32 = (outer & !m) | ((inner * 2) & m);
                self.banks.map_prg_16k(0, bank32);
                self.banks.map_prg_16k(2, bank32 + 1);
            }
            2 => {
                self.banks.map_prg_16k(0, outer);
                self.banks.map_prg_16k(2, (outer & !m) | (inner & m));
            }
            _ => {
                self.banks.map_prg_16k(0, (outer & !m) | (inner & m));
                self.banks.map_prg_16k(2, outer + 1);
            }
        }
    }
}

impl Mapper for Action53 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.select);
        writer.write_u8(self.chr_bank);
        writer.write_u8(self.inner_prg);
        writer.write_u8(self.mode);
        writer.write_u8(self.outer_prg);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.select = reader.read_i32()?;
        self.chr_bank = reader.read_u8()?;
        self.inner_prg = reader.read_u8()?;
        self.mode = reader.read_u8()?;
        self.outer_prg = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        28
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.select = 0;
        self.chr_bank = 0;
        self.inner_prg = 15;
        self.mode = 0;
        self.outer_prg = 63;
        self.update_banks(memory);
    }

    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if (0x5000..=0x5FFF).contains(&address) {
            self.select = i32::from(((value & 0x80) >> 6) | (value & 0x01));
            return true;
        }

        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match self.select {
            0 => {
                self.chr_bank = value & 0x03;
                self.take_mirroring_bit(value);
            }
            1 => {
                self.inner_prg = value & 0x0F;
                self.take_mirroring_bit(value);
            }
            2 => self.mode = value & 0x3F,
            _ => self.outer_prg = value & 0x3F,
        }

        self.update_banks(memory);
    }
}

/// iNES mappers 87 and 101, Jaleco J87: a CHR register at $6000-$7FFF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoJ87 {
    banks: BankWindows,
    mapper_number: i32,
    swap_bits: bool,
    register: u8,
}

impl JalecoJ87 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            mapper_number,
            swap_bits: mapper_number == 87,
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(0);
        self.banks.map_chr_8k(i32::from(if self.swap_bits {
            ((self.register & 0x01) << 1) | ((self.register >> 1) & 0x01)
        } else {
            self.register & 0x03
        }));
    }
}

impl Mapper for JalecoJ87 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    // Mapper 87 decodes $6000-$7FFF only; mapper 101 decodes bits 13 and 14 of the address,
    // which adds $E000-$FFFF (R2-003).
    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        let selected = if self.swap_bits {
            (0x6000..=0x7FFF).contains(&address)
        } else {
            address & 0x6000 == 0x6000
        };
        if selected {
            self.register = value;
            self.update_banks();
        }

        selected
    }
}
