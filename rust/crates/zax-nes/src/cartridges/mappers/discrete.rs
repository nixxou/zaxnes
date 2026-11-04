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
    /// The header's arrangement is wired: vertical or horizontal. Bit 7 does nothing.
    Fixed,
    /// Byte 6 `%....1..0`: bit 7 picks screen A or screen B.
    OneScreen,
    /// Submapper 3: bit 7 picks horizontal or vertical.
    HorizontalVertical,
    /// Byte 6 `%....1..1`, "4-Screen with cartridge VRAM": the board serves all four
    /// nametables, and the scratch RAM at PPU $3000-$3EFF with them, out of the last 8 KiB of
    /// its CHR-RAM (MR-30-6). Bit 7 does nothing.
    FourScreenCartridgeVram,
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
    /// Whether the board can erase and program its own PRG-ROM (MR-30-4). Decided at load:
    /// submapper 2's bus conflicts "interfere with the self-flashing procedure", and a
    /// submapper-0 dump without the battery bit has those conflicts too (see
    /// [`super::super::registry::has_bus_conflicts`]).
    can_flash: bool,
    /// Where the board is in an SST39SF040 command sequence: 0 when none is open, then one
    /// step per byte of the prologue (MR-30-4).
    flash_step: u8,
    /// Whether the open sequence is the six-byte erase rather than the four-byte byte-write.
    flash_erasing: bool,
    /// What [`Mapper::nametable_page`] answers, precomputed at load:
    /// `SERVED_BY_CARTRIDGE` in the four-screen mode and `FOLLOW_MIRRORING` otherwise.
    ///
    /// A field and not a `match`, because `Board::nametable_page` is consulted **once per
    /// nametable fetch for every game**: an arm that branches grows that dispatch enough to
    /// push `Ppu2C02::read_nametable_as` out of line, and then every game pays a call per
    /// fetch. Measured: it took `read_pattern_planes` from 106 instructions to 122. See
    /// `docs/progress/MF-2.md`, step 7.
    nametable_answer: i32,
    /// First byte of CHR-RAM the four-screen mode serves nametables from, precomputed for the
    /// same reason as `nametable_answer`.
    cartridge_vram_base: usize,
    register: u8,
}

impl UnRom512 {
    /// Bytes of CHR-RAM the four-screen mode takes for its nametables and scratch RAM.
    const CARTRIDGE_VRAM_SIZE: usize = 8 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let submapper = if header.is_nes20 { header.submapper } else { 0 };
        let four_screen_vram =
            header.byte6 & 0x09 == 0x09 && memory.chr_ram().len() >= Self::CARTRIDGE_VRAM_SIZE;
        let mirroring_control = if submapper == 3 {
            UnRom512Mirroring::HorizontalVertical
        } else if header.byte6 & 0x09 == 0x08 {
            UnRom512Mirroring::OneScreen
        } else if four_screen_vram {
            UnRom512Mirroring::FourScreenCartridgeVram
        } else {
            UnRom512Mirroring::Fixed
        };
        Self {
            banks: windows_for(memory),
            mirroring_control,
            nametable_answer: if four_screen_vram {
                super::super::Cartridge::SERVED_BY_CARTRIDGE
            } else {
                super::super::Cartridge::FOLLOW_MIRRORING
            },
            cartridge_vram_base: memory
                .chr_ram()
                .len()
                .saturating_sub(Self::CARTRIDGE_VRAM_SIZE),
            has_led_register: submapper == 4,
            // MR-30-4 and MR-BUS-1 are interlocked: a board whose ROM goes on driving the data
            // bus during a write cannot run the command sequences at all.
            can_flash: !super::super::registry::has_bus_conflicts(30, header),
            flash_step: 0,
            flash_erasing: false,
            register: 0,
        }
    }

    /// Offset in CHR-RAM of PPU `address`, which is $2000-$3EFF here: the four nametables fill
    /// the first 4 KiB of the window and the scratch RAM of $3000-$3EFF follows them, so the
    /// thirteen low bits of the address are the offset (nesdev, "UNROM 512").
    const fn cartridge_vram_offset(&self, address: u16) -> usize {
        self.cartridge_vram_base + (address & 0x1FFF) as usize
    }

    /// Advances the SST39SF040 command state machine on a CPU write to $8000-$FFFF, and runs
    /// the command when its last byte lands (MR-30-4).
    ///
    /// nesdev ("UNROM 512") gives the two sequences: erase a 4 KiB sector with $AA to $9555,
    /// $55 to $AAAA, $80 to $9555, $AA to $9555, $55 to $AAAA and $30 to an address in the
    /// sector; program a byte with $AA to $9555, $55 to $AAAA, $A0 to $9555 and the byte to its
    /// address. Either completes before the CPU can look, so the verification reads the page
    /// describes ("read the written location twice until you get $FF twice", and the same for
    /// the data) succeed at once and the board never has to answer a read with status — which
    /// is what keeps `rom_windows` a plain window and this fix off every other board's read
    /// path.
    ///
    /// **Q-117.** Every byte of a sequence is also a write to the mapper's own latch, so the
    /// 16 KiB window the chip sees when the last byte lands is the one the *previous* byte
    /// chose. The page says nothing about that interaction, and taken literally it puts every
    /// byte-write in bank 0 and every erase in bank 21. The chip is given the window in force,
    /// which is what the register diagram and the write cycle's own order imply, and the ticket
    /// records the two other readings. Persistence of a reprogrammed ROM is also out of scope
    /// there: the flash is changed in memory and a reload loses it.
    fn step_flash(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match (self.flash_step, address, value) {
            (0 | 3, 0x9555, 0xAA) | (1 | 4, 0xAAAA, 0x55) => self.flash_step += 1,
            (2, 0x9555, 0x80) => {
                self.flash_erasing = true;
                self.flash_step = 3;
            }
            (2, 0x9555, 0xA0) => {
                self.flash_erasing = false;
                self.flash_step = 6;
            }
            (5, _, 0x30) if self.flash_erasing => {
                self.erase_sector(memory, address);
                self.flash_step = 0;
            }
            (6, _, _) => {
                self.program_byte(memory, address, value);
                self.flash_step = 0;
            }
            // Anything else aborts the sequence. A plain bank write is the common case.
            _ => self.flash_step = 0,
        }
    }

    /// Erases the 4 KiB flash sector `address` falls in, to $FF.
    fn erase_sector(&self, memory: &mut CartridgeMemory, address: u16) {
        let Some(offset) = self.banks.prg_rom_offset(address) else {
            return;
        };

        let sector = offset & !0x0FFF;
        let rom = memory.prg_rom_mut();
        let end = (sector + 0x1000).min(rom.len());
        if sector < end {
            rom[sector..end].fill(0xFF);
        }
    }

    /// Programs one byte. Flash can only clear bits, so the byte written is ANDed with what is
    /// already there — a game that has not erased the sector first sees exactly that.
    fn program_byte(&self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let Some(offset) = self.banks.prg_rom_offset(address) else {
            return;
        };

        let rom = memory.prg_rom_mut();
        if let Some(byte) = rom.get_mut(offset) {
            *byte &= value;
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_prg_16k(0, i32::from(self.register & 0x1F));
        self.banks.map_prg_16k(2, -1);
        self.banks
            .map_chr_8k(i32::from((self.register >> 5) & 0x03));
        let high = self.register & 0x80 != 0;
        match self.mirroring_control {
            UnRom512Mirroring::Fixed | UnRom512Mirroring::FourScreenCartridgeVram => {}
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
        // Written last, so a mapper-30 state from before this lot loads with no flash command
        // open — which is the state of a board the moment a game is not in the middle of a
        // sequence, i.e. almost always (REQ-CORE-091).
        writer.write_u8(self.flash_step);
        writer.write_bool(self.flash_erasing);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        if reader.remaining() > 0 {
            self.flash_step = reader.read_u8()?;
            self.flash_erasing = reader.read_bool()?;
        } else {
            self.flash_step = 0;
            self.flash_erasing = false;
        }

        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        30
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        self.flash_step = 0;
        self.flash_erasing = false;
        self.update_banks(memory);
    }

    /// MR-30-6. All four nametables come from the board in the four-screen mode, and only
    /// there: every other arrangement leaves the layout to `memory.mirroring()` as before.
    fn nametable_page(&self, _table: i32) -> i32 {
        self.nametable_answer
    }

    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let offset = self.cartridge_vram_offset(address);
        memory.chr_ram().get(offset).copied().unwrap_or(0)
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let offset = self.cartridge_vram_offset(address);
        if let Some(byte) = memory.chr_ram_mut().get_mut(offset) {
            *byte = value;
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        // MR-30-4. The flash chip sees every write to $8000-$FFFF, whatever the mapper latch
        // does with the same write: the command sequences deliberately run through addresses
        // that also move the bank, which is why a flashing routine lives in the fixed window.
        // One `bool` test for a board that cannot flash, and no other board reaches this line.
        if self.can_flash {
            self.step_flash(memory, address, value);
        }

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
