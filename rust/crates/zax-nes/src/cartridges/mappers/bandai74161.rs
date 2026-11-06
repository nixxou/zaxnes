use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 70 and 152, the Bandai board of two 74161 counters and a 7432 (REQ-CORE-102).
///
/// One register over the whole of $8000-$FFFF, a 16 KiB PRG window at $8000 with the last bank
/// fixed at $C000, an 8 KiB CHR bank, and **bus conflicts** on both numbers (each page files the
/// board under "Mappers with bus conflicts"). The two numbers are the same board with one wire
/// different, which is what `mirroring_control` is:
///
/// * **70** — nesdev ("INES Mapper 070"): `$8000-FFFF: [PPPP CCCC]`, four PRG bits and **no
///   mirroring control**; "the version of this board with 1-screen mapper-controlled mirroring
///   is mapper 152".
/// * **152** — nesdev ("INES Mapper 152"): `$8000-FFFF: [MPPP CCCC]`, `M` being one-screen
///   select (`0 = 1ScA`, `1 = 1ScB`) and the PRG field therefore three bits; "the version of
///   this board without mirroring control is mapper 70".
///
/// So the parameter decides two things at once — whether bit 7 is a nametable select and whether
/// it is part of the bank — which is why the test writes `$F0` to both numbers and reads the
/// windows and the layout off each.
///
/// **Mesen 2's two compatibility hacks are deliberately not copied** (AMD-061 §1): it forces
/// vertical mirroring at power-on "to make Kamen Rider Club … work correctly (bad header)", and
/// it treats mapper 70 as having mirroring control as soon as a game sets bit 7, "a hack to make
/// as many games as possible work without CRC checks" (`Bandai74161_7432.h:21-36`). Neither is
/// the hardware either page describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bandai74161 {
    banks: BankWindows,
    register: u8,
    /// Mapper 152: bit 7 is a one-screen select and not a PRG bank bit.
    mirroring_control: bool,
}

impl Bandai74161 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
            mirroring_control: mapper_number == 152,
        }
    }

    fn update_banks(&mut self) {
        let prg = if self.mirroring_control {
            (self.register >> 4) & 0x07
        } else {
            self.register >> 4
        };
        self.banks.map_prg_16k(0, i32::from(prg));
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_8k(i32::from(self.register & 0x0F));
    }
}

impl Mapper for Bandai74161 {
    banked!();

    fn mapper_number(&self) -> i32 {
        if self.mirroring_control {
            152
        } else {
            70
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        if self.mirroring_control {
            memory.set_mirroring(NametableMirroring::SingleScreenA);
        }

        self.update_banks();
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        self.register = value;
        if self.mirroring_control {
            memory.set_mirroring(if value & 0x80 == 0 {
                NametableMirroring::SingleScreenA
            } else {
                NametableMirroring::SingleScreenB
            });
        }

        self.update_banks();
    }
}
