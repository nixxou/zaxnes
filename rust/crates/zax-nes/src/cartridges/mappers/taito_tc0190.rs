use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 33, the Taito TC0190 / TC0350FMC (REQ-CORE-102).
///
/// nesdev ("INES Mapper 033"), register mask **$A003**, so the eight registers repeat over the
/// whole of $8000-$FFFF:
///
/// * `$8000: [.MPP PPPP]` — PRG register 0, 8 KiB at $8000, and `M` the mirroring bit;
/// * `$8001: [..PP PPPP]` — PRG register 1, 8 KiB at $A000;
/// * `$8002`, `$8003` — CHR registers 0 and 1, **2 KiB** at $0000 and $0800;
/// * `$A000`-`$A003` — CHR registers 2 to 5, 1 KiB at $1000, $1400, $1800 and $1C00.
///
/// The last two 8 KiB PRG banks are fixed to the end. The page's own note on the CHR registers is
/// the one claim that separates this board from an MMC3: the two 2 KiB registers "use the full
/// written value (not dropping the LSB like MMC3), allowing 512 KiB addressability", where the
/// four 1 KiB ones reach 256 KiB.
///
/// `IRQ: None` — "Mapper 033 does not have IRQs" — which with no PRG-RAM is what the page gives
/// as the whole of the difference from **mapper 48**, the TC0690 that MF-6b adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaitoTc0190 {
    banks: BankWindows,
    prg: [u8; 2],
    chr: [u8; 6],
    mirroring: u8,
}

impl TaitoTc0190 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 1],
            chr: [0; 6],
            mirroring: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_8k(0, i32::from(self.prg[0] & 0x3F));
        self.banks.map_prg_8k(1, i32::from(self.prg[1] & 0x3F));
        self.banks.map_prg_8k(2, -2);
        self.banks.map_prg_8k(3, -1);

        // The whole byte, LSB included: a 2 KiB bank number here, not a 1 KiB one with its low
        // bit dropped.
        self.banks.map_chr_2k(0, i32::from(self.chr[0]));
        self.banks.map_chr_2k(2, i32::from(self.chr[1]));
        for window in 0..4 {
            self.banks
                .map_chr_1k(4 + window, i32::from(self.chr[2 + window]));
        }
    }
}

impl Mapper for TaitoTc0190 {
    banked!();

    fn mapper_number(&self) -> i32 {
        33
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.mirroring);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.mirroring = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.prg = [0, 1];
        self.chr = [0; 6];
        self.mirroring = 0;
        memory.set_mirroring(NametableMirroring::Vertical);
        self.update_banks();
    }

    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and this lot keeps the five boards of step 2 out of
    /// that function for the same reason.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xA003 {
            0x8000 => {
                self.prg[0] = value;
                self.mirroring = value & 0x40;
                memory.set_mirroring(if self.mirroring == 0 {
                    NametableMirroring::Vertical
                } else {
                    NametableMirroring::Horizontal
                });
            }
            0x8001 => self.prg[1] = value,
            0x8002 => self.chr[0] = value,
            0x8003 => self.chr[1] = value,
            address @ 0xA000..=0xA003 => {
                self.chr[2 + usize::from(address & 0x03)] = value;
            }
            _ => return,
        }

        self.update_banks();
    }
}
