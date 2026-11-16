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
    /// 395, `mapper_395.c` (Realtec 8210): two outer registers at `$6000`/`$6010`, a lock.
    M395,
    /// 399, `mapper_399.c` (BATMAP-000): the MMC3's IRQ and mirroring with its own PRG and CHR
    /// banks, written at `$8000-$9FFF` by A0 and D7.
    M399,
    /// 401, `mapper_401.c` (KC885): mapper 45's four registers with DIP-selected lines.
    M401,
    /// 410, `mapper_410.c` (JC-016-2 variant): mapper 45's four registers, a CHR-RAM mode.
    M410,
    /// 411, `mapper_411.c` (A88S-1): two registers at `$5000`/`$5001`, NROM modes.
    M411,
    /// 420, `mapper_420.c` (A971210): four registers at `$6000-$6003`, an NROM mode.
    M420,
    /// 455, `mapper_455.c` (N625836): `$4100` the address and data are the outer registers, an
    /// NROM mode.
    M455,
    /// 456, `mapper_456.c` (K6C3001A): `$4100` the outer 128 KiB bank.
    M456,
    /// 457, `mapper_457.c` (810431C): `$6000` the outer bank, D3 doubling the inner one.
    M457,
    /// 516, `mapper_516.c` (Brilliant Com Cocoma Pack): a chip write with A4 set latches A0-A3.
    M516,
    /// 393, `mapper_393.c` (820720C): the address at `$6000` the outer bank, two NROM-like
    /// modes, a CHR-RAM mode; every chip write is also latched.
    M393,
    /// 412, `mapper_412.c` (Intellivision 10-in-1 PVP): `$5000-$7FFF` four registers until a lock,
    /// an NROM and CNROM-like mode.
    M412,
    /// 422, `mapper_126.c` (BS-300/BS-400/BS-4040 multicarts): four registers at `$6000-$6003`
    /// with a lock, NROM modes and a CNROM-like CHR mode; puNES's own reading of the TEC9719
    /// family, with 422's CHR outer bank from `$6000` D3-D5.
    M422,
    /// 534, `mapper_126.c` (ING003C): 422's board whose `$C000`/`$C001` data is inverted.
    M534,
    /// 370, `mapper_370.c` (F600): the address at `$5000-$5FFF` the outer bank; mode 1 takes a
    /// CIRAM page per table from the CHR registers' D7, as the `TxSROM` does.
    M370,
    /// 344, `mapper_344.c` (GN-26): the address at `$6000` the outer bank, an NROM-256 mode from
    /// R6, a DIP read over `$8000-$FFFF`.
    M344,
    /// 432, `mapper_432.c` (Realtec 8090): `$6000`/`$6001` (even/odd) the solder pad and the outer
    /// bank, NROM modes, a solder-pad read over `$8000-$FFFF`; nesdev "NES 2.0 Mapper 432".
    M432,
    /// 356, `mapper_356.c` (JY-215 / 7-in-1 *Rockman*): mapper 410's four registers, CHR-RAM
    /// while `reg[2]` D5 is clear, four-screen mirroring from `reg[2]` D6.
    M356,
    /// 269, `mapper_269.c` (Games Xplosion 121-in-1): four registers written in turn at
    /// `$5000-$5FFF` (CHR offset, PRG offset, CHR mask, PRG mask and lock); an image without
    /// CHR-ROM reads its CHR from the PRG-ROM with the data lines permuted.
    M269,
    /// 353, `mapper_353.c` (81-03-05-C, *Super Mario Family*): a chip write to `$x080` selects one
    /// of four 256 KiB games from A13-A14; game 0 pages CIRAM like the `TxSROM`, game 2 has a
    /// CHR-RAM mode and game 3 its own `$C000`/`$E000` banks.
    M353,
    /// 100, `mapper_100.c` (Nesticle's "buggy MMC3" images): the chip's IRQ and mirroring, but
    /// the bank select is a whole byte whose values `$00-$07`, `$46`/`$47` and `$80-$85` load
    /// eight 1 KiB CHR and four 8 KiB PRG registers directly, with no PRG/CHR mode.
    M100,
    /// 555, `mapper_555.c` (*Nintendo Campus Challenge 1991*): two registers at `$5800`/`$5C00`
    /// (an outer PRG/CHR bank and masks), work RAM at `$5000-$57FF`, a competition timer whose
    /// expiry `$5800-$5FFF` reads in D7, and a mode in which CHR register D6 picks the 8 KiB
    /// CHR-RAM beside the CHR-ROM.
    M555,
    /// 369, `mapper_369.c` (N-49, *Super Mario Bros. Party*): `$4100` picks the game: NROM
    /// banks (`$00`/`$01`), the *SMB2J* conversion with a 12-bit CPU-cycle IRQ (`$13`), or the
    /// MMC3 at two outer banks (`$37`, `$FF`).
    M369,
    /// 512, `mapper_512.c` (*Zhōngguó Dàhēng*, SL-12): `$4100` D0-D1: D1 maps every CHR window to
    /// the 8 KiB CHR-RAM beside the CHR-ROM (bank D0-D1 of the register), and the value 1 makes
    /// the nametables four-screen in the CHR-RAM's second 4 KiB.
    M512,
    /// 406, `mapper_406.c` (*Haradius Zero*, *Sidewinder*): an MMC3 whose PRG-ROM is a flash chip
    /// (Macronix MX29F040 on submapper 0, AMD AM29F040 on 1), no WRAM, the chip's address lines
    /// crossed (submapper 0 swaps A0 and A1, submapper 1 flips A13-A14 of `$8000-$9000` and
    /// `$E000-$FFFF`).
    M406,
    /// 451, `mapper_451.c` (*Haratyler HP/MP*): an AMIC A29040B flash PRG-ROM; the MMC3 is only
    /// mirroring and IRQ, fed from the address lines, and `$E000` A0-A1 the banks.
    M451,
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
            395 => Self::M395,
            399 => Self::M399,
            401 => Self::M401,
            410 => Self::M410,
            411 => Self::M411,
            420 => Self::M420,
            455 => Self::M455,
            456 => Self::M456,
            457 => Self::M457,
            516 => Self::M516,
            393 => Self::M393,
            412 => Self::M412,
            422 => Self::M422,
            534 => Self::M534,
            370 => Self::M370,
            344 => Self::M344,
            432 => Self::M432,
            356 => Self::M356,
            269 => Self::M269,
            353 => Self::M353,
            100 => Self::M100,
            555 => Self::M555,
            369 => Self::M369,
            512 => Self::M512,
            406 => Self::M406,
            451 => Self::M451,
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
            Self::M395 => 395,
            Self::M399 => 399,
            Self::M401 => 401,
            Self::M410 => 410,
            Self::M411 => 411,
            Self::M420 => 420,
            Self::M455 => 455,
            Self::M456 => 456,
            Self::M457 => 457,
            Self::M516 => 516,
            Self::M393 => 393,
            Self::M412 => 412,
            Self::M422 => 422,
            Self::M534 => 534,
            Self::M370 => 370,
            Self::M344 => 344,
            Self::M432 => 432,
            Self::M356 => 356,
            Self::M269 => 269,
            Self::M353 => 353,
            Self::M100 => 100,
            Self::M555 => 555,
            Self::M369 => 369,
            Self::M512 => 512,
            Self::M406 => 406,
            Self::M451 => 451,
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
    /// The NES 2.0 submapper (0 on iNES 1.0), which 422's PRG A20 doubling reads.
    submapper: u8,
    /// The PRG-ROM size in bytes, which 432's solder-pad decode reads (`less1024`).
    prg_size: usize,
    /// Mapper 269's CHR when the image has no CHR-ROM: the PRG-ROM with its data lines permuted
    /// (`map_init_269`), empty otherwise. Held here because the core clears the CHR-RAM after a
    /// board's power-on (`Cartridge::clear_volatile_ram`); derived, so not in the save state.
    chr_269: Vec<u8>,
    /// The last value written to the chip's `$A000`, which 356 returns to when it leaves its
    /// four-screen mode (the chip keeps no copy).
    a000: u8,
    /// Mapper 100's bank select, four PRG and eight CHR registers (`m100.reg`, `.prg`,
    /// `.chr`), saved for that kind only.
    raw_100: [u8; 13],
    /// Mapper 555's timer (`m555.counter.timer`) and its expiry (`.disabled`), saved for that
    /// kind only.
    timer_555: u32,
    expired_555: bool,
    /// Mapper 555's windows on the CHR-RAM chip (bit per 1 KiB window) and the RAM bank of each;
    /// derived by [`Mmc3P::refresh`], so not saved.
    ram_windows: u8,
    ram_banks: [u8; 8],
    /// 406's and 451's flash command sequence and busy countdown (`sst39sf040.sequence`,
    /// `.time_out`), saved for those kinds only.
    #[cfg(any(feature = "board-406", feature = "board-451"))]
    flash: crate::cartridges::jedec_flash::JedecFlash,
}

impl Mmc3P {
    pub(crate) fn new(memory: &CartridgeMemory, kind: Mmc3PKind) -> Self {
        Self {
            mmc3: Mmc3::new(memory, kind.number()),
            kind,
            outer: [0; 4],
            index: 0,
            chr_ram: memory.rom().chr_rom().is_empty(),
            prg_size: memory.rom().prg_rom().len(),
            a000: 0,
            raw_100: Self::RAW_100_POWER_ON,
            timer_555: 0,
            expired_555: false,
            ram_windows: 0,
            ram_banks: [0; 8],
            #[cfg(any(feature = "board-406", feature = "board-451"))]
            flash: crate::cartridges::jedec_flash::JedecFlash::default(),
            chr_269: if kind == Mmc3PKind::M269 {
                Self::chr_269(memory)
            } else {
                Vec::new()
            },
            submapper: {
                let header = memory.rom().header();
                if header.is_nes20 {
                    u8::try_from(header.submapper).unwrap_or(0)
                } else {
                    0
                }
            },
        }
    }

    /// Whether the flash chip answers reads itself (software ID or an erase running), which takes
    /// the ROM windows off the cached path.
    #[allow(clippy::unused_self)]
    const fn flash_answers(&self) -> bool {
        #[cfg(any(feature = "board-406", feature = "board-451"))]
        if matches!(self.kind, Mmc3PKind::M406 | Mmc3PKind::M451) {
            return self.flash.answers();
        }
        false
    }

    /// The chip of `map_init_406`/`_451`: IDs, unlock addresses, 64 KiB sectors.
    #[cfg(any(feature = "board-406", feature = "board-451"))]
    const fn flash_chip(&self) -> crate::cartridges::jedec_flash::JedecChip {
        let (ids, unlock) = match self.kind {
            Mmc3PKind::M451 => ((0x37, 0x86), (0x0555, 0x02AA)),
            _ if self.submapper == 0 => ((0xC2, 0xA4), (0x5555, 0x2AAA)),
            _ => ((0x01, 0xA4), (0x5555, 0x2AAA)),
        };
        crate::cartridges::jedec_flash::JedecChip {
            ids,
            unlock,
            sector: 0x1_0000,
        }
    }

    /// `sst39sf040_write` at the chip address (the PRG-ROM offset) the CPU address shows.
    #[cfg(any(feature = "board-406", feature = "board-451"))]
    fn write_flash(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let Some(offset) = self
            .mmc3
            .banks()
            .and_then(|banks| banks.prg_rom_offset(address))
        else {
            return;
        };
        let chip = self.flash_chip();
        self.flash
            .write(memory, &chip, offset, value, self.prg_size);
    }

    /// `extcl_cpu_wr_mem_451`: the MMC3 fed from the address lines.
    fn write_451(&mut self, memory: &mut CartridgeMemory, address: u16) {
        match address & 0xE000 {
            0xA000 => self
                .mmc3
                .write_register(memory, 0xA000, u8::from(address & 0x01 != 0)),
            0xC000 => {
                let low = address.to_le_bytes()[0];
                self.mmc3
                    .write_register(memory, 0xC000, low.wrapping_sub(1));
                self.mmc3.write_register(memory, 0xC001, 0);
                let ack = if low == 0xFF { 0xE000 } else { 0xE001 };
                self.mmc3.write_register(memory, ack, 0);
            }
            0xE000 => self.outer[0] = address & 0x03,
            _ => {}
        }
    }

    /// `prg_fix_mmc3_369`: the NROM games and the *SMB2J* layout replace the chip's windows.
    fn prg_369(&self, chip: [i32; BankWindows::PRG_WINDOWS]) -> [i32; BankWindows::PRG_WINDOWS] {
        match self.outer[0] {
            game @ (0x00 | 0x01) => {
                let base = i32::from(game) << 2;
                [base, base + 1, base + 2, base + 3]
            }
            0x13 => [0x0C, 0x0D, 0x08 | (i32::from(self.outer[1]) & 0x03), 0x0F],
            _ => chip,
        }
    }

    /// `chr_fix_mmc3_369`/`chr_swap_mmc3_369`: an 8 KiB bank from the mode's low bits in the
    /// NROM and *SMB2J* modes, the chip's banks at `$80` (`$37`) or `$100` otherwise.
    fn chr_369(&self, raw: &[i32; BankWindows::CHR_WINDOWS]) -> [i32; BankWindows::CHR_WINDOWS] {
        match self.outer[0] {
            game @ (0x00 | 0x01 | 0x13) => {
                let base = (i32::from(game) & 0x03) << 3;
                std::array::from_fn(|slot| base | i32::try_from(slot).unwrap_or(0))
            }
            0x37 => std::array::from_fn(|slot| 0x80 | (raw[slot] & 0x7F)),
            _ => std::array::from_fn(|slot| 0x100 | (raw[slot] & 0x7F)),
        }
    }

    /// 369's *SMB2J* mode, in which `$6000` is PRG-ROM and the IRQ counts CPU cycles.
    const fn smb2j_369(&self) -> bool {
        matches!(self.kind, Mmc3PKind::M369) && self.outer[0] == 0x13
    }

    /// `chr_swap_mmc3_555`: in the mode `reg[0] & 6 == 2`, a CHR register with D6 set puts its
    /// window on the CHR-RAM, bank D0-D2 (`memmap_vram_1k`, which wraps on the chip's size).
    fn settle_ram_555(&mut self, raw_chr: &[i32; BankWindows::CHR_WINDOWS]) {
        self.ram_windows = 0;
        if self.outer[0] & 0x06 != 0x02 {
            return;
        }
        for (slot, value) in raw_chr.iter().enumerate() {
            if value & 0x40 != 0 {
                self.ram_windows |= 1 << slot;
                self.ram_banks[slot] = u8::try_from(value & 0x07).unwrap_or(0);
            }
        }
    }

    /// Where a CHR address on one of 555's CHR-RAM windows falls in the chip, or `None`.
    fn ram_offset_555(&self, address: u16) -> Option<usize> {
        let window = usize::from(address >> 10) & 0x07;
        (address < 0x2000 && self.ram_windows & (1 << window) != 0)
            .then(|| usize::from(self.ram_banks[window]) * 0x400 + usize::from(address & 0x03FF))
    }

    /// `map_init_100`: PRG 0, 1, `$FE`, `$FF`, CHR 0-7.
    const RAW_100_POWER_ON: [u8; 13] = [0, 0, 1, 0xFE, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7];

    /// `extcl_cpu_wr_mem_100`: `$8000` the select byte, `$8001` the register it names.
    fn write_100(&mut self, address: u16, value: u8) {
        let r = &mut self.raw_100;
        match address & 0xE001 {
            0x8000 => r[0] = value,
            0x8001 => match r[0] {
                0x00 => [r[5], r[6]] = [value & 0xFE, value | 0x01],
                0x01 => [r[7], r[8]] = [value & 0xFE, value | 0x01],
                0x02..=0x05 => r[usize::from(r[0]) + 7] = value,
                0x06 | 0x07 => r[usize::from(r[0]) - 5] = value,
                0x46 => r[3] = value,
                // puNES's `0x47` loads `prg[1]`, as FCEUX's mapper 100 does.
                0x47 => r[2] = value,
                0x80 => [r[9], r[10]] = [value & 0xFE, value | 0x01],
                0x81 => [r[11], r[12]] = [value & 0xFE, value | 0x01],
                0x82..=0x85 => r[usize::from(r[0]) - 0x82 + 5] = value,
                _ => {}
            },
            _ => {}
        }
    }

    fn power_on(&mut self) {
        self.outer = [0; 4];
        self.index = 0;
        self.raw_100 = Self::RAW_100_POWER_ON;
        self.timer_555 = 0;
        self.expired_555 = false;
        #[cfg(any(feature = "board-406", feature = "board-451"))]
        {
            self.flash = crate::cartridges::jedec_flash::JedecFlash::default();
        }
        match self.kind {
            // `map_init_372`/`_401`/`_410`/`_420`: "reg[2] = 0x0F", the CHR mask fully open.
            Mmc3PKind::M356
            | Mmc3PKind::M372
            | Mmc3PKind::M401
            | Mmc3PKind::M410
            | Mmc3PKind::M420 => {
                self.outer[2] = 0x0F;
            }
            // `map_init_269`: CHR offset 0, PRG offset 0, CHR mask `$FF`, PRG mask `$3F` (the
            // fourth register keeps its last value in its high byte, for the lock).
            Mmc3PKind::M269 => self.outer = [0, 0, 0xFF, 0x3F],
            // `map_init_399`: banks 0, 1 for PRG and CHR.
            Mmc3PKind::M399 => self.outer = [0, 1, 0, 1],
            // `map_init_411`: `$80`, `$82`.
            Mmc3PKind::M411 => self.outer = [0x80, 0x82, 0, 0],
            // `map_init_455`: `reg[1] = 1`, the NROM mode.
            Mmc3PKind::M455 => self.outer[1] = 1,
            _ => {}
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
            _ => self.prg_page_b2(slot, page),
        }
    }

    /// [`Mmc3P::prg_page`] for group 2.
    fn prg_page_b2(&self, slot: usize, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        let window = i32::try_from(slot).unwrap_or(0);
        match self.kind {
            Mmc3PKind::M395 => {
                let base = ((o[0] & 0x08) << 4) | ((o[0] & 0x30) << 1) | ((o[1] & 0x01) << 4);
                base | (page & (0x1F >> ((o[1] & 0x08) >> 3)))
            }
            // `prg_fix_mmc3_399`: 0, `reg[0]`, `reg[1]`, last.
            Mmc3PKind::M399 => [0, o[0], o[1], LAST][slot],
            // `prg_swap_mmc3_401` with the DIP switches at 0 (J37): `reg[1]` D5-D6 cross into A19-A20.
            Mmc3PKind::M401 => {
                let base =
                    (o[1] & 0x1F) | (o[2] & 0x80) | ((o[1] & 0x40) >> 1) | ((o[1] & 0x20) << 1);
                base | (page & (!o[3] & 0x1F))
            }
            Mmc3PKind::M410 => (o[1] | ((o[2] & 0xC0) << 2)) | (page & (!o[3] & 0x3F)),
            // `prg_swap_mmc3_356`: 410's offset, but masked (`base & ~mask`).
            Mmc3PKind::M356 => {
                let mask = !o[3] & 0x3F;
                ((o[1] | ((o[2] & 0xC0) << 2)) & !mask) | (page & mask)
            }
            // `prg_swap_mmc3_411`: `$5000` D6 the NROM mode, D1 NROM-256.
            Mmc3PKind::M411 => {
                if o[0] & 0x40 != 0 {
                    let base =
                        ((o[1] & 0x40) >> 2) | (o[1] & 0x08) | (o[0] & 0x05) | ((o[0] & 0x08) >> 2);
                    if o[0] & 0x02 != 0 {
                        ((base >> 1) << 2) | (window & 0x03)
                    } else {
                        (base << 1) | (window & 0x01)
                    }
                } else {
                    let base = ((o[1] & 0x40) >> 1) | ((o[1] & 0x08) << 1);
                    base | (page & (0x0F | ((o[1] & 0x02) << 3)))
                }
            }
            // `prg_swap_mmc3_420`: `$6000` D7 the NROM-256 mode.
            Mmc3PKind::M420 => {
                if o[0] & 0x80 != 0 {
                    ((((o[3] & 0x20) >> 2) | ((o[0] & 0x0E) >> 1)) << 2) | (window & 0x03)
                } else {
                    let shift = if o[0] & 0x20 != 0 {
                        2
                    } else {
                        (o[3] & 0x20) >> 5
                    };
                    ((o[3] & 0x04) << 3) | (page & (0x3F >> shift))
                }
            }
            // `prg_swap_mmc3_455`: `reg[1]` D0 the NROM mode, D1 NROM-256; `reg[0]` A0 doubles the
            // inner bank.
            Mmc3PKind::M455 => {
                let base = ((o[1] & 0x40) >> 2) | ((o[0] & 0x04) << 1) | ((o[1] & 0x1C) >> 2);
                if o[1] & 0x01 != 0 {
                    let nrom = (o[1] & 0x02) >> 1;
                    let base = if slot >= 2 { base | nrom } else { base & !nrom };
                    ((base << 1) & !1) | (window & 0x01)
                } else {
                    let mask = 0x1F >> i32::from(o[0] & 0x01 == 0);
                    ((base << 1) & !mask) | (page & mask)
                }
            }
            Mmc3PKind::M456 => ((o[0] << 4) & !0x0F) | (page & 0x0F),
            Mmc3PKind::M457 => {
                let mask = 0x1F >> i32::from(o[0] & 0x08 == 0);
                (((o[0] & 0x07) << 4) & !mask) | (page & mask)
            }
            Mmc3PKind::M516 => (((o[0] & 0x03) << 4) & !0x0F) | (page & 0x0F),
            // `prg_swap_mmc3_393`: `reg[0]` D5 the NROM-like modes, D4 picking UNROM from the
            // latch over a 32 KiB bank from R6.
            Mmc3PKind::M393 => {
                let value = if o[0] & 0x20 == 0 {
                    page
                } else if o[0] & 0x10 != 0 {
                    ((if slot >= 2 { 0x07 } else { o[1] & 0x07 }) << 1) | (window & 0x01)
                } else {
                    (i32::from(self.mmc3.bank_register(6)) & 0x0C) | (window & 0x03)
                };
                (((o[0] & 0x07) << 4) & !0x0F) | (value & 0x0F)
            }
            // `prg_swap_mmc3_412`: `reg[2]` D1 the NROM mode, D2 NROM-256.
            Mmc3PKind::M412 => {
                if o[2] & 0x02 != 0 {
                    let n256 = (o[2] & 0x04) >> 2;
                    let base = o[2] >> 3;
                    let base = if slot >= 2 { base | n256 } else { base & !n256 };
                    (base << 1) | (window & 0x01)
                } else {
                    let mask = ((!o[1] & 0x02) << 4) | (!o[1] & 0x10) | 0x0F;
                    ((((o[1] & 0x40) >> 2) | ((o[1] & 0x04) << 3)) & !mask) | (page & mask)
                }
            }
            Mmc3PKind::M422 | Mmc3PKind::M534 => self.prg_page_126(slot, page),
            Mmc3PKind::M370 => ((o[0] & 0x38) << 1) | (page & (0x1F >> ((o[0] & 0x20) >> 5))),
            Mmc3PKind::M269 => o[1] | (page & (o[3] & 0x3F)),
            // `prg_swap_mmc3_369`: `$37` the 128 KiB at `$10`, other modes 256 KiB at `$20`.
            Mmc3PKind::M369 if o[0] == 0x37 => 0x10 | (page & 0x0F),
            Mmc3PKind::M369 => 0x20 | (page & 0x1F),
            // `prg_swap_mmc3_555`: base `reg[0]` D2 (256 KiB), mask from D0-D1.
            Mmc3PKind::M555 => ((o[0] & 0x04) << 3) | (page & (((o[0] & 0x03) << 3) | 0x07)),
            // `prg_swap_mmc3_406`.
            Mmc3PKind::M406 => page & 0x3F,
            Mmc3PKind::M353 => self.prg_page_353(slot, page),
            Mmc3PKind::M344 | Mmc3PKind::M432 => self.prg_page_dip(slot, page),
            _ => page,
        }
    }

    /// `prg_swap_mmc3_126` for 422 and 534: `$6000` D6 halves the mask, D4-D5 and D0-D2 the
    /// outer bank, `$6003` D0-D1 the NROM modes built from R6.
    fn prg_page_126(&self, slot: usize, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        let window = i32::try_from(slot).unwrap_or(0);
        let mut mask = ((!o[0] & 0x40) >> 2) | 0x0F;
        let mut base = (((o[0] & 0x30) << 3) | ((o[0] & 0x07) << 4)) & !mask;
        if self.submapper == 1 {
            base |= (base & 0x100) >> 1;
        }
        let r6 = i32::from(self.mmc3.bank_register(6));
        let value = match o[3] & 0x03 {
            0 => page,
            3 => {
                base |= r6 & mask;
                mask = 0x03;
                window
            }
            _ => {
                base |= r6 & mask;
                mask = 0x01;
                window & 0x01
            }
        };
        (base & !mask) | (value & mask)
    }

    /// Whether 432's images are under 1 MiB, where `$6001` D5 is the pad read and not A19.
    const fn small_432(&self) -> bool {
        self.prg_size < 0x10_0000
    }

    /// `prg_swap_mmc3_344` (chip order 0: the CRC-selected order of one dump is not kept, J37)
    /// and `prg_swap_mmc3_432`, whose NROM pages are R6 and R7 as nesdev gives them.
    fn prg_page_dip(&self, slot: usize, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        let window = i32::try_from(slot).unwrap_or(0);
        if self.kind == Mmc3PKind::M344 {
            let mut base = (o[0] & 0x03) << 4;
            if o[0] & 0x04 != 0 {
                base |= i32::from(self.mmc3.bank_register(6)) & 0x0C;
                return (base & !0x03) | (window & 0x03);
            }
            return (base & !0x0F) | (page & 0x0F);
        }
        let r1 = o[1];
        let mut base = ((r1 & 0x01) << 4) | ((r1 & 0x10) << 1);
        if !self.small_432() {
            base |= (r1 & 0x20) << 1;
        }
        let mask = 0x1F >> ((r1 & 0x02) >> 1);
        let value = if r1 & 0x40 != 0 {
            let register = i32::from(self.mmc3.bank_register(6 | (slot & 0x01)));
            let nrom256 = (r1 & 0x80) >> 6;
            if slot < 2 {
                register & !nrom256
            } else {
                register | nrom256
            }
        } else {
            page
        };
        (base & !mask) | (value & mask)
    }

    /// `chr_swap_mmc3_344`/`_432`.
    fn chr_page_dip(&self, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        if self.kind == Mmc3PKind::M344 {
            let mask = if o[0] & 0x02 != 0 { 0x7F } else { 0xFF };
            return ((o[0] & 0x03) << 7) | (page & mask);
        }
        let r1 = o[1];
        let mut base = ((r1 & 0x08) << 5) | ((r1 & 0x01) << 7);
        if !self.small_432() {
            base |= (r1 & 0x20) << 4;
        }
        let mask = 0xFF >> ((r1 & 0x04) >> 2);
        (base & !mask) | (page & mask)
    }

    /// Whether `$8000-$FFFF` reads the DIP switches / solder pad now (`extcl_cpu_rd_mem_344`:
    /// the latch's A3; `_432`: `read_dp`).
    const fn reads_dip(&self) -> bool {
        match self.kind {
            Mmc3PKind::M344 => self.outer[0] & 0x08 != 0,
            Mmc3PKind::M432 => {
                self.outer[0] & 0x01 != 0 || (self.small_432() && self.outer[1] & 0x20 != 0)
            }
            _ => false,
        }
    }

    /// `prg_swap_mmc3_353`: game 2 halves the inner bank, R0 D7 its A17; game 3 with R0 D7 clear
    /// shows R6/R7 at `$C000`/`$E000` from bank `$70` up.
    fn prg_page_353(&self, slot: usize, page: i32) -> i32 {
        let game = i32::from(self.outer[0]);
        let r0 = self.mmc3.bank_register(0);
        let mut base = game << 5;
        let mut mask = 0x1F;
        let mut value = page;
        if game == 2 {
            if r0 & 0x80 != 0 {
                base |= 0x10;
            }
            mask >>= 1;
        } else if game == 3 && r0 & 0x80 == 0 && slot >= 2 {
            base = 0x70;
            mask = 0x0F;
            value = i32::from(self.mmc3.bank_register(4 + slot));
        }
        (base & !mask) | (value & mask)
    }

    /// `chr_swap_mmc3_370`.
    fn chr_page_370(&self, page: i32) -> i32 {
        let r = i32::from(self.outer[0]);
        ((r & 0x07) << 7) | (page & (0xFF >> ((!r & 0x04) >> 2)))
    }

    /// `mirroring_fix_mmc3_370` in mode 1: the CIRAM page of `table`, D7 of the CHR register
    /// that serves it inverted.
    fn page_370(&self, table: usize) -> i32 {
        let register = if self.mmc3.select() & 0x80 != 0 {
            2 + table
        } else {
            table >> 1
        };
        i32::from((self.mmc3.bank_register(register) >> 7) ^ 0x01)
    }

    /// `chr_swap_mmc3_126` for 422 and 534 (`reg0 ^ $20`, the non-"old dump" reading); `slot` is
    /// the 1 KiB window, which the CNROM-like mode (`$6003` D4) uses as the page.
    fn chr_page_126(&self, slot: usize, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        let reg0 = o[0] ^ 0x20;
        let mut base = (reg0 & 0x38) << 4;
        let mut mask = (!reg0 & 0x80) | 0x7F;
        let mut value = page;
        if o[3] & 0x10 != 0 {
            base = ((o[2] & (mask >> 3)) | (base >> 3)) << 3;
            mask = 0x07;
            value = i32::try_from(slot).unwrap_or(0);
        }
        (base & !mask) | (value & mask)
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
            _ => self.chr_page_b2(page),
        }
    }

    /// [`Mmc3P::chr_page`] for group 2 (399 maps its CHR in [`Mmc3P::refresh`]).
    fn chr_page_b2(&self, page: i32) -> i32 {
        let o = self.outer.map(i32::from);
        match self.kind {
            Mmc3PKind::M395 => {
                let base = ((o[1] & 0x20) << 5) | ((o[0] & 0x30) << 4) | ((o[1] & 0x10) << 3);
                base | (page & (0xFF >> ((o[1] & 0x40) >> 6)))
            }
            Mmc3PKind::M401 | Mmc3PKind::M410 => {
                (o[0] | ((o[2] & 0xF0) << 4)) | (page & (0xFF >> (!o[2] & 0x0F)))
            }
            Mmc3PKind::M356 => {
                let mask = 0xFF >> (!o[2] & 0x0F);
                ((((o[2] & 0xF0) << 4) | o[0]) & !mask) | (page & mask)
            }
            Mmc3PKind::M411 => {
                let base = ((o[0] & 0x10) << 4) | ((o[1] & 0x04) << 5);
                base | (page & (0x7F | ((o[1] & 0x02) << 6)))
            }
            Mmc3PKind::M420 => {
                let base = ((o[1] & 0x80) << 1) | ((o[1] & 0x04) << 5);
                base | (page & (0xFF >> ((o[1] & 0x80) >> 7)))
            }
            Mmc3PKind::M455 => {
                let base =
                    (((o[1] & 0x40) >> 2) | ((o[1] & 0x1C) >> 2) | ((o[0] & 0x04) << 1)) << 4;
                let mask = 0xFF >> i32::from(o[0] & 0x02 == 0);
                (base & !mask) | (page & mask)
            }
            Mmc3PKind::M456 => ((o[0] << 7) & !0x7F) | (page & 0x7F),
            Mmc3PKind::M457 => {
                let mask = 0xFF >> i32::from(o[0] & 0x08 == 0);
                (((o[0] & 0x07) << 7) & !mask) | (page & mask)
            }
            Mmc3PKind::M516 => (((o[0] & 0x0C) << 5) & !0x7F) | (page & 0x7F),
            Mmc3PKind::M393 => (((o[0] & 0x07) << 8) & !0xFF) | (page & 0xFF),
            Mmc3PKind::M412 => {
                let base = ((o[1] & 0x08) << 5) | (o[1] & 0x80);
                let mask = 0xFF >> ((o[1] & 0x20) >> 5);
                (base & !mask) | (page & mask)
            }
            _ => page,
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
                Mmc3PKind::M410 => self.outer[2] & 0x40 != 0,
                // `chr_swap_mmc3_356`: the RAM while D5 is **clear**.
                Mmc3PKind::M356 => self.outer[2] & 0x20 == 0,
                Mmc3PKind::M353 => self.outer[0] == 2 && self.mmc3.bank_register(0) & 0x80 != 0,
                Mmc3PKind::M393 => self.outer[0] & 0x08 != 0,
                _ => false,
            }
    }

    /// Recomputes the twelve windows from the chip and the outer registers.
    fn refresh(&mut self) {
        let raw_prg = self.mmc3_prg();
        let prg: [i32; BankWindows::PRG_WINDOWS] = if self.kind == Mmc3PKind::M100 {
            std::array::from_fn(|slot| i32::from(self.raw_100[1 + slot]))
        } else {
            std::array::from_fn(|slot| self.prg_page(slot, raw_prg[slot]))
        };
        let prg = if self.kind == Mmc3PKind::M369 {
            self.prg_369(prg)
        } else if self.kind == Mmc3PKind::M451 {
            // `prg_fix_451`: the chip's own banks are never written.
            let reg = i32::from(self.outer[0]);
            let inner = (reg & 0x01) | ((reg & 0x02) << 2);
            [0, 0x10 + inner, 0x20 + inner, 0x30]
        } else {
            prg
        };
        let raw_chr = self.mmc3_chr();
        let chr: [i32; BankWindows::CHR_WINDOWS] = if self.kind == Mmc3PKind::M369 {
            self.chr_369(&raw_chr)
        } else if self.kind == Mmc3PKind::M451 {
            // `chr_fix_451`: an 8 KiB bank, `$E000` A0.
            let base = (i32::from(self.outer[0]) & 0x01) << 3;
            std::array::from_fn(|slot| base | i32::try_from(slot).unwrap_or(0))
        } else if self.kind == Mmc3PKind::M100 {
            std::array::from_fn(|slot| i32::from(self.raw_100[5 + slot]))
        } else if self.chr_ram_mode() {
            std::array::from_fn(|slot| i32::try_from(slot).unwrap_or(0))
        } else if self.kind == Mmc3PKind::M412 && self.outer[2] & 0x02 != 0 {
            // `chr_swap_mmc3_412` in the NROM mode: an 8 KiB bank from `reg[0]` D2 up.
            let base = (i32::from(self.outer[0]) >> 2) << 3;
            std::array::from_fn(|slot| base | i32::try_from(slot).unwrap_or(0))
        } else if matches!(self.kind, Mmc3PKind::M344 | Mmc3PKind::M432) {
            std::array::from_fn(|slot| self.chr_page_dip(raw_chr[slot]))
        } else if self.kind == Mmc3PKind::M555 {
            let base = (i32::from(self.outer[0]) & 0x04) << 5;
            let mixed = self.outer[0] & 0x06 == 0x02;
            std::array::from_fn(|slot| {
                if mixed {
                    base | raw_chr[slot]
                } else {
                    base | (raw_chr[slot] & 0x7F)
                }
            })
        } else if self.kind == Mmc3PKind::M269 {
            let (offset, mask) = (i32::from(self.outer[0]), i32::from(self.outer[2]));
            std::array::from_fn(|slot| offset | (raw_chr[slot] & mask))
        } else if self.kind == Mmc3PKind::M353 {
            let base = i32::from(self.outer[0]) << 7;
            std::array::from_fn(|slot| base | (raw_chr[slot] & 0x7F))
        } else if self.kind == Mmc3PKind::M370 {
            std::array::from_fn(|slot| self.chr_page_370(raw_chr[slot]))
        } else if matches!(self.kind, Mmc3PKind::M422 | Mmc3PKind::M534) {
            std::array::from_fn(|slot| self.chr_page_126(slot, raw_chr[slot]))
        } else if self.kind == Mmc3PKind::M399 {
            // `chr_fix_mmc3_399`: two 4 KiB banks, `reg[2]` and `reg[3]`.
            let (low, high) = (i32::from(self.outer[2]) << 2, i32::from(self.outer[3]) << 2);
            [
                low,
                low + 1,
                low + 2,
                low + 3,
                high,
                high + 1,
                high + 2,
                high + 3,
            ]
        } else {
            std::array::from_fn(|slot| self.chr_page(raw_chr[slot]))
        };
        if self.kind == Mmc3PKind::M555 {
            self.settle_ram_555(&raw_chr);
        }
        // `chr_swap_mmc3_512`: with `$4100` D1 set every window is CHR-RAM bank `value & 3`.
        if self.kind == Mmc3PKind::M512 {
            self.ram_windows = if self.outer[0] & 0x02 != 0 && !self.chr_ram {
                0xFF
            } else {
                0
            };
            for (slot, value) in raw_chr.iter().enumerate() {
                self.ram_banks[slot] = u8::try_from(value & 0x03).unwrap_or(0);
            }
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

    /// The mirroring a kind imposes over the chip's (`mirroring_fix_mmc3_345`: D5 single-screen).
    fn settle_mirroring(&mut self, memory: &mut CartridgeMemory) {
        // `mirroring_fix_mmc3_512`: `$4100` = 1 makes the four tables the CHR-RAM's second
        // 4 KiB, decided when the chip's mirroring is (`outer[1]` holds the decision).
        if self.kind == Mmc3PKind::M512 {
            self.outer[1] = u16::from(self.outer[0] == 1 && memory.chr_ram().len() >= 0x2000);
        }
        // `mirroring_fix_mmc3_356`: D6 of `reg[2]` is four-screen (the PPU's 4 KiB).
        if self.kind == Mmc3PKind::M356 {
            memory.set_mirroring(if self.outer[2] & 0x40 != 0 {
                NametableMirroring::FourScreen
            } else if self.a000 & 0x01 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            });
            return;
        }
        if self.kind == Mmc3PKind::M345 && self.outer[0] & 0x20 != 0 {
            memory.set_mirroring(if self.outer[0] & 0x10 != 0 {
                NametableMirroring::SingleScreenB
            } else {
                NametableMirroring::SingleScreenA
            });
        }
    }

    /// `extcl_cpu_wr_mem_432`: A0 the register; on an image under 1 MiB an even write with D0
    /// clear also clears `$6001` D5.
    fn write_432(r: &mut [u16; 4], prg_size: usize, address: u16, data: u16) {
        r[usize::from(address & 0x01)] = data;
        if prg_size < 0x10_0000 && address & 0x01 == 0 && data & 0x01 == 0 {
            r[1] &= !0x20;
        }
    }

    /// `extcl_cpu_wr_mem_269`: the four writes in turn. `r[0]`/`r[1]` the CHR/PRG offsets, `r[2]`
    /// the CHR mask, `r[3]` the PRG mask with the fourth value above it.
    fn write_269(r: &mut [u16; 4], index: u8, value: u8) {
        let v = u16::from(value);
        match index {
            0 => r[0] = (r[0] & 0xFF00) | v,
            1 => r[1] = (r[1] & 0xFF00) | v,
            2 => {
                r[0] = (r[0] & 0xF0FF) | ((v & 0xF0) << 4);
                r[2] = 0xFF >> (!v & 0x0F);
            }
            _ => {
                r[0] = (r[0] & 0xEFFF) | ((v & 0x40) << 6);
                r[1] = (r[1] & 0xFEFF) | ((v & 0x40) << 2);
                r[3] = (v << 8) | (!v & 0x3F);
            }
        }
    }

    /// `map_init_269`: without CHR-ROM, the CHR is the PRG-ROM with D0→D6, D1→D4, D2→D2,
    /// D3→D0, D4→D1, D5→D3, D6→D5, D7→D7 (the board's wiring); empty with CHR-ROM.
    fn chr_269(memory: &CartridgeMemory) -> Vec<u8> {
        if !memory.rom().chr_rom().is_empty() {
            return Vec::new();
        }
        memory
            .rom()
            .prg_rom()
            .iter()
            .map(|value| {
                ((value & 0x01) << 6)
                    | ((value & 0x02) << 3)
                    | (value & 0x04)
                    | ((value & 0x08) >> 3)
                    | ((value & 0x10) >> 3)
                    | ((value & 0x20) >> 2)
                    | ((value & 0x40) >> 1)
                    | (value & 0x80)
            })
            .collect()
    }

    /// `extcl_cpu_wr_mem_126`: `$6000-$6003` until `$6003` D7 locks them; locked, `$6002` still
    /// takes its low one or two bits (one when its D4 is set).
    fn write_126(r: &mut [u16; 4], address: u16, data: u16) {
        let index = usize::from(address & 0x03);
        if r[3] & 0x80 == 0 {
            r[index] = data;
        } else if index == 2 {
            let mask = 0x03 >> u16::from(r[2] & 0x10 != 0);
            r[2] = (r[2] & !mask) | (data & mask);
        }
    }

    /// What follows a write that reached an outer register: the RAM's copy, the windows and the
    /// mirroring.
    fn after_low_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
        writable: bool,
    ) {
        // `wram_wr` before the board: the RAM takes the byte too.
        if writable && address >= 0x6000 {
            let ram = memory.prg_ram_mut();
            if !ram.is_empty() {
                let len = ram.len();
                ram[usize::from(address & 0x1FFF) % len] = value;
            }
        }
        self.refresh();
        self.settle_mirroring(memory);
    }

    /// A write to `$4020-$5FFF` that reaches an outer register (no RAM there); `false` when it
    /// is not one.
    fn write_5000(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        let data = u16::from(value);
        let r = &mut self.outer;
        match self.kind {
            // `extcl_cpu_wr_mem_455`/`_456`: `$4100` (A8 set in `$4000-$5FFF`), no RAM there.
            Mmc3PKind::M455 if address < 0x6000 && address & 0x0100 != 0 => {
                r[0] = address;
                r[1] = data;
            }
            Mmc3PKind::M456 if address < 0x6000 && address & 0x0100 != 0 => r[0] = data,
            // `extcl_cpu_wr_mem_269`.
            Mmc3PKind::M269 if (0x5000..=0x5FFF).contains(&address) => {
                if (r[3] >> 8) & 0x80 == 0 {
                    Self::write_269(r, self.index, value);
                    self.index = (self.index + 1) & 0x03;
                }
            }
            // `extcl_cpu_wr_mem_555`: `$5800`/`$5C00` (A11, A10 the register); `wram_wr` for the
            // whole of `$5000-$5FFF`, which `wram_fix_mmc3_555` maps to the RAM's 4 KiB bank 2.
            Mmc3PKind::M555 if (0x5000..=0x5FFF).contains(&address) => {
                if address & 0x0800 != 0 {
                    r[usize::from((address & 0x0400) >> 10)] = data;
                }
                let ram = memory.prg_ram_mut();
                if !ram.is_empty() {
                    let len = ram.len();
                    ram[(0x2000 + usize::from(address & 0x0FFF)) % len] = value;
                }
            }
            // `extcl_cpu_wr_mem_369`: `$4100` (A8 in `$4000-$4FFF`) the mode.
            Mmc3PKind::M369 if address & 0xF100 == 0x4100 => r[0] = data,
            // `extcl_cpu_wr_mem_512`: `$4100` (A8 in `$4000-$4FFF`), D0-D1.
            Mmc3PKind::M512 if address & 0xF100 == 0x4100 => r[0] = data & 0x03,
            // `extcl_cpu_wr_mem_370`: `$5000-$5FFF`, the low address byte.
            Mmc3PKind::M370 if (0x5000..=0x5FFF).contains(&address) => r[0] = address & 0xFF,
            // `extcl_cpu_wr_mem_411`: `$5000-$5FFF`, A0 the register, no RAM there.
            Mmc3PKind::M411 if (0x5000..=0x5FFF).contains(&address) => {
                r[usize::from(address & 0x0001)] = data;
            }
            _ => return false,
        }
        self.after_low_write(memory, address, value, false);
        true
    }

    /// A write below `$8000` that reaches an outer register; `false` when it is not one.
    fn write_low(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        if address < 0x6000 {
            return self.write_5000(memory, address, value);
        }
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
            Mmc3PKind::M315 | Mmc3PKind::M345 | Mmc3PKind::M457
                if (0x6000..=0x7FFF).contains(&address) =>
            {
                if writable {
                    r[0] = data;
                }
            }
            Mmc3PKind::M322 | Mmc3PKind::M339 | Mmc3PKind::M344
                if (0x6000..=0x7FFF).contains(&address) =>
            {
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
            // `extcl_cpu_wr_mem_395`: A4 picks the register until `reg[1]` D7 locks both.
            Mmc3PKind::M395 if (0x6000..=0x7FFF).contains(&address) => {
                if r[1] & 0x80 == 0 && writable {
                    r[usize::from((address & 0x0010) >> 4)] = data;
                }
            }
            // `extcl_cpu_wr_mem_420`: `$6000-$6003` under mask 3.
            Mmc3PKind::M420 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    r[usize::from(address & 0x03)] = data;
                }
            }
            Mmc3PKind::M393 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    r[0] = address & 0xFF;
                }
            }
            // `extcl_cpu_wr_mem_412`: `$5000-$7FFF` under mask 3 until `reg[1]` D0 locks them.
            Mmc3PKind::M412 if (0x5000..=0x7FFF).contains(&address) => {
                if writable && r[1] & 0x01 == 0 {
                    r[usize::from(address & 0x03)] = data;
                }
            }
            // `extcl_cpu_wr_mem_432`.
            Mmc3PKind::M432 if (0x6000..=0x7FFF).contains(&address) => {
                if writable {
                    Self::write_432(r, self.prg_size, address, data);
                }
            }
            // `extcl_cpu_wr_mem_401`/`_410`: four registers in turn until `reg[3]` D6 locks them.
            Mmc3PKind::M356 | Mmc3PKind::M401 | Mmc3PKind::M410
                if (0x6000..=0x7FFF).contains(&address) =>
            {
                if r[3] & 0x40 == 0 && writable {
                    r[usize::from(self.index)] = data;
                    self.index = (self.index + 1) & 0x03;
                }
            }
            // `extcl_cpu_wr_mem_126`.
            Mmc3PKind::M422 | Mmc3PKind::M534 if (0x6000..=0x7FFF).contains(&address) => {
                Self::write_126(r, address, data);
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
        self.after_low_write(memory, address, value, writable);
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
        writer.write_u8(self.a000);
        if self.kind == Mmc3PKind::M100 {
            writer.write_raw(&self.raw_100);
        }
        if self.kind == Mmc3PKind::M555 {
            writer.write_u32(self.timer_555);
            writer.write_bool(self.expired_555);
        }
        if matches!(self.kind, Mmc3PKind::M406 | Mmc3PKind::M451) {
            #[cfg(any(feature = "board-406", feature = "board-451"))]
            self.flash.save_state(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        for value in &mut self.outer {
            *value = reader.read_u16()?;
        }
        self.index = reader.read_u8()? & 0x03;
        self.a000 = reader.read_u8()?;
        if self.kind == Mmc3PKind::M100 {
            reader.read_into(&mut self.raw_100)?;
        }
        if self.kind == Mmc3PKind::M555 {
            self.timer_555 = reader.read_u32()?;
            self.expired_555 = reader.read_bool()?;
        }
        if matches!(self.kind, Mmc3PKind::M406 | Mmc3PKind::M451) {
            #[cfg(any(feature = "board-406", feature = "board-451"))]
            self.flash.load_state(reader)?;
        }
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.power_on();
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted() || (self.kind == Mmc3PKind::M369 && self.outer[2] & 0x02 != 0)
    }

    /// `wram_fix_mmc3_406`: 406 has no WRAM.
    fn prg_ram_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled() && !self.smb2j_369() && self.kind != Mmc3PKind::M406
    }

    fn prg_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable() && !self.smb2j_369() && self.kind != Mmc3PKind::M406
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
        // `extcl_cpu_wr_mem_406`/`_451`: every write is the flash chip's first, at the address
        // the CPU drove; then the chip's address lines as the board crosses them.
        let address = match self.kind {
            Mmc3PKind::M406 => {
                self.write_flash(memory, address, value);
                if self.submapper == 0 {
                    (address & 0xFFFC) | ((address & 0x0001) << 1) | ((address & 0x0002) >> 1)
                } else if address <= 0x9000 || address >= 0xE000 {
                    address ^ 0x6000
                } else {
                    address
                }
            }
            Mmc3PKind::M451 => {
                self.write_flash(memory, address, value);
                self.write_451(memory, address);
                self.refresh();
                self.settle_mirroring(memory);
                return;
            }
            _ => address,
        };
        // `extcl_cpu_wr_mem_399`: `$8000-$9FFF` also writes its own bank registers, A0 clear
        // the CHR pair and D7 the half.
        // `extcl_cpu_wr_mem_393`: every chip write is also the UNROM latch.
        if self.kind == Mmc3PKind::M393 {
            self.outer[1] = u16::from(value);
        }
        if address & 0xE001 == 0xA000 {
            self.a000 = value;
        }
        if self.kind == Mmc3PKind::M100 {
            self.write_100(address, value);
        }
        // `extcl_cpu_wr_mem_353`: `$x080` selects the game from A13-A14.
        // `extcl_cpu_wr_mem_369` in the *SMB2J* mode: `$8000-$9FFF` disables and acknowledges the
        // IRQ (`outer[2]` D0 enable, D1 pending), `$A000-$BFFF` D1 enables it, `$E000-$FFFF`
        // the bank at `$C000`.
        if self.smb2j_369() {
            match address & 0xE000 {
                0x8000 => self.outer[2] = 0,
                0xA000 => self.outer[2] = (self.outer[2] & 0x02) | u16::from((value >> 1) & 0x01),
                0xE000 => self.outer[1] = u16::from(value),
                _ => {}
            }
        }
        if self.kind == Mmc3PKind::M353 && address & 0x0FFF == 0x0080 {
            self.outer[0] = (address >> 13) & 0x03;
        }
        // `extcl_cpu_wr_mem_516`: a write with A4 set also latches A0-A3.
        if self.kind == Mmc3PKind::M516 && address & 0x0010 != 0 {
            self.outer[0] = address & 0x0F;
        }
        // `extcl_cpu_wr_mem_126`: 534 inverts the data of `$C000`/`$C001` (the IRQ latch).
        let value = if self.kind == Mmc3PKind::M534 && address & 0xE000 == 0xC000 {
            !value
        } else {
            value
        };
        if self.kind == Mmc3PKind::M399 && address < 0xA000 {
            let index = (usize::from(address & 0x0001 == 0) << 1) | usize::from(value >> 7);
            self.outer[index] = u16::from(value);
        }
        self.mmc3.write_register(memory, address, value);
        self.refresh();
        self.settle_mirroring(memory);
    }

    /// `extcl_cpu_rd_mem_334`: `$6000-$7FFF` with A1 set reads `(open bus & $FE) | DIP` while the
    /// RAM is readable; the DIP is 0 and the high address byte stands in for the bus, as
    /// [`super::Mmc3Clone`] does for its DIP reads (FIDELITY-LEADS J33).
    /// The DIP / solder-pad read of 344 and 432 replaces the whole of `$8000-$FFFF`, so their
    /// windows leave the cached path while it is on.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.reads_dip() || self.flash_answers() {
            return [super::NO_ROM_WINDOW; 4];
        }
        self.mmc3
            .banks()
            .map_or([super::NO_ROM_WINDOW; 4], |banks| {
                banks.rom_windows(memory.rom().prg_rom().len())
            })
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        // The switches and the pad are 0 (J37).
        if address >= 0x8000 && self.reads_dip() {
            return Some(0);
        }
        // `sst39sf040_read`: the IDs in the software-ID mode (A0 the model), the toggle bit D6
        // and `& $77` while an erase runs.
        #[cfg(any(feature = "board-406", feature = "board-451"))]
        if address >= 0x8000 && self.flash_answers() {
            let byte = self.mmc3.try_read_prg(memory, address);
            return self.flash.read(&self.flash_chip(), address, byte);
        }
        // `extcl_cpu_rd_mem_370`: `$5000-$5FFF` reads `DIP & $80 | open bus & $7F`; the DIP is 0
        // and the high address byte stands in for the bus (J33).
        // `extcl_cpu_rd_mem_555`: `$5800-$5FFF` the timer's expiry in D7 over `$5C`, `$5000-$57FF`
        // the RAM's 4 KiB bank 2.
        if self.kind == Mmc3PKind::M555 && (0x5000..=0x5FFF).contains(&address) {
            if address & 0x0800 != 0 {
                return Some(if self.expired_555 { 0xDC } else { 0x5C });
            }
            let ram = memory.prg_ram();
            return (!ram.is_empty())
                .then(|| ram[(0x2000 + usize::from(address & 0x0FFF)) % ram.len()]);
        }
        // `wram_fix_mmc3_369`: PRG-ROM bank `$0E` at `$6000` in the *SMB2J* mode.
        if self.smb2j_369() && (0x6000..=0x7FFF).contains(&address) {
            let rom = memory.rom().prg_rom();
            return (!rom.is_empty())
                .then(|| rom[(0xE * 0x2000 + usize::from(address & 0x1FFF)) % rom.len()]);
        }
        if self.kind == Mmc3PKind::M370 && (0x5000..=0x5FFF).contains(&address) {
            return Some(address.to_be_bytes()[0] & 0x7F);
        }
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
        if !self.chr_269.is_empty() || matches!(self.kind, Mmc3PKind::M555 | Mmc3PKind::M512) {
            return None;
        }
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        if let Some(offset) = self.ram_offset_555(address) {
            let ram = memory.chr_ram();
            return if ram.is_empty() {
                0
            } else {
                ram[offset % ram.len()]
            };
        }
        if !self.chr_269.is_empty() && address < 0x2000 {
            return self
                .mmc3
                .banks()
                .map_or(0, |banks| banks.read_chr(address, &self.chr_269));
        }
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    /// 269's PRG-derived CHR is ROM to the PPU.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if !self.chr_269.is_empty() {
            return;
        }
        if let Some(offset) = self.ram_offset_555(address) {
            let ram = memory.chr_ram_mut();
            if !ram.is_empty() {
                let len = ram.len();
                ram[offset % len] = value;
            }
            return;
        }
        self.mmc3.ppu_write(memory, address, value);
    }

    /// `extcl_cpu_every_cycle_555`: with `reg[0]` D3 clear the timer is held at 0; set, it counts
    /// CPU cycles to `(DIP | $10) << 25`, the DIP being 0 here (J37), about five minutes.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        // `sst39sf040_tick`: the end of an erase ends the command.
        #[cfg(any(feature = "board-406", feature = "board-451"))]
        if self.flash.tick() {
            return;
        }
        // `extcl_cpu_every_cycle_369`: a 12-bit counter asserting on its wrap to 0.
        if self.smb2j_369() {
            if self.outer[2] & 0x01 != 0 {
                self.outer[3] = (self.outer[3] + 1) & 0x0FFF;
                if self.outer[3] == 0 {
                    self.outer[2] |= 0x02;
                }
            }
            return;
        }
        if self.kind != Mmc3PKind::M555 {
            return;
        }
        if self.outer[0] & 0x08 == 0 {
            self.timer_555 = 0;
            self.expired_555 = false;
        } else {
            self.timer_555 = self.timer_555.wrapping_add(1);
            if self.timer_555 == 0x10 << 25 {
                self.expired_555 = true;
            }
        }
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        // 369's *SMB2J* mode leaves the chip's A12 counter alone (`extcl_ppu_*_369`).
        if self.smb2j_369() {
            return;
        }
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }

    /// `memmap_nmt_vram_4k(…, 1)`: 512's tables in the CHR-RAM from `$1000`.
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }
        ram[(0x1000 + usize::from(address & 0x0FFF)) % ram.len()]
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[(0x1000 + usize::from(address & 0x0FFF)) % len] = value;
        }
    }

    fn nametable_page(&self, table: i32) -> i32 {
        if self.kind == Mmc3PKind::M512 && self.outer[1] != 0 {
            return super::super::Cartridge::SERVED_BY_CARTRIDGE;
        }
        if (self.kind == Mmc3PKind::M370 && self.outer[0] & 0x07 == 0x01)
            || (self.kind == Mmc3PKind::M353 && self.outer[0] == 0)
        {
            return self.page_370(usize::try_from(table & 0x03).unwrap_or(0));
        }
        self.mmc3.nametable_page(table)
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        self.mmc3.ppu_irq_horizon()
    }
}
