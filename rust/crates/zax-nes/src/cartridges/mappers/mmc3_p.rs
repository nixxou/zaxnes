//! Lot MAPPERS-P: the MMC3 multicarts puNES runs and Zaxnes did not (`docs/progress/MAPPERS-P.md`).
//!
//! The shape of lot MAPPERS-N's [`super::Mmc3Clone`]: an [`Mmc3`] that runs unchanged, and
//! [`Mmc3P::refresh`] recomputing the twelve windows from its registers and the board's outer
//! registers after every write. Each kind is puNES's pair of hooks, `prg_swap_mmc3_NNN` and
//! `chr_swap_mmc3_NNN` (puNES 0.111, `src/core/mappers/mapper_NNN.c`, read, not copied), which
//! receive the chip's 8 KiB and 1 KiB bank numbers — the units of [`BankWindows`] — and return the
//! board's.
//!
//! puNES writes `$6000-$7FFF` to the work RAM **and then** to the board (`cpu_inline.h`,
//! `wram_wr` before `extcl_cpu_wr_mem`), so an outer register in that range does not hide the RAM:
//! [`Mmc3P::write_low`] stores the byte in the RAM first when the chip allows it, then updates
//! the register, and answers `true` so that the cartridge refreshes its cached ROM windows.
//! `memmap_adr_is_writable` is the chip's RAM write enable ([`Mmc3::prg_ram_writable`]).
use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The chip's fixed second-last and last 8 KiB banks before a kind masks them, as
/// [`super::Mmc3Clone`] writes them.
const SECOND_LAST: i32 = 0xFFFE;
const LAST: i32 = 0xFFFF;

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mmc3PKind {
    /// 267, `mapper_267.c` (JY-119): `$6000-$6FFF` the outer bank, locked by its D7.
    M267,
    /// 291, `mapper_291.c` (Kasheng 2-in-1): `$6000-$6FFF` the outer bank and a 32 KiB NROM mode.
    M291,
    /// 315, `mapper_315.c` (820732C / 830134C): the outer bank, a GNROM mode, CHR A16-A17 ORed.
    M315,
    /// 322, `mapper_322.c` (K-3033): the address at `$6000` the outer bank, MMC3 or NROM mode.
    M322,
    /// 327, `mapper_327.c` (10-24-C-A1): the address the outer bank until its low bits lock it,
    /// a CHR-RAM mode.
    M327,
    /// 334, `mapper_334.c` (5/20-in-1 1993 Copyright): `$6000` even a 32 KiB bank, `$6002` a DIP
    /// read.
    M334,
    /// 339, `mapper_339.c` (K-3006): the address the outer bank, NROM-128/256 modes.
    M339,
    /// 345, `mapper_345.c` (L6IN1): the outer bank, an AxROM mode, single-screen mirroring.
    M345,
    /// 361, `mapper_361.c` (YY841101C): `$7000-$7FFF` the outer bank.
    M361,
    /// 372, `mapper_372.c` (SFC-12): mapper 45's four registers written in turn, a CHR-RAM mode.
    M372,
    /// 377, `mapper_377.c` (JY-111/JY-112): `$6000-$6FFF` the outer bank.
    M377,
}

impl Mmc3PKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            267 => Self::M267,
            291 => Self::M291,
            315 => Self::M315,
            322 => Self::M322,
            327 => Self::M327,
            334 => Self::M334,
            339 => Self::M339,
            345 => Self::M345,
            361 => Self::M361,
            372 => Self::M372,
            377 => Self::M377,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M267 => 267,
            Self::M291 => 291,
            Self::M315 => 315,
            Self::M322 => 322,
            Self::M327 => 327,
            Self::M334 => 334,
            Self::M339 => 339,
            Self::M345 => 345,
            Self::M361 => 361,
            Self::M372 => 372,
            Self::M377 => 377,
        }
    }
}

/// An MMC3 multicart of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3P {
    mmc3: Mmc3,
    kind: Mmc3PKind,
    /// The outer registers, their meaning per kind.
    outer: [u16; 4],
    /// Mapper 372's write cursor.
    index: u8,
    /// Whether the cartridge has CHR-RAM rather than CHR-ROM.
    chr_ram: bool,
}

impl Mmc3P {
    pub(crate) fn new(memory: &CartridgeMemory, kind: Mmc3PKind) -> Self {
        Self {
            mmc3: Mmc3::new(memory, kind.number()),
            kind,
            outer: [0; 4],
            index: 0,
            chr_ram: memory.rom().chr_rom().is_empty(),
        }
    }

    fn power_on(&mut self) {
        self.outer = [0; 4];
        self.index = 0;
        // `map_init_372`: "m372.reg[2] = 0x0F", the CHR mask fully open.
        if self.kind == Mmc3PKind::M372 {
            self.outer[2] = 0x0F;
        }
        self.refresh();
    }

    /// `prg_swap_mmc3_NNN`: the 8 KiB bank `page` of window `slot` becomes.
    fn prg_page(&self, slot: usize, page: i32) -> i32 {
        let r = i32::from(self.outer[0]);
        let window = i32::try_from(slot).unwrap_or(0);
        let mix = |base: i32, mask: i32, value: i32| (base & !mask) | (value & mask);
        match self.kind {
            Mmc3PKind::M267 => mix(((r & 0x20) << 2) | ((r & 0x06) << 4), 0x1F, page),
            // `prg_swap_mmc3_291`: D5 selects a 32 KiB bank from D1-D4 and D6.
            Mmc3PKind::M291 => {
                if r & 0x20 != 0 {
                    let base = (((r & 0x1F) >> 1) | ((r & 0x40) >> 4)) << 2;
                    base | window
                } else {
                    ((r & 0x40) >> 2) | (page & 0x0F)
                }
            }
            // `prg_swap_mmc3_315`: D1-D2 both set is the GNROM mode, the 32 KiB from R6/R7.
            Mmc3PKind::M315 => {
                let value = if r & 0x06 == 0x06 {
                    let register = i32::from(self.mmc3.bank_register(6 | (slot & 0x01)));
                    if slot < 2 {
                        register & 0xFD
                    } else {
                        register | 0x02
                    }
                } else {
                    page
                };
                mix(((r >> 1) & 0x03) << 4, 0x0F, value)
            }
            // `prg_swap_mmc3_322`: A5 the MMC3 mode, 128 KiB or (A7) 256 KiB; otherwise NROM.
            Mmc3PKind::M322 => {
                let mmc3_mode = r & 0x0020 != 0;
                let small = i32::from(!(r & 0x0080 != 0 && mmc3_mode));
                if mmc3_mode {
                    let base = (((r & 0x0040) >> 4) | ((r & 0x0018) >> 3)) << (5 - small);
                    mix(base, 0x1F >> small, page)
                } else {
                    let bank = ((r & 0x0040) >> 1) | (r & 0x001F);
                    let mode = i32::from(r & 0x03 != 0);
                    let bank = if slot >= 2 { bank | mode } else { bank & !mode };
                    (bank << 1) | (window & 0x01)
                }
            }
            Mmc3PKind::M327 => {
                let mask = if r & 0x08 != 0 { 0x1F } else { 0x0F };
                mix((r & 0x07) << 4, mask, page)
            }
            // `prg_swap_mmc3_334`: the chip's PRG is ignored, a 32 KiB bank from D1 up.
            Mmc3PKind::M334 => ((r & !1) << 1) | window,
            // `prg_swap_mmc3_339`: A5 clear is NROM, 32 KiB with A1-A2 set, else 16 KiB.
            Mmc3PKind::M339 => {
                let base = (r & 0x1F) << 1;
                if r & 0x20 != 0 {
                    mix(base, 0x0F, page)
                } else if r & 0x06 == 0x06 {
                    mix(base & 0xFC, 0x03, window)
                } else {
                    mix(base & 0xFE, 0x01, window)
                }
            }
            // `prg_swap_mmc3_345`: D2-D3 clear is the AxROM mode, a 32 KiB bank.
            Mmc3PKind::M345 => {
                let base = (r & 0xC0) >> 2;
                if r & 0x0C == 0 {
                    mix(((base >> 2) | (r & 0x03)) << 2, 0x03, window)
                } else {
                    mix(base, 0x0F, page)
                }
            }
            Mmc3PKind::M361 => mix(r & 0xF0, 0x0F, page),
            // `prg_swap_mmc3_372`: mapper 45's PRG offset and mask.
            Mmc3PKind::M372 => {
                let o = self.outer.map(i32::from);
                (o[1] | ((o[2] & 0xC0) << 2)) | (page & (!o[3] & 0x3F))
            }
            Mmc3PKind::M377 => mix((((r & 0x20) >> 2) | (r & 0x06)) << 3, 0x0F, page),
        }
    }

    /// `chr_swap_mmc3_NNN`: the 1 KiB bank `page` becomes.
    fn chr_page(&self, page: i32) -> i32 {
        let r = i32::from(self.outer[0]);
        let mix = |base: i32, mask: i32, value: i32| (base & !mask) | (value & mask);
        match self.kind {
            Mmc3PKind::M267 => mix(((r & 0x20) << 4) | ((r & 0x06) << 6), 0x7F, page),
            Mmc3PKind::M291 => ((r & 0x40) << 2) | (page & 0xFF),
            // `chr_swap_mmc3_315`: D3 and D1 ORed into CHR A16 and A17, D0 is A18.
            Mmc3PKind::M315 => {
                let value = ((r & 0x08) << 3) | ((r & 0x02) << 6) | page;
                mix((r & 0x01) << 8, 0xFF, value)
            }
            Mmc3PKind::M322 => {
                let small = i32::from(!(r & 0x0080 != 0 && r & 0x0020 != 0));
                let base = (((r & 0x0040) >> 4) | ((r & 0x0018) >> 3)) << (8 - small);
                mix(base, 0xFF >> small, page)
            }
            Mmc3PKind::M327 => {
                let mask = if r & 0x20 != 0 { 0xFF } else { 0x7F };
                ((r & 0x07) << 7) | (page & mask)
            }
            Mmc3PKind::M334 | Mmc3PKind::M345 => page & 0xFF,
            Mmc3PKind::M339 => mix((r & 0x18) << 4, 0x7F, page),
            Mmc3PKind::M361 => mix((r & 0xF0) << 3, 0x7F, page),
            Mmc3PKind::M372 => {
                let o = self.outer.map(i32::from);
                (o[0] | ((o[2] & 0xF0) << 4)) | (page & (0xFF >> (!o[2] & 0x0F)))
            }
            Mmc3PKind::M377 => mix((((r & 0x20) >> 2) | (r & 0x06)) << 6, 0x7F, page),
        }
    }

    /// The chip's own four PRG pages.
    fn mmc3_prg(&self) -> [i32; BankWindows::PRG_WINDOWS] {
        let r6 = i32::from(self.mmc3.bank_register(6));
        let r7 = i32::from(self.mmc3.bank_register(7));
        if self.mmc3.select() & 0x40 == 0 {
            [r6, r7, SECOND_LAST, LAST]
        } else {
            [SECOND_LAST, r7, r6, LAST]
        }
    }

    /// The chip's own eight CHR pages.
    fn mmc3_chr(&self) -> [i32; BankWindows::CHR_WINDOWS] {
        let reg = |index: usize| i32::from(self.mmc3.bank_register(index));
        let two = [reg(0) & 0xFE, reg(0) | 0x01, reg(1) & 0xFE, reg(1) | 0x01];
        let one = [reg(2), reg(3), reg(4), reg(5)];
        if self.mmc3.select() & 0x80 == 0 {
            [
                two[0], two[1], two[2], two[3], one[0], one[1], one[2], one[3],
            ]
        } else {
            [
                one[0], one[1], one[2], one[3], two[0], two[1], two[2], two[3],
            ]
        }
    }

    /// Whether a kind's CHR-RAM mode maps the RAM straight (327's D4, 372's `reg[2]` D5); on an
    /// image with CHR-ROM **and** CHR-RAM puNES maps the RAM over the ROM, which this build's
    /// single CHR memory cannot (FIDELITY-LEADS J37), so only a CHR-RAM image takes it.
    const fn chr_ram_mode(&self) -> bool {
        self.chr_ram
            && match self.kind {
                Mmc3PKind::M327 => self.outer[0] & 0x10 != 0,
                Mmc3PKind::M372 => self.outer[2] & 0x20 != 0,
                _ => false,
            }
    }

    /// Recomputes the twelve windows from the chip and the outer registers.
    fn refresh(&mut self) {
        let raw_prg = self.mmc3_prg();
        let prg: [i32; BankWindows::PRG_WINDOWS] =
            std::array::from_fn(|slot| self.prg_page(slot, raw_prg[slot]));
        let raw_chr = self.mmc3_chr();
        let chr: [i32; BankWindows::CHR_WINDOWS] = if self.chr_ram_mode() {
            std::array::from_fn(|slot| i32::try_from(slot).unwrap_or(0))
        } else {
            std::array::from_fn(|slot| self.chr_page(raw_chr[slot]))
        };
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        for (slot, bank) in prg.iter().enumerate() {
            banks.map_prg_8k(slot, *bank);
        }
        for (slot, bank) in chr.iter().enumerate() {
            banks.map_chr_1k(slot, *bank);
        }
    }

    /// The mirroring a kind imposes over the chip's (`mirroring_fix_mmc3_345`: D5 single-screen).
    fn settle_mirroring(&self, memory: &mut CartridgeMemory) {
        if self.kind == Mmc3PKind::M345 && self.outer[0] & 0x20 != 0 {
            memory.set_mirroring(if self.outer[0] & 0x10 != 0 {
                NametableMirroring::SingleScreenB
            } else {
                NametableMirroring::SingleScreenA
            });
        }
    }

    /// A write below `$8000` that reaches an outer register; `false` when it is not one.
    fn write_low(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        let writable = self.mmc3.prg_ram_writable();
        let data = u16::from(value);
        let r = &mut self.outer;
        match self.kind {
            Mmc3PKind::M267 if (0x6000..=0x6FFF).contains(&address) => {
                if r[0] & 0x80 == 0 {
                    r[0] = data;
                }
            }
            Mmc3PKind::M291 | Mmc3PKind::M377 if (0x6000..=0x6FFF).contains(&address) => {
                if writable {
                    r[0] = data;
                }
            }
            Mmc3PKind::M315 | Mmc3PKind::M345 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    r[0] = data;
                }
            }
            Mmc3PKind::M322 | Mmc3PKind::M339 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    r[0] = address;
                }
            }
            // `extcl_cpu_wr_mem_327`: until A0-A2 are non-zero.
            Mmc3PKind::M327 if (0x6000..=0x7FFF).contains(&address) => {
                if matches!(r[0] & 0x07, 0) && writable {
                    r[0] = address;
                }
            }
            // `extcl_cpu_wr_mem_334`: even addresses only.
            Mmc3PKind::M334 if (0x6000..=0x7FFF).contains(&address) => {
                if address & 0x0001 == 0 && writable {
                    r[0] = data;
                }
            }
            Mmc3PKind::M361 if (0x7000..=0x7FFF).contains(&address) => {
                if writable {
                    r[0] = data;
                }
            }
            // `extcl_cpu_wr_mem_372`: four registers in turn.
            Mmc3PKind::M372 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    r[usize::from(self.index)] = data;
                    self.index = (self.index + 1) & 0x03;
                }
            }
            _ => return false,
        }
        // `wram_wr` before the board: the RAM takes the byte too.
        if writable {
            let ram = memory.prg_ram_mut();
            if !ram.is_empty() {
                let len = ram.len();
                ram[usize::from(address & 0x1FFF) % len] = value;
            }
        }
        self.refresh();
        self.settle_mirroring(memory);
        true
    }
}

impl Mapper for Mmc3P {
    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        for value in self.outer {
            writer.write_u16(value);
        }
        writer.write_u8(self.index);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        for value in &mut self.outer {
            *value = reader.read_u16()?;
        }
        self.index = reader.read_u8()? & 0x03;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.power_on();
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable()
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        address < 0x8000 && self.write_low(memory, address, value)
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.mmc3.write_register(memory, address, value);
        self.refresh();
        self.settle_mirroring(memory);
    }

    /// `extcl_cpu_rd_mem_334`: `$6000-$7FFF` with A1 set reads `(open bus & $FE) | DIP` while the
    /// RAM is readable; the DIP is 0 and the high address byte stands in for the bus, as
    /// [`super::Mmc3Clone`] does for its DIP reads (FIDELITY-LEADS J33).
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if self.kind == Mmc3PKind::M334
            && (0x6000..=0x7FFF).contains(&address)
            && address & 0x0002 != 0
            && self.mmc3.prg_ram_enabled()
        {
            return Some(address.to_be_bytes()[0] & 0xFE);
        }
        self.mmc3.try_read_prg(memory, address)
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc3.ppu_write(memory, address, value);
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        self.mmc3.ppu_irq_horizon()
    }
}
