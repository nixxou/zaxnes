//! Lot MAPPERS-P: discrete boards with a CPU-cycle IRQ counter (`docs/progress/MAPPERS-P.md`),
//! most of them *Super Mario Bros. 2* (Japan) conversions, whose FDS timer becomes an M2 counter.
//!
//! Read from puNES 0.111 (`mapper_311.c`, `mapper_357.c`, `mapper_368.c`, `mapper_416.c`,
//! `mapper_417.c`; read, not copied). An [`super::IrqBoard`] variant, because the counter runs on
//! [`Mapper::on_cpu_cycle`] and the line on [`Mapper::irq_asserted`], which that enum forwards.
//! The PRG-ROM these boards show at `$5000`/`$6000` is answered by [`Mapper::try_read_prg`], with
//! the work RAM disabled ([`Mapper::prg_ram_enabled`]).
use super::vrc_irq::VrcIrq;
use super::{windows_for, Eeprom93C66, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// 560's nametable memory: the console's 2 KiB it serves itself, then the 1 KiB extension RAM.
const NT_560: usize = 0xC00;

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqPKind {
    /// 311, `mapper_311.c` (*SMB2J* conversion): `$4022` the 32 KiB bank, `$4122` the IRQ, fixed
    /// PRG-ROM at `$5000` (4 KiB bank 17) and `$6000` (8 KiB bank 9).
    M311,
    /// 357, `mapper_357.c` (Bit Corp 4-in-1, *SMB2J* first): the DIP switch picks the game; at 0
    /// the *SMB2J* layout from `$4022` and `$4120`.
    M357,
    /// 368, `mapper_368.c` (YUNG-08, *SMB2J*): `$4022` the `$C000` bank, `$4122` the IRQ and a
    /// readable status.
    M368,
    /// 416, `mapper_416.c` (4-in-1 N-32): `$4020` the *SMB2J* bank, `$4120` the IRQ, `$8000` a
    /// latch with NROM modes.
    M416,
    /// 417, `mapper_417.c` (*Batman* pirate): 8 KiB PRG and 1 KiB CHR registers, a CIRAM page
    /// per nametable, a CPU-cycle IRQ.
    M417,
    /// 471, `mapper_471.c` (*Impact Soft* IM1): the written address is a 32 KiB PRG and an 8 KiB
    /// CHR bank, and every PPU A12 rise asserts the IRQ, which any write acknowledges.
    M471,
    /// 526, `mapper_526.c` (BJ-56, a *Gradius II* bootleg): four 8 KiB PRG and eight 1 KiB CHR
    /// registers at `$8000-$800B`, a free-running CPU-cycle counter whose bit 12 asserts the
    /// IRQ, `$800D` restarting it and `$800F` acknowledging.
    M526,
    /// 558, `mapper_558.c` (Yancheng YC-03-09, the PEC-9588's later version): a 32 KiB bank from
    /// `$5000`/`$5100` (D0 and D1 swapped while `$5300` D0 is clear) and, when the NES 2.0
    /// header declares 512 bytes of PRG-NVRAM, a 93C66 EEPROM on `$5200` read back at `$5000`.
    /// No IRQ: it sits here for the boxed variant, not for a counter.
    M558,
    /// 272, `mapper_272.c` (J.Y. *Akumajō Special* bootleg): two 8 KiB PRG registers, VRC-style
    /// nibble CHR registers, and an IRQ counting the rendering's tile slots, asserting at 84
    /// (every second rendered line).
    M272,
    /// 442, `mapper_442.c` (a PEC-9588 relative): `$5000` the 32 KiB bank and the 1 bpp mode,
    /// in which a pattern fetch takes CHR A3 and A12 from PPU A0 and A9 latched on the last rise
    /// of PPU A13. No IRQ.
    M442,
    /// 359, `mapper_359.c` (BS-400R / BS-4040R): four 8 KiB PRG registers (the fourth at
    /// `$6000`) with an outer bank and mask, eight 1 KiB CHR registers with an outer bank and
    /// mask, and an IRQ counting CPU cycles or (mode bit) filtered A12 rises.
    M359,
    /// 540, `mapper_359.c` too (the *Master Fighter VI'* board): 359 with four 2 KiB CHR banks
    /// from registers 0, 1, 6 and 7.
    M540,
    /// 528, `mapper_528.c` (1201, a *Gimmick!* bootleg): the FME-7's command/data registers
    /// written in one go at `$A000`/`$C000` (A0-A3 the command, `$C000` adding an outer bank),
    /// with a VRC7's IRQ on commands `$D`-`$F` and command `$B` ignored.
    M528,
    /// 560, `mapper_560.c` (a *Mahjong* board): any write toggles the 32 KiB bank; a write to
    /// tables 1 or 3 is copied to a 1 KiB extension RAM, and with the bank at 1 each pattern
    /// fetch reads the CHR-ROM bank that RAM holds for the tile the last nametable fetch named.
    M560,
    /// 413, `mapper_413.c` (*Super Mario Bros. 3* / *Batman* bootleg): `$E000-$FFFF` loads four
    /// bank registers (D6-D7 the register), an MMC3-style A12 counter at `$8000-$BFFF`, and a
    /// serial read-out of the NES 2.0 misc ROM at `$4800-$4FFF` and `$C000-$CFFF`.
    M413,
}

impl IrqPKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            311 => Self::M311,
            357 => Self::M357,
            368 => Self::M368,
            416 => Self::M416,
            417 => Self::M417,
            471 => Self::M471,
            526 => Self::M526,
            558 => Self::M558,
            272 => Self::M272,
            442 => Self::M442,
            359 => Self::M359,
            540 => Self::M540,
            528 => Self::M528,
            560 => Self::M560,
            413 => Self::M413,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M311 => 311,
            Self::M357 => 357,
            Self::M368 => 368,
            Self::M416 => 416,
            Self::M417 => 417,
            Self::M471 => 471,
            Self::M526 => 526,
            Self::M558 => 558,
            Self::M272 => 272,
            Self::M442 => 442,
            Self::M359 => 359,
            Self::M540 => 540,
            Self::M528 => 528,
            Self::M560 => 560,
            Self::M413 => 413,
        }
    }
}

/// A board of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrqP {
    banks: BankWindows,
    kind: IrqPKind,
    /// The registers, their meaning per kind; 417 keeps its three PRG registers in 0-2.
    regs: [u8; 4],
    /// 417's eight CHR registers and four nametable registers.
    chr: [u8; 8],
    nametables: [u8; 4],
    irq_enabled: bool,
    irq_counter: u32,
    irq_pending: bool,
    /// 558's 93C66, whose 512 bytes follow the 8 KiB of work RAM in PRG-RAM; present only when
    /// that RAM is `EEPROM_RAM_SIZE` long (`registry::board_memory_sizes`).
    eeprom: Option<Eeprom93C66>,
    /// 359's and 540's other registers (`m359`): `prg[4]`, `prg[5]`, `chr[8]`, `chr[9]`, the
    /// mirroring, the IRQ flags (D1 mode, D2 auto-enable, D3 reload), the CPU-cycle A12 filter
    /// and the last A12 level; saved for those kinds only. `regs` hold `prg[0..4]`, `chr` the
    /// eight CHR registers, `irq_counter` the 16-bit counter.
    ext_359: [u8; 8],
    /// 560's nametables and extension RAM ([`NT_560`] bytes), empty for the other kinds; saved
    /// for that kind only. `regs[0]` is its bank toggle, `regs[1]` the PPU A13 latch, and
    /// `irq_counter` the nametable address latched on A13's rise.
    nt_560: Vec<u8>,
    /// 413's misc ROM (the image past its CHR-ROM), empty for the other kinds; derived from the
    /// image, so not saved.
    misc_413: Vec<u8>,
    /// 413's serial address (`m413.serial.address`); `nametables[0]` holds its IRQ latch and
    /// `nametables[1]` the serial control. Saved for that kind only.
    serial_413: u32,
    /// 528's VRC7 IRQ (`init_VRC7(0x01, 0x00)`), saved for that kind only.
    vrc_irq: VrcIrq,
    /// Whether the image has CHR-ROM, which 359's CHR banking needs (`chrrom_size()`).
    chr_rom: bool,
    /// 471's A12 edge detector (the MMC3's filter; puNES clocks once per rendered line).
    a12: A12Watcher,
}

impl IrqP {
    pub(crate) fn new(memory: &CartridgeMemory, kind: IrqPKind) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            kind,
            regs: [0; 4],
            chr: [0; 8],
            nametables: [0; 4],
            irq_enabled: false,
            irq_counter: 0,
            irq_pending: false,
            a12: A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS),
            ext_359: [0, 0x3F, 0, 0xFF, 0, 0, 0, 0],
            chr_rom: !memory.rom().chr_rom().is_empty(),
            vrc_irq: VrcIrq::new(),
            misc_413: if kind == IrqPKind::M413 {
                let rom = memory.rom();
                rom.raw()
                    .get(rom.header().announced_size()..)
                    .unwrap_or_default()
                    .to_vec()
            } else {
                Vec::new()
            },
            serial_413: 0,
            nt_560: if kind == IrqPKind::M560 {
                vec![0; NT_560]
            } else {
                Vec::new()
            },
            eeprom: (kind == IrqPKind::M558 && memory.prg_ram().len() == Self::EEPROM_RAM_SIZE)
                .then(Eeprom93C66::new),
        };
        // `map_init_359`: PRG `$FC`, `$FD`, `$FF` and `prg[3]` 0 (puNES sets `prg[2]` twice),
        // CHR 0-7.
        if matches!(kind, IrqPKind::M359 | IrqPKind::M540) {
            board.regs = [0xFC, 0xFD, 0xFF, 0x00];
            board.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        }
        // `map_init_560`: `reg = 1`.
        if kind == IrqPKind::M560 {
            board.regs[0] = 1;
        }
        // `init_FME7`: `prg[0..4]` = 0, 0, 1, `$FE`; CHR 0-7. `nametables[0]` is the FME-7's
        // mirroring command, `nametables[1]` the outer bank (`m528.reg`).
        if kind == IrqPKind::M528 {
            board.regs = [0, 0, 0x01, 0xFE];
            board.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        }
        // `map_init_526`: PRG `$FC-$FF`, CHR 0-7.
        if kind == IrqPKind::M526 {
            board.regs = [0xFC, 0xFD, 0xFE, 0xFF];
            board.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        }
        board.map();
        board
    }

    /// The windows the registers select.
    fn map(&mut self) {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        match self.kind {
            IrqPKind::M311 => b.map_prg_32k(r[0]),
            // `prg_fix_357` with the DIP switch at 0 (J37): the *SMB2J* layout, `$4120` D0
            // picking the second table.
            IrqPKind::M357 => {
                const BANKS: [[i32; 8]; 2] = [[4, 3, 5, 3, 6, 3, 7, 3], [1, 1, 5, 1, 4, 1, 5, 1]];
                let second = r[2] != 0;
                b.map_prg_8k(0, i32::from(!second));
                b.map_prg_8k(1, 0);
                b.map_prg_8k(
                    2,
                    BANKS[usize::from(second)][usize::try_from(r[1] & 0x07).unwrap_or(0)],
                );
                b.map_prg_8k(3, if second { 8 } else { 10 });
            }
            // `prg_fix_368`: 1, 0, the `$4022` table, 8.
            IrqPKind::M368 => {
                b.map_prg_8k(0, 1);
                b.map_prg_8k(1, 0);
                b.map_prg_8k(
                    2,
                    if r[0] & 0x01 != 0 {
                        3
                    } else {
                        4 | ((r[0] & 0x06) >> 1)
                    },
                );
                b.map_prg_8k(3, 8);
            }
            // `prg_fix_416`: `$8000` D3 the NROM modes (D7 32 KiB, D6 16 KiB, else 8 KiB),
            // otherwise the *SMB2J* layout with `$4020` the `$C000` bank.
            IrqPKind::M416 => {
                if r[1] & 0x08 != 0 {
                    let bank = ((r[1] & 0x08) >> 1) | ((r[1] & 0x80) >> 6) | ((r[1] & 0x20) >> 5);
                    if r[1] & 0x80 != 0 {
                        b.map_prg_32k(bank >> 1);
                    } else if r[1] & 0x40 != 0 {
                        b.map_prg_16k(0, bank);
                        b.map_prg_16k(2, bank);
                    } else {
                        for slot in 0..4 {
                            b.map_prg_8k(slot, bank << 1);
                        }
                    }
                } else {
                    let bank = (r[0] & 0x08) | ((r[0] & 0x01) << 2) | ((r[0] & 0x06) >> 1);
                    b.map_prg_8k(0, 0);
                    b.map_prg_8k(1, 1);
                    b.map_prg_8k(2, bank);
                    b.map_prg_8k(3, 3);
                }
                b.map_chr_8k((r[1] & 0x06) >> 1);
            }
            IrqPKind::M359 | IrqPKind::M540 => self.map_359(),
            IrqPKind::M413 => self.map_413(),
            // `prg_fix_560`.
            IrqPKind::M560 => {
                b.map_prg_32k(r[0]);
                b.map_chr_8k(0);
            }
            IrqPKind::M528 => self.map_528(),
            // `prg_fix_442`: `reg[0]` D6 above D0-D4.
            IrqPKind::M442 => {
                b.map_prg_32k(((r[0] & 0x40) >> 1) | (r[0] & 0x1F));
                b.map_chr_8k(0);
            }
            // `prg_fix_272`/`chr_fix_272`: two 8 KiB registers, the last 16 KiB fixed.
            IrqPKind::M272 => {
                b.map_prg_8k(0, r[0]);
                b.map_prg_8k(1, r[1]);
                b.map_prg_16k(2, -1);
                for (slot, bank) in self.chr.iter().enumerate() {
                    b.map_chr_1k(slot, i32::from(*bank));
                }
            }
            // `prg_fix_558`: `reg[1]` above, `reg[0]` D0-D3 below, D0-D1 forced on while
            // `reg[3]` D2 is clear.
            IrqPKind::M558 => {
                let low = (r[0] & 0x0F) | if r[3] & 0x04 == 0 { 0x03 } else { 0 };
                b.map_prg_32k((r[1] << 4) | low);
                b.map_chr_8k(0);
            }
            // `prg_fix_526`/`chr_fix_526`.
            IrqPKind::M526 => {
                for (slot, bank) in r.iter().enumerate() {
                    b.map_prg_8k(slot, *bank);
                }
                for (slot, bank) in self.chr.iter().enumerate() {
                    b.map_chr_1k(slot, i32::from(*bank));
                }
            }
            // `prg_fix_471`/`chr_fix_471`: the address's low byte.
            IrqPKind::M471 => {
                b.map_prg_32k(r[0]);
                b.map_chr_8k(r[0]);
            }
            IrqPKind::M417 => {
                b.map_prg_8k(0, r[0]);
                b.map_prg_8k(1, r[1]);
                b.map_prg_8k(2, r[2]);
                b.map_prg_8k(3, -1);
                for (slot, bank) in self.chr.iter().enumerate() {
                    b.map_chr_1k(slot, i32::from(*bank));
                }
            }
        }
    }

    /// 558's PRG-RAM with the EEPROM: 8 KiB of work RAM, then the 93C66's 512 bytes.
    pub(crate) const EEPROM_RAM_SIZE: usize = 0x2000 + 512;

    /// `extcl_cpu_wr_mem_558`: `$5000`-`$5200` swap D0/D1 while `$5300` D0 is clear; `$5200` also
    /// drives the EEPROM, CS on bit 2, CLK on bit 0 and DI on bit 1 (`ee93cx6_write`).
    fn write_558(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let index = usize::from((address >> 8) & 0x03);
        let value = if index < 3 && self.regs[3] & 0x01 == 0 {
            (value & 0xFC) | ((value & 0x01) << 1) | ((value & 0x02) >> 1)
        } else {
            value
        };
        self.regs[index] = value;
        if index == 2 {
            if let Some(eeprom) = &mut self.eeprom {
                let bytes = memory.prg_ram_mut().get_mut(0x2000..).unwrap_or_default();
                eeprom.write(bytes, (value >> 2) & 1, value & 1, (value >> 1) & 1);
            }
        }
    }

    /// `extcl_cpu_wr_mem_272`: two decodes on every write, by `A15-A14` and A2-A3 (mirroring,
    /// the IRQ), then by `A15-A12` (PRG, mirroring, the CHR nibbles).
    fn latch_272(&mut self, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match (address & 0xC000, address & 0x000C) {
            (0x8000, 0x0004) => self.regs[2] = value & 0x03,
            (0x8000, 0x000C) => self.irq_pending = true,
            (0xC000, 0x0004) => self.irq_pending = false,
            (0xC000, 0x0008) => self.irq_enabled = true,
            (0xC000, 0x000C) => {
                self.irq_pending = false;
                self.irq_enabled = false;
                self.irq_counter = 0;
            }
            _ => {}
        }
        match address & 0xF000 {
            0x8000 => self.regs[0] = value,
            0x9000 => self.regs[3] = value & 0x01,
            0xA000 => self.regs[1] = value,
            0xF000 => {}
            _ => {
                let index =
                    usize::from(((address - 0xB000) >> 11) | ((address & 0x02) >> 1)) & 0x07;
                let old = self.chr[index];
                self.chr[index] = if address & 0x0001 != 0 {
                    (old & 0x0F) | ((value & 0x0F) << 4)
                } else {
                    (old & 0xF0) | (value & 0x0F)
                };
            }
        }
    }

    /// `prg_fix_359`: an 8 KiB bank through `prg[4]` D3-D5 (the outer bank) and `prg[5]` (mask).
    fn prg_359(&self, value: u8) -> u16 {
        ((u16::from(self.ext_359[0]) & 0x38) << 1) | u16::from(value & self.ext_359[1])
    }

    /// `prg_fix_359` and `chr_fix_359`.
    fn map_359(&mut self) {
        let prg = [
            self.prg_359(self.regs[0]),
            self.prg_359(self.regs[1]),
            self.prg_359(self.regs[2]),
            self.prg_359(0xFF),
        ];
        for (slot, bank) in prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, i32::from(*bank));
        }
        if !self.chr_rom {
            self.banks.map_chr_8k(0);
        } else if self.kind == IrqPKind::M540 {
            for (slot, register) in [0, 1, 6, 7].into_iter().enumerate() {
                self.banks
                    .map_chr_2k(slot * 2, i32::from(self.chr[register]));
            }
        } else {
            let base = i32::from(self.ext_359[2]) << 7;
            let mask = self.ext_359[3];
            for (slot, register) in self.chr.iter().enumerate() {
                self.banks
                    .map_chr_1k(slot, base | i32::from(register & mask));
            }
        }
    }

    /// `extcl_cpu_wr_mem_359`.
    fn latch_359(&mut self, address: u16, value: u8) {
        let x = &mut self.ext_359;
        match (address & 0xF000, address & 0x03) {
            (0x8000, a) => self.regs[usize::from(a)] = value,
            (0x9000, 0) => x[0] = value,
            (0x9000, 1) => {
                x[1] = [0x3F, 0x1F, 0x2F, 0x0F][usize::from(value & 0x03)];
                x[3] = if value & 0x40 != 0 { 0xFF } else { 0x7F };
            }
            (0x9000, 2) => x[4] = value,
            (0x9000, _) => x[2] = value,
            (0xA000 | 0xB000, a) => {
                self.chr[usize::from((address & 0x1000) >> 10) | usize::from(a)] = value;
            }
            (0xC000, a) => {
                let auto = x[5] & 0x04 != 0;
                match a {
                    0 => {
                        if auto {
                            self.irq_enabled = false;
                        }
                        self.irq_counter = (self.irq_counter & 0xFF00) | u32::from(value);
                    }
                    1 => {
                        if auto {
                            self.irq_enabled = true;
                        }
                        self.irq_counter = (self.irq_counter & 0x00FF) | (u32::from(value) << 8);
                        x[5] |= 0x08;
                    }
                    2 => {
                        self.irq_enabled = value & 0x01 != 0;
                        x[5] = (x[5] & 0x08) | (value & 0x06);
                    }
                    _ => self.irq_enabled = value & 0x01 != 0,
                }
                self.irq_pending = false;
            }
            _ => {}
        }
    }

    /// `extcl_cpu_wr_mem_528`: `$A000`/`$C000` (A0-A3 the FME-7 command), the outer bank from
    /// A14.
    fn latch_528(&mut self, address: u16, value: u8) {
        if !matches!(address & 0xF000, 0xA000 | 0xC000) {
            return;
        }
        match address & 0x0F {
            0x0B => {}
            0x0D => self.vrc_irq.write_control(value),
            0x0E => self.vrc_irq.acknowledge(),
            0x0F => self.vrc_irq.write_latch(value),
            command @ 0x00..=0x07 => self.chr[usize::from(command)] = value,
            command @ 0x08..=0x0A => self.regs[usize::from(command - 0x08)] = value,
            _ => self.nametables[0] = value,
        }
        self.nametables[1] = u8::try_from((address & 0x4000) >> 10).unwrap_or(0);
    }

    /// `extcl_cpu_wr_mem_413`.
    fn latch_413(&mut self, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 => self.nametables[0] = value,
            0x9000 => self.irq_counter = 0,
            0xA000 | 0xB000 => {
                self.irq_enabled = address & 0x1000 != 0;
                if !self.irq_enabled {
                    self.irq_pending = false;
                }
            }
            0xC000 => self.serial_413 = (self.serial_413 << 1) | u32::from(value >> 7),
            0xD000 => self.nametables[1] = value,
            0xE000 | 0xF000 => self.regs[usize::from(value >> 6)] = value & 0x3F,
            _ => {}
        }
    }

    /// `extcl_cpu_rd_mem_413`: a byte of the misc ROM at the serial address; `None` outside the
    /// port. A read (not a peek) advances the address when the control's D1 is set.
    #[allow(clippy::option_option)]
    fn read_413(&self, address: u16) -> Option<Option<u8>> {
        if self.kind != IrqPKind::M413 || !matches!(address & 0xF800, 0x4800 | 0xC000 | 0xC800) {
            return None;
        }
        let misc = &self.misc_413;
        // `miscrom_size() - 1` as the mask: a power-of-two misc ROM, as the board's chips are.
        Some((!misc.is_empty()).then(|| {
            let mask = misc.len() - 1;
            misc[usize::try_from(self.serial_413).unwrap_or(0) & mask]
        }))
    }

    /// `irq_clock_359`: in the A12 mode, past the filter, the low counter byte reloads from the
    /// high one at 0 or on a pending reload, else counts down; at 0 it asserts while enabled.
    fn clock_359(&mut self) {
        if self.ext_359[6] == 0 && self.ext_359[5] & 0x02 != 0 {
            let low = self.irq_counter & 0xFF;
            let low = if low == 0 || self.ext_359[5] & 0x08 != 0 {
                self.irq_counter >> 8
            } else {
                low - 1
            };
            self.irq_counter = (self.irq_counter & 0xFF00) | low;
            if low == 0 && self.irq_enabled {
                self.irq_pending = true;
            }
            self.ext_359[5] &= !0x08;
        }
        self.ext_359[6] = 5;
    }

    /// Where a nametable address falls in 560's copy of the console's 2 KiB, by the header's
    /// mirroring.
    fn nt_offset_560(memory: &CartridgeMemory, address: u16) -> usize {
        let page = match memory.mirroring() {
            NametableMirroring::Horizontal => (address >> 11) & 0x01,
            NametableMirroring::SingleScreenA => 0,
            NametableMirroring::SingleScreenB => 1,
            _ => (address >> 10) & 0x01,
        };
        usize::from(page) * 0x400 + usize::from(address & 0x03FF)
    }

    /// `prg_fix_413`/`chr_fix_413`: `$C000-$CFFF` is the serial port (`low_rom` and
    /// `rom_windows`), `$D000` 4 KiB bank 7 (the second half of 8 KiB bank 3).
    fn map_413(&mut self) {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        b.map_prg_8k(0, r[1]);
        b.map_prg_8k(1, r[2]);
        b.map_prg_8k(2, 3);
        b.map_prg_8k(3, 4);
        b.map_chr_4k(0, r[3]);
        b.map_chr_4k(4, 0xFD);
    }

    /// `prg_swap_fme7_528`/`chr_swap_fme7_528`: `outer + (value & (outer | $0F))`, the last
    /// window `~0`; CHR `outer << 4 | value`.
    fn map_528(&mut self) {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let outer = i32::from(self.nametables[1]);
        let mask = outer | 0x0F;
        for (slot, value) in [r[1], r[2], r[3], 0xFF].into_iter().enumerate() {
            b.map_prg_8k(slot, outer + (value & mask));
        }
        for (slot, bank) in self.chr.iter().enumerate() {
            b.map_chr_1k(slot, (outer << 4) | i32::from(*bank));
        }
    }

    /// The mirroring a kind drives, if any (`mirroring_fix_357`: vertical with the DIP at 0;
    /// `mirroring_fix_416`: `$8000` D2).
    fn mirroring(&self) -> Option<NametableMirroring> {
        match self.kind {
            // `mirroring_fix_526`: vertical.
            IrqPKind::M357 | IrqPKind::M526 => Some(NametableMirroring::Vertical),
            // The FME-7's mirroring command.
            IrqPKind::M528 => Some(match self.nametables[0] & 0x03 {
                0 => NametableMirroring::Vertical,
                1 => NametableMirroring::Horizontal,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            // `mirroring_fix_359`.
            IrqPKind::M359 | IrqPKind::M540 => Some(match self.ext_359[4] & 0x03 {
                0 => NametableMirroring::Vertical,
                1 => NametableMirroring::Horizontal,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            // `mirroring_fix_272`: `regs[2]` 2/3 single-screen, else `regs[3]` H/V.
            IrqPKind::M272 => Some(match self.regs[2] {
                2 => NametableMirroring::SingleScreenA,
                3 => NametableMirroring::SingleScreenB,
                _ if self.regs[3] != 0 => NametableMirroring::Horizontal,
                _ => NametableMirroring::Vertical,
            }),
            IrqPKind::M416 => Some(if self.regs[1] & 0x04 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            _ => None,
        }
    }

    /// The 8 KiB (or 4 KiB) PRG-ROM bank a kind shows below `$8000`, and its size.
    fn low_rom(&self, address: u16) -> Option<(usize, usize)> {
        match (self.kind, address) {
            (IrqPKind::M311, 0x5000..=0x5FFF) => Some((17, 0x1000)),
            (IrqPKind::M311, 0x6000..=0x7FFF) => Some((9, 0x2000)),
            (IrqPKind::M357, 0x6000..=0x7FFF) => {
                Some((if self.regs[2] != 0 { 0 } else { 2 }, 0x2000))
            }
            (IrqPKind::M368, 0x6000..=0x7FFF) => Some((2, 0x2000)),
            (IrqPKind::M416, 0x6000..=0x7FFF) => Some((7, 0x2000)),
            // `wram_swap_fme7_528`: `prg[0]` 1 is the work RAM, any other value a ROM bank.
            (IrqPKind::M528, 0x6000..=0x7FFF) if self.regs[0] != 0x01 => {
                let outer = usize::from(self.nametables[1]);
                Some((outer + (usize::from(self.regs[0]) & (outer | 0x0F)), 0x2000))
            }
            // `wram_fix_413`: 4 KiB bank 1 at `$5000`, `reg[0]` at `$6000`; `prg_fix_413`'s 4 KiB
            // bank 7 at `$D000`.
            (IrqPKind::M413, 0x5000..=0x5FFF) => Some((1, 0x1000)),
            (IrqPKind::M413, 0x6000..=0x7FFF) => Some((usize::from(self.regs[0]), 0x2000)),
            (IrqPKind::M413, 0xD000..=0xDFFF) => Some((7, 0x1000)),
            // `wram_fix_359`: `prg[3]` through the outer bank and mask.
            (IrqPKind::M359 | IrqPKind::M540, 0x6000..=0x7FFF) => {
                Some((usize::from(self.prg_359(self.regs[3])), 0x2000))
            }
            _ => None,
        }
    }

    fn settle(&mut self, memory: &mut CartridgeMemory) {
        self.map();
        if let Some(mirroring) = self.mirroring() {
            memory.set_mirroring(mirroring);
        }
    }

    fn acknowledge(&mut self) {
        self.irq_counter = 0;
        self.irq_pending = false;
    }

    /// A CPU write the board decodes, `$4020-$FFFF`.
    fn latch(&mut self, address: u16, value: u8) {
        let r = &mut self.regs;
        match self.kind {
            IrqPKind::M311 => match address {
                0x4022 => r[0] = value & 0x01,
                0x4122 => {
                    self.irq_enabled = value & 0x01 != 0;
                    self.acknowledge();
                }
                _ => {}
            },
            IrqPKind::M357 => {
                if address & 0x8000 != 0 {
                    r[0] = value & 0x07;
                }
                match address & 0x71FF {
                    0x4022 => r[1] = value & 0x07,
                    0x4120 => r[2] = value & 0x01,
                    _ => {}
                }
                if address & 0xF1FF == 0x4122 {
                    self.irq_enabled = value & 0x01 != 0;
                    self.acknowledge();
                }
            }
            IrqPKind::M368 => match address & 0xF1FF {
                0x4022 => r[0] = value,
                0x4122 => {
                    r[1] = value;
                    self.irq_enabled = value & 0x01 != 0;
                    if !self.irq_enabled {
                        self.acknowledge();
                    }
                }
                _ => {}
            },
            IrqPKind::M416 => match address & 0xF000 {
                0x4000 | 0x5000 if address & 0x0020 != 0 => {
                    if address & 0x0100 != 0 {
                        self.irq_enabled = value & 0x01 != 0;
                    } else {
                        r[0] = value;
                    }
                }
                0x8000 | 0x9000 => r[1] = value,
                _ => {}
            },
            // `extcl_cpu_wr_mem_417`: by `address & $8073`.
            IrqPKind::M417 => match address & 0x8073 {
                a @ 0x8000..=0x8002 => r[usize::from(a & 0x03)] = value,
                a @ 0x8010..=0x8013 => self.chr[usize::from(a & 0x03)] = value,
                a @ 0x8020..=0x8023 => self.chr[0x04 | usize::from(a & 0x03)] = value,
                0x8030 => {
                    self.irq_enabled = true;
                    self.irq_counter = 0;
                }
                0x8040 => {
                    self.irq_enabled = false;
                    self.irq_pending = false;
                }
                a @ 0x8050..=0x8053 => self.nametables[usize::from(a & 0x03)] = value,
                _ => {}
            },
            // `extcl_cpu_wr_mem_526`: by `address & $E00F`.
            IrqPKind::M526 => match address & 0xE00F {
                a @ 0x8000..=0x8007 => self.chr[usize::from(a & 0x07)] = value,
                a @ 0x8008..=0x800B => r[usize::from(a & 0x03)] = value,
                0x800D => self.irq_counter = 0,
                0x800F => self.irq_pending = false,
                _ => {}
            },
            // `extcl_cpu_wr_mem_471`: the address, and the IRQ acknowledged.
            IrqPKind::M471 => {
                if address >= 0x8000 {
                    r[0] = address.to_le_bytes()[0];
                    self.irq_pending = false;
                }
            }
            IrqPKind::M272 => self.latch_272(address, value),
            IrqPKind::M359 | IrqPKind::M540 => self.latch_359(address, value),
            IrqPKind::M528 => self.latch_528(address, value),
            IrqPKind::M413 => self.latch_413(address, value),
            // `extcl_cpu_wr_mem_560`: any write toggles the bank.
            IrqPKind::M560 => {
                if address >= 0x8000 {
                    r[0] ^= 1;
                }
            }
            // `extcl_cpu_wr_mem_442`: `$5000-$5FFF`, A8-A10 the register, of which only the
            // first is read.
            IrqPKind::M442 => {
                if (0x5000..=0x5FFF).contains(&address) && address & 0x0700 == 0 {
                    r[0] = value;
                }
            }
            // `extcl_cpu_wr_mem_558` decodes `$5000-$53FF` only ([`IrqP::write_558`]).
            IrqPKind::M558 => {}
        }
    }
}

impl Mapper for IrqP {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
        writer.write_raw(&self.chr);
        writer.write_raw(&self.nametables);
        writer.write_bool(self.irq_enabled);
        writer.write_i64(i64::from(self.irq_counter));
        writer.write_bool(self.irq_pending);
        if let Some(eeprom) = &self.eeprom {
            eeprom.save_state(writer);
        }
        if matches!(self.kind, IrqPKind::M359 | IrqPKind::M540) {
            writer.write_raw(&self.ext_359);
        }
        if self.kind == IrqPKind::M528 {
            self.vrc_irq.save_state(writer);
        }
        if self.kind == IrqPKind::M560 {
            writer.write_raw(&self.nt_560);
        }
        if self.kind == IrqPKind::M413 {
            writer.write_u32(self.serial_413);
            self.a12.save_state(writer);
        }
        if self.kind == IrqPKind::M471 {
            self.a12.save_state(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.nametables)?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_counter = u32::try_from(reader.read_i64()? & 0xFFFF_FFFF).unwrap_or(0);
        self.irq_pending = reader.read_bool()?;
        if let Some(eeprom) = &mut self.eeprom {
            eeprom.load_state(reader)?;
        }
        if matches!(self.kind, IrqPKind::M359 | IrqPKind::M540) {
            reader.read_into(&mut self.ext_359)?;
        }
        if self.kind == IrqPKind::M528 {
            self.vrc_irq.load_state(reader)?;
        }
        if self.kind == IrqPKind::M560 {
            reader.read_into(&mut self.nt_560)?;
        }
        if self.kind == IrqPKind::M413 {
            self.serial_413 = reader.read_u32()?;
            self.a12.load_state(reader)?;
        }
        if self.kind == IrqPKind::M471 {
            self.a12.load_state(reader)?;
        }
        self.map();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        let kind = self.kind;
        *self = Self::new(memory, kind);
        self.settle(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending || self.vrc_irq.asserted()
    }

    /// `extcl_cpu_every_cycle_NNN`. 311: past 4096 cycles the line stays asserted. 357: at
    /// `$1000` the counter wraps and asserts. 368: a 12-bit counter asserting at 0. 416: puNES
    /// resets the counter on every cycle that does not reach `$1000`, so its IRQ never fires as
    /// written; this board counts while enabled, asserts at `$1000` and clears when disabled,
    /// which is the evident intent (FIDELITY-LEADS J37). 417: the counter runs always, bit 10
    /// asserts while enabled and restarts it.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        match self.kind {
            IrqPKind::M311 => {
                if self.irq_enabled {
                    self.irq_counter = self.irq_counter.saturating_add(1);
                    if self.irq_counter > 4096 {
                        self.irq_pending = true;
                    }
                }
            }
            IrqPKind::M357 => {
                if self.irq_enabled {
                    self.irq_counter += 1;
                    if self.irq_counter == 0x1000 {
                        self.irq_counter = 0;
                        self.irq_pending = true;
                    }
                }
            }
            IrqPKind::M368 => {
                if self.irq_enabled {
                    self.irq_counter = (self.irq_counter + 1) & 0x0FFF;
                    if self.irq_counter == 0 {
                        self.irq_pending = true;
                    }
                }
            }
            IrqPKind::M416 => {
                if self.irq_enabled {
                    self.irq_counter = self.irq_counter.saturating_add(1);
                    if self.irq_counter == 0x1000 {
                        self.irq_pending = true;
                    }
                } else {
                    self.acknowledge();
                }
            }
            IrqPKind::M417 => {
                self.irq_counter = (self.irq_counter + 1) & 0xFFFF;
                if self.irq_counter & 0x400 != 0 && self.irq_enabled {
                    self.irq_counter = 0;
                    self.irq_pending = true;
                }
            }
            // `extcl_cpu_every_cycle_526`: a 16-bit counter, asserting while bit 12 is set.
            IrqPKind::M526 => {
                self.irq_counter = (self.irq_counter + 1) & 0xFFFF;
                if self.irq_counter & 0x1000 != 0 {
                    self.irq_pending = true;
                }
            }
            // `extcl_cpu_every_cycle_359`: the A12 filter counts down; in the CPU mode the 16-bit
            // counter counts down while enabled and asserts on reaching 0.
            IrqPKind::M359 | IrqPKind::M540 => {
                self.ext_359[6] = self.ext_359[6].saturating_sub(1);
                if self.irq_enabled && self.ext_359[5] & 0x02 == 0 && self.irq_counter != 0 {
                    self.irq_counter -= 1;
                    if self.irq_counter == 0 {
                        self.irq_pending = true;
                    }
                }
            }
            IrqPKind::M528 => self.vrc_irq.step_cpu_cycle(),
            IrqPKind::M471
            | IrqPKind::M558
            | IrqPKind::M272
            | IrqPKind::M442
            | IrqPKind::M560
            | IrqPKind::M413 => {}
        }
    }

    /// `irq_clock_471`: puNES compares the background and sprite pattern addresses through the
    /// line and asserts on a rise of A12, which is the filtered rise the MMC3 counts.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.kind == IrqPKind::M471 && self.a12.rose(address, clock) {
            self.irq_pending = true;
        }
        // `extcl_ppu_*_359`/`extcl_update_r2006_359`: a rise of A12 clocks the counter.
        if matches!(self.kind, IrqPKind::M359 | IrqPKind::M540) {
            let a12 = u8::from(address & 0x1000 != 0);
            if self.ext_359[7] == 0 && a12 != 0 {
                self.clock_359();
            }
            self.ext_359[7] = a12;
        }
        // `extcl_rd_nmt_560`/`extcl_rd_chr_560`: the first nametable fetch after a pattern fetch
        // latches its address.
        if self.kind == IrqPKind::M560 {
            if address >= 0x2000 {
                if self.regs[1] == 0 {
                    self.irq_counter = u32::from(address & 0x03FF);
                }
                self.regs[1] = 1;
            } else {
                self.regs[1] = 0;
            }
        }
        // `extcl_irq_A12_clock_413`: the MMC3's counter without its reload flag.
        if self.kind == IrqPKind::M413 && self.a12.rose(address, clock) {
            self.irq_counter = if self.irq_counter == 0 {
                u32::from(self.nametables[0])
            } else {
                self.irq_counter - 1
            };
            if self.irq_counter == 0 && self.irq_enabled {
                self.irq_pending = true;
            }
        }
        // `mode1_bpp`: PPU A0 and A9 on the rise of A13 (`regs[1]`, `regs[2]`; `regs[3]` the
        // previous A13).
        if self.kind == IrqPKind::M442 {
            let a13 = u8::from(address & 0x2000 != 0);
            if self.regs[3] == 0 && a13 != 0 {
                self.regs[1] = u8::from(address & 0x0001 != 0);
                self.regs[2] = u8::from(address & 0x0200 != 0);
            }
            self.regs[3] = a13;
        }
        // `extcl_ppu_000_to_34x_272`: puNES steps every eighth dot of a rendered line, which are
        // its 34 background tile slots and 8 sprite slots; here each slot's low-plane pattern
        // fetch is one step (FIDELITY-LEADS J37).
        if self.kind == IrqPKind::M272
            && self.irq_enabled
            && address < 0x2000
            && address & 0x0008 == 0
        {
            self.irq_counter += 1;
            if self.irq_counter == 84 {
                self.irq_pending = true;
                self.irq_counter = 0;
            }
        }
    }

    /// `memmap_prgrom_*(0, MMCPU(0x5000/0x6000), …)`: ROM where the work RAM would be.
    fn prg_ram_enabled(&self) -> bool {
        matches!(self.kind, IrqPKind::M417 | IrqPKind::M558)
            || (self.kind == IrqPKind::M528 && self.regs[0] == 0x01)
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        // `extcl_cpu_rd_mem_558`: `$5000-$5FFF` the EEPROM's DO in D2, or `reg[2]` D2 without one.
        if self.kind == IrqPKind::M558 && address & 0xF000 == 0x5000 {
            return Some(match &self.eeprom {
                Some(eeprom) => (eeprom.output() & 1) << 2,
                None => self.regs[2] & 0x04,
            });
        }
        // `extcl_cpu_rd_mem_311`: `$4042-$4055` read `$FF`.
        if self.kind == IrqPKind::M311 && (0x4042..=0x4055).contains(&address) {
            return Some(0xFF);
        }
        // `extcl_cpu_rd_mem_368`: the IRQ register reads `$8A | (value & $35)`.
        if self.kind == IrqPKind::M368 && address < 0x8000 && address & 0xF1FF == 0x4122 {
            return Some(0x8A | (self.regs[1] & 0x35));
        }
        if let Some((bank, size)) = self.low_rom(address) {
            let rom = memory.rom().prg_rom();
            if rom.is_empty() {
                return None;
            }
            let offset = bank * size + usize::from(address) % size;
            return rom.get(offset % rom.len()).copied();
        }
        self.banks
            .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        // Every register below `$8000` is in `$4020-$5FFF`.
        if address >= 0x6000 {
            return false;
        }
        if self.kind == IrqPKind::M558 {
            if address & 0xFC00 == 0x5000 {
                self.write_558(memory, address, value);
                self.settle(memory);
            }
            return true;
        }
        self.latch(address, value);
        self.settle(memory);
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.latch(address, value);
        self.settle(memory);
    }

    /// 442's 1 bpp mode reads its own CHR address, so the peek view is not the windows'.
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if matches!(self.kind, IrqPKind::M442 | IrqPKind::M560) {
            return None;
        }
        let chr_rom = memory.rom().chr_rom();
        let chr = if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        };
        Some(ChrView::new(chr, &self.banks))
    }

    /// `extcl_rd_chr_442`: in the 1 bpp mode (`reg[0]` D7) CHR A12 and A3 are the latched PPU A9
    /// and A0.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        // `extcl_rd_chr_560`: `A12-A10 → 11-9`, `A9-A4 → 8-3`, `A2-A0`, plane A3 dropped; above
        // them the extension RAM's byte for the latched tile (bank 1) or A3 as A16 (bank 0).
        if self.kind == IrqPKind::M560 && address < 0x2000 {
            let a = usize::from(address);
            let base = ((a >> 10) << 9) | ((a & 0x03F0) >> 1) | (a & 0x07);
            let high = if self.regs[0] != 0 {
                let tile = usize::try_from(self.irq_counter & 0x03FF).unwrap_or(0);
                (usize::from(self.nt_560[0x800 + tile]) << 11) & 0x1_F800
            } else {
                ((a & 0x03FF) << 13) & 0x1_0000
            };
            return memory.read_chr_direct(high | base);
        }
        let address = if self.kind == IrqPKind::M442 && self.regs[0] & 0x80 != 0 {
            (u16::from(self.regs[2]) << 12) | (address & 0x0FF7) | (u16::from(self.regs[1]) << 3)
        } else {
            address
        };
        let chr_rom = memory.rom().chr_rom();
        self.banks.read_chr(
            address,
            if chr_rom.is_empty() {
                memory.chr_ram()
            } else {
                chr_rom
            },
        )
    }

    fn cpu_read_override(&mut self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        let value = self.read_413(address)?;
        if self.nametables[1] & 0x02 != 0 {
            self.serial_413 = self.serial_413.wrapping_add(1);
        }
        Some(value)
    }

    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.read_413(address)
    }

    /// 413's serial port takes `$C000-$CFFF` off the ROM windows.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        let mut windows = self.banks.rom_windows(memory.rom().prg_rom().len());
        if self.kind == IrqPKind::M413 {
            windows[2] = super::NO_ROM_WINDOW;
        }
        windows
    }

    /// 560 serves the console's nametables itself, to see the writes `extcl_wr_nmt_560` copies.
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        self.nt_560
            .get(Self::nt_offset_560(memory, address))
            .copied()
            .unwrap_or(0)
    }

    /// `extcl_wr_nmt_560`: a write to table 1 or 3 is also the extension RAM's byte.
    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let offset = Self::nt_offset_560(memory, address);
        if let Some(byte) = self.nt_560.get_mut(offset) {
            *byte = value;
        }
        if address & 0x0400 != 0 {
            if let Some(byte) = self.nt_560.get_mut(0x800 + usize::from(address & 0x03FF)) {
                *byte = value;
            }
        }
    }

    /// `mirroring_fix_417`: a CIRAM page per table from each register's D0.
    fn nametable_page(&self, table: i32) -> i32 {
        if self.kind == IrqPKind::M417 {
            i32::from(self.nametables[usize::try_from(table & 0x03).unwrap_or(0)] & 0x01)
        } else if self.kind == IrqPKind::M560 {
            super::super::Cartridge::SERVED_BY_CARTRIDGE
        } else {
            super::super::Cartridge::FOLLOW_MIRRORING
        }
    }
}
