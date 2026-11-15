//! Lot MAPPERS-P: the discrete boards puNES runs and Zaxnes did not (`docs/progress/MAPPERS-P.md`).
//!
//! Each kind is two functions, kept side by side: what a CPU write does to the registers
//! ([`MultiP::latch`]) and what the registers map ([`MultiP::map`]). The behaviour of every kind is
//! read from puNES 0.111 (`src/core/mappers/mapper_NNN.c`, read, not copied) and, where it exists,
//! nesdev's page for the number; each arm names its source. Where puNES leaves a part out (a
//! microphone, a DIP switch), the arm says what stands in for it, and FIDELITY-LEADS J37 lists it.
//!
//! puNES conventions the arms translate: `extcl_cpu_wr_mem` sees `$8000-$FFFF` unless the board sets
//! `extend_wr` (then `$4020-$7FFF` too); `memmap_auto_*` banks wrap on the chip's size, as
//! [`BankWindows`] does; `mirroring_H`/`mirroring_V` are horizontal and vertical.
use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The boards of this module, by iNES / NES 2.0 number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiPKind {
    /// 55, `mapper_055.c` (BTL-MARIO1-MALEE2): NROM-256 with 2 KiB of PRG-ROM at `$6000` and
    /// 2 KiB of work RAM at `$7000`.
    M55,
    /// 81, `mapper_081.c` (NTDEC N715021, *Super Gun*): address latch, A2-A3 the 16 KiB bank, A0-A1
    /// the 8 KiB CHR bank.
    M81,
    /// 122, `mapper_184.c` (Sunsoft-1 wiring, the number puNES maps onto 184's code): `$6000-$7FFF`
    /// two 4 KiB CHR banks.
    M122,
    /// 237, `mapper_237.c` (Teletubbies 420-in-1): address and data latch, a lock bit, a DIP read.
    M237,
    /// 389, `mapper_389.c` (Caltron 9-in-1): three address registers at `$8000`, `$9000` and
    /// `$A000-$FFFF`.
    M389,
    /// 517, `mapper_517.c` (Kkachi-wa Norae Chingu, a karaoke with a microphone ADC at
    /// `$6000-$6FFF`).
    M517,
    /// 271, `mapper_271.c` (TXC 22026): one data latch, D4-D7 the 32 KiB bank, D0-D3 the CHR bank.
    M271,
    /// 335, `mapper_335.c` (CTC-09): `$8000-$BFFF` the CHR bank, `$C000-$FFFF` PRG and mirroring.
    M335,
    /// 338, `mapper_338.c` (SA005-A): address latch, NROM-128 bank, two DIP reads.
    M338,
    /// 341, `mapper_341.c` (TJ-03): address latch, A8 up the 32 KiB and CHR bank.
    M341,
    /// 343, `mapper_343.c` (reset-based 4-in-1 Gamestar): inverted data latch.
    M343,
    /// 352, `mapper_352.c` (Kaiser KS-106C): no register, the game chosen by counting resets.
    M352,
    /// 360, `mapper_360.c` (Bit Corp 31-in-1): the game chosen by a DIP switch, no register.
    M360,
    /// 381, `mapper_381.c` (KN-42): UNROM data latch with scrambled bits, outer bank by reset.
    M381,
    /// 337, `mapper_337.c` (CTC-12IN1): two half registers, three PRG modes, PRG-ROM at `$6000`.
    M337,
    /// 350, `mapper_350.c` (Super 15-in-1): two registers, a second 128 KiB chip, PRG-ROM at
    /// `$6000`.
    M350,
    /// 375, `mapper_375.c` (135-in-1): a lockable address latch and a data latch, CHR-RAM
    /// write protection.
    M375,
    /// 354, `mapper_354.c` (FAM250 / 810139C): address and data latch at `$F000` (`$E000` on
    /// submapper 1), seven PRG modes, PRG-ROM at `$6000` in mode 5, CHR-RAM write protection.
    M354,
    /// 380, `mapper_380.c` (970630C / KN-35A): address latch, a DIP read (submapper 0), CHR-RAM
    /// write protection, submapper-dependent last bank and mirroring bit.
    M380,
    /// 382, `mapper_382.c` (830928C): lockable address latch (A5), data latch with bus conflicts.
    M382,
    /// 390, `mapper_390.c` (Realtec 8031): two address registers chosen by A14.
    M390,
    /// 396, `mapper_396.c` (YY850437C): `$A000-$BFFF` the outer bank and mirroring, elsewhere the
    /// UNROM inner bank.
    M396,
    /// 400, `mapper_400.c` (8-BIT XMAS 2017): `$7800-$7FFF` the outer bank, `$C000-$FFFF` the
    /// inner PRG and CHR bank with bus conflicts, `$8000-$BFFF` an LED.
    M400,
    /// 403, `mapper_403.c` (89433): three registers at `$4100-$4103`, CHR from `$8000` when enabled.
    M403,
    /// 409, `mapper_409.c` (retroUSB DPCMcart): `$C000-$CFFF` the address is the 16 KiB bank.
    M409,
    /// 414, `mapper_414.c` (9999999-in-1): address latch and a CHR data latch with bus conflicts.
    M414,
    /// 415, `mapper_415.c` (Lucky Rabbit 0353): fixed 32 KiB, a switchable 8 KiB at `$6000`.
    M415,
    /// 428, `mapper_428.c` (TU-01): four registers at `$6000-$7FFF`, a CHR latch at `$8000`.
    M428,
    /// 429, `mapper_429.c` (LIKO BBG-235-8-1B): one data latch, 32 KiB and 8 KiB banks.
    M429,
    /// 431, `mapper_431.c` (Realtec GN-91B): two data registers with bus conflicts.
    M431,
    /// 433, `mapper_433.c` (NC-20MB): one data latch, NROM-128/256, CHR-RAM write enable in D7.
    M433,
    /// 434, `mapper_434.c` (S-009): `$6000-$7FFF` the outer bank, `$8000` UNROM with bus conflicts.
    M434,
    /// 437, `mapper_437.c` (NTDEC TH2348): `$5000-$5FFF` address the outer bank, `$8000` UNROM with
    /// bus conflicts.
    M437,
    /// 438, `mapper_438.c` (K-3071): address and data latch, NROM-128/256 by A0.
    M438,
    /// 481, `mapper_481.c` (045N): UNROM with D7 the outer 128 KiB bank.
    M481,
    /// 538, `mapper_538.c` (60-1064-16L, an FDS conversion): `$C000-$DFFF` latch, fixed upper
    /// 24 KiB, a switchable 8 KiB at `$8000` and at `$6000`.
    M538,
    /// 541, `mapper_541.c` (LittleCom 160-in-1): `$C000-$FFFF` the address is the register.
    M541,
    /// 564, `mapper_564.c`: a data latch whose writable bits shrink once its D5 is set, four
    /// mirrorings.
    M564,
    /// 525, `mapper_525.c` (Kaiser KS-7021A): `$8000` the 16 KiB bank (D1 up), `$9000` the
    /// mirroring, `$B000-$B007` eight 1 KiB CHR banks.
    M525,
    /// 554, `mapper_554.c` (Kaiser KS-7010, an FDS conversion): no register; **reads** of a few
    /// addresses in `$CAB6-$CAD7`, `$EBE2`, `$EE32` and `$FFFC` latch the CHR bank and the 8 KiB
    /// bank at `$6000` from A2-A5.
    M554,
}

impl MultiPKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            55 => Self::M55,
            81 => Self::M81,
            122 => Self::M122,
            237 => Self::M237,
            389 => Self::M389,
            517 => Self::M517,
            271 => Self::M271,
            335 => Self::M335,
            338 => Self::M338,
            341 => Self::M341,
            343 => Self::M343,
            352 => Self::M352,
            360 => Self::M360,
            381 => Self::M381,
            337 => Self::M337,
            350 => Self::M350,
            375 => Self::M375,
            354 => Self::M354,
            380 => Self::M380,
            382 => Self::M382,
            390 => Self::M390,
            396 => Self::M396,
            400 => Self::M400,
            403 => Self::M403,
            409 => Self::M409,
            414 => Self::M414,
            415 => Self::M415,
            428 => Self::M428,
            429 => Self::M429,
            431 => Self::M431,
            433 => Self::M433,
            434 => Self::M434,
            437 => Self::M437,
            438 => Self::M438,
            481 => Self::M481,
            538 => Self::M538,
            541 => Self::M541,
            564 => Self::M564,
            525 => Self::M525,
            554 => Self::M554,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M55 => 55,
            Self::M81 => 81,
            Self::M122 => 122,
            Self::M237 => 237,
            Self::M389 => 389,
            Self::M517 => 517,
            Self::M271 => 271,
            Self::M335 => 335,
            Self::M338 => 338,
            Self::M341 => 341,
            Self::M343 => 343,
            Self::M352 => 352,
            Self::M360 => 360,
            Self::M381 => 381,
            Self::M337 => 337,
            Self::M350 => 350,
            Self::M375 => 375,
            Self::M354 => 354,
            Self::M380 => 380,
            Self::M382 => 382,
            Self::M390 => 390,
            Self::M396 => 396,
            Self::M400 => 400,
            Self::M403 => 403,
            Self::M409 => 409,
            Self::M414 => 414,
            Self::M415 => 415,
            Self::M428 => 428,
            Self::M429 => 429,
            Self::M431 => 431,
            Self::M433 => 433,
            Self::M434 => 434,
            Self::M437 => 437,
            Self::M438 => 438,
            Self::M481 => 481,
            Self::M538 => 538,
            Self::M541 => 541,
            Self::M564 => 564,
            Self::M525 => 525,
            Self::M554 => 554,
        }
    }

    /// Whether the board takes writes below `$8000` (puNES `extend_wr`), and where.
    const fn decodes_low(self, address: u16) -> bool {
        match self {
            Self::M122 | Self::M428 | Self::M434 => address >= 0x6000,
            Self::M400 => address >= 0x7800,
            Self::M437 => address >= 0x5000 && address <= 0x5FFF,
            Self::M403 => address < 0x6000 && address & 0x0100 != 0,
            _ => false,
        }
    }

    /// Whether `$6000-$7FFF` is something else than work RAM.
    const fn owns_6000(self) -> bool {
        matches!(
            self,
            Self::M55
                | Self::M122
                | Self::M434
                | Self::M538
                | Self::M554
                | Self::M517
                | Self::M337
                | Self::M350
                | Self::M354
                | Self::M415
                | Self::M428
        )
    }
}

/// A board of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiP {
    banks: BankWindows,
    kind: MultiPKind,
    /// The latched registers, their meaning per kind.
    regs: [u16; 4],
    /// Mapper 517's ADC: state, and the two counters it reads out.
    adc: [i16; 3],
    /// Mapper 525's eight 1 KiB CHR registers.
    chr: [u8; 8],
    /// The PRG-ROM size in bytes, which 338's and 341's decodes depend on.
    prg_size: usize,
    /// The NES 2.0 submapper (0 on an iNES 1.0 image), which 354's and 380's decodes read.
    submapper: u8,
    /// The header's mirroring, which 400 returns to while its outer register holds `$80`.
    header_mirroring: NametableMirroring,
}

impl MultiP {
    pub(crate) fn new(memory: &CartridgeMemory, kind: MultiPKind) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            kind,
            // `map_init_343`: the latch powers on as `$FF` (inverted, bank 0 of the last game).
            regs: match kind {
                MultiPKind::M343 => [0xFF, 0, 0, 0],
                // `map_init_350` and `map_init_400`: register 0 powers on as `$80`.
                MultiPKind::M350 | MultiPKind::M400 => [0x80, 0, 0, 0],
                // `map_init_429`: the latch powers on as 4 (32 KiB bank 1).
                MultiPKind::M429 => [0x04, 0, 0, 0],
                _ => [0; 4],
            },
            adc: [0; 3],
            chr: [0; 8],
            prg_size: memory.rom().prg_rom().len(),
            header_mirroring: memory.rom().header().mirroring,
            submapper: {
                let header = memory.rom().header();
                if header.is_nes20 {
                    u8::try_from(header.submapper).unwrap_or(0)
                } else {
                    0
                }
            },
        };
        let _ = board.map();
        board
    }

    /// A CPU write the board decodes, `$4020-$FFFF`.
    fn latch(&mut self, address: u16, value: u8) {
        let r = &mut self.regs;
        match self.kind {
            // `extcl_cpu_wr_mem_081`: the whole address is the register.
            // `extcl_cpu_wr_mem_237`: with bit 1 of the address register set (the lock), only the
            // inner three bits of the data register move.
            MultiPKind::M237 => {
                if r[0] & 0x02 != 0 {
                    r[1] = (r[1] & 0xF8) | u16::from(value & 0x07);
                } else {
                    r[0] = address;
                    r[1] = u16::from(value);
                }
            }
            // `extcl_cpu_wr_mem_389`: by the top address nibble.
            MultiPKind::M389 => match address & 0xF000 {
                0x8000 => r[0] = address & 0xFF,
                0x9000 => r[1] = address & 0xFF,
                _ => r[2] = address & 0x0F,
            },
            // `extcl_cpu_wr_mem_517`: a write to `$8000-$8FFF` samples the microphone (silent
            // here: puNES's own TODO, FIDELITY-LEADS J37), every write is the bank.
            MultiPKind::M517 => {
                if (0x8000..=0x8FFF).contains(&address) {
                    let data = 0i16;
                    self.adc[1] = data >> 2;
                    self.adc[2] = 0x40 - self.adc[1] - ((data & 0x03) << 2);
                    self.adc[0] = 0;
                }
                r[0] = u16::from(value);
            }
            MultiPKind::M122
            | MultiPKind::M271
            | MultiPKind::M381
            | MultiPKind::M415
            | MultiPKind::M429 => r[0] = u16::from(value),
            // `extcl_cpu_wr_mem_335`: by A13-A14.
            MultiPKind::M335 => match address & 0xE000 {
                0x8000 | 0xA000 => r[0] = u16::from(value),
                _ => r[1] = u16::from(value),
            },
            MultiPKind::M81 | MultiPKind::M338 | MultiPKind::M341 | MultiPKind::M380 => {
                r[0] = address;
            }
            // `extcl_cpu_wr_mem_382`: the data already ANDed with the ROM (`has_bus_conflicts`).
            MultiPKind::M382 => {
                if r[0] & 0x0020 == 0 {
                    r[0] = address;
                }
                r[1] = u16::from(value);
            }
            MultiPKind::M390 => r[usize::from((address >> 14) & 0x01)] = address & 0xFF,
            MultiPKind::M354 => {
                let start = if self.submapper == 1 { 0xE000 } else { 0xF000 };
                if address >= start {
                    r[0] = address;
                    r[1] = u16::from(value);
                }
            }
            MultiPKind::M343 => r[0] = u16::from(!value),
            // `extcl_cpu_wr_mem_337`: `$8000-$BFFF` the high five bits, `$C000-$FFFF` the low three.
            MultiPKind::M337 => {
                r[0] = if address < 0xC000 {
                    (r[0] & 0x07) | u16::from(value & !0x07)
                } else {
                    (r[0] & !0x07) | u16::from(value & 0x07)
                };
            }
            MultiPKind::M350 => match address & 0xE000 {
                0x8000 | 0xA000 => r[0] = u16::from(value),
                _ => r[1] = u16::from(value),
            },
            // `extcl_cpu_wr_mem_375`: A11 of the latched address locks it.
            MultiPKind::M375 => {
                if r[0] & 0x0800 == 0 {
                    r[0] = address;
                }
                r[1] = u16::from(value);
            }
            MultiPKind::M55 | MultiPKind::M352 | MultiPKind::M360 => {}
            _ => self.latch_late(address, value),
        }
    }

    /// [`MultiP::latch`] for the kinds of batch 8.
    fn latch_b8(&mut self, address: u16, value: u8) {
        let r = &mut self.regs;
        match self.kind {
            MultiPKind::M433 | MultiPKind::M481 => r[0] = u16::from(value),
            // `extcl_cpu_wr_mem_434`/`_437`: the outer register below `$8000` (437's from the
            // address), the inner one above with bus conflicts (`has_bus_conflicts`).
            MultiPKind::M434 => r[usize::from(address >= 0x8000)] = u16::from(value),
            MultiPKind::M437 => {
                if address >= 0x8000 {
                    r[1] = u16::from(value);
                } else {
                    r[0] = address;
                }
            }
            MultiPKind::M438 => {
                r[0] = address;
                r[1] = u16::from(value);
            }
            MultiPKind::M538 => {
                if (0xC000..=0xDFFF).contains(&address) {
                    r[0] = u16::from(value);
                }
            }
            MultiPKind::M541 => {
                if address >= 0xC000 {
                    r[0] = address;
                }
            }
            // `extcl_cpu_wr_mem_564`: with D5 set only D0-D2 and D4 move (D0-D1 and D4 when D3
            // is clear too).
            // `extcl_cpu_wr_mem_525`: by the top address nibble.
            MultiPKind::M525 => match address & 0xF000 {
                0x8000 => r[0] = u16::from(value),
                0x9000 => r[1] = u16::from(value),
                0xB000 => self.chr[usize::from(address & 0x07)] = value,
                _ => {}
            },
            MultiPKind::M564 => {
                let v = u16::from(value);
                r[0] = if r[0] & 0x20 == 0 {
                    v
                } else if r[0] & 0x08 != 0 {
                    (r[0] & 0xE8) | (v & 0x17)
                } else {
                    (r[0] & 0xEC) | (v & 0x13)
                };
            }
            _ => {}
        }
    }

    /// [`MultiP::latch`] for the kinds of batches 6 and 7.
    fn latch_late(&mut self, address: u16, value: u8) {
        let r = &mut self.regs;
        match self.kind {
            MultiPKind::M396 => {
                r[usize::from((0xA000..=0xBFFF).contains(&address))] = u16::from(value);
            }
            // `extcl_cpu_wr_mem_400`: the data latch's value is already ANDed with the ROM
            // (`has_bus_conflicts`); the LED (`$8000-$BFFF`) is not modelled.
            MultiPKind::M400 => match address & 0xF000 {
                0x7000 => r[0] = u16::from(value),
                0xC000..=0xF000 => r[1] = u16::from(value),
                _ => {}
            },
            // `extcl_cpu_wr_mem_403`: `$4100-$4103` the registers, `$8000-$FFFF` the CHR bank
            // while `$4102` bit 2 allows it.
            MultiPKind::M403 => {
                if address < 0x8000 {
                    r[usize::from(address & 0x03)] = u16::from(value);
                } else if r[2] & 0x04 != 0 {
                    r[1] = u16::from(value);
                }
            }
            MultiPKind::M409 => {
                if (0xC000..=0xCFFF).contains(&address) {
                    r[0] = address;
                }
            }
            // `extcl_cpu_wr_mem_414`: the data latch is ANDed with the ROM (`has_bus_conflicts`).
            MultiPKind::M414 => {
                r[0] = address;
                r[1] = u16::from(value);
            }
            // `extcl_cpu_wr_mem_428`: `$6000-$6003` four registers of which the fixes read `$6001`
            // and `$6002` (kept in slots 1-2); the `$8000` latch takes slot 3.
            MultiPKind::M428 => {
                if address >= 0x8000 {
                    r[3] = u16::from(value);
                } else if matches!(address & 0x03, 1 | 2) {
                    r[usize::from(address & 0x03)] = u16::from(value);
                }
            }
            // `extcl_cpu_wr_mem_431`: `$8000-$BFFF` and `$C000-$FFFF`, with bus conflicts.
            MultiPKind::M431 => r[usize::from(address >= 0xC000)] = u16::from(value),
            _ => self.latch_b8(address, value),
        }
    }

    /// The banks the registers select, and the mirroring when the board drives it.
    fn map(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M55 => {
                b.map_prg_32k(0);
                None
            }
            MultiPKind::M81 => {
                b.map_prg_16k(0, r[0] >> 2);
                b.map_prg_16k(2, -1);
                b.map_chr_8k(r[0] & 0x03);
                None
            }
            MultiPKind::M122 => {
                b.map_prg_32k(0);
                b.map_chr_4k(0, r[0] & 0x0F);
                b.map_chr_4k(4, r[0] >> 4);
                None
            }
            // `prg_fix_237`: an outer 128 KiB block from A2 and D3-D4, four inner modes in D6-D7.
            MultiPKind::M237 => {
                let base = ((r[0] & 0x04) << 3) | (r[1] & 0x18);
                let (first, second) = match r[1] & 0xC0 {
                    0x00 => (base | (r[1] & 0x07), base | 0x07),
                    0x40 => (base | (r[1] & 0x06), base | 0x07),
                    0x80 => (base | (r[1] & 0x07), base | (r[1] & 0x07)),
                    _ => (base | (r[1] & 0x06), base | (r[1] & 0x06) | 0x01),
                };
                b.map_prg_16k(0, first);
                b.map_prg_16k(2, second);
                b.map_chr_8k(0);
                mirror(r[1] & 0x20 == 0)
            }
            // `prg_fix_389`/`chr_fix_389`: UNROM-like mode with `$9000` bit 1, NROM-256 otherwise.
            MultiPKind::M389 => {
                if r[1] & 0x02 != 0 {
                    b.map_prg_16k(0, (r[0] >> 2) | ((r[2] & 0x0C) >> 2));
                    b.map_prg_16k(2, (r[0] >> 2) | 0x03);
                } else {
                    b.map_prg_32k(r[0] >> 3);
                }
                b.map_chr_8k(((r[1] & 0x38) >> 1) | (r[2] & 0x03));
                mirror(r[0] & 0x01 == 0)
            }
            MultiPKind::M517 => {
                b.map_prg_16k(0, r[0]);
                b.map_prg_16k(2, -1);
                None
            }
            _ => self.map_b2(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 2 (`docs/progress/MAPPERS-P.md`).
    fn map_b2(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M271 => {
                b.map_prg_32k(r[0] >> 4);
                b.map_chr_8k(r[0] & 0x0F);
                mirror(r[0] & 0x20 != 0)
            }
            // `prg_fix_335`: D4 picks NROM-128 (bank D0-D2 with D3 as its low bit) over NROM-256.
            MultiPKind::M335 => {
                if r[1] & 0x10 != 0 {
                    let bank = ((r[1] & 0x07) << 1) | ((r[1] & 0x08) >> 3);
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                } else {
                    b.map_prg_32k(r[1] & 0x07);
                }
                b.map_chr_8k(r[0]);
                mirror(r[1] & 0x20 == 0)
            }
            MultiPKind::M338 => {
                b.map_prg_16k(0, r[0]);
                b.map_prg_16k(2, r[0]);
                b.map_chr_8k(r[0]);
                mirror(r[0] & 0x08 != 0)
            }
            // `mirroring_fix_341`: the mirroring bit is A11 on a 256 KiB image, A9 otherwise.
            MultiPKind::M341 => {
                b.map_prg_32k(r[0] >> 8);
                b.map_chr_8k(r[0] >> 8);
                let bit = if self.prg_size & 0x4_0000 != 0 {
                    0x800
                } else {
                    0x200
                };
                mirror(r[0] & bit == 0)
            }
            MultiPKind::M343 => {
                b.map_prg_32k(r[0]);
                b.map_chr_8k(r[0]);
                mirror(r[0] & 0x80 != 0)
            }
            // `prg_fix_352`: the reset count is the 32 KiB bank, the CHR bank and the mirroring.
            MultiPKind::M352 => {
                b.map_prg_32k(r[3]);
                b.map_chr_8k(r[3]);
                mirror(r[3] & 0x01 != 0)
            }
            // `prg_fix_360` with the DIP switch at 0 (J37): NROM-256 bank 0, CHR bank 0, vertical.
            MultiPKind::M360 => {
                b.map_prg_32k(0);
                b.map_chr_8k(0);
                mirror(true)
            }
            // `prg_fix_381`: D0-D3 rotated left (D3 → bit 0), the reset count as the outer bank.
            MultiPKind::M381 => {
                let outer = r[3] << 4;
                b.map_prg_16k(0, (((r[0] << 1) | (r[0] >> 4)) & 0x0F) | outer);
                b.map_prg_16k(2, outer | 0x0F);
                None
            }
            _ => self.map_b3(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 3 (`docs/progress/MAPPERS-P.md`).
    fn map_b3(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M337 => {
                match (r[0] >> 6) & 0x03 {
                    0 => {
                        b.map_prg_16k(0, r[0]);
                        b.map_prg_16k(2, r[0]);
                    }
                    1 => b.map_prg_32k(r[0] >> 1),
                    _ => {
                        b.map_prg_16k(0, r[0]);
                        b.map_prg_16k(2, r[0] | 0x07);
                    }
                }
                mirror(r[0] & 0x20 == 0)
            }
            // `prg_fix_350`: modes 2-3 use the second chip (128 KiB, A20 from bit 5).
            MultiPKind::M350 => {
                let mut bank = (r[0] & 0x18) | (r[1] & 0x07);
                match (r[0] >> 5) & 0x03 {
                    0 => {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank);
                    }
                    1 => b.map_prg_32k(bank >> 1),
                    _ => {
                        if r[0] & 0x20 != 0 {
                            bank &= 0x07;
                        }
                        let base = r[0] & 0x20;
                        b.map_prg_16k(0, base | bank);
                        b.map_prg_16k(2, base | bank | 0x07);
                    }
                }
                mirror(r[0] & 0x80 == 0)
            }
            // `prg_fix_375`, its bit arithmetic kept as puNES writes it.
            MultiPKind::M375 => {
                let a = r[0];
                let bank = ((a & 0x400) >> 4) | ((a & 0x100) >> 3) | ((a & 0xF80) >> 2);
                let bit0 = a & 0x0001;
                let bit7 = (a & 0x0080) >> 7;
                let bit9 = (a & 0x0200) >> 9;
                let bit11 = (a & 0x0800) >> 11;
                let low_clear = i32::from(bit7 == 0 && bit9 == 0) * 7;
                b.map_prg_16k(0, ((bank & !bit0) & !(bit11 * 7)) | (bit11 * r[1]));
                b.map_prg_16k(2, ((bank | bit0) & !low_clear) | (bit7 * bit9 * 7));
                mirror(a & 0x02 == 0)
            }
            _ => self.map_b4(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 4 (`docs/progress/MAPPERS-P.md`).
    fn map_b4(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            // `prg_fix_354`: A12, A4 and D0-D5 the 16 KiB bank, D7 the 8 KiB half, A0-A2 the mode.
            MultiPKind::M354 => {
                let bank = Self::bank_354(r[0], r[1]);
                let half = (bank << 1) | (r[1] >> 7);
                match r[0] & 0x07 {
                    1 => {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank | 0x07);
                    }
                    2 | 6 => {
                        for slot in 0..4 {
                            b.map_prg_8k(slot, half);
                        }
                    }
                    3 | 7 => {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank);
                    }
                    5 => b.map_prg_32k((bank >> 1) | 0x03),
                    _ => b.map_prg_32k(bank >> 1),
                }
                mirror(r[1] & 0x40 == 0)
            }
            // `prg_fix_380`, its bit arithmetic kept as puNES writes it.
            MultiPKind::M380 => {
                let bit0 = i32::from(r[0] & 0x0001 == 0);
                let bit8 = (r[0] & 0x0100) >> 8;
                let bit9 = (r[0] & 0x0200) >> 9;
                let not9 = i32::from(bit9 == 0);
                let bank = ((r[0] & 0x007C) >> 2) & !(bit0 * bit9);
                b.map_prg_16k(0, bank);
                let extra = if self.submapper == 1 {
                    8 * not9 * bit8
                } else {
                    0
                };
                b.map_prg_16k(2, bank | (bit0 * bit9) | (7 * not9) | extra);
                let bit = if self.submapper == 2 { 0x040 } else { 0x002 };
                mirror(r[0] & bit == 0)
            }
            MultiPKind::M382 => {
                let outer = r[0] & 0x07;
                if r[0] & 0x0008 != 0 {
                    b.map_prg_32k((outer << 2) | (r[1] & 0x03));
                } else {
                    b.map_prg_16k(0, (outer << 3) | (r[1] & 0x07));
                    b.map_prg_16k(2, (outer << 3) | 0x07);
                }
                mirror(r[0] & 0x0010 == 0)
            }
            // `prg_fix_390`: the second register's D4-D5 the mode (its DIP read ORs 0: J37).
            MultiPKind::M390 => {
                let bank = r[1];
                match r[1] & 0x30 {
                    0x20 => b.map_prg_32k(bank >> 1),
                    0x30 => {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank);
                    }
                    _ => {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank | 0x07);
                    }
                }
                b.map_chr_8k(r[0]);
                mirror(r[0] & 0x20 == 0)
            }
            _ => self.map_b6(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 6 (`docs/progress/MAPPERS-P.md`).
    fn map_b6(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M396 => {
                let base = (r[1] & 0x0F) << 3;
                b.map_prg_16k(0, base | (r[0] & 0x07));
                b.map_prg_16k(2, base | 0x07);
                mirror(r[1] & 0x60 == 0)
            }
            // `mirroring_fix_400`: `$80` in the outer register leaves the header's mirroring.
            MultiPKind::M400 => {
                b.map_prg_16k(0, (r[0] & 0xF8) | (r[1] & 0x07));
                b.map_prg_16k(2, (r[0] & 0xF8) | 0x07);
                b.map_chr_8k(r[1] >> 5);
                if r[0] == 0x80 {
                    Some(self.header_mirroring)
                } else {
                    mirror(r[0] & 0x20 == 0)
                }
            }
            MultiPKind::M403 => {
                let bank = (r[0] & 0x7E) >> 1;
                if r[2] & 0x01 != 0 {
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                } else {
                    b.map_prg_32k(bank >> 1);
                }
                b.map_chr_8k(r[1] & 0x03);
                mirror(r[2] & 0x10 == 0)
            }
            MultiPKind::M409 => {
                b.map_prg_16k(0, r[0]);
                b.map_prg_16k(2, -1);
                None
            }
            _ => self.map_b7(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 7 (`docs/progress/MAPPERS-P.md`).
    fn map_b7(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            // `prg_fix_414` with the DIP switch at 0, so its `$C000` disable never applies (J37).
            MultiPKind::M414 => {
                let bank = r[0] >> 1;
                if r[0] & 0x2000 != 0 {
                    b.map_prg_32k(bank >> 1);
                } else {
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                }
                b.map_chr_8k(r[1]);
                mirror(r[0] & 0x01 == 0)
            }
            MultiPKind::M415 => {
                b.map_prg_32k(-1);
                mirror(r[0] & 0x10 == 0)
            }
            // `chr_fix_428`: `$6002` bits 6-7 pick which CHR bits come from the `$8000` latch.
            MultiPKind::M428 => {
                let bank = r[1] >> 5;
                if r[1] & 0x10 != 0 {
                    b.map_prg_32k(bank >> 1);
                } else {
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                }
                let latch_bits = r[2] >> 6;
                b.map_chr_8k(((r[1] & 0x07) & !latch_bits) | (r[3] & latch_bits));
                mirror(r[1] & 0x08 == 0)
            }
            MultiPKind::M429 => {
                b.map_prg_32k((r[0] & 0x3C) >> 2);
                b.map_chr_8k(r[0] & 0x03);
                None
            }
            MultiPKind::M431 => {
                let base = (r[0] & 0x20) >> 2;
                b.map_prg_16k(0, base | (r[1] & 0x07));
                b.map_prg_16k(2, base | 0x07);
                mirror(r[0] & 0x01 == 0)
            }
            _ => self.map_b8(),
        }
    }

    /// [`MultiP::map`] for the kinds of batch 8 (`docs/progress/MAPPERS-P.md`).
    fn map_b8(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M433 => {
                let bank = r[0] & 0x1F;
                if r[0] & 0x20 == 0 {
                    b.map_prg_32k(bank >> 1);
                } else {
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                }
                mirror(r[0] & 0x40 == 0)
            }
            MultiPKind::M434 => {
                let base = (r[0] & 0x07) << 3;
                b.map_prg_16k(0, base | (r[1] & 0x07));
                b.map_prg_16k(2, base | 0x07);
                mirror(r[0] & 0x20 != 0)
            }
            MultiPKind::M437 => {
                let base = (r[0] & 0x0F) << 3;
                b.map_prg_16k(0, base | (r[1] & 0x07));
                b.map_prg_16k(2, base | 0x07);
                mirror(r[0] & 0x08 == 0)
            }
            MultiPKind::M438 => {
                let bank = r[0] >> 1;
                b.map_prg_16k(0, bank & !(r[0] & 0x01));
                b.map_prg_16k(2, bank | (r[0] & 0x01));
                b.map_chr_8k(r[1] >> 1);
                mirror(r[1] & 0x01 == 0)
            }
            MultiPKind::M481 => {
                let bank = (r[0] & 0x80) >> 4;
                b.map_prg_16k(0, bank | (r[0] & 0x07));
                b.map_prg_16k(2, bank | 0x07);
                None
            }
            // `prg_fix_538`: D0 set with D3 clear shows bank `$0A` at `$8000`.
            MultiPKind::M538 => {
                let low = if r[0] & 0x01 != 0 && r[0] & 0x08 == 0 {
                    0x0A
                } else {
                    r[0] & 0xFE
                };
                b.map_prg_8k(0, low);
                b.map_prg_8k(1, 0x0D);
                b.map_prg_8k(2, 0x0E);
                b.map_prg_8k(3, 0x0F);
                None
            }
            MultiPKind::M541 => {
                if r[0] & 0x02 != 0 {
                    let bank = (r[0] & 0xFF) >> 2;
                    b.map_prg_16k(0, bank);
                    b.map_prg_16k(2, bank);
                } else {
                    b.map_prg_32k((r[0] & 0xFF) >> 3);
                }
                mirror(r[0] & 0x01 != 0)
            }
            MultiPKind::M525 => {
                b.map_prg_16k(0, r[0] >> 1);
                b.map_prg_16k(2, -1);
                for (slot, bank) in self.chr.iter().enumerate() {
                    b.map_chr_1k(slot, i32::from(*bank));
                }
                mirror(r[1] & 0x01 == 0)
            }
            // `prg_fix_554`/`chr_fix_554`: PRG fixed to banks `$0A`, `$0B`, 6 and 7; the latch is
            // the CHR bank.
            MultiPKind::M554 => {
                b.map_prg_8k(0, 0x0A);
                b.map_prg_8k(1, 0x0B);
                b.map_prg_8k(2, 0x06);
                b.map_prg_8k(3, 0x07);
                b.map_chr_8k(r[0]);
                None
            }
            MultiPKind::M564 => {
                b.map_prg_32k(r[0]);
                Some(match r[0] & 0x30 {
                    0x00 => NametableMirroring::Vertical,
                    0x10 => NametableMirroring::Horizontal,
                    0x20 => NametableMirroring::SingleScreenA,
                    _ => NametableMirroring::SingleScreenB,
                })
            }
            _ => None,
        }
    }

    /// Mapper 354's 16 KiB bank: A12 → bit 7, A4 → bit 6, D0-D5.
    const fn bank_354(address: i32, data: i32) -> i32 {
        ((address & 0x1000) >> 5) | ((address & 0x10) << 2) | (data & 0x3F)
    }

    /// Whether CHR-RAM writes are blocked now (`memmap_vram_wp_8k`'s write-enable argument).
    const fn chr_write_protected(&self) -> bool {
        match self.kind {
            MultiPKind::M375 | MultiPKind::M380 => self.regs[0] & 0x80 != 0,
            MultiPKind::M354 => self.regs[0] & 0x08 != 0,
            // `chr_fix_433`: D7 is the write enable.
            MultiPKind::M433 => self.regs[0] & 0x80 == 0,
            _ => false,
        }
    }

    /// Whether reads of `$8000-$FFFF` cannot come from the cached windows now.
    const fn reads_are_special(&self) -> bool {
        match self.kind {
            MultiPKind::M237 => self.regs[0] & 0x01 != 0,
            // `extcl_cpu_rd_mem_338`: `$F004` (on a 64 KiB image or smaller) and `$F008` latched
            // read the DIP switches.
            MultiPKind::M338 => {
                let latched = self.regs[0] & 0xFF0F;
                latched == 0xF008 || (latched == 0xF004 && self.prg_size <= 0x1_0000)
            }
            // `extcl_cpu_rd_mem_554`: every read of `$8000-$FFFF` may be a trigger.
            MultiPKind::M554 => true,
            // `extcl_cpu_rd_mem_380`: submapper 0 with A8 latched reads `(A0-A7) | DIP`.
            MultiPKind::M380 => self.submapper == 0 && self.regs[0] & 0x100 != 0,
            _ => false,
        }
    }

    /// A read of `$6000-$7FFF` the board answers itself, with its side effect: `Some(None)` is
    /// open bus (the `Mapper::cpu_read_override` shape).
    #[allow(clippy::option_option)]
    fn read_6000(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        match self.kind {
            // `wram_fix_055`: `$6000-$6FFF` PRG-ROM 4 KiB bank 8 (2 KiB mirrored on the board),
            // `$7000-$7FFF` work RAM.
            MultiPKind::M55 if address < 0x7000 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                Some(
                    rom.get((0x8000 + usize::from(address & 0x0FFF)) % rom.len())
                        .copied(),
                )
            }
            MultiPKind::M55 => {
                let ram = memory.prg_ram();
                (!ram.is_empty()).then(|| Some(ram[usize::from(address & 0x0FFF) % ram.len()]))
            }
            // `extcl_cpu_rd_mem_428`: open bus with the two DIP bits (0) below; this core's open
            // bus is whole, so the read is open bus (J37).
            MultiPKind::M122 | MultiPKind::M428 | MultiPKind::M434 => Some(None),
            // `wram_fix_415`: `$6000-$7FFF` an 8 KiB PRG-ROM bank from D0-D3.
            MultiPKind::M415 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                let bank = usize::from(self.regs[0] & 0x0F);
                Some(
                    rom.get((bank * 0x2000 + usize::from(address & 0x1FFF)) % rom.len())
                        .copied(),
                )
            }
            // `wram_fix_354`: mode 5 shows the 8 KiB half bank, every other mode disables it.
            MultiPKind::M354 => {
                let (a, d) = (i32::from(self.regs[0]), i32::from(self.regs[1]));
                if a & 0x07 != 5 {
                    return Some(None);
                }
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                let bank = usize::try_from((Self::bank_354(a, d) << 1) | (d >> 7)).unwrap_or(0);
                Some(
                    rom.get((bank * 0x2000 + usize::from(address & 0x1FFF)) % rom.len())
                        .copied(),
                )
            }
            // `wram_fix_538`: `$6000-$7FFF` PRG-ROM 8 KiB bank `latch | 1`; `wram_fix_554`: the
            // latch itself.
            MultiPKind::M538 | MultiPKind::M554 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                let bank = usize::from(if self.kind == MultiPKind::M538 {
                    self.regs[0] | 0x01
                } else {
                    self.regs[0]
                });
                Some(
                    rom.get((bank * 0x2000 + usize::from(address & 0x1FFF)) % rom.len())
                        .copied(),
                )
            }
            // `wram_fix_337`/`wram_fix_350`: `$6000-$7FFF` is PRG-ROM 8 KiB bank 1.
            MultiPKind::M337 | MultiPKind::M350 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                Some(
                    rom.get((0x2000 + usize::from(address & 0x1FFF)) % rom.len())
                        .copied(),
                )
            }
            // `extcl_cpu_rd_mem_517`: the ADC's serial read-out.
            MultiPKind::M517 if address < 0x7000 => {
                let bit = if address == 0x6000 {
                    match self.adc[0] {
                        0 => {
                            self.adc[0] = 1;
                            0
                        }
                        1 => {
                            self.adc[0] = 2;
                            1
                        }
                        _ => {
                            if self.adc[2] > 0 {
                                self.adc[2] -= 1;
                                1
                            } else {
                                self.adc[0] = 0;
                                0
                            }
                        }
                    }
                } else {
                    let high = self.adc[1];
                    self.adc[1] = high.saturating_sub(1);
                    u8::from(high <= 0)
                };
                Some(Some(bit))
            }
            _ => None,
        }
    }
}

impl Mapper for MultiP {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        for reg in self.regs {
            writer.write_u16(reg);
        }
        for value in self.adc {
            writer.write_i16(value);
        }
        writer.write_raw(&self.chr);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for reg in &mut self.regs {
            *reg = reader.read_u16()?;
        }
        for value in &mut self.adc {
            *value = reader.read_i16()?;
        }
        reader.read_into(&mut self.chr)?;
        let _ = self.map();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        let kind = self.kind;
        *self = Self::new(memory, kind);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
    }

    /// 352 and 381 count the console's resets (`map_init_352`/`map_init_381` on `RESET`): the
    /// count picks the game. The other kinds keep the default, nothing.
    fn soft_reset(&mut self, memory: &mut CartridgeMemory) {
        if matches!(self.kind, MultiPKind::M352 | MultiPKind::M381) {
            self.regs[3] = self.regs[3].wrapping_add(1) & 0xFF;
            if let Some(mirroring) = self.map() {
                memory.set_mirroring(mirroring);
            }
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        !self.kind.owns_6000()
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= 0x8000 || !self.kind.decodes_low(address) {
            // Mapper 55's `$7000-$7FFF` is RAM the board serves itself.
            if self.kind == MultiPKind::M55 && (0x7000..=0x7FFF).contains(&address) {
                let ram = memory.prg_ram_mut();
                if !ram.is_empty() {
                    let len = ram.len();
                    ram[usize::from(address & 0x0FFF) % len] = value;
                }
                return true;
            }
            return false;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.chr_write_protected() || !memory.rom().chr_rom().is_empty() {
            return;
        }
        self.banks.write_chr(address, value, memory.chr_ram_mut());
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.reads_are_special() {
            return [NO_ROM_WINDOW; 4];
        }
        self.banks.rom_windows(memory.rom().prg_rom().len())
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        // `extcl_cpu_rd_mem_554`: the trigger addresses latch A2-A5, the read is ROM.
        if self.kind == MultiPKind::M554 && address >= 0x8000 {
            let even = address & 0xFFFE;
            let trigger = match address & 0xF000 {
                0xC000 => (0xCAB6..=0xCAD7).contains(&address),
                0xE000 => even == 0xEBE2 || even == 0xEE32,
                0xF000 => even == 0xFFFC,
                _ => false,
            };
            if trigger {
                self.regs[0] = (address & 0x3C) >> 2;
                let _ = self.map();
            }
            return Some(
                self.banks
                    .read_prg(address, memory.rom().prg_rom(), memory.prg_ram()),
            );
        }
        if (0x6000..=0x7FFF).contains(&address) && self.kind.owns_6000() {
            return self.read_6000(memory, address);
        }
        self.cpu_peek_override(memory, address)
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if (0x6000..=0x7FFF).contains(&address) && self.kind.owns_6000() {
            if self.kind == MultiPKind::M517 && address < 0x7000 {
                return Some(Some(0));
            }
            return self.clone().read_6000(memory, address);
        }
        // `extcl_cpu_rd_mem_237`: with A0 of the address register set, `$8000-$FFFF` reads the
        // DIP switches (none set here: J37); 338's two DIP reads the same way.
        if address >= 0x8000 && self.reads_are_special() {
            // 380's DIP read returns the low address byte ORed with the switches (0: J37).
            if self.kind == MultiPKind::M380 {
                return Some(Some(address.to_le_bytes()[0]));
            }
            if self.kind == MultiPKind::M554 {
                return Some(self.banks.read_prg(
                    address,
                    memory.rom().prg_rom(),
                    memory.prg_ram(),
                ));
            }
            return Some(Some(0));
        }
        None
    }
}
