//! iNES mapper 188, Bandai's *Karaoke Studio* (lot MAPPERS-N): a 16 KiB PRG bank chosen from the
//! cartridge's own ROM or from an expansion cartridge plugged into it, and the microphone, whose
//! two buttons and one-bit level the CPU reads at `$6000-$7FFF`.
//!
//! nesdev ("INES Mapper 188"), Mesen2 `Bandai/BandaiKaraoke.h` and puNES `mapper_188.c` agree on
//! the bank register (read, not copied):
//!
//! * a write anywhere in `$8000-$FFFF`, "Subject to bus conflicts: Yes": bit 4 "1: Select internal
//!   ROM", 0 the external one; bits 3-0 the bank, which Mesen2 takes as `value & 7` for the
//!   internal ROM and `(value & 7) | 8` for the expansion, the second 128 KiB of a 256 KiB image;
//!   bit 5 "CIRAM A10 is connected to 0: PPU A10 ("vertical mirroring"), 1: PPU A11";
//! * "The bank from $C000-$FFFF is fixed to the last bank of the internal ROM", bank 7;
//! * with no expansion ROM in the image (128 KiB), the external selection reads open bus — nesdev
//!   warns that "several emulators" map the internal ROM's last bank there instead and that
//!   "Properly implementing open bus is more accurate"; Mesen2 unmaps the window, and so does
//!   this board, whose windows `$8000-$BFFF` then go through [`Mapper::cpu_read_override`].
//!
//! The microphone read: bit 0 "0: A button is pressed", bit 1 "0: B button is pressed", bit 2
//! "1-bit ADC microphone input", bits 7-3 open bus. The two buttons are controller 1's A and B,
//! as Mesen2's `BandaiMicrophone` reads them from a key binding of their own; the level stays 0,
//! there being no microphone input here, and the open bus is the high byte of the address, which
//! an absolute read leaves on the bus (FIDELITY-LEADS J23). puNES answers a constant 3.
use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Mapper 188.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandaiKaraoke {
    banks: BankWindows,
    reg: u8,
    /// Whether the image carries the expansion ROM (256 KiB), settled at load.
    has_expansion: bool,
    /// Controller 1's A and B this frame, in bits 0 and 1 (pressed = 1).
    buttons: u8,
}

impl BandaiKaraoke {
    /// The internal ROM, 128 KiB.
    const INTERNAL: usize = 0x2_0000;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            reg: 0,
            has_expansion: memory.rom().prg_rom().len() > Self::INTERNAL,
            buttons: 0,
        };
        board.refresh();
        board
    }

    /// The external ROM selected in an image without one: `$8000-$BFFF` is open bus.
    const fn unmapped(&self) -> bool {
        self.reg & 0x10 == 0 && !self.has_expansion
    }

    fn refresh(&mut self) {
        let bank = if self.reg & 0x10 != 0 {
            self.reg & 0x07
        } else {
            (self.reg & 0x07) | 0x08
        };
        self.banks.map_prg_16k(0, i32::from(bank));
        self.banks.map_prg_16k(2, 7);
    }

    /// The microphone: the two buttons active low, the level 0, bits 7-3 from the bus.
    const fn microphone(&self, address: u16) -> u8 {
        let high = address.to_be_bytes()[0];
        (high & 0xF8) | (!self.buttons & 0x03)
    }
}

impl Mapper for BandaiKaraoke {
    banked!();

    fn mapper_number(&self) -> i32 {
        188
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.reg);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.reg = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.reg = 0;
        self.refresh();
        memory.set_mirroring(NametableMirroring::Vertical);
    }

    fn set_input_word(&mut self, word: u16) {
        self.buttons = (word & 0x03) as u8;
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        let mut windows = self.banks.rom_windows(memory.rom().prg_rom().len());
        if self.unmapped() {
            windows[0] = NO_ROM_WINDOW;
            windows[1] = NO_ROM_WINDOW;
        }
        windows
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.cpu_peek_override(memory, address)
    }

    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        match address {
            0x6000..=0x7FFF => Some(Some(self.microphone(address))),
            0x8000..=0xBFFF if self.unmapped() => Some(None),
            _ => None,
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.reg = value;
        self.refresh();
        memory.set_mirroring(if value & 0x20 != 0 {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        });
    }
}
