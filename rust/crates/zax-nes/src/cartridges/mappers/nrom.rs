use super::Mapper;
use crate::cartridges::{wrap_offset, CartridgeMemory, ChrFetch};

/// iNES mapper 0, NROM (REQ-CORE-100).
///
/// No banking at all: 16 or 32 KiB of PRG-ROM at $8000-$FFFF, 8 KiB of CHR at $0000-$1FFF,
/// nametable layout fixed by the header. An NROM-128 board (16 KiB of PRG) shows its PRG
/// twice, at $8000 and at $C000 (nesdev, "NROM").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nrom;

impl Nrom {
    /// iNES mapper number.
    pub const MAPPER_NUMBER: i32 = 0;
}

impl Mapper for Nrom {
    fn mapper_number(&self) -> i32 {
        Self::MAPPER_NUMBER
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address < 0x8000 {
            return None;
        }

        let prg = memory.rom().prg_rom();
        Some(prg[wrap_offset(usize::from(address - 0x8000), prg.len())])
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        memory.read_chr_direct(usize::from(address & 0x1FFF))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        memory.write_chr_direct(usize::from(address & 0x1FFF), value);
    }
}
