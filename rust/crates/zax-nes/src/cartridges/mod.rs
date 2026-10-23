//! Cartridges (port of `src/Zax.Nes/Cartridges`): the shared cartridge state, the bank
//! window machinery and the mapper boards.
//!
//! A [`Cartridge`] holds the memories every board needs ([`CartridgeMemory`]) and a
//! [`Board`] enum, created once at load time, that carries the mapper registers. Every bus
//! entry point dispatches on the board (REQ-CORE-100).

mod a12_watcher;
mod banked;
mod board;
mod chr_fetch;
pub mod mappers;
pub mod registry;

pub use a12_watcher::A12Watcher;
pub use banked::BankWindows;
pub use board::Board;
pub use chr_fetch::ChrFetch;

use crate::rom::{InesHeader, RomImage, RomLoadError, RomLoadFailure};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;
use mappers::Mapper;

/// `offset % len`, without the division when `offset` is already below `len`: bank offsets
/// almost always fall inside the memory, and this runs on every PRG and CHR access.
#[inline]
pub(crate) const fn wrap_offset(offset: usize, len: usize) -> usize {
    if offset < len {
        offset
    } else {
        offset % len
    }
}

/// The memories and the state every board shares: the ROM image, the PRG-RAM at
/// $6000-$7FFF (REQ-CORE-012), the CHR-RAM buffer, the current nametable layout and the CPU
/// cycle counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeMemory {
    rom: RomImage,
    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
    mirroring: NametableMirroring,
    cpu_cycle: i64,
}

impl CartridgeMemory {
    /// Initialises the shared cartridge state from `rom`; `minimum_prg_ram_size` is for
    /// boards that carry more than 8 KiB of PRG-RAM and bank it (MMC1 SOROM and SXROM,
    /// MMC5).
    fn new(rom: RomImage, minimum_prg_ram_size: usize) -> Self {
        let prg_ram_size = rom
            .header()
            .prg_ram_size
            .max(InesHeader::DEFAULT_PRG_RAM_SIZE)
            .max(minimum_prg_ram_size);
        let chr_ram_size = rom.header().chr_ram_size;
        let mirroring = rom.header().mirroring;
        let mut memory = Self {
            rom,
            prg_ram: vec![0; prg_ram_size],
            chr_ram: vec![0; chr_ram_size],
            mirroring,
            cpu_cycle: 0,
        };
        memory.load_trainer();
        memory
    }

    /// Copies the 512-byte trainer to $7000 inside the PRG-RAM (REQ-CORE-012).
    ///
    /// The reference does not skip the trainer at all; handling it is an accepted
    /// divergence (REQ-CORE-012, notes). It is re-applied whenever PRG-RAM is cleared,
    /// because a power-on must leave the trainer in place.
    fn load_trainer(&mut self) {
        if self.rom.header().has_trainer && self.prg_ram.len() >= 0x1000 + InesHeader::TRAINER_SIZE
        {
            let start = usize::from(InesHeader::TRAINER_LOAD_ADDRESS - 0x6000);
            let trainer = self.rom.trainer();
            self.prg_ram[start..start + trainer.len()].copy_from_slice(trainer);
        }
    }

    /// The ROM image this cartridge was built from.
    #[must_use]
    pub const fn rom(&self) -> &RomImage {
        &self.rom
    }

    /// PRG-RAM at $6000-$7FFF, 8 KiB unless the board or the header asks for more.
    #[must_use]
    pub fn prg_ram(&self) -> &[u8] {
        &self.prg_ram
    }

    /// PRG-RAM, mutably.
    pub fn prg_ram_mut(&mut self) -> &mut [u8] {
        &mut self.prg_ram
    }

    /// CHR-RAM, empty when the cartridge has CHR-ROM.
    #[must_use]
    pub fn chr_ram(&self) -> &[u8] {
        &self.chr_ram
    }

    /// CHR-RAM, mutably.
    pub fn chr_ram_mut(&mut self) -> &mut [u8] {
        &mut self.chr_ram
    }

    /// Current nametable layout.
    #[must_use]
    pub const fn mirroring(&self) -> NametableMirroring {
        self.mirroring
    }

    /// Changes the nametable layout (mappers that switch it).
    pub fn set_mirroring(&mut self, mirroring: NametableMirroring) {
        self.mirroring = mirroring;
    }

    /// CPU cycles elapsed since power-on, as the cartridge sees them.
    #[must_use]
    pub const fn cpu_cycle(&self) -> i64 {
        self.cpu_cycle
    }

    /// Reads CHR-ROM if present, CHR-RAM otherwise, with the address wrapped to the
    /// available size. Helper for the simple boards.
    #[must_use]
    pub fn read_chr_direct(&self, offset: usize) -> u8 {
        let chr_rom = self.rom.chr_rom();
        if !chr_rom.is_empty() {
            return chr_rom[wrap_offset(offset, chr_rom.len())];
        }

        if self.chr_ram.is_empty() {
            0
        } else {
            self.chr_ram[wrap_offset(offset, self.chr_ram.len())]
        }
    }

    /// Writes CHR-RAM, ignoring writes when the cartridge has CHR-ROM.
    pub fn write_chr_direct(&mut self, offset: usize, value: u8) {
        if self.rom.chr_rom().is_empty() && !self.chr_ram.is_empty() {
            let len = self.chr_ram.len();
            self.chr_ram[wrap_offset(offset, len)] = value;
        }
    }
}

/// A cartridge: PRG and CHR banking, nametable layout, PRG-RAM and mapper IRQ
/// (REQ-CORE-100).
///
/// **Side-effect-free reads.** The tile emitter reads pattern data outside of real
/// rendering and must not disturb the machine: no A12 IRQ counting (MMC3), no latch toggling
/// (MMC2/MMC4), no read-buffer change (REQ-CORE-033). Boards with such side effects
/// therefore put them in [`Cartridge::ppu_read`] and [`Cartridge::on_ppu_address_bus`],
/// never in [`Cartridge::ppu_peek`], which is what the tile emitter and the host memory
/// accessors call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cartridge {
    memory: CartridgeMemory,
    board: Board,
}

impl Cartridge {
    /// Returned by [`Cartridge::nametable_page`] when the board leaves the layout to
    /// [`Cartridge::mirroring`], which is what every mapper but MMC5 does.
    pub const FOLLOW_MIRRORING: i32 = -1;

    /// Returned by [`Cartridge::nametable_page`] when the cartridge serves the nametable
    /// itself (MMC5 `ExRAM` and fill mode) through [`Cartridge::read_nametable`].
    pub const SERVED_BY_CARTRIDGE: i32 = -2;

    /// Mapper numbers this build can run, in ascending order (REQ-CORE-102).
    pub const SUPPORTED_MAPPERS: &'static [i32] = registry::SUPPORTED_MAPPERS;

    /// Whether `mapper` is implemented.
    #[must_use]
    pub fn is_supported(mapper: i32) -> bool {
        registry::is_supported(mapper)
    }

    /// Builds the cartridge of `rom`, already powered on, so that a caller which only wants
    /// to look at the banks never sees a board whose registers have not been initialised.
    ///
    /// # Errors
    ///
    /// [`RomLoadFailure::UnsupportedMapper`] with an REQ-CORE-011 message when the mapper
    /// number is not implemented (REQ-CORE-102).
    pub fn create(rom: RomImage) -> Result<Self, RomLoadError> {
        let mapper = rom.mapper();
        if !registry::is_supported(mapper) {
            // Q-001: the wording is ours; REQ-CORE-011 only requires the number to appear,
            // and AMD-002 makes the typed cause the normative part.
            return Err(RomLoadError::new(
                RomLoadFailure::UnsupportedMapper,
                format!("Unsupported mapper {mapper}."),
            ));
        }

        let memory = CartridgeMemory::new(rom, registry::minimum_prg_ram_size(mapper));
        let Some(board) = registry::create_board(mapper, &memory) else {
            return Err(RomLoadError::new(
                RomLoadFailure::UnsupportedMapper,
                format!("Unsupported mapper {mapper}."),
            ));
        };

        let mut cartridge = Self { memory, board };
        cartridge.reset();
        Ok(cartridge)
    }

    /// The shared memories.
    #[must_use]
    pub const fn memory(&self) -> &CartridgeMemory {
        &self.memory
    }

    /// Consumes the cartridge and returns the ROM image it was built from.
    #[must_use]
    pub fn into_rom(self) -> RomImage {
        self.memory.rom
    }

    /// The board and its registers.
    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// The ROM image this cartridge was built from.
    #[must_use]
    pub const fn rom(&self) -> &RomImage {
        &self.memory.rom
    }

    /// iNES mapper number of this cartridge.
    #[must_use]
    pub fn mapper_number(&self) -> i32 {
        self.board.mapper_number()
    }

    /// Current nametable layout (SPEC-02 §2, "current mirroring").
    #[must_use]
    pub const fn mirroring(&self) -> NametableMirroring {
        self.memory.mirroring
    }

    /// Whether the cartridge is asserting the CPU IRQ line.
    #[must_use]
    pub fn irq_asserted(&self) -> bool {
        self.board.irq_asserted()
    }

    /// PPU dots during which the IRQ line cannot change through the PPU's activity, `None`
    /// when the board gives no bound (see [`Mapper::ppu_irq_horizon`]).
    #[must_use]
    pub fn ppu_irq_horizon(&self) -> Option<i64> {
        self.board.ppu_irq_horizon()
    }

    /// CPU cycles elapsed since power-on, as the cartridge sees them.
    ///
    /// Boards that time anything in CPU cycles read it: the MMC1 shift register ignores a
    /// write made on the cycle right after another one, and the VRC4, FCG and SS88006 IRQ
    /// counters tick on it.
    #[must_use]
    pub const fn cpu_cycle(&self) -> i64 {
        self.memory.cpu_cycle
    }

    /// True when $6000-$7FFF is readable and writable. Mappers may disable it.
    fn prg_ram_enabled(&self) -> bool {
        self.board.prg_ram_enabled()
    }

    /// True when $6000-$7FFF accepts writes. Mappers may write-protect it.
    fn prg_ram_writable(&self) -> bool {
        self.board.prg_ram_writable()
    }

    /// Index in PRG-RAM of CPU address `address` in $6000-$7FFF. Boards that bank their
    /// PRG-RAM override it (MMC1 SOROM, MMC5).
    fn prg_ram_offset(&self, address: u16) -> usize {
        self.board.prg_ram_offset(&self.memory, address)
    }

    /// Battery-backed PRG-RAM, exposed so that a host can persist it (REQ-CORE-095,
    /// libretro "`SAVE_RAM`"). Empty when bit 1 of header byte 6 is clear.
    #[must_use]
    pub fn battery_ram(&self) -> &[u8] {
        if self.memory.rom.header().has_battery {
            &self.memory.prg_ram
        } else {
            &[]
        }
    }

    /// Battery-backed PRG-RAM, mutably; empty without a battery.
    pub fn battery_ram_mut(&mut self) -> &mut [u8] {
        if self.memory.rom.header().has_battery {
            &mut self.memory.prg_ram
        } else {
            &mut []
        }
    }

    /// Reads a byte the cartridge drives on the CPU bus, in $4020-$FFFF; `None` when it does
    /// not drive the bus there and the caller keeps the open-bus value (REQ-CORE-081).
    pub fn try_cpu_read(&mut self, address: u16) -> Option<u8> {
        if let Some(handled) = self.board.cpu_read_override(&self.memory, address) {
            return handled;
        }

        self.default_cpu_read(address)
    }

    /// Side-effect-free counterpart of [`Cartridge::try_cpu_read`], used by the host memory
    /// accessor (REQ-CORE-080, REQ-CORE-081).
    #[must_use]
    pub fn try_cpu_peek(&self, address: u16) -> Option<u8> {
        if let Some(handled) = self.board.cpu_peek_override(&self.memory, address) {
            return handled;
        }

        self.default_cpu_read(address)
    }

    fn default_cpu_read(&self, address: u16) -> Option<u8> {
        if (0x6000..=0x7FFF).contains(&address)
            && self.prg_ram_enabled()
            && !self.memory.prg_ram.is_empty()
        {
            return Some(self.memory.prg_ram[self.prg_ram_offset(address)]);
        }

        self.board.try_read_prg(&self.memory, address)
    }

    /// Writes a byte from the CPU bus, in $4020-$FFFF.
    pub fn cpu_write(&mut self, address: u16, value: u8) {
        if self
            .board
            .intercept_cpu_write(&mut self.memory, address, value)
        {
            return;
        }

        if (0x6000..=0x7FFF).contains(&address) && !self.memory.prg_ram.is_empty() {
            if self.prg_ram_writable() {
                let offset = self.prg_ram_offset(address);
                self.memory.prg_ram[offset] = value;
            }

            return;
        }

        self.board.write_register(&mut self.memory, address, value);
    }

    /// Reads CHR at `address` ($0000-$1FFF) as the PPU would, side effects included
    /// (latches, A12 counting).
    pub fn ppu_read(&mut self, address: u16, fetch: ChrFetch) -> u8 {
        self.board.ppu_read(&mut self.memory, address, fetch)
    }

    /// Reads CHR without any side effect. The tile emitter and the host accessors use this
    /// one (REQ-CORE-033).
    #[must_use]
    pub fn ppu_peek(&self, address: u16, fetch: ChrFetch) -> u8 {
        self.board.ppu_peek(&self.memory, address, fetch)
    }

    /// Writes CHR-RAM at `address` ($0000-$1FFF).
    pub fn ppu_write(&mut self, address: u16, value: u8) {
        self.board.ppu_write(&mut self.memory, address, value);
    }

    /// Opaque state of the CHR latches of a board whose banks depend on the patterns fetched
    /// before (MMC2 and MMC4, REQ-CORE-039). Always 0 on every other board.
    ///
    /// The tile emitter reads it, walks the line forward with
    /// [`Cartridge::advance_chr_latch`] so that each record carries the pattern the hardware
    /// would show at that column, then writes the saved value back: the emulation is
    /// unchanged whether or not records were built (REQ-CORE-033).
    #[must_use]
    pub fn chr_latch_state(&self) -> i32 {
        self.board.chr_latch_state()
    }

    /// Restores the opaque CHR latch state (see [`Cartridge::chr_latch_state`]).
    pub fn set_chr_latch_state(&mut self, state: i32) {
        self.board.set_chr_latch_state(state);
    }

    /// Moves the CHR latches on as the fetch of pattern `tile` from `pattern_table` ($0000
    /// or $1000) would (REQ-CORE-039).
    pub fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        self.board.advance_chr_latch(pattern_table, tile);
    }

    /// Signals that the PPU put `address` (14 bits) on its address bus during real rendering
    /// or a $2006/$2007 access. Boards that count A12 rises (MMC3 and compatibles) hook
    /// their scanline counter here. Never called for a peek (REQ-CORE-033).
    ///
    /// `ppu_clock` is the PPU dot count since power-on, which the A12 filter of MMC3-style
    /// boards needs to tell a real rise from the short dips of the fetch pipeline.
    pub fn on_ppu_address_bus(&mut self, address: u16, ppu_clock: i64) {
        self.board
            .on_ppu_address_bus(&mut self.memory, address, ppu_clock);
    }

    /// Signals a write to PPUCTRL ($2000). MMC5 needs it: the sprite size decides whether
    /// its two CHR register sets are used separately (nesdev, "MMC5").
    pub fn on_ppu_control(&mut self, control: u8) {
        self.board.on_ppu_control(control);
    }

    /// Signals the start of a scanline (0 to 239 for a visible line) to boards that count
    /// them outside the PPU address bus (MMC5, whose IRQ follows the rendered line, nesdev
    /// "MMC5").
    pub fn on_ppu_scanline(&mut self, scanline: i32, rendering_enabled: bool) {
        self.board
            .on_ppu_scanline(&mut self.memory, scanline, rendering_enabled);
    }

    /// Which physical 1 KiB page of console VRAM the logical nametable `table` (0 to 3)
    /// shows: a page number 0 to 3, [`Cartridge::FOLLOW_MIRRORING`] when the layout is the
    /// one [`Cartridge::mirroring`] describes, or [`Cartridge::SERVED_BY_CARTRIDGE`] when the
    /// cartridge supplies the bytes itself.
    #[must_use]
    pub fn nametable_page(&self, table: i32) -> i32 {
        self.board.nametable_page(table)
    }

    /// Reads a nametable byte the cartridge serves itself (MMC5 `ExRAM`, fill mode).
    #[must_use]
    pub fn read_nametable(&self, address: u16) -> u8 {
        self.board.read_nametable(address)
    }

    /// Writes a nametable byte the cartridge serves itself (MMC5 `ExRAM`).
    pub fn write_nametable(&mut self, address: u16, value: u8) {
        self.board.write_nametable(address, value);
    }

    /// Advances the cartridge by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        self.memory.cpu_cycle += 1;
        self.board.on_cpu_cycle(&mut self.memory);
    }

    /// Returns the cartridge to its power-on state (REQ-CORE-004). PRG-RAM is cleared by the
    /// core, not here, so that a host which reloaded battery RAM can keep it
    /// (REQ-CORE-095).
    pub fn reset(&mut self) {
        self.memory.mirroring = self.memory.rom.header().mirroring;
        self.memory.cpu_cycle = 0;
        self.board.reset(&mut self.memory);
    }

    /// Clears PRG-RAM (when `clear_prg_ram`) and CHR-RAM (part of power-on, REQ-CORE-004).
    pub fn clear_volatile_ram(&mut self, clear_prg_ram: bool) {
        if clear_prg_ram {
            self.memory.prg_ram.fill(0);
            self.memory.load_trainer();
        }

        self.memory.chr_ram.fill(0);
    }
}

impl Cartridge {
    /// Appends the cartridge state, memories and mapper registers, to `writer`
    /// (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.memory.prg_ram);
        writer.write_blob(&self.memory.chr_ram);
        writer.write_u8(self.memory.mirroring as u8);
        writer.write_i64(self.memory.cpu_cycle);
        self.board.save_state(writer);
    }

    /// Restores the cartridge state from `reader` (REQ-CORE-091). The cartridge must have
    /// been created from the same ROM image the state was saved with.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_blob_into(&mut self.memory.prg_ram)?;
        reader.read_blob_into(&mut self.memory.chr_ram)?;
        let mirroring = reader.read_u8()?;
        self.memory.mirroring = NametableMirroring::from_u8(mirroring).ok_or_else(|| {
            InvalidState::corrupt(format!(
                "Corrupt savestate: unknown nametable layout {mirroring}."
            ))
        })?;
        self.memory.cpu_cycle = reader.read_i64()?;
        self.board.load_state(reader)
    }
}
