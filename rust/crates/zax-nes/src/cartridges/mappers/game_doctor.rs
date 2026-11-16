//! Lot MAPPERS-P: mapper 562, the Venus Turbo Game Doctor — a copier whose DRAM holds the game
//! (the image's PRG and CHR, loaded at power-on) and whose 2 KiB BIOS shows at `$4800-$4FFF`.
//!
//! Read from puNES 0.111 (`mapper_562.c`, read, not copied). The BIOS is the copier's firmware
//! dump that puNES carries as a byte table; the owner chose to carry it here too (MAPPERS-P,
//! 2026-11-15), in `data/vtgd_bios.bin`. [`super::IrqP`] carries the board (kind `M562`); this
//! module holds the copier's registers, its RAM and its decodes. Its launch program is the NES 2.0
//! misc ROM, run as the 561's ([`super::Mapper::boot_program`]).
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The BIOS (`bios[]`), read at `$4800-$4FFF` through `address & $07FF`.
const BIOS: &[u8; 0x800] = include_bytes!("data/vtgd_bios.bin");

/// The copier's state (`_m562`), its field names puNES's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameDoctor {
    prg: [u8; 4],
    latch: u8,
    chr_lock: bool,
    reg1k: [u8; 8],
    reg8k: u8,
    lastbank: u8,
    m1: u8,
    m2: u8,
    tgd: u8,
    irq_target: u16,
    irq_counter: u16,
    fds_control: u8,
    fds_counter: i16,
    pub(crate) irq: bool,
    /// The CHR write enable `chr_fix_562` last computed (`memmap_vram_wp_1k`): a write changes
    /// the lock before it lands, but the permission is the one already mapped.
    chr_enabled: bool,
    /// The copier DRAM: the image's PRG and CHR (at least 8 KiB), saved in the state.
    pub(crate) prg_ram: Vec<u8>,
    pub(crate) chr_ram: Vec<u8>,
}

impl GameDoctor {
    /// `map_init_562` at power-on and `extcl_after_mapper_init_562`'s CHR copy.
    pub(crate) fn new(
        prg: &[u8],
        chr: &[u8],
        chr_ram: usize,
        submapper: u8,
        vertical: bool,
    ) -> Self {
        let mut chr_copy = vec![0; chr.len().max(chr_ram).max(0x2000)];
        chr_copy[..chr.len()].copy_from_slice(chr);
        let mut board = Self {
            prg: [0x1C, 0x1D, 0x1E, 0x1F],
            latch: 0,
            chr_lock: false,
            reg1k: [0, 1, 2, 3, 4, 5, 6, 7],
            reg8k: 0,
            lastbank: 0,
            m1: (submapper << 5) | if vertical { 0x01 } else { 0x11 } | 0x06,
            m2: 0x03,
            tgd: 0x03,
            irq_target: 0,
            irq_counter: 0xFFFF,
            fds_control: 0,
            fds_counter: 0,
            irq: false,
            chr_enabled: true,
            prg_ram: prg.to_vec(),
            chr_ram: chr_copy,
        };
        board.fix_chr();
        board
    }

    /// `map_init_562` on RESET: only the IRQ counter restarts.
    pub(crate) fn soft_reset(&mut self) {
        self.irq_target = 0;
        self.irq_counter = 0xFFFF;
    }

    /// `mirroring_fix_562`.
    pub(crate) const fn mirroring(&self) -> NametableMirroring {
        match self.m1 & 0x11 {
            0x00 => NametableMirroring::SingleScreenA,
            0x01 => NametableMirroring::Vertical,
            0x10 => NametableMirroring::SingleScreenB,
            _ => NametableMirroring::Horizontal,
        }
    }

    /// `chr_fix_562`'s write enable.
    fn fix_chr(&mut self) {
        self.chr_enabled = !((self.m1 & 0xE1) >= 0x81 || self.chr_lock);
    }

    /// `extcl_cpu_wr_mem_562` at `$4000-$4FFF` and `$8000-$FFFF`; `$8000+` with the latch off
    /// writes the DRAM (`memmap_auto_wp_8k(…, wr)`).
    pub(crate) fn write(&mut self, address: u16, value: u8) {
        if address >= 0x8000 {
            if self.m1 & 0x02 == 0 {
                let offset = self.prg_offset(address);
                if let Some(byte) = self.prg_ram.get_mut(offset) {
                    *byte = value;
                }
                return;
            }
            self.latch = value;
            match self.m1 >> 5 {
                0 | 2 => self.reg8k = 0,
                1 | 4 | 5 => self.reg8k = value & 0x03,
                3 => self.reg8k = (value & 0x30) >> 4,
                _ => {}
            }
            self.prg[usize::from((address >> 13) & 0x03)] = value >> 2;
            return;
        }
        let low = address.to_le_bytes()[0];
        match address {
            0x4024 => self.irq = false,
            0x4025 => {
                self.fds_control = value;
                if value & 0x42 != 0 {
                    self.fds_counter = 0;
                }
                self.irq = false;
            }
            0x42FC..=0x42FF => {
                self.m1 = (value & 0xF0) | (low & 0x03);
                if self.m1 >= 0x80 {
                    self.chr_lock = false;
                }
                self.fix_chr();
            }
            0x43FC..=0x43FF => {
                self.m2 = (value & 0xF0) | (low & 0x03);
                self.reg8k = value & 0x03;
            }
            0x4400..=0x4407 => self.reg1k[usize::from(low & 0x07)] = value,
            0x440C => {
                self.irq_target = (self.irq_target & 0x00FF) | (u16::from(value) << 8);
                if value & 0x80 == 0 {
                    self.irq_counter = 0x8000;
                }
                self.irq = false;
            }
            0x440D => {
                self.irq_target = (self.irq_target & 0xFF00) | u16::from(value);
                self.irq = false;
            }
            0x4411 => self.tgd = value,
            _ => {}
        }
    }

    /// `extcl_cpu_rd_mem_562` at `$4000-$4FFF`: the registers, the BIOS where A11 is set, `None`
    /// (the work RAM's open bus) elsewhere.
    pub(crate) fn read_register(&self, address: u16) -> Option<u8> {
        let low = address.to_le_bytes()[0];
        Some(match address {
            0x4400..=0x4407 => self.reg1k[usize::from(low & 0x07)],
            0x4408..=0x440B => (self.prg[usize::from(low & 0x03)] << 2) | (self.latch & 0x03),
            0x440C => self.irq_counter.to_be_bytes()[0],
            0x440D => self.irq_counter.to_be_bytes()[1],
            0x4411 => self.tgd,
            0x4415 => self.m1,
            0x4420 => self.reg1k[usize::from(self.lastbank & 0x07)],
            _ if address & 0x0800 != 0 => BIOS[usize::from(address & 0x07FF)],
            _ => return None,
        })
    }

    /// `prg_fix_562`: the 8 KiB DRAM bank of a `$8000-$FFFF` window.
    fn prg_bank(&self, slot: usize) -> usize {
        let prg = usize::from(self.prg[slot]);
        if self.tgd & 0x80 != 0 {
            return prg;
        }
        if self.m2 & 0x01 == 0 {
            let high = if slot < 2 {
                usize::from(self.m2 & 0x40) >> 2
            } else {
                0
            };
            return high | (prg & 0x0F);
        }
        let latch = usize::from(self.latch);
        // A 16 KiB bank `b` is 8 KiB banks `2b` and `2b + 1`, a 32 KiB one `4b` to `4b + 3`.
        let k16 = |low: usize, high: usize| {
            let bank = if slot < 2 { low } else { high };
            (bank << 1) | (slot & 0x01)
        };
        match self.m1 >> 5 {
            0 => k16(latch & 0x07, 0x07),
            1 => k16((latch & 0x3C) >> 2, 0x07),
            2 => k16(latch & 0x0F, 0x0F),
            3 => k16(0x0F, latch & 0x0F),
            4 => (((latch & 0x30) >> 4) << 2) | slot,
            5 => (0x03 << 2) | slot,
            6 => match slot {
                0 => latch & 0x0F,
                1 => latch >> 4,
                _ => (0x07 << 1) | (slot & 0x01),
            },
            _ => match slot {
                0 => latch & 0x0E,
                1 => (latch >> 4) | 0x01,
                _ => (0x07 << 1) | (slot & 0x01),
            },
        }
    }

    /// The DRAM offset of a `$8000-$FFFF` address, wrapped as `memmap_auto_8k` wraps.
    fn prg_offset(&self, address: u16) -> usize {
        let len = self.prg_ram.len().max(1);
        (self.prg_bank(usize::from((address >> 13) & 0x03)) * 0x2000
            + usize::from(address & 0x1FFF))
            % len
    }

    /// A `$8000-$FFFF` byte.
    pub(crate) fn read_prg(&self, address: u16) -> u8 {
        self.prg_ram
            .get(self.prg_offset(address))
            .copied()
            .unwrap_or(0)
    }

    /// `chr_fix_562`'s 1 KiB bank of a pattern address.
    fn chr_offset(&self, address: u16) -> usize {
        let slot = usize::from(address >> 10) & 0x07;
        let bank = if self.tgd & 0x40 != 0 {
            usize::from(self.reg1k[slot])
        } else {
            (usize::from(self.reg8k) << 3) | slot
        };
        (bank * 0x400 + usize::from(address & 0x03FF)) % self.chr_ram.len().max(1)
    }

    /// `extcl_rd_chr_562`: the read also latches the window for `$4420`.
    pub(crate) fn chr_byte(&self, address: u16) -> u8 {
        self.chr_ram
            .get(self.chr_offset(address))
            .copied()
            .unwrap_or(0)
    }

    /// The pattern fetch's side of `extcl_rd_chr_562`.
    pub(crate) fn see_chr(&mut self, address: u16) {
        if address < 0x2000 {
            self.lastbank = u8::try_from(address >> 10).unwrap_or(0);
        }
    }

    /// `extcl_wr_chr_562`.
    pub(crate) fn write_chr(&mut self, address: u16, value: u8) {
        if self.m1 >= 0xA0 && self.m1 & 0x01 == 0 {
            self.chr_lock = self.m1 & 0x10 != 0;
        }
        self.lastbank = u8::try_from(address >> 10).unwrap_or(0);
        if self.chr_enabled {
            let offset = self.chr_offset(address);
            if let Some(byte) = self.chr_ram.get_mut(offset) {
                *byte = value;
            }
        }
    }

    /// `extcl_cpu_every_cycle_562`: the FDS-style timer gains 3 a cycle and asserts every 448
    /// while `$4025` D7 is set; the 16-bit counter counts up to its target while that has D15.
    pub(crate) fn step(&mut self) {
        self.fds_counter = self.fds_counter.wrapping_add(3);
        while self.fds_counter >= 448 && self.fds_control & 0x80 != 0 {
            self.fds_counter -= 448;
            self.irq = true;
        }
        if self.irq_target & 0x8000 != 0 {
            if self.irq_counter == self.irq_target && self.irq_counter != 0xFFFF {
                self.irq = true;
            } else {
                self.irq_counter = self.irq_counter.wrapping_add(1);
            }
        }
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.reg1k);
        writer.write_raw(&[
            self.latch,
            self.reg8k,
            self.lastbank,
            self.m1,
            self.m2,
            self.tgd,
            self.fds_control,
        ]);
        writer.write_bool(self.chr_lock);
        writer.write_bool(self.irq);
        writer.write_bool(self.chr_enabled);
        writer.write_u16(self.irq_target);
        writer.write_u16(self.irq_counter);
        writer.write_i16(self.fds_counter);
        writer.write_raw(&self.prg_ram);
        writer.write_raw(&self.chr_ram);
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        read_exact(reader, &mut self.prg)?;
        read_exact(reader, &mut self.reg1k)?;
        let mut bytes = [0u8; 7];
        read_exact(reader, &mut bytes)?;
        [
            self.latch,
            self.reg8k,
            self.lastbank,
            self.m1,
            self.m2,
            self.tgd,
            self.fds_control,
        ] = bytes;
        self.chr_lock = reader.read_bool()?;
        self.irq = reader.read_bool()?;
        self.chr_enabled = reader.read_bool()?;
        self.irq_target = reader.read_u16()?;
        self.irq_counter = reader.read_u16()?;
        self.fds_counter = reader.read_i16()?;
        read_exact(reader, &mut self.prg_ram)?;
        read_exact(reader, &mut self.chr_ram)?;
        Ok(())
    }
}

/// Fills `target` from the state.
fn read_exact(reader: &mut StateReader, target: &mut [u8]) -> Result<(), InvalidState> {
    target.copy_from_slice(reader.read_raw(target.len())?);
    Ok(())
}
