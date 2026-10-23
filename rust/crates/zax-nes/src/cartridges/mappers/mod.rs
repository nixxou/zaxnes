// Board names (UxROM, AxROM) read as prose; the register structs carry the IRQ flags of the
// hardware, which are independent bits.
#![allow(clippy::doc_markdown, clippy::struct_excessive_bools)]
//! The mapper boards (port of `src/Zax.Nes/Cartridges/Mappers`).
//!
//! Every board is a small struct that implements [`Mapper`]; the [`super::Board`] enum
//! dispatches to it. The memories shared by all boards stay in the
//! [`CartridgeMemory`] the cartridge owns and are passed to each call.

/// Implements [`Mapper::banks`] for a board with a `banks: BankWindows` field.
macro_rules! banked {
    () => {
        fn banks(&self) -> Option<&$crate::cartridges::BankWindows> {
            Some(&self.banks)
        }
    };
}

mod bandai_fcg;
mod discrete;
mod iremg101;
mod jaleco_ss88006;
mod mmc1;
mod mmc2;
mod mmc3;
mod mmc5;
mod multicart;
mod nrom;
mod rambo1;
mod vrc2_vrc4;

pub use bandai_fcg::BandaiFcg;
pub use discrete::{AxRom, CnRom, Codemasters, ColorDreams, CpRom, GxRom, UnRom512, UxRom};
pub use iremg101::IremG101;
pub use jaleco_ss88006::JalecoSs88006;
pub use mmc1::Mmc1;
pub use mmc2::Mmc2;
pub use mmc3::Mmc3;
pub use mmc5::Mmc5;
pub use multicart::{
    Action52, Action53, Caltron6In1, JalecoJ87, Mapper015, Mapper034, Multicart225,
};
pub use nrom::Nrom;
pub use rambo1::Rambo1;
pub use vrc2_vrc4::Vrc2Vrc4;

use super::{BankWindows, CartridgeMemory, ChrFetch};

/// The behaviour a board adds to the shared cartridge state (REQ-CORE-100). Every method has
/// the default of a board that does nothing special; banked boards expose their
/// [`BankWindows`] through [`Mapper::banks`] and get PRG and CHR access for free.
pub trait Mapper {
    /// iNES mapper number.
    fn mapper_number(&self) -> i32;

    /// The bank windows of a banked board.
    fn banks(&self) -> Option<&BankWindows> {
        None
    }

    /// Puts the registers back to their power-on values and recomputes the windows.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {}

    /// Whether the board asserts the CPU IRQ line.
    fn irq_asserted(&self) -> bool {
        false
    }

    /// True when $6000-$7FFF is readable and writable.
    fn prg_ram_enabled(&self) -> bool {
        true
    }

    /// True when $6000-$7FFF accepts writes.
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_enabled()
    }

    /// Index in PRG-RAM of CPU address `address` in $6000-$7FFF.
    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        usize::from(address - 0x6000) % memory.prg_ram().len()
    }

    /// Reads PRG-ROM, or a readable register, at `address`.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    /// Lets a board answer a CPU read of $4020-$FFFF before the default decode (MMC5
    /// registers, MMC6 RAM halves). `Some(value)` means the board handled it, `value` being
    /// what it drives. Boards with read side effects override this and
    /// [`Mapper::cpu_peek_override`] separately.
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.cpu_peek_override(memory, address)
    }

    /// Side-effect-free counterpart of [`Mapper::cpu_read_override`].
    fn cpu_peek_override(&self, _memory: &CartridgeMemory, _address: u16) -> Option<Option<u8>> {
        None
    }

    /// Lets a board take a CPU write before the PRG-RAM and the register decode see it
    /// (boards whose registers sit below $8000). Returns true when the write is consumed.
    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        _address: u16,
        _value: u8,
    ) -> bool {
        false
    }

    /// Handles a write that is not PRG-RAM.
    fn write_register(&mut self, _memory: &mut CartridgeMemory, _address: u16, _value: u8) {}

    /// Reads CHR as the PPU would, side effects included.
    fn ppu_read(&mut self, memory: &mut CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.ppu_peek(memory, address, fetch)
    }

    /// Reads CHR without any side effect.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        self.banks().map_or(0, |banks| {
            let chr_rom = memory.rom().chr_rom();
            banks.read_chr(
                address,
                if chr_rom.is_empty() {
                    memory.chr_ram()
                } else {
                    chr_rom
                },
            )
        })
    }

    /// Writes CHR-RAM; ignored on a CHR-ROM board.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if memory.rom().chr_rom().is_empty() {
            if let Some(banks) = self.banks() {
                banks.write_chr(address, value, memory.chr_ram_mut());
            }
        }
    }

    /// Opaque CHR latch state (MMC2/MMC4).
    fn chr_latch_state(&self) -> i32 {
        0
    }

    /// Restores the CHR latch state.
    fn set_chr_latch_state(&mut self, _state: i32) {}

    /// Moves the CHR latches on as the fetch of `tile` from `pattern_table` would.
    fn advance_chr_latch(&mut self, _pattern_table: i32, _tile: i32) {}

    /// The PPU put `address` on its bus (A12 counters).
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, _address: u16, _clock: i64) {}

    /// A write to PPUCTRL.
    fn on_ppu_control(&mut self, _control: u8) {}

    /// The start of a scanline.
    fn on_ppu_scanline(&mut self, _memory: &mut CartridgeMemory, _scanline: i32, _rendering: bool) {
    }

    /// Physical nametable page of logical table `table`, or a negative `FOLLOW_MIRRORING` /
    /// `SERVED_BY_CARTRIDGE`.
    fn nametable_page(&self, _table: i32) -> i32 {
        -1
    }

    /// Reads a nametable byte the board serves itself.
    fn read_nametable(&self, _address: u16) -> u8 {
        0
    }

    /// Writes a nametable byte the board serves itself.
    fn write_nametable(&mut self, _address: u16, _value: u8) {}

    /// One CPU cycle elapsed (`memory.cpu_cycle()` already incremented).
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {}
}

/// A fresh window table sized for the memories of `memory`.
pub(crate) fn windows_for(memory: &CartridgeMemory) -> BankWindows {
    let chr_rom = memory.rom().chr_rom().len();
    let chr_len = if chr_rom == 0 {
        memory.chr_ram().len()
    } else {
        chr_rom
    };
    BankWindows::new(
        memory.rom().prg_rom().len(),
        chr_len,
        memory.prg_ram().len(),
    )
}
