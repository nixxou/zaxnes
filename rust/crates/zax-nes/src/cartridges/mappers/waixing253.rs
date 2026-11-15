//! iNES mapper 253, the 外星 (Wàixīng) VRC4-like board with 2 KiB of CHR-RAM beside its CHR-ROM
//! (lot MAPPERS-N). *Dragon Ball Z: Kyōshū! Saiyajin* (Chinese) and the Excitebike FDS
//! conversion use it.
//!
//! Read from Mesen2 `Core/NES/Mappers/Waixing/Mapper253.h` (the line numbers below are that
//! file's): eight 1 KiB CHR registers written a nibble at a time as on a VRC4, two 8 KiB PRG
//! registers at exactly `$8010` and `$A010`, the last two 8 KiB banks fixed, mirroring at
//! exactly `$9400`, and a VRC-style IRQ whose counter steps every 114 CPU cycles. CHR banks
//! 4 and 5 read the 2 KiB CHR-RAM unless the "force CHR-ROM" latch is set, which a write of
//! `$88` to the first CHR register sets and `$C8` clears. FCEUX's `mappers/253.cpp` has the same
//! decode.

use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Mapper 253.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waixing253 {
    banks: BankWindows,
    prg: [u8; 2],
    chr_low: [u8; 8],
    chr_high: [u8; 8],
    force_chr_rom: bool,
    irq_reload: u8,
    irq_counter: u8,
    irq_enabled: bool,
    irq_scaler: u16,
    irq_pending: bool,
    /// Which 1 KiB windows read the CHR-RAM, one bit each. Derived from the CHR registers by
    /// [`Waixing253::refresh`], so no state byte carries it.
    ram_windows: u8,
}

impl Waixing253 {
    /// "GetChrRamSize() { return 0x800; }" (`Mapper253.h:21`).
    pub const CHR_RAM_SIZE: usize = 0x800;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 0],
            chr_low: [0; 8],
            chr_high: [0; 8],
            force_chr_rom: false,
            irq_reload: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_scaler: 0,
            irq_pending: false,
            ram_windows: 0,
        }
    }

    /// `Mapper253.h:58-69` (`UpdateChr`) and `:27-28` (the fixed banks).
    fn refresh(&mut self) {
        self.banks.map_prg_8k(0, i32::from(self.prg[0]));
        self.banks.map_prg_8k(1, i32::from(self.prg[1]));
        self.banks.map_prg_8k(2, -2);
        self.banks.map_prg_8k(3, -1);
        self.ram_windows = 0;
        for slot in 0..8 {
            let page = i32::from(self.chr_low[slot]) | (i32::from(self.chr_high[slot]) << 8);
            if matches!(self.chr_low[slot], 4 | 5) && !self.force_chr_rom {
                self.ram_windows |= 1 << slot;
            }
            self.banks.map_chr_1k(slot, page);
        }
    }

    /// The CHR-RAM offset of a window that reads it: "page & 0x01" (`Mapper253.h:64`).
    fn ram_offset(&self, address: u16) -> usize {
        let slot = usize::from(address >> 10) & 0x07;
        usize::from(self.chr_low[slot] & 0x01) * 0x400 + usize::from(address & 0x03FF)
    }

    fn reads_ram(&self, address: u16) -> bool {
        self.ram_windows & (1 << ((address >> 10) & 0x07)) != 0
    }
}

impl Mapper for Waixing253 {
    banked!();

    fn mapper_number(&self) -> i32 {
        253
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr_low);
        writer.write_raw(&self.chr_high);
        writer.write_bool(self.force_chr_rom);
        writer.write_u8(self.irq_reload);
        writer.write_u8(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_u16(self.irq_scaler);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr_low)?;
        reader.read_into(&mut self.chr_high)?;
        self.force_chr_rom = reader.read_bool()?;
        self.irq_reload = reader.read_u8()?;
        self.irq_counter = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_scaler = reader.read_u16()?;
        self.irq_pending = reader.read_bool()?;
        self.refresh();
        Ok(())
    }

    /// `Mapper253.h:24-29` (`InitMapper`).
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0, 0];
        self.chr_low = [0; 8];
        self.chr_high = [0; 8];
        self.force_chr_rom = false;
        self.irq_reload = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_scaler = 0;
        self.irq_pending = false;
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `Mapper253.h:71-85` (`ProcessCpuClock`).
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }
        self.irq_scaler += 1;
        if self.irq_scaler >= 114 {
            self.irq_scaler = 0;
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter == 0 {
                self.irq_counter = self.irq_reload;
                self.irq_pending = true;
            }
        }
    }

    /// `Mapper253.h:87-139` (`WriteRegister`).
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if (0xB000..=0xE00C).contains(&address) {
            let slot = usize::from(((((address & 0x08) | (address >> 8)) >> 3) + 2) & 0x07);
            let shift = u32::from(address & 0x04);
            let low = (self.chr_low[slot] & (0xF0 >> shift)) | (value << shift);
            self.chr_low[slot] = low;
            if slot == 0 {
                if low == 0xC8 {
                    self.force_chr_rom = false;
                } else if low == 0x88 {
                    self.force_chr_rom = true;
                }
            }
            if shift != 0 {
                self.chr_high[slot] = value >> 4;
            }
        } else {
            match address {
                0x8010 => self.prg[0] = value,
                0xA010 => self.prg[1] = value,
                0x9400 => memory.set_mirroring(match value & 0x03 {
                    0 => NametableMirroring::Vertical,
                    1 => NametableMirroring::Horizontal,
                    2 => NametableMirroring::SingleScreenA,
                    _ => NametableMirroring::SingleScreenB,
                }),
                0xF000 => {
                    self.irq_reload = (self.irq_reload & 0xF0) | (value & 0x0F);
                    self.irq_pending = false;
                }
                0xF004 => {
                    self.irq_reload = (self.irq_reload & 0x0F) | (value << 4);
                    self.irq_pending = false;
                }
                0xF008 => {
                    self.irq_counter = self.irq_reload;
                    self.irq_enabled = value & 0x02 != 0;
                    self.irq_scaler = 0;
                    self.irq_pending = false;
                }
                _ => return,
            }
        }
        self.refresh();
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.ram_windows == 0 {
            Some(ChrView::new(memory.rom().chr_rom(), &self.banks))
        } else {
            None
        }
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.reads_ram(address) {
            let ram = memory.chr_ram();
            return ram.get(self.ram_offset(address)).copied().unwrap_or(0);
        }
        let rom = memory.rom().chr_rom();
        if rom.is_empty() {
            return self.banks.read_chr(address, memory.chr_ram());
        }
        self.banks.read_chr(address, rom)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.reads_ram(address) {
            let offset = self.ram_offset(address);
            if let Some(byte) = memory.chr_ram_mut().get_mut(offset) {
                *byte = value;
            }
        } else if memory.rom().chr_rom().is_empty() {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }
}
