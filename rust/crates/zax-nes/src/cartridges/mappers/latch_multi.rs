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
    regs: [u8; 4],
    /// 230's *Contra* mode, 233's reset-toggled PRG bit, 59's DIP-switch read, 236's DIP read
    /// mode: one per-kind flag.
    flag: bool,
    /// The 8 KiB PRG windows that read open bus (235, 63), one bit each. Derived.
    open_bus: u8,
    /// Mapper 235's mode, from the size of its PRG-ROM (`Bmc235.h`, the `switch` on
    /// `GetPrgPageCount()`).
    size_mode: u8,
    /// Whether mapper 236 has CHR-RAM, so that its register drives the outer PRG bank instead of
    /// CHR (`Bmc70in1.h`, `_useOuterBank`).
    chr_ram: bool,
}

impl LatchMulti {
    pub(crate) fn new(memory: &CartridgeMemory, kind: LatchKind) -> Self {
        let pages_16k = memory.rom().prg_rom().len() / 0x4000;
        Self {
            banks: windows_for(memory),
            kind,
            addr: 0,
            regs: [0; 4],
            flag: false,
            open_bus: 0,
            size_mode: match pages_16k {
                64 => 0,
                128 => 1,
                256 => 2,
                _ => 3,
            },
            chr_ram: memory.rom().chr_rom().is_empty(),
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
            // `NovelDiamond.h:16-19`, 32 KiB pages.
            LatchKind::M201 => {
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
            LatchKind::M241 => {
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
            LatchKind::M203 | LatchKind::M240 | LatchKind::M241 => self.regs[0] = value,
            _ => {
                self.addr = address;
                self.flag = true;
            }
        }
    }

    fn power_on(&mut self, memory: &mut CartridgeMemory) {
        self.addr = 0;
        self.regs = [0; 4];
        self.flag = false;
        // `Mapper230.h:14`: `InitMapper` calls `Reset(true)`, which toggles into *Contra* mode.
        if self.kind == LatchKind::M230 {
            self.flag = true;
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

    /// Mappers 230 and 233 react to RESET, which is what they are for: nesdev, "INES Mapper
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
            LatchKind::M233 => {
                self.regs = [0; 4];
                self.flag = !self.flag;
                let _ = self.map();
            }
            _ => {}
        }
    }

    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        // `Mapper240.h:9-10`: the register is `$4020-$5FFF`.
        if self.kind == LatchKind::M240 && (0x4020..=0x5FFF).contains(&address) {
            self.latch(address, value);
            let _ = self.map();
            return true;
        }
        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 || self.kind == LatchKind::M240 {
            return;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
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
        if self.kind == LatchKind::M212 && (0x6000..=0x7FFF).contains(&address) && ram.is_empty() {
            let open = address.to_be_bytes()[0];
            return Some(if address & 0xE010 == 0x6000 {
                open | 0x80
            } else {
                open
            });
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
