//! The MMC3 clones of lot MAPPERS-N: an [`Mmc3`] plus a few outer registers that mask, offset or
//! replace the bank numbers the chip produces, or a different decode of the chip's own ports.
//!
//! This is the shape Mesen2 gives every one of them — a subclass of its `MMC3` that overrides
//! `SelectPrgPage` / `SelectChrPage` (`Core/NES/Mappers/Mmc3Variants/MMC3_*.h`) — and the shape
//! [`super::Mmc3Outer`] already has here for mapper 268: the chip runs unchanged, and
//! [`Mmc3Clone::refresh`] recomputes the twelve windows from its registers after every write.
//!
//! **Power-on only.** Mesen2 clears most of these outer registers on a soft reset too; this build's
//! rule is that a board cannot see the RESET button ([`Mapper::soft_reset`]), so they come back to
//! their power-on values on a power cycle alone. `docs/review/FIDELITY-LEADS.md` records it.
//!
//! Each kind cites the Mesen2 file it is read from; the line numbers are those of the copy in
//! `cleanroom/third_party/mesen2` (Mesen2 2.x, GPL-3; read, not copied).

use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Which clone, one variant per iNES number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mmc3CloneKind {
    /// Mapper 44, the "Super Big 7-in-1" (Mesen2 `MMC3_44.h`): `$A001` also selects one of seven
    /// 128 KiB / 256 KiB blocks.
    M44,
    /// Mapper 45, the GA23C/TC3294 multicarts (Mesen2 `MMC3_45.h`): four outer registers written in
    /// turn at `$6000-$7FFF`, a mask and an offset for PRG and CHR, and a lock bit.
    M45,
    /// Mapper 49, the "Super HiK 4-in-1" (Mesen2 `MMC3_49.h`): a block register at `$6000-$7FFF`
    /// and a 32 KiB NROM-like mode.
    M49,
    /// Mapper 52, the Realtec 8213 multicarts (Mesen2 `MMC3_52.h`): one lockable outer register at
    /// `$6000-$7FFF`.
    M52,
    /// Mapper 134, the "Family Kid 4-in-1" T4A54A board (Mesen2 `MMC3_134.h`): one register at
    /// `$6001`.
    M134,
    /// Mapper 196, the MMC3 with permuted address lines of *Master Fighter II* and its kind
    /// (Mesen2 `MMC3_196.h`), plus a 32 KiB PRG override at `$6000-$6FFF`.
    M196,
    /// Mapper 197, the MMC3 whose CHR registers are 2 KiB/4 KiB (Mesen2 `MMC3_197.h`).
    M197,
    /// Mapper 205, the JC-016-2 multicarts (Mesen2 `MMC3_205.h`): a block register at
    /// `$6000-$7FFF`.
    M205,
    /// Mapper 238, the MMC3 with the `$4020-$7FFF` protection latch (Mesen2 `MMC3_238.h`).
    M238,
    /// Mapper 245, the Waixing MMC3 with CHR-RAM whose `R0` bit 1 is PRG A19 (Mesen2 `MMC3_245.h`).
    M245,
    /// Mapper 249, the MMC3 with an address-line scramble switched by `$5000` (Mesen2 `MMC3_249.h`).
    M249,
    /// Mapper 250, the Nitra MMC3 whose ports are decoded from the address lines (Mesen2
    /// `MMC3_250.h`): *Queen Bee V*.
    M250,
}

impl Mmc3CloneKind {
    /// The kind of an iNES number, or `None` when the number is not one of this file's.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            44 => Self::M44,
            45 => Self::M45,
            49 => Self::M49,
            52 => Self::M52,
            134 => Self::M134,
            196 => Self::M196,
            197 => Self::M197,
            205 => Self::M205,
            238 => Self::M238,
            245 => Self::M245,
            249 => Self::M249,
            250 => Self::M250,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M44 => 44,
            Self::M45 => 45,
            Self::M49 => 49,
            Self::M52 => 52,
            Self::M134 => 134,
            Self::M196 => 196,
            Self::M197 => 197,
            Self::M205 => 205,
            Self::M238 => 238,
            Self::M245 => 245,
            Self::M249 => 249,
            Self::M250 => 250,
        }
    }
}

/// Mesen2's fixed pages are `uint16_t` -2 and -1 (`MMC3.h:121-122`): the masks below act on
/// those sixteen bits, so the same two values are kept here.
const SECOND_LAST: i32 = 0xFFFE;
const LAST: i32 = 0xFFFF;

/// An MMC3 clone of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Clone {
    mmc3: Mmc3,
    kind: Mmc3CloneKind,
    /// The outer registers, their meaning per kind (see each kind's `refresh` arm).
    outer: [u8; 4],
    /// Mapper 45's write cursor (`MMC3_45.h:66-67`).
    index: u8,
    /// Whether the cartridge has CHR-RAM rather than CHR-ROM (mappers 45 and 245 read it).
    chr_ram: bool,
}

impl Mmc3Clone {
    /// The board, its registers settled by the `reset` the cartridge runs right after.
    pub(crate) fn new(memory: &CartridgeMemory, kind: Mmc3CloneKind) -> Self {
        Self {
            mmc3: Mmc3::new(memory, kind.number()),
            kind,
            outer: [0; 4],
            index: 0,
            chr_ram: memory.rom().chr_rom().is_empty(),
        }
    }

    fn power_on(&mut self, memory: &mut CartridgeMemory) {
        self.outer = [0; 4];
        self.index = 0;
        // `MMC3_45.h:43`: "_reg[2] = 0x0F", the CHR mask fully open.
        if self.kind == Mmc3CloneKind::M45 {
            self.outer[2] = 0x0F;
            // `MMC3_45.h:21-27`: R0-R5 power on as 0, 2, 4, 5, 6, 7, "needed by Famicom
            // Yarou Vol 1 - Game apparently writes to CHR RAM before initializing the
            // registers". Written through the chip's own ports, then `$8000` back to 0.
            for (register, value) in [0u8, 2, 4, 5, 6, 7].iter().enumerate() {
                self.mmc3
                    .write_register(memory, 0x8000, u8::try_from(register).unwrap_or(0));
                self.mmc3.write_register(memory, 0x8001, *value);
            }
            self.mmc3.write_register(memory, 0x8000, 0);
        }
        self.refresh();
    }

    /// The bank number `page` of PRG window `slot` becomes, Mesen2's `SelectPrgPage` override.
    fn prg_page(&self, slot: usize, page: i32) -> i32 {
        let r = &self.outer;
        match self.kind {
            // `MMC3_44.h:28-31`.
            Mmc3CloneKind::M44 => {
                let block = i32::from(r[0]);
                (page & if block <= 5 { 0x0F } else { 0x1F }) | (block * 0x10)
            }
            // `MMC3_45.h:57-61`.
            Mmc3CloneKind::M45 => (page & (0x3F ^ i32::from(r[3] & 0x3F))) | i32::from(r[1]),
            // `MMC3_49.h:36-44`: bit 0 of the register is the MMC3 mode, otherwise a 32 KiB bank.
            Mmc3CloneKind::M49 => {
                let value = i32::from(r[0]);
                if value & 0x01 != 0 {
                    (page & 0x0F) | (0x10 * ((value >> 6) & 0x03))
                } else {
                    ((value >> 4) & 0x03) * 4 + i32::try_from(slot).unwrap_or(0)
                }
            }
            // `MMC3_52.h:40-48`.
            Mmc3CloneKind::M52 => {
                let value = i32::from(r[0]);
                if value & 0x08 != 0 {
                    (page & 0x0F) | ((value & 0x07) << 4)
                } else {
                    (page & 0x1F) | ((value & 0x06) << 4)
                }
            }
            // `MMC3_134.h:17-20`.
            Mmc3CloneKind::M134 => (page & 0x1F) | ((i32::from(r[0]) & 0x02) << 4),
            // `MMC3_205.h:33-37`.
            Mmc3CloneKind::M205 => {
                let block = i32::from(r[0]);
                (page & if block <= 1 { 0x1F } else { 0x0F }) | (block * 0x10)
            }
            // `MMC3_249.h:28-38`, with the page as Mesen2's `uint16_t`.
            Mmc3CloneKind::M249 if r[0] & 0x02 != 0 => {
                let page = page & 0xFFFF;
                if page < 0x20 {
                    (page & 0x01)
                        | ((page >> 3) & 0x02)
                        | ((page >> 1) & 0x04)
                        | ((page << 2) & 0x18)
                } else {
                    scramble_249(page - 0x20)
                }
            }
            _ => page,
        }
    }

    /// The bank number `page` of 1 KiB CHR window `slot` becomes, Mesen2's `SelectChrPage`
    /// override.
    fn chr_page(&self, page: i32) -> i32 {
        let r = &self.outer;
        match self.kind {
            // `MMC3_44.h:21-24`.
            Mmc3CloneKind::M44 => {
                let block = i32::from(r[0]);
                (page & if block <= 5 { 0x7F } else { 0xFF }) | (block * 0x80)
            }
            // `MMC3_45.h:48-53`, CHR-ROM only.
            Mmc3CloneKind::M45 if !self.chr_ram => {
                let mask = 0xFF >> (0x0F - i32::from(r[2] & 0x0F));
                (page & mask) | i32::from(r[0]) | ((i32::from(r[2]) & 0xF0) << 4)
            }
            // `MMC3_49.h:29-32`.
            Mmc3CloneKind::M49 => (page & 0x7F) | (0x80 * ((i32::from(r[0]) >> 6) & 0x03)),
            // `MMC3_52.h:26-35`.
            Mmc3CloneKind::M52 => {
                let value = i32::from(r[0]);
                if value & 0x40 != 0 {
                    (page & 0x7F) | (((value & 0x04) | ((value >> 4) & 0x03)) << 7)
                } else {
                    (page & 0xFF) | (((value & 0x04) | ((value >> 4) & 0x02)) << 7)
                }
            }
            // `MMC3_134.h:12-15`.
            Mmc3CloneKind::M134 => (page & 0xFF) | ((i32::from(r[0]) & 0x20) << 3),
            // `MMC3_205.h:22-30`.
            Mmc3CloneKind::M205 => {
                let block = r[0];
                let mut page = page;
                if block >= 2 {
                    page = (page & 0x7F) | 0x100;
                }
                if block == 1 || block == 3 {
                    page |= 0x80;
                }
                page
            }
            // `MMC3_249.h:21-25`.
            Mmc3CloneKind::M249 if r[0] & 0x02 != 0 => scramble_249(page & 0xFFFF),
            _ => page,
        }
    }

    /// The chip's own four PRG pages, `MMC3.h:116-129`.
    fn mmc3_prg(&self) -> [i32; BankWindows::PRG_WINDOWS] {
        let r6 = i32::from(self.mmc3.bank_register(6));
        let r7 = i32::from(self.mmc3.bank_register(7));
        if self.mmc3.select() & 0x40 == 0 {
            [r6, r7, SECOND_LAST, LAST]
        } else {
            [SECOND_LAST, r7, r6, LAST]
        }
    }

    /// The chip's own eight CHR pages, `MMC3.h:91-114`.
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

    /// Recomputes the twelve windows from the chip and the outer registers.
    fn refresh(&mut self) {
        let raw_prg = self.mmc3_prg();
        let mut prg: [i32; BankWindows::PRG_WINDOWS] =
            std::array::from_fn(|slot| self.prg_page(slot, raw_prg[slot]));
        let raw_chr = self.mmc3_chr();
        let mut chr: [i32; BankWindows::CHR_WINDOWS] =
            std::array::from_fn(|slot| self.chr_page(raw_chr[slot]));

        match self.kind {
            // `MMC3_196.h:20-27`: once `$6000-$6FFF` is written, one 32 KiB bank, bypassing the
            // chip's PRG mapping (`SelectPrgPage4x(0, _exRegs[1] << 2)`).
            Mmc3CloneKind::M196 if self.outer[0] != 0 => {
                let base = i32::from(self.outer[1]) << 2;
                prg = [base, base + 1, base + 2, base + 3];
            }
            // `MMC3_197.h:9-21`: R0 drives 4 KiB and R2/R3 2 KiB each (mode 0), or R2 4 KiB and R0
            // both 2 KiB halves (mode 1).
            Mmc3CloneKind::M197 => {
                let reg = |index: usize| i32::from(self.mmc3.bank_register(index)) << 1;
                let (four, a, b) = if self.mmc3.select() & 0x80 == 0 {
                    (reg(0), reg(2), reg(3))
                } else {
                    (reg(2), reg(0), reg(0))
                };
                chr = [four, four + 1, four + 2, four + 3, a, a + 1, b, b + 1];
            }
            // `MMC3_245.h:9-46`: `R0` bit 1 is PRG A19 for both switchable banks and for the fixed
            // ones (the last two of that 512 KiB block, or of the image when it is smaller), and
            // with CHR-RAM the two 4 KiB halves follow the CHR mode.
            Mmc3CloneKind::M245 => {
                let or = if self.mmc3.bank_register(0) & 0x02 != 0 {
                    0x40
                } else {
                    0x00
                };
                let r6 = (i32::from(self.mmc3.bank_register(6)) & 0x3F) | or;
                let r7 = (i32::from(self.mmc3.bank_register(7)) & 0x3F) | or;
                let last = 0x3F | or;
                prg = if self.mmc3.select() & 0x40 == 0 {
                    [r6, r7, last - 1, last]
                } else {
                    [last - 1, r7, r6, last]
                };
                if self.chr_ram {
                    let (low, high) = if self.mmc3.select() & 0x80 != 0 {
                        (4, 0)
                    } else {
                        (0, 4)
                    };
                    chr = [
                        low,
                        low + 1,
                        low + 2,
                        low + 3,
                        high,
                        high + 1,
                        high + 2,
                        high + 3,
                    ];
                }
            }
            _ => {}
        }

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

    /// The chip port a write to `$8000-$FFFF` reaches, for the kinds that re-decode it.
    fn chip_port(&self, address: u16, value: u8) -> (u16, u8) {
        match self.kind {
            // `MMC3_250.h:9`: "(addr & 0xE000) | ((addr & 0x0400) >> 10), addr & 0xFF".
            Mmc3CloneKind::M250 => {
                let port = (address & 0xE000) | ((address & 0x0400) >> 10);
                (port, address.to_le_bytes()[0])
            }
            // `MMC3_196.h:30-38`: A0 of the chip comes from A1, A2 or A3.
            Mmc3CloneKind::M196 => {
                let port = if address >= 0xC000 {
                    (address & 0xFFFE) | ((address >> 2) & 0x01) | ((address >> 3) & 0x01)
                } else {
                    (address & 0xFFFE)
                        | ((address >> 2) & 0x01)
                        | ((address >> 3) & 0x01)
                        | ((address >> 1) & 0x01)
                };
                (port, value)
            }
            _ => (address, value),
        }
    }

    /// Whether the chip allows a write to its work RAM (Mesen2's `CanWriteToWorkRam`,
    /// `MMC3.h:131-134`), which gates the outer registers of mappers 49 and 52.
    fn work_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable()
    }

    /// A write below `$8000`: `Some(consumed)` when it reaches an outer register.
    #[inline(never)]
    fn write_outer(&mut self, address: u16, value: u8) -> Option<bool> {
        match self.kind {
            // `MMC3_45.h:64-75`: four registers in turn, until bit 6 of the fourth locks them and
            // the window is RAM again.
            Mmc3CloneKind::M45 if (0x6000..=0x7FFF).contains(&address) => {
                if self.outer[3] & 0x40 != 0 {
                    return Some(false);
                }
                self.outer[usize::from(self.index)] = value;
                self.index = (self.index + 1) & 0x03;
            }
            // `MMC3_49.h:47-55`.
            Mmc3CloneKind::M49 if (0x6000..=0x7FFF).contains(&address) => {
                if self.work_ram_writable() {
                    self.outer[0] = value;
                }
            }
            // `MMC3_52.h:52-62`: once bit 7 is set the register is locked and the write is RAM's.
            Mmc3CloneKind::M52 if (0x6000..=0x7FFF).contains(&address) => {
                if !self.work_ram_writable() {
                    return Some(true);
                }
                if self.outer[0] & 0x80 != 0 {
                    return Some(false);
                }
                self.outer[0] = value;
            }
            // `MMC3_134.h:23,39-42`: `$6001` only.
            Mmc3CloneKind::M134 if address == 0x6001 => self.outer[0] = value,
            // `MMC3_196.h:13,30-34`.
            Mmc3CloneKind::M196 if (0x6000..=0x6FFF).contains(&address) => {
                self.outer[0] = 1;
                self.outer[1] = (value & 0x0F) | (value >> 4);
            }
            // `MMC3_205.h:41-44`.
            Mmc3CloneKind::M205 if (0x6000..=0x7FFF).contains(&address) => {
                self.outer[0] = value & 0x03;
            }
            // `MMC3_238.h:9,21-24`: "_securityLut[value & 0x03]", { 0, 2, 2, 3 }.
            Mmc3CloneKind::M238 if (0x4020..=0x7FFF).contains(&address) => {
                self.outer[0] = [0x00, 0x02, 0x02, 0x03][usize::from(value & 0x03)];
            }
            // `MMC3_249.h:13,42-46`.
            Mmc3CloneKind::M249 if address == 0x5000 => self.outer[0] = value,
            _ => return None,
        }
        self.refresh();
        Some(true)
    }
}

/// `MMC3_249.h:22` and `:35`, the scramble of the eight bank lines.
const fn scramble_249(page: i32) -> i32 {
    (page & 0x03)
        | ((page >> 1) & 0x04)
        | ((page >> 4) & 0x08)
        | ((page >> 2) & 0x10)
        | ((page << 3) & 0x20)
        | ((page << 2) & 0xC0)
}

impl Mapper for Mmc3Clone {
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
        writer.write_raw(&self.outer);
        writer.write_u8(self.index);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        reader.read_into(&mut self.outer)?;
        self.index = reader.read_u8()? & 0x03;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.power_on(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted()
    }

    /// Mapper 238's latch answers the whole of `$4020-$7FFF` (`MMC3_238.h:15-16`), so there is no
    /// RAM window there.
    fn prg_ram_enabled(&self) -> bool {
        self.kind != Mmc3CloneKind::M238 && self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.kind != Mmc3CloneKind::M238 && self.mmc3.prg_ram_writable()
    }

    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= 0x8000 {
            return false;
        }
        self.write_outer(address, value).unwrap_or(false)
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        // `MMC3_44.h:36-42`: `$A001` also selects the block, "7" reading as 6.
        if self.kind == Mmc3CloneKind::M44 && address & 0xE001 == 0xA001 {
            self.outer[0] = (value & 0x07).min(6);
        }
        let (port, value) = self.chip_port(address, value);
        self.mmc3.write_register(memory, port, value);
        self.refresh();
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if self.kind == Mmc3CloneKind::M238 && (0x4020..=0x7FFF).contains(&address) {
            return Some(self.outer[0]);
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
