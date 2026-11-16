//! Lot MAPPERS-P: multicarts whose ASIC switches between two mapper chips at run time
//! (`docs/progress/MAPPERS-P.md`).
//!
//! Read from puNES 0.111 (`mapper_351.c`, `mapper_446.c`, `mapper_556.c`; read, not copied).
//! The board holds each chip whole ([`Mmc3`], [`Vrc2Vrc4`], [`Mmc1`], and for 446 [`Vrc6`]) and
//! routes the CPU writes, the CPU-cycle and PPU-bus hooks and the IRQ line to the chip its mode
//! register selects; the chip's windows are then narrowed to the board's outer bank with
//! [`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`], which is puNES's
//! `(base & ~mask) | (value & mask)` applied after the chip's own swap. An [`super::IrqBoard`]
//! variant, boxed, because both chips' IRQs are that enum's business.
#[cfg(feature = "board-446")]
use super::Vrc6;
use super::{Mapper, Mmc1, Mmc3, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipSwitchKind {
    /// 556, `mapper_556.c` (JY-215 relative): four outer registers written in turn at
    /// `$5000-$5FFF` (until `reg[3]` D7 locks them), `reg[2]` D7 selecting a VRC4 (A0 on CPU
    /// A0|A2, A1 on A1|A3, A2 and A3 swapped above A11) instead of the MMC3.
    M556,
    /// 351, `mapper_351.c` (Techline XB): `$5000` D0-D1 pick the MMC3, the MMC1 or a VRC4e
    /// (A2/A3, swapped above A11), `$5001` the outer PRG bank, `$5002` the NROM, CHR-ROM and
    /// mask modes; `$5000-$5FFF` reads the DIP switches.
    M351,
    /// 446, `mapper_446.c` (Mindkids SMD172B flash multicart): `$5000-$5007` until `$5000` D7
    /// locks the board into one of its personalities (UNROM, CNROM, ANROM, GNROM, NROM, the
    /// Bandai latch, the MMC1, the MMC3 with its TLSROM and mapper-189 variants, VRC2, VRC4 and
    /// VRC6) inside the outer bank `$5001`-`$5003` and `$5006` give it; until then the PRG-ROM
    /// is a flash chip the menu programs.
    M446,
}

impl ChipSwitchKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            556 => Self::M556,
            351 => Self::M351,
            446 => Self::M446,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M556 => 556,
            Self::M351 => 351,
            Self::M446 => 446,
        }
    }
}

/// A board of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipSwitch {
    kind: ChipSwitchKind,
    /// The outer registers, their meaning per kind.
    regs: [u8; 4],
    /// 556's write cursor.
    index: u8,
    mmc3: Mmc3,
    /// 556's and 351's VRC4; 446's VRC4 wired as mapper 23 (A0|A2, A1|A3).
    vrc: Vrc2Vrc4,
    /// 351's and 446's third chip.
    mmc1: Mmc1,
    /// 446's board, `None` for the other kinds.
    #[cfg(feature = "board-446")]
    ext_446: Option<Box<Mindkids>>,
}

/// The chip a board is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chip {
    Mmc3,
    Vrc4,
    Mmc1,
    /// 446's other chips and its discrete latches.
    Vrc2,
    Vrc4b,
    Vrc6,
    Discrete,
}

/// 446's personalities (`enum _m116_mappers` of `mapper_446.c`).
mod mode {
    pub const UNROM: u8 = 0;
    pub const MMC3: u8 = 1;
    pub const NROM: u8 = 2;
    pub const CNROM: u8 = 3;
    pub const ANROM: u8 = 4;
    pub const SLROM: u8 = 5;
    pub const SNROM: u8 = 6;
    pub const GNROM: u8 = 8;
    pub const BANDAI: u8 = 11;
    pub const TLSROM: u8 = 12;
    pub const M189: u8 = 13;
    pub const VRC6: u8 = 14;
    pub const VRC2_22: u8 = 15;
    pub const VRC4_25: u8 = 16;
    pub const VRC4_23: u8 = 17;
}

/// Mapper 446's board around the chips: its registers, the discrete modes' latch and windows,
/// the chips only it carries, and the flash chip.
#[cfg(feature = "board-446")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mindkids {
    /// `m446.reg`, `$5000-$5007` by A0-A2.
    reg: [u8; 8],
    /// `m446.reg189`, the low byte of a `$6000-$6FFF` write in the mapper-189 mode.
    reg189: u8,
    /// `m446.latch`, the discrete modes' `$8000-$FFFF` latch.
    latch: u8,
    /// `m446.mapper`, the personality `$5000` D0-D4 locked in.
    mode: u8,
    submapper: u8,
    /// The discrete modes' windows.
    banks: BankWindows,
    /// The VRC2 wired as mapper 22 and the VRC4 wired as mapper 25 (A1|A3, A0|A2).
    vrc2: Vrc2Vrc4,
    vrc4b: Vrc2Vrc4,
    vrc6: Vrc6,
    flash: crate::cartridges::jedec_flash::JedecFlash,
}

#[cfg(feature = "board-446")]
impl Mindkids {
    /// `sst39sf040_init(…, 0x01, 0x76, 0x0AAA, 0x0555, 131072)`.
    const CHIP: crate::cartridges::jedec_flash::JedecChip =
        crate::cartridges::jedec_flash::JedecChip {
            ids: (0x01, 0x76),
            unlock: (0x0AAA, 0x0555),
            sector: 0x2_0000,
        };

    fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            reg: [0; 8],
            reg189: 0,
            latch: 0,
            mode: 0,
            submapper: if header.is_nes20 {
                u8::try_from(header.submapper & 0x0F).unwrap_or(0)
            } else {
                0
            },
            banks: super::windows_for(memory),
            vrc2: Vrc2Vrc4::new(memory, 22),
            vrc4b: Vrc2Vrc4::new_vrc4_or_lines(memory, 446, [1, 0], [3, 2]),
            vrc6: Vrc6::new(memory, 24),
            flash: crate::cartridges::jedec_flash::JedecFlash::default(),
        }
    }

    const fn locked(&self) -> bool {
        self.reg[0] & 0x80 != 0
    }

    /// `prg_base()`, `prg_mask()` (8 KiB banks) and `chr_base()` (8 KiB banks).
    fn outer(&self) -> (i32, i32, i32) {
        (
            i32::from(self.reg[1]) | (i32::from(self.reg[2]) << 8),
            i32::from(!self.reg[3]) | 0xFF00,
            i32::from(self.reg[6]),
        )
    }

    /// `fix_all`'s discrete modes and the unlocked menu, into the board's own windows.
    fn fix_discrete(&mut self, memory: &mut CartridgeMemory) {
        let (base, mask, chr) = self.outer();
        let latch = i32::from(self.latch);
        let (locked, mode) = (self.locked(), self.mode);
        let b = &mut self.banks;
        let mut mirroring = if self.reg[4] & 0x01 != 0 {
            crate::NametableMirroring::Vertical
        } else {
            crate::NametableMirroring::Horizontal
        };
        if locked {
            match mode {
                mode::NROM | mode::CNROM => {
                    for slot in 0..4 {
                        b.map_prg_8k(slot, base | (i32::try_from(slot).unwrap_or(0) & mask));
                    }
                    b.map_chr_8k(if mode == mode::CNROM {
                        latch & 0x03
                    } else {
                        chr
                    });
                }
                mode::UNROM => {
                    b.map_prg_16k(0, (base >> 1) | (latch & (mask >> 1)));
                    b.map_prg_16k(2, (base >> 1) | (0x1F & (mask >> 1)));
                    b.map_chr_8k(chr);
                }
                mode::BANDAI => {
                    b.map_prg_16k(0, (base >> 1) | ((latch >> 4) & (mask >> 1)));
                    b.map_prg_16k(2, (base >> 1) | (0xFF & (mask >> 1)));
                    b.map_chr_8k(latch & 0x0F);
                    mirroring = if latch & 0x10 != 0 {
                        crate::NametableMirroring::SingleScreenB
                    } else {
                        crate::NametableMirroring::SingleScreenA
                    };
                }
                mode::ANROM => {
                    b.map_prg_32k((base >> 2) | (latch & (mask >> 2)));
                    b.map_chr_8k(latch & 0x03);
                }
                mode::GNROM => {
                    b.map_prg_32k((base >> 2) | ((latch >> 4) & (mask >> 2)));
                    b.map_chr_8k(latch & 0x03);
                }
                // SUROM, PNROM, HNROM and VRC1 have no `fix_all` arm in puNES: nothing moves.
                _ => return,
            }
        } else {
            // `prg_fix_no_mapper_446`.
            b.map_prg_8k(0, base);
            b.map_prg_8k(1, 0x3D);
            b.map_prg_8k(2, 0x3E);
            b.map_prg_8k(3, 0x3F);
            b.map_chr_8k(chr);
        }
        memory.set_mirroring(mirroring);
    }
}

impl ChipSwitch {
    pub(crate) fn new(memory: &CartridgeMemory, kind: ChipSwitchKind) -> Self {
        let number = kind.number();
        let mut board = Self {
            kind,
            regs: Self::power_on_regs(kind),
            index: 0,
            mmc3: Mmc3::new(memory, number),
            vrc: match kind {
                // `init_VRC2and4(VRC24_VRC4, 0x04, 0x08, …)`: the VRC4e wiring.
                ChipSwitchKind::M351 => Vrc2Vrc4::new_vrc4_or_lines(memory, number, [2, 3], [2, 3]),
                // `init_VRC2and4(VRC24_VRC4, 0x05, 0x0A, …)`: index bit 0 from A0 or A2, bit 1
                // from A1 or A3 (556, and 446's mapper-23 wiring).
                _ => Vrc2Vrc4::new_vrc4_or_lines(memory, number, [0, 1], [2, 3]),
            },
            mmc1: Mmc1::new(memory),
            #[cfg(feature = "board-446")]
            ext_446: (kind == ChipSwitchKind::M446).then(|| Box::new(Mindkids::new(memory))),
        };
        board.apply(None);
        board
    }

    /// `map_init_556`: `reg[2] = $0F`, the CHR mask fully open.
    const fn power_on_regs(kind: ChipSwitchKind) -> [u8; 4] {
        match kind {
            ChipSwitchKind::M556 => [0, 0, 0x0F, 0],
            ChipSwitchKind::M351 | ChipSwitchKind::M446 => [0; 4],
        }
    }

    /// The chip in use: `reg[2]` D7 on 556 (VRC4 or MMC3), `switch_mode` on 351 and 446.
    fn chip(&self) -> Chip {
        match self.kind {
            ChipSwitchKind::M556 if self.regs[2] & 0x80 != 0 => Chip::Vrc4,
            ChipSwitchKind::M556 => Chip::Mmc3,
            ChipSwitchKind::M351 => match self.regs[0] & 0x03 {
                2 => Chip::Mmc1,
                3 => Chip::Vrc4,
                _ => Chip::Mmc3,
            },
            ChipSwitchKind::M446 => self.chip_446(),
        }
    }

    #[allow(clippy::unused_self)]
    fn chip_446(&self) -> Chip {
        #[cfg(feature = "board-446")]
        if let Some(ext) = &self.ext_446 {
            if ext.locked() {
                return match ext.mode {
                    mode::SLROM | mode::SNROM => Chip::Mmc1,
                    mode::MMC3 | mode::TLSROM | mode::M189 => Chip::Mmc3,
                    mode::VRC2_22 => Chip::Vrc2,
                    mode::VRC4_23 => Chip::Vrc4,
                    mode::VRC4_25 => Chip::Vrc4b,
                    mode::VRC6 => Chip::Vrc6,
                    _ => Chip::Discrete,
                };
            }
        }
        Chip::Discrete
    }

    fn vrc_active(&self) -> bool {
        matches!(self.chip(), Chip::Vrc4)
    }

    /// The outer banks and masks the chip's windows go through, in 8 KiB and 1 KiB units:
    /// `prg_base`, `prg_mask`, `chr_base`, `chr_mask` of each file (351's MMC1 halves them to
    /// its 16 KiB and 4 KiB banks, which in these units keep the low bits open).
    fn outer(&self) -> (i32, i32, i32, i32) {
        let r = self.regs.map(i32::from);
        match self.kind {
            ChipSwitchKind::M556 => (
                ((r[3] & 0x40) << 2) | r[1],
                !r[3] & 0x3F,
                ((r[3] & 0x40) << 6) | ((r[2] & 0xF0) << 4) | r[0],
                0xFF >> (!r[2] & 0x0F),
            ),
            ChipSwitchKind::M351 => {
                let (prg_base, prg_mask) = (r[1] >> 1, 0x1F >> ((r[2] & 0x04) >> 2));
                let chr_base = r[0] << 1;
                let chr_mask = if r[2] & 0x10 != 0 {
                    0x1F
                } else if r[2] & 0x20 != 0 {
                    0x7F
                } else {
                    0xFF
                };
                if self.chip() == Chip::Mmc1 {
                    (
                        ((prg_base >> 1) & !(prg_mask >> 1)) << 1,
                        ((prg_mask >> 1) << 1) | 0x01,
                        ((chr_base >> 2) & !(chr_mask >> 2)) << 2,
                        ((chr_mask >> 2) << 2) | 0x03,
                    )
                } else {
                    (prg_base, prg_mask, chr_base, chr_mask)
                }
            }
            // 446 places its chips in `apply_446`.
            ChipSwitchKind::M446 => (0, -1, 0, -1),
        }
    }

    /// The chip in use rebuilds its windows, then the board narrows them. The MMC1 needs the
    /// memory to rebuild (its mirroring and size); without it, as at power-on, its windows are
    /// only narrowed.
    fn apply(&mut self, memory: Option<&mut CartridgeMemory>) {
        if self.kind == ChipSwitchKind::M446 {
            self.apply_446(memory);
            return;
        }
        let (prg_base, prg_mask, chr_base, chr_mask) = self.outer();
        let r = self.regs.map(i32::from);
        let banks = match self.chip() {
            Chip::Vrc4 => {
                self.vrc.refresh_banks();
                self.vrc.banks_mut()
            }
            Chip::Mmc3 => {
                self.mmc3.refresh_banks();
                self.mmc3.banks_mut()
            }
            Chip::Mmc1 => {
                if let Some(memory) = memory {
                    self.mmc1.refresh_banks(memory);
                }
                self.mmc1.banks_mut()
            }
            _ => None,
        };
        let Some(b) = banks else {
            return;
        };
        b.remap_prg_banks(prg_mask, prg_base & !prg_mask);
        b.remap_chr_banks(chr_mask, chr_base & !chr_mask);
        if self.kind == ChipSwitchKind::M351 {
            // `prg_fix`: `$5002` D4 the NROM modes, D2 NROM-128.
            if r[2] & 0x10 != 0 {
                if r[2] & 0x04 != 0 {
                    b.map_prg_16k(0, r[1] >> 2);
                    b.map_prg_16k(2, r[1] >> 2);
                } else {
                    b.map_prg_32k(r[1] >> 3);
                }
            }
            // `chr_fix`: D0 the CHR-RAM (here the image's own CHR, J37), D6 an 8 KiB CHR bank.
            if r[2] & 0x01 != 0 {
                b.map_chr_8k(0);
            } else if r[2] & 0x40 != 0 {
                b.map_chr_8k(r[0] >> 2);
            }
        }
    }

    /// `fix_all` of `mapper_446.c`: the discrete modes into the board's own windows, the chips'
    /// windows through `prg_swap_*_446`/`chr_swap_*_446` (`base | (value & mask)`).
    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    fn apply_446(&mut self, memory: Option<&mut CartridgeMemory>) {
        #[cfg(feature = "board-446")]
        {
            let chip = self.chip();
            let Some(ext) = self.ext_446.as_deref_mut() else {
                return;
            };
            let (base, mask, chr) = ext.outer();
            let b = match chip {
                Chip::Discrete => {
                    if let Some(memory) = memory {
                        ext.fix_discrete(memory);
                    }
                    return;
                }
                Chip::Mmc1 => {
                    if let Some(memory) = memory {
                        self.mmc1.refresh_banks(memory);
                    }
                    let Some(b) = self.mmc1.banks_mut() else {
                        return;
                    };
                    // `prg_swap_mmc1_446`/`chr_swap_mmc1_446`: 16 KiB and 4 KiB units.
                    b.remap_prg_banks(((mask >> 1) << 1) | 0x01, (base >> 1) << 1);
                    b.remap_chr_banks(0x7F, (chr >> 2) << 2);
                    return;
                }
                Chip::Mmc3 => {
                    self.mmc3.refresh_banks();
                    let Some(b) = self.mmc3.banks_mut() else {
                        return;
                    };
                    if ext.mode == mode::M189 {
                        // `prg_swap_mmc3_m189_446`: a 32 KiB bank from `reg189` D0-D1.
                        let block = (base & !3) | ((i32::from(ext.reg189) & 0x03) << 2);
                        for slot in 0..4 {
                            b.map_prg_8k(slot, block | i32::try_from(slot).unwrap_or(0));
                        }
                    } else {
                        b.remap_prg_banks(mask, base);
                    }
                    let chr_mask = if ext.mode == mode::TLSROM { 0x7F } else { 0xFF };
                    b.remap_chr_banks(chr_mask, chr);
                    return;
                }
                Chip::Vrc4 => {
                    self.vrc.refresh_banks();
                    self.vrc.banks_mut()
                }
                Chip::Vrc2 => {
                    ext.vrc2.refresh_banks();
                    ext.vrc2.banks_mut()
                }
                Chip::Vrc4b => {
                    ext.vrc4b.refresh_banks();
                    ext.vrc4b.banks_mut()
                }
                Chip::Vrc6 => ext.vrc6.banks_mut(),
            };
            // `prg_swap_vrc2and4_*_446`/`prg_swap_vrc6_446`: the PRG through the outer bank, the
            // CHR the chip's own.
            if let Some(b) = b {
                b.remap_prg_banks(mask, base);
            }
        }
    }

    /// `extcl_cpu_wr_mem_556`: the four registers in turn, unless `reg[3]` D7 locks them;
    /// `extcl_cpu_wr_mem_351`: `$5000`-`$5002` by A0-A1.
    fn write_low(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        if self.kind == ChipSwitchKind::M446 {
            return self.write_low_446(memory, address, value);
        }
        if !(0x5000..=0x5FFF).contains(&address) {
            return false;
        }
        match self.kind {
            ChipSwitchKind::M556 => {
                if self.regs[3] & 0x80 == 0 {
                    self.regs[usize::from(self.index)] = value;
                    self.index = (self.index + 1) & 0x03;
                }
            }
            ChipSwitchKind::M351 => {
                let register = usize::from(address & 0x03);
                if register < 3 {
                    self.regs[register] = value;
                }
            }
            ChipSwitchKind::M446 => {}
        }
        self.apply(Some(memory));
        true
    }

    /// `extcl_cpu_wr_mem_446` below `$8000`: `$5000-$5FFF` the eight registers while unlocked
    /// (submapper 0 turns a `$5000` mode of 1 into the SNROM), `$6000-$6FFF` the mapper-189
    /// register; `switch_mode` re-initialises the chip `$5000` D0-D4 names once D7 locks it.
    #[allow(clippy::unused_self)]
    fn write_low_446(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        #[cfg(feature = "board-446")]
        {
            let Some(ext) = self.ext_446.as_deref_mut() else {
                return false;
            };
            match address & 0xF000 {
                0x5000 => {
                    if !ext.locked() {
                        let index = usize::from(address & 0x07);
                        let value = if index == 0 && ext.submapper == 0 && value & 0x1F == 0x01 {
                            (value & !0x1F) | mode::SNROM
                        } else {
                            value
                        };
                        ext.reg[index] = value;
                        if ext.locked() {
                            ext.mode = ext.reg[0] & 0x1F;
                            self.switch_446(memory);
                        }
                        self.apply(Some(memory));
                    }
                    true
                }
                0x6000 => {
                    if ext.locked() && ext.mode == mode::M189 {
                        ext.reg189 = address.to_le_bytes()[0];
                        self.apply(Some(memory));
                        return true;
                    }
                    false
                }
                _ => false,
            }
        }
        #[cfg(not(feature = "board-446"))]
        {
            let _ = (memory, address, value);
            false
        }
    }

    /// `switch_mode`: the chosen chip starts over (`init_*(…, HARD)`), the discrete latch at 0.
    #[cfg(feature = "board-446")]
    fn switch_446(&mut self, memory: &mut CartridgeMemory) {
        let chip = self.chip();
        let Some(ext) = self.ext_446.as_deref_mut() else {
            return;
        };
        match chip {
            Chip::Discrete => ext.latch = 0,
            Chip::Mmc1 => self.mmc1.reset(memory),
            Chip::Mmc3 => self.mmc3.reset(memory),
            Chip::Vrc2 => ext.vrc2.reset(memory),
            Chip::Vrc4 => self.vrc.reset(memory),
            Chip::Vrc4b => ext.vrc4b.reset(memory),
            Chip::Vrc6 => ext.vrc6.reset(memory),
        }
    }

    /// `extcl_cpu_wr_mem_446` at `$8000-$FFFF`: locked, the chip in use or the discrete latch;
    /// unlocked, the flash chip.
    #[allow(clippy::unused_self)]
    fn write_high_446(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        #[cfg(feature = "board-446")]
        {
            let chip = self.chip();
            let Some(ext) = self.ext_446.as_deref_mut() else {
                return;
            };
            if !ext.locked() {
                if let Some(offset) = ext.banks.prg_rom_offset(address) {
                    let size = memory.rom().prg_rom().len();
                    ext.flash
                        .write(memory, &Mindkids::CHIP, offset, value, size);
                }
                return;
            }
            match chip {
                Chip::Discrete => ext.latch = value,
                Chip::Mmc1 => self.mmc1.write_register(memory, address, value),
                Chip::Mmc3 => self.mmc3.write_register(memory, address, value),
                Chip::Vrc4 => self.vrc.write_register(memory, address, value),
                Chip::Vrc2 => ext.vrc2.write_register(memory, address, value),
                Chip::Vrc4b => ext.vrc4b.write_register(memory, address, value),
                Chip::Vrc6 => ext.vrc6.write_register(memory, address, value),
            }
            self.apply(Some(memory));
        }
        #[cfg(not(feature = "board-446"))]
        {
            let _ = (memory, address, value);
        }
    }

    /// 446's flash chip, when it answers reads itself.
    #[allow(clippy::unused_self)]
    fn flash_446(&self) -> Option<&crate::cartridges::jedec_flash::JedecFlash> {
        #[cfg(feature = "board-446")]
        if let Some(ext) = &self.ext_446 {
            return ext.flash.answers().then_some(&ext.flash);
        }
        None
    }
}

impl Mapper for ChipSwitch {
    fn banks(&self) -> Option<&BankWindows> {
        match self.chip() {
            Chip::Vrc4 => self.vrc.banks(),
            Chip::Mmc3 => self.mmc3.banks(),
            Chip::Mmc1 => self.mmc1.banks(),
            #[cfg(feature = "board-446")]
            chip => self.ext_446.as_deref().and_then(|ext| match chip {
                Chip::Vrc2 => ext.vrc2.banks(),
                Chip::Vrc4b => ext.vrc4b.banks(),
                Chip::Vrc6 => ext.vrc6.banks(),
                _ => Some(&ext.banks),
            }),
            #[cfg(not(feature = "board-446"))]
            _ => None,
        }
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        match self.chip() {
            Chip::Vrc4 => self.vrc.banks_mut(),
            Chip::Mmc3 => self.mmc3.banks_mut(),
            Chip::Mmc1 => self.mmc1.banks_mut(),
            #[cfg(feature = "board-446")]
            chip => self.ext_446.as_deref_mut().and_then(|ext| match chip {
                Chip::Vrc2 => ext.vrc2.banks_mut(),
                Chip::Vrc4b => ext.vrc4b.banks_mut(),
                Chip::Vrc6 => ext.vrc6.banks_mut(),
                _ => Some(&mut ext.banks),
            }),
            #[cfg(not(feature = "board-446"))]
            _ => None,
        }
    }

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
        writer.write_u8(self.index);
        self.mmc3.save_registers(writer);
        self.vrc.save_registers(writer);
        if self.kind != ChipSwitchKind::M556 {
            self.mmc1.save_registers(writer);
        }
        #[cfg(feature = "board-446")]
        if let Some(ext) = &self.ext_446 {
            writer.write_raw(&ext.reg);
            writer.write_u8(ext.reg189);
            writer.write_u8(ext.latch);
            writer.write_u8(ext.mode);
            ext.vrc2.save_registers(writer);
            ext.vrc4b.save_registers(writer);
            ext.vrc6.save_registers(writer);
            ext.flash.save_state(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.index = reader.read_u8()? & 0x03;
        self.mmc3.load_registers(reader)?;
        self.vrc.load_registers(reader)?;
        if self.kind != ChipSwitchKind::M556 {
            self.mmc1.load_registers(reader)?;
        }
        #[cfg(feature = "board-446")]
        if let Some(ext) = self.ext_446.as_deref_mut() {
            reader.read_into(&mut ext.reg)?;
            ext.reg189 = reader.read_u8()?;
            ext.latch = reader.read_u8()?;
            ext.mode = reader.read_u8()?;
            ext.vrc2.load_registers(reader)?;
            ext.vrc4b.load_registers(reader)?;
            ext.vrc6.load_registers(reader)?;
            ext.flash.load_state(reader)?;
        }
        // The windows themselves come back with the block `Mapper::save_state` wrote first.
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = Self::power_on_regs(self.kind);
        self.index = 0;
        self.mmc3.reset(memory);
        self.vrc.reset(memory);
        self.mmc1.reset(memory);
        #[cfg(feature = "board-446")]
        if self.ext_446.is_some() {
            self.ext_446 = Some(Box::new(Mindkids::new(memory)));
        }
        self.apply(Some(memory));
    }

    fn irq_asserted(&self) -> bool {
        match self.chip() {
            Chip::Vrc4 => self.vrc.irq_asserted(),
            Chip::Mmc3 => self.mmc3.irq_asserted(),
            #[cfg(feature = "board-446")]
            Chip::Vrc2 | Chip::Vrc4b | Chip::Vrc6 => {
                self.ext_446
                    .as_deref()
                    .is_some_and(|ext| match self.chip() {
                        Chip::Vrc2 => ext.vrc2.irq_asserted(),
                        Chip::Vrc4b => ext.vrc4b.irq_asserted(),
                        _ => ext.vrc6.irq_asserted(),
                    })
            }
            _ => false,
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        match self.chip() {
            Chip::Vrc4 | Chip::Vrc2 | Chip::Vrc4b => true,
            Chip::Mmc3 => self.mmc3.prg_ram_enabled(),
            Chip::Mmc1 => self.mmc1.prg_ram_enabled(),
            #[cfg(feature = "board-446")]
            Chip::Vrc6 => self
                .ext_446
                .as_deref()
                .is_some_and(|ext| ext.vrc6.prg_ram_enabled()),
            // `wram_fix_*_446`: the discrete modes disable the work RAM.
            _ => false,
        }
    }

    fn prg_ram_writable(&self) -> bool {
        match self.chip() {
            Chip::Mmc3 => self.mmc3.prg_ram_writable(),
            Chip::Mmc1 => self.mmc1.prg_ram_writable(),
            _ => self.prg_ram_enabled(),
        }
    }

    /// `extcl_cpu_rd_mem_351`: `$5000-$5FFF` reads `(open bus & $F8) | DIP`; the DIP is 0 and the
    /// high address byte stands in for the bus (FIDELITY-LEADS J33). `extcl_cpu_rd_mem_446`:
    /// `$8000-$FFFF` through the flash chip, which answers itself while busy or in its ID mode.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if self.kind == ChipSwitchKind::M351 && (0x5000..=0x5FFF).contains(&address) {
            return Some(address.to_be_bytes()[0] & 0xF8);
        }
        let byte = self
            .banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()));
        #[cfg(feature = "board-446")]
        if address >= 0x8000 {
            if let Some(flash) = self.flash_446() {
                return flash.read(&Mindkids::CHIP, address, byte);
            }
        }
        byte
    }

    /// 446's flash takes the ROM windows off the cached path while it answers.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.flash_446().is_some() {
            return [super::NO_ROM_WINDOW; 4];
        }
        self.banks().map_or([super::NO_ROM_WINDOW; 4], |banks| {
            banks.rom_windows(memory.rom().prg_rom().len())
        })
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if self.kind == ChipSwitchKind::M446 {
            return address < 0x8000 && self.write_low(memory, address, value);
        }
        address < 0x6000 && self.write_low(memory, address, value)
    }

    /// `extcl_cpu_wr_mem_556`: the chip in use; the VRC4 sees A2 and A3 swapped above A11.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        if self.kind == ChipSwitchKind::M446 {
            self.write_high_446(memory, address, value);
            return;
        }
        match self.chip() {
            Chip::Vrc4 => {
                let address = if address & 0x0800 != 0 {
                    (address & 0xFFF3) | ((address & 0x0004) << 1) | ((address & 0x0008) >> 1)
                } else {
                    address
                };
                self.vrc.write_register(memory, address, value);
            }
            Chip::Mmc3 => self.mmc3.write_register(memory, address, value),
            Chip::Mmc1 => self.mmc1.write_register(memory, address, value),
            _ => {}
        }
        self.apply(Some(memory));
    }

    /// 446 write-protects its CHR-RAM with `$5005` D2 (`chr_disable_write`).
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        #[cfg(feature = "board-446")]
        if let Some(ext) = &self.ext_446 {
            if ext.reg[5] & 0x04 != 0 {
                return;
            }
        }
        if memory.rom().chr_rom().is_empty() {
            if let Some(banks) = self.banks() {
                banks.write_chr(address, value, memory.chr_ram_mut());
            }
        }
    }

    /// `extcl_cpu_every_cycle_556`/`_446`: the VRC counters while in use; 446's flash countdown.
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        if self.vrc_active() {
            self.vrc.on_cpu_cycle(memory);
        }
        #[cfg(feature = "board-446")]
        {
            let chip = self.chip();
            if let Some(ext) = self.ext_446.as_deref_mut() {
                ext.flash.tick();
                match chip {
                    Chip::Vrc2 => ext.vrc2.on_cpu_cycle(memory),
                    Chip::Vrc4b => ext.vrc4b.on_cpu_cycle(memory),
                    Chip::Vrc6 => ext.vrc6.on_cpu_cycle(memory),
                    _ => {}
                }
            }
        }
    }

    /// `extcl_ppu_*_556`: the MMC3's A12 counter while it is in use.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.chip() == Chip::Mmc3 {
            self.mmc3.on_ppu_address_bus(memory, address, clock);
        }
    }

    /// 446's VRC6 arranges the nametables itself.
    fn nametable_page(&self, table: i32) -> i32 {
        #[cfg(feature = "board-446")]
        if self.chip() == Chip::Vrc6 {
            if let Some(ext) = &self.ext_446 {
                return ext.vrc6.nametable_page(table);
            }
        }
        if self.chip() == Chip::Mmc3 {
            return self.mmc3.nametable_page(table);
        }
        let _ = table;
        super::super::Cartridge::FOLLOW_MIRRORING
    }

    /// 446's VRC6 sound, 0 while another chip runs (the board is wired for it either way).
    fn expansion_audio_output(&self) -> Option<i32> {
        #[cfg(feature = "board-446")]
        if let Some(ext) = &self.ext_446 {
            return Some(if self.chip() == Chip::Vrc6 {
                ext.vrc6.expansion_audio_output().unwrap_or(0)
            } else {
                0
            });
        }
        None
    }
}
