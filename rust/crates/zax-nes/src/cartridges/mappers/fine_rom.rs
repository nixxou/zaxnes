//! Boards whose PRG windows are finer than the 8 KiB of [`BankWindows`], or mix ROM and RAM inside
//! one 8 KiB window (lot MAPPERS-N): every CPU read of `$6000-$FFFF` is answered here from the
//! board's own decode, and only CHR goes through the window table.
//!
//! Read from Mesen2 (`cleanroom/third_party/mesen2/Core/NES/Mappers/…`), nesdev and puNES
//! (`src/core/mappers/mapper_NNN.c`), all read and not copied; where they part, the kind's own
//! note says which is followed (FIDELITY-LEADS J24).

use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FineRomKind {
    /// 31, the NSF-style homebrew board (`Homebrew/NsfCart31.h`, nesdev "INES Mapper 031").
    M31,
    /// 103, the *Doki Doki Panic* FDS conversion (`Unlicensed/Mapper103.h`, `mapper_103.c`).
    M103,
    /// 175, Kaiser KS7022, the *15-in-1* (`Kaiser/Kaiser7022.h`, FCEUX `boards/175.cpp`).
    M175,
    /// 302, `Kaiser7057.h` (KS7057, *Gyruss* FDS conversion).
    M302,
    /// 305, `Kaiser7031.h` (KS7031, *Dracula II* FDS conversion).
    M305,
    /// 307, Kaiser KS7037, *Metroid* FDS conversion (`Kaiser/Kaiser7037.h`, `mapper_307.c`).
    M307,
}

impl FineRomKind {
    /// The kind of an iNES number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        match mapper {
            31 => Some(Self::M31),
            103 => Some(Self::M103),
            175 => Some(Self::M175),
            302 => Some(Self::M302),
            305 => Some(Self::M305),
            307 => Some(Self::M307),
            _ => None,
        }
    }

    const fn number(self) -> i32 {
        match self {
            Self::M31 => 31,
            Self::M103 => 103,
            Self::M175 => 175,
            Self::M302 => 302,
            Self::M305 => 305,
            Self::M307 => 307,
        }
    }
}

/// Where a CPU address lands.
enum Target {
    /// A byte offset into PRG-ROM, wrapped by the ROM's size.
    Rom(i64),
    /// A byte offset into PRG-RAM.
    Ram(usize),
}

/// A board of this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FineRom {
    /// CHR only.
    banks: BankWindows,
    kind: FineRomKind,
    regs: [u8; 8],
    /// 175: the value `$A000` wrote, waiting for the read of `$FFFC`; 307: the register index.
    pending: u8,
}

impl FineRom {
    /// 103: 16 KiB of work RAM (`Mapper103.h`, `GetWorkRamSize`).
    pub const RAM_103: usize = 0x4000;
    /// 307: 8 KiB, two 4 KiB pages (`Kaiser7037.h`, `GetWorkRamPageSize`).
    pub const RAM_307: usize = 0x2000;

    pub(crate) fn new(memory: &CartridgeMemory, kind: FineRomKind) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            kind,
            regs: [0; 8],
            pending: 0,
        };
        // nesdev ("INES Mapper 031"): "At power on, the register at $5FFF is set to $FF", and
        // "There is no change to this register on reset". Mesen2 leaves the other seven at 0;
        // puNES powers them on at $F8-$FE.
        if kind == FineRomKind::M31 {
            board.regs[7] = 0xFF;
        }
        board
    }

    /// The 2 KiB PRG-ROM page CPU `address` shows on 302 and 305, for `$6000-$FFFF`.
    fn page(&self, address: u16) -> i64 {
        let r = |i: usize| i64::from(self.regs[i]);
        let slot = usize::from((address - 0x6000) >> 11);
        match self.kind {
            // `Kaiser7057.h` `UpdateState`: `$6000-$7FFF` from R4-R7, `$8000-$9FFF` from R0-R3,
            // then `SelectPrgPage4x(1, 0x34)`, `(2, 0x38)`, `(3, 0x3C)`: 2 KiB pages $34-$3F.
            FineRomKind::M302 => match slot {
                0..=3 => r(4 + slot),
                4..=7 => r(slot - 4),
                _ => 0x34 + i64::try_from(slot - 8).unwrap_or(0),
            },
            // `Kaiser7031.h`: `$6000-$7FFF` from the four registers, `$8000-$FFFF` the pages 15
            // down to 0 (`SelectPrgPage(i, 15 - i)`).
            _ => match slot {
                0..=3 => r(slot),
                _ => 15 - i64::try_from(slot - 4).unwrap_or(0),
            },
        }
    }

    /// Where CPU `address` (`$6000-$FFFF`) lands, `None` for nothing.
    fn target(&self, address: u16, rom_len: i64) -> Option<Target> {
        let low = |mask: u16| i64::from(address & mask);
        Some(match self.kind {
            FineRomKind::M302 | FineRomKind::M305 => {
                Target::Rom(self.page(address) * 0x800 + low(0x07FF))
            }
            // `NsfCart31.h`: eight 4 KiB pages at `$8000-$FFFF`, "SelectPrgPage(addr & 0x07, …)".
            FineRomKind::M31 => {
                if address < 0x8000 {
                    return None;
                }
                let page = i64::from(self.regs[usize::from((address >> 12) & 0x07)]);
                Target::Rom(page * 0x1000 + low(0x0FFF))
            }
            // `Mapper103.h` `UpdateState`: the last 32 KiB at `$8000`, RAM page 1 over
            // `$B800-$D7FF` and RAM page 0 at `$6000` unless `$F000` bit 4 maps the ROM bank of
            // `$8000`'s register there instead.
            FineRomKind::M103 => {
                let rom_mode = self.regs[1] & 0x10 != 0;
                match address {
                    0x6000..=0x7FFF if rom_mode => {
                        Target::Rom(i64::from(self.regs[0]) * 0x2000 + low(0x1FFF))
                    }
                    0x6000..=0x7FFF => Target::Ram(usize::from(address - 0x6000)),
                    0xB800..=0xD7FF if !rom_mode => {
                        Target::Ram(0x2000 + usize::from(address - 0xB800))
                    }
                    _ => Target::Rom(rom_len - 0x8000 + i64::from(address - 0x8000)),
                }
            }
            // `Kaiser7022.h`: one 16 KiB bank at both halves; nothing below `$8000`.
            FineRomKind::M175 => {
                if address < 0x8000 {
                    return None;
                }
                Target::Rom(i64::from(self.regs[0]) * 0x4000 + low(0x3FFF))
            }
            // `Kaiser7037.h` `UpdateState`, in 4 KiB pages: RAM 0, ROM 15, R6 (8 KiB), ROM -4,
            // RAM 1, R7 (8 KiB), the last 8 KiB.
            FineRomKind::M307 => match address {
                0x6000..=0x6FFF => Target::Ram(usize::from(address - 0x6000)),
                0x7000..=0x7FFF => Target::Rom(15 * 0x1000 + low(0x0FFF)),
                0x8000..=0x9FFF => Target::Rom(i64::from(self.regs[6]) * 0x2000 + low(0x1FFF)),
                0xA000..=0xAFFF => Target::Rom(rom_len - 0x4000 + low(0x0FFF)),
                0xB000..=0xBFFF => Target::Ram(0x1000 + usize::from(address - 0xB000)),
                0xC000..=0xDFFF => Target::Rom(i64::from(self.regs[7]) * 0x2000 + low(0x1FFF)),
                _ => Target::Rom(rom_len - 0x2000 + low(0x1FFF)),
            },
        })
    }

    /// 175: the value `$A000` wrote takes effect (`ReadRegister`: "SelectChrPage(0, _reg)").
    fn latch_175(&mut self) {
        self.regs[0] = self.pending;
        self.banks.map_chr_8k(i32::from(self.regs[0]));
    }

    /// The RAM write of 103 and 307, which reaches the RAM whatever the window reads: "Workram is
    /// always writeable, even when PRG ROM is mapped" (`Mapper103.h`, and puNES `mapper_103.c`
    /// in the same words). Answers whether `address` is one.
    fn write_ram(&self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        let offset = match (self.kind, address) {
            (FineRomKind::M103, 0x6000..=0x7FFF) | (FineRomKind::M307, 0x6000..=0x6FFF) => {
                usize::from(address - 0x6000)
            }
            (FineRomKind::M103, 0xB800..=0xD7FF) => 0x2000 + usize::from(address - 0xB800),
            (FineRomKind::M307, 0xB000..=0xBFFF) => 0x1000 + usize::from(address - 0xB000),
            // 103's `$B000-$B7FF` is no register; 307's `RemoveRegisterRange(0xA000, 0xBFFF)`
            // and the ROM at `$7000`: nothing there.
            (FineRomKind::M103, 0xB000..=0xD7FF)
            | (FineRomKind::M307, 0x7000..=0x7FFF | 0xA000..=0xAFFF) => return true,
            _ => return false,
        };
        if let Some(byte) = memory.prg_ram_mut().get_mut(offset) {
            *byte = value;
        }
        true
    }
}

impl Mapper for FineRom {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
        writer.write_u8(self.pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.pending = reader.read_u8()?;
        if self.kind == FineRomKind::M175 {
            self.banks.map_chr_8k(i32::from(self.regs[0]));
        }
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.banks.map_chr_8k(0);
        // 31's registers survive a reset: "There is no change to this register on reset", and
        // Mesen2's `NsfCart31` has no `Reset` either.
        if self.kind == FineRomKind::M31 {
            return;
        }
        self.regs = [0; 8];
        self.pending = 0;
        match self.kind {
            // `Kaiser7031.h` `InitMapper`: "SetMirroringType(MirroringType::Vertical)".
            FineRomKind::M305 => memory.set_mirroring(NametableMirroring::Vertical),
            // `Kaiser7022.h` `Reset`: "_reg = 0; ReadRegister(0xFFFC)".
            FineRomKind::M175 => self.latch_175(),
            _ => {}
        }
    }

    /// `$6000-$7FFF` is decoded here on every board of this file.
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn rom_windows(&self, _memory: &CartridgeMemory) -> [usize; 4] {
        [NO_ROM_WINDOW; 4]
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address < 0x6000 {
            return None;
        }
        let rom = memory.rom().prg_rom();
        let rom_len = i64::try_from(rom.len()).unwrap_or(0);
        match self.target(address, rom_len)? {
            Target::Rom(offset) => {
                let at = usize::try_from(offset.rem_euclid(rom_len.max(1))).unwrap_or(0);
                rom.get(at).copied()
            }
            Target::Ram(offset) => memory.prg_ram().get(offset).copied(),
        }
    }

    /// 175 applies its register on a read of `$FFFC`, the reset vector (`Kaiser7022.h`
    /// `AddRegisterRange(0xFFFC, 0xFFFC, …)`, FCEUX `175.cpp` `if (A == 0xFFFC)`); puNES on any
    /// read of `$F000-$FFFF`.
    fn cpu_read_override(&mut self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if self.kind == FineRomKind::M175 && address == 0xFFFC {
            self.latch_175();
        }
        None
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        self.write_ram(memory, address, value)
    }

    fn nametable_page(&self, table: i32) -> i32 {
        // `Kaiser7037.h`: "SetNametables(_regs[2] & 1, _regs[4] & 1, _regs[3] & 1, _regs[5] & 1)".
        if self.kind == FineRomKind::M307 {
            let register = match table & 0x03 {
                0 => 2,
                1 => 4,
                2 => 3,
                _ => 5,
            };
            return i32::from(self.regs[register] & 0x01);
        }
        -1
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match self.kind {
            // nesdev: "0101 .... .... .AAA", $5000-$5FFF.
            FineRomKind::M31 => {
                if (0x5000..=0x5FFF).contains(&address) {
                    self.regs[usize::from(address & 0x07)] = value;
                }
            }
            FineRomKind::M103 => match address & 0xF000 {
                0x8000 => self.regs[0] = value & 0x0F,
                0xE000 => memory.set_mirroring(if value & 0x08 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                }),
                0xF000 => self.regs[1] = value,
                _ => {}
            },
            FineRomKind::M175 => match address {
                0x8000 => memory.set_mirroring(if value & 0x04 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                }),
                0xA000 => self.pending = value & 0x0F,
                _ => {}
            },
            // `Kaiser7037.h`: `addr & 0xE001`, `$8000` the index and `$8001` the register.
            FineRomKind::M307 => match address & 0xE001 {
                0x8000 => self.pending = value & 0x07,
                0x8001 => self.regs[usize::from(self.pending & 0x07)] = value,
                _ => {}
            },
            // `Kaiser7057.h` `WriteRegister`: mask `$F002`, A0 picks the nibble.
            FineRomKind::M302 => {
                let index = match address & 0xF002 {
                    0x8000 | 0x8002 | 0x9000 | 0x9002 => {
                        memory.set_mirroring(if value & 0x01 != 0 {
                            NametableMirroring::Vertical
                        } else {
                            NametableMirroring::Horizontal
                        });
                        return;
                    }
                    0xB000 => 0,
                    0xB002 => 1,
                    0xC000 => 2,
                    0xC002 => 3,
                    0xD000 => 4,
                    0xD002 => 5,
                    0xE000 => 6,
                    0xE002 => 7,
                    _ => return,
                };
                let register = &mut self.regs[index];
                *register = if address & 0x01 == 0 {
                    (*register & 0xF0) | (value & 0x0F)
                } else {
                    (*register & 0x0F) | ((value << 4) & 0xF0)
                };
            }
            // `Kaiser7031.h`: "_regs[(addr >> 11) & 0x03] = value".
            FineRomKind::M305 => {
                if address >= 0x8000 {
                    self.regs[usize::from((address >> 11) & 0x03)] = value;
                }
            }
        }
    }
}
