//! The [`Board`] enum: one variant per mapper family, dispatching [`Mapper`] to it
//! (REQ-CORE-100). Created once at load time by the registry.

use super::mappers::{
    Action52, Action53, AxRom, BandaiFcg, Caltron6In1, CnRom, Codemasters, ColorDreams, CpRom,
    GxRom, IremG101, JalecoJ87, JalecoSs88006, Mapper, Mapper015, Mapper034, Mmc1, Mmc2, Mmc3,
    Mmc5, Multicart225, Nrom, Rambo1, UnRom512, UxRom, Vrc2Vrc4,
};
use super::{BankWindows, CartridgeMemory, ChrFetch};

/// The mapper of a cartridge, with its registers (REQ-CORE-100).
// Created once at load time, so the size of the MMC5 variant (its ExRAM) costs nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Board {
    /// Mapper 0.
    Nrom(Nrom),
    /// Mapper 1.
    Mmc1(Mmc1),
    /// Mapper 2.
    UxRom(UxRom),
    /// Mapper 3.
    CnRom(CnRom),
    /// Mapper 4.
    Mmc3(Mmc3),
    /// Mapper 5.
    Mmc5(Mmc5),
    /// Mapper 7.
    AxRom(AxRom),
    /// Mappers 9 and 10.
    Mmc2(Mmc2),
    /// Mapper 11.
    ColorDreams(ColorDreams),
    /// Mapper 13.
    CpRom(CpRom),
    /// Mapper 15.
    Mapper015(Mapper015),
    /// Mapper 16.
    BandaiFcg(BandaiFcg),
    /// Mapper 18.
    JalecoSs88006(JalecoSs88006),
    /// Mappers 21, 22, 23, 25 and 27.
    Vrc2Vrc4(Vrc2Vrc4),
    /// Mapper 28.
    Action53(Action53),
    /// Mapper 30.
    UnRom512(UnRom512),
    /// Mapper 32.
    IremG101(IremG101),
    /// Mapper 34.
    Mapper034(Mapper034),
    /// Mapper 41.
    Caltron6In1(Caltron6In1),
    /// Mapper 64.
    Rambo1(Rambo1),
    /// Mapper 66.
    GxRom(GxRom),
    /// Mapper 71.
    Codemasters(Codemasters),
    /// Mappers 87 and 101.
    JalecoJ87(JalecoJ87),
    /// Mapper 225.
    Multicart225(Multicart225),
    /// Mapper 228.
    Action52(Action52),
}

/// Expands `$body` once per variant with the board bound to `$b`.
macro_rules! dispatch {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            Board::Nrom($b) => $body,
            Board::Mmc1($b) => $body,
            Board::UxRom($b) => $body,
            Board::CnRom($b) => $body,
            Board::Mmc3($b) => $body,
            Board::Mmc5($b) => $body,
            Board::AxRom($b) => $body,
            Board::Mmc2($b) => $body,
            Board::ColorDreams($b) => $body,
            Board::CpRom($b) => $body,
            Board::Mapper015($b) => $body,
            Board::BandaiFcg($b) => $body,
            Board::JalecoSs88006($b) => $body,
            Board::Vrc2Vrc4($b) => $body,
            Board::Action53($b) => $body,
            Board::UnRom512($b) => $body,
            Board::IremG101($b) => $body,
            Board::Mapper034($b) => $body,
            Board::Caltron6In1($b) => $body,
            Board::Rambo1($b) => $body,
            Board::GxRom($b) => $body,
            Board::Codemasters($b) => $body,
            Board::JalecoJ87($b) => $body,
            Board::Multicart225($b) => $body,
            Board::Action52($b) => $body,
        }
    };
}

impl Mapper for Board {
    fn mapper_number(&self) -> i32 {
        dispatch!(self, b => b.mapper_number())
    }

    fn banks(&self) -> Option<&BankWindows> {
        dispatch!(self, b => b.banks())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        dispatch!(self, b => b.reset(memory));
    }

    fn irq_asserted(&self) -> bool {
        dispatch!(self, b => b.irq_asserted())
    }

    fn prg_ram_enabled(&self) -> bool {
        dispatch!(self, b => b.prg_ram_enabled())
    }

    fn prg_ram_writable(&self) -> bool {
        dispatch!(self, b => b.prg_ram_writable())
    }

    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        dispatch!(self, b => b.prg_ram_offset(memory, address))
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        dispatch!(self, b => b.try_read_prg(memory, address))
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        dispatch!(self, b => b.cpu_read_override(memory, address))
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        dispatch!(self, b => b.cpu_peek_override(memory, address))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        dispatch!(self, b => b.intercept_cpu_write(memory, address, value))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.write_register(memory, address, value));
    }

    fn ppu_read(&mut self, memory: &mut CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        dispatch!(self, b => b.ppu_read(memory, address, fetch))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        dispatch!(self, b => b.ppu_peek(memory, address, fetch))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.ppu_write(memory, address, value));
    }

    fn chr_latch_state(&self) -> i32 {
        dispatch!(self, b => b.chr_latch_state())
    }

    fn set_chr_latch_state(&mut self, state: i32) {
        dispatch!(self, b => b.set_chr_latch_state(state));
    }

    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        dispatch!(self, b => b.advance_chr_latch(pattern_table, tile));
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        dispatch!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }

    fn on_ppu_control(&mut self, control: u8) {
        dispatch!(self, b => b.on_ppu_control(control));
    }

    fn on_ppu_scanline(&mut self, memory: &mut CartridgeMemory, scanline: i32, rendering: bool) {
        dispatch!(self, b => b.on_ppu_scanline(memory, scanline, rendering));
    }

    fn nametable_page(&self, table: i32) -> i32 {
        dispatch!(self, b => b.nametable_page(table))
    }

    fn read_nametable(&self, address: u16) -> u8 {
        dispatch!(self, b => b.read_nametable(address))
    }

    fn write_nametable(&mut self, address: u16, value: u8) {
        dispatch!(self, b => b.write_nametable(address, value));
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        dispatch!(self, b => b.on_cpu_cycle(memory));
    }
}
