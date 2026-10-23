//! The discrete-logic boards: one latch, a handful of banks (REQ-CORE-100).

use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::NametableMirroring;

/// iNES mapper 2, UxROM: a write in $8000-$FFFF selects the 16 KiB bank at $8000, the last
/// bank is fixed at $C000, CHR is not switched (nesdev, "UxROM").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UxRom {
    banks: BankWindows,
    bank: u8,
}

impl UxRom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            bank: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.bank));
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_8k(0);
    }
}

impl Mapper for UxRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        2
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = 0;
        self.update_banks();
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.bank = value;
            self.update_banks();
        }
    }
}

/// iNES mapper 3, CNROM: a write in $8000-$FFFF selects the 8 KiB CHR bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CnRom {
    banks: BankWindows,
    bank: u8,
}

impl CnRom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            bank: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(0);
        self.banks.map_chr_8k(i32::from(self.bank));
    }
}

impl Mapper for CnRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        3
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = 0;
        self.update_banks();
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.bank = value;
            self.update_banks();
        }
    }
}

/// iNES mapper 7, AxROM: 32 KiB PRG bank and single-screen page from one register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxRom {
    banks: BankWindows,
    register: u8,
}

impl AxRom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_32k(i32::from(self.register & 0x0F));
        self.banks.map_chr_8k(0);
        memory.set_mirroring(if self.register & 0x10 != 0 {
            NametableMirroring::SingleScreenB
        } else {
            NametableMirroring::SingleScreenA
        });
    }
}

impl Mapper for AxRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        7
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.register = value;
            self.update_banks(memory);
        }
    }
}

/// iNES mapper 66, GxROM: 32 KiB PRG bank in bits 4-5, 8 KiB CHR bank in bits 0-1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GxRom {
    banks: BankWindows,
    register: u8,
}

impl GxRom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks
            .map_prg_32k(i32::from((self.register >> 4) & 0x03));
        self.banks.map_chr_8k(i32::from(self.register & 0x03));
    }
}

impl Mapper for GxRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        66
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.register = value;
            self.update_banks();
        }
    }
}

/// iNES mapper 13, CPROM: 16 KiB of CHR-RAM, the upper 4 KiB banked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpRom {
    banks: BankWindows,
    bank: u8,
}

impl CpRom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            bank: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(0);
        self.banks.map_chr_4k(0, 0);
        self.banks.map_chr_4k(4, i32::from(self.bank));
    }
}

impl Mapper for CpRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        13
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = 0;
        self.update_banks();
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.bank = value & 0x03;
            self.update_banks();
        }
    }
}

/// iNES mapper 11, Color Dreams: 32 KiB PRG in bits 0-1, 8 KiB CHR in bits 4-7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorDreams {
    banks: BankWindows,
    register: u8,
}

impl ColorDreams {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(i32::from(self.register & 0x03));
        self.banks.map_chr_8k(i32::from(self.register >> 4));
    }
}

impl Mapper for ColorDreams {
    banked!();

    fn mapper_number(&self) -> i32 {
        11
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.register = value;
            self.update_banks();
        }
    }
}

/// iNES mapper 71, Codemasters: 16 KiB PRG at $C000-$FFFF writes, optional single-screen
/// select at $9000 (Fire Hawk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codemasters {
    banks: BankWindows,
    bank: u8,
    single_screen: bool,
    single_screen_b: bool,
}

impl Codemasters {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            bank: 0,
            single_screen: false,
            single_screen_b: false,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_16k(0, i32::from(self.bank));
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_8k(0);
        if self.single_screen {
            memory.set_mirroring(if self.single_screen_b {
                NametableMirroring::SingleScreenB
            } else {
                NametableMirroring::SingleScreenA
            });
        }
    }
}

impl Mapper for Codemasters {
    banked!();

    fn mapper_number(&self) -> i32 {
        71
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.bank = 0;
        self.single_screen = false;
        self.single_screen_b = false;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & 0xF000 == 0x9000 {
            self.single_screen = true;
            self.single_screen_b = value & 0x10 != 0;
        } else if address >= 0xC000 {
            self.bank = value;
        } else {
            return;
        }

        self.update_banks(memory);
    }
}

/// iNES mapper 30, UNROM 512: 16 KiB PRG, 8 KiB CHR-RAM bank and (four-screen header)
/// single-screen select from one register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnRom512 {
    banks: BankWindows,
    mapper_controlled_mirroring: bool,
    register: u8,
}

impl UnRom512 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            mapper_controlled_mirroring: memory.rom().header().mirroring
                == NametableMirroring::FourScreen,
            register: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_16k(0, i32::from(self.register & 0x1F));
        self.banks.map_prg_16k(2, -1);
        self.banks
            .map_chr_8k(i32::from((self.register >> 5) & 0x03));
        if self.mapper_controlled_mirroring {
            memory.set_mirroring(if self.register & 0x80 != 0 {
                NametableMirroring::SingleScreenB
            } else {
                NametableMirroring::SingleScreenA
            });
        }
    }
}

impl Mapper for UnRom512 {
    banked!();

    fn mapper_number(&self) -> i32 {
        30
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.register = value;
            self.update_banks(memory);
        }
    }
}
