//! iNES mapper 234, the AVE "Maxi 15" multicart (lot MAPPERS-N): its two registers latch on a CPU
//! **read** as well as a write of their addresses, inside the PRG-ROM's own last page.
//!
//! Read from Mesen2 `Unlicensed/Mapper234.h` and puNES `mapper_234.c` (read, not copied):
//!
//! * register 0 at `$FF80-$FF9F` (outer bank, mode, mirroring) takes the byte only while its low
//!   six bits are zero — once a game is chosen it locks;
//! * register 1 at `$FFE8-$FFF7` (inner banks) always takes it, as puNES decodes it
//!   (`$FFE8`/`$FFF0` under `$FFF8`); Mesen2 extends the range to `$FFF8` (FIDELITY-LEADS J18);
//! * a read latches the byte the ROM returns (`ReadRegister`, `extcl_cpu_rd_mem_234`), so a game
//!   can select a bank with `LDA $FF80,X` from a table in that page;
//! * bit 6 of register 0 picks NINA-03 mode (PRG A15 from register 1, CHR A13-A15 from it) or
//!   CNROM mode (PRG from register 0 alone, CHR A13-A14 from register 1).
//!
//! A read can switch the 32 KiB bank, and the cartridge refreshes its cached windows only after a
//! write, so no window is cacheable ([`super::NO_ROM_WINDOW`]): every read of `$8000-$FFFF` comes
//! through [`Mapper::cpu_read_override`].
use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maxi15 {
    banks: BankWindows,
    regs: [u8; 2],
    /// Set by a latch that happened during a read; consumed by the next write path's refresh.
    mirroring: NametableMirroring,
}

impl Maxi15 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            regs: [0; 2],
            mirroring: NametableMirroring::Vertical,
        };
        board.update();
        board
    }

    fn update(&mut self) {
        let [r0, r1] = self.regs.map(i32::from);
        if r0 & 0x40 != 0 {
            self.banks.map_prg_32k((r0 & 0x0E) | (r1 & 0x01));
            self.banks
                .map_chr_8k(((r0 & 0x0E) << 2) | ((r1 >> 4) & 0x07));
        } else {
            self.banks.map_prg_32k(r0 & 0x0F);
            self.banks
                .map_chr_8k(((r0 & 0x0F) << 2) | ((r1 >> 4) & 0x03));
        }
        self.mirroring = if r0 & 0x80 != 0 {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        };
    }

    /// Latches `value` if `address` is a register; answers whether it was.
    fn latch(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xFF80..=0xFF9F => {
                // Its low six bits clear: no game chosen yet.
                if self.regs[0].trailing_zeros() >= 6 {
                    self.regs[0] = value;
                }
            }
            0xFFE8..=0xFFF7 => self.regs[1] = value,
            _ => return false,
        }
        self.update();
        true
    }
}

impl Mapper for Maxi15 {
    banked!();

    fn mapper_number(&self) -> i32 {
        234
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.update();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = [0; 2];
        self.update();
        memory.set_mirroring(self.mirroring);
    }

    fn rom_windows(&self, _memory: &CartridgeMemory) -> [usize; 4] {
        [NO_ROM_WINDOW; 4]
    }

    /// A read of a register latches the ROM byte under it (`Mapper234.h` `ReadRegister`). The
    /// mirroring it may change is applied at the next write, the only place a board reaches the
    /// cartridge memory mutably — a game sets it from the menu's write before any picture.
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if address < 0x8000 {
            return None;
        }
        let value = self.banks.read_prg(address, memory.rom().prg_rom(), &[]);
        if let Some(byte) = value {
            if (0xFF80..=0xFFF7).contains(&address) {
                self.latch(address, byte);
            }
        }
        Some(value)
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        (address >= 0x8000).then(|| self.banks.read_prg(address, memory.rom().prg_rom(), &[]))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.latch(address, value);
        memory.set_mirroring(self.mirroring);
    }
}
