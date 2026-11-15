//! Discrete and small-ASIC boards **with an IRQ counter** (lot MAPPERS-N): mostly the FDS-to-
//! cartridge conversions of the Whirlwind Manu and Kaiser kind, whose counter stands in for the
//! disk system's timer, plus two pirate boards that count PPU A12 rises.
//!
//! Each kind is read from the Mesen2 file it names (`cleanroom/third_party/mesen2/Core/NES/
//! Mappers/…`, read, not copied); PRG windows are 8 KiB here as in all those files.

use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which board, one variant per iNES number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqLatchKind {
    /// 40, `Whirlwind/Mapper40.h` (NTDEC 2722, *SMB2J* conversion).
    M40,
    /// 42, `Unlicensed/Mapper42.h` (*Ai Senshi Nicol* / *Mario Baby* conversions).
    M42,
    /// 43, `Unlicensed/Mapper43.h` (*SMB2J* "LF36" conversion).
    M43,
    /// 50, `Unlicensed/Mapper50.h` (*SMB2J* "N-32" conversion).
    M50,
    /// 106, `Unlicensed/Mapper106.h` (*SMB3* bootleg).
    M106,
    /// 117, `Unlicensed/Mapper117.h` (Future Media, *Crayon Shin-chan*).
    M117,
    /// 183, `Unlicensed/Mapper183.h` (*Shui Hu Zhuan* bootleg, a VRC4-like board).
    M183,
    /// 222, `Unlicensed/Mapper222.h` (*Dragon Ninja* bootleg).
    M222,
}

impl IrqLatchKind {
    /// The kind of an iNES number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            40 => Self::M40,
            42 => Self::M42,
            43 => Self::M43,
            50 => Self::M50,
            106 => Self::M106,
            117 => Self::M117,
            183 => Self::M183,
            222 => Self::M222,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M40 => 40,
            Self::M42 => 42,
            Self::M43 => 43,
            Self::M50 => 50,
            Self::M106 => 106,
            Self::M117 => 117,
            Self::M183 => 183,
            Self::M222 => 222,
        }
    }

    /// Whether `$6000-$7FFF` shows PRG-ROM rather than work RAM.
    const fn rom_at_6000(self) -> bool {
        matches!(
            self,
            Self::M40 | Self::M42 | Self::M43 | Self::M50 | Self::M183
        )
    }

    /// Whether the counter follows PPU A12 rather than the CPU clock.
    const fn counts_a12(self) -> bool {
        matches!(self, Self::M117 | Self::M222)
    }
}

/// A board of this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrqLatch {
    banks: BankWindows,
    kind: IrqLatchKind,
    /// The PRG register(s), per kind; `prg[4]` is the `$6000-$7FFF` bank where there is one,
    /// `prg[5]` 43's swap bit.
    prg: [u8; 6],
    /// The 1 KiB CHR registers (106, 117, 183, 222), or `chr[0]` the 8 KiB one (42).
    chr: [u8; 8],
    counter: u16,
    scaler: u16,
    reload: u8,
    enabled: bool,
    /// 117's second enable (`_irqEnabledAlt`), 183's one-cycle delay (`_needIrq`).
    second: bool,
    pending: bool,
    a12: A12Watcher,
    has_chr_rom: bool,
}

impl IrqLatch {
    pub(crate) fn new(memory: &CartridgeMemory, kind: IrqLatchKind) -> Self {
        Self {
            banks: windows_for(memory),
            kind,
            prg: [0; 6],
            chr: [0; 8],
            counter: 0,
            scaler: 0,
            reload: 0,
            enabled: false,
            second: false,
            pending: false,
            a12: A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS),
            has_chr_rom: !memory.rom().chr_rom().is_empty(),
        }
    }

    fn map(&mut self) {
        let p = |i: usize| i32::from(self.prg[i]);
        let prg: [i32; 4] = match self.kind {
            // `Mapper40.h:13-18,39`.
            IrqLatchKind::M40 => [4, 5, p(0), 7],
            // `Mapper42.h:15-18`.
            IrqLatchKind::M42 => [-4, -3, -2, -1],
            // `Mapper43.h:35-38`, `:21-22`.
            IrqLatchKind::M43 => [1, 0, p(0), if self.prg[5] != 0 { 8 } else { 9 }],
            // `Mapper50.h:16-19`, `:33`.
            IrqLatchKind::M50 => [8, 9, p(0), 0x0B],
            // `Mapper106.h:15-18,37-38`: power-on -1 everywhere.
            // `Mapper117.h:17` (`SelectPrgPage4x(0, -4)`) and `:32-34`: four 8 KiB registers too.
            IrqLatchKind::M106 | IrqLatchKind::M117 => [p(0), p(1), p(2), p(3)],
            // `Mapper183.h` `UpdatePrg` and the `$8800`/`$A800`/`$A000` ports.
            IrqLatchKind::M183 => [p(0), p(1), p(2), -1],
            // `Mapper222.h:17` (`SelectPrgPage2x(1, -2)`) and `$8000`/`$A000`.
            IrqLatchKind::M222 => [p(0), p(1), -2, -1],
        };
        for (slot, bank) in prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, *bank);
        }
        match self.kind {
            IrqLatchKind::M106 | IrqLatchKind::M117 | IrqLatchKind::M183 | IrqLatchKind::M222 => {
                for (slot, bank) in self.chr.iter().enumerate() {
                    self.banks.map_chr_1k(slot, i32::from(*bank));
                }
            }
            IrqLatchKind::M42 => self.banks.map_chr_8k(i32::from(self.chr[0])),
            _ => self.banks.map_chr_8k(0),
        }
    }

    /// The 8 KiB PRG-ROM page `$6000-$7FFF` shows.
    fn page_6000(&self) -> i32 {
        match self.kind {
            IrqLatchKind::M40 => 6,
            IrqLatchKind::M42 => i32::from(self.prg[4] & 0x0F),
            IrqLatchKind::M43 => {
                if self.prg[5] != 0 {
                    0
                } else {
                    2
                }
            }
            IrqLatchKind::M50 => 0x0F,
            _ => i32::from(self.prg[4]),
        }
    }

    fn rom_byte(memory: &CartridgeMemory, page: i32, offset: u16) -> Option<u8> {
        let rom = memory.rom().prg_rom();
        let count = i32::try_from(rom.len() / 0x2000).unwrap_or(1).max(1);
        let base = usize::try_from(page.rem_euclid(count)).unwrap_or(0) * 0x2000;
        rom.get(base + usize::from(offset)).copied()
    }

    /// A write anywhere the board decodes, `$4020-$FFFF`. Returns whether it was a register.
    #[allow(clippy::too_many_lines)]
    fn write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        match self.kind {
            // `Mapper40.h:29-41`.
            IrqLatchKind::M40 => match address & 0xE000 {
                0x8000 => {
                    self.counter = 0;
                    self.pending = false;
                }
                0xA000 => self.counter = 4096,
                0xE000 => self.prg[0] = value,
                _ => return address >= 0x8000,
            },
            // `Mapper42.h:39-60`.
            IrqLatchKind::M42 => match address & 0xE003 {
                0x8000 if self.has_chr_rom => self.chr[0] = value & 0x0F,
                0xE000 => self.prg[4] = value & 0x0F,
                0xE001 => memory.set_mirroring(if value & 0x08 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                }),
                0xE002 => {
                    self.enabled = value == 0x02;
                    if !self.enabled {
                        self.pending = false;
                        self.counter = 0;
                    }
                }
                _ => return address >= 0x8000,
            },
            // `Mapper43.h:44-55`: registers over `$4020-$FFFF` under mask `$F1FF`.
            IrqLatchKind::M43 => match address & 0xF1FF {
                0x4022 => self.prg[0] = [4, 3, 5, 3, 6, 3, 7, 3][usize::from(value & 0x07)],
                0x4120 => self.prg[5] = value & 0x01,
                0x8122 | 0x4122 => {
                    self.enabled = value & 0x01 != 0;
                    self.pending = false;
                    self.counter = 0;
                }
                _ => return address >= 0x8000,
            },
            // `Mapper50.h:30-44`: registers over `$4020-$5FFF` under mask `$4120`.
            IrqLatchKind::M50 => {
                if !(0x4020..=0x5FFF).contains(&address) {
                    return address >= 0x8000;
                }
                match address & 0x4120 {
                    0x4020 => {
                        self.prg[0] =
                            (value & 0x08) | ((value & 0x01) << 2) | ((value & 0x06) >> 1);
                    }
                    0x4120 => {
                        if value & 0x01 != 0 {
                            self.enabled = true;
                        } else {
                            self.pending = false;
                            self.counter = 0;
                            self.enabled = false;
                        }
                    }
                    _ => {}
                }
            }
            // `Mapper106.h:31-49`: sixteen ports under mask `$000F`.
            IrqLatchKind::M106 => {
                if address < 0x8000 {
                    return false;
                }
                let port = usize::from(address & 0x0F);
                match port {
                    0 | 2 => self.chr[port] = value & 0xFE,
                    1 | 3 => self.chr[port] = value | 0x01,
                    4..=7 => self.chr[port] = value,
                    8 | 0x0B => self.prg[port - 8] = (value & 0x0F) | 0x10,
                    9 | 0x0A => self.prg[port - 8] = value & 0x1F,
                    0x0D => {
                        self.enabled = false;
                        self.counter = 0;
                        self.pending = false;
                    }
                    0x0E => self.counter = (self.counter & 0xFF00) | u16::from(value),
                    0x0F => {
                        self.counter = (self.counter & 0x00FF) | (u16::from(value) << 8);
                        self.enabled = true;
                    }
                    _ => {}
                }
            }
            // `Mapper117.h:30-52`: exact addresses.
            IrqLatchKind::M117 => match address {
                0x8000..=0x8003 => self.prg[usize::from(address & 0x03)] = value,
                0xA000..=0xA007 => self.chr[usize::from(address & 0x07)] = value,
                0xC001 => self.reload = value,
                0xC002 => self.pending = false,
                0xC003 => {
                    self.counter = u16::from(self.reload);
                    self.second = true;
                }
                0xD000 => memory.set_mirroring(if value & 0x01 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                }),
                0xE000 => {
                    self.enabled = value & 0x01 != 0;
                    self.pending = false;
                }
                _ => return address >= 0x8000,
            },
            // `Mapper183.h` `WriteRegister`: `$6800-$6FFF` under `$F800` banks `$6000`, the CHR
            // nibbles are VRC4-style, the rest under mask `$F80C`.
            IrqLatchKind::M183 => {
                if address < 0x6000 {
                    return false;
                }
                let masked = address & 0xF80C;
                if address & 0xF800 == 0x6800 {
                    self.prg[4] = (address & 0x3F).to_le_bytes()[0];
                } else if (0xB000..=0xE00C).contains(&masked) {
                    let a = i32::from(address);
                    let slot = usize::try_from((((a >> 11) - 6) | (a >> 3)) & 0x07).unwrap_or(0);
                    let shift = u32::from(address & 0x04);
                    self.chr[slot] = (self.chr[slot] & (0xF0 >> shift)) | ((value & 0x0F) << shift);
                } else {
                    match masked {
                        0x8800 => self.prg[0] = value,
                        0xA800 => self.prg[1] = value,
                        0xA000 => self.prg[2] = value,
                        0x9800 => memory.set_mirroring(match value & 0x03 {
                            0 => NametableMirroring::Vertical,
                            1 => NametableMirroring::Horizontal,
                            2 => NametableMirroring::SingleScreenA,
                            _ => NametableMirroring::SingleScreenB,
                        }),
                        0xF000 => self.counter = (self.counter & 0xF0) | u16::from(value & 0x0F),
                        0xF004 => {
                            self.counter = (self.counter & 0x0F) | (u16::from(value & 0x0F) << 4);
                        }
                        0xF008 => {
                            self.enabled = value > 0;
                            if !self.enabled {
                                self.scaler = 0;
                            }
                            self.pending = false;
                        }
                        _ => {}
                    }
                }
            }
            // `Mapper222.h:30-48`, mask `$F003`.
            IrqLatchKind::M222 => match address & 0xF003 {
                0x8000 => self.prg[0] = value,
                0x9000 => memory.set_mirroring(if value & 0x01 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                }),
                0xA000 => self.prg[1] = value,
                0xB000 => self.chr[0] = value,
                0xB002 => self.chr[1] = value,
                0xC000 => self.chr[2] = value,
                0xC002 => self.chr[3] = value,
                0xD000 => self.chr[4] = value,
                0xD002 => self.chr[5] = value,
                0xE000 => self.chr[6] = value,
                0xE002 => self.chr[7] = value,
                0xF000 => {
                    self.counter = u16::from(value);
                    self.pending = false;
                }
                _ => return address >= 0x8000,
            },
        }
        self.map();
        true
    }
}

impl Mapper for IrqLatch {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u16(self.counter);
        writer.write_u16(self.scaler);
        writer.write_u8(self.reload);
        writer.write_bool(self.enabled);
        writer.write_bool(self.second);
        writer.write_bool(self.pending);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.counter = reader.read_u16()?;
        self.scaler = reader.read_u16()?;
        self.reload = reader.read_u8()?;
        self.enabled = reader.read_bool()?;
        self.second = reader.read_bool()?;
        self.pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        self.map();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0; 6];
        self.chr = [0; 8];
        match self.kind {
            // `Mapper106.h:15-18`: every PRG window -1.
            IrqLatchKind::M106 => self.prg[..4].fill(0xFF),
            // `Mapper117.h:17`: `SelectPrgPage4x(0, -4)`, the last 32 KiB.
            IrqLatchKind::M117 => self.prg[..4].copy_from_slice(&[0xFC, 0xFD, 0xFE, 0xFF]),
            _ => {}
        }
        self.counter = 0;
        self.scaler = 0;
        self.reload = 0;
        self.enabled = false;
        self.second = false;
        self.pending = false;
        self.a12.reset();
        self.map();
    }

    fn irq_asserted(&self) -> bool {
        self.pending
    }

    fn prg_ram_enabled(&self) -> bool {
        !self.kind.rom_at_6000()
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
        match self.kind {
            IrqLatchKind::M43 | IrqLatchKind::M50 | IrqLatchKind::M183 => {
                self.write(memory, address, value)
            }
            _ => false,
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            let _ = self.write(memory, address, value);
        }
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if (0x6000..=0x7FFF).contains(&address) && self.kind.rom_at_6000() {
            return Self::rom_byte(memory, self.page_6000(), address & 0x1FFF);
        }
        // `Mapper43.h:20`: `$5000-$5FFF` is the first 4 KiB of PRG-ROM page 8.
        if self.kind == IrqLatchKind::M43 && (0x5000..=0x5FFF).contains(&address) {
            return Self::rom_byte(memory, 8, address & 0x0FFF);
        }
        self.banks
            .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
    }

    /// The CPU-clocked counters.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        match self.kind {
            // `Mapper40.h:20-28`.
            IrqLatchKind::M40 => {
                if self.counter > 0 {
                    self.counter -= 1;
                    if self.counter == 0 {
                        self.pending = true;
                    }
                }
            }
            // `Mapper42.h:26-37`: the line follows the counter's top bits, set and cleared.
            IrqLatchKind::M42 => {
                if self.enabled {
                    self.counter += 1;
                    if self.counter >= 0x8000 {
                        self.counter -= 0x8000;
                    }
                    self.pending = self.counter >= 0x6000;
                }
            }
            // `Mapper43.h:24-33`.
            IrqLatchKind::M43 => {
                if self.enabled {
                    self.counter += 1;
                    if self.counter >= 4096 {
                        self.enabled = false;
                        self.pending = true;
                    }
                }
            }
            // `Mapper50.h:21-28`.
            IrqLatchKind::M50 => {
                if self.enabled {
                    self.counter += 1;
                    if self.counter == 0x1000 {
                        self.pending = true;
                        self.enabled = false;
                    }
                }
            }
            // `Mapper106.h:20-28`.
            IrqLatchKind::M106 => {
                if self.enabled {
                    self.counter = self.counter.wrapping_add(1);
                    if self.counter == 0 {
                        self.pending = true;
                        self.enabled = false;
                    }
                }
            }
            // `Mapper183.h` `ProcessCpuClock`: a 114-cycle prescaler, and the line one cycle late.
            IrqLatchKind::M183 => {
                if self.second {
                    self.pending = true;
                    self.second = false;
                }
                self.scaler += 1;
                if self.scaler == 114 {
                    self.scaler = 0;
                    if self.enabled {
                        self.counter = (self.counter + 1) & 0xFF;
                        if self.counter == 0 {
                            self.second = true;
                        }
                    }
                }
            }
            IrqLatchKind::M117 | IrqLatchKind::M222 => {}
        }
    }

    /// `Mapper117.h:20-28` and `Mapper222.h:20-28`, on Mesen2's `A12Watcher` rise.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if !self.kind.counts_a12() || !self.a12.rose(address, clock) {
            return;
        }
        if self.kind == IrqLatchKind::M117 {
            if self.enabled && self.second && self.counter != 0 {
                self.counter -= 1;
                if self.counter == 0 {
                    self.pending = true;
                    self.second = false;
                }
            }
        } else if self.counter != 0 {
            self.counter += 1;
            if self.counter >= 240 {
                self.pending = true;
                self.counter = 0;
            }
        }
    }

    /// The A12 kinds have no bound while a counter runs; the CPU-clocked ones never depend on
    /// the PPU.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.kind.counts_a12() && self.counter != 0 {
            None
        } else {
            Some(i64::MAX)
        }
    }
}
