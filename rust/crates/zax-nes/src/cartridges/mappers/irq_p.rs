//! Lot MAPPERS-P: discrete boards with a CPU-cycle IRQ counter (`docs/progress/MAPPERS-P.md`),
//! most of them *Super Mario Bros. 2* (Japan) conversions, whose FDS timer becomes an M2 counter.
//!
//! Read from puNES 0.111 (`mapper_311.c`, `mapper_357.c`, `mapper_368.c`, `mapper_416.c`,
//! `mapper_417.c`; read, not copied). An [`super::IrqBoard`] variant, because the counter runs on
//! [`Mapper::on_cpu_cycle`] and the line on [`Mapper::irq_asserted`], which that enum forwards.
//! The PRG-ROM these boards show at `$5000`/`$6000` is answered by [`Mapper::try_read_prg`], with
//! the work RAM disabled ([`Mapper::prg_ram_enabled`]).
use super::{windows_for, Eeprom93C66, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

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
            eeprom: (kind == IrqPKind::M558 && memory.prg_ram().len() == Self::EEPROM_RAM_SIZE)
                .then(Eeprom93C66::new),
        };
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

    /// The mirroring a kind drives, if any (`mirroring_fix_357`: vertical with the DIP at 0;
    /// `mirroring_fix_416`: `$8000` D2).
    fn mirroring(&self) -> Option<NametableMirroring> {
        match self.kind {
            // `mirroring_fix_526`: vertical.
            IrqPKind::M357 | IrqPKind::M526 => Some(NametableMirroring::Vertical),
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
        self.irq_pending
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
            IrqPKind::M471 | IrqPKind::M558 | IrqPKind::M272 | IrqPKind::M442 => {}
        }
    }

    /// `irq_clock_471`: puNES compares the background and sprite pattern addresses through the
    /// line and asserts on a rise of A12, which is the filtered rise the MMC3 counts.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.kind == IrqPKind::M471 && self.a12.rose(address, clock) {
            self.irq_pending = true;
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
        if self.kind == IrqPKind::M442 {
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

    /// `mirroring_fix_417`: a CIRAM page per table from each register's D0.
    fn nametable_page(&self, table: i32) -> i32 {
        if self.kind == IrqPKind::M417 {
            i32::from(self.nametables[usize::try_from(table & 0x03).unwrap_or(0)] & 0x01)
        } else {
            super::super::Cartridge::FOLLOW_MIRRORING
        }
    }
}
