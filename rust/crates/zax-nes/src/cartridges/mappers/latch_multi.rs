//! The discrete-logic multicarts of lot MAPPERS-N: one or two latched registers — most of them
//! the **address** of the write rather than its data — that choose a PRG bank, a CHR bank and the
//! mirroring. No IRQ, no RAM banking.
//!
//! Each kind is read from the Mesen2 file it names (`cleanroom/third_party/mesen2/Core/NES/
//! Mappers/…`, read, not copied); the line references are to those files. Mesen2's page units are
//! kept in the comments: "16K" means its `GetPrgPageSize() == 0x4000`.
//!
//! **Power-on only**, as everywhere in this build ([`Mapper::soft_reset`]): Mesen2 clears a few of
//! these on a soft reset too (226). The two exceptions are the documented reset-detecting boards,
//! 230 and 233, whose RESET reaction *is* their function — the menu-or-game switch of a "22-in-1".

use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

const H: NametableMirroring = NametableMirroring::Horizontal;
const V: NametableMirroring = NametableMirroring::Vertical;

/// The low byte of a latched address field (already masked to fit).
const fn low_byte(value: u16) -> u8 {
    value.to_le_bytes()[0]
}

/// `cond ? first : second`, as the Mesen2 sources write it.
const fn pick(
    cond: bool,
    first: NametableMirroring,
    second: NametableMirroring,
) -> NametableMirroring {
    if cond {
        first
    } else {
        second
    }
}

/// Which board, one variant per iNES number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchKind {
    /// 57, `Unlicensed/Mapper57.h`.
    M57,
    /// 58, `Unlicensed/Mapper58.h`.
    M58,
    /// 59, `Unlicensed/UnlD1038.h` (BMC-T3H53 / D1038).
    M59,
    /// 63, `Ntdec/Bmc63.h` (NTDEC 2291 "Powerful 250-in-1").
    M63,
    /// 200, `Unlicensed/Mapper200.h`.
    M200,
    /// 201, `Unlicensed/NovelDiamond.h`.
    M201,
    /// 202, `Unlicensed/Mapper202.h`.
    M202,
    /// 203, `Unlicensed/Mapper203.h`.
    M203,
    /// 204, `Unlicensed/Mapper204.h`.
    M204,
    /// 212, `Unlicensed/Mapper212.h`.
    M212,
    /// 213, `Unlicensed/Mapper213.h`.
    M213,
    /// 214, `Unlicensed/Mapper214.h`.
    M214,
    /// 221, `Ntdec/Mapper221.h` (NTDEC N625092).
    M221,
    /// 226, `Unlicensed/Mapper226.h`.
    M226,
    /// 227, `Unlicensed/Mapper227.h`.
    M227,
    /// 229, `Unlicensed/Mapper229.h`.
    M229,
    /// 230, `Unlicensed/Mapper230.h` (22-in-1 with *Contra*, reset-switched).
    M230,
    /// 231, `Unlicensed/Mapper231.h`.
    M231,
    /// 233, `Unlicensed/Mapper233.h` (226 with a reset-toggled PRG A20).
    M233,
    /// 235, `Unlicensed/Bmc235.h`.
    M235,
    /// 236, `Unlicensed/Bmc70in1.h`.
    M236,
    /// 240, `Unlicensed/Mapper240.h`.
    M240,
    /// 241, `Unlicensed/Mapper241.h`.
    M241,
    /// 242, `Waixing/Mapper242.h`.
    M242,
    /// 244, `Unlicensed/Mapper244.h`.
    M244,
    /// 255, `Unlicensed/Bmc255.h`.
    M255,
    /// 46, `Unlicensed/ColorDreams46.h` (Rumble Station).
    M46,
    /// 51, `Unlicensed/Bmc51.h` (11-in-1 Ball Games).
    M51,
    /// 60, `Unlicensed/Mapper60.h` (reset-based 4-in-1).
    M60,
    /// 61, `Txc/Mapper61.h`.
    M61,
    /// 62, `Unlicensed/Mapper62.h`.
    M62,
    /// 104, `Codemasters/GoldenFive.h` (Golden Five / Pegasus 5-in-1).
    M104,
    /// 108, `Unlicensed/Bb.h` (FDS conversions).
    M108,
    /// 113, `Unlicensed/Nina03_06.h`, multicart mode.
    M113,
    /// 120, `Unlicensed/Mapper120.h` (Tobidase Daisakusen FDS conversion).
    M120,
    /// 125, `Whirlwind/Lh32.h` (Monty no Doki Doki Daisasso FDS conversion).
    M125,
    /// 170, `Unlicensed/Mapper170.h` (Fujiya protection).
    M170,
    /// 174, `Ntdec/Mapper174.h`.
    M174,
    /// 177, `Unlicensed/Henggedianzi177.h`.
    M177,
    /// 179, `Unlicensed/Henggedianzi179.h`.
    M179,
    /// 216, `Unlicensed/Mapper216.h` (Magic Jewelry II, Bonza).
    M216,
    /// 261, `Unlicensed/Bmc810544CA1.h`.
    M261,
    /// 274, `Unlicensed/Bmc80013B.h`.
    M274,
    /// 286, `Waixing/Bs5.h` (BS-5).
    M286,
    /// 39, `Unlicensed/Mapper39.h` (Study and Game 32-in-1).
    M39,
    /// 54, `Unlicensed/NovelDiamond.h`, the same board as 201.
    M54,
    /// 148, `Sachen/Sachen_148.h` (Sachen SA-008-A, Tengen 800008).
    M148,
    /// 171, `Kaiser/Kaiser7058.h`.
    M171,
    /// 312, `Kaiser/Kaiser7013B.h`.
    M312,
    /// 306, `Kaiser/Kaiser7016.h` (KS7016).
    M306,
    /// 265, `Unlicensed/T262.h` (BMC-T-262).
    M265,
    /// 283, `Unlicensed/Gs2004.h` (GS-2004 / GS-2013).
    M283,
    /// 285, `Unlicensed/A65AS.h`.
    M285,
    /// 288, `Unlicensed/Gkcx1.h` (GKCX1).
    M288,
    /// 290, `Ntdec/BmcNtd03.h` (NTD-03).
    M290,
    /// 299, `Txc/Bmc11160.h` (BMC-11160).
    M299,
    /// 300, `Unlicensed/Bmc190in1.h`.
    M300,
    /// 336, `Unlicensed/BmcK3046.h`.
    M336,
    /// 349, `Unlicensed/BmcG146.h`.
    M349,
    /// 289, `Unlicensed/Bmc60311C.h` (BMC-60311C).
    M289,
    /// 301, `Unlicensed/Bmc8157.h` (BMC-8157).
    M301,
    /// 314, `Unlicensed/Bmc64in1NoRepeat.h`.
    M314,
    /// 319, `Unlicensed/Hp898f.h` (HP-898F).
    M319,
    /// 309, `Whirlwind/Lh51.h` (LH51, FDS conversion).
    M309,
    /// 320, `Unlicensed/Bmc830425C4391T.h`.
    M320,
    /// 329, `Unlicensed/Edu2000.h` (EDU2000, 32 KiB banked work RAM).
    M329,
    /// 331, `Unlicensed/Bmc12in1.h`.
    M331,
    /// 332, `Unlicensed/Super40in1Ws.h`.
    M332,
    /// 346, `Kaiser/Kaiser7012.h`.
    M346,
    /// 521, `Unlicensed/DreamTech01.h`.
    M521,
    /// 166, `Unlicensed/Subor166.h` (Subor).
    M166,
    /// 167, `Unlicensed/Subor166.h`, the alternate wiring.
    M167,
    /// 324, `Homebrew/FaridUnrom.h` (FARID_UNROM_8-IN-1).
    M324,
    /// 522, `Whirlwind/Lh10.h` (LH10, FDS conversion).
    M522,
}

impl LatchKind {
    /// The kind of an iNES number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            57 => Self::M57,
            58 => Self::M58,
            59 => Self::M59,
            63 => Self::M63,
            200 => Self::M200,
            201 => Self::M201,
            202 => Self::M202,
            203 => Self::M203,
            204 => Self::M204,
            212 => Self::M212,
            213 => Self::M213,
            214 => Self::M214,
            221 => Self::M221,
            226 => Self::M226,
            227 => Self::M227,
            229 => Self::M229,
            230 => Self::M230,
            231 => Self::M231,
            233 => Self::M233,
            235 => Self::M235,
            236 => Self::M236,
            240 => Self::M240,
            241 => Self::M241,
            242 => Self::M242,
            244 => Self::M244,
            255 => Self::M255,
            46 => Self::M46,
            51 => Self::M51,
            60 => Self::M60,
            61 => Self::M61,
            62 => Self::M62,
            104 => Self::M104,
            108 => Self::M108,
            113 => Self::M113,
            120 => Self::M120,
            125 => Self::M125,
            170 => Self::M170,
            174 => Self::M174,
            177 => Self::M177,
            179 => Self::M179,
            216 => Self::M216,
            261 => Self::M261,
            274 => Self::M274,
            286 => Self::M286,
            39 => Self::M39,
            54 => Self::M54,
            148 => Self::M148,
            171 => Self::M171,
            312 => Self::M312,
            306 => Self::M306,
            265 => Self::M265,
            283 => Self::M283,
            285 => Self::M285,
            288 => Self::M288,
            290 => Self::M290,
            299 => Self::M299,
            300 => Self::M300,
            336 => Self::M336,
            349 => Self::M349,
            289 => Self::M289,
            301 => Self::M301,
            314 => Self::M314,
            319 => Self::M319,
            309 => Self::M309,
            320 => Self::M320,
            329 => Self::M329,
            331 => Self::M331,
            332 => Self::M332,
            346 => Self::M346,
            521 => Self::M521,
            166 => Self::M166,
            167 => Self::M167,
            324 => Self::M324,
            522 => Self::M522,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M57 => 57,
            Self::M58 => 58,
            Self::M59 => 59,
            Self::M63 => 63,
            Self::M200 => 200,
            Self::M201 => 201,
            Self::M202 => 202,
            Self::M203 => 203,
            Self::M204 => 204,
            Self::M212 => 212,
            Self::M213 => 213,
            Self::M214 => 214,
            Self::M221 => 221,
            Self::M226 => 226,
            Self::M227 => 227,
            Self::M229 => 229,
            Self::M230 => 230,
            Self::M231 => 231,
            Self::M233 => 233,
            Self::M235 => 235,
            Self::M236 => 236,
            Self::M240 => 240,
            Self::M241 => 241,
            Self::M242 => 242,
            Self::M244 => 244,
            Self::M255 => 255,
            Self::M46 => 46,
            Self::M51 => 51,
            Self::M60 => 60,
            Self::M61 => 61,
            Self::M62 => 62,
            Self::M104 => 104,
            Self::M108 => 108,
            Self::M113 => 113,
            Self::M120 => 120,
            Self::M125 => 125,
            Self::M170 => 170,
            Self::M174 => 174,
            Self::M177 => 177,
            Self::M179 => 179,
            Self::M216 => 216,
            Self::M261 => 261,
            Self::M274 => 274,
            Self::M286 => 286,
            Self::M39 => 39,
            Self::M54 => 54,
            Self::M148 => 148,
            Self::M171 => 171,
            Self::M312 => 312,
            Self::M306 => 306,
            Self::M265 => 265,
            Self::M283 => 283,
            Self::M285 => 285,
            Self::M288 => 288,
            Self::M290 => 290,
            Self::M299 => 299,
            Self::M300 => 300,
            Self::M336 => 336,
            Self::M349 => 349,
            Self::M289 => 289,
            Self::M301 => 301,
            Self::M314 => 314,
            Self::M319 => 319,
            Self::M309 => 309,
            Self::M320 => 320,
            Self::M329 => 329,
            Self::M331 => 331,
            Self::M332 => 332,
            Self::M346 => 346,
            Self::M521 => 521,
            Self::M166 => 166,
            Self::M167 => 167,
            Self::M324 => 324,
            Self::M522 => 522,
        }
    }

    /// Whether Mesen2's `InitMapper`/`Reset` reaches `SetMirroringType` at power-on (through a
    /// `WriteRegister(0x8000, 0)` or an `UpdateState`); the others keep the header's mirroring
    /// until the first write.
    const fn mirrors_at_power_on(self) -> bool {
        matches!(
            self,
            Self::M57
                | Self::M59
                | Self::M63
                | Self::M204
                | Self::M212
                | Self::M221
                | Self::M227
                | Self::M229
                | Self::M230
                | Self::M242
                | Self::M255
                | Self::M174
                | Self::M261
                | Self::M274
                | Self::M51
                | Self::M312
                | Self::M331
                | Self::M332
                | Self::M289
                | Self::M301
                | Self::M314
                | Self::M319
                | Self::M285
                | Self::M290
                | Self::M299
                | Self::M300
                | Self::M349
        )
    }
}

/// A discrete multicart of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatchMulti {
    banks: BankWindows,
    kind: LatchKind,
    /// The latched write address (most kinds), or 221's mode register.
    addr: u16,
    /// The latched data registers, their meaning per kind.
    regs: [u8; 8],
    /// 230's *Contra* mode, 233's reset-toggled PRG bit, 59's DIP-switch read, 236's DIP read
    /// mode: one per-kind flag.
    flag: bool,
    /// The 8 KiB PRG windows that read open bus (235, 63), one bit each. Derived.
    open_bus: u8,
    /// The 8 KiB PRG-ROM bank `$6000-$7FFF` shows (51, 108, 120, 125), or `None`. Derived.
    rom_6000: Option<i32>,
    /// Mapper 235's mode, from the size of its PRG-ROM (`Bmc235.h`, the `switch` on
    /// `GetPrgPageCount()`).
    size_mode: u8,
    /// Whether mapper 236 has CHR-RAM, so that its register drives the outer PRG bank instead of
    /// CHR (`Bmc70in1.h`, `_useOuterBank`).
    chr_ram: bool,
    /// Mapper 108's submapper, 1 to 4 (nesdev "INES Mapper 108"); 0 on every other kind.
    submapper: u8,
}

impl LatchMulti {
    pub(crate) fn new(memory: &CartridgeMemory, kind: LatchKind) -> Self {
        let pages_16k = memory.rom().prg_rom().len() / 0x4000;
        Self {
            banks: windows_for(memory),
            kind,
            addr: 0,
            regs: [0; 8],
            flag: false,
            open_bus: 0,
            rom_6000: None,
            size_mode: match pages_16k {
                64 => 0,
                128 => 1,
                256 => 2,
                _ => 3,
            },
            chr_ram: memory.rom().chr_rom().is_empty(),
            submapper: if kind == LatchKind::M108 {
                Self::submapper_108(memory)
            } else {
                0
            },
        }
    }

    /// nesdev ("INES Mapper 108") splits the FDS conversions into four submappers by where the
    /// register sits; puNES `mapper_108.c:37-40` guesses a missing one the same way the page's
    /// table reads: no CHR-ROM is 1 (Bubble Bobble) unless the mirroring is vertical (3,
    /// Meikyuu Jiin Dababa), and CHR-ROM is 4 (Pro Wrestling, 32 KiB) or 2 (Falsion).
    fn submapper_108(memory: &CartridgeMemory) -> u8 {
        let header = memory.rom().header();
        if (1..=4).contains(&header.submapper) {
            return u8::try_from(header.submapper).unwrap_or(1);
        }
        if memory.rom().chr_rom().is_empty() {
            if header.mirroring == NametableMirroring::Vertical {
                3
            } else {
                1
            }
        } else if memory.rom().prg_rom().len() <= 0x8000 {
            4
        } else {
            2
        }
    }

    fn prg_16k(&mut self, low: i32, high: i32) {
        self.banks.map_prg_16k(0, low);
        self.banks.map_prg_16k(2, high);
    }

    /// Computes the windows from the latched state and answers the mirroring, or `None` for a
    /// kind that has no mirroring control.
    #[allow(clippy::too_many_lines)]
    fn map(&mut self) -> Option<NametableMirroring> {
        let a = i32::from(self.addr);
        let r0 = i32::from(self.regs[0]);
        let r1 = i32::from(self.regs[1]);
        self.open_bus = 0;
        self.rom_6000 = None;
        let regs = self.regs;
        match self.kind {
            // `Mapper57.h:20-33`.
            LatchKind::M57 => {
                self.banks
                    .map_chr_8k(((r0 & 0x40) >> 3) | ((r0 | r1) & 0x07));
                if r1 & 0x10 != 0 {
                    let page = (r1 >> 5) & 0x06;
                    self.prg_16k(page, page + 1);
                } else {
                    let page = (r1 >> 5) & 0x07;
                    self.prg_16k(page, page);
                }
                Some(pick(r1 & 0x08 != 0, H, V))
            }
            // `Mapper58.h:19-29`.
            LatchKind::M58 => {
                let bank = a & 0x07;
                if a & 0x40 != 0 {
                    self.prg_16k(bank, bank);
                } else {
                    self.prg_16k(bank & 0x06, (bank & 0x06) + 1);
                }
                self.banks.map_chr_8k((a >> 3) & 0x07);
                Some(pick(a & 0x80 != 0, H, V))
            }
            // `UnlD1038.h:31-41`.
            LatchKind::M59 => {
                if a & 0x80 != 0 {
                    let page = (a & 0x70) >> 4;
                    self.prg_16k(page, page);
                } else {
                    let page = (a & 0x60) >> 4;
                    self.prg_16k(page, page + 1);
                }
                self.banks.map_chr_8k(a & 0x07);
                self.flag = a & 0x100 != 0;
                Some(pick(a & 0x08 != 0, H, V))
            }
            // `Bmc63.h:27-37`, 8 KiB pages; `$8000-$BFFF` is open bus when A9-A8 are both set.
            LatchKind::M63 => {
                let base = (a >> 1) & 0x1FC;
                let sel = |even: i32| {
                    if a & 0x02 != 0 {
                        even
                    } else {
                        ((a >> 1) & 0x02) | (even & 1)
                    }
                };
                if a & 0x0300 == 0x0300 {
                    self.open_bus = 0b0011;
                } else {
                    self.banks.map_prg_8k(0, base | sel(0));
                    self.banks.map_prg_8k(1, base | sel(1));
                }
                self.banks
                    .map_prg_8k(2, base | if a & 0x02 != 0 { 2 } else { (a >> 1) & 0x02 });
                let last = if a & 0x800 != 0 {
                    (a & 0x07C) | if a & 0x06 != 0 { 0x03 } else { 0x01 }
                } else {
                    base | if a & 0x02 != 0 {
                        0x03
                    } else {
                        ((a >> 1) & 0x02) | 0x01
                    }
                };
                self.banks.map_prg_8k(3, last);
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x01 != 0, H, V))
            }
            // `Mapper200.h:18-24`.
            LatchKind::M200 => {
                let bank = a & 0x07;
                self.prg_16k(bank, bank);
                self.banks.map_chr_8k(bank);
                (self.addr != 0 || self.flag).then(|| pick(a & 0x08 != 0, V, H))
            }
            // `NovelDiamond.h:16-19`, 32 KiB pages; 54 is the same class in Mesen2.
            LatchKind::M54 | LatchKind::M201 => {
                self.banks.map_prg_32k(a & 0x03);
                self.banks.map_chr_8k(a & 0x07);
                None
            }
            // `Mapper202.h:19-31`.
            LatchKind::M202 => {
                let page = (a >> 1) & 0x07;
                if a & 0x09 == 0x09 {
                    self.prg_16k(page, page + 1);
                } else {
                    self.prg_16k(page, page);
                }
                self.banks.map_chr_8k(page);
                self.flag.then(|| pick(a & 0x01 != 0, H, V))
            }
            // `Mapper203.h:17-20`: the data, not the address.
            LatchKind::M203 => {
                self.prg_16k(r0 >> 2, r0 >> 2);
                self.banks.map_chr_8k(r0 & 0x03);
                None
            }
            // `Mapper204.h:15-22`.
            LatchKind::M204 => {
                let mask = a & 0x06;
                let page = mask + if mask == 0x06 { 0 } else { a & 0x01 };
                self.prg_16k(page, mask + if mask == 0x06 { 1 } else { a & 0x01 });
                self.banks.map_chr_8k(page);
                Some(pick(a & 0x10 != 0, H, V))
            }
            // `Mapper212.h:27-36`.
            LatchKind::M212 => {
                if a & 0x4000 != 0 {
                    self.prg_16k(a & 0x06, (a & 0x06) + 1);
                } else {
                    self.prg_16k(a & 0x07, a & 0x07);
                }
                self.banks.map_chr_8k(a & 0x07);
                Some(pick(a & 0x08 != 0, H, V))
            }
            // `Mapper213.h:15-18`, 32 KiB pages.
            LatchKind::M213 => {
                self.banks.map_chr_8k((a >> 3) & 0x07);
                self.banks.map_prg_32k((a >> 1) & 0x03);
                None
            }
            // `Mapper214.h:15-19`.
            LatchKind::M214 => {
                self.banks.map_chr_8k(a & 0x03);
                self.prg_16k((a >> 2) & 0x03, (a >> 2) & 0x03);
                None
            }
            // `Mapper221.h:22-37`; `addr` is `_mode`, `regs[0]` the inner PRG register.
            LatchKind::M221 => {
                let outer = (a & 0xFC) >> 2;
                if a & 0x02 != 0 {
                    if a & 0x0100 != 0 {
                        self.prg_16k(outer | r0, outer | 0x07);
                    } else {
                        let page = outer | (r0 & 0x06);
                        self.prg_16k(page, page + 1);
                    }
                } else {
                    self.prg_16k(outer | r0, outer | r0);
                }
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x01 != 0, H, V))
            }
            // `Mapper226.h:40-55` and `Mapper233.h:27-30`.
            LatchKind::M226 | LatchKind::M233 => {
                let middle = if self.kind == LatchKind::M233 {
                    i32::from(self.flag) << 5
                } else {
                    (r0 & 0x80) >> 2
                };
                let page = (r0 & 0x1F) | middle | ((r1 & 0x01) << 6);
                if r0 & 0x20 != 0 {
                    self.prg_16k(page, page);
                } else {
                    self.prg_16k(page & 0xFE, (page & 0xFE) + 1);
                }
                self.banks.map_chr_8k(0);
                (self.addr != 0).then(|| pick(r0 & 0x40 != 0, V, H))
            }
            // `Mapper227.h:14-46`.
            LatchKind::M227 => {
                let bank = ((a >> 2) & 0x1F) | ((a & 0x100) >> 3);
                let s = a & 0x01 != 0;
                let l = (a >> 9) & 0x01 != 0;
                if (a >> 7) & 0x01 != 0 {
                    if s {
                        self.prg_16k(bank & 0xFE, (bank & 0xFE) + 1);
                    } else {
                        self.prg_16k(bank, bank);
                    }
                } else {
                    let low = if s { bank & 0x3E } else { bank };
                    let high = if l { bank | 0x07 } else { bank & 0x38 };
                    self.prg_16k(low, high);
                }
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x02 != 0, H, V))
            }
            // `Mapper229.h:15-23`.
            LatchKind::M229 => {
                self.banks.map_chr_8k(a & 0xFF);
                if a & 0x1E == 0 {
                    self.prg_16k(0, 1);
                } else {
                    self.prg_16k(a & 0x1F, a & 0x1F);
                }
                Some(pick(a & 0x20 != 0, H, V))
            }
            // `Mapper230.h:19-50`: *Contra* mode is a 128 KiB UNROM at banks 0-7, the other mode
            // a 16/32 KiB multicart above it. `regs[0]` is the last value written.
            LatchKind::M230 => {
                self.banks.map_chr_8k(0);
                if self.flag {
                    // `Mapper230.h:23-26`: entering the mode selects bank 0 again.
                    let bank = if self.addr == 0 { 0 } else { r0 & 0x07 };
                    self.prg_16k(bank, 7);
                    (self.addr == 0).then_some(V)
                } else if self.addr == 0 {
                    self.prg_16k(8, 9);
                    Some(H)
                } else {
                    if r0 & 0x20 != 0 {
                        self.prg_16k((r0 & 0x1F) + 8, (r0 & 0x1F) + 8);
                    } else {
                        self.prg_16k((r0 & 0x1E) + 8, (r0 & 0x1E) + 9);
                    }
                    Some(pick(r0 & 0x40 != 0, V, H))
                }
            }
            // `Mapper231.h:21-26`.
            LatchKind::M231 => {
                let bank = ((a >> 5) & 0x01) | (a & 0x1E);
                self.prg_16k(bank & 0x1E, bank);
                self.banks.map_chr_8k(0);
                (self.addr != 0).then(|| pick(a & 0x80 != 0, H, V))
            }
            // `Bmc235.h:24-55`.
            LatchKind::M235 => {
                const CONFIG: [[(i32, bool); 4]; 4] = [
                    [(0x00, false), (0x00, true), (0x00, true), (0x00, true)],
                    [(0x00, false), (0x00, true), (0x20, false), (0x00, true)],
                    [(0x00, false), (0x00, true), (0x20, false), (0x40, false)],
                    [(0x00, false), (0x20, false), (0x40, false), (0x60, false)],
                ];
                self.banks.map_chr_8k(0);
                if self.addr == 0 && !self.flag {
                    self.prg_16k(0, 1);
                    return None;
                }
                let (high, open) =
                    CONFIG[usize::from(self.size_mode)][usize::from((self.addr >> 8) & 0x03)];
                let bank = high | (a & 0x1F);
                if open {
                    self.open_bus = 0b1111;
                } else if a & 0x800 != 0 {
                    let page = (bank << 1) | ((a >> 12) & 0x01);
                    self.prg_16k(page, page);
                } else {
                    self.prg_16k(bank << 1, (bank << 1) + 1);
                }
                Some(if a & 0x0400 != 0 {
                    NametableMirroring::SingleScreenA
                } else {
                    pick(a & 0x2000 != 0, H, V)
                })
            }
            // `Bmc70in1.h:33-51`: `regs` = bank mode, outer bank, PRG register, CHR register.
            LatchKind::M236 => {
                let outer = i32::from(self.regs[1]);
                let prg = i32::from(self.regs[2]);
                match self.regs[0] {
                    0x20 => {
                        let page = (outer | prg) & 0xFE;
                        self.prg_16k(page, page + 1);
                    }
                    0x30 => self.prg_16k(outer | prg, outer | prg),
                    _ => self.prg_16k(outer | prg, outer | 7),
                }
                self.banks.map_chr_8k(if self.chr_ram {
                    0
                } else {
                    i32::from(self.regs[3])
                });
                self.flag.then(|| pick(a & 0x20 != 0, H, V))
            }
            // `Mapper240.h:15-18`, 32 KiB pages.
            LatchKind::M240 => {
                self.banks.map_prg_32k((r0 >> 4) & 0x0F);
                self.banks.map_chr_8k(r0 & 0x0F);
                None
            }
            // `Mapper241.h:13-16`, 32 KiB pages.
            // `Mapper39.h:13-19` and `Kaiser7012.h:9-19` (power-on 1) are the same board.
            LatchKind::M39 | LatchKind::M241 | LatchKind::M346 => {
                self.banks.map_prg_32k(r0);
                self.banks.map_chr_8k(0);
                None
            }
            // `Mapper242.h:13-25`, 32 KiB pages.
            LatchKind::M242 => {
                self.banks.map_prg_32k((a >> 3) & 0x0F);
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x02 != 0, H, V))
            }
            // `Mapper244.h:9-40`, 32 KiB pages: `regs[0]` the PRG value, `regs[1]` the CHR one.
            LatchKind::M244 => {
                const PRG: [[u8; 4]; 4] = [[0, 1, 2, 3], [3, 2, 1, 0], [0, 2, 1, 3], [3, 1, 2, 0]];
                const CHR: [[u8; 8]; 8] = [
                    [0, 1, 2, 3, 4, 5, 6, 7],
                    [0, 2, 1, 3, 4, 6, 5, 7],
                    [0, 1, 4, 5, 2, 3, 6, 7],
                    [0, 4, 1, 5, 2, 6, 3, 7],
                    [0, 4, 2, 6, 1, 5, 3, 7],
                    [0, 2, 4, 6, 1, 3, 5, 7],
                    [7, 6, 5, 4, 3, 2, 1, 0],
                    [7, 6, 5, 4, 3, 2, 1, 0],
                ];
                let p = usize::from(self.regs[0]);
                let c = usize::from(self.regs[1]);
                self.banks
                    .map_prg_32k(i32::from(PRG[(p >> 4) & 0x03][p & 0x03]));
                self.banks
                    .map_chr_8k(i32::from(CHR[(c >> 4) & 0x07][c & 0x07]));
                None
            }
            // `ColorDreams46.h:24-28`, 32 KiB pages: `regs[0]` at `$6000-$7FFF`, `regs[1]` above.
            LatchKind::M46 => {
                self.banks.map_prg_32k(((r0 & 0x0F) << 1) | (r1 & 0x01));
                self.banks
                    .map_chr_8k(((r0 & 0xF0) >> 1) | ((r1 & 0x70) >> 4));
                None
            }
            // `Bmc51.h:27-39`, 8 KiB pages; `regs[0]` the bank, `regs[1]` the mode (power-on 1).
            LatchKind::M51 => {
                let bank = r0 << 2;
                if r1 & 0x01 != 0 {
                    self.banks.map_prg_32k(r0);
                    self.rom_6000 = Some(0x23 | bank);
                } else {
                    let low = bank | r1;
                    let high = bank | 0x0E;
                    self.banks.map_prg_8k(0, low);
                    self.banks.map_prg_8k(1, low + 1);
                    self.banks.map_prg_8k(2, high);
                    self.banks.map_prg_8k(3, high + 1);
                    self.rom_6000 = Some(0x2F | bank);
                }
                self.banks.map_chr_8k(0);
                Some(pick(r1 == 0x03, H, V))
            }
            // `Mapper60.h:20-27`: the reset counter selects a 16 KiB bank and its CHR.
            LatchKind::M60 => {
                self.prg_16k(r0, r0);
                self.banks.map_chr_8k(r0);
                None
            }
            // `Mapper61.h:20-30`.
            LatchKind::M61 => {
                let page = ((a & 0x0F) << 1) | ((a >> 5) & 0x01);
                if a & 0x10 != 0 {
                    self.prg_16k(page, page);
                } else {
                    self.prg_16k(page & 0xFE, (page & 0xFE) + 1);
                }
                self.banks.map_chr_8k(0);
                self.flag.then(|| pick(a & 0x80 != 0, H, V))
            }
            // `Mapper62.h:26-38`: the address and the data's low two bits.
            LatchKind::M62 => {
                let page = ((a & 0x3F00) >> 8) | (a & 0x40);
                if a & 0x20 != 0 {
                    self.prg_16k(page, page);
                } else {
                    self.prg_16k(page & 0xFE, (page & 0xFE) + 1);
                }
                self.banks.map_chr_8k(((a & 0x1F) << 2) | (r0 & 0x03));
                self.flag.then(|| pick(a & 0x80 != 0, H, V))
            }
            // `GoldenFive.h:14-33`: `regs[0]` the PRG register, `regs[1]` the outer block.
            LatchKind::M104 => {
                self.prg_16k(r0, r1 | 0x0F);
                self.banks.map_chr_8k(0);
                None
            }
            // puNES `mapper_108.c:75-87` (nesdev "INES Mapper 108"): `$8000-$FFFF` is the last
            // 32 KiB; `$6000-$7FFF` the register's 8 KiB bank, the last one on submapper 4;
            // CHR-ROM follows the register, CHR-RAM is never banked. Mesen2 `Bb.h` merges the
            // submappers and hangs submapper 1's Bubble Bobble (FIDELITY-LEADS J11).
            LatchKind::M108 => {
                self.banks.map_prg_32k(-1);
                self.rom_6000 = Some(if self.submapper == 4 { -1 } else { r0 });
                self.banks.map_chr_8k(if self.chr_ram { 0 } else { r0 });
                None
            }
            // `Nina03_06.h:22-26`, multicart mode, 32 KiB pages.
            LatchKind::M113 => {
                self.banks.map_prg_32k((r0 >> 3) & 0x07);
                self.banks.map_chr_8k((r0 & 0x07) | ((r0 >> 3) & 0x08));
                self.flag.then(|| pick(r0 & 0x80 != 0, V, H))
            }
            // `Mapper120.h:15-27`: 8 KiB pages 8-11 at `$8000`, the register's bank at `$6000`.
            LatchKind::M120 => {
                self.banks.map_prg_8k(0, 8);
                self.banks.map_prg_8k(1, 9);
                self.banks.map_prg_8k(2, 10);
                self.banks.map_prg_8k(3, 11);
                self.rom_6000 = Some(r0);
                self.banks.map_chr_8k(0);
                None
            }
            // `Lh32.h:16-30`: -4, -3, work RAM at `$C000`, -1; the register's bank at `$6000`.
            LatchKind::M125 => {
                self.banks.map_prg_8k(0, -4);
                self.banks.map_prg_8k(1, -3);
                self.banks.map_prg_ram_8k(2, 0);
                self.banks.map_prg_8k(3, -1);
                self.rom_6000 = Some(r0);
                self.banks.map_chr_8k(0);
                None
            }
            // `Mapper170.h`: NROM with a one-bit protection latch.
            LatchKind::M170 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_8k(0);
                None
            }
            // `Mapper174.h:14-23`.
            LatchKind::M174 => {
                let bank = (a >> 4) & 0x07;
                if a & 0x80 != 0 {
                    self.prg_16k(bank & 0x06, (bank & 0x06) + 1);
                } else {
                    self.prg_16k(bank, bank);
                }
                self.banks.map_chr_8k((a >> 1) & 0x07);
                Some(pick(a & 0x01 != 0, H, V))
            }
            // `Henggedianzi177.h:16-20`, 32 KiB pages.
            LatchKind::M177 => {
                self.banks.map_prg_32k(r0);
                self.banks.map_chr_8k(0);
                self.flag.then(|| pick(r0 & 0x20 != 0, H, V))
            }
            // `Henggedianzi179.h:17-24`: `$5000-$5FFF` the 32 KiB bank, `$8000+` the mirroring.
            LatchKind::M179 => {
                self.banks.map_prg_32k(r0 >> 1);
                self.banks.map_chr_8k(0);
                self.flag.then(|| pick(r1 & 0x01 != 0, H, V))
            }
            // `Mapper216.h:22-25`, 32 KiB pages.
            LatchKind::M216 => {
                self.banks.map_prg_32k(a & 0x01);
                self.banks.map_chr_8k((a & 0x0E) >> 1);
                None
            }
            // `Bmc810544CA1.h:20-30`.
            LatchKind::M261 => {
                let bank = (a >> 6) & 0xFFFE;
                if a & 0x40 != 0 {
                    self.prg_16k(bank, bank + 1);
                } else {
                    let page = bank | ((a >> 5) & 0x01);
                    self.prg_16k(page, page);
                }
                self.banks.map_chr_8k(a & 0x0F);
                Some(pick(a & 0x10 != 0, H, V))
            }
            // `Bmc80013B.h:23-32`: `regs[0]`, `regs[1]` and the mode in `regs[2]`.
            LatchKind::M274 => {
                let low = if regs[2] & 0x02 != 0 {
                    (r0 & 0x0F) | (r1 & 0x70)
                } else {
                    r0 & 0x03
                };
                self.prg_16k(low, r1 & 0x7F);
                self.banks.map_chr_8k(0);
                Some(pick(r0 & 0x10 != 0, V, H))
            }
            // `Bs5.h:14-35`, 8 KiB PRG and 2 KiB CHR pages: `regs[0..4]` PRG, `regs[4..8]` CHR,
            // power-on all -1 (`$FF`, wrapped).
            LatchKind::M286 => {
                for (slot, bank) in regs.iter().take(4).enumerate() {
                    self.banks.map_prg_8k(slot, i32::from(*bank));
                }
                for (slot, bank) in regs.iter().skip(4).enumerate() {
                    self.banks.map_chr_2k(slot * 2, i32::from(*bank));
                }
                None
            }
            // `Sachen_148.h:12-16`, 32 KiB pages.
            LatchKind::M148 => {
                self.banks.map_prg_32k((r0 >> 3) & 0x01);
                self.banks.map_chr_8k(r0 & 0x07);
                None
            }
            // `Kaiser7058.h:10-22`: 32 KiB of PRG, two 4 KiB CHR registers at `$F000`/`$F080`.
            LatchKind::M171 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_4k(0, r0);
                self.banks.map_chr_4k(4, r1);
                None
            }
            // `Kaiser7013B.h:10-24`: `$6000-$7FFF` banks `$8000`, the last 16 KiB fixed, `$8000+`
            // the mirroring (power-on vertical).
            LatchKind::M312 => {
                self.prg_16k(r0, -1);
                self.banks.map_chr_8k(0);
                Some(pick(r1 & 0x01 != 0, H, V))
            }
            // `Kaiser7016.h`: 8 KiB pages $0C-$0F at `$8000`, the register's page at `$6000`
            // (power-on 8).
            LatchKind::M306 => {
                self.banks.map_prg_8k(0, 0x0C);
                self.banks.map_prg_8k(1, 0x0D);
                self.banks.map_prg_8k(2, 0x0E);
                self.banks.map_prg_8k(3, 0x0F);
                self.rom_6000 = Some(r0);
                self.banks.map_chr_8k(0);
                None
            }
            // `T262.h:16-28`: `regs` = base, mode, lock, inner bank.
            LatchKind::M265 => {
                let base = i32::from(regs[0]);
                let bank = i32::from(regs[3]);
                self.prg_16k(base | bank, base | if regs[1] != 0 { bank } else { 7 });
                self.banks.map_chr_8k(0);
                self.flag.then(|| pick(a & 0x02 != 0, H, V))
            }
            // `Gs2004.h:14-20`: `$6000` is 8 KiB page $20, `$8000` a 32 KiB bank (power-on 7).
            LatchKind::M283 => {
                self.banks
                    .map_prg_32k(if self.flag { r0 & 0x07 } else { 0x07 });
                self.rom_6000 = Some(0x20);
                self.banks.map_chr_8k(0);
                None
            }
            // `A65AS.h:15-27`.
            LatchKind::M285 => {
                if r0 & 0x40 != 0 {
                    self.prg_16k(r0 & 0x1E, (r0 & 0x1E) + 1);
                } else {
                    let outer = (r0 & 0x30) >> 1;
                    self.prg_16k(outer | (r0 & 0x07), outer | 0x07);
                }
                self.banks.map_chr_8k(0);
                Some(if r0 & 0x80 != 0 {
                    if r0 & 0x20 != 0 {
                        NametableMirroring::SingleScreenB
                    } else {
                        NametableMirroring::SingleScreenA
                    }
                } else {
                    pick(r0 & 0x08 != 0, H, V)
                })
            }
            // `Gkcx1.h:15-19`, 32 KiB pages.
            LatchKind::M288 => {
                self.banks.map_prg_32k((a >> 3) & 0x03);
                self.banks.map_chr_8k(a & 0x07);
                None
            }
            // `BmcNtd03.h:18-31`.
            LatchKind::M290 => {
                let prg = (a >> 10) & 0x1E;
                if a & 0x80 != 0 {
                    let page = prg | ((a >> 6) & 0x01);
                    self.prg_16k(page, page);
                } else {
                    self.prg_16k(prg & 0xFE, (prg & 0xFE) + 1);
                }
                self.banks.map_chr_8k(((a & 0x0300) >> 5) | (a & 0x07));
                Some(pick(a & 0x400 != 0, H, V))
            }
            // `Bmc11160.h:18-24`, 32 KiB pages.
            LatchKind::M299 => {
                let bank = (r0 >> 4) & 0x07;
                self.banks.map_prg_32k(bank);
                self.banks.map_chr_8k((bank << 2) | (r0 & 0x03));
                Some(pick(r0 & 0x80 != 0, V, H))
            }
            // `Bmc190in1.h:14-20`.
            LatchKind::M300 => {
                let page = (r0 >> 2) & 0x07;
                self.prg_16k(page, page);
                self.banks.map_chr_8k(page);
                Some(pick(r0 & 0x01 != 0, H, V))
            }
            // `BmcK3046.h:14-22`.
            LatchKind::M336 => {
                self.prg_16k((r0 & 0x38) | (r0 & 0x07), (r0 & 0x38) | 0x07);
                self.banks.map_chr_8k(0);
                None
            }
            // `BmcG146.h:18-34`.
            LatchKind::M349 => {
                if a & 0x800 != 0 {
                    self.prg_16k((a & 0x1F) | (a & ((a & 0x40) >> 6)), (a & 0x18) | 0x07);
                } else if a & 0x40 != 0 {
                    self.prg_16k(a & 0x1F, a & 0x1F);
                } else {
                    self.prg_16k(a & 0x1E, (a & 0x1E) + 1);
                }
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x80 != 0, H, V))
            }
            // `Bmc60311C.h:20-44`: `regs` = inner (`$8000+`), mode (`$6000`), outer (`$6001`).
            LatchKind::M289 => {
                let (inner, mode, outer) = (r0, i32::from(regs[1]), i32::from(regs[2]));
                let page = outer | if mode & 0x04 != 0 { 0 } else { inner };
                match mode & 0x03 {
                    0 => self.prg_16k(page, page),
                    1 => self.prg_16k(page & 0xFE, (page & 0xFE) + 1),
                    2 => self.prg_16k(page, outer | 7),
                    _ => {}
                }
                self.banks.map_chr_8k(0);
                Some(pick(mode & 0x08 != 0, H, V))
            }
            // `Bmc8157.h:22-44`, DIP switch 0 (so never its open-bus case).
            LatchKind::M301 => {
                let inner0 = (a >> 2) & 0x07;
                let inner1 = ((a >> 7) & 0x01) | ((a >> 8) & 0x02);
                let outer = (((a >> 8) & 0x01) << 6) | (((a >> 5) & 0x03) << 3);
                let base = match inner1 {
                    0 => 0,
                    1 => inner0,
                    _ => 7,
                };
                self.prg_16k(outer | inner0, outer | base);
                self.banks.map_chr_8k(0);
                Some(pick(a & 0x02 != 0, H, V))
            }
            // `Bmc64in1NoRepeat.h:20-35`; `regs[4]` keeps the `$8000` page the UNROM-like mode
            // leaves in place.
            LatchKind::M314 => {
                let r1 = i32::from(regs[1]);
                let bank = ((r1 & 0x1F) << 1) | ((r1 >> 6) & 0x01);
                if r0 & 0x80 != 0 {
                    if r1 & 0x80 != 0 {
                        let page = (r1 & 0x1F) << 1;
                        self.prg_16k(page, page + 1);
                        self.regs[4] = page.to_le_bytes()[0];
                    } else {
                        self.prg_16k(bank, bank);
                        self.regs[4] = bank.to_le_bytes()[0];
                    }
                } else {
                    self.prg_16k(i32::from(self.regs[4]), bank);
                }
                self.banks
                    .map_chr_8k((i32::from(regs[2]) << 2) | ((r0 >> 1) & 0x03));
                Some(pick(r0 & 0x20 != 0, H, V))
            }
            // `Hp898f.h:18-27`.
            LatchKind::M319 => {
                let r1 = i32::from(regs[1]);
                let prg = (r1 >> 3) & 7;
                let mask = (r1 >> 4) & 4;
                self.banks
                    .map_chr_8k(((r0 >> 4) & 0x07) & !(((r0 & 0x01) << 2) | (r0 & 0x02)));
                self.prg_16k(prg & !mask, prg | mask);
                Some(pick(r1 & 0x80 != 0, V, H))
            }
            // `Lh51.h:9-24`, 8 KiB pages: one register at `$8000`, pages 13-15 fixed.
            LatchKind::M309 => {
                self.banks.map_prg_8k(0, r0 & 0x0F);
                self.banks.map_prg_8k(1, 13);
                self.banks.map_prg_8k(2, 14);
                self.banks.map_prg_8k(3, 15);
                self.banks.map_chr_8k(0);
                self.flag.then(|| pick(r1 & 0x08 != 0, H, V))
            }
            // `Bmc830425C4391T.h:16-28`: `regs` = inner, outer, mode.
            LatchKind::M320 => {
                let outer = i32::from(regs[1]) << 3;
                if regs[2] != 0 {
                    self.prg_16k((r0 & 0x07) | outer, 0x07 | outer);
                } else {
                    self.prg_16k(r0 | outer, 0x0F | outer);
                }
                self.banks.map_chr_8k(0);
                None
            }
            // `Edu2000.h:18-22`, 32 KiB pages; the work RAM bank is `prg_ram_window`'s.
            LatchKind::M329 => {
                self.banks.map_prg_32k(r0 & 0x1F);
                self.banks.map_chr_8k(0);
                None
            }
            // `Bmc12in1.h:17-29`, 4 KiB CHR pages: `regs` = R0, R1, mode.
            LatchKind::M331 => {
                let mode = i32::from(regs[2]);
                let bank = (mode & 0x03) << 3;
                self.banks.map_chr_4k(0, (r0 >> 3) | (bank << 2));
                self.banks.map_chr_4k(4, (r1 >> 3) | (bank << 2));
                if mode & 0x08 != 0 {
                    let page = bank | (r0 & 0x06);
                    self.prg_16k(page, page + 1);
                } else {
                    self.prg_16k(bank | (r0 & 0x07), bank | 0x07);
                }
                Some(pick(mode & 0x04 != 0, H, V))
            }
            // `Super40in1Ws.h:14-26`: `regs` = PRG value, CHR value, lock.
            LatchKind::M332 => {
                let inverted = ((r0 >> 3) & 0x01) ^ 0x01;
                self.prg_16k(r0 & !inverted, r0 | inverted);
                self.banks.map_chr_8k(r1);
                Some(pick(r0 & 0x10 != 0, H, V))
            }
            // `DreamTech01.h:9-17`: `$5020` banks `$8000`, 16 KiB page 8 at `$C000`.
            LatchKind::M521 => {
                self.prg_16k(r0 & 0x07, 8);
                self.banks.map_chr_8k(0);
                None
            }
            // `Subor166.h:20-41`: `regs` = the four masked registers.
            LatchKind::M166 | LatchKind::M167 => {
                let alt = self.kind == LatchKind::M167;
                let outer = ((i32::from(regs[0]) ^ i32::from(regs[1])) & 0x10) << 1;
                let inner = i32::from(regs[2]) ^ i32::from(regs[3]);
                if regs[1] & 0x08 != 0 {
                    let bank = (outer | inner) & 0xFE;
                    if alt {
                        self.prg_16k(bank + 1, bank);
                    } else {
                        self.prg_16k(bank, bank + 1);
                    }
                } else if regs[1] & 0x04 != 0 {
                    self.prg_16k(0x1F, outer | inner);
                } else {
                    self.prg_16k(outer | inner, if alt { 0x20 } else { 0x07 });
                }
                self.banks.map_chr_8k(0);
                None
            }
            // `FaridUnrom.h:28-38`: the latched outer bank and the inner one.
            LatchKind::M324 => {
                let outer = (r0 & 0x70) >> 1;
                self.prg_16k((r0 & 0x07) | outer, 0x07 | outer);
                self.banks.map_chr_8k(0);
                None
            }
            // `Lh10.h:20-27`: R6, R7, work RAM at `$C000`, the last bank; `$6000` page -2.
            LatchKind::M522 => {
                self.banks.map_prg_8k(0, i32::from(regs[6]));
                self.banks.map_prg_8k(1, i32::from(regs[7]));
                self.banks.map_prg_ram_8k(2, 0);
                self.banks.map_prg_8k(3, -1);
                self.rom_6000 = Some(-2);
                self.banks.map_chr_8k(0);
                None
            }
            // `Bmc255.h:14-21`.
            LatchKind::M255 => {
                let bit = i32::from(a & 0x1000 == 0);
                let bank = ((a >> 8) & 0x40) | ((a >> 6) & 0x3F);
                self.prg_16k(bank & !bit, bank | bit);
                self.banks.map_chr_8k(((a >> 8) & 0x40) | (a & 0x3F));
                Some(pick(a & 0x2000 != 0, H, V))
            }
        }
    }

    /// Latches a write. `addr` is the CPU address, `value` the data.
    // One arm per board, each with its Mesen2 lines: a flat table, as `map` above it.
    #[allow(clippy::too_many_lines)]
    fn latch(&mut self, address: u16, value: u8) {
        match self.kind {
            // `Mapper57.h:36-40`.
            LatchKind::M57 => match address & 0x8800 {
                0x8000 => self.regs[0] = value,
                0x8800 => self.regs[1] = value,
                _ => {}
            },
            // `Mapper221.h:40-50`.
            LatchKind::M221 => match address & 0xC000 {
                0x8000 => self.addr = address,
                0xC000 => self.regs[0] = low_byte(address & 0x07),
                _ => {}
            },
            // `Mapper226.h:57-62`.
            LatchKind::M226 | LatchKind::M233 => {
                self.regs[usize::from(address & 0x01)] = value;
                self.addr = 1;
            }
            // `Mapper230.h:36-50`.
            LatchKind::M230 => {
                self.regs[0] = value;
                self.addr = 1;
            }
            // `Bmc70in1.h:62-73`.
            LatchKind::M236 => {
                if address & 0x4000 != 0 {
                    self.regs[0] = low_byte(address & 0x30);
                    self.regs[2] = low_byte(address & 0x07);
                } else {
                    self.flag = true;
                    self.addr = address;
                    if self.chr_ram {
                        self.regs[1] = low_byte((address & 0x03) << 3);
                    } else {
                        self.regs[3] = low_byte(address & 0x07);
                    }
                }
            }
            // `Mapper244.h:33-40`.
            LatchKind::M244 => {
                if value & 0x08 != 0 {
                    self.regs[1] = value;
                } else {
                    self.regs[0] = value;
                }
            }
            // `ColorDreams46.h:31-37`: `$8000+` here, `$6000-$7FFF` in `intercept_cpu_write`.
            LatchKind::M46 => self.regs[1] = value,
            // `Bmc51.h:42-52`.
            LatchKind::M51 => {
                if (0x6000..=0x7FFF).contains(&address) {
                    self.regs[1] = ((value >> 3) & 0x02) | ((value >> 1) & 0x01);
                } else if (0xC000..=0xDFFF).contains(&address) {
                    self.regs[0] = value & 0x0F;
                    self.regs[1] = ((value >> 3) & 0x02) | (self.regs[1] & 0x01);
                } else {
                    self.regs[0] = value & 0x0F;
                }
            }
            // `Mapper62.h:26-28`.
            LatchKind::M62 => {
                self.addr = address;
                self.regs[0] = value;
                self.flag = true;
            }
            // `GoldenFive.h:22-33`.
            LatchKind::M104 => {
                if address >= 0xC000 {
                    self.regs[0] = (self.regs[0] & 0xF0) | (value & 0x0F);
                } else if address <= 0x9FFF && value & 0x08 != 0 {
                    self.regs[0] = (self.regs[0] & 0x0F) | ((value << 4) & 0x70);
                    self.regs[1] = (value << 4) & 0x70;
                }
            }
            // puNES `mapper_108.c:41-55,63-69`: one register from `$F000` (submapper 1), `$E000`
            // (2) or `$8000` (3 and 4).
            LatchKind::M108 => {
                let start = match self.submapper {
                    1 => 0xF000,
                    2 => 0xE000,
                    _ => 0x8000,
                };
                if address >= start {
                    self.regs[0] = value;
                }
            }
            // `Henggedianzi179.h:17-24`.
            LatchKind::M179 => {
                if address >= 0x8000 {
                    self.regs[1] = value;
                    self.flag = true;
                } else {
                    self.regs[0] = value;
                }
            }
            // `Bmc80013B.h:35-44`.
            LatchKind::M274 => {
                let reg = (address >> 13) & 0x03;
                if reg == 0 {
                    self.regs[0] = value;
                } else {
                    self.regs[1] = value;
                    self.regs[2] = low_byte(reg);
                }
            }
            // `Bs5.h:20-33`: A10-A11 the slot; `$A000` writes only when A4 (DIP 0) is set.
            LatchKind::M286 => {
                let slot = usize::from((address >> 10) & 0x03);
                match address & 0xF000 {
                    0x8000 => self.regs[4 + slot] = low_byte(address & 0x1F),
                    0xA000 if address & 0x10 != 0 => self.regs[slot] = low_byte(address & 0x0F),
                    _ => {}
                }
            }
            // No register reached through here: 60 has none, the others' are below `$8000`.
            LatchKind::M60 | LatchKind::M120 | LatchKind::M125 | LatchKind::M170 => {}
            // `Kaiser7058.h:17-21`, registers `$F000-$FFFF` under mask `$F080`.
            LatchKind::M171 => match address & 0xF080 {
                0xF000 => self.regs[0] = value,
                0xF080 => self.regs[1] = value,
                _ => {}
            },
            // `Kaiser7016.h` `WriteRegister`, mask `$D943`.
            LatchKind::M306 => {
                let mode = address & 0x30 == 0x30;
                match address & 0xD943 {
                    0xD943 => {
                        self.regs[0] = if mode {
                            0x0B
                        } else {
                            low_byte((address >> 2) & 0x0F)
                        };
                    }
                    0xD903 => {
                        self.regs[0] = if mode {
                            0x08 | low_byte((address >> 2) & 0x03)
                        } else {
                            0x0B
                        };
                    }
                    _ => {}
                }
            }
            LatchKind::M312 => {
                if address < 0x8000 {
                    self.regs[0] = value;
                } else {
                    self.regs[1] = value;
                }
            }
            // `T262.h:28-36`: the address fields until A13 locks them; the data's low bits always.
            LatchKind::M265 => {
                if self.regs[2] == 0 {
                    self.regs[0] = low_byte(((address & 0x60) >> 2) | ((address & 0x100) >> 3));
                    self.regs[1] = u8::from(address & 0x80 != 0);
                    self.regs[2] = u8::from(address & 0x2000 != 0);
                    self.addr = address;
                    self.flag = true;
                }
                self.regs[3] = value & 0x07;
            }
            // `Bmc60311C.h:46-60`.
            LatchKind::M289 => {
                if address >= 0x8000 {
                    self.regs[0] = value & 0x07;
                } else {
                    match address & 0xE001 {
                        0x6000 => self.regs[1] = value & 0x0F,
                        0x6001 => self.regs[2] = value,
                        _ => {}
                    }
                }
            }
            // `Bmc64in1NoRepeat.h:37-44`.
            LatchKind::M314 => {
                if address < 0x8000 {
                    self.regs[usize::from(address & 0x03)] = value;
                } else {
                    self.regs[3] = value;
                }
            }
            // `Hp898f.h:29-34`: `(addr & $6000) == $6000`, A2 the register.
            LatchKind::M319 => {
                if address & 0x6000 == 0x6000 {
                    self.regs[usize::from((address & 0x04) >> 2)] = value;
                }
            }
            // `Lh51.h:18-27`.
            LatchKind::M309 => match address & 0xE000 {
                0x8000 => self.regs[0] = value,
                0xE000 => {
                    self.regs[1] = value;
                    self.flag = true;
                }
                _ => {}
            },
            // `Bmc830425C4391T.h:30-36`.
            LatchKind::M320 => {
                self.regs[0] = value & 0x0F;
                if address & 0xFFE0 == 0xF0E0 {
                    self.regs[1] = low_byte(address & 0x0F);
                    self.regs[2] = low_byte((address >> 4) & 0x01);
                }
            }
            // `Bmc12in1.h:31-37`.
            LatchKind::M331 => match address & 0xE000 {
                0xA000 => self.regs[0] = value,
                0xC000 => self.regs[1] = value,
                0xE000 => self.regs[2] = value & 0x0F,
                _ => {}
            },
            // `Super40in1Ws.h:16-26`: `$6000-$6FFF`, A0 picks CHR or PRG, until bit 5 locks.
            LatchKind::M332 => {
                if self.regs[2] == 0 {
                    if address & 0x01 != 0 {
                        self.regs[1] = value;
                    } else {
                        self.regs[2] = u8::from(value & 0x20 != 0);
                        self.regs[0] = value;
                    }
                }
            }
            // `Kaiser7012.h:15-19`: two exact addresses.
            LatchKind::M346 => match address {
                0xE0A0 => self.regs[0] = 0,
                0xEE36 => self.regs[0] = 1,
                _ => {}
            },
            // `Subor166.h:20-26`.
            LatchKind::M166 | LatchKind::M167 => match address & 0xE000 {
                0x8000 => self.regs[0] = value & 0x10,
                0xA000 => self.regs[1] = value & 0x1C,
                0xC000 => self.regs[2] = value & 0x1F,
                _ => self.regs[3] = value & 0x1F,
            },
            // `FaridUnrom.h:28-34`: bit 7 rising latches the outer bits, until bit 3 locks them.
            LatchKind::M324 => {
                let reg = self.regs[0];
                let mut next = reg;
                if reg & 0x08 == 0 && reg & 0x80 == 0 && value & 0x80 != 0 {
                    next = (reg & 0x87) | (value & 0x78);
                }
                self.regs[0] = (next & 0x78) | (value & 0x87);
            }
            // `Lh10.h:29-34`: `$8000` the index, `$8001` the data; `$C000-$DFFF` is work RAM.
            LatchKind::M522 => match address & 0xE001 {
                0x8000 => self.addr = u16::from(value & 0x07),
                0x8001 => self.regs[usize::from(self.addr & 0x07)] = value,
                _ => {}
            },
            LatchKind::M113
            | LatchKind::M177
            | LatchKind::M283
            | LatchKind::M285
            | LatchKind::M299
            | LatchKind::M300
            | LatchKind::M336 => {
                self.regs[0] = value;
                self.flag = true;
            }
            // `Edu2000.h:24-28` and `DreamTech01.h:15-17` too.
            LatchKind::M329
            | LatchKind::M521
            | LatchKind::M39
            | LatchKind::M148
            | LatchKind::M203
            | LatchKind::M240
            | LatchKind::M241 => self.regs[0] = value,
            _ => {
                self.addr = address;
                self.flag = true;
            }
        }
    }

    fn power_on(&mut self, memory: &mut CartridgeMemory) {
        self.addr = 0;
        self.regs = [0; 8];
        self.flag = false;
        // `Mapper230.h:14`: `InitMapper` calls `Reset(true)`, which toggles into *Contra* mode.
        match self.kind {
            LatchKind::M230 => self.flag = true,
            // `Bmc51.h:17-18`: "_mode = 1".
            LatchKind::M51 => self.regs[1] = 1,
            // `Kaiser7016.h`: "_prgReg = 8".
            LatchKind::M306 => self.regs[0] = 8,
            // `Kaiser7012.h:11`: "SelectPrgPage(0, 1)".
            LatchKind::M346 => self.regs[0] = 1,
            // `Bmc64in1NoRepeat.h:16-19`.
            LatchKind::M314 => {
                self.regs[0] = 0x80;
                self.regs[1] = 0x43;
            }
            // `Bs5.h:16-19`: every slot -1.
            LatchKind::M286 => self.regs = [0xFF; 8],
            _ => {}
        }
        let mirroring = self.map();
        if self.kind.mirrors_at_power_on() {
            if let Some(mirroring) = mirroring {
                memory.set_mirroring(mirroring);
            }
        }
        // 200, 202, 226, 231, 233 and 236 do not reach `SetMirroringType` before a write, and
        // `map` keys their answer off `addr`/`flag`, which a write sets.
        self.flag = self.kind == LatchKind::M230;
    }

    /// Whether a CPU read of `$8000-$FFFF` has to come through [`Mapper::try_read_prg`]: 59 with
    /// its DIP read on, 236 in its DIP mode, and any open-bus window.
    const fn reads_are_special(&self) -> bool {
        self.open_bus != 0
            || (matches!(self.kind, LatchKind::M59) && self.flag)
            || (matches!(self.kind, LatchKind::M236) && self.regs[0] == 0x10)
    }
}

impl Mapper for LatchMulti {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u16(self.addr);
        writer.write_raw(&self.regs);
        writer.write_bool(self.flag);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.addr = reader.read_u16()?;
        reader.read_into(&mut self.regs)?;
        self.flag = reader.read_bool()?;
        // The windows are in the state block already; this rebuilds `open_bus`.
        let _ = self.map();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.power_on(memory);
    }

    /// Mappers 60, 230 and 233 react to RESET, which is what they are for: nesdev, "INES Mapper
    /// 230" ("pressing reset switches between the Contra game and the multicart menu") and
    /// Mesen2 `Mapper230.h:19-34`, `Mapper233.h:14-24`. 233 also takes 226's register clear
    /// (`Mapper226.h:24-33`).
    fn soft_reset(&mut self, memory: &mut CartridgeMemory) {
        match self.kind {
            LatchKind::M230 => {
                self.flag = !self.flag;
                self.addr = 0;
                if let Some(mirroring) = self.map() {
                    memory.set_mirroring(mirroring);
                }
            }
            // `Mapper60.h:18-27`: "_resetCounter = (_resetCounter + 1) % 4", the board's whole
            // function, a reset-switched 4-in-1 (nesdev, "INES Mapper 060").
            LatchKind::M60 => {
                self.regs[0] = (self.regs[0] + 1) % 4;
                let _ = self.map();
            }
            // `FaridUnrom.h:15-20`: RESET keeps only the inner bank and the latch-arm bit, which
            // releases the lock — the board's documented way back to its menu.
            LatchKind::M324 => {
                self.regs[0] &= 0x87;
                let _ = self.map();
            }
            LatchKind::M233 => {
                self.regs = [0; 8];
                self.flag = !self.flag;
                let _ = self.map();
            }
            _ => {}
        }
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        let consumed = match self.kind {
            // `Mapper240.h:9-10`: the register is `$4020-$5FFF`.
            LatchKind::M240 => (0x4020..=0x5FFF).contains(&address),
            // `ColorDreams46.h:9,33`: `regs[0]` at `$6000-$7FFF`.
            LatchKind::M46 if (0x6000..=0x7FFF).contains(&address) => {
                self.regs[0] = value;
                let _ = self.map();
                return true;
            }
            // `Bmc51.h:10` (registers from `$6000`), `Nina03_06.h:12,20` (`$4100-$5FFF` under
            // `$E100`) and `Henggedianzi179.h:13` (`$5000-$5FFF`) go through `latch`.
            // and `Kaiser7013B.h:8` (registers from `$6000`).
            LatchKind::M51 | LatchKind::M289 | LatchKind::M312 | LatchKind::M319 => {
                (0x6000..=0x7FFF).contains(&address)
            }
            // `Bmc60311C.h:10`, `Hp898f.h:10` (registers from `$6000`) and
            // `Bmc64in1NoRepeat.h:13` (`$5000-$5003`).
            LatchKind::M314 => (0x5000..=0x5003).contains(&address),
            // `Super40in1Ws.h:10-11` (`$6000-$6FFF`) and `DreamTech01.h:9-10` (`$5020`).
            LatchKind::M332 => (0x6000..=0x6FFF).contains(&address),
            LatchKind::M521 => address == 0x5020,
            LatchKind::M113 => (0x4100..=0x5FFF).contains(&address) && address & 0xE100 == 0x4100,
            LatchKind::M179 => (0x5000..=0x5FFF).contains(&address),
            // `Mapper120.h:9` (`$41FF`) and `Lh32.h:12` (`$6000`): the register is the bank of
            // `$6000-$7FFF`.
            LatchKind::M120 if address == 0x41FF => {
                self.regs[0] = value;
                let _ = self.map();
                return true;
            }
            LatchKind::M125 if address == 0x6000 => {
                self.regs[0] = value;
                let _ = self.map();
                return true;
            }
            // `Mapper170.h:14-20`: writes at `$6502` and `$7000` latch `(value << 1) & $80`.
            LatchKind::M170 if address == 0x6502 || address == 0x7000 => {
                self.regs[0] = (value << 1) & 0x80;
                return true;
            }
            // `Mapper216.h:16`: `$5000` is a register too.
            LatchKind::M216 if address == 0x5000 => {
                self.addr = address;
                let _ = self.map();
                return true;
            }
            _ => false,
        };
        if consumed {
            self.latch(address, value);
            if let Some(mirroring) = self.map() {
                memory.set_mirroring(mirroring);
            }
        }
        consumed
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000
            || matches!(
                self.kind,
                LatchKind::M113 | LatchKind::M120 | LatchKind::M170 | LatchKind::M240
            )
        {
            return;
        }
        // `Lh32.h:18`: `$C000-$DFFF` is work RAM, the board's only register being `$6000`.
        if matches!(self.kind, LatchKind::M125 | LatchKind::M522)
            && (self.kind == LatchKind::M125 || (0xC000..=0xDFFF).contains(&address))
        {
            let _ = self
                .banks
                .try_write_prg_window(address, value, memory.prg_ram_mut(), true);
            return;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
    }

    /// `$6000-$7FFF` is PRG-ROM (51, 108, 120, 125) or the board's register window (46, 170,
    /// 212's read-back), never work RAM, on these kinds.
    fn prg_ram_enabled(&self) -> bool {
        !matches!(
            self.kind,
            LatchKind::M46
                | LatchKind::M212
                | LatchKind::M312
                | LatchKind::M306
                | LatchKind::M283
                | LatchKind::M522
                | LatchKind::M289
                | LatchKind::M319
                | LatchKind::M51
                | LatchKind::M108
                | LatchKind::M120
                | LatchKind::M125
                | LatchKind::M170
        )
    }

    /// `Edu2000.h:21`: mapper 329's `$6000-$7FFF` is one of four 8 KiB work RAM banks, chosen by
    /// bits 6-7 of its register.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let len = memory.prg_ram().len();
        if self.kind == LatchKind::M329 && len != 0 {
            ((usize::from(self.regs[0] >> 6) * 0x2000) % len, len)
        } else {
            (0, len)
        }
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.reads_are_special() {
            return [NO_ROM_WINDOW; 4];
        }
        self.banks.rom_windows(memory.rom().prg_rom().len())
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        let rom = memory.rom().prg_rom();
        let ram = memory.prg_ram();
        // `Mapper212.h:19-25`: bit 7 of a `$6000-$7FFF` read under mask `$E010` is driven high.
        // The other bits are open bus, which a board cannot see; the high address byte is what
        // an absolute read leaves on the bus. FIDELITY-LEADS records the approximation.
        if self.kind == LatchKind::M212 && (0x6000..=0x7FFF).contains(&address) {
            let open = address.to_be_bytes()[0];
            return Some(if address & 0xE010 == 0x6000 {
                open | 0x80
            } else {
                open
            });
        }
        if (0x6000..=0x7FFF).contains(&address) {
            if let Some(bank) = self.rom_6000 {
                let count = i32::try_from(rom.len() / 0x2000).unwrap_or(1).max(1);
                let offset = usize::try_from(bank.rem_euclid(count)).unwrap_or(0) * 0x2000;
                return rom.get(offset + usize::from(address & 0x1FFF)).copied();
            }
            // `Mapper170.h:24-27`: `$7001` and `$7777` read the latch over the high address bits.
            if self.kind == LatchKind::M170 && (address == 0x7001 || address == 0x7777) {
                return Some(self.regs[0] | (address.to_be_bytes()[0] & 0x7F));
            }
        }
        // `Mapper216.h:18-21`: "For Videopoker Bonza?", `$5000` reads 0.
        if self.kind == LatchKind::M216 && address == 0x5000 {
            return Some(0);
        }
        if address >= 0x8000 {
            if self.open_bus & (1 << ((address - 0x8000) >> 13)) != 0 {
                return None;
            }
            // `UnlD1038.h:22-28`: the DIP switches (none set) instead of ROM.
            if self.kind == LatchKind::M59 && self.flag {
                return Some(0);
            }
            // `Bmc70in1.h:54-60`: mode `$10` reads `(addr & $FFF0) | DIP`.
            if self.kind == LatchKind::M236 && self.regs[0] == 0x10 {
                return self.banks.read_prg(address & 0xFFF0, rom, ram);
            }
        }
        self.banks.read_prg(address, rom, ram)
    }
}
