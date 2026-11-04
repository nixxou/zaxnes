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
/// **Where the register is depends on the configuration (Q-117).** nesdev ("UNROM 512") gives it
/// as `D~[NCCP PPPP] A~[1... .... .... ....]` — all of $8000-$FFFF — and then: "For
/// self-flashable configurations, the address range shifts to `$C000-$FFFF`" with
/// `A~[11.. .... .... ....]`, $8000-$BFFF becoming the flash write window. A self-flashable
/// configuration is one without bus conflicts, since they "interfere with the self-flashing
/// procedure": submapper 0 with the battery bit, and submappers 1, 3 and 4. That is the same set
/// `registry::has_bus_conflicts` already decides, so one load-time value says both things.
///
/// MR-30-5 falls out of it: submapper 4's LED register is at `D~[BYGR bygr]
/// A~[10.. .... .... ....]`, $8000-$BFFF, which is where that board's flash window is too, and
/// its bank register is at $C000-$FFFF like every other self-flashable board's. Nothing in the
/// engine lights, so an LED write is just a write the latch does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnRom512 {
    banks: BankWindows,
    mirroring_control: UnRom512Mirroring,
    /// First CPU address the bank latch answers: $C000 on a self-flashable configuration,
    /// $8000 otherwise (Q-117). Decided at load, and it decides the flash window with it —
    /// everything in $8000-$FFFF *below* this address goes to the flash chip, which on a
    /// non-flashable board is nothing at all. One compare in `write_register` for both
    /// questions, and the field replaces MF-2's `can_flash` and `has_led_register` bools.
    latch_base: u16,
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
            // Q-117, MR-30-4 and MR-BUS-1 are interlocked: a board whose ROM goes on driving the
            // data bus during a write cannot run the command sequences at all, and it is exactly
            // the boards that can which move their latch up to $C000.
            latch_base: if super::super::registry::has_bus_conflicts(30, header) {
                0x8000
            } else {
                0xC000
            },
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

    /// The address the flash chip sees for a CPU write in its window: "bank × $4000 +
    /// (address & $3FFF)", the latch driving the chip's upper address lines (Q-117, R0's answer).
    const fn chip_address(&self, address: u16) -> u32 {
        (self.register & 0x1F) as u32 * 0x4000 + (address & 0x3FFF) as u32
    }

    /// The two SST39SF040 command addresses, on the **chip's** address lines (Q-117).
    const UNLOCK_5555: u32 = 0x5555;
    const UNLOCK_2AAA: u32 = 0x2AAA;

    /// Advances the SST39SF040 command state machine on a CPU write to the flash window, and
    /// runs the command when its last byte lands (MR-30-4, Q-117).
    ///
    /// nesdev ("UNROM 512") gives the two sequences with their bank writes, which is what makes
    /// them work at all:
    ///
    /// * erase a 4 KiB sector — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`;
    ///   `$C000:$01, $9555:$80`; `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`;
    ///   `$C000:BANK, ADDR:$30`;
    /// * program a byte — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`;
    ///   `$C000:$01, $9555:$A0`; `$C000:BANK, ADDR:DATA`.
    ///
    /// Each unlock write is preceded by the bank that puts it on the chip address the chip
    /// expects: bank 1 at $9555 is chip $5555, bank 0 at $AAAA is chip $2AAA. **So the chip
    /// recognises its sequences on chip addresses, not on CPU addresses** — this is the
    /// substance of R0's answer to Q-117, and it is why the state machine below matches
    /// [`Self::chip_address`]. The bank writes land at $C000, outside the window, so they never
    /// perturb the state machine; what MF-2 had to reason about — the latch moving under the
    /// chip's feet — simply does not happen.
    ///
    /// A side effect a real driver has to live with: `$C000:$01` and `$C000:$00` also set the
    /// CHR-RAM bank (bits 5-6) to 0 and the arrangement bit (bit 7) to 0, because they are
    /// writes to the whole register. That is the hardware.
    ///
    /// Either command completes before the CPU can look, so the verification reads the page
    /// describes ("read the written location twice until you get $FF twice", and the same for
    /// the data) succeed at once and the board never has to answer a read with status — which
    /// is what keeps `rom_windows` a plain window and this off every other board's read path.
    /// Persistence of a reprogrammed ROM stays out of scope (Q-117, to be settled with
    /// REQ-CORE-095): the flash is changed in memory and a reload loses it.
    fn step_flash(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match (self.flash_step, self.chip_address(address), value) {
            (0 | 3, Self::UNLOCK_5555, 0xAA) | (1 | 4, Self::UNLOCK_2AAA, 0x55) => {
                self.flash_step += 1;
            }
            (2, Self::UNLOCK_5555, 0x80) => {
                self.flash_erasing = true;
                self.flash_step = 3;
            }
            (2, Self::UNLOCK_5555, 0xA0) => {
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
            // Anything else aborts the sequence.
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

        // Q-117. The latch and the flash chip split $8000-$FFFF between them, at `latch_base`:
        // on a self-flashable configuration the chip has $8000-$BFFF and the latch $C000-$FFFF,
        // and on every other one the latch has the lot and there is no chip to write. One
        // compare decides, and no other board reaches this line.
        if address < self.latch_base {
            self.step_flash(memory, address, value);
            return;
        }

        self.register = value;
        self.update_banks(memory);
    }
}
