//! Small unlicensed boards of lot MAPPERS-N with one or two registers and one quirk each, read
//! from Mesen2 (`cleanroom/third_party/mesen2/Core/NES/Mappers/Unlicensed/…`) beside puNES and
//! nesdev (read, not copied; FIDELITY-LEADS J26):
//!
//! * 53, `Supervision.h`: the *Supervision 16-in-1*, an outer register at `$6000-$7FFF` and an
//!   inner one at `$8000-$FFFF`, `$6000-$7FFF` reading ROM, and an optional 32 KiB EPROM ahead
//!   of the main ROM recognised by its CRC;
//! * 487, `Mapper487.h`: a NINA-03 register at `$4100` (A8 set, A7 clear), an outer register at
//!   `$4180` (A8 and A7 set), and in its second half a Color Dreams register at `$8000-$FFFF`;
//! * 518, `Dance2000.h`: `$5000` PRG and `$5200` mode, whose bit 1 switches CHR's first 4 KiB with
//!   the nametable the PPU addresses, and whose `$5000` bit 6 opens the bus over `$8000-$FFFF`;
//! * 519, `Eh8813A.h`: the bank in the **address** of a `$8000-$FFFF` write with A8 clear, and a
//!   mode (A6) where reads see the DIP switches in place of the low nibble of the address;
//! * 328, `Rt01.h`: the RT-01 test cartridge, one 32 KiB bank and one 8 KiB of CHR, whose reads
//!   of `$CE80-$CEFF` and `$FE80-$FEFF` return `$F2` with bits 0, 2 and 3 random — here from a
//!   hash of the CPU cycle, so that a replay reads the same bytes (FIDELITY-LEADS J32).
use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiscKind {
    /// 53, Supervision 16-in-1.
    M53,
    /// 487.
    M487,
    /// 518, Dance 2000.
    M518,
    /// 519, EH8813A.
    M519,
    /// 328, RT-01.
    M328,
}

impl MiscKind {
    /// The kind of an iNES number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        match mapper {
            53 => Some(Self::M53),
            487 => Some(Self::M487),
            518 => Some(Self::M518),
            519 => Some(Self::M519),
            328 => Some(Self::M328),
            _ => None,
        }
    }

    const fn number(self) -> i32 {
        match self {
            Self::M53 => 53,
            Self::M487 => 487,
            Self::M518 => 518,
            Self::M519 => 519,
            Self::M328 => 328,
        }
    }
}

/// A board of this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiscN {
    banks: BankWindows,
    kind: MiscKind,
    /// The registers, their meaning per kind.
    regs: [u8; 3],
    /// 518: the 4 KiB CHR page the PPU's nametable address picked; 519: the read mode.
    state: u8,
    /// 53: the 32 KiB EPROM is present ahead of the main ROM. Settled at load.
    eprom: bool,
}

impl MiscN {
    /// 53: the CRC-32 of the EPROM's 32 KiB (`Supervision.h`, `EPROM_CRC`).
    const EPROM_CRC: u32 = 0x6379_4E25;
    /// 519: the DIP switches, none set.
    const DIP: u16 = 0;

    pub(crate) fn new(memory: &CartridgeMemory, kind: MiscKind) -> Self {
        let rom = memory.rom().prg_rom();
        let eprom = kind == MiscKind::M53
            && rom.len() >= 0x8000
            && crate::builtin_patches::crc32(&rom[..0x8000]) == Self::EPROM_CRC;
        Self {
            banks: windows_for(memory),
            kind,
            regs: [0; 3],
            state: 0,
            eprom,
        }
    }

    /// 53's 8 KiB ROM bank at `$6000-$7FFF`.
    fn bank_6000(&self) -> i32 {
        let r = i32::from((self.regs[0] << 3) & 0x78);
        ((r << 1) | 0x0F) + if self.eprom { 0x04 } else { 0 }
    }

    /// Maps the banks and answers the mirroring, `None` to leave it.
    fn update(&mut self) -> Option<NametableMirroring> {
        let [r0, r1, _] = self.regs.map(i32::from);
        let horizontal = |bit: bool| {
            Some(if bit {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            })
        };
        match self.kind {
            // `Supervision.h` `UpdateState`, in 16 KiB banks.
            MiscKind::M53 => {
                let r = (r0 << 3) & 0x78;
                let eprom = if self.eprom { 0x02 } else { 0 };
                let (low, high) = if r0 & 0x10 != 0 {
                    ((r | (r1 & 0x07)) + eprom, (r | 0x07) + eprom)
                } else if self.eprom {
                    (0x00, 0x01)
                } else {
                    (0x80, 0x81)
                };
                self.banks.map_prg_16k(0, low);
                self.banks.map_prg_16k(2, high);
                self.banks.map_chr_8k(0);
                horizontal(r0 & 0x20 != 0)
            }
            // `Mapper487.h` `UpdateState`.
            MiscKind::M487 => {
                let mut prg = r1 & 0x1E;
                let mut chr = ((r1 & 0x1E) << 2) | (r0 & 0x03);
                if r1 & 0x40 != 0 {
                    prg |= (r0 & 0x08) >> 3;
                    chr |= r0 & 0x04;
                } else {
                    prg |= r1 & 0x01;
                    chr |= (r1 & 0x01) << 2;
                }
                if r1 & 0x20 != 0 {
                    prg += 0x10;
                    chr += 0x40;
                }
                self.banks.map_prg_32k(prg);
                self.banks.map_chr_8k(chr);
                horizontal(r1 & 0x80 != 0)
            }
            // `Dance2000.h` `UpdateState`: `_mode` in `regs[1]`.
            MiscKind::M518 => {
                self.banks.map_chr_4k(0, i32::from(self.state));
                self.banks.map_chr_4k(4, 1);
                if r1 & 0x04 != 0 {
                    self.banks.map_prg_32k(r0 & 0x07);
                } else {
                    self.banks.map_prg_16k(0, r0 & 0x0F);
                    self.banks.map_prg_16k(2, 0);
                }
                horizontal(r1 & 0x01 != 0)
            }
            // `Rt01.h` `InitMapper`: PRG page 0 twice, CHR page 0 four times; puNES `mapper_328.c`
            // maps 32 KiB bank 0. The same windows once wrapped by a 16 KiB / 2 KiB image.
            MiscKind::M328 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_8k(0);
                None
            }
            // `Eh8813A.h` `WriteRegister`: the latched address in `regs[0..2]`, CHR in `regs[2]`.
            MiscKind::M519 => {
                let address = u16::from_le_bytes([self.regs[0], self.regs[1]]);
                let bank = i32::from(address & 0x07);
                if address & 0x80 != 0 {
                    self.banks.map_prg_16k(0, bank);
                    self.banks.map_prg_16k(2, bank);
                } else {
                    self.banks.map_prg_32k(bank >> 1);
                }
                self.banks.map_chr_8k(i32::from(self.regs[2] & 0x0F));
                None
            }
        }
    }

    fn apply(&mut self, memory: &mut CartridgeMemory) {
        if let Some(mirroring) = self.update() {
            memory.set_mirroring(mirroring);
        }
    }

    /// 518 with `$5000` bit 6 and 519 in its DIP mode answer `$8000-$FFFF` themselves.
    const fn reads_differ(&self) -> bool {
        match self.kind {
            MiscKind::M518 => self.regs[0] & 0x40 != 0,
            MiscKind::M519 => self.state != 0,
            MiscKind::M53 | MiscKind::M487 | MiscKind::M328 => false,
        }
    }
}

impl Mapper for MiscN {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
        writer.write_u8(self.state);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.state = reader.read_u8()?;
        self.update();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = [0; 3];
        self.state = 0;
        // `Eh8813A.h` `InitMapper`: "SetMirroringType(MirroringType::Vertical)"; the other three
        // set theirs from a register of zero, vertical too.
        memory.set_mirroring(NametableMirroring::Vertical);
        self.apply(memory);
    }

    /// 53 shows ROM at `$6000-$7FFF`.
    fn prg_ram_enabled(&self) -> bool {
        self.kind != MiscKind::M53
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.reads_differ() {
            return [NO_ROM_WINDOW; 4];
        }
        let mut windows = self.banks.rom_windows(memory.rom().prg_rom().len());
        // 328's two random ranges sit in the windows of `$C000` and `$E000`.
        if self.kind == MiscKind::M328 {
            windows[2] = NO_ROM_WINDOW;
            windows[3] = NO_ROM_WINDOW;
        }
        windows
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        let rom = memory.rom().prg_rom();
        if self.kind == MiscKind::M53 && (0x6000..0x8000).contains(&address) {
            let count = i32::try_from(rom.len() / 0x2000).unwrap_or(1).max(1);
            let bank = usize::try_from(self.bank_6000().rem_euclid(count)).unwrap_or(0);
            return rom
                .get(bank * 0x2000 + usize::from(address & 0x1FFF))
                .copied();
        }
        if address < 0x8000 {
            return self.banks.read_prg(address, rom, memory.prg_ram());
        }
        match self.kind {
            // `Rt01.h` `ReadRegister`: "0xF2 | (rand() & 0x0D)".
            MiscKind::M328 if matches!(address, 0xCE80..=0xCEFF | 0xFE80..=0xFEFF) => {
                let cycle = memory.cpu_cycle().to_le_bytes();
                let mixed = cycle.iter().fold(0x5Au8, |hash, byte| {
                    hash.rotate_left(3) ^ byte.wrapping_mul(0x9D)
                });
                Some(0xF2 | (mixed & 0x0D))
            }
            // `Dance2000.h` `ReadRegister`: "(_prgReg & 0x40) ? … GetOpenBus()".
            MiscKind::M518 if self.reads_differ() => None,
            // `Eh8813A.h` `ReadRegister`: "addr = (addr & 0xFFF0) + GetDipSwitches()".
            MiscKind::M519 if self.reads_differ() => {
                self.banks
                    .read_prg((address & 0xFFF0) + Self::DIP, rom, &[])
            }
            _ => self.banks.read_prg(address, rom, &[]),
        }
    }

    /// `Dance2000.h` `NotifyVramAddressChange`: with `$5200` bit 1, a nametable address picks
    /// CHR's first 4 KiB; without it, that page goes back to 0.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if self.kind != MiscKind::M518 {
            return;
        }
        let page = if self.regs[1] & 0x02 != 0 {
            if address & 0x3000 != 0x2000 {
                return;
            }
            u8::from(address & 0x0800 != 0)
        } else {
            0
        };
        if page != self.state {
            self.state = page;
            self.banks.map_chr_4k(0, i32::from(page));
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match self.kind {
            // `Rt01.h`: no register.
            MiscKind::M328 => return,
            MiscKind::M53 => {
                if address < 0x6000 {
                    return;
                }
                self.regs[usize::from(address >= 0x8000)] = value;
            }
            MiscKind::M487 => {
                if address < 0x6000 {
                    if address & 0x100 == 0 || address < 0x4100 {
                        return;
                    }
                    if address & 0x80 != 0 {
                        self.regs[1] = value;
                    } else if self.regs[1] & 0x20 == 0 {
                        self.regs[0] = value & 0x0F;
                    }
                } else if address >= 0x8000 && self.regs[1] & 0x20 != 0 {
                    self.regs[0] = ((value & 0x01) << 3) | ((value & 0x70) >> 4);
                } else {
                    return;
                }
            }
            MiscKind::M518 => match address {
                0x5000 => self.regs[0] = value,
                0x5200 => {
                    self.regs[1] = value;
                    // Only bit 2 rebuilds the banks at once (`if(_mode & 0x04) UpdateState()`).
                    if value & 0x04 == 0 {
                        return;
                    }
                }
                _ => return,
            },
            MiscKind::M519 => {
                if address < 0x8000 || address & 0x0100 != 0 {
                    return;
                }
                [self.regs[0], self.regs[1]] = address.to_le_bytes();
                self.regs[2] = value;
                self.state = u8::from(address & 0x40 != 0);
            }
        }
        self.apply(memory);
    }
}
