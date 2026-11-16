//! NES 2.0 mapper 284, the board of the *Drip* homebrew (lot MAPPERS-N): a 16 KiB PRG bank, four
//! 2 KiB CHR banks, a 15-bit CPU-cycle IRQ, two FIFO-fed 8-bit PCM channels and **extended
//! attributes**, one palette per background tile read from 2 KiB of board memory.
//!
//! Read from Mesen2 `Homebrew/UnlDripGame.h` and `UnlDripGameAudio.h` (read, not copied), the one
//! implementation among the allowed sources, whose comments quote the board's own documentation:
//!
//! * `$8000-$BFFF` (mask `$800F`): `$8000-$8003` and `$8004-$8007` the two sound channels (clear
//!   FIFO, data, period low, period high and volume), `$8008`/`$8009` the IRQ counter ("buffered
//!   until writing to IRQ Counter High"; bit 7 of the high byte enables, and the write
//!   acknowledges), `$800A` mirroring (bits 0-1), extended attributes (bit 2) and work-RAM write
//!   enable (bit 3), `$800B` the PRG bank at `$8000` (`$C000` the last), `$800C-$800F` the CHR
//!   banks;
//! * `$C000-$FFFF`: the attribute memory, "mirrored throughout", A10 picking one of two 1 KiB;
//! * reads: `$4800` DIP switch in bit 7 over `$64`, `$5000`/`$5800` each channel's FIFO full
//!   (bit 7) and empty (bit 6);
//! * with extended attributes on, an **attribute** fetch of the rendering returns the 2-bit
//!   palette stored for the tile whose **nametable** byte was fetched just before, repeated four
//!   times "so the PPU can select the right palette no matter the shift value", from the 1 KiB the
//!   mirroring assigns to that table.
//!
//! In this core the substitution rides the MMC5's path (`board-mmc5-exattr-split`): the board
//! answers every table while the mode is on ([`Mapper::nametable_page`]), serves CIRAM itself in
//! [`Mapper::read_nametable_from_vram`], remembers the last nametable fetch through
//! [`Mapper::on_ppu_address_bus`], and is positioned per column on the 3D tile capture
//! ([`Mapper::position_captured_background_tile`]). What that path cannot tell apart is a `$2007`
//! read of an attribute byte from a rendering fetch (FIDELITY-LEADS J34).
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// One of the two sound channels (`UnlDripGameAudio.h`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DripChannel {
    buffer: [u8; 256],
    read: u8,
    write: u8,
    full: bool,
    empty: bool,
    period: u16,
    timer: u16,
    volume: u8,
    output: i32,
}

impl DripChannel {
    const fn new() -> Self {
        Self {
            buffer: [0; 256],
            read: 0,
            write: 0,
            full: false,
            empty: true,
            period: 0,
            timer: 0,
            volume: 0,
            output: 0,
        }
    }

    fn sample(&self, index: u8) -> i32 {
        (i32::from(self.buffer[usize::from(index)]) - 0x80) * i32::from(self.volume)
    }

    /// `ClockAudio`: each time the timer reaches zero a byte leaves the FIFO and is output.
    fn clock(&mut self) {
        if self.empty {
            return;
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.timer = self.period;
            if self.read == self.write {
                self.full = false;
            }
            self.read = self.read.wrapping_add(1);
            self.output = self.sample(self.read);
            if self.read == self.write {
                self.empty = true;
            }
        }
    }

    fn status(&self) -> u8 {
        (if self.full { 0x80 } else { 0 }) | (if self.empty { 0x40 } else { 0 })
    }

    fn write(&mut self, port: u16, value: u8) {
        match port & 0x03 {
            // "Writing any value will silence the corresponding sound channel".
            0 => {
                self.buffer = [0; 256];
                self.read = 0;
                self.write = 0;
                self.full = false;
                self.empty = true;
                self.output = 0;
                self.timer = self.period;
            }
            1 => {
                if self.read == self.write {
                    self.empty = false;
                    self.output = (i32::from(value) - 0x80) * i32::from(self.volume);
                    self.timer = self.period;
                }
                self.buffer[usize::from(self.write)] = value;
                self.write = self.write.wrapping_add(1);
                if self.read == self.write {
                    self.full = true;
                }
            }
            2 => self.period = (self.period & 0x0F00) | u16::from(value),
            _ => {
                self.period = (self.period & 0x00FF) | (u16::from(value & 0x0F) << 8);
                self.volume = value >> 4;
                if !self.empty {
                    self.output = self.sample(self.read);
                }
            }
        }
    }

    fn save(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.buffer);
        writer.write_u8(self.read);
        writer.write_u8(self.write);
        writer.write_bool(self.full);
        writer.write_bool(self.empty);
        writer.write_u16(self.period);
        writer.write_u16(self.timer);
        writer.write_u8(self.volume);
    }

    fn load(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.buffer)?;
        self.read = reader.read_u8()?;
        self.write = reader.read_u8()?;
        self.full = reader.read_bool()?;
        self.empty = reader.read_bool()?;
        self.period = reader.read_u16()?;
        self.timer = reader.read_u16()?;
        self.volume = reader.read_u8()?;
        self.output = if self.empty {
            0
        } else {
            self.sample(self.read)
        };
        Ok(())
    }
}

/// Mapper 284.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drip {
    banks: BankWindows,
    prg: u8,
    chr: [u8; 4],
    /// `$800A`: mirroring, extended attributes, work-RAM write enable.
    control: u8,
    irq_low: u8,
    counter: u16,
    irq_enabled: bool,
    irq_pending: bool,
    attributes: [[u8; 0x400]; 2],
    /// The offset in its table of the last nametable byte fetched.
    last_tile: u16,
    channels: [DripChannel; 2],
}

impl Drip {
    /// The level of one channel's full-scale sample, in the mixer's units: a guess, the two
    /// channels together spanning about the DMC's own range (FIDELITY-LEADS J34).
    const AUDIO_SCALE: i32 = 4;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            prg: 0,
            chr: [0; 4],
            control: 0,
            irq_low: 0,
            counter: 0,
            irq_enabled: false,
            irq_pending: false,
            attributes: [[0; 0x400]; 2],
            last_tile: 0,
            channels: [DripChannel::new(), DripChannel::new()],
        };
        board.refresh();
        board
    }

    fn refresh(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg & 0x0F));
        self.banks.map_prg_16k(2, -1);
        for (slot, bank) in self.chr.iter().enumerate() {
            self.banks.map_chr_2k(slot * 2, i32::from(*bank & 0x0F));
        }
    }

    const fn extended(&self) -> bool {
        self.control & 0x04 != 0
    }

    /// The CIRAM page, and the attribute memory, a nametable address belongs to under `$800A`'s
    /// mirroring (`MapperReadVram`'s `switch(GetMirroringType())`).
    const fn page(&self, address: u16) -> usize {
        match self.control & 0x03 {
            0 => ((address >> 10) & 0x01) as usize,
            1 => ((address >> 11) & 0x01) as usize,
            2 => 0,
            _ => 1,
        }
    }

    const fn mirroring(&self) -> NametableMirroring {
        match self.control & 0x03 {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }
}

impl Mapper for Drip {
    banked!();

    fn mapper_number(&self) -> i32 {
        284
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.control);
        writer.write_u8(self.irq_low);
        writer.write_u16(self.counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        writer.write_raw(&self.attributes[0]);
        writer.write_raw(&self.attributes[1]);
        writer.write_u16(self.last_tile);
        for channel in &self.channels {
            channel.save(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg = reader.read_u8()?;
        reader.read_into(&mut self.chr)?;
        self.control = reader.read_u8()?;
        self.irq_low = reader.read_u8()?;
        self.counter = reader.read_u16()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        let [first, second] = &mut self.attributes;
        reader.read_into(first)?;
        reader.read_into(second)?;
        self.last_tile = reader.read_u16()? & 0x03FF;
        for channel in &mut self.channels {
            channel.load(reader)?;
        }
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        let attributes = self.attributes;
        *self = Self::new(memory);
        // The attribute memory is RAM: a reset leaves it, a power-on starts it at 0 here.
        self.attributes = attributes;
        memory.set_mirroring(self.mirroring());
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// "While the IRQ counter is enabled, the timer is decremented once per CPU cycle. Once the
    /// timer reaches zero, the /IRQ line is set to logic 0 and the timer stops decrementing".
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.irq_enabled && self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.irq_enabled = false;
                self.irq_pending = true;
            }
        }
        self.channels[0].clock();
        self.channels[1].clock();
    }

    fn expansion_audio_output(&self) -> Option<i32> {
        Some((self.channels[0].output + self.channels[1].output) * Self::AUDIO_SCALE)
    }

    /// `$6000-$7FFF` is always readable and written only with `$800A` bit 3.
    fn prg_ram_writable(&self) -> bool {
        self.control & 0x08 != 0
    }

    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !(0x4800..0x6000).contains(&address) {
            return None;
        }
        Some(Some(match address & 0x5800 {
            0x4800 => 0x64,
            0x5000 => self.channels[0].status(),
            _ => self.channels[1].status(),
        }))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        if address >= 0xC000 {
            self.attributes[usize::from(address & 0x0400 != 0)][usize::from(address & 0x03FF)] =
                value;
            return;
        }
        match address & 0x800F {
            0x8000..=0x8003 => self.channels[0].write(address, value),
            0x8004..=0x8007 => self.channels[1].write(address, value),
            0x8008 => self.irq_low = value,
            0x8009 => {
                self.counter = (u16::from(value & 0x7F) << 8) | u16::from(self.irq_low);
                self.irq_enabled = value & 0x80 != 0;
                self.irq_pending = false;
            }
            0x800A => {
                self.control = value;
                memory.set_mirroring(self.mirroring());
            }
            0x800B => self.prg = value,
            port => self.chr[usize::from(port & 0x03)] = value,
        }
        self.refresh();
    }

    fn nametable_page(&self, _table: i32) -> i32 {
        if self.extended() {
            Cartridge::SERVED_BY_CARTRIDGE
        } else {
            Cartridge::FOLLOW_MIRRORING
        }
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        let address = address & 0x3FFF;
        if (0x2000..0x3F00).contains(&address) && address & 0x03FF < 0x03C0 {
            self.last_tile = address & 0x03FF;
        }
    }

    fn read_nametable_from_vram(&self, _memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        let page = self.page(address);
        let offset = usize::from(address & 0x03FF);
        if self.extended() && offset >= 0x03C0 {
            let palette = self.attributes[page][usize::from(self.last_tile)] & 0x03;
            return palette * 0x55;
        }
        vram.get((page << 10) | offset).copied().unwrap_or(0)
    }

    fn nametable_write_offset(&self, address: u16) -> Option<usize> {
        Some((self.page(address) << 10) | usize::from(address & 0x03FF))
    }

    fn substitutes_background_tiles(&self) -> bool {
        self.extended()
    }

    fn background_capture_state(&self) -> u64 {
        u64::from(self.last_tile)
    }

    fn set_background_capture_state(&mut self, state: u64) {
        self.last_tile = u16::try_from(state & 0x03FF).unwrap_or(0);
    }

    fn position_captured_background_tile(&mut self, address: u16, _column: i32, _line: i32) {
        self.last_tile = address & 0x03FF;
    }
}
