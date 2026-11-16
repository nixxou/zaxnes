//! Lot MAPPERS-P: the V.R. Technology "`OneBus`" famiclones (VT02/VT03) of mappers 256 and 436.
//!
//! Read from puNES 0.111 (`OneBus.c`, `mapper_256.c`, `mapper_436.c`; read, not copied). The
//! chip folds the cartridge's banking into the console: its PRG registers sit at `$4100-$41FF`,
//! its CHR registers among the PPU's at `$2010-$201F`, a PCM channel among the APU's at
//! `$4012`/`$4013`/`$4015` and `$4030`, and an MMC3-style register file at `$8000-$FFFF` is an
//! alias for them. [`super::IrqP`] carries the board (kinds `M256` and `M436`); this module holds
//! the chip's state and its decodes, and the three bus hooks the core gives these two numbers
//! alone (`Mapper::snoop_apu_write`, `Mapper::apu_status`, `Mapper::take_dac_write`) plus the
//! routing of every CPU write to `$2008-$3FFF` to the cartridge.
//!
//! Like puNES this models the VT02/VT03's ordinary 2 bpp rendering: the 4 bpp modes change only
//! which bytes the banks select (the planes puNES de-interleaves), not how the PPU draws them,
//! and the VT09/VT369 extras (GPIO, the `$3000` window, the relative bank) are absent
//! (FIDELITY-LEADS J37).
use crate::cartridges::CartridgeMemory;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// `regs[M256_SUBMAPPERS]` of `mapper_256.c`: how each submapper's wiring permutes the
/// `$4107-$410A` PRG registers, the MMC3-alias register numbers and the `$2012-$2017` CHR
/// registers. Submappers 6-15 are wired as 0.
const WIRING_256: [([u8; 4], [u8; 8], [u8; 6]); 6] = [
    ([0, 1, 2, 3], [0, 1, 2, 3, 4, 5, 6, 7], [0, 1, 2, 3, 4, 5]),
    ([0, 1, 2, 3], [5, 4, 3, 2, 1, 0, 6, 7], [1, 0, 5, 4, 3, 2]),
    ([1, 0, 2, 3], [0, 1, 2, 3, 4, 5, 7, 6], [0, 1, 2, 3, 4, 5]),
    ([0, 1, 2, 3], [0, 1, 2, 3, 4, 5, 6, 7], [5, 4, 3, 2, 0, 1]),
    ([0, 1, 2, 3], [0, 1, 2, 3, 4, 5, 6, 7], [2, 5, 0, 4, 3, 1]),
    ([0, 1, 2, 3], [0, 1, 2, 3, 4, 5, 6, 7], [1, 0, 5, 4, 3, 2]),
];

/// `chr_mask` of `chr_swap_OneBus_base`, by `$201A` D0-D2.
const CHR_MASK: [u16; 8] = [0xFF, 0x7F, 0x3F, 0x00, 0x1F, 0x0F, 0x07, 0x00];

/// The `OneBus` chip's state (`_onebus`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OneBus {
    /// `$4100-$41FF`.
    cpu: [u8; 0x100],
    /// `$2000-$20FF` as the chip decodes them (`$2008` and up).
    ppu: [u8; 0x100],
    /// `$4000-$403F`.
    apu: [u8; 0x40],
    /// The PCM channel: address, remaining bytes, cycle countdown, its period, whether it runs,
    /// and its IRQ flag (`$80` while asserted).
    pcm_address: i16,
    pcm_size: i16,
    pcm_latch: i16,
    pcm_clock: i16,
    pcm_enable: bool,
    pcm_irq: u8,
    /// The byte the PCM channel last fetched for the APU's `$4011`, until the bus takes it.
    dac: Option<u8>,
    /// The IRQ counter (`irqA12.counter`), its enable, and the line.
    pub(crate) irq_counter: u8,
    pub(crate) irq_enabled: bool,
    pub(crate) irq_pending: bool,
    /// 256 or 436, and the submapper (256's from the PRG-ROM's CRC on an iNES 1.0 image).
    mapper: u16,
    submapper: u8,
    /// The CHR windows and plane mode [`OneBus::chr_windows`] gives, derived after every write.
    pub(crate) windows: [usize; 8],
    pub(crate) planes: Option<u8>,
    /// The CHR data's length: the CHR-ROM's, or the PRG-ROM's on an image without one
    /// (`init_OneBus`: the chip fetches its patterns from the one ROM).
    data_len: usize,
}

impl OneBus {
    pub(crate) fn new(memory: &CartridgeMemory, mapper: u16) -> Self {
        let header = memory.rom().header();
        let mut submapper = if header.is_nes20 {
            u8::try_from(header.submapper & 0x0F).unwrap_or(0)
        } else {
            0
        };
        // `map_init_256`: the iNES 1.0 images puNES recognises by their PRG-ROM.
        if mapper == 256 && !header.is_nes20 {
            match crate::builtin_patches::crc32(memory.rom().prg_rom()) {
                0x947A_C898 | 0x1AB4_5228 => submapper = 2,
                0x1242_DA7F => submapper = 4,
                _ => {}
            }
        }
        let mut chip = Self {
            cpu: [0; 0x100],
            ppu: [0; 0x100],
            apu: [0; 0x40],
            pcm_address: 0,
            pcm_size: 0,
            pcm_latch: 0,
            pcm_clock: 0xE1,
            pcm_enable: false,
            pcm_irq: 0,
            dac: None,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
            mapper,
            submapper,
            windows: [0; 8],
            planes: None,
            data_len: {
                let rom = memory.rom();
                if rom.chr_rom().is_empty() {
                    rom.prg_rom().len()
                } else {
                    rom.chr_rom().len()
                }
            },
        };
        chip.reset_registers();
        chip.refresh_chr();
        chip
    }

    /// `init_OneBus`'s registers, set on every reset (`$2012-$2017` the identity layout, the PRG
    /// banks 0, 1, `$FE`, `$FF`).
    pub(crate) fn reset_registers(&mut self) {
        for (index, value) in [
            (0x10, 0x00),
            (0x12, 0x04),
            (0x13, 0x05),
            (0x14, 0x06),
            (0x15, 0x07),
            (0x16, 0x00),
            (0x17, 0x02),
            (0x18, 0x00),
            (0x1A, 0x00),
        ] {
            self.ppu[index] = value;
        }
        for (index, value) in [
            (0x00, 0x00),
            (0x05, 0x00),
            (0x07, 0x00),
            (0x08, 0x01),
            (0x09, 0xFE),
            (0x0A, 0x00),
            (0x0B, 0x00),
            (0x0F, 0xFF),
            (0x60, 0x00),
            (0x61, 0x00),
        ] {
            self.cpu[index] = value;
        }
    }

    /// The submapper's wiring (256 only; 436 and unknown submappers are straight).
    fn wiring(&self) -> ([u8; 4], [u8; 8], [u8; 6]) {
        if self.mapper == 256 {
            WIRING_256
                .get(usize::from(self.submapper))
                .copied()
                .unwrap_or(WIRING_256[0])
        } else {
            WIRING_256[0]
        }
    }

    /// A CPU write the chip decodes, through the board's wiring (`extcl_cpu_wr_mem_256`,
    /// `extcl_wr_ppu_reg_256`): `$2008-$3FFF`, `$4020-$41FF` and `$8000-$FFFF`. Returns whether
    /// the chip took it.
    pub(crate) fn write(&mut self, address: u16, value: u8) -> bool {
        let (cpu_wiring, mmc3_wiring, ppu_wiring) = self.wiring();
        match address {
            0x2008..=0x3FFF => {
                let address = if (0x2012..=0x2017).contains(&address) {
                    0x2012 + u16::from(ppu_wiring[usize::from(address - 0x2012)])
                } else {
                    address
                };
                self.write_ppu(address, value);
                true
            }
            0x4020..=0x403F => {
                self.apu[usize::from(address & 0x3F)] = value;
                true
            }
            0x4100..=0x41FF => {
                let address = if (0x4107..=0x410A).contains(&address) {
                    0x4107 + u16::from(cpu_wiring[usize::from(address - 0x4107)])
                } else {
                    address
                };
                self.write_cpu(address, value);
                true
            }
            0x8000..=0xFFFF => {
                let value = if address & 0xE001 == 0x8000 {
                    (value & 0xF8) | mmc3_wiring[usize::from(value & 0x07)]
                } else {
                    value
                };
                self.write_mmc3(address, value);
                true
            }
            _ => false,
        }
    }

    /// `extcl_wr_ppu_reg_OneBus`: `$2008` and up, A6 ignored on these chips.
    fn write_ppu(&mut self, address: u16, value: u8) {
        self.ppu[usize::from(address & !0x40) & 0xFF] = value;
    }

    /// `extcl_cpu_wr_mem_OneBus`'s `$4100-$41FF`.
    fn write_cpu(&mut self, address: u16, value: u8) {
        self.cpu[usize::from(address & 0xFF)] = value;
        match address & 0x01FF {
            0x101 => self.irq_counter = self.cpu[0x01],
            0x102 => self.irq_counter = 0,
            0x103 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            0x104 => self.irq_enabled = true,
            _ => {}
        }
    }

    /// `extcl_cpu_wr_mem_OneBus`'s `$8000-$FFFF`: the MMC3 aliases, unless `$410B` D3 turns them
    /// off.
    fn write_mmc3(&mut self, address: u16, value: u8) {
        if self.cpu[0x0B] & 0x08 != 0 {
            return;
        }
        match address & 0xE001 {
            0x8000 => self.write_cpu(0x4105, value & !0x20),
            0x8001 => match self.cpu[0x05] & 0x07 {
                reg @ (0 | 1) => self.write_ppu(0x2016 + u16::from(reg), value),
                reg @ 2..=5 => self.write_ppu(0x2010 + u16::from(reg), value),
                reg => self.write_cpu(0x4101 + u16::from(reg), value),
            },
            0xA000 => self.write_cpu(0x4106, value & 0x01),
            0xC000 => self.write_cpu(0x4101, value),
            0xC001 => self.write_cpu(0x4102, value),
            0xE000 => self.write_cpu(0x4103, value),
            _ => self.write_cpu(0x4104, value),
        }
    }

    /// `extcl_wr_apu_OneBus`: the PCM channel's registers while `$4030` D4 is set; the `$4015`
    /// value the APU sees loses D4 then, so its own DMC stays off.
    pub(crate) fn snoop_apu_write(&mut self, address: u16, value: u8) -> u8 {
        if address > 0x403F {
            return value;
        }
        self.apu[usize::from(address & 0x3F)] = value;
        if self.apu[0x30] & 0x10 == 0 {
            return value;
        }
        match address & 0x3F {
            0x12 => self.pcm_address = i16::from(value) << 6,
            0x13 => self.pcm_size = (i16::from(value) << 4) + 1,
            0x15 => {
                self.pcm_enable = value & 0x10 != 0;
                self.pcm_irq = 0;
                if self.pcm_enable {
                    self.pcm_latch = self.pcm_clock;
                }
                return value & 0xEF;
            }
            _ => {}
        }
        value
    }

    /// `extcl_rd_apu_OneBus`: `$4015` D7 is the PCM IRQ while `$4030` D4 is set.
    pub(crate) const fn apu_status(&self, status: u8) -> u8 {
        if self.apu[0x30] & 0x10 != 0 {
            (status & 0x7F) | self.pcm_irq
        } else {
            status
        }
    }

    /// `extcl_cpu_rd_mem_OneBus`: `$4100-$4FFF`, `None` for the open bus puNES reads there.
    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        match address & 0x0FFF {
            0x140..=0x15B | 0x15D..=0x15F => Some(0xFF),
            0x15C => Some(0x10),
            0x18A | 0x1B7 => Some(0x04),
            0x1B9 => Some(0x80),
            _ if address <= 0x410D || (0x4160..0x4800).contains(&address) => {
                Some(self.cpu[usize::from(address & 0xFF)])
            }
            _ => None,
        }
    }

    /// `extcl_cpu_every_cycle_OneBus`: one PCM byte every `clock` cycles, from the CPU address
    /// `address | (($4030 ^ 3) << 14)`, until the size runs out and the IRQ rises. The byte's
    /// upper seven bits go to the APU's `$4011`.
    pub(crate) fn step_pcm(&mut self, memory: &CartridgeMemory, prg: impl Fn(u16) -> u8) {
        if !self.pcm_enable {
            return;
        }
        self.pcm_latch -= 1;
        if self.pcm_latch > 0 {
            return;
        }
        self.pcm_latch += self.pcm_clock;
        self.pcm_size -= 1;
        if self.pcm_size < 0 {
            self.pcm_irq = 0x80;
            self.pcm_enable = false;
            return;
        }
        let address = u16::from_le_bytes(self.pcm_address.to_le_bytes())
            | (u16::from((self.apu[0x30] ^ 0x03) & 0x03) << 14);
        // Below the work RAM the console's own memory would answer, which the board cannot
        // read; that reads 0 here (J37).
        let byte = if address >= 0x8000 {
            prg(address)
        } else if address >= 0x6000 {
            let ram = memory.prg_ram();
            if ram.is_empty() {
                0
            } else {
                ram[usize::from(address & 0x1FFF) % ram.len()]
            }
        } else {
            0
        };
        self.dac = Some(byte >> 1);
        self.pcm_address = (self.pcm_address + 1) & 0x7FFF;
    }

    /// Recomputes [`OneBus::windows`] and [`OneBus::planes`].
    pub(crate) fn refresh_chr(&mut self) {
        let (windows, planes) = self.chr_windows(self.data_len);
        self.windows = windows;
        self.planes = planes;
    }

    pub(crate) fn take_dac(&mut self) -> Option<u8> {
        self.dac.take()
    }

    /// Whether the PCM channel's IRQ is up.
    pub(crate) const fn pcm_irq(&self) -> bool {
        self.pcm_irq != 0
    }

    /// `irq_tick_OneBus`: the MMC3's counter, without its reload flag.
    pub(crate) fn clock_irq(&mut self) {
        self.irq_counter = if self.irq_counter == 0 {
            self.cpu[0x01]
        } else {
            self.irq_counter - 1
        };
        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }

    /// `mirroring_fix_OneBus_base`.
    pub(crate) const fn mirroring(&self) -> NametableMirroring {
        if self.cpu[0x06] & 0x01 != 0 {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        }
    }

    /// The `mmask`/`mblock` pair the board applies to the PRG and CHR banks
    /// (`extcl_after_mapper_init_256`/`_436`): `(prg mask, prg block, chr mask, chr block)`.
    fn outer(&self) -> (u16, u16, u16, u16) {
        let c = &self.cpu;
        if self.mapper == 256 {
            return (0x0FFF, 0, 0x7FFF, 0);
        }
        if self.submapper == 1 {
            let full = c[0x1C] & 0x01 != 0 && c[0x0F] & 0x04 != 0;
            (
                0xF7FF,
                if full { 0 } else { 0x0800 },
                0xBFFF,
                if full { 0 } else { 0x4000 },
            )
        } else {
            (
                0xF3FF,
                (u16::from(c[0x00] & 0x40) << 5) | (u16::from(c[0x0F] & 0x20) << 5),
                0x9FFF,
                (u16::from(c[0x00] & 0x04) << 12) | (u16::from(c[0x0F] & 0x20) << 8),
            )
        }
    }

    /// `prg_fix_8k_OneBus_base`: the four 8 KiB PRG banks of `$8000-$FFFF`.
    pub(crate) fn prg_banks(&self) -> [u16; 4] {
        let c = &self.cpu;
        let (outer_mask, outer_block, _, _) = self.outer();
        let mode = c[0x0B] & 0x07;
        let inner = u16::from(if mode == 0x07 { 0xFFu8 } else { 0x3F } >> mode);
        let block = (u16::from(c[0x00] & 0xF0) << 4) | (u16::from(c[0x0A]) & !inner);
        let bank = |value: u8| outer_block | ((block | (u16::from(value) & inner)) & outer_mask);
        let third = if c[0x0B] & 0x40 != 0 { c[0x09] } else { 0xFE };
        let mut banks = [bank(c[0x07]), bank(c[0x08]), bank(third), bank(0xFF)];
        // `$4105` D6 swaps `$8000` and `$C000`.
        if c[0x05] & 0x40 != 0 {
            banks.swap(0, 2);
        }
        banks
    }

    /// `chr_fix_OneBus_base`/`chr_swap_OneBus_base`: the byte offset of each 1 KiB CHR window in
    /// the CHR data, and whether those offsets are in the de-interleaved low planes (`Some(16)`
    /// for `low16`, `Some(4)` for `low`) of the 4 bpp modes, `None` for the data itself.
    fn chr_windows(&self, data_len: usize) -> ([usize; 8], Option<u8>) {
        let (c, p) = (&self.cpu, &self.ppu);
        let (_, _, mut outer_mask, mut outer_block) = self.outer();
        let v16 = p[0x10] & 0x40 != 0 || c[0x2B] == 0x61;
        let four_bpp = p[0x10] & 0x06 != 0;
        let extended = p[0x10] & 0x18 != 0;
        let mask = CHR_MASK[usize::from(p[0x1A] & 0x07)];
        let mut block = u16::from(p[0x1A] & 0xF8) & !mask;
        let mut chr_mask = data_len.next_power_of_two().max(1) - 1;
        if four_bpp {
            chr_mask >>= 1;
            outer_mask >>= 1;
            outer_block >>= 1;
        }
        let eva = if extended {
            u16::from(p[0x18] & 0x07)
        } else {
            block |= u16::from(p[0x18] & 0x70) << 4;
            0
        };
        let outer = u16::from(c[0x00] & 0x0F) << 11;
        let point = |input: u8| {
            let bank =
                outer_block | ((block | (u16::from(input) & mask) | eva | outer) & outer_mask);
            (usize::from(bank) << 10) & chr_mask
        };
        let swap = usize::from((c[0x05] & 0x80) >> 5);
        let mut windows = [0; 8];
        for (slot, input) in [
            p[0x16] & 0xFE,
            p[0x16] | 0x01,
            p[0x17] & 0xFE,
            p[0x17] | 0x01,
            p[0x12],
            p[0x13],
            p[0x14],
            p[0x15],
        ]
        .into_iter()
        .enumerate()
        {
            windows[slot ^ swap] = point(input);
        }
        let planes = four_bpp.then_some(if v16 { 16 } else { 4 });
        (windows, planes)
    }

    /// A CHR byte: `windows[address >> 10] + (address & $3FF)` in the data, or in its low planes
    /// (`low`: the bytes with A4 clear; `low16`: the even bytes) in the 4 bpp modes.
    pub(crate) fn chr_byte(
        data: &[u8],
        windows: &[usize; 8],
        planes: Option<u8>,
        address: u16,
    ) -> u8 {
        if data.is_empty() {
            return 0;
        }
        let offset = windows[usize::from(address >> 10) & 0x07] + usize::from(address & 0x03FF);
        let offset = match planes {
            Some(16) => offset << 1,
            Some(_) => (offset & 0x0F) | ((offset & !0x0F) << 1),
            None => offset,
        };
        data[offset % data.len()]
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.cpu);
        writer.write_raw(&self.ppu);
        writer.write_raw(&self.apu);
        for value in [
            self.pcm_address,
            self.pcm_size,
            self.pcm_latch,
            self.pcm_clock,
        ] {
            writer.write_u16(u16::from_le_bytes(value.to_le_bytes()));
        }
        writer.write_bool(self.pcm_enable);
        writer.write_u8(self.pcm_irq);
        writer.write_u8(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.cpu)?;
        reader.read_into(&mut self.ppu)?;
        reader.read_into(&mut self.apu)?;
        for value in [
            &mut self.pcm_address,
            &mut self.pcm_size,
            &mut self.pcm_latch,
            &mut self.pcm_clock,
        ] {
            *value = i16::from_le_bytes(reader.read_u16()?.to_le_bytes());
        }
        self.pcm_enable = reader.read_bool()?;
        self.pcm_irq = reader.read_u8()?;
        self.irq_counter = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.dac = None;
        Ok(())
    }
}
