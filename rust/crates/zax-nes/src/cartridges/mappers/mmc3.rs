use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 4, MMC3 and MMC6 (REQ-CORE-100, REQ-CORE-114).
///
/// Eight bank registers behind $8000/$8001, an A12-clocked scanline counter behind
/// $C000-$E001 (nesdev, "MMC3"), PRG-RAM protection at $A001. MMC6 (NES 2.0 submapper 1)
/// has 1 KiB of RAM in two halves with their own enable bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3 {
    banks: BankWindows,
    a12: A12Watcher,
    bank: [u8; 8],
    is_mmc6: bool,
    four_screen: bool,
    select: u8,
    protect: u8,
    irq_latch: u8,
    irq_counter: i32,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Mmc3 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            banks: windows_for(memory),
            a12: A12Watcher::new(),
            bank: [0; 8],
            is_mmc6: header.is_nes20 && header.submapper == 1,
            four_screen: header.mirroring == NametableMirroring::FourScreen,
            select: 0,
            protect: 0x80,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    fn mmc6_offset(memory: &CartridgeMemory, address: u16) -> usize {
        usize::from(address & 0x03FF) % memory.prg_ram().len()
    }

    fn mmc6_half_readable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x40 } else { 0x10 } != 0
    }

    fn mmc6_half_writable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x80 } else { 0x20 } != 0
    }

    fn update_banks(&mut self) {
        if self.select & 0x40 != 0 {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.bank[6]));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.bank[6]));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.bank[7]));
        self.banks.map_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        self.banks.map_chr_2k(high, i32::from(self.bank[0] >> 1));
        self.banks
            .map_chr_2k(high + 2, i32::from(self.bank[1] >> 1));
        for i in 0..4 {
            self.banks.map_chr_1k(low + i, i32::from(self.bank[2 + i]));
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

impl Mapper for Mmc3 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
        writer.write_u8(self.protect);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        self.protect = reader.read_u8()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        // The windows again from the registers, over the PRG-ROM loaded: a state of the same
        // game made with a PRG-ROM of another size (a widescreen hack that added banks, its
        // fixed ones moved to the end) maps the fixed banks of this one.
        self.update_banks();
        Ok(())
    }

    fn chr_view<'m>(
        &self,
        _memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its fetches have side effects or another source: the PPU takes the full path.
        None
    }

    fn chr_peek_view<'m>(
        &self,
        memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its peek is the default one, through its bank windows: only the fetches clock the
        // IRQ counter.
        let chr_rom = memory.rom().chr_rom();
        let chr = if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        };
        self.banks()
            .map(|banks| crate::cartridges::ChrView::new(chr, banks))
    }

    fn mapper_number(&self) -> i32 {
        4
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// The line only rises when the counter reaches 0 with IRQs on, and the counter only
    /// moves on an A12 rise, which come at least `MIN_RISE_INTERVAL` dots apart.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.irq_pending || !self.irq_enabled {
            return Some(i64::MAX);
        }

        let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
            1 + i64::from(self.irq_latch)
        } else {
            i64::from(self.irq_counter)
        };
        Some((clocks_to_zero - 1) * A12Watcher::MIN_RISE_INTERVAL)
    }

    fn prg_ram_enabled(&self) -> bool {
        if self.is_mmc6 {
            self.select & 0x20 != 0
        } else {
            self.protect & 0x80 != 0
        }
    }

    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_enabled() && self.protect & 0x40 == 0
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 8];
        self.bank[7] = 1;
        self.select = 0;
        // The power-on value of $A001 is not documented; PRG-RAM enabled and writable is the
        // only choice that lets a game, or blargg's MMC3 test ROMs which never write $A001,
        // use the RAM out of a cold start.
        self.protect = 0x80;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.a12.reset();
        self.update_banks();
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return None;
        }

        if address < 0x7000 || !self.prg_ram_enabled() {
            return Some((address >= 0x7000).then_some(0));
        }

        Some(Some(if self.mmc6_half_readable(address) {
            memory.prg_ram()[Self::mmc6_offset(memory, address)]
        } else {
            0
        }))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if address >= 0x7000 && self.prg_ram_enabled() && self.mmc6_half_writable(address) {
            let offset = Self::mmc6_offset(memory, address);
            memory.prg_ram_mut()[offset] = value;
        }

        true
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
                self.bank[usize::from(self.select & 0x07)] = value;
                self.update_banks();
            }
            0xA000 => {
                if !self.four_screen {
                    memory.set_mirroring(if value & 0x01 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                }
            }
            0xA001 => self.protect = value,
            0xC000 => self.irq_latch = value,
            0xC001 => {
                self.irq_counter = 0;
                self.irq_reload = true;
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            _ => self.irq_enabled = true,
        }
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.a12.rose(address, clock) {
            self.clock_irq_counter();
        }
    }
}
