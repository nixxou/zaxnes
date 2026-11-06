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
pub use banked::{BankWindows, ChrView};
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
    /// First byte of `prg_ram` the cartridge's battery retains; the rest of the RAM comes up
    /// uninitialised (see [`registry::battery_offset`]). Derived from the ROM at load, so it is
    /// not part of the savestate.
    battery_offset: usize,
}

impl CartridgeMemory {
    /// Initialises the shared cartridge state from `rom`, with the volatile-memory sizes the
    /// board dictates (see [`registry::BoardMemorySizes`]): an exact size for a board whose
    /// RAM the header cannot describe, a floor for one that banks more than 8 KiB.
    fn new(rom: RomImage, sizes: registry::BoardMemorySizes) -> Self {
        let prg_ram_size = sizes.prg_ram.unwrap_or_else(|| {
            rom.header()
                .prg_ram_size
                .max(InesHeader::DEFAULT_PRG_RAM_SIZE)
                .max(sizes.min_prg_ram)
        });
        let chr_ram_size = sizes.chr_ram.unwrap_or(rom.header().chr_ram_size);
        let mirroring = rom.header().mirroring;
        let battery_offset = registry::battery_offset(rom.mapper(), prg_ram_size);
        let mut memory = Self {
            rom,
            prg_ram: vec![0; prg_ram_size],
            chr_ram: vec![0; chr_ram_size],
            mirroring,
            cpu_cycle: 0,
            battery_offset,
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

    /// PRG-ROM, mutably: the one board that writes its own is the UNROM 512, whose PRG is
    /// flash and not mask ROM (MR-30-4). No other board calls this.
    pub(crate) fn prg_rom_mut(&mut self) -> &mut [u8] {
        self.rom.prg_rom_mut()
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
// Four independent facts the board settles at load time, each read on a path of its own — the
// nametable mapping, the register-write bus conflict, the PPU's reads and the PPU's writes. They
// are `bool` and not bits of one byte on purpose: each test must stay the single `cmpb` the
// measurements of MF-3b and of this lot are written against.
#[allow(clippy::struct_excessive_bools)]
pub struct Cartridge {
    memory: CartridgeMemory,
    board: Board,
    /// [`Mapper::rom_windows`], kept for [`Cartridge::read_rom_window`].
    rom_windows: [usize; 4],
    /// [`Mapper::nametable_page`] for the four logical tables, kept for
    /// [`Cartridge::nametable_page`].
    ///
    /// Cached for the same reason as `rom_windows`, and it matters more: the PPU asks for a
    /// page **once per nametable fetch**, and answering it through the `Board` enum put a
    /// 26-arm dispatch on that path. Two boards give a page of their own — the MMC5 from
    /// `$5105` and the UNROM 512 from its four-screen wiring — and a second non-default arm was
    /// enough to push `Ppu2C02::read_nametable_as` out of line, which cost every game a call
    /// per fetch (`read_pattern_planes` went from 106 instructions to 122). Refreshed wherever
    /// `rom_windows` is, by [`Cartridge::refresh_board_views`].
    nametable_pages: [i32; 4],
    /// Whether a register write can move [`Cartridge::nametable_pages`], which only the MMC5
    /// can ($5105). Derived from the ROM at load; see [`Cartridge::refresh_rom_windows`].
    nametable_pages_vary: bool,
    /// Whether the board lets the ROM drive the data bus during a register write, so that the
    /// value latched is the AND of the written value and the ROM byte at that address
    /// (MR-BUS-1; see [`registry::has_bus_conflicts`]). Derived from the ROM at load, so it is
    /// not part of the savestate, and `false` for 27 of the 32 mapper numbers and for every
    /// submapper-0 dump of three of the other five.
    bus_conflicts: bool,
    /// Whether the board has anything to do when the PPU puts an address on its bus, which only
    /// an A12 counter needs: the MMC3 and its clones (mappers 4 and 118, and the MC-ACC) and the
    /// RAMBO-1 (mapper 64). Three of the twenty-nine [`Board`] variants, and the other
    /// twenty-six inherit the empty [`Mapper::on_ppu_address_bus`].
    ///
    /// Derived from the mapper number at load, so it is not part of the savestate, and read by
    /// [`Cartridge::on_ppu_address_bus`] **before** the `Board` dispatch. That order is the whole
    /// point. The PPU calls that hook on every fetch, and for a board with nothing to do the
    /// answer used to be a bitset test the compiler folded into the dispatch prologue —
    /// `movl $66584559, %edx; btl %eax, %edx; jae`, no indirect branch. MF-3's MC-ACC needed an
    /// arm of its own there, which gave the dispatch a third distinct body; LLVM replaced the
    /// bitset with a **jump table**, and every board that does nothing began paying one more
    /// instruction and an indirect jump per fetch (R0's callgrind:
    /// `Board::on_ppu_address_bus` +4.84 M on Mega Man 2). MF-3 step 8 measured the MMC3's own
    /// path, which the jump table did shorten, and so reported the change as a win.
    ///
    /// **A board that overrides [`Mapper::on_ppu_address_bus`] must have its mapper numbers
    /// added here**, or its counter will never be clocked. `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`
    /// in `mapper_tests.rs` fails if one is missing.
    watches_ppu_reads: bool,
    /// The same, for the two cold sites where the PPU's address lines move with **/RD high**
    /// ([`Cartridge::on_ppu_address_bus_without_read`]): a `$2007` write, and a `$2006` write
    /// that loads `v`. The three A12 counters, and **not** the MMC5.
    ///
    /// A12 is an address line: it is driven on a write, and a `$2006` write alone is enough to
    /// clock an MMC3 counter by hand, which is how `mmc3_test` drives it (REQ-CORE-114). So an
    /// A12 counter must be in both lists. The MMC5's in-frame detection must see **reads
    /// only**: nesdev ("MMC5") clears the flag when "3 CPU cycles pass without a PPU read
    /// having occurred (PPU **/RD** has not been low during the last 3 M2 rises)", and /RD is
    /// not low on either of those two. A game that uploads to `$2007` through vblank with the
    /// NMI disabled would otherwise hold the flag set for the whole upload. Mesen draws the
    /// same line, with `MapperReadVram` on one side and `NotifyVramAddressChange` on the other,
    /// and this board overriding only the first.
    ///
    /// The two latches share one `Board` dispatch, which is the point: a fourth argument on
    /// [`Mapper::on_ppu_address_bus`] would cost the three A12 boards one `mov` on every fetch,
    /// and a second `Board` method would give the enum a new distinct arm — the shape MF-3b
    /// rule 4 records turning a bitset into a jump table. A second `bool` read on the cold path
    /// costs nothing on the fetch path.
    watches_ppu_bus_without_read: bool,
    /// The iNES mapper number, latched at load. See [`Cartridge::mapper_number`] for why this is
    /// a field and not the `Board` dispatch.
    mapper_number: i32,
    /// Whether the board has anything to do on a CPU cycle, which only a counter or a sound chip
    /// needs: the MMC5's PPU-idle counter, the FCG and SS88006 IRQ counters, Konami's VRC IRQ,
    /// the RAMBO-1's cycle mode, and the FME-7 and VRC6 with their expansion audio. Twelve of
    /// the thirty-eight supported mapper numbers; the other twenty-six inherit the empty
    /// [`Mapper::on_cpu_cycle`].
    ///
    /// **This is the `on_ppu_address_bus` story of MF-3b step 2, on the other clock.**
    /// [`Cartridge::step_cpu_cycle`] runs once per CPU cycle and used to dispatch
    /// unconditionally, so a board with nothing to do paid a `callq`, a twelve-instruction niche
    /// prologue, a thirty-entry jump table and a three-instruction `retq` arm — measured by
    /// callgrind at **eleven instructions per CPU cycle**, 98.3 M over 300 frames of any game on
    /// the set, whatever its board. Testing a `bool` first costs two.
    ///
    /// Lot MF-5a is what forced it: the VRC6's expansion audio gave the dispatch an arm with
    /// real work in it, and on Linux that cost `Board::on_cpu_cycle` +17.9 M instructions — **+2
    /// per CPU cycle, on all thirteen ROMs of the set, every one of them on a board that has no
    /// expansion audio at all.** `#[inline(never)]` on the audio's own body fixes the Windows
    /// build's prologue and does nothing for the Linux one, because the two ABIs allocate
    /// registers differently; the latch is ABI-independent, which is the point.
    ///
    /// Derived from the mapper number at load, so it is not part of the savestate.
    ///
    /// **A board that overrides [`Mapper::on_cpu_cycle`] must have its mapper numbers added
    /// here**, or its counter will never be clocked and its sound will never play.
    /// `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` fails if one is missing.
    clocks_on_cpu_cycle: bool,
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

        let sizes = registry::board_memory_sizes(mapper, rom.header());
        let bus_conflicts = registry::has_bus_conflicts(mapper, rom.header());
        let memory = CartridgeMemory::new(rom, sizes);
        let Some(board) = registry::create_board(mapper, &memory) else {
            return Err(RomLoadError::new(
                RomLoadFailure::UnsupportedMapper,
                format!("Unsupported mapper {mapper}."),
            ));
        };

        let mut cartridge = Self {
            memory,
            board,
            rom_windows: [mappers::NO_ROM_WINDOW; 4],
            nametable_pages: [Self::FOLLOW_MIRRORING; 4],
            // The MMC5 ($5105), the TxSROM of mapper 118 (bit 7 of its CHR registers), the VRC6
            // since MF-5a, whose `$B003` and CHR registers between them pick a CIRAM page per
            // table (nesdev, "VRC6", the `$B003` tables), and — since MF-6a step 1 — the
            // NAMCOT-3425 of mapper 95, which wires CHR A15 to CIRAM A10 so that bit 5 of its
            // two 2 KiB CHR registers is a nametable select (nesdev, "INES Mapper 095").
            //
            // **The other four numbers of the Namco 108 family are deliberately absent**: 76,
            // 88, 154 and 206 have hardwired mirroring, and 154's one-screen bit goes through
            // `memory.set_mirroring` like every other mirroring register rather than through a
            // page override.
            // MF-6b step 5 adds Tengen's 800037 (158), the RAMBO-1 with CHR A17 wired to
            // CIRAM A10, which is the TxSROM's arrangement on that chip (nesdev, "INES Mapper
            // 158") -- §4.5's cheap level, a page override and not a `read_nametable`.
            // MF-6c step 2 adds mapper 207, the Taito X1-005 wiring whose page says it "modifies
            // Taito's X1-005 (80) boards in the exact same way that TLSROM (118) differs from
            // MMC3 (4)" -- CHR A17 to CIRAM A10, so bit 7 of each 2 KiB CHR register is a
            // nametable select (nesdev, "INES Mapper 207"). §4.5's cheap level again: a page
            // override and not a `read_nametable`. **Mapper 80 is deliberately absent**, and so
            // are 82 and 552: those three put their mirroring through `memory.set_mirroring`.
            nametable_pages_vary: matches!(mapper, 5 | 19 | 24 | 26 | 95 | 118 | 158 | 207),
            bus_conflicts,
            // The MMC3 board (4, 118, and the MC-ACC submapper of 4), the RAMBO-1 (64) and —
            // since MF-4c applied MR-5-4 — the MMC5 (5), whose in-frame detection listens for
            // three identical nametable reads. See the two fields. A new board with an A12
            // counter belongs in both lists; one that waits on /RD, in the first only.
            // MF-6b step 4 adds the Taito TC0690 (48), whose counter is the MMC3's and is
            // clocked through the same A12 filter (nesdev, "INES Mapper 048"), so it is in both.
            // MF-6b step 5 adds Tengen's 800037 (158), which is a RAMBO-1 and so watches A12
            // exactly as mapper 64 does.
            // MF-6a3 step 1 adds the TQROM (119) and the two MMC3 multicarts (47, 37): all
            // three *are* the MMC3's board, so all three are in both lists. §4.3's warning,
            // and `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` is the test
            // that catches it -- it failed with its own expected list until both were extended.
            // The three numbers are **not** behind their Cargo features here: a number whose
            // board is off is not in `SUPPORTED_MAPPERS`, so no cartridge of it is ever
            // created and the arm is unreachable either way. One `matches!` is also one
            // `matches!` whatever the features, which is what keeps this load-time decision
            // out of the diff when a feature is flipped.
            watches_ppu_reads: matches!(mapper, 4 | 5 | 37 | 47 | 48 | 64 | 116 | 118 | 119 | 158),
            watches_ppu_bus_without_read: matches!(
                mapper,
                4 | 37 | 47 | 48 | 64 | 116 | 118 | 119 | 158
            ),
            mapper_number: mapper,
            // The MMC5 (5), the Bandai FCG (16), the Jaleco SS88006 (18), the Namco 163
            // (19), Konami's VRC2/VRC4 (21, 22, 23, 25, 27), VRC6 (24, 26) and VRC7 (85),
            // the RAMBO-1 (64) and the Sunsoft FME-7 (69). See the field.
            //
            // **Mapper 210 is deliberately absent**: the Namcot 175 and 340 are the Namco
            // 163 with the IRQ counter and the sound cut out, so their board moves on no CPU
            // cycle at all and `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched`
            // proves it rather than taking its word.
            // MF-6b step 1 adds the Konami VRC3 (73), whose 16-bit counter is clocked by M2 and
            // has no scanline mode at all (nesdev, "VRC3").
            // MF-6b step 2 adds the Irem H3001 (65) and the Sunsoft-3 (67), the two plain
            // 16-bit CPU-cycle down counters (nesdev, "INES Mapper 065" and "INES Mapper 067").
            // MF-6b step 3 adds the NES-EVENT (105), whose "30-bit up counter clocked by M2"
            // is the widest of them (nesdev, "NES-EVENT").
            // MF-6b step 4 adds the Taito TC0690 (48): its counter is the PPU's, but the
            // four-cycle delay on its assertion is counted down here (nesdev, "INES Mapper 048").
            // MF-6c step 1 adds mapper 153, which is the Bandai FCG's own board and so the Bandai
            // FCG's own counter: its IRQ registers "function the same way as on INES Mapper 016,
            // submapper 5" (nesdev, "INES Mapper 153").
            // MF-6c step 3 adds 157 and 159 for the same reason, in the same words: "All
            // registers function the same way as INES Mapper 016, submapper 5, except that the
            // serial EEPROM is not a 24C02 but Xicor's X24C01" (nesdev, "INES Mapper 159").
            clocks_on_cpu_cycle: matches!(
                mapper,
                5 | 16
                    | 18
                    | 19
                    | 21
                    | 22
                    | 23
                    | 24
                    | 25
                    | 26
                    | 27
                    | 48
                    | 64
                    | 65
                    | 67
                    | 69
                    | 73
                    | 85
                    | 105
                    | 153
                    | 157
                    | 159
            ),
        };
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

    /// The ROM image, to write a hack pack's hooks into (see [`crate::hack_pack`]).
    pub(crate) fn rom_mut(&mut self) -> &mut RomImage {
        &mut self.memory.rom
    }

    /// iNES mapper number of this cartridge.
    ///
    /// A latched field, not the `Board` dispatch, and that is a measured decision (MF-3b rule 5).
    /// `Ppu2C02::capture_sprites` asks this **once per sprite batch** — mappers 9 and 10 sort
    /// their sprites by x (REQ-CORE-044) and nothing else does — and `Board::mapper_number` is
    /// the one dispatch whose arms return a *per-variant constant*, so LLVM compiles it as a
    /// bitset over the boards that answer with a literal plus a field read at the offset of
    /// `mapper_number` for the boards that carry one (`UxRom`, `Mmc2`, `Mmc3`, `Vrc2Vrc4`,
    /// `JalecoJ87`…). Two distinct field offsets fit that shape; MF-5a's `Vrc6` was a **third**,
    /// which turned the bitset into a jump table and cost `Ppu2C02::run_dots` one instruction
    /// per sprite emitted — measured, +37 990 instructions over 300 frames of Zelda II and
    /// exactly **0** on `oam_stress` and `ppu_vbl_nmi`, the two ROMs of the set that emit no
    /// sprite at all.
    ///
    /// Latching it is below the baseline rather than level with it: the question is answered by
    /// one load for every board, where the baseline paid four instructions and a branch.
    #[must_use]
    pub const fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    /// Current nametable layout (SPEC-02 §2, "current mirroring").
    #[must_use]
    pub const fn mirroring(&self) -> NametableMirroring {
        self.memory.mirroring
    }

    /// Whether the cartridge is asserting the CPU IRQ line.
    ///
    /// `#[inline(always)]` for the reason `<NesBus as CpuBus>::irq_asserted` carries: this is the
    /// middle link of the per-CPU-cycle /IRQ sample, and pinning only the outer one moved the
    /// boundary here instead (6.6 M instructions on `m1`, 12.9 M on `mmc5`, MF-5b step 1).
    #[must_use]
    #[allow(clippy::inline_always)] // Measured: MF-5b step 1.
    #[inline(always)]
    pub fn irq_asserted(&self) -> bool {
        self.board.irq_asserted()
    }

    /// `M` of REQ-CORE-073: the board's expansion audio in the mixer's integer units
    /// (REQ-CORE-075), or `None` for a board without any. Only the MMC5 answers.
    #[must_use]
    pub fn expansion_audio_output(&self) -> Option<i32> {
        self.board.expansion_audio_output()
    }

    /// Whether the board has expansion audio at all, asked once when the cartridge is attached
    /// so that the bus's cycle never pays for the question (REQ-CORE-075).
    #[must_use]
    pub fn has_expansion_audio(&self) -> bool {
        self.board.expansion_audio_output().is_some()
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
    ///
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
    ///
    /// On a board whose battery is wired to only one of its RAM chips this is a sub-slice,
    /// not the whole PRG-RAM (MR-1-5; see [`registry::battery_offset`]).
    #[must_use]
    pub fn battery_ram(&self) -> &[u8] {
        if self.memory.rom.header().has_battery {
            &self.memory.prg_ram[self.memory.battery_offset..]
        } else {
            &[]
        }
    }

    /// Battery-backed PRG-RAM, mutably; empty without a battery.
    pub fn battery_ram_mut(&mut self) -> &mut [u8] {
        if self.memory.rom.header().has_battery {
            &mut self.memory.prg_ram[self.memory.battery_offset..]
        } else {
            &mut []
        }
    }

    /// Reads a byte the cartridge drives on the CPU bus, in $4020-$FFFF; `None` when it does
    /// not drive the bus there and the caller keeps the open-bus value (REQ-CORE-081).
    /// A CPU read of $8000-$FFFF that is a plain PRG-ROM read, without asking the board; `None`
    /// when the board has to decode it ([`Cartridge::try_cpu_read`]).
    #[inline]
    #[must_use]
    pub fn read_rom_window(&self, address: u16) -> Option<u8> {
        let start = self.rom_windows[usize::from(address >> 13) & 3];
        if start == mappers::NO_ROM_WINDOW {
            return None;
        }
        let value = self.memory.rom.prg_rom()[start + (usize::from(address) & 0x1FFF)];
        debug_assert_eq!(Some(value), self.board.try_read_prg(&self.memory, address));
        Some(value)
    }

    /// Re-reads the PRG-ROM windows, and the nametable pages too on the one board whose
    /// mapping a register can move.
    ///
    /// The gate matters: this runs after **every** CPU write to the cartridge, and asking the
    /// `Board` enum for four pages costs about 112 instructions. Two boards can move them at
    /// run time — the MMC5 through `$5105` and the `TxSROM` of mapper 118 through bit 7 of its CHR
    /// registers; the UNROM 512's come from its wiring and are settled at load, so the pages it
    /// answers are computed once by `refresh_nametable_pages` in `reset` and in `load_state` and
    /// then left alone.
    fn refresh_rom_windows(&mut self) {
        self.rom_windows = self.board.rom_windows(&self.memory);
        if self.nametable_pages_vary {
            self.refresh_nametable_pages();
        }
    }

    fn refresh_nametable_pages(&mut self) {
        self.nametable_pages = std::array::from_fn(|table| {
            self.board.nametable_page(i32::try_from(table).unwrap_or(0))
        });
    }

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
    ///
    /// **`#[inline(always)]` since MF-6b step 5, and it has to stay.** This function and
    /// `NesBus::write_internal` are one body as far as every measurement of this project is
    /// concerned, and which of the two holds it is LLVM's choice. Step 5's second `Rambo1`
    /// monomorphisation tipped that choice: `write_internal` went from **3 577 instructions to
    /// 1 102** and this function from **0 to 2 448** as the body moved out, and the call that
    /// appeared between them cost `m4` **+5 089 659** instructions over 300 frames against
    /// −3 537 021 in its caller, `m1` +3 776 318 against −2 658 017, and `m0` +3 543 400 — about
    /// 1.2 M net on three ROMs whose boards the step does not touch.
    ///
    /// Pinning it is the same fix, and for the same reason, as the four `#[inline(always)]`
    /// attributes MF-5b and MF-6a left on `Board::irq_asserted`, `Board::expansion_audio_output`,
    /// `Board::try_read_prg` and `Board::cpu_read_override`: a chain the cost model keeps
    /// re-deciding as boards are added is pinned rather than shortened
    /// (PERFORMANCE-HISTORY §5.10).
    #[allow(clippy::inline_always)] // Measured: MF-6b step 5, about 1.2 M on three ROMs.
    #[inline(always)]
    pub fn cpu_write(&mut self, address: u16, value: u8) {
        if self
            .board
            .intercept_cpu_write(&mut self.memory, address, value)
        {
            self.refresh_rom_windows();
            return;
        }

        if (0x6000..=0x7FFF).contains(&address) && !self.memory.prg_ram.is_empty() {
            if self.prg_ram_writable() {
                let offset = self.prg_ram_offset(address);
                self.memory.prg_ram[offset] = value;
            }

            return;
        }

        // MR-BUS-1, MR-66-1, MR-34-2: on a board with no logic to disable the ROM's output
        // enable during a write, the ROM goes on driving the bus and the register latches "the
        // bitwise AND of the written value and the value in ROM" (nesdev, "NES 2.0
        // submappers"). Only $8000-$FFFF is concerned, because that is where the PRG-ROM
        // drives; none of the seven boards has a register below it.
        //
        // Every other game pays **one test of a bool** that is false at load and stays in
        // cache beside `rom_windows`. No `Mapper` method is consulted, the `Board` dispatch is
        // untouched, and nothing here is on a per-cycle, per-PPU-fetch or CPU-read path: a CPU
        // write to the cartridge is the rarest bus event there is.
        let value = if self.bus_conflicts && address >= 0x8000 {
            value & self.read_rom_window(address).unwrap_or(0xFF)
        } else {
            value
        };

        self.board.write_register(&mut self.memory, address, value);
        self.refresh_rom_windows();
    }

    /// Reads CHR at `address` ($0000-$1FFF) as the PPU would, side effects included
    /// (latches, A12 counting).
    pub fn ppu_read(&mut self, address: u16, fetch: ChrFetch) -> u8 {
        self.board.ppu_read(&mut self.memory, address, fetch)
    }

    /// The CHR the rendering fetches read when they have no side effect and nothing watches
    /// the PPU address bus (see [`Mapper::chr_view`]); `None` for the other boards.
    #[must_use]
    pub fn chr_view(&self) -> Option<ChrView<'_>> {
        self.board.chr_view(&self.memory)
    }

    /// The CHR [`Self::ppu_peek`] reads with the banks in place now, as a view (see
    /// [`Mapper::chr_peek_view`]); `None` for the boards whose peek reads another source.
    #[must_use]
    pub fn chr_peek_view(&self) -> Option<ChrView<'_>> {
        self.board.chr_peek_view(&self.memory)
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

    /// Signals that the PPU **read** `address` (14 bits) off its bus, during real rendering or a
    /// $2006/$2007 access. Boards that count A12 rises (MMC3 and compatibles) hook their
    /// scanline counter here, and the MMC5 its in-frame detection. Never called for a peek
    /// (REQ-CORE-033).
    ///
    /// Every fetch of a rendered line comes through here, through
    /// [`Ppu2C02::read_vram_inner`](crate::ppu::Ppu2C02): the nametable byte, the attribute
    /// byte, the two background planes, the two discarded nametable fetches of dots 338 and
    /// 340, and every sprite pattern plane — 57 600-odd calls a frame, measured.
    ///
    /// `ppu_clock` is the PPU dot count since power-on, which the A12 filter of MMC3-style
    /// boards needs to tell a real rise from the short dips of the fetch pipeline, and which
    /// the MMC5 does not look at.
    ///
    /// The [`Cartridge::watches_ppu_reads`] test comes first so that the twenty-five boards
    /// with nothing to do here never reach the `Board` dispatch — one load and one
    /// not-taken branch per fetch, instead of the dispatch's niche prologue and indirect jump.
    pub fn on_ppu_address_bus(&mut self, address: u16, ppu_clock: i64) {
        if !self.watches_ppu_reads {
            return;
        }

        self.board
            .on_ppu_address_bus(&mut self.memory, address, ppu_clock);
    }

    /// Signals that the PPU put `address` (14 bits) on its bus with **/RD high**: a `$2007`
    /// write, or a `$2006` write loading `v`.
    ///
    /// The same `Board` hook as [`Cartridge::on_ppu_address_bus`], behind its own latch: A12 is
    /// an address line and is driven by both, so the three A12 counters need this, and a board
    /// whose rule is about /RD must not see it. See
    /// [`Cartridge::watches_ppu_bus_without_read`].
    pub fn on_ppu_address_bus_without_read(&mut self, address: u16, ppu_clock: i64) {
        if !self.watches_ppu_bus_without_read {
            return;
        }

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
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// Reads a nametable byte the cartridge serves itself (MMC5 `ExRAM` and fill mode, UNROM
    /// 512's four-screen CHR-RAM).
    #[must_use]
    pub fn read_nametable(&self, address: u16) -> u8 {
        self.board.read_nametable(&self.memory, address)
    }

    /// Writes a nametable byte the cartridge serves itself.
    pub fn write_nametable(&mut self, address: u16, value: u8) {
        self.board.write_nametable(&mut self.memory, address, value);
    }

    /// Advances the cartridge by one CPU cycle.
    ///
    /// The clock itself always moves — the MMC1's shift register and three IRQ counters read it
    /// (see [`Cartridge::cpu_cycle`]) — and only the dispatch is behind the latch. See
    /// `clocks_on_cpu_cycle` for the eleven instructions a board with nothing to do used to pay
    /// here.
    pub fn step_cpu_cycle(&mut self) {
        self.memory.cpu_cycle += 1;
        if self.clocks_on_cpu_cycle {
            self.board.on_cpu_cycle(&mut self.memory);
        }
    }

    /// Returns the cartridge to its power-on state (REQ-CORE-004). PRG-RAM is cleared by the
    /// core, not here, so that a host which reloaded battery RAM can keep it
    /// (REQ-CORE-095).
    pub fn reset(&mut self) {
        self.memory.mirroring = self.memory.rom.header().mirroring;
        self.memory.cpu_cycle = 0;
        self.board.reset(&mut self.memory);
        self.refresh_nametable_pages();
        self.refresh_rom_windows();
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
        // The two memories are read as prefixes rather than exact blobs: their size is the
        // board's, and lot MF-1 shrank three boards' RAM to the size of the real cartridge
        // (see [`StateReader::read_blob_prefix_into`]), so a state written before that lot
        // carries a larger blob for them and must still load (REQ-CORE-091).
        reader.read_blob_prefix_into(&mut self.memory.prg_ram)?;
        reader.read_blob_prefix_into(&mut self.memory.chr_ram)?;
        let mirroring = reader.read_u8()?;
        self.memory.mirroring = NametableMirroring::from_u8(mirroring).ok_or_else(|| {
            InvalidState::corrupt(format!(
                "Corrupt savestate: unknown nametable layout {mirroring}."
            ))
        })?;
        self.memory.cpu_cycle = reader.read_i64()?;
        let loaded = self.board.load_state(reader);
        self.refresh_nametable_pages();
        self.refresh_rom_windows();
        loaded
    }
}

#[cfg(test)]
mod tests {
    use super::mappers::Mapper;
    use super::{registry, Cartridge};
    use crate::rom::RomImage;

    /// A minimal NES 2.0 image for `mapper` / `submapper`: 64 KiB of PRG and 16 KiB of CHR,
    /// which every supported board can be created on.
    ///
    /// Byte 8 carries the submapper in its high nibble **and mapper bits 8-11 in its low one**,
    /// which an iNES 1.0 header has nowhere to put. MF-6c step 2 added the second half: NES 2.0
    /// mapper **552** is the first number in `SUPPORTED_MAPPERS` above 255, and without it the
    /// four sweeps below built a mapper-40 image for that number and failed on an unsupported
    /// mapper rather than on what they test.
    fn image(mapper: i32, submapper: u8) -> Vec<u8> {
        const PRG_16K_BANKS: u8 = 4;
        const CHR_8K_BANKS: u8 = 2;
        let prg = usize::from(PRG_16K_BANKS) * 16 * 1024;
        let chr = usize::from(CHR_8K_BANKS) * 8 * 1024;
        let mut image = vec![0u8; 16 + prg + chr];
        image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
        image[4] = PRG_16K_BANKS;
        image[5] = CHR_8K_BANKS;
        image[6] = u8::try_from(mapper & 0x0F).unwrap_or(0) << 4;
        image[7] = u8::try_from(mapper & 0xF0).unwrap_or(0) | 0x08; // NES 2.0, for the submapper.
        image[8] = (submapper << 4) | u8::try_from((mapper >> 8) & 0x0F).unwrap_or(0);
        image
    }

    /// The addresses of one rendered line as the PPU puts them on the cartridge's bus, in order:
    /// per tile the nametable byte, the attribute byte and the two pattern planes (each driven
    /// one dot before it is latched), then the two discarded nametable fetches of dots 338 and
    /// 340 — which, with the fetch of dot 337, are the **three consecutive reads of the same
    /// nametable address** the MMC5 looks for.
    ///
    /// `clock` steps by one dot per entry, which is enough for the A12 filter of MMC3-style
    /// boards to ignore the pattern fetches and for every board to see a plausible line.
    fn one_rendered_line() -> Vec<(u16, i64)> {
        let mut line = Vec::new();
        let mut clock = 0i64;
        let mut push = |address: u16, clock: &mut i64| {
            *clock += 1;
            line.push((address, *clock));
        };
        for tile in 0u16..32 {
            push(0x2000 | tile, &mut clock);
            push(0x23C0 | (tile >> 2), &mut clock);
            for plane in [0x0000u16, 0x0000, 0x0008, 0x0008] {
                push(0x1000 | (tile << 4) | plane, &mut clock);
            }
        }

        // Dots 337, 338 and 340: the same address three times, then the attribute fetch of the
        // next line, which is the point at which the scanline is detected.
        for _ in 0..3 {
            push(0x2020, &mut clock);
        }
        push(0x23C8, &mut clock);
        line
    }

    /// Every board whose [`Mapper::on_ppu_address_bus`] does something must be latched in
    /// [`Cartridge::watches_ppu_reads`], or [`Cartridge::on_ppu_address_bus`] — which tests
    /// that flag before it dispatches — will never clock it (MF-3b step 2).
    ///
    /// The check is the invariant itself and not a list: the board is driven directly, through
    /// the `Board` dispatch, and the flag is required to be `true` wherever that moves the
    /// board's state. A future lot adding an A12 counter, or a second board that listens for
    /// nametable fetches, and forgetting the flag fails here.
    #[test]
    fn mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched() {
        let line = one_rendered_line();
        let mut moved = Vec::new();
        for &mapper in registry::SUPPORTED_MAPPERS {
            // Submapper 3 of mapper 4 is the Acclaim MC-ACC, a board of its own (MR-4-3).
            for submapper in [0u8, 3] {
                let rom = RomImage::try_load(&image(mapper, submapper), "latch.nes")
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                let mut cartridge = Cartridge::create(rom)
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                let latched = cartridge.watches_ppu_reads;
                let before = cartridge.board.clone();

                // Twenty A12 rises, spaced well past the MMC3's filter, which is what an A12
                // counter reacts to, then three whole rendered lines, which is what a board
                // detecting the start of a scanline on its own bus reacts to.
                let mut clock = 0i64;
                for _ in 0..20 {
                    for address in [0x0000u16, 0x1000] {
                        clock += 16;
                        cartridge
                            .board
                            .on_ppu_address_bus(&mut cartridge.memory, address, clock);
                    }
                }
                for _ in 0..3 {
                    let base = clock;
                    for &(address, dot) in &line {
                        clock = base + dot;
                        cartridge
                            .board
                            .on_ppu_address_bus(&mut cartridge.memory, address, clock);
                    }
                }

                if cartridge.board != before {
                    moved.push(mapper);
                    assert!(
                        latched,
                        "mapper {mapper} submapper {submapper}: the board changes state when the \
                         PPU drives its address bus, but Cartridge::watches_ppu_reads is \
                         false, so Cartridge::on_ppu_address_bus returns before dispatching and \
                         the counter is never clocked. Add the mapper number to the matches! in \
                         Cartridge::create."
                    );
                }

                // A board that listens to the fetches of a line must also keep the PPU on the
                // path that makes them: `Ppu2C02::run_visible_span` takes a whole line of dots
                // without telling the cartridge anything, and it is entered exactly when
                // `chr_view` answers `Some`.
                assert!(
                    !(latched && cartridge.chr_view().is_some()),
                    "mapper {mapper} submapper {submapper}: the board watches the PPU's reads \
                     but offers a chr_view, so Ppu2C02::run_visible_span will run dots 2 to 255 \
                     of every visible line without a single notification. Return None from \
                     Mapper::chr_view."
                );
            }
        }

        // Guards against a vacuous pass: the sweep above must actually be reaching the hook.
        // Both submappers of each board move — the MMC3 and the MC-ACC for mapper 4, the
        // MMC5's scanline detection (MF-4c, finding MR-5-4), the Taito TC0690 of mapper 48
        // (MF-6b step 4), whose counter is the MMC3's, the RAMBO-1 and Tengen's 800037 of
        // mapper 158 (MF-6b step 5), which is the same chip, and the TxSROM, whose submapper 3
        // is not a board of its own.
        //
        // MF-6a3 step 1 adds the TQROM (119) and the two MMC3 multicarts (47, 37): all three
        // *are* the MMC3's board, counter included. They are behind their Cargo features, so
        // the expected list is built rather than written — a number whose board is off is not
        // in `SUPPORTED_MAPPERS`, so the sweep never reaches it.
        let mut expected = vec![4, 4, 5, 5];
        #[cfg(feature = "board-mmc3-multicart")]
        expected.extend_from_slice(&[37, 37, 47, 47]);
        expected.extend_from_slice(&[48, 48, 64, 64]);
        #[cfg(feature = "board-116")]
        expected.extend_from_slice(&[116, 116]);
        expected.extend_from_slice(&[118, 118]);
        #[cfg(feature = "board-119")]
        expected.extend_from_slice(&[119, 119]);
        #[cfg(feature = "board-158")]
        expected.extend_from_slice(&[158, 158]);
        assert_eq!(
            moved, expected,
            "expected exactly the MMC3, the MC-ACC, the MMC5, the two MMC3 multicarts, the \
             TC0690, the RAMBO-1, the TxSROM, the TQROM and the Tengen 800037 to move on the \
             PPU address bus, and the sweep to reach all of them"
        );
    }

    /// MF-5a, and MF-3b rule 5's warning in its second instance: `Cartridge::step_cpu_cycle`
    /// tests `clocks_on_cpu_cycle` **before** the `Board` dispatch, so a board whose mapper
    /// number nobody added to that list would have its counter silently never clocked and its
    /// expansion audio silently never played — a correctness bug, not a slow path.
    ///
    /// It checks the invariant itself rather than the list, so it covers a board that does not
    /// exist yet. `Board` derives `PartialEq`, so "moved" is the whole board state.
    #[test]
    fn mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched() {
        let mut moved = Vec::new();
        for &mapper in registry::SUPPORTED_MAPPERS {
            for submapper in [0u8, 3] {
                let rom = RomImage::try_load(&image(mapper, submapper), "latch.nes")
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                let mut cartridge = Cartridge::create(rom)
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                let latched = cartridge.clocks_on_cpu_cycle;
                let before = cartridge.board.clone();

                // Enough cycles for the slowest thing on this clock to show: the Sunsoft 5B's
                // divide-by-16 needs sixteen, the VRC IRQ's prescaler 114, and the MMC5's PPU
                // idle counter three.
                for _ in 0..400 {
                    cartridge.board.on_cpu_cycle(&mut cartridge.memory);
                }

                if cartridge.board != before {
                    moved.push(mapper);
                    assert!(
                        latched,
                        "mapper {mapper} submapper {submapper}: the board changes state on a CPU                          cycle, but Cartridge::clocks_on_cpu_cycle is false, so                          Cartridge::step_cpu_cycle returns before dispatching and the board is                          never clocked. Add the mapper number to the matches! in                          Cartridge::create."
                    );
                }
            }
        }

        // Guards against a vacuous pass, and the list is **short on purpose**: only the four
        // boards whose state moves with no register written at all appear. The MMC5's PPU idle
        // counter runs from power-on; the FME-7 carries the Sunsoft 5B, whose divide-by-16
        // prescaler ticks whether or not a volume was ever set; the Namco 163's sound
        // multiplexer walks its channels every fifteen cycles from power-on, which is MF-5a
        // step 4's addition; and the VRC7's FM chip counts down to its next sample from power-on
        // whether or not a channel was ever keyed, which is MF-5b step 2's. The FCG, the
        // SS88006, the VRC IRQ, the RAMBO-1's cycle mode and the VRC6's three oscillators all
        // need an enable bit first, so they are latched here without this sweep seeing them
        // move — what covers those is `mapper_irq_tests` and the audio suites, which drive whole
        // machines through `step_cpu_cycle` and would fail outright if the latch dropped them.
        //
        // **Mapper 210 is absent and that is the point of its own guard**: the Namcot 175 and
        // 340 are the 163 with the IRQ counter and the sound cut out, so `Namco163::on_cpu_cycle`
        // returns at once for them and they are rightly not latched.
        assert_eq!(
            moved,
            [5, 5, 19, 19, 69, 69, 85, 85],
            "expected the MMC5's idle counter, the Namco 163's sound multiplexer, the FME-7's              5B prescaler and the VRC7's sample divider to move on a bare CPU cycle, and the              sweep to reach them"
        );
    }

    /// The write latch is the read latch minus the boards that wait on /RD.
    ///
    /// A12 is driven on a PPU write as much as on a read, so every A12 counter must be in both
    /// lists; a board whose rule is about /RD belongs in the read list only. The reverse cannot
    /// happen: a board that ignores the PPU's reads has no business hearing its writes.
    #[test]
    fn mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd() {
        for &mapper in registry::SUPPORTED_MAPPERS {
            for submapper in [0u8, 3] {
                let rom = RomImage::try_load(&image(mapper, submapper), "latch.nes")
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                let cartridge = Cartridge::create(rom)
                    .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
                assert!(
                    !cartridge.watches_ppu_bus_without_read || cartridge.watches_ppu_reads,
                    "mapper {mapper} submapper {submapper}: watches the PPU's writes without \
                     watching its reads"
                );
            }
        }
    }
}
