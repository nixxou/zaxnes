use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 97, the Irem TAM-S1 of *Kaiketsu Yanchamaru* (REQ-CORE-102).
///
/// nesdev ("INES Mapper 097"), one register and nothing else:
///
/// ```text
/// $8000-BFFF: [M..P PPPP]
///   M = mirroring, %0 = Horz (PPU A11), %1 = Vert (PPU A10)
///   P = 16 KiB PRG bank at $C000
/// ```
///
/// $8000-$BFFF holds the **last** bank, fixed (`{ -1}` in the page's layout), and the switchable
/// window is the upper one — the opposite way round from every other 16 KiB board in this build,
/// which is the one thing to get right here. "This mapper does not have bus conflicts", and the
/// page documents no IRQ.
///
/// **Two readings differ from Mesen 2 and the page is taken, under AMD-061 §1**
/// ("le matériel tel que le décrit la documentation publique prime sur le comportement du
/// programme d'origine"): `IremTamS1.h` takes the bank from `value & $0F` where the diagram gives
/// five bits, and decodes **four** mirroring modes from bits 7-6 (`ScreenAOnly`, `Horizontal`,
/// `Vertical`, `ScreenBOnly`) where the page gives `M` alone with two. The two agree whenever
/// bits 7 and 6 differ and part company when they are equal; if *Kaiketsu Yanchamaru* is ever
/// seen with the wrong layout, this comment is where to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IremTamS1 {
    banks: BankWindows,
    register: u8,
}

impl IremTamS1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        // The fixed half is the *lower* one: the last 16 KiB bank at $8000-$BFFF.
        self.banks.map_prg_16k(0, -1);
        self.banks.map_prg_16k(2, i32::from(self.register & 0x1F));
    }
}

impl Mapper for IremTamS1 {
    banked!();

    fn mapper_number(&self) -> i32 {
        97
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if !(0x8000..=0xBFFF).contains(&address) {
            return;
        }

        self.register = value;
        memory.set_mirroring(if value & 0x80 == 0 {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        });
        self.update_banks();
    }
}
