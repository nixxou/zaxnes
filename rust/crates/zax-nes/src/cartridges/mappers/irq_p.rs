//! Lot MAPPERS-P: discrete boards with a CPU-cycle IRQ counter (`docs/progress/MAPPERS-P.md`),
//! most of them *Super Mario Bros. 2* (Japan) conversions, whose FDS timer becomes an M2 counter.
//!
//! Read from puNES 0.111 (`mapper_311.c`, `mapper_357.c`, `mapper_368.c`, `mapper_416.c`,
//! `mapper_417.c`; read, not copied). An [`super::IrqBoard`] variant, because the counter runs on
//! [`Mapper::on_cpu_cycle`] and the line on [`Mapper::irq_asserted`], which that enum forwards.
//! The PRG-ROM these boards show at `$5000`/`$6000` is answered by [`Mapper::try_read_prg`], with
//! the work RAM disabled ([`Mapper::prg_ram_enabled`]).
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
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
        };
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

    /// The mirroring a kind drives, if any (`mirroring_fix_357`: vertical with the DIP at 0;
    /// `mirroring_fix_416`: `$8000` D2).
    fn mirroring(&self) -> Option<NametableMirroring> {
        match self.kind {
            IrqPKind::M357 => Some(NametableMirroring::Vertical),
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
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.nametables)?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_counter = u32::try_from(reader.read_i64()? & 0xFFFF_FFFF).unwrap_or(0);
        self.irq_pending = reader.read_bool()?;
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
        }
    }

    /// `memmap_prgrom_*(0, MMCPU(0x5000/0x6000), …)`: ROM where the work RAM would be.
    fn prg_ram_enabled(&self) -> bool {
        self.kind == IrqPKind::M417
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
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

    /// `mirroring_fix_417`: a CIRAM page per table from each register's D0.
    fn nametable_page(&self, table: i32) -> i32 {
        if self.kind == IrqPKind::M417 {
            i32::from(self.nametables[usize::try_from(table & 0x03).unwrap_or(0)] & 0x01)
        } else {
            super::super::Cartridge::FOLLOW_MIRRORING
        }
    }
}
