//! iNES mapper 111's **other** board: the "non-serialized" MMC1 of the Chinese *Ninja Ryukenden*
//! translation (lot MAPPERS-N, the owner's Hackset check of the old exotic boards).
//!
//! nesdev ("INES Mapper 111"): "Mapper 111 was assigned to a Chinese homebrew translation of Ninja
//! Ryukenden", which "uses a non-serialized version of MMC1 and supports 256KiB of CHR-ROM", and
//! "This assignment can coexist with GTROM provided the emulator recognizes that the translation
//! uses CHR-ROM". So an image with CHR-ROM is this board and one without is
//! [`super::Cheapocabra`] (`registry::create_board`).
//!
//! The register semantics are puNES's (`mapper_111.c:76-82,170-175`, read, not copied): a write
//! anywhere in `$8000-$FFFF` loads the whole byte into the MMC1 register A13-A14 select — control,
//! CHR 0, CHR 1, PRG — with no shift register; the PRG bank keeps 4 bits and the CHR banks 6, which
//! is the 256 KiB. Everything else is the MMC1B's (`init_MMC1(MMC1B, …)`): control `$0C` at
//! power-on, PRG modes and CHR modes, mirroring, and bit 4 of the PRG register disabling the
//! work RAM. Mesen2 and FCEUX implement only GTROM under 111.
//!
//! Its own file and the generic path, not a mode of [`super::Mmc1`]: that board sits on lot
//! CORE-2's specialised path, where every byte of its layout has been measured.
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1Parallel {
    banks: BankWindows,
    /// Control, CHR 0, CHR 1 and PRG, in the A13-A14 order.
    regs: [u8; 4],
}

impl Mmc1Parallel {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            regs: [0x0C, 0, 0, 0],
        }
    }

    fn update(&mut self, memory: &mut CartridgeMemory) {
        let [control, chr0, chr1, prg] = self.regs;
        let bank = i32::from(prg & 0x0F);
        match (control >> 2) & 0x03 {
            0 | 1 => self.banks.map_prg_32k(bank >> 1),
            2 => {
                self.banks.map_prg_16k(0, 0);
                self.banks.map_prg_16k(2, bank);
            }
            _ => {
                self.banks.map_prg_16k(0, bank);
                self.banks.map_prg_16k(2, 0x0F);
            }
        }
        if control & 0x10 != 0 {
            self.banks.map_chr_4k(0, i32::from(chr0 & 0x3F));
            self.banks.map_chr_4k(4, i32::from(chr1 & 0x3F));
        } else {
            self.banks.map_chr_8k(i32::from((chr0 & 0x3F) >> 1));
        }
        memory.set_mirroring(match control & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::SingleScreenB,
            2 => NametableMirroring::Vertical,
            _ => NametableMirroring::Horizontal,
        });
    }
}

impl Mapper for Mmc1Parallel {
    banked!();

    fn mapper_number(&self) -> i32 {
        111
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = [0x0C, 0, 0, 0];
        self.update(memory);
    }

    /// The MMC1B's PRG register bit 4, "PRG-RAM chip enable (0: enabled; 1: disabled)".
    fn prg_ram_enabled(&self) -> bool {
        self.regs[3] & 0x10 == 0
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.regs[usize::from((address >> 13) & 0x03)] = value;
        self.update(memory);
    }
}
