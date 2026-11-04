//! The discrete-logic boards: one latch, a handful of banks (REQ-CORE-100).

use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.bank);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.bank = reader.read_u8()?;
        Ok(())
    }

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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.bank);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.bank = reader.read_u8()?;
        Ok(())
    }

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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

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
///
/// The board page gives the RAM as "16 KiB RAM (in two 8KiB RAMs)" (MR-13-1) and the nametable
/// arrangement as "Vertical", with no solder pad among its characteristics, so the arrangement
/// is wiring and not a header default (MR-13-2). *Videomation* is the only cartridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpRom {
    banks: BankWindows,
    bank: u8,
}

impl CpRom {
    /// MR-13-1: the CHR-RAM the board carries, four 4 KiB banks, whatever the header says.
    /// Settled at load by `registry::board_memory_sizes`.
    pub(crate) const CHR_RAM_SIZE: usize = 0x4000;

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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.bank);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.bank = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        13
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.bank = 0;
        // MR-13-2: the arrangement is wired, so a mis-headered dump does not get to choose.
        memory.set_mirroring(NametableMirroring::Vertical);
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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.bank);
        writer.write_bool(self.single_screen);
        writer.write_bool(self.single_screen_b);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.bank = reader.read_u8()?;
        self.single_screen = reader.read_bool()?;
        self.single_screen_b = reader.read_bool()?;
        Ok(())
    }

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

/// What bit 7 of the UNROM 512 register does to the nametable arrangement.
///
/// nesdev ("UNROM 512") decodes header byte 6 bits 0 and 3 **together** — "`%....0..0`
/// Vertical, `%....0..1` Horizontal, `%....1..0` 1-Screen, switchable, `%....1..1` 4-Screen,
/// cartridge VRAM" — where iNES reads bit 3 alone (MR-30-1); and NES 2.0 submapper 3 is "No
/// bus conflicts, mapper-controlled horizontal/vertical nametable arrangement" (MR-30-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnRom512Mirroring {
    /// The header's arrangement is wired: vertical, horizontal or genuine four-screen. Bit 7
    /// does nothing.
    Fixed,
    /// Byte 6 `%....1..0`: bit 7 picks screen A or screen B.
    OneScreen,
    /// Submapper 3: bit 7 picks horizontal or vertical.
    HorizontalVertical,
}

/// iNES mapper 30, UNROM 512: 16 KiB PRG, 8 KiB CHR-RAM bank and a nametable select from one
/// register.
///
/// NES 2.0 submapper 4 is "No bus conflicts, an additional register controls LEDs inside the
/// cartridge shell", at `D~[BYGR bygr] A~[10.. .... .... ....]`, i.e. $8000-$BFFF — so on that
/// board the bank register is only at $C000-$FFFF (MR-30-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnRom512 {
    banks: BankWindows,
    mirroring_control: UnRom512Mirroring,
    /// Submapper 4: $8000-$BFFF is the LED register, not the bank register.
    has_led_register: bool,
    register: u8,
}

impl UnRom512 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let submapper = if header.is_nes20 { header.submapper } else { 0 };
        let mirroring_control = if submapper == 3 {
            UnRom512Mirroring::HorizontalVertical
        } else if header.byte6 & 0x09 == 0x08 {
            UnRom512Mirroring::OneScreen
        } else {
            // Includes `%....1..1`, which is four-screen with cartridge VRAM and not
            // mapper-controlled at all (MR-30-1). The VRAM itself is MR-30-6, out of lot MF-1.
            UnRom512Mirroring::Fixed
        };
        Self {
            banks: windows_for(memory),
            mirroring_control,
            has_led_register: submapper == 4,
            register: 0,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_16k(0, i32::from(self.register & 0x1F));
        self.banks.map_prg_16k(2, -1);
        self.banks
            .map_chr_8k(i32::from((self.register >> 5) & 0x03));
        let high = self.register & 0x80 != 0;
        match self.mirroring_control {
            UnRom512Mirroring::Fixed => {}
            UnRom512Mirroring::OneScreen => memory.set_mirroring(if high {
                NametableMirroring::SingleScreenB
            } else {
                NametableMirroring::SingleScreenA
            }),
            UnRom512Mirroring::HorizontalVertical => memory.set_mirroring(if high {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
        }
    }
}

impl Mapper for UnRom512 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        30
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        // MR-30-5: on a submapper-4 board $8000-$BFFF is the LED register, which drives four
        // colour pairs inside the cartridge shell and no bank. Nothing in the engine lights,
        // so the write is simply not a bank write; the bank register is at $C000-$FFFF.
        let first = if self.has_led_register {
            0xC000
        } else {
            0x8000
        };
        if address >= first {
            self.register = value;
            self.update_banks(memory);
        }
    }
}
