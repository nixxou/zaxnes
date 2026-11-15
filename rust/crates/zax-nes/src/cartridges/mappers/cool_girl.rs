//! Lot MAPPERS-P: mapper 342, the COOLGIRL — an open FPGA multicart that re-implements some
//! thirty mappers in one register file, with a NOR flash for its PRG and a save area in it.
//!
//! Read from puNES 0.111 (`mapper_342.c`, itself after FCEUX's `m342.cpp` by the board's author;
//! read, not copied). [`super::IrqP`] carries the board (kind `M342`); this module holds the
//! FPGA's state and decodes. Its PRG banks reach the image's PRG-ROM, except the top 8 MiB of the
//! 1 GiB bank space, which is the flash's save area: that lives here, allocated by 8 KiB page on
//! the first write, saved in the state, and not persisted by the host (FIDELITY-LEADS J37). The
//! CHR is CHR-RAM, the work RAM four 8 KiB pages.
//!
//! puNES clocks its scanline counters (the MMC3's, the MMC5 compare, mapper 163's latch) at dot
//! 260 of each rendered line. No board hook sees PPU dots here, so the board finds the lines on
//! its own bus the way the MMC5 does — three identical nametable reads end a line — and runs the
//! dot-260 work on the first fetch at or past dot 260 of a line it saw start.
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The save area's size and page (`SAVE_FLASH_SIZE`, 8 MiB).
const SAVE_SIZE: usize = 0x80_0000;
const SAVE_PAGE: usize = 0x2000;

/// The first 8 KiB bank of the save area: the last 8 MiB of the 1 GiB bank space.
const SAVE_FIRST_BANK: u32 = 0x2_0000 - 0x400;

/// The flash chip's CFI query table (`cfi_data`), repeated through `$8000-$FFFF` in CFI mode.
const CFI: [u8; 256] = {
    let head: &[u8] = &[
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x51, 0x51, 0x52, 0x52, 0x59, 0x59, 0x02, 0x02, 0x00, 0x00, 0x40, 0x40, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, 0x27, 0x36, 0x36, 0x00, 0x00,
        0x00, 0x00, 0x06, 0x06, 0x06, 0x06, 0x09, 0x09, 0x13, 0x13, 0x03, 0x03, 0x05, 0x05, 0x03,
        0x03, 0x02, 0x02, 0x1E, 0x1E, 0x02, 0x02, 0x00, 0x00, 0x06, 0x06, 0x00, 0x00, 0x01, 0x01,
        0xFF, 0xFF, 0x03, 0x03, 0x00, 0x00, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x50, 0x50,
        0x52, 0x52, 0x49, 0x49, 0x31, 0x31, 0x33, 0x33, 0x14, 0x14, 0x02, 0x02, 0x01, 0x01, 0x00,
        0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x02, 0x02, 0xB5, 0xB5, 0xC5, 0xC5, 0x05, 0x05,
        0x01, 0x01,
    ];
    let mut table = [0xFF; 256];
    let mut i = 0;
    while i < head.len() {
        table[i] = head[i];
        i += 1;
    }
    table
};

/// Where a CPU window reads: an 8 KiB bank of the PRG-ROM or of the save area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Window {
    Rom(u32),
    Save(u32),
}

/// The COOLGIRL's state (`_m342`), its field names puNES's.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CoolGirl {
    mapper: u8,
    fscreen: bool,
    mirroring: u8,
    flags: u8,
    lock: bool,
    cfi_mode: bool,
    map_rom_on_6000: bool,
    wram_enabled: bool,
    wram_page: u8,
    flash_can_write: bool,
    flash_state: u8,
    flash_a: [u16; 10],
    flash_v: [u8; 10],
    prg_b6000: u16,
    /// `prg.a`-`prg.d`.
    prg: [u16; 4],
    prg_a_mapped: u32,
    prg_mode: u8,
    prg_base: u16,
    prg_mask: u8,
    /// `chr.a`-`chr.h`.
    chr: [u16; 8],
    chr_can_write: bool,
    chr_mode: u8,
    chr_mask: u8,
    mmc1_load: u8,
    latch0: u8,
    latch1: u8,
    mmc3_internal: u8,
    mmc3_irq_enabled: bool,
    mmc3_irq_counter: u8,
    mmc3_irq_latch: u8,
    mmc3_irq_reload: bool,
    mmc5_irq_enabled: bool,
    mmc5_irq_line: u8,
    mmc5_irq_out: bool,
    vrc3_value: u16,
    vrc3_control: u8,
    vrc3_latch: u16,
    vrc4_value: u8,
    vrc4_control: u8,
    vrc4_latch: u8,
    vrc4_prescaler: u8,
    vrc4_prescaler_counter: u8,
    m69_internal: u8,
    m69_irq_enabled: bool,
    m69_counter_enabled: bool,
    m69_value: u16,
    m112_internal: u8,
    /// Mapper 163's latch and `r0`-`r5`.
    m163: [u8; 7],
    m83_enabled_latch: bool,
    m83_enabled: bool,
    m83_counter: u16,
    m90_mul: [u8; 2],
    m90_xor: u8,
    m18_value: u16,
    m18_control: u8,
    m18_latch: u16,
    m65_enabled: bool,
    m65_value: u16,
    m65_latch: u16,
    m42_enabled: bool,
    m42_value: u16,
    m67_enabled: bool,
    m67_latch: u8,
    m67_counter: u16,
    /// The cartridge's IRQ line (`EXT_IRQ`).
    pub(crate) irq: bool,

    /// The decoded windows (`prg_fix_342`, `wram_fix_342`, `chr_fix_342`): derived.
    pub(crate) windows: [Window; 4],
    /// `$6000-$7FFF`: a PRG-ROM bank, or a work-RAM page (`None` keeps puNES's power-on page 0).
    pub(crate) low: Window,
    low_is_ram: bool,
    pub(crate) chr_banks: [u32; 8],
    /// Mapper 189's (TxSROM) CIRAM page per table, `None` when the mirroring register rules.
    nt_pages: Option<[u8; 4]>,

    /// The save area, by 8 KiB page, allocated on the first write.
    save: Vec<Option<Box<[u8]>>>,

    /// The scanline tracker on the board's own bus: the last address, how often it repeated,
    /// the dot clock a line started at, the line number (`None` outside the frame), and
    /// whether this line's dot-260 work ran.
    last_address: u16,
    repeats: u8,
    line_start: i64,
    line: Option<u16>,
    line_done: bool,
}

impl CoolGirl {
    pub(crate) fn new() -> Self {
        let mut board = Self {
            mapper: 0,
            fscreen: false,
            mirroring: 0,
            flags: 0,
            lock: false,
            cfi_mode: false,
            map_rom_on_6000: false,
            wram_enabled: false,
            wram_page: 0,
            flash_can_write: false,
            flash_state: 0,
            flash_a: [0; 10],
            flash_v: [0; 10],
            prg_b6000: 0,
            prg: [0, 1, 0xFE, 0xFF],
            prg_a_mapped: 0,
            prg_mode: 0,
            prg_base: 0,
            prg_mask: 0xF8,
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            chr_can_write: false,
            chr_mode: 0,
            chr_mask: 0,
            mmc1_load: 0,
            latch0: 0,
            latch1: 0,
            mmc3_internal: 0,
            mmc3_irq_enabled: false,
            mmc3_irq_counter: 0,
            mmc3_irq_latch: 0,
            mmc3_irq_reload: false,
            mmc5_irq_enabled: false,
            mmc5_irq_line: 0,
            mmc5_irq_out: false,
            vrc3_value: 0,
            vrc3_control: 0,
            vrc3_latch: 0,
            vrc4_value: 0,
            vrc4_control: 0,
            vrc4_latch: 0,
            vrc4_prescaler: 0,
            vrc4_prescaler_counter: 0,
            m69_internal: 0,
            m69_irq_enabled: false,
            m69_counter_enabled: false,
            m69_value: 0,
            m112_internal: 0,
            m163: [0; 7],
            m83_enabled_latch: false,
            m83_enabled: false,
            m83_counter: 0,
            m90_mul: [0; 2],
            m90_xor: 0,
            m18_value: 0,
            m18_control: 0,
            m18_latch: 0,
            m65_enabled: false,
            m65_value: 0,
            m65_latch: 0,
            m42_enabled: false,
            m42_value: 0,
            m67_enabled: false,
            m67_latch: 0,
            m67_counter: 0,
            irq: false,
            windows: [Window::Rom(0); 4],
            low: Window::Rom(0),
            low_is_ram: true,
            chr_banks: [0; 8],
            nt_pages: None,
            save: vec![None; SAVE_SIZE / SAVE_PAGE],
            last_address: 0,
            repeats: 0,
            line_start: 0,
            line: None,
            line_done: true,
        };
        board.fix();
        board
    }

    /// `state_fix_342`.
    pub(crate) fn fix(&mut self) {
        self.fix_prg();
        self.fix_chr();
        self.fix_wram();
    }

    /// `prg_mapped_342`.
    fn mapped(&self, bank: u16) -> u32 {
        (u32::from(self.prg_base) << 1)
            | u32::from(bank & ((u16::from(!self.prg_mask & 0x7F) << 1) | 1))
    }

    /// An 8 KiB bank of the 1 GiB space as a window.
    const fn window(bank: u32) -> Window {
        if bank >= SAVE_FIRST_BANK {
            Window::Save(bank)
        } else {
            Window::Rom(bank)
        }
    }

    /// `prg_fix_342`: in CFI mode the windows stay as they are.
    fn fix_prg(&mut self) {
        let [a, b, c, d] = self.prg.map(|bank| self.mapped(bank));
        self.prg_a_mapped = a;
        if self.cfi_mode {
            return;
        }
        // A 16 KiB (32 KiB) bank keeps its 8 KiB halves (quarters) and the flash test of the
        // mapped bank puNES makes before the shift.
        let k16 = |bank: u32| {
            let save = bank >= SAVE_FIRST_BANK;
            let base = (bank >> 1) << 1;
            [0, 1].map(|half| {
                if save {
                    Window::Save(base | half)
                } else {
                    Window::Rom(base | half)
                }
            })
        };
        let k32 = |bank: u32| {
            let save = bank >= SAVE_FIRST_BANK;
            let base = (bank >> 2) << 2;
            [0, 1, 2, 3].map(|quarter| {
                if save {
                    Window::Save(base | quarter)
                } else {
                    Window::Rom(base | quarter)
                }
            })
        };
        self.windows = match self.prg_mode & 0x07 {
            1 => {
                let [w0, w1] = k16(c);
                let [w2, w3] = k16(a);
                [w0, w1, w2, w3]
            }
            4 => [a, b, c, d].map(Self::window),
            5 => [c, b, a, d].map(Self::window),
            6 => k32(b),
            7 => k32(a),
            _ => {
                let [w0, w1] = k16(a);
                let [w2, w3] = k16(c);
                [w0, w1, w2, w3]
            }
        };
    }

    /// `wram_fix_342`: the ROM bank at `$6000`, or the work-RAM page while enabled; neither
    /// leaves the previous mapping.
    fn fix_wram(&mut self) {
        if self.map_rom_on_6000 {
            self.low = Window::Rom(self.mapped(self.prg_b6000));
            self.low_is_ram = false;
        } else if self.wram_enabled {
            self.low = Window::Rom(u32::from(self.wram_page));
            self.low_is_ram = true;
        }
    }

    /// `chr_fix_342`: eight 1 KiB CHR-RAM banks by the CHR mode.
    #[allow(clippy::too_many_lines)]
    fn fix_chr(&mut self) {
        let shift = u32::from(self.mapper == 24 && self.flags & 0x02 != 0);
        let mask = ((((u32::from(!self.chr_mask & 0x1F) + 1) * 0x2000) / 0x400) - 1) >> shift;
        let c = self.chr.map(|bank| u32::from(bank) >> shift);
        let unit = |bank: u32, bits: u32| ((bank >> bits) & (mask >> bits)) << bits;
        let one = |bank: u32| bank & mask;
        let four = |base: u32| [base, base | 1, base | 2, base | 3];
        let two = |base: u32| [base, base | 1];
        let mut bank = [0u32; 8];
        match self.chr_mode & 0x07 {
            1 => {
                let base = unit(u32::from(self.m163[0]) >> shift, 2);
                bank = [
                    base,
                    base | 1,
                    base | 2,
                    base | 3,
                    base,
                    base | 1,
                    base | 2,
                    base | 3,
                ];
            }
            2 => {
                bank[..2].copy_from_slice(&two(unit(c[0], 1)));
                bank[2..4].copy_from_slice(&two(unit(c[2], 1)));
                for slot in 4..8 {
                    bank[slot] = one(c[slot]);
                }
            }
            3 => {
                for slot in 0..4 {
                    bank[slot] = one(c[4 + slot]);
                }
                bank[4..6].copy_from_slice(&two(unit(c[0], 1)));
                bank[6..8].copy_from_slice(&two(unit(c[2], 1)));
            }
            4 => {
                bank[..4].copy_from_slice(&four(unit(c[0], 2)));
                bank[4..].copy_from_slice(&four(unit(c[4], 2)));
            }
            5 => {
                let low = if self.latch0 == 0 { c[0] } else { c[1] };
                let high = if self.latch1 == 0 { c[4] } else { c[5] };
                bank[..4].copy_from_slice(&four(unit(low, 2)));
                bank[4..].copy_from_slice(&four(unit(high, 2)));
            }
            6 => {
                for pair in 0..4 {
                    bank[pair * 2..pair * 2 + 2].copy_from_slice(&two(unit(c[pair * 2], 1)));
                }
            }
            7 => {
                for slot in 0..8 {
                    bank[slot] = one(c[slot]);
                }
            }
            _ => {
                let base = unit(c[0], 3);
                for (slot, value) in bank.iter_mut().enumerate() {
                    *value = base | u32::try_from(slot).unwrap_or(0);
                }
            }
        }
        self.chr_banks = bank;
    }

    /// `mirroring_fix_342`: four-screen, or the mirroring register unless mapper 189 (TxSROM)
    /// pages the tables itself.
    pub(crate) const fn mirroring(&self) -> Option<NametableMirroring> {
        if self.fscreen {
            return Some(NametableMirroring::FourScreen);
        }
        if self.mapper == 20 && self.flags & 0x01 != 0 {
            return None;
        }
        Some(match self.mirroring & 0x03 {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        })
    }

    /// Mapper 189's CIRAM page of table `table`, `None` when the mirroring rules.
    pub(crate) fn nametable_page(&self, table: usize) -> Option<u8> {
        if self.mapper == 20 && self.flags & 0x01 != 0 && !self.fscreen {
            return self.nt_pages.map(|pages| pages[table & 0x03]);
        }
        None
    }

    /// `extcl_cpu_wr_mem_342`.
    pub(crate) fn write(&mut self, address: u16, value: u8) {
        if address < 0x8000 {
            if (0x5000..0x5FFF).contains(&address) && !self.lock {
                self.write_config(address, value);
            }
            self.write_low(address, value);
            self.fix();
            return;
        }
        if self.flash_can_write {
            self.write_flash(address, value);
            return;
        }
        self.write_high(address, value);
        self.fix();
    }

    /// `mapper00_cpu_wr_low`: `$5000-$5FFE` while unlocked, by A0-A2.
    fn write_config(&mut self, address: u16, value: u8) {
        match address & 0x0007 {
            0 => self.prg_base = (self.prg_base & 0xFF) | (u16::from(value) << 8),
            1 => self.prg_base = (self.prg_base & 0xFF00) | u16::from(value),
            2 => {
                self.chr_mask = (self.chr_mask & 0x1F) | ((value & 0x80) >> 2);
                self.prg_mask = value & 0x7F;
            }
            3 => {
                self.prg_mode = value >> 5;
                self.chr[0] = (self.chr[0] & 0x07) | (u16::from(value) << 3);
            }
            4 => {
                self.chr_mode = value >> 5;
                self.chr_mask = (self.chr_mask & 0x20) | (value & 0x1F);
            }
            5 => {
                self.chr[0] = (self.chr[0] & 0xFF) | (u16::from(value & 0x80) << 1);
                self.prg[0] = (self.prg[0] & 0xC1) | u16::from((value & 0x7C) >> 1);
                self.wram_page = value & 0x03;
            }
            6 => {
                self.flags = value >> 5;
                self.mapper = (self.mapper & 0x20) | (value & 0x1F);
            }
            _ => {
                self.lock = value & 0x80 != 0;
                self.mapper = (self.mapper & 0x1F) | ((value & 0x40) >> 1);
                self.fscreen = value & 0x20 != 0;
                self.mirroring = (value & 0x18) >> 3;
                self.flash_can_write = value & 0x04 != 0;
                self.chr_can_write = value & 0x02 != 0;
                self.wram_enabled = value & 0x01 != 0;
                match self.mapper {
                    // Mapper 65, Irem H3001.
                    14 => self.prg[1] = 1,
                    // MMC2.
                    17 => self.prg[1] = 0xFD,
                    // Mapper 42.
                    23 => self.map_rom_on_6000 = true,
                    _ => {}
                }
            }
        }
    }

    /// The per-mapper writes below `$8000` (`mapperNN_cpu_wr_low`).
    #[allow(clippy::too_many_lines)]
    fn write_low(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match self.mapper {
            // Mapper 163.
            6 => {
                if address == 0x5101 {
                    if self.m163[5] != 0 && value == 0 {
                        self.m163[6] ^= 1;
                    }
                    self.m163[5] = value;
                } else if address == 0x5100 && value == 0x06 {
                    self.prg_mode &= 0xFE;
                    self.prg[1] = 12;
                } else if address & 0x7000 == 0x5000 {
                    match (address & 0x0300) >> 8 {
                        2 => {
                            self.prg_mode |= 1;
                            self.prg[0] = (self.prg[0] & 0x3F) | ((v & 0x03) << 6);
                            self.m163[1] = value;
                        }
                        0 => {
                            self.prg_mode |= 1;
                            self.prg[0] = (self.prg[0] & 0xC3) | ((v & 0x0F) << 2);
                            self.chr_mode = (self.chr_mode & 0xFE) | (value >> 7);
                            self.m163[2] = value;
                        }
                        3 => self.m163[3] = value,
                        _ => self.m163[4] = value,
                    }
                }
            }
            // Mapper 87.
            12 => {
                if address & 0x6000 == 0x6000 {
                    self.chr[0] = (self.chr[0] & 0xE7) | ((v & 0x01) << 4) | ((v & 0x02) << 2);
                }
            }
            // MMC5 (partly).
            15 => self.write_mmc5(address, value),
            // Mapper 189.
            20 => {
                if self.flags & 0x02 != 0 && address >= 0x4120 {
                    self.prg[0] = (self.prg[0] & 0xC3) | ((v & 0x0F) << 2) | ((v & 0xF0) >> 2);
                }
            }
            // Mappers 79 and 146.
            27 => {
                if address & 0x6100 == 0x4100 {
                    self.chr[0] = (self.chr[0] & 0xC7) | ((v & 0x07) << 3);
                    self.prg[0] = (self.prg[0] & 0xFB) | ((v & 0x08) >> 1);
                }
            }
            // Mapper 133.
            28 => {
                if address & 0x6100 == 0x4100 {
                    self.chr[0] = (self.chr[0] & 0xE7) | ((v & 0x03) << 3);
                    self.prg[0] = (self.prg[0] & 0xFB) | (v & 0x04);
                }
            }
            // Mapper 184.
            31 => {
                if address & 0x6000 == 0x6000 {
                    self.chr[0] = (self.chr[0] & 0xE3) | ((v & 0x07) << 2);
                    self.chr[4] = (self.chr[4] & 0xE3) | ((v & 0x30) >> 2) | 0x10;
                }
            }
            // Mapper 38.
            32 if address & 0x7000 == 0x7000 => {
                self.prg[0] = (self.prg[0] & 0xF7) | ((v & 0x03) << 2);
                self.chr[0] = (self.chr[0] & 0xE7) | ((v & 0x0C) << 1);
            }
            _ => {}
        }
    }

    /// `mapper15_cpu_wr_low`: the MMC5 registers the FPGA decodes.
    fn write_mmc5(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address {
            0x5105 => {
                if value == 0xFF {
                    self.fscreen = true;
                } else {
                    self.fscreen = false;
                    self.mirroring = match ((value >> 2) & 0x01) | ((value >> 3) & 0x02) {
                        0 => 2,
                        1 => 0,
                        2 => 1,
                        _ => 3,
                    };
                }
            }
            0x5115 => {
                self.prg[0] = v & 0x1E;
                self.prg[1] = (v & 0x1E) | 1;
            }
            0x5116 => self.prg[2] = v & 0x1F,
            0x5117 => self.prg[3] = v & 0x1F,
            0x5120..=0x5123 => self.chr[usize::from(address & 0x03)] = v,
            0x5128..=0x512B => self.chr[4 + usize::from(address & 0x03)] = v,
            0x5203 => {
                self.irq = false;
                self.mmc5_irq_out = false;
                self.mmc5_irq_line = value;
            }
            0x5204 => {
                self.irq = false;
                self.mmc5_irq_out = false;
                self.mmc5_irq_enabled = value & 0x80 != 0;
            }
            _ => {}
        }
    }

    /// `flash_write_342`: the command buffer of up to ten writes, the sector erase (128 KiB at
    /// `prg.a`), the byte program and the CFI mode; `$F0` resets.
    fn write_flash(&mut self, address: u16, value: u8) {
        if self.flash_state < 10 {
            let state = usize::from(self.flash_state);
            self.flash_a[state] = address & 0x0FFF;
            self.flash_v[state] = value;
            self.flash_state += 1;
            let unlock = |a: &[u16; 10], v: &[u8; 10], third: u8| {
                a[0] == 0x0AAA
                    && v[0] == 0xAA
                    && a[1] == 0x0555
                    && v[1] == 0x55
                    && a[2] == 0x0AAA
                    && v[2] == third
            };
            let sector = usize::try_from(self.prg_a_mapped).unwrap_or(0) * 0x2000;
            if self.flash_state == 6
                && unlock(&self.flash_a, &self.flash_v, 0x80)
                && self.flash_a[3] == 0x0AAA
                && self.flash_v[3] == 0xAA
                && self.flash_a[4] == 0x0555
                && self.flash_v[4] == 0x55
                && self.flash_v[5] == 0x30
            {
                for offset in (sector..sector + 0x2_0000).step_by(SAVE_PAGE) {
                    self.save_page_mut(offset % SAVE_SIZE).fill(0xFF);
                }
                self.flash_state = 0;
            }
            if self.flash_state == 4 && unlock(&self.flash_a, &self.flash_v, 0xA0) {
                let offset = (sector + usize::from(address) % 0x8000) % SAVE_SIZE;
                self.save_page_mut(offset)[offset % SAVE_PAGE] = value;
                self.flash_state = 0;
            }
            if self.flash_state == 1 && self.flash_a[0] == 0x0AAA && self.flash_v[0] == 0x98 {
                self.cfi_mode = true;
                self.fix_prg();
                self.flash_state = 0;
                return;
            }
        }
        if value == 0xF0 {
            self.flash_state = 0;
            self.cfi_mode = false;
            self.fix_prg();
        }
    }

    /// The save-area page holding `offset`, allocated (zeroes, as puNES's buffer) on first use.
    fn save_page_mut(&mut self, offset: usize) -> &mut [u8] {
        let page = (offset / SAVE_PAGE) % self.save.len();
        self.save[page].get_or_insert_with(|| vec![0; SAVE_PAGE].into_boxed_slice())
    }

    /// A byte of the save area.
    fn save_byte(&self, offset: usize) -> u8 {
        self.save[(offset / SAVE_PAGE) % self.save.len()]
            .as_ref()
            .map_or(0, |page| page[offset % SAVE_PAGE])
    }

    /// The per-mapper writes at `$8000-$FFFF` (`mapperNN_cpu_wr_high`).
    #[allow(clippy::too_many_lines)]
    fn write_high(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match self.mapper {
            // UxROM; flag 0 the Fire Hawk's mirroring register (mapper 71).
            1 => {
                if self.flags & 0x01 == 0 || address & 0x7000 != 0x1000 {
                    self.prg[0] = (self.prg[0] & 0xC1) | ((v & 0x1F) << 1);
                    if self.flags & 0x02 != 0 {
                        self.mirroring = 0x02 | (value >> 7);
                        self.chr[0] = (self.chr[0] & 0xFC) | ((v & 0x60) >> 5);
                    }
                } else {
                    self.mirroring = 0x02 | ((value >> 4) & 0x01);
                }
            }
            // CNROM.
            2 => self.chr[0] = (self.chr[0] & 0x07) | ((v & 0x1F) << 3),
            // Mapper 78.
            3 => {
                self.prg[0] = (self.prg[0] & 0xF1) | ((v & 0x07) << 1);
                self.chr[0] = (self.chr[0] & 0x87) | ((v & 0xF0) >> 1);
                self.mirroring = ((value >> 3) & 0x01) ^ 1;
            }
            // Mapper 97.
            4 => {
                self.prg[0] = (self.prg[0] & 0xC1) | ((v & 0x1F) << 1);
                self.mirroring = (value >> 7) ^ 0x01;
            }
            // Mapper 93.
            5 => {
                self.prg[0] = (self.prg[0] & 0xF1) | ((v & 0x70) >> 3);
                self.chr_can_write = value & 0x01 != 0;
            }
            7 => self.write_18(address, value),
            // AxROM / BNROM.
            8 => {
                self.prg[0] = (self.prg[0] & 0xC3) | ((v & 0x0F) << 2);
                if self.flags & 0x01 == 0 {
                    self.mirroring = 0x02 | ((value >> 4) & 0x01);
                }
            }
            // Mapper 228.
            9 => {
                self.prg[0] = (self.prg[0] & 0xC3) | ((address & 0x0780) >> 5);
                self.chr[0] = (self.chr[0] & 0x07) | ((address & 0x0007) << 5) | ((v & 0x03) << 3);
                self.mirroring = u8::from((address >> 13) & 0x01 != 0);
            }
            // Color Dreams.
            10 => {
                self.prg[0] = (self.prg[0] & 0xF3) | ((v & 0x03) << 2);
                self.chr[0] = (self.chr[0] & 0x87) | ((v & 0xF0) >> 1);
            }
            // GxROM.
            11 => {
                self.prg[0] = (self.prg[0] & 0xF3) | ((v & 0x30) >> 2);
                self.chr[0] = (self.chr[0] & 0xE7) | ((v & 0x03) << 3);
            }
            13 => self.write_90(address, value),
            14 => self.write_65(address, value),
            16 => self.write_mmc1(address, value),
            // MMC2 / MMC4 (flag 0).
            17 => match (address >> 12) & 0x0007 {
                2 => {
                    self.prg[0] = if self.flags & 0x01 == 0 {
                        (self.prg[0] & 0xF0) | (v & 0x0F)
                    } else {
                        (self.prg[0] & 0xE1) | ((v & 0x0F) << 1)
                    };
                }
                slot @ 3..=6 => {
                    let index = [0, 1, 4, 5][usize::from(slot - 3)];
                    self.chr[index] = (self.chr[index] & 0x83) | ((v & 0x1F) << 2);
                }
                7 => self.mirroring = value & 0x01,
                _ => {}
            },
            // Mapper 152.
            18 => {
                self.chr[0] = (self.chr[0] & 0x87) | ((v & 0x0F) << 3);
                self.prg[0] = (self.prg[0] & 0xF1) | ((v & 0x70) >> 3);
                self.mirroring = 0x02 | (value >> 7);
            }
            19 => self.write_vrc3(address, value),
            20 => self.write_mmc3(address, value),
            21 => self.write_112(address, value),
            22 => self.write_taito(address, value),
            23 => self.write_42(address, value),
            24 => self.write_vrc24(address, value),
            25 => self.write_fme7(address, value),
            26 => self.write_g101(address, value),
            // Mapper 36 (TXC 01-22000-400).
            29 => {
                if address & 0x7FFE == 0x7FFE {
                    self.prg[0] = (self.prg[0] & 0xC3) | ((v & 0xF0) >> 2);
                    self.chr[0] = (self.chr[0] & 0x87) | ((v & 0x0F) << 3);
                }
            }
            // Mapper 70.
            30 => {
                self.prg[0] = (self.prg[0] & 0xE1) | ((v & 0xF0) >> 3);
                self.chr[0] = (self.chr[0] & 0x87) | ((v & 0x0F) << 3);
            }
            34 => self.write_vrc1(address, value),
            35 => self.write_83(address, value),
            36 => self.write_67(address, value),
            // Mapper 89.
            37 => {
                self.prg[0] = (self.prg[0] & 0xF1) | ((v & 0x70) >> 3);
                self.chr[0] = (self.chr[0] & 0x87) | ((v & 0x80) >> 1) | ((v & 0x07) << 3);
                self.mirroring = 0x02 | ((value & 0x08) >> 3);
            }
            _ => {}
        }
    }

    /// Sets the low (`high` false) or high nibble of a register.
    const fn nibble(register: u16, value: u8, high: bool) -> u16 {
        let value = (value & 0x0F) as u16;
        if high {
            (register & 0x0F) | (value << 4)
        } else {
            (register & 0xF0) | value
        }
    }

    /// `mapper07_cpu_wr_high`: mapper 18.
    fn write_18(&mut self, address: u16, value: u8) {
        let index = ((address & 0x7000) >> 10) | (address & 0x03);
        let high = index & 1 != 0;
        match index {
            0..=5 => {
                let slot = usize::from(index >> 1);
                self.prg[slot] = Self::nibble(self.prg[slot], value, high);
            }
            8..=23 => {
                let slot = usize::from((index - 8) >> 1);
                self.chr[slot] = Self::nibble(self.chr[slot], value, high);
            }
            24..=27 => {
                let shift = (index - 24) * 4;
                self.m18_latch =
                    (self.m18_latch & !(0x0F << shift)) | (u16::from(value & 0x0F) << shift);
            }
            28 => {
                self.m18_value = self.m18_latch;
                self.irq = false;
            }
            29 => {
                self.m18_control = value & 0x0F;
                self.irq = false;
            }
            30 => {
                self.mirroring = match value & 0x03 {
                    0 => 1,
                    1 => 0,
                    other => other,
                };
            }
            _ => {}
        }
    }

    /// `mapper13_cpu_wr_high`: mapper 90 (J.Y.), its MMC3-style IRQ.
    fn write_90(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address & 0xF000 {
            0x8000 => {
                let slot = usize::from(address & 0x03);
                self.prg[slot] = (self.prg[slot] & 0xC0) | (v & 0x3F);
            }
            0x9000 => self.chr[usize::from(address & 0x07)] = v,
            0xC000 => match address & 0x0007 {
                0 => {
                    self.mmc3_irq_enabled = value & 0x01 != 0;
                    if !self.mmc3_irq_enabled {
                        self.irq = false;
                    }
                }
                2 => {
                    self.mmc3_irq_enabled = false;
                    self.irq = false;
                }
                3 => self.mmc3_irq_enabled = true,
                5 => {
                    self.mmc3_irq_latch = value ^ self.m90_xor;
                    self.mmc3_irq_reload = true;
                }
                6 => self.m90_xor = value,
                _ => {}
            },
            0xD000 if address & 0x0003 == 0x0001 => self.mirroring = value & 0x03,
            _ => {}
        }
    }

    /// `mapper14_cpu_wr_high`: mapper 65 (Irem H3001).
    fn write_65(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match ((address & 0x7000) >> 9) | (address & 0x0007) {
            0 => self.prg[0] = (self.prg[0] & 0xC0) | (v & 0x3F),
            9 => self.mirroring = (value >> 7) & 0x01,
            11 => {
                self.m65_enabled = value & 0x80 != 0;
                self.irq = false;
            }
            12 => {
                self.m65_value = self.m65_latch;
                self.irq = false;
            }
            13 => self.m65_latch = (self.m65_latch & 0x00FF) | (v << 8),
            14 => self.m65_latch = (self.m65_latch & 0xFF00) | v,
            16 => self.prg[1] = (self.prg[1] & 0xC0) | (v & 0x3F),
            index @ 24..=31 => self.chr[usize::from(index - 24)] = v,
            32 => self.prg[2] = (self.prg[2] & 0xC0) | (v & 0x3F),
            _ => {}
        }
    }

    /// `mapper16_cpu_wr_high`: the MMC1's serial port.
    fn write_mmc1(&mut self, address: u16, value: u8) {
        if value & 0x80 != 0 {
            self.mmc1_load = (self.mmc1_load & 0xC0) | 0x20;
            self.prg_mode = 0;
            self.prg[2] = (self.prg[2] & 0xE0) | 0x1E;
            return;
        }
        self.mmc1_load =
            (self.mmc1_load & 0xC0) | ((value & 0x01) << 5) | ((self.mmc1_load & 0x3E) >> 1);
        if self.mmc1_load & 0x01 == 0 {
            return;
        }
        let load = u16::from(self.mmc1_load);
        match (address >> 13) & 0x0003 {
            0 => {
                if load & 0x18 == 0x18 {
                    self.prg_mode = 0;
                    self.prg[2] = (self.prg[2] & 0xE1) | 0x1E;
                } else if load & 0x18 == 0x10 {
                    self.prg_mode = 1;
                    self.prg[2] &= 0xE1;
                } else {
                    self.prg_mode = 7;
                }
                self.chr_mode = if load & 0x20 != 0 { 4 } else { 0 };
                self.mirroring = ((self.mmc1_load >> 1) & 0x03) ^ 0x02;
            }
            1 => {
                self.chr[0] = (self.chr[0] & 0x83) | ((load & 0x3E) << 1);
                self.prg[0] = (self.prg[0] & 0xDF) | (load & 0x20);
                self.prg[2] = (self.prg[2] & 0xDF) | (load & 0x20);
            }
            2 => self.chr[4] = (self.chr[4] & 0x83) | ((load & 0x3E) << 1),
            _ => {
                self.prg[0] = (self.prg[0] & 0xE1) | (load & 0x1E);
                self.wram_enabled = (self.mmc1_load >> 5) & 0x01 == 0;
            }
        }
        self.mmc1_load = 0x20;
    }

    /// `mapper19_cpu_wr_high`: mapper 73 (VRC3).
    fn write_vrc3(&mut self, address: u16, value: u8) {
        match address & 0x7000 {
            area @ 0x0000..=0x3000 => {
                let shift = (area >> 12) * 4;
                self.vrc3_latch =
                    (self.vrc3_latch & !(0x0F << shift)) | (u16::from(value & 0x0F) << shift);
            }
            0x4000 => {
                self.vrc3_control = (self.vrc3_control & 0xF8) | (value & 0x07);
                if self.vrc3_control & 0x02 != 0 {
                    self.vrc3_value = self.vrc3_latch;
                }
                self.irq = false;
            }
            0x5000 => {
                self.vrc3_control = (self.vrc3_control & 0xFD) | ((self.vrc3_control & 0x01) << 1);
                self.irq = false;
            }
            0x7000 => self.prg[0] = (self.prg[0] & 0xF1) | (u16::from(value & 0x07) << 1),
            _ => {}
        }
    }

    /// `mapper20_cpu_wr_high`: the MMC3 (flag 0 TxSROM, flag 1 mapper 189, flag 2 fixed
    /// modes).
    fn write_mmc3(&mut self, address: u16, value: u8) {
        match ((address & 0x6000) >> 12) | (address & 0x0001) {
            0 => {
                self.mmc3_internal = (self.mmc3_internal & 0xF8) | (value & 0x07);
                if self.flags & 0x06 == 0 {
                    self.prg_mode = if value & 0x40 != 0 { 5 } else { 4 };
                }
                if self.flags & 0x04 == 0 {
                    self.chr_mode = if value & 0x80 != 0 { 3 } else { 2 };
                }
            }
            1 => {
                let register = self.mmc3_internal & 0x07;
                match register {
                    0 => self.chr[0] = u16::from(value),
                    1 => self.chr[2] = u16::from(value),
                    2..=5 => self.chr[usize::from(register) + 2] = u16::from(value),
                    6 if self.flags & 0x02 == 0 => self.prg[0] = u16::from(value),
                    7 if self.flags & 0x02 == 0 => self.prg[1] = u16::from(value),
                    _ => {}
                }
                // TxSROM: CHR register D7 picks the CIRAM page of the tables it covers.
                if self.flags & 0x01 != 0 {
                    let page = (value >> 7) ^ 0x01;
                    let pages = self.nt_pages.get_or_insert([0, 0, 1, 1]);
                    match register {
                        0 | 1 if self.chr_mode == 2 => {
                            let first = usize::from(register) * 2;
                            pages[first] = page;
                            pages[first + 1] = page;
                        }
                        2..=5 if self.chr_mode == 3 => pages[usize::from(register) - 2] = page,
                        _ => {}
                    }
                }
            }
            2 => {
                if self.flags & 0x04 == 0 {
                    self.mirroring = value & 0x01;
                }
            }
            4 => self.mmc3_irq_latch = value,
            5 => self.mmc3_irq_reload = true,
            6 => {
                self.mmc3_irq_enabled = false;
                self.irq = false;
            }
            7 => self.mmc3_irq_enabled = true,
            _ => {}
        }
    }

    /// `mapper21_cpu_wr_high`: mapper 112.
    fn write_112(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address & 0xE000 {
            0x8000 => self.m112_internal = (self.m112_internal & 0xF8) | (value & 0x07),
            0xA000 => match self.m112_internal & 0x07 {
                0 => self.prg[0] = (self.prg[0] & 0xC0) | (v & 0x3F),
                1 => self.prg[1] = (self.prg[1] & 0xC0) | (v & 0x3F),
                2 => self.chr[0] = v,
                3 => self.chr[2] = v,
                register => self.chr[usize::from(register)] = v,
            },
            0xE000 => self.mirroring = value & 0x01,
            _ => {}
        }
    }

    /// `mapper22_cpu_wr_high`: mappers 33 and 48 (Taito; flag 0 the 48).
    fn write_taito(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match ((address & 0x6000) >> 11) | (address & 0x0003) {
            0 => {
                self.prg[0] = (self.prg[0] & 0xC0) | (v & 0x3F);
                if self.flags & 0x01 == 0 {
                    self.mirroring = (value >> 6) & 0x01;
                }
            }
            1 => self.prg[1] = (self.prg[1] & 0xC0) | (v & 0x3F),
            2 => self.chr[0] = v << 1,
            3 => self.chr[2] = v << 1,
            index @ 4..=7 => self.chr[usize::from(index)] = v,
            12 => {
                if self.flags & 0x01 != 0 {
                    self.mirroring = (value >> 6) & 0x01;
                }
            }
            8 => self.mmc3_irq_latch = value ^ 0xFF,
            9 => self.mmc3_irq_reload = true,
            10 => self.mmc3_irq_enabled = true,
            11 => {
                self.mmc3_irq_enabled = false;
                self.irq = false;
            }
            _ => {}
        }
    }

    /// `mapper23_cpu_wr_high`: mapper 42.
    fn write_42(&mut self, address: u16, value: u8) {
        match ((address >> 12) & 0x0004) | (address & 0x0003) {
            0 => self.chr[0] = (self.chr[0] & 0xE0) | (u16::from(value & 0x1F) << 3),
            4 => self.prg_b6000 = (self.prg_b6000 & 0xF0) | u16::from(value & 0x0F),
            5 => self.mirroring = (value >> 3) & 0x01,
            6 => {
                self.m42_enabled = value & 0x02 != 0;
                if !self.m42_enabled {
                    self.m42_value = 0;
                    self.irq = false;
                }
            }
            _ => {}
        }
    }

    /// `mapper24_cpu_wr_high`: the VRC2/VRC4 (flag 0 swaps A0/A1, flag 1 halves the CHR
    /// banks), each index line the OR of four address lines.
    fn write_vrc24(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        let hi = u16::from(
            (address >> 1) & 0x01 != 0
                || (address >> 3) & 0x01 != 0
                || (address >> 5) & 0x01 != 0
                || (address >> 7) & 0x01 != 0,
        );
        let low = u16::from(
            address & 0x01 != 0
                || (address >> 2) & 0x01 != 0
                || (address >> 4) & 0x01 != 0
                || (address >> 6) & 0x01 != 0,
        );
        let (first, second) = if self.flags & 0x01 != 0 {
            (low, hi)
        } else {
            (hi, low)
        };
        let index = ((address >> 10) & 0x1C) | (first << 1) | second;
        match index {
            0..=3 => self.prg[0] = (self.prg[0] & 0xE0) | (v & 0x1F),
            4 | 5 => {
                // VRC2 games write 0 or 1, but *Wai Wai World* writes `$FF` once.
                if value != 0xFF {
                    self.mirroring = value & 0x03;
                }
            }
            6 | 7 => self.prg_mode = (self.prg_mode & 0xFE) | ((value >> 1) & 0x01),
            8..=11 => self.prg[1] = (self.prg[1] & 0xE0) | (v & 0x1F),
            12..=27 => {
                let slot = usize::from((index - 12) >> 1);
                self.chr[slot] = Self::nibble(self.chr[slot], value, index & 1 != 0);
            }
            _ => {}
        }
        if address & 0x7000 == 0x7000 {
            match (first << 1) | second {
                0 => self.vrc4_latch = (self.vrc4_latch & 0xF0) | (value & 0x0F),
                1 => self.vrc4_latch = (self.vrc4_latch & 0x0F) | ((value & 0x0F) << 4),
                2 => {
                    self.vrc4_control = (self.vrc4_control & 0xF8) | (value & 0x07);
                    if self.vrc4_control & 0x02 != 0 {
                        self.vrc4_prescaler_counter = 0;
                        self.vrc4_prescaler = 0;
                        self.vrc4_value = self.vrc4_latch;
                    }
                    self.irq = false;
                }
                _ => {
                    self.vrc4_control =
                        (self.vrc4_control & 0xFD) | ((self.vrc4_control & 0x01) << 1);
                    self.irq = false;
                }
            }
        }
    }

    /// `mapper25_cpu_wr_high`: mapper 69 (Sunsoft FME-7).
    fn write_fme7(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match (address & 0x6000) >> 13 {
            0 => self.m69_internal = (self.m69_internal & 0xF0) | (value & 0x0F),
            1 => match self.m69_internal & 0x0F {
                register @ 0..=7 => self.chr[usize::from(register)] = v,
                8 => {
                    self.wram_enabled = value & 0x80 != 0;
                    self.map_rom_on_6000 = (value >> 6) & 0x01 == 0;
                    self.prg_b6000 = v & 0x3F;
                }
                register @ 9..=11 => {
                    let slot = usize::from(register - 9);
                    self.prg[slot] = (self.prg[slot] & 0xC0) | (v & 0x3F);
                }
                12 => self.mirroring = value & 0x03,
                13 => {
                    self.m69_counter_enabled = value & 0x80 != 0;
                    self.m69_irq_enabled = value & 0x01 != 0;
                    self.irq = false;
                }
                14 => self.m69_value = (self.m69_value & 0xFF00) | v,
                _ => self.m69_value = (self.m69_value & 0x00FF) | (v << 8),
            },
            _ => {}
        }
    }

    /// `mapper26_cpu_wr_high`: mapper 32 (Irem G-101).
    fn write_g101(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address & 0xF000 {
            0x8000 => self.prg[0] = (self.prg[0] & 0xC0) | (v & 0x3F),
            0x9000 => {
                self.prg_mode = (self.prg_mode & 0x06) | ((value >> 1) & 0x01);
                self.mirroring = value & 0x01;
            }
            0xA000 => self.prg[1] = (self.prg[1] & 0xC0) | (v & 0x3F),
            0xB000 => self.chr[usize::from(address & 0x07)] = v,
            _ => {}
        }
    }

    /// `mapper34_cpu_wr_high`: mapper 75 (VRC1). `chr.e` takes `chr.a`'s other bits, as
    /// puNES writes it.
    fn write_vrc1(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address & 0x7000 {
            0x0000 => self.prg[0] = (self.prg[0] & 0xF0) | (v & 0x0F),
            0x1000 => {
                self.mirroring = value & 0x01;
                self.chr[0] = (self.chr[0] & 0xBF) | ((v & 0x02) << 5);
                self.chr[4] = (self.chr[0] & 0xBF) | ((v & 0x04) << 4);
            }
            0x2000 => self.prg[1] = (self.prg[1] & 0xF0) | (v & 0x0F),
            0x4000 => self.prg[2] = (self.prg[2] & 0xF0) | (v & 0x0F),
            0x6000 => self.chr[0] = (self.chr[0] & 0xC3) | ((v & 0x0F) << 2),
            0x7000 => self.chr[4] = (self.chr[4] & 0xC3) | ((v & 0x0F) << 2),
            _ => {}
        }
    }

    /// `mapper35_cpu_wr_high`: mapper 83 (Cony/Yoko).
    fn write_83(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        match address & 0x0300 {
            0x0100 => {
                self.mirroring = value & 0x03;
                self.m83_enabled_latch = value & 0x80 != 0;
            }
            0x0200 => {
                if address & 0x0001 == 0 {
                    self.m83_counter = (self.m83_counter & 0xFF00) | v;
                    self.irq = false;
                } else {
                    self.m83_enabled = self.m83_enabled_latch;
                    self.m83_counter = (self.m83_counter & 0x00FF) | (v << 8);
                }
            }
            0x0300 => {
                if address & 0x0010 == 0 {
                    match address & 0x0003 {
                        0 => self.prg[0] = v,
                        1 | 2 => self.prg[1] = v,
                        _ => {}
                    }
                } else {
                    self.chr[usize::from(address & 0x07)] = v;
                }
            }
            _ => {}
        }
    }

    /// `mapper36_cpu_wr_high`: mapper 67 (Sunsoft-3).
    fn write_67(&mut self, address: u16, value: u8) {
        let v = u16::from(value);
        if address & 0x0800 == 0 {
            // Interrupt acknowledge ($8000).
            self.irq = false;
            return;
        }
        match address & 0x7000 {
            area @ 0x0000..=0x3000 => {
                let slot = usize::from(area >> 12) * 2;
                self.chr[slot] = (self.chr[slot] & 0x81) | ((v & 0x3F) << 1);
            }
            0x4000 => {
                self.m67_latch = !self.m67_latch;
                self.m67_counter = if self.m67_latch != 0 {
                    (self.m67_counter & 0x00FF) | (v << 8)
                } else {
                    (self.m67_counter & 0xFF00) | v
                };
            }
            0x5000 => {
                self.m67_latch = 0;
                self.m67_enabled = value & 0x10 != 0;
            }
            0x6000 => self.mirroring = value & 0x03,
            _ => self.prg[0] = (self.prg[0] & 0xE1) | ((v & 0x0F) << 1),
        }
    }

    /// `extcl_cpu_rd_mem_342` below `$8000` and its side effect (`$5204`'s acknowledge):
    /// `None` when the read is not the board's (`$6000-$7FFF` and open bus).
    pub(crate) fn read_register(&mut self, address: u16) -> Option<u8> {
        let value = self.peek_register(address)?;
        if self.mapper == 15 && address == 0x5204 {
            self.mmc5_irq_out = false;
            self.irq = false;
        }
        Some(value)
    }

    /// The same without side effects.
    pub(crate) fn peek_register(&self, address: u16) -> Option<u8> {
        if self.mapper == 29 && address & 0xE100 == 0x4100 {
            return Some(u8::try_from((self.prg[0] & 0x0C) << 2).unwrap_or(0));
        }
        if address & 0xF000 != 0x5000 {
            return None;
        }
        let m = &self.m163;
        match self.mapper {
            0 => Some(0),
            6 if address & 0x7700 == 0x5100 => Some(m[3] | m[1] | m[2] | !m[4]),
            6 if address & 0x7700 == 0x5500 => Some(if m[6] & 0x01 != 0 { m[3] } else { m[2] }),
            13 if address == 0x5800 || address == 0x5801 => {
                let product = u16::from(self.m90_mul[0]) * u16::from(self.m90_mul[1]);
                Some(product.to_le_bytes()[usize::from(address & 1)])
            }
            15 if address == 0x5204 => {
                let in_frame = self.line.is_some_and(|line| line + 1 < 241);
                Some((u8::from(self.mmc5_irq_out) << 7) | if in_frame { 0x40 } else { 0 })
            }
            35 => Some(self.flags & 0x03),
            _ => None,
        }
    }

    /// A `$8000-$FFFF` byte: the CFI table in CFI mode, else the window's bank.
    pub(crate) fn read_prg(&self, rom: &[u8], address: u16) -> u8 {
        if self.cfi_mode {
            return CFI[usize::from(address & 0xFF)];
        }
        self.read_window(
            rom,
            self.windows[usize::from((address >> 13) & 0x03)],
            address,
        )
    }

    fn read_window(&self, rom: &[u8], window: Window, address: u16) -> u8 {
        let low = usize::from(address & 0x1FFF);
        match window {
            Window::Rom(bank) => {
                if rom.is_empty() {
                    return 0;
                }
                rom[(usize::try_from(bank).unwrap_or(0) * 0x2000 + low) % rom.len()]
            }
            Window::Save(bank) => self.save_byte(usize::try_from(bank).unwrap_or(0) * 0x2000 + low),
        }
    }

    /// `$6000-$7FFF`: the work-RAM offset, or `None` while the window is a ROM bank.
    pub(crate) fn ram_offset(&self, address: u16) -> Option<usize> {
        match self.low {
            Window::Rom(page) if self.low_is_ram => {
                Some(usize::try_from(page).unwrap_or(0) * 0x2000 + usize::from(address & 0x1FFF))
            }
            _ => None,
        }
    }

    /// `$6000-$7FFF` as a ROM bank.
    pub(crate) fn read_low_rom(&self, rom: &[u8], address: u16) -> u8 {
        self.read_window(rom, self.low, address)
    }

    /// Whether the PPU's pattern writes reach the CHR-RAM.
    pub(crate) const fn chr_writable(&self) -> bool {
        self.chr_can_write
    }

    /// The CHR-RAM offset of a pattern address.
    pub(crate) fn chr_offset(&self, address: u16) -> usize {
        usize::try_from(self.chr_banks[usize::from(address >> 10) & 0x07]).unwrap_or(0) * 0x400
            + usize::from(address & 0x03FF)
    }

    /// `extcl_cpu_every_cycle_342`.
    pub(crate) fn step_cpu(&mut self) {
        match self.mapper {
            7 => self.step_18(),
            14 => {
                if self.m65_enabled && self.m65_value != 0 {
                    self.m65_value -= 1;
                    if self.m65_value == 0 {
                        self.irq = true;
                    }
                }
            }
            19 => self.step_vrc3(),
            23 => {
                if self.m42_enabled {
                    self.m42_value = self.m42_value.wrapping_add(1);
                    if self.m42_value >> 15 != 0 {
                        self.m42_value = 0;
                    }
                    self.irq = (self.m42_value >> 13) & 0x03 == 0x03;
                }
            }
            24 => self.step_vrc4(),
            25 => {
                if self.m69_counter_enabled {
                    self.m69_value = self.m69_value.wrapping_sub(1);
                    if self.m69_value == 0xFFFF {
                        self.irq = true;
                    }
                }
            }
            35 => {
                if self.m83_enabled {
                    if self.m83_counter == 0 {
                        self.irq = true;
                    }
                    self.m83_counter = self.m83_counter.wrapping_sub(1);
                }
            }
            36 if self.m67_enabled => {
                self.m67_counter = self.m67_counter.wrapping_sub(1);
                if self.m67_counter == 0xFFFF {
                    self.irq = true;
                    self.m67_enabled = false;
                }
            }
            _ => {}
        }
    }

    /// `mapper07_cpu_every_cycle`: mapper 18's counter, its width by the control nibble.
    fn step_18(&mut self) {
        if self.m18_control & 0x01 == 0 {
            return;
        }
        let nibble = |value: u16, shift: u16| u8::try_from((value >> shift) & 0x0F).unwrap_or(0);
        let mut carry = nibble(self.m18_value, 0).wrapping_sub(1);
        self.m18_value = (self.m18_value & 0xFFF0) | u16::from(carry & 0x0F);
        carry = (carry >> 4) & 0x01;
        for (shift, wide) in [(4u16, 0x08u8), (8, 0x0C), (12, 0x0E)] {
            if self.m18_control & wide == 0 {
                carry = nibble(self.m18_value, shift).wrapping_sub(carry);
                self.m18_value =
                    (self.m18_value & !(0x0F << shift)) | (u16::from(carry & 0x0F) << shift);
                carry = (carry >> 4) & 0x01;
            }
        }
        if carry != 0 {
            self.irq = true;
        }
    }

    /// `mapper19_cpu_every_cycle`: the VRC3's 8- or 16-bit up-counter.
    fn step_vrc3(&mut self) {
        if self.vrc3_control & 0x02 == 0 {
            return;
        }
        if self.vrc3_control & 0x04 != 0 {
            let low = self.vrc3_value.to_le_bytes()[0].wrapping_add(1);
            self.vrc3_value = (self.vrc3_value & 0xFF00) | u16::from(low);
            if low == 0 {
                self.vrc3_value = (self.vrc3_value & 0xFF00) | (self.vrc3_latch & 0xFF);
                self.irq = true;
            }
        } else {
            self.vrc3_value = self.vrc3_value.wrapping_add(1);
            if self.vrc3_value == 0 {
                self.vrc3_value = self.vrc3_latch;
                self.irq = true;
            }
        }
    }

    /// `mapper24_cpu_every_cycle`: the VRC4's prescaler (114, 114, 113) and counter.
    fn step_vrc4(&mut self) {
        if self.vrc4_control & 0x02 == 0 {
            return;
        }
        self.vrc4_prescaler = self.vrc4_prescaler.wrapping_add(1);
        let period = if self.vrc4_prescaler_counter & 0x02 == 0 {
            114
        } else {
            113
        };
        if self.vrc4_prescaler == period {
            self.vrc4_prescaler = 0;
            self.vrc4_prescaler_counter += 1;
            if self.vrc4_prescaler_counter == 3 {
                self.vrc4_prescaler_counter = 0;
            }
            self.vrc4_value = self.vrc4_value.wrapping_add(1);
            if self.vrc4_value == 0 {
                self.vrc4_value = self.vrc4_latch;
                self.irq = true;
            }
        }
    }

    /// A PPU fetch on the board's bus: MMC2/MMC4 latches (`extcl_after_rd_chr_342`, also on a
    /// `$2006` write), and the line tracker driving `extcl_ppu_000_to_34x_342`'s dot-260 work.
    pub(crate) fn on_ppu_bus(&mut self, address: u16, clock: i64) {
        if self.mapper == 17 {
            let latch = match address & 0xFFF0 {
                0x0FD0 => Some((0, 0)),
                0x0FE0 => Some((0, 1)),
                0x1FD0 => Some((1, 0)),
                0x1FE0 => Some((1, 1)),
                _ => None,
            };
            if let Some((window, value)) = latch {
                if window == 0 {
                    self.latch0 = value;
                } else {
                    self.latch1 = value;
                }
                self.fix_chr();
            }
        }
        // A gap longer than a line (V-blank, rendering off) leaves the frame.
        if clock - self.line_start > 2 * 341 {
            self.line = None;
            self.line_done = true;
        }
        if self.repeats >= 2 && (0x2000..0x3000).contains(&self.last_address) {
            // Three identical nametable reads ended a line; this fetch starts the next one.
            self.line = Some(self.line.map_or(0, |line| line + 1));
            self.line_start = clock;
            self.line_done = false;
            self.repeats = 0;
        } else if address == self.last_address {
            self.repeats = self.repeats.saturating_add(1);
        } else {
            self.repeats = 0;
        }
        self.last_address = address;
        if !self.line_done && clock - self.line_start >= 259 {
            self.line_done = true;
            if let Some(line) = self.line.filter(|line| *line < 240) {
                self.dot_260(line);
            }
        }
    }

    /// `extcl_ppu_000_to_34x_342` at dot 260 of rendered line `line`.
    fn dot_260(&mut self, line: u16) {
        if self.mmc3_irq_reload || self.mmc3_irq_counter == 0 {
            self.mmc3_irq_counter = self.mmc3_irq_latch;
            self.mmc3_irq_reload = false;
        } else {
            self.mmc3_irq_counter -= 1;
        }
        if self.mmc3_irq_counter == 0 && self.mmc3_irq_enabled {
            self.irq = true;
        }
        if u16::from(self.mmc5_irq_line) == line + 1 && self.mmc5_irq_enabled {
            self.mmc5_irq_out = true;
            self.irq = true;
        }
        if self.mapper == 6 {
            match line {
                239 => {
                    self.m163[0] = 0;
                    self.fix_chr();
                }
                127 => {
                    self.m163[0] = 1;
                    self.fix_chr();
                }
                _ => {}
            }
        }
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        let flags = [
            self.fscreen,
            self.lock,
            self.cfi_mode,
            self.map_rom_on_6000,
            self.wram_enabled,
            self.flash_can_write,
            self.chr_can_write,
            self.mmc3_irq_enabled,
            self.mmc3_irq_reload,
            self.mmc5_irq_enabled,
            self.mmc5_irq_out,
            self.m69_irq_enabled,
            self.m69_counter_enabled,
            self.m83_enabled_latch,
            self.m83_enabled,
            self.m65_enabled,
            self.m42_enabled,
            self.m67_enabled,
            self.irq,
        ];
        for flag in flags {
            writer.write_bool(flag);
        }
        writer.write_raw(&[
            self.mapper,
            self.mirroring,
            self.flags,
            self.wram_page,
            self.flash_state,
            self.prg_mode,
            self.prg_mask,
            self.chr_mode,
            self.chr_mask,
            self.mmc1_load,
            self.latch0,
            self.latch1,
            self.mmc3_internal,
            self.mmc3_irq_counter,
            self.mmc3_irq_latch,
            self.mmc5_irq_line,
            self.vrc3_control,
            self.vrc4_value,
            self.vrc4_control,
            self.vrc4_latch,
            self.vrc4_prescaler,
            self.vrc4_prescaler_counter,
            self.m69_internal,
            self.m112_internal,
            self.m90_mul[0],
            self.m90_mul[1],
            self.m90_xor,
            self.m18_control,
            self.m67_latch,
        ]);
        writer.write_raw(&self.m163);
        writer.write_raw(&self.flash_v);
        for value in self
            .flash_a
            .iter()
            .chain(&self.prg)
            .chain(&self.chr)
            .chain(&[
                self.prg_b6000,
                self.prg_base,
                self.vrc3_value,
                self.vrc3_latch,
                self.m69_value,
                self.m83_counter,
                self.m18_value,
                self.m18_latch,
                self.m65_value,
                self.m65_latch,
                self.m42_value,
                self.m67_counter,
            ])
        {
            writer.write_u16(*value);
        }
        for window in self.windows.iter().chain(std::iter::once(&self.low)) {
            let (save, bank) = match window {
                Window::Rom(bank) => (false, *bank),
                Window::Save(bank) => (true, *bank),
            };
            writer.write_bool(save);
            writer.write_u32(bank);
        }
        writer.write_bool(self.low_is_ram);
        writer.write_bool(self.nt_pages.is_some());
        writer.write_raw(&self.nt_pages.unwrap_or_default());
        let pages: Vec<usize> = (0..self.save.len())
            .filter(|page| self.save[*page].is_some())
            .collect();
        writer.write_u32(u32::try_from(pages.len()).unwrap_or(0));
        for page in pages {
            writer.write_u32(u32::try_from(page).unwrap_or(0));
            if let Some(data) = &self.save[page] {
                writer.write_raw(data);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let mut flags = [false; 19];
        for flag in &mut flags {
            *flag = reader.read_bool()?;
        }
        [
            self.fscreen,
            self.lock,
            self.cfi_mode,
            self.map_rom_on_6000,
            self.wram_enabled,
            self.flash_can_write,
            self.chr_can_write,
            self.mmc3_irq_enabled,
            self.mmc3_irq_reload,
            self.mmc5_irq_enabled,
            self.mmc5_irq_out,
            self.m69_irq_enabled,
            self.m69_counter_enabled,
            self.m83_enabled_latch,
            self.m83_enabled,
            self.m65_enabled,
            self.m42_enabled,
            self.m67_enabled,
            self.irq,
        ] = flags;
        let mut bytes = [0u8; 29];
        read_exact(reader, &mut bytes)?;
        [
            self.mapper,
            self.mirroring,
            self.flags,
            self.wram_page,
            self.flash_state,
            self.prg_mode,
            self.prg_mask,
            self.chr_mode,
            self.chr_mask,
            self.mmc1_load,
            self.latch0,
            self.latch1,
            self.mmc3_internal,
            self.mmc3_irq_counter,
            self.mmc3_irq_latch,
            self.mmc5_irq_line,
            self.vrc3_control,
            self.vrc4_value,
            self.vrc4_control,
            self.vrc4_latch,
            self.vrc4_prescaler,
            self.vrc4_prescaler_counter,
            self.m69_internal,
            self.m112_internal,
            self.m90_mul[0],
            self.m90_mul[1],
            self.m90_xor,
            self.m18_control,
            self.m67_latch,
        ] = bytes;
        read_exact(reader, &mut self.m163)?;
        read_exact(reader, &mut self.flash_v)?;
        if self.flash_state > 10 {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: COOLGIRL flash state past its buffer.".to_owned(),
            ));
        }
        for value in &mut self.flash_a {
            *value = reader.read_u16()?;
        }
        for value in &mut self.prg {
            *value = reader.read_u16()?;
        }
        for value in &mut self.chr {
            *value = reader.read_u16()?;
        }
        for value in [
            &mut self.prg_b6000,
            &mut self.prg_base,
            &mut self.vrc3_value,
            &mut self.vrc3_latch,
            &mut self.m69_value,
            &mut self.m83_counter,
            &mut self.m18_value,
            &mut self.m18_latch,
            &mut self.m65_value,
            &mut self.m65_latch,
            &mut self.m42_value,
            &mut self.m67_counter,
        ] {
            *value = reader.read_u16()?;
        }
        let mut windows = [Window::Rom(0); 5];
        for window in &mut windows {
            let save = reader.read_bool()?;
            let bank = reader.read_u32()?;
            *window = if save {
                Window::Save(bank)
            } else {
                Window::Rom(bank)
            };
        }
        self.windows = [windows[0], windows[1], windows[2], windows[3]];
        self.low = windows[4];
        self.low_is_ram = reader.read_bool()?;
        let has_pages = reader.read_bool()?;
        let mut pages = [0u8; 4];
        read_exact(reader, &mut pages)?;
        self.nt_pages = has_pages.then_some(pages);
        self.save = vec![None; SAVE_SIZE / SAVE_PAGE];
        let count = reader.read_u32()?;
        for _ in 0..count {
            let page = usize::try_from(reader.read_u32()?).unwrap_or(usize::MAX);
            if page >= self.save.len() {
                return Err(InvalidState::corrupt(
                    "Corrupt savestate: COOLGIRL save page out of range.".to_owned(),
                ));
            }
            let mut data = vec![0u8; SAVE_PAGE].into_boxed_slice();
            read_exact(reader, &mut data)?;
            self.save[page] = Some(data);
        }
        self.prg_a_mapped = self.mapped(self.prg[0]);
        self.fix_chr();
        Ok(())
    }

    /// The save file's tag and version.
    const SAVE_MAGIC: &'static [u8; 4] = b"ZXCG";
    const SAVE_VERSION: u8 = 1;

    /// The save area as a file: `ZXCG`, a version byte, then each written 8 KiB page as its
    /// 16-bit index (little-endian) and its bytes; empty when nothing was ever written.
    pub(crate) fn export_save(&self) -> Vec<u8> {
        let pages: Vec<(usize, &[u8])> = self
            .save
            .iter()
            .enumerate()
            .filter_map(|(index, page)| page.as_deref().map(|data| (index, data)))
            .collect();
        if pages.is_empty() {
            return Vec::new();
        }
        let mut file = Self::SAVE_MAGIC.to_vec();
        file.push(Self::SAVE_VERSION);
        for (index, data) in pages {
            file.extend_from_slice(&u16::try_from(index).unwrap_or(0).to_le_bytes());
            file.extend_from_slice(data);
        }
        file
    }

    /// Applies a file of [`Self::export_save`]; `false`, the area untouched, when it is not one.
    pub(crate) fn import_save(&mut self, data: &[u8]) -> bool {
        let Some(body) = data
            .strip_prefix(Self::SAVE_MAGIC.as_slice())
            .and_then(|rest| rest.strip_prefix(&[Self::SAVE_VERSION]))
        else {
            return false;
        };
        let record = 2 + SAVE_PAGE;
        if body.len() % record != 0 {
            return false;
        }
        let mut save = vec![None; SAVE_SIZE / SAVE_PAGE];
        for chunk in body.chunks_exact(record) {
            let index = usize::from(u16::from_le_bytes([chunk[0], chunk[1]]));
            let Some(slot) = save.get_mut(index) else {
                return false;
            };
            *slot = Some(chunk[2..].to_vec().into_boxed_slice());
        }
        self.save = save;
        true
    }

    /// Takes the save area of the board `old` replaces (a reset keeps the flash).
    pub(crate) fn keep_save(&mut self, old: &mut Self) {
        self.save = std::mem::take(&mut old.save);
    }
}

/// Fills `target` from the state.
fn read_exact(reader: &mut StateReader, target: &mut [u8]) -> Result<(), InvalidState> {
    target.copy_from_slice(reader.read_raw(target.len())?);
    Ok(())
}
