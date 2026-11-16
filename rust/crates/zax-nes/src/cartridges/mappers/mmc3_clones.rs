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
    /// Mapper 126, Mesen2 `MMC3_126.h` (TEC9719 / PJ-008 multicarts).
    M126,
    /// Mapper 224, Mesen2 `MMC3_224.h` (KT-008, outer bank at `$5000`).
    M224,
    /// Mapper 259, Mesen2 `MMC3_BmcF15.h` (BMC-F-15).
    M259,
    /// Mapper 287, Mesen2 `MMC3_Bmc411120C.h` (BMC-411120-C / K-3088).
    M287,
    /// Mapper 313, Mesen2 `ResetTxrom.h` (reset-based TxROM multicarts).
    M313,
    /// Mapper 325, Mesen2 `MMC3_MaliSB.h` (UNL-MALISB).
    M325,
    /// Mapper 217, Mesen2 `MMC3_217.h` (BMC-SPC009: a scrambled MMC3 with an outer bank).
    M217,
    /// Mapper 254, Mesen2 `MMC3_254.h` (*Pokémon* hacks: work RAM read through an XOR).
    M254,
    /// Mapper 258, Mesen2 `Unl158B.h` (UNL-158B, protection reads at `$5000`).
    M258,
    /// Mapper 333, Mesen2 `Bmc8in1.h` (BMC-8IN1).
    M333,
    /// Mapper 348, Mesen2 `Bmc830118C.h` (BMC-830118C).
    M348,
    /// Mapper 366, Mesen2 `BmcGn45.h` (BMC-GN-45).
    M366,
    /// Mapper 165, Mesen2 `MMC3_165.h`: MMC2-style CHR latches over two 4 KiB halves, page 0 being
    /// 4 KiB of CHR-RAM (*Fire Emblem* Chinese translations).
    M165,
    /// Mapper 198, nesdev "INES Mapper 198" / puNES `mapper_198.c`: 640 KiB of PRG, 8 KiB of
    /// battery WRAM at `$6000` and 4 KiB more, not backed, at `$5000-$5FFF`.
    M198,
    /// Mapper 199, nesdev "INES Mapper 199" (Waixing FS309) / puNES `mapper_199.c`: 8 KiB of
    /// unbanked CHR-RAM, 32 KiB of battery PRG-RAM of which `$6000-$7FFF` and `$5000-$5FFF`
    /// (its 4 KiB page 2) are mapped.
    M199,
    /// NES 2.0 mapper 513, Sachen SA-9602B: `Sachen/Sachen9602.h` and puNES `mapper_513.c`. The
    /// top two bits of a CHR register write (R0-R5) pick the 512 KiB PRG block of the two
    /// switchable windows; the fixed windows stay at pages `$3E`/`$3F` of the first block; 32 KiB
    /// of CHR-RAM.
    M513,
    /// NES 2.0 mapper 260, the HP10xx/HP20xx multicarts: nesdev "NES 2.0 Mapper 260" and puNES
    /// `mapper_260.c` (Mesen2's `BmcHpxx.h` is followed where they agree). `$5000` mode (bits
    /// 0-2: four MMC3 sizes, NROM-128, NROM-256, two CNROM sizes; bit 7 locks `$5xxx`), `$5001`
    /// the 16 KiB PRG base, `$5002` the 8 KiB CHR base, and a CNROM/mirroring latch at
    /// `$8000-$FFFF` (`outer[3]`).
    M260,
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
            126 => Self::M126,
            224 => Self::M224,
            259 => Self::M259,
            287 => Self::M287,
            313 => Self::M313,
            325 => Self::M325,
            217 => Self::M217,
            254 => Self::M254,
            258 => Self::M258,
            333 => Self::M333,
            348 => Self::M348,
            366 => Self::M366,
            165 => Self::M165,
            198 => Self::M198,
            199 => Self::M199,
            513 => Self::M513,
            260 => Self::M260,
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
            Self::M126 => 126,
            Self::M224 => 224,
            Self::M259 => 259,
            Self::M287 => 287,
            Self::M313 => 313,
            Self::M325 => 325,
            Self::M217 => 217,
            Self::M254 => 254,
            Self::M258 => 258,
            Self::M333 => 333,
            Self::M348 => 348,
            Self::M366 => 366,
            Self::M165 => 165,
            Self::M198 => 198,
            Self::M199 => 199,
            Self::M513 => 513,
            Self::M260 => 260,
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

    /// `MMC3_165.h:38-49` (`NotifyVramAddressChange`): a fetch of `$0FD8-$0FDF` or `$0FE8-$0FEF`
    /// (or the same in the right table) sets that half's latch, and the mapping follows at the
    /// **next** PPU address, as Mesen2's `_needUpdate` defers it. `outer[0]`/`outer[1]` are the
    /// latches, `outer[2]` the pending update.
    fn latch_165(&mut self, address: u16) {
        if self.outer[2] != 0 {
            self.outer[2] = 0;
            self.refresh();
        }
        if matches!(address & 0x2FF8, 0x0FD0 | 0x0FE8) {
            self.outer[usize::from((address >> 12) & 0x01)] = u8::from(address & 0x08 != 0);
            self.outer[2] = 1;
        }
    }

    /// Mapper 198's non-backed 4 KiB at `$5000-$5FFF`, the head of the PRG-RAM.
    pub const RAM_198_LOW: usize = 0x1000;

    /// The PRG-RAM offset of a mapper-198 or -199 `$5000-$5FFF` access, `None` otherwise: the
    /// head of 198's RAM, page 2 (offset 8 KiB) of 199's 32 KiB (puNES `wram_fix_mmc3_199`).
    fn ram_198(&self, memory: &CartridgeMemory, address: u16) -> Option<usize> {
        let base = match self.kind {
            Mmc3CloneKind::M198 => 0,
            Mmc3CloneKind::M199 => 0x2000,
            _ => return None,
        };
        ((0x5000..=0x5FFF).contains(&address) && memory.prg_ram().len() >= base + Self::RAM_198_LOW)
            .then(|| base + usize::from(address & 0x0FFF))
    }

    /// The CHR-RAM offset of a mapper-165 window on page 0, `None` otherwise.
    fn ram_165(&self, memory: &CartridgeMemory, address: u16) -> Option<usize> {
        let window = 1u8 << ((address >> 10) & 0x07);
        let len = memory.chr_ram().len();
        (self.kind == Mmc3CloneKind::M165 && self.outer[3] & window != 0 && len != 0)
            .then(|| usize::from(address & 0x0FFF) % len)
    }

    /// `MMC3_126.h:40-47`, `GetChrOuterBank`.
    fn outer_126(&self) -> i32 {
        let reg = i32::from(self.outer[0]);
        ((!reg) & 0x0080 & i32::from(self.outer[2]))
            | ((reg << 4) & 0x0080 & reg)
            | ((reg << 3) & 0x0100)
            | ((reg << 5) & 0x0200)
    }

    fn power_on(&mut self, memory: &mut CartridgeMemory) {
        self.outer = [0; 4];
        self.index = 0;
        // `MMC3_217.h:15-18`: "_exRegs[1] = 0xFF; _exRegs[2] = 0x03".
        if self.kind == Mmc3CloneKind::M217 {
            self.outer = [0, 0xFF, 0x03, 0];
        }
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
            // `Sachen9602.h`: `(page & 0x3F) | (_regs[1] << 6)`, then the fixed `$3E`/`$3F`
            // without the block (puNES `prg_fix_mmc3_513`, `~1 & 0x3F` and `~0 & 0x3F`).
            Mmc3CloneKind::M513 if page >= SECOND_LAST => page & 0x3F,
            // nesdev: "masking the MMC3 bank register content according to the specified size
            // … and OR'ing with the opposite-masked content" of `$5001` (16 KiB units).
            Mmc3CloneKind::M260 => {
                let mask = if r[0] & 0x02 != 0 { 0x0F } else { 0x1F };
                ((i32::from(r[1] & 0x3F) << 1) & !mask) | (page & mask)
            }
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
            // `MMC3_126.h:9-13`: the outer register masks and offsets every page; the 16/32 KiB
            // modes are in `refresh`.
            Mmc3CloneKind::M126 => {
                let reg = i32::from(r[0]);
                (page & (((!reg >> 2) & 0x10) | 0x0F))
                    | ((reg & (0x06 | ((reg & 0x40) >> 6))) << 4)
                    | ((reg & 0x10) << 3)
            }
            // `MMC3_224.h:20-32`: `$5000` bit 2 is PRG A19; 513's switchable windows, the same
            // formula with its block from a CHR register write.
            Mmc3CloneKind::M224 | Mmc3CloneKind::M513 => (page & 0x3F) | (i32::from(r[0]) << 6),
            // `MMC3_Bmc411120C.h:19-25`, DIP switch 0: `$xx` bits 0-1 are PRG A17-A18.
            Mmc3CloneKind::M287 => (page & 0x0F) | ((i32::from(r[0]) & 0x03) << 4),
            // `ResetTxrom.h:22-26`: the reset counter is PRG A17-A18.
            Mmc3CloneKind::M313 => (i32::from(r[0]) << 4) | (page & 0x0F),
            // `MMC3_MaliSB.h:13-16`: PRG A15 and A16 swapped.
            Mmc3CloneKind::M325 => (page & 0x03) | ((page & 0x08) >> 1) | ((page & 0x04) << 1),
            // `MMC3_217.h:31-38`.
            Mmc3CloneKind::M217 => {
                let ex1 = i32::from(r[1]);
                let page = if ex1 & 0x08 != 0 {
                    page & 0x1F
                } else {
                    (page & 0x0F) | (ex1 & 0x10)
                };
                ((ex1 << 5) & 0x60) | page
            }
            // `Unl158B.h:24-34`, MMC3 mode.
            Mmc3CloneKind::M258 => page & 0x0F,
            // `Bmc8in1.h:17-22` and `Bmc830118C.h:23-36`, the ordinary case.
            Mmc3CloneKind::M333 | Mmc3CloneKind::M348 => {
                ((i32::from(r[0]) & 0x0C) << 2) | (page & 0x0F)
            }
            // `BmcGn45.h:22-25`.
            Mmc3CloneKind::M366 => (page & 0x0F) | i32::from(r[0]),
            // puNES `mapper_198.c` (`prg_swap_mmc3_198`): the 640 KiB board's second chip holds
            // banks $40-$4F (nesdev, "INES Mapper 198"), so a number from $50 folds onto it; the
            // MMC3's fixed pages land on $4E/$4F, as the page has them.
            Mmc3CloneKind::M198 if page >= 0x50 => 0x40 | (page & 0x0F),
            _ => page,
        }
    }

    /// The bank number `page` of 1 KiB CHR window `slot` becomes, Mesen2's `SelectChrPage`
    /// override.
    fn chr_page(&self, page: i32) -> i32 {
        let r = &self.outer;
        match self.kind {
            Mmc3CloneKind::M260 => {
                let mask = if r[0] & 0x01 != 0 { 0x7F } else { 0xFF };
                ((i32::from(r[2] & 0x7F) << 3) & !mask) | (page & mask)
            }
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
            // `MMC3_126.h:33-37,40-47`.
            Mmc3CloneKind::M126 => {
                let mask = if r[0] & 0x80 != 0 { 0x7F } else { 0xFFFF };
                self.outer_126() | (page & mask)
            }
            // `MMC3_Bmc411120C.h:15-17`.
            Mmc3CloneKind::M287 => page | ((i32::from(r[0]) & 0x03) << 7),
            // `ResetTxrom.h:16-20`.
            Mmc3CloneKind::M313 => (i32::from(r[0]) << 7) | (page & 0x7F),
            // `MMC3_MaliSB.h:8-11`: CHR A11 and A15 swapped.
            Mmc3CloneKind::M325 => (page & 0xDD) | ((page & 0x20) >> 4) | ((page & 0x02) << 4),
            // `MMC3_217.h:24-29`.
            Mmc3CloneKind::M217 => {
                let ex1 = i32::from(r[1]);
                let page = if ex1 & 0x08 == 0 {
                    ((ex1 << 3) & 0x80) | (page & 0x7F)
                } else {
                    page
                };
                ((ex1 << 8) & 0x0300) | page
            }
            // `Bmc8in1.h:12-15`, `Bmc830118C.h:18-21`.
            Mmc3CloneKind::M333 | Mmc3CloneKind::M348 => {
                ((i32::from(r[0]) & 0x0C) << 5) | (page & 0x7F)
            }
            // `BmcGn45.h:17-20`.
            Mmc3CloneKind::M366 => (page & 0x7F) | (i32::from(r[0]) << 3),
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
    // One arm per kind that re-lays the windows out, each with its Mesen2 lines.
    #[allow(clippy::too_many_lines)]
    fn refresh(&mut self) {
        let raw_prg = self.mmc3_prg();
        let mut prg: [i32; BankWindows::PRG_WINDOWS] =
            std::array::from_fn(|slot| self.prg_page(slot, raw_prg[slot]));
        let raw_chr = self.mmc3_chr();
        let mut chr: [i32; BankWindows::CHR_WINDOWS] =
            std::array::from_fn(|slot| self.chr_page(raw_chr[slot]));

        match self.kind {
            // `MMC3_165.h:24-36` (`UpdateChrMapping`): each 4 KiB half takes R0 or R1 (left) and R2
            // or R4 (right) by its latch, in 4 KiB pages; page 0 is the CHR-RAM, which
            // `outer[3]` marks window by window.
            Mmc3CloneKind::M165 => {
                let left = self.mmc3.bank_register(usize::from(self.outer[0] != 0));
                let right = self
                    .mmc3
                    .bank_register(if self.outer[1] != 0 { 4 } else { 2 });
                let mut ram = 0u8;
                for (half, page) in [left, right].into_iter().enumerate() {
                    if page == 0 {
                        ram |= 0x0F << (half * 4);
                    }
                    let base = i32::from(page >> 2) * 4;
                    for (j, slot) in chr[half * 4..half * 4 + 4].iter_mut().enumerate() {
                        *slot = base + i32::try_from(j).unwrap_or(0);
                    }
                }
                self.outer[3] = ram;
            }
            // nesdev ("INES Mapper 199"): "The 8 KiB of CHR RAM are unbanked"; puNES
            // `chr_swap_mmc3_199` maps window n to page n.
            Mmc3CloneKind::M199 => chr = [0, 1, 2, 3, 4, 5, 6, 7],
            // `MMC3_196.h:20-27`: once `$6000-$6FFF` is written, one 32 KiB bank, bypassing the
            // chip's PRG mapping (`SelectPrgPage4x(0, _exRegs[1] << 2)`).
            // Modes 4-7: NROM-128, NROM-256 and the two CNROM sizes, the chip's banks unused.
            Mmc3CloneKind::M260 if self.outer[0] & 0x04 != 0 => {
                let r = self.outer.map(i32::from);
                prg = if r[0] & 0x07 == 4 {
                    let base = (r[1] & 0x3F) << 1;
                    [base, base + 1, base, base + 1]
                } else {
                    let base = (r[1] & 0x3E) << 1;
                    [base, base + 1, base + 2, base + 3]
                };
                let inner = match r[0] & 0x07 {
                    6 => 0x01,
                    7 => 0x03,
                    _ => 0x00,
                };
                let base = ((r[2] & !inner) | (r[3] & inner)) << 3;
                chr = std::array::from_fn(|slot| base + i32::try_from(slot).unwrap_or(0));
            }
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
            // `MMC3_126.h:14-27`: `$6003` bits 0-1 select an NROM-128/256 mode built from the
            // switchable window's page; bit 4 one 8 KiB CHR bank (`MMC3_126.h:57-61`).
            Mmc3CloneKind::M126 => {
                let mode = self.outer[3] & 0x03;
                if mode != 0 {
                    let base = prg[if self.mmc3.select() & 0x40 == 0 { 0 } else { 2 }];
                    prg = if mode == 0x03 {
                        [base, base + 1, base + 2, base + 3]
                    } else {
                        [base, base + 1, base, base + 1]
                    };
                }
                if self.outer[3] & 0x10 != 0 {
                    let base = self.outer_126() | ((i32::from(self.outer[2]) & 0x0F) << 3);
                    chr = std::array::from_fn(|slot| base + i32::try_from(slot).unwrap_or(0));
                }
            }
            // `MMC3_Bmc411120C.h:19-22`: `$xx` bit 3 (DIP 0) selects one 32 KiB bank.
            Mmc3CloneKind::M287 if self.outer[0] & 0x08 != 0 => {
                let base = ((i32::from(self.outer[0] >> 4) & 0x03) | 0x0C) << 2;
                prg = [base, base + 1, base + 2, base + 3];
            }
            // `MMC3_BmcF15.h:14-21`: two 16 KiB banks from the outer register alone.
            Mmc3CloneKind::M259 => {
                let bank = i32::from(self.outer[0] & 0x0F);
                let mode = (bank & 0x08) >> 3;
                let low = (bank & !mode) << 1;
                let high = ((bank & !mode) | mode) << 1;
                prg = [low, low + 1, high, high + 1];
            }
            // `MMC3_217.h:46-55`: `$5000` bit 7 selects one 32 KiB-style page pair, through the
            // same translation. Mesen2 loses it at the chip's next PRG update; here it holds until
            // `$5000` is written again (FIDELITY-LEADS J9).
            Mmc3CloneKind::M217 if self.outer[0] & 0x80 != 0 => {
                let value = i32::from(self.outer[0]);
                let base = ((value & 0x0F) | ((i32::from(self.outer[1]) << 4) & 0x30)) << 1;
                prg = [
                    self.prg_page(0, base),
                    self.prg_page(1, base + 1),
                    self.prg_page(2, base),
                    self.prg_page(3, base + 1),
                ];
            }
            // `Unl158B.h:24-34`: bit 7 of `$5000` selects 16 or 32 KiB NROM-style banking.
            Mmc3CloneKind::M258 if self.outer[0] & 0x80 != 0 => {
                let bank = i32::from(self.outer[0] & 0x07);
                prg = if self.outer[0] & 0x20 != 0 {
                    let base = (bank & 0x06) << 1;
                    [base, base + 1, base + 2, base + 3]
                } else {
                    let base = bank << 1;
                    [base, base + 1, base, base + 1]
                };
            }
            // `Bmc8in1.h:17-22`: without bit 4, one 32 KiB bank.
            Mmc3CloneKind::M333 if self.outer[0] & 0x10 == 0 => {
                let base = i32::from(self.outer[0] & 0x0F) << 2;
                prg = [base, base + 1, base + 2, base + 3];
            }
            // `Bmc830118C.h:23-32`: in block 3 the two switchable banks are mirrored at `$C000` and
            // `$E000` from page $32.
            Mmc3CloneKind::M348 if self.outer[0] & 0x0C == 0x0C => {
                prg[2] = 0x32 | (raw_prg[0] & 0x0F);
                prg[3] = 0x32 | (raw_prg[1] & 0x0F);
            }
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
            // `MMC3_MaliSB.h:19-25`.
            Mmc3CloneKind::M325 => {
                let port = if address >= 0xC000 {
                    (address & 0xFFFE) | ((address >> 2) & 0x01) | ((address >> 3) & 0x01)
                } else {
                    (address & 0xFFFE) | ((address >> 3) & 0x01)
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
            // `MMC3_126.h:62-81`: four registers under mask 3; 0 and 3 lock when bit 7 of 3 is set.
            Mmc3CloneKind::M126 if (0x6000..=0x7FFF).contains(&address) => {
                let index = usize::from(address & 0x03);
                if index == 1 || index == 2 || self.outer[3] & 0x80 == 0 {
                    self.outer[index] = value;
                }
            }
            // nesdev: "Lock, do not respond to further writes in the $5xxx range"; `$5003` has
            // no description and is not kept.
            Mmc3CloneKind::M260 if (0x5000..=0x5FFF).contains(&address) => {
                if self.outer[0] & 0x80 == 0 && address & 0x03 != 0x03 {
                    self.outer[usize::from(address & 0x03)] = value;
                }
            }
            // `MMC3_224.h:35-41`: `$5000` bit 2.
            Mmc3CloneKind::M224 if address == 0x5000 => self.outer[0] = (value >> 2) & 0x01,
            // `MMC3_Bmc411120C.h:27-33`: the **address** is the register, over `$6000-$7FFF`.
            Mmc3CloneKind::M287 if (0x6000..=0x7FFF).contains(&address) => {
                self.outer[0] = address.to_le_bytes()[0];
            }
            // `MMC3_BmcF15.h:24-29`: only while `$A001` enables the work RAM.
            Mmc3CloneKind::M259 if (0x6000..=0x7FFF).contains(&address) => {
                if self.mmc3.prg_ram_enabled() {
                    self.outer[0] = value & 0x0F;
                }
            }
            // `MMC3_217.h:57-75`.
            Mmc3CloneKind::M217 => match address {
                0x5000 => self.outer[0] = value,
                0x5001 => self.outer[1] = value,
                0x5007 => self.outer[2] = value,
                _ => return None,
            },
            // `Unl158B.h:40-46`: `$5000-$5FFF` with A0-A2 clear.
            Mmc3CloneKind::M258 if (0x5000..=0x5FFF).contains(&address) => {
                if address.trailing_zeros() >= 3 {
                    self.outer[0] = value;
                }
            }
            // `Bmc830118C.h:12,38-42`.
            Mmc3CloneKind::M348 if (0x6800..=0x68FF).contains(&address) => self.outer[0] = value,
            // `BmcGn45.h:27-45`: until bit 7 of the address enables the work RAM, `$6000-$6FFF`
            // latches the block from the address and `$7000-$7FFF` from the data.
            Mmc3CloneKind::M366 if (0x6000..=0x7FFF).contains(&address) => {
                if self.outer[1] != 0 {
                    return Some(false);
                }
                if address < 0x7000 {
                    self.outer[0] = (address & 0x30).to_le_bytes()[0];
                    self.outer[1] = u8::from(address & 0x80 != 0);
                } else {
                    self.outer[0] = value & 0x30;
                }
            }
            _ => return None,
        }
        self.refresh();
        Some(true)
    }
}

impl Mmc3Clone {
    /// `MMC3_217.h:77-110`: with `$5007` non-zero the chip's ports are scrambled — `$8000` is
    /// `$C000`, `$8001` writes a permuted index to `$8000`, `$A000` carries the data and `$A001`
    /// the mirroring.
    fn write_217(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        const LUT: [u8; 8] = [0, 6, 3, 7, 5, 2, 4, 1];
        let mirroring = |value: u8| {
            if value & 0x01 != 0 {
                crate::NametableMirroring::Horizontal
            } else {
                crate::NametableMirroring::Vertical
            }
        };
        match address & 0xE001 {
            0x8000 => self.mmc3.write_register(memory, 0xC000, value),
            0x8001 => {
                self.outer[3] = 1;
                let value = (value & 0xC0) | LUT[usize::from(value & 0x07)];
                self.mmc3.write_register(memory, 0x8000, value);
            }
            0xA000 => {
                if self.outer[3] != 0
                    && (self.outer[0] & 0x80 == 0 || self.mmc3.select() & 0x07 < 6)
                {
                    self.outer[3] = 0;
                    self.mmc3.write_register(memory, 0x8001, value);
                }
            }
            0xA001 => memory.set_mirroring(mirroring(value)),
            _ => self.mmc3.write_register(memory, address, value),
        }
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

    /// Mapper 313 is a reset-based multicart: "When an interruption of the M2 signal is detected,
    /// the higher address lines are automatically incremented" (nesdev, "NES 2.0 Mapper 313");
    /// Mesen2 `ResetTxrom.h:9-14`. No other kind reacts to RESET.
    fn soft_reset(&mut self, _memory: &mut CartridgeMemory) {
        if self.kind == Mmc3CloneKind::M313 {
            self.outer[0] = (self.outer[0] + 1) & 0x03;
            self.refresh();
        }
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
        !matches!(self.kind, Mmc3CloneKind::M238 | Mmc3CloneKind::M254)
            && self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.kind != Mmc3CloneKind::M238 && self.mmc3.prg_ram_writable()
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= 0x8000 {
            return false;
        }
        if let Some(offset) = self.ram_198(memory, address) {
            memory.prg_ram_mut()[offset] = value;
            return true;
        }
        self.write_outer(address, value).unwrap_or(false)
    }

    /// Mapper 198's `$6000-$7FFF` is the tail of the PRG-RAM, after the 4 KiB of `$5000`.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let len = memory.prg_ram().len();
        if self.kind == Mmc3CloneKind::M198 && len > Self::RAM_198_LOW {
            (Self::RAM_198_LOW, len)
        } else {
            (0, len)
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        // `MMC3_44.h:36-42`: `$A001` also selects the block, "7" reading as 6.
        if self.kind == Mmc3CloneKind::M44 && address & 0xE001 == 0xA001 {
            self.outer[0] = (value & 0x07).min(6);
        }
        // nesdev: the `$8000-$FFFF` latch, CHR in modes 6-7 and the mirroring in modes 4-7, "0
        // gives … vertical mirroring, and 1 gives … horizontal mirroring"; the MMC3 registers
        // keep answering.
        if self.kind == Mmc3CloneKind::M260 {
            self.outer[3] = value;
            let (port, chip_value) = self.chip_port(address, value);
            self.mmc3.write_register(memory, port, chip_value);
            if self.outer[0] & 0x04 != 0 {
                memory.set_mirroring(if value & 0x04 != 0 {
                    crate::NametableMirroring::Horizontal
                } else {
                    crate::NametableMirroring::Vertical
                });
            }
            self.refresh();
            return;
        }
        // `Sachen9602.h` `WriteRegister`: a write to R0-R5 also sets the block, and the chip
        // takes the value's low five bits.
        let value = if self.kind == Mmc3CloneKind::M513
            && address & 0xE001 == 0x8001
            && self.mmc3.select() & 0x07 < 6
        {
            self.outer[0] = value >> 6;
            value & 0x1F
        } else {
            value
        };
        match self.kind {
            Mmc3CloneKind::M217 if self.outer[2] != 0 => {
                self.write_217(memory, address, value);
                self.refresh();
                return;
            }
            // `Bmc8in1.h:25-31`: any write with A12 set is the outer register, not the chip's.
            Mmc3CloneKind::M333 if address & 0x1000 != 0 => {
                self.outer[0] = value;
                self.refresh();
                return;
            }
            // `MMC3_254.h:24-28`.
            Mmc3CloneKind::M254 if address == 0x8000 => self.outer[0] = 0xFF,
            Mmc3CloneKind::M254 if address == 0xA001 => self.outer[1] = value,
            _ => {}
        }
        let (port, value) = self.chip_port(address, value);
        self.mmc3.write_register(memory, port, value);
        self.refresh();
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if let Some(offset) = self.ram_198(memory, address) {
            return Some(memory.prg_ram()[offset]);
        }
        // nesdev: `$5000` read, bits 1-0 the DIP switches (none set); the rest the bus, for
        // which the high address byte stands (FIDELITY-LEADS J33).
        if self.kind == Mmc3CloneKind::M260 && (0x5000..=0x5FFF).contains(&address) {
            return Some(address.to_be_bytes()[0] & !0x03);
        }
        if self.kind == Mmc3CloneKind::M238 && (0x4020..=0x7FFF).contains(&address) {
            return Some(self.outer[0]);
        }
        // `Unl158B.h:36-38`: the open bus ORed with a protection table; the high address byte
        // stands in for the bus (FIDELITY-LEADS J2).
        if self.kind == Mmc3CloneKind::M258 && (0x5000..=0x5FFF).contains(&address) {
            const LUT: [u8; 8] = [0x00, 0x00, 0x00, 0x01, 0x02, 0x04, 0x0F, 0x00];
            return Some(address.to_be_bytes()[0] | LUT[usize::from(address & 0x07)]);
        }
        // `MMC3_254.h:15-22`: the work RAM, XORed with `$A001`'s last value until `$8000` has
        // been written.
        if self.kind == Mmc3CloneKind::M254 && (0x6000..=0x7FFF).contains(&address) {
            let ram = memory.prg_ram();
            if ram.is_empty() || !self.mmc3.prg_ram_enabled() {
                return None;
            }
            let value = ram[usize::from(address - 0x6000) % ram.len()];
            return Some(if self.outer[0] != 0 {
                value
            } else {
                value ^ self.outer[1]
            });
        }
        self.mmc3.try_read_prg(memory, address)
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.kind == Mmc3CloneKind::M165 && self.outer[3] != 0 {
            return None;
        }
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        if let Some(offset) = self.ram_165(memory, address) {
            return memory.chr_ram()[offset];
        }
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if let Some(offset) = self.ram_165(memory, address) {
            memory.chr_ram_mut()[offset] = value;
            return;
        }
        self.mmc3.ppu_write(memory, address, value);
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
        if self.kind == Mmc3CloneKind::M165 {
            self.latch_165(address);
        }
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        self.mmc3.ppu_irq_horizon()
    }
}
