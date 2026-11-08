use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 18, Jaleco SS88006: nibble-wide bank registers and a CPU-cycle IRQ counter of
/// selectable width (REQ-CORE-100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoSs88006 {
    banks: BankWindows,
    prg: [u8; 3],
    chr: [u8; 8],
    irq_latch: i32,
    irq_counter: i32,
    irq_width: i32,
    irq_enabled: bool,
    irq_pending: bool,
    /// `$9002`, the PRG-RAM protect register (MF-LEG-1 step 4, report entry 18-2; **Q-175**).
    ///
    /// nesdev ("INES Mapper 018") gives it as the MMC3's `$A001` under another address:
    ///
    /// ```text
    /// 7  bit  0
    /// ---------
    /// .... ..WR
    ///        ||
    ///        |+- PRG RAM chip enable (0: disable; 1: enable)
    ///        +-- Write protection (0: deny writes; 1: allow writes)
    /// ```
    ///
    /// **Powered on at `$03`, both permitted**, which is Q-175 section 1: the page says nothing
    /// about power-on, puNES and this build's own MMC3 come up permitted (MR-4-5), and Nestopia
    /// comes up with **both denied** — which only works if every mapper-18 game writes the
    /// register before touching its save. Permitted is the reading under which a game that never
    /// writes `$9002` keeps its save RAM, and the entry asks for it by name.
    ///
    /// **Not in the savestate**, for Q-174 section 2's reason in the same words: a byte written
    /// here would leave every mapper-18 state written before this step one short of the board's
    /// block, and there is no per-board length to read it tolerantly against. A state load
    /// therefore returns the register to `$03`, `NesCore::restore_state` building a fresh
    /// `Cartridge` from the ROM before it reads the block — and `$03` is the permissive value, so
    /// the one direction a load can move the board in is the one that cannot lose a save.
    /// Q-175 section 2 holds the choice.
    prg_ram_protect: u8,
}

fn set_nibble(register: &mut u8, half: u16, nibble: u8) {
    *register = if half == 0 {
        (*register & 0xF0) | nibble
    } else {
        (*register & 0x0F) | (nibble << 4)
    };
}

impl JalecoSs88006 {
    /// `$9002` at power-on: the chip enabled and writes allowed (Q-175 section 1).
    const PRG_RAM_PROTECT_AT_POWER_ON: u8 = 0x03;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 1, 2],
            chr: [0; 8],
            irq_latch: 0,
            irq_counter: 0,
            irq_width: 16,
            irq_enabled: false,
            irq_pending: false,
            prg_ram_protect: Self::PRG_RAM_PROTECT_AT_POWER_ON,
        }
    }

    fn write_irq_control(&mut self, memory: &mut CartridgeMemory, index: u16, value: u8) {
        match index {
            0 => {
                self.irq_counter = self.irq_latch;
                self.irq_pending = false;
            }
            1 => {
                self.irq_enabled = value & 0x01 != 0;
                self.irq_pending = false;
                self.irq_width = if value & 0x08 != 0 {
                    4
                } else if value & 0x04 != 0 {
                    8
                } else if value & 0x02 != 0 {
                    12
                } else {
                    16
                };
            }
            2 => memory.set_mirroring(match value & 0x03 {
                0 => NametableMirroring::Horizontal,
                1 => NametableMirroring::Vertical,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            _ => {}
        }
    }

    fn update_banks(&mut self) {
        for slot in 0..3 {
            self.banks.map_prg_8k(slot, i32::from(self.prg[slot]));
        }

        self.banks.map_prg_8k(3, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for JalecoSs88006 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_i32(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_i32(self.irq_width);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.irq_latch = reader.read_i32()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_width = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        18
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `$9002` bit 0, "PRG RAM chip enable (0: disable; 1: enable)" — the page's own words, and
    /// "same as MMC3" for the pair (see [`JalecoSs88006::prg_ram_protect`]).
    fn prg_ram_enabled(&self) -> bool {
        self.prg_ram_protect & 0x01 != 0
    }

    /// `$9002` bit 1, "Write protection (0: deny writes; 1: allow writes)", **and** bit 0: a
    /// disabled chip accepts no writes either. Same shape as `Mmc3Board::prg_ram_writable`, whose
    /// bit 6 is the MMC3's inverted equivalent of this bit.
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_protect & 0x03 == 0x03
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0, 1, 2];
        self.chr = [0; 8];
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_width = 16;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.prg_ram_protect = Self::PRG_RAM_PROTECT_AT_POWER_ON;
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let group = (address >> 12) & 0x07;
        let index = address & 0x03;
        let nibble = value & 0x0F;
        match group {
            // $8000-$9003: the three PRG banks.
            0 => {
                set_nibble(&mut self.prg[usize::from(index >> 1)], index & 1, nibble);
                self.update_banks();
            }
            1 => {
                if index < 2 {
                    set_nibble(&mut self.prg[2], index, nibble);
                    self.update_banks();
                } else if index == 2 {
                    // $9002, the PRG-RAM protect register. $9003 stays undecoded: the page lists
                    // no register there (MF-LEG-1 step 4, report entry 18-2).
                    self.prg_ram_protect = value & 0x03;
                }
            }
            // $A000-$D003: the eight CHR banks.
            2..=5 => {
                set_nibble(
                    &mut self.chr[usize::from((group - 2) * 2 + (index >> 1))],
                    index & 1,
                    nibble,
                );
                self.update_banks();
            }
            // $E000-$E003: the four nibbles of the IRQ latch.
            6 => {
                let shift = u32::from(index) * 4;
                self.irq_latch = (self.irq_latch & !(0x0F << shift)) | (i32::from(nibble) << shift);
            }
            _ => self.write_irq_control(memory, index, value),
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }

        let mask = if self.irq_width == 16 {
            0xFFFF
        } else {
            (1 << self.irq_width) - 1
        };
        let counted = self.irq_counter & mask;
        if counted == 0 {
            self.irq_pending = true;
            self.irq_counter = (self.irq_counter & !mask) | mask;
            return;
        }

        self.irq_counter = (self.irq_counter & !mask) | (counted - 1);
    }
}
