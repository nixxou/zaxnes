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
mod flash;
pub mod mappers;
pub mod registry;

pub use a12_watcher::A12Watcher;
pub use banked::{BankWindows, ChrView};
pub use board::Board;
pub use chr_fetch::ChrFetch;
pub use flash::FlashRom;
// The command machine belongs to the board that issues the sequences, and mapper 111 is the one
// board of this build that does; a second flash board adds its feature here and to the four `cfg`s
// of [`flash`] that go with it. `FlashRom` itself stays unconditional: the field, the save file
// and the savestate section are on paths every configuration compiles.
#[cfg(feature = "board-111")]
pub(crate) use flash::{FlashCommand, FlashCommandState};

use crate::rom::{InesHeader, RomImage, RomLoadError, RomLoadFailure};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;
use mappers::Mapper;
pub use board::{AnyBoard, AxRomKind, BoardKind, CnRomKind, Mmc1Kind, Mmc3Kind, Mmc5Kind, NromKind, UxRomKind};
#[cfg(feature = "board-mmc5-exattr-split")]
pub use board::Mmc5ExtKind;
pub(crate) use board::with_board_kind;

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
    /// The flash chip standing in for the PRG-ROM on the one board of this build whose game saves
    /// by writing it (MF-UNL-9 step 1), and `None` — one pointer, last field — on every other.
    ///
    /// **Boxed, and last.** The bytes a game flashes live in `rom` where every read already finds
    /// them; this side-table only records which sectors moved, it is read when a state or a save
    /// file is written and never on a bus access, and keeping it to one null pointer is what keeps
    /// it off the layout of the fields that *are* read per access (`docs/review/BOARD-FEATURES.md`;
    /// [`flash`]'s own note, decision 1).
    flash: Option<Box<flash::FlashRom>>,
    /// The console's zero page at **`$6A` and `$FF`**, as it stood when the CPU read now being
    /// answered began, for the one board whose page says its game stages values there
    /// (MF-UNL-12 step 2, MF-FIX, NES 2.0 mapper 292, Q-172 §2).
    ///
    /// nesdev ("NES 2.0 Mapper 292"): "All CPU writes, including writes to zero page memory, are
    /// visible on the cartridge connector. The mapper keeps the data of the last CPU write to any
    /// address in a temporary latch register, so it can store that data in one of its two Extra
    /// Data registers when so requested by *reading* from $6000-$7FFF. **The game writes the
    /// desired values to zero page.**"
    ///
    /// **MF-UNL-12 built the first sentence and MF-FIX built the last one.** A latch of every CPU
    /// write is the general mechanism, and it cost a store in `NesBus::write_internal` on every
    /// write of every game -- `oamstr` +0.026 % and `mmc5exram` **+1.098 %** (MF-UNL-12 step 6),
    /// the figure that parked `board-292`. Mesen 2 and puNES both read the two zero-page addresses
    /// instead (`Mmc3Variants/DragonFighter.h:50-58`, `mapper_292.c:86-98`), which is the page's
    /// own last sentence read one step further on, and that costs two loads on this board's own
    /// window's reads and **nothing at all** anywhere else. `docs/review/Q-XCHECK.md` §CHANGE 8.
    ///
    /// What it gives up is generality: the latch is right for any game on this number, and reading
    /// `$6A`/`$FF` is right only for one that stages its values there -- which is what the page
    /// says the one known game does. **If a second mapper-292 image ever appears that stages
    /// elsewhere, the latch is the general answer and its price is already measured.**
    ///
    /// Index 0 is `$6A`, which fills Extra Data Register #0, and index 1 is `$FF`, which fills
    /// #1 -- the pairing both implementations use; see `Mmc3Dragonfighter::fill_data_register`.
    ///
    /// **Filled by `NesBus::read_wide_window`**, which already runs on exactly the reads able to
    /// reach this board's window and is already `#[cold]`; a `bool` on `NesBus` settled at `attach`
    /// keeps it to one test for every other game. Behind `board-292`, so a build without that
    /// feature has neither the field nor the fill.
    ///
    /// **Not in the savestate** (REQ-CORE-090/091), deliberately: it is a transient copy of console
    /// RAM that the savestate carries in full anyway, nothing is derived from it until a read of
    /// `$6000-$7FFF` asks, and the two Extra Data registers it feeds *are* saved by the board. So
    /// no board's block grows by a byte in any configuration and golden-core is unmoved -- the same
    /// argument `flash` above makes for itself one field up, from the other direction.
    #[cfg(feature = "board-292")]
    zero_page: [u8; 2],
    /// The byte the **cartridge-edge data bus** was holding when the CPU read now being answered
    /// began, for the one board whose page leaves some of its bits undriven (Q-145, MF-FIX).
    ///
    /// nesdev ("INES Mapper 143") gives the TCA-01's protection read as `[xxii iiii]`, with `ii
    /// iiii` the inversion of the address's low six bits and `xx` annotated **open bus**. Nothing
    /// drives bits 7-6, so what a game reads there is whatever last travelled the bus — and this
    /// is that byte. [`Mapper::try_read_prg`] has no other way to it: it is handed
    /// `CartridgeMemory` and an address and nothing else.
    ///
    /// **Filled by `NesBus::read_wide_window`**, which is the function that already runs on
    /// exactly the reads able to reach such a window and is already `#[cold]`; a `bool` on
    /// `NesBus` settled at `attach` keeps it to one test for every other game. Behind
    /// `board-143-open-bus`, so a build without that feature has neither the field nor the fill
    /// and the board answers zero in those two bits, as it did before MF-FIX.
    ///
    /// **Not in the savestate** (REQ-CORE-090/091), for [`CartridgeMemory::zero_page`]'s reason
    /// above: it is a transient bus byte that the next such read overwrites, nothing is derived
    /// from it between two reads, and no board register holds it. So no board's block grows by a
    /// byte in any configuration and golden-core is unmoved.
    #[cfg(feature = "board-143-open-bus")]
    cpu_edge_bus: u8,
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
        // The one board whose PRG-ROM is a flash chip its game writes gets the side-table that
        // records what it wrote; every other board gets `None` (MF-UNL-9 step 1).
        let flash = sizes
            .flash_sector
            .map(|sector| Box::new(flash::FlashRom::new(rom.prg_rom().len(), sector)));
        let mut memory = Self {
            rom,
            prg_ram: vec![0; prg_ram_size],
            chr_ram: vec![0; chr_ram_size],
            mirroring,
            cpu_cycle: 0,
            battery_offset,
            flash,
            #[cfg(feature = "board-292")]
            zero_page: [0; 2],
            #[cfg(feature = "board-143-open-bus")]
            cpu_edge_bus: 0,
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

    /// The console's zero page at `$6A` and `$FF`, in that order, as it stood when the CPU read
    /// now being answered began; see [`CartridgeMemory::zero_page`].
    #[cfg(feature = "board-292")]
    #[must_use]
    pub const fn zero_page(&self) -> [u8; 2] {
        self.zero_page
    }

    /// Records those two bytes. Called by `NesBus::read_wide_window` before the cartridge is asked,
    /// and only for a cartridge whose board reads them.
    #[cfg(feature = "board-292")]
    pub fn set_zero_page(&mut self, bytes: [u8; 2]) {
        self.zero_page = bytes;
    }

    /// The byte the cartridge-edge data bus held when the CPU read now being answered began; see
    /// [`CartridgeMemory::cpu_edge_bus`].
    #[cfg(feature = "board-143-open-bus")]
    #[must_use]
    pub const fn cpu_edge_bus(&self) -> u8 {
        self.cpu_edge_bus
    }

    /// Records `value` as that byte. Called by `NesBus::read_wide_window` before the cartridge is
    /// asked, and only for a cartridge whose board reads it.
    #[cfg(feature = "board-143-open-bus")]
    pub fn set_cpu_edge_bus(&mut self, value: u8) {
        self.cpu_edge_bus = value;
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

    /// The flash chip this board's PRG-ROM is, when it is one (MF-UNL-9 step 1); `None` on every
    /// other board. A host reads it to know whether there is a save file to keep.
    #[must_use]
    pub fn flash(&self) -> Option<&FlashRom> {
        self.flash.as_deref()
    }

    /// Programs one byte of the flash, the `$A0` command: the byte is `ANDed` with what is there,
    /// because flash can only clear bits, and the sector is marked changed.
    ///
    /// A board with no flash row in [`registry::board_memory_sizes`] still gets the write, so the
    /// behaviour of a board that had one before this module existed is unchanged.
    #[cfg(feature = "board-111")]
    pub(crate) fn flash_program(&mut self, offset: usize, value: u8) {
        let rom = self.rom.prg_rom_mut();
        if let Some(flash) = &mut self.flash {
            flash.program(rom, offset, value);
        } else if let Some(byte) = rom.get_mut(offset) {
            *byte &= value;
        }
    }

    /// Erases the flash sector `offset` falls in to `$FF`, the `$80`/`$30` command, and marks it
    /// changed.
    #[cfg(feature = "board-111")]
    pub(crate) fn flash_erase_sector(&mut self, offset: usize) {
        let rom = self.rom.prg_rom_mut();
        if let Some(flash) = &mut self.flash {
            flash.erase_sector(rom, offset);
        }
    }

    /// The changed sectors as a save file, empty when the board has no flash or nothing has been
    /// written (MF-UNL-9 step 4).
    #[must_use]
    pub fn export_flash_save(&self) -> Vec<u8> {
        self.flash
            .as_ref()
            .map_or_else(Vec::new, |flash| flash.export_save(self.rom.prg_rom()))
    }

    /// Applies such a file. `false` when the board has no flash or the file is not one this build
    /// reads, in which case the chip is left as it was.
    pub fn import_flash_save(&mut self, data: &[u8]) -> bool {
        let rom = self.rom.prg_rom_mut();
        self.flash
            .as_mut()
            .is_some_and(|flash| flash.import_save(rom, data))
    }

    /// Appends the changed sectors to a savestate block; writes **nothing at all** for a board
    /// with no flash, which is what keeps every other board's block byte for byte what it was
    /// (MF-UNL-9 step 3).
    fn save_flash_state(&self, writer: &mut StateWriter) {
        if let Some(flash) = &self.flash {
            flash.save_state(self.rom.prg_rom(), writer);
        }
    }

    /// Restores them, reading nothing for a board with no flash.
    fn load_flash_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let rom = self.rom.prg_rom_mut();
        match &mut self.flash {
            Some(flash) => flash.load_state(rom, reader),
            None => Ok(()),
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
    /// What a CPU read of $6000-$7FFF answers, settled wherever [`Cartridge::rom_windows`] is
    /// and read by [`Cartridge::default_cpu_read`]: the byte offset in PRG-RAM of $6000, and the
    /// length the offset wraps at. **`prg_ram_read_wrap == 0` means the window is not RAM for a
    /// read** — the board has it disabled, or the cartridge carries none — and the read falls
    /// through to [`Mapper::try_read_prg`] as it did before.
    ///
    /// **This field is lot MF-CART, and it is the third load-time cache of a board answer on
    /// this struct for the same reason as the first two.** `rom_windows` exists because asking
    /// the `Board` enum for a window on every CPU read put a 35-arm dispatch on that path, and
    /// `nametable_pages` because asking it for a page on every nametable fetch did; this one
    /// exists because `default_cpu_read` asked it **two** questions per CPU read of a
    /// $6000-$7FFF address — `Mapper::prg_ram_enabled` and the old `Mapper::prg_ram_offset` —
    /// and the MMC5 is the one board that maps PRG-RAM into $8000-$DFFF, so
    /// [`Cartridge::read_rom_window`] cannot answer and **every** CPU read of an MMC5 game
    /// reaches that function.
    ///
    /// What those two dispatches cost was not their own arms. They made `default_cpu_read` 429
    /// instructions holding the inlined `Board::try_read_prg` jump table **twice**, and past a
    /// certain pressure LLVM hoists `leaq 240(%rdi), %rax` — `&self.board`, wanted only by that
    /// table's cold arms, among them mapper 116's boxed `IrqBoard::Somari(Box<Somari>)` — above
    /// the table, where it runs on every call: **+7 456 756 instructions over 300 frames of
    /// `mmc5exram`, +0.066 %**, one per read, against the owner's tolerance of +0.02 % per ROM.
    /// MF-6c measured that figure nine times in both directions and parked two boards on it;
    /// MF-CART step 1 measured it a tenth time at **three units** for `board-taito-x1`
    /// (+0.198 %). See `docs/progress/MF-CART.md`.
    ///
    /// **The enable is folded into the wrap on purpose**, so that the read is one test and not
    /// two, and the test is ordered *after* the range test because an MMC5 game's reads are
    /// nearly all of $8000-$FFFF and one signed comparison sends those away.
    ///
    /// Derived, so it is not in the savestate — [`Cartridge::load_state`] recomputes it, exactly
    /// as it does `rom_windows` and `nametable_pages`. The **write** path does not use it: it
    /// asks `Mapper::prg_ram_writable`, which is not `prg_ram_enabled` on four boards, and a CPU
    /// write to the cartridge is the rarest bus event there is.
    prg_ram_read_base: usize,
    /// See [`Cartridge::prg_ram_read_base`]. Zero means "not RAM for a read".
    prg_ram_read_wrap: usize,
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

/// Whether `mapper`'s board moves when the PPU puts an address on its bus during a read
/// ([`Cartridge::watches_ppu_reads`]).
///
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
// MF-6c step 8 adds mapper 96, the Oeka Kids board, to **both** lists, and it is
// the first number in either with no IRQ at all: its 4 KiB CHR bank is latched
// off the PPU address bus on the entry into $2000-$2FFF, and its page says the
// latch "occurs regardless of whether the PPU or game code initiated the address
// change, enabling software to manually select CHR pages by writing specific
// values to $2006" -- so it needs the rendering's fetches *and* the bare writes.
// See `OekaKids`'s own note, which does not settle MR-1's question for the MMC2.
// MF-UNL-2 step 2 adds 74, 192, 194 and 195, the other four members of mapper
// 119's CHR-RAM family: all four *are* the MMC3's board, counter included, so all
// four go in both lists on the reason the step-1 comment above gives for 119.
// MF-UNL-2 step 3 adds 114, 115, 121, 123, 182 and 187 for the same reason: each
// one holds an `Mmc3` whose counter is the chip's.
// MF-UNL-5 step 1 adds mapper 163, the Nanjing FC-001, to **both** lists, for
// mapper 96's reason and not an IRQ's: its 4 KiB CHR-RAM page is CHR A12 driven
// from "PPU A9 latched on last rise of PPU A13", so the board must hear the
// rendering's own fetches *and* a bare $2006 write that drives the address into
// $2000-$3FFF with no fetch behind it. The board has no IRQ at all, which makes
// it the second number in either list with none. See `ChrWindowBoard`'s own note.
// MF-UNL-5 step 2 adds mapper 162, the Waixing FS304, for 163's reason in the
// same words: the two pages give the same CHR-RAM switch in the same three lines
// and the two boards share one `DecodeBoard` variant.
///
/// **Extracted from `Cartridge::create` by MF-UNL-2 step 3**, which took that function past
/// `clippy::too_many_lines` -- six more numbers in each list. Two `const fn` of the mapper
/// number and nothing else: the decision is the same one, made at the same moment, and no board
/// of either list moves an instruction.
const fn watches_ppu_reads(mapper: i32) -> bool {
    matches!(
        mapper,
        4 | 5
            // MF-UNL-8: mappers 12 and 14 hold an `Mmc3` whose counter is the Huang-1's own, so
            // both are in both lists for mapper 116's reason -- which is the same chip's.
            | 12
            | 14
            | 35
            | 37
            | 47
            | 48
            | 64
            | 74
            // MF-UNL-8 step 3: the Cony / Yoko ASIC, whose $8318 selects "unfiltered PPU A12
            // rising edges" as its counter's clock. In both lists, and **not** behind
            // `board-cony` here for mapper 68's reason: one `matches!` is one `matches!`, and a
            // number whose board is off is never created. The pair is also in the lists whatever
            // $8318 holds, because the board keeps the previous PPU address whatever the source
            // is -- the J.Y. ASIC's own reason.
            | 83
            | 90
            // MF-UNL-8 step 4: mapper 91's submapper 0 counts "64 unfiltered rises of PPU A12",
            // and the board keeps the previous PPU address on both submappers so that the number
            // may be here whichever it is.
            | 91
            | 96
            | 114
            | 115
            | 116
            | 118
            | 119
            | 121
            | 123
            | 158
            | 162
            | 163
            // MF-UNL-10 step 2: mapper 164 latches CHR A12 from "PPU A9 … on last rise of PPU
            // A13" exactly as 162 and 163 do, and CHR A3 from PPU A0 on the same rise -- its
            // 1 bpp video mode. In both lists for those two numbers' reason, and it has no IRQ
            // either. See `Pec9588`'s own note.
            | 164
            | 182
            | 187
            | 189
            | 192
            | 194
            | 195
            | 209
            // MF-UNL-11 step 2: mapper 208's board holds an `Mmc3` whose counter the page calls
            // "identical to the respective MMC3 registers".
            | 208
            | 211
            // MF-UNL-11 step 1: the UNL-8237 and UNL-8237A hold an `Mmc3`, counter included, and
            // the scramble of `$5007` leaves every one of the chip's IRQ ports reachable.
            | 215
            // MF-UNL-11 step 3: the A9461's counter is the chip's in both of its modes -- its
            // `$C000-$E001` are "as on regular MMC3" whatever `$8002` bit 5 holds.
            | 219
            // MF-UNL-12 step 1: the UNL-SHERO holds an `Mmc3` whose `$C000-$E001` the page calls
            // "as normal MMC3", so its counter is the chip's.
            | 262
            | 263
            | 264
            // MF-UNL-6: mapper 268's board holds an `Mmc3`, counter included.
            | 268
            // MF-UNL-12 step 2: the BMW8544 holds an `Mmc3` whose "scanline IRQ work[s] as on a
            // normal MMC3".
            | 292
    )
}

/// The second of the two lists [`watches_ppu_reads`] documents: the boards that also move when
/// the PPU's address lines are driven with **/RD high**. Every A12 counter is in both; the
/// MMC5's in-frame detection is in the first only.
const fn watches_ppu_bus_without_read(mapper: i32) -> bool {
    matches!(
        mapper,
        // MF-UNL-8: mappers 12 and 14, the Huang-1's MMC3 counter; see the list above.
        4 | 12
            | 14
            | 35
            | 37
            | 47
            | 48
            | 64
            | 74
            // MF-UNL-8 step 3: the Cony / Yoko ASIC's A12 source; see the list above.
            | 83
            | 90
            // MF-UNL-8 step 4: mapper 91's A12 counter; see the list above.
            | 91
            | 96
            | 114
            | 115
            | 116
            | 118
            | 119
            | 121
            | 123
            | 158
            | 162
            | 163
            // MF-UNL-10 step 2: mapper 164's two CHR latches; see the list above.
            | 164
            | 182
            | 187
            | 189
            | 192
            | 194
            | 195
            | 209
            // MF-UNL-11 step 2: mapper 208's MMC3 counter; see the list above.
            | 208
            | 211
            // MF-UNL-11 step 1: the UNL-8237's MMC3 counter; see the list above.
            | 215
            // MF-UNL-11 step 3: the A9461's MMC3 counter; see the list above.
            | 219
            // MF-UNL-12 step 1: the UNL-SHERO's MMC3 counter; see the list above.
            | 262
            | 263
            | 264
            // MF-UNL-6: mapper 268's board holds an `Mmc3`, counter included.
            | 268
            // MF-UNL-12 step 2: the BMW8544's MMC3 counter; see the list above.
            | 292
    )
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
            // Both settled by the `refresh_rom_windows` below `cartridge.reset()` runs, as
            // `rom_windows` itself is; the zero wrap is the "not RAM" answer, so a cartridge is
            // never readable at $6000-$7FFF before the board has been asked.
            prg_ram_read_base: 0,
            prg_ram_read_wrap: 0,
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
            // MF-6c step 4 adds mapper 68, the Sunsoft-4, whose $E000 bit 4 answers
            // `SERVED_BY_CARTRIDGE` for all four tables when the nametables come from CHR-ROM
            // (nesdev, "INES Mapper 068"). It is in the latch whatever
            // `board-68-chr-nametables` says, for the reason the three A12 numbers are not behind
            // their features here: one `matches!` is one `matches!`, and a number whose board is
            // off is never created.
            // MF-6c step 9 adds mapper 111, the GTROM, which answers `SERVED_BY_CARTRIDGE`
            // for all four tables always: its two 8 KiB nametable banks hold two sets of four
            // distinct tables where this core's VRAM holds one (nesdev, "GTROM": "the mapper
            // always uses 4-screen mirroring").
            // MF-UNL-1 step 2 adds mappers 150 and 243, Sachen's eight-register ASIC, whose R7
            // mode 0 is `S0-S0-S0-S1` -- "lower right unique, or vertically-flipped L" -- three
            // tables on CIRAM page 0 and the fourth on page 1 (nesdev, "INES Mapper 150" and
            // "INES Mapper 243"). §4.5's cheap level: a page override and not a `read_nametable`
            // body, so Q-123's dispatch is untouched. The board's other three modes go through
            // `memory.set_mirroring` like every other mirroring register.
            // MF-UNL-4 step 5 adds mappers 35, 90, 209 and 211, the J.Y. Company ASIC, which
            // answers a CIRAM page per table under Extended Mirroring ($D001 bit 3, "use
            // $B000-$B003 bit 0") and `SERVED_BY_CARTRIDGE` per table under ROM nametables
            // ($D000 bit 5) -- the only board here that reaches **both** of §4.5's two levels
            // from the same four registers (nesdev, "J.Y. Company ASIC"). All four numbers are
            // in the latch whatever `board-jy-asic` says, for mapper 68's reason above: one
            // `matches!` is one `matches!`, and a number whose board is off is never created.
            // MF-UNL-7 step 1 adds mappers 137, 138, 139 and 141, the Sachen 8259, whose R7 mode 2
            // is `(0,1,1,1)` -- one table on CIRAM page 0 and the other three on page 1, which is
            // the eight-register ASIC's mode 0 read the other way round (nesdev, "Sachen 8259"
            // and "INES Mapper 137"). §4.5's cheap level again: a page override and not a
            // `read_nametable` body. All four are in the latch whatever `board-sachen8259` says,
            // for mapper 68's reason above.
            // MF-UNL-7 step 5 adds mapper 218, the Magic Floor board, which answers
            // `SERVED_BY_CARTRIDGE` for all four tables always: its 2 KiB *is* the console's
            // nametable RAM and the board maps the same 2 KiB into the pattern tables, so one
            // memory has to answer both windows (nesdev, "INES Mapper 218"). The number is in the
            // latch whatever `board-218` says, for mapper 68's reason above.
            nametable_pages_vary: matches!(
                mapper,
                5 | 19
                    | 24
                    | 26
                    | 35
                    | 68
                    | 90
                    | 95
                    | 111
                    | 118
                    | 137
                    | 138
                    | 139
                    | 141
                    | 150
                    | 158
                    | 207
                    | 209
                    | 211
                    | 218
                    | 243
            ),
            bus_conflicts,
            // The two lists are `watches_ppu_reads` and `watches_ppu_bus_without_read` below,
            // which carry the numbers and the lot that added each.
            watches_ppu_reads: watches_ppu_reads(mapper),
            watches_ppu_bus_without_read: watches_ppu_bus_without_read(mapper),
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
            // MF-UNL-4 step 3 adds 35, 90, 209 and 211, the J.Y. Company ASIC, whose IRQ
            // source 0 -- "CPU M2 rise", and the power-on setting of $C001 -- is this clock
            // (nesdev, "J.Y. Company ASIC").
            // MF-UNL-7 step 4 adds NES 2.0 mapper 298, whose board "uses a VRC4 clone in the
            // VRC4b configuration" -- the same chip, the same counter and the same clock, with one
            // documented difference at $F003 (nesdev, "NES 2.0 Mapper 298").
            // MF-UNL-8 step 3 adds 83 and 264, the Cony / Yoko ASIC, whose $8318 default value
            // "$00 selects CPU M2 cycles as the source of the IRQ counter" -- this clock, and the
            // board's power-on setting (nesdev, "INES Mapper 083").
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
                    | 35
                    | 48
                    | 64
                    | 65
                    | 67
                    | 69
                    | 73
                    | 83
                    | 85
                    | 90
                    // MF-UNL-8 step 4: mapper 91's submapper 1 counts M2, "counting down by five
                    // every fourth M2 cycle" (nesdev, "INES Mapper 091").
                    | 91
                    | 105
                    | 153
                    | 157
                    | 159
                    | 209
                    | 211
                    | 264
                    | 298
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

    /// The shared memories, mutably, for the host-facing save paths of `NesCore`: importing a
    /// flash save file is the one of them that writes a memory no board register reaches
    /// (MF-UNL-9 step 2).
    pub const fn memory_mut(&mut self) -> &mut CartridgeMemory {
        &mut self.memory
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
        self.irq_asserted_as::<AnyBoard>()
    }

    #[inline(always)]
        pub fn irq_asserted_as<K: BoardKind>(&self) -> bool {
        K::board(&self.board).irq_asserted()
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
        self.ppu_irq_horizon_as::<AnyBoard>()
    }

    #[must_use]
        pub fn ppu_irq_horizon_as<K: BoardKind>(&self) -> Option<i64> {
        K::board(&self.board).ppu_irq_horizon()
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
        fn prg_ram_enabled_as<K: BoardKind>(&self) -> bool {
        K::board(&self.board).prg_ram_enabled()
    }

    /// True when $6000-$7FFF accepts writes. Mappers may write-protect it.
        fn prg_ram_writable_as<K: BoardKind>(&self) -> bool {
        K::board(&self.board).prg_ram_writable()
    }

    /// Index in PRG-RAM of CPU address `address` in $6000-$7FFF. Boards that bank their
    /// PRG-RAM move the window (MMC1 SOROM, MMC5, FME-7, Namco 163).
    ///
    /// The **write** path's own, and since lot MF-CART the only caller outside a
    /// `debug_assert!`: a read takes the cached [`Cartridge::prg_ram_read_base`] instead. The
    /// write path cannot share that cache, because the enable folded into it is
    /// `Mapper::prg_ram_enabled` and what gates a write is `Mapper::prg_ram_writable` — the same
    /// answer on every board but four, and this is not a path worth proving an implication on.
    #[allow(clippy::inline_always)] // Measured: MF-CART step 3, variant B2.
    #[inline(always)]
        fn prg_ram_offset_as<K: BoardKind>(&self, address: u16) -> usize {
        let (base, wrap) = K::board(&self.board).prg_ram_window(&self.memory);
        (base + usize::from(address - 0x6000)) % wrap
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

    /// The cartridge's CHR-RAM, read only, addressed linearly from 0 in the order of the
    /// cartridge's memory and whatever banks the PPU currently sees; empty when the cartridge
    /// has none (AMD-062 REQ-CORE-123).
    ///
    /// This is a plain borrow of the buffer the boards write through, so reading it has no
    /// effect on the emulation or on its determinism: nothing is latched, no bank is switched
    /// and no cycle is spent. It is what the tile resolution of AMD-062 REQ-PROF-080 confronts
    /// with a "3dh" profile's manifest between two frames, for the games that build their tiles
    /// in CHR-RAM instead of carrying them in CHR-ROM.
    #[must_use]
    pub fn chr_ram(&self) -> &[u8] {
        self.memory.chr_ram()
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
    /// **One `Board` dispatch, and it has to stay one** (lot MF-CART step 4). The three answers
    /// the CPU read path needs — the four ROM windows, the $6000-$7FFF base and its wrap with
    /// the enable folded in — come from [`Mapper::cpu_read_windows`], whose default body calls
    /// the three `Mapper` methods on the board's own monomorphised `Self`. Asked as three
    /// separate `Board` methods they were three jump tables on this path, which step 4 measured
    /// at `mm2/core` **+0.005 %** — Mega Man 2 being an MMC1 that writes its registers often.
    fn refresh_rom_windows(&mut self) {
        let windows = self.board.cpu_read_windows(&self.memory);
        self.rom_windows = windows.rom;
        self.prg_ram_read_base = windows.prg_ram_base;
        self.prg_ram_read_wrap = windows.prg_ram_wrap;
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
        self.try_cpu_read_as::<AnyBoard>(address)
    }

        pub fn try_cpu_read_as<K: BoardKind>(&mut self, address: u16) -> Option<u8> {
        if let Some(handled) = K::board_mut(&mut self.board).cpu_read_override(&self.memory, address) {
            return handled;
        }

        self.default_cpu_read_as::<K>(address)
    }

    /// Side-effect-free counterpart of [`Cartridge::try_cpu_read`], used by the host memory
    /// accessor (REQ-CORE-080, REQ-CORE-081).
    #[must_use]
    pub fn try_cpu_peek(&self, address: u16) -> Option<u8> {
        self.try_cpu_peek_as::<AnyBoard>(address)
    }

    #[must_use]
        pub fn try_cpu_peek_as<K: BoardKind>(&self, address: u16) -> Option<u8> {
        if let Some(handled) = K::board(&self.board).cpu_peek_override(&self.memory, address) {
            return handled;
        }

        self.default_cpu_read_as::<K>(address)
    }

    /// The CPU read of $4020-$FFFF that no board wanted to override: PRG-RAM at $6000-$7FFF, and
    /// the board's own decode for the rest.
    ///
    /// **No `Board` dispatch answers the $6000-$7FFF half** — [`Cartridge::prg_ram_read_base`]
    /// and its wrap were settled by the last CPU write, a reset or a loaded state, the way
    /// `rom_windows` is — and that is lot MF-CART's whole change. The `debug_assert_eq!` is the
    /// idiom [`Cartridge::read_rom_window`] already uses for the same kind of cache: every test
    /// of the suite checks the cached answer against the dispatch it replaces.
    /// **`#[inline(always)]` since lot MF-CART step 4, and it is the step's whole result.**
    /// Inlined into [`Cartridge::try_cpu_read`], this function stops existing as a symbol and
    /// the MMC5's own `try_read_prg` arm — the big one, the board that maps PRG-RAM into
    /// $8000-$DFFF — goes **out of line** instead. On 300 frames of `mmc5exram` that is
    /// `default_cpu_read` **283 356 728 → 0**, `Mmc5::try_read_prg` **0 → 141 678 364** and
    /// `try_cpu_read` **300 643 690 → 429 758 682**: a net **−12.6 M, −0.111 %** on the one ROM
    /// nine measurements of MF-6c and two of MF-CART had been charged +7 456 756 a time for.
    ///
    /// **And it is what lifts the limit on new boards.** The quantum those eleven measurements
    /// moved in was one instruction per CPU read, paid in this function's copy of the inlined
    /// `Board::try_read_prg` jump table: past a certain pressure LLVM shuffled two registers and
    /// materialised `&self.board` above the table, for the table's cold arms — mapper 116's
    /// boxed `IrqBoard::Somari(Box<Somari>)` and the arms that are `#[inline(never)]`. Inlined
    /// here, there is no such copy: the MMC5 reaches its own arm through a `callq` whose callee
    /// is one board's code, and **`Mmc5::try_read_prg` and `try_cpu_read` then measure
    /// 141 678 364 and 429 758 682 whether `Board` has thirty-six variants, thirty-eight or
    /// thirty-nine** — identical to the instruction in all three configurations MF-CART step 4
    /// measured. A board added to any shared enum no longer touches the MMC5's read path at all.
    ///
    /// MF-6c step 2 measured this attribute as its shape c and had to reject it: `mmc5` came out
    /// −0.065 % but `m0` +0.022 % and `m1` +0.024 %, because `Board::prg_ram_enabled` went out
    /// of line as the body moved. **That dispatch is no longer in this function** — step 2 of
    /// this lot settled it on `Cartridge` — so the shape's one drawback is gone, and `m0` and
    /// `m1` come out −0.059 % and −0.098 % instead. The two halves of the fix only work
    /// together, and neither reaches the owner's tolerance alone.
    #[allow(clippy::inline_always)] // Measured: MF-CART step 4, -0.111 % on an MMC5.
    #[inline(always)]
        fn default_cpu_read_as<K: BoardKind>(&self, address: u16) -> Option<u8> {
        if (0x6000..=0x7FFF).contains(&address) && self.prg_ram_read_wrap != 0 {
            let offset =
                (self.prg_ram_read_base + usize::from(address - 0x6000)) % self.prg_ram_read_wrap;
            debug_assert_eq!(offset, self.prg_ram_offset_as::<K>(address));
            debug_assert!(self.prg_ram_enabled_as::<K>() && !self.memory.prg_ram.is_empty());
            return Some(self.memory.prg_ram[offset]);
        }

        debug_assert!(
            !(0x6000..=0x7FFF).contains(&address)
                || !self.prg_ram_enabled_as::<K>()
                || self.memory.prg_ram.is_empty()
        );
        K::board(&self.board).try_read_prg(&self.memory, address)
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
        self.cpu_write_as::<AnyBoard>(address, value)
    }

    #[inline(always)]
        pub fn cpu_write_as<K: BoardKind>(&mut self, address: u16, value: u8) {
        if self
            .board
            .intercept_cpu_write(&mut self.memory, address, value)
        {
            self.refresh_rom_windows();
            return;
        }

        if (0x6000..=0x7FFF).contains(&address) && !self.memory.prg_ram.is_empty() {
            if self.prg_ram_writable_as::<K>() {
                let offset = self.prg_ram_offset_as::<K>(address);
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

        if K::SWITCHES && !K::matches(&self.board) {
            // The intercept above has just armed the MMC5's full board: the write is its.
            self.board.write_register(&mut self.memory, address, value);
        } else {
            K::board_mut(&mut self.board).write_register(&mut self.memory, address, value);
        }
        self.refresh_rom_windows();
    }

    /// Reads CHR at `address` ($0000-$1FFF) as the PPU would, side effects included
    /// (latches, A12 counting).
    pub fn ppu_read(&mut self, address: u16, fetch: ChrFetch) -> u8 {
        self.ppu_read_as::<AnyBoard>(address, fetch)
    }

        pub fn ppu_read_as<K: BoardKind>(&mut self, address: u16, fetch: ChrFetch) -> u8 {
        K::board_mut(&mut self.board).ppu_read(&mut self.memory, address, fetch)
    }

    /// The CHR the rendering fetches read when they have no side effect and nothing watches
    /// the PPU address bus (see [`Mapper::chr_view`]); `None` for the other boards.
    #[must_use]
    pub fn chr_view(&self) -> Option<ChrView<'_>> {
        self.chr_view_as::<AnyBoard>()
    }

    #[must_use]
        pub fn chr_view_as<K: BoardKind>(&self) -> Option<ChrView<'_>> {
        K::board(&self.board).chr_view(&self.memory)
    }

    /// The CHR [`Self::ppu_peek`] reads with the banks in place now, as a view (see
    /// [`Mapper::chr_peek_view`]); `None` for the boards whose peek reads another source.
    #[must_use]
    pub fn chr_peek_view(&self) -> Option<ChrView<'_>> {
        self.chr_peek_view_as::<AnyBoard>()
    }

    #[must_use]
        pub fn chr_peek_view_as<K: BoardKind>(&self) -> Option<ChrView<'_>> {
        K::board(&self.board).chr_peek_view(&self.memory)
    }

    /// Reads CHR without any side effect. The tile emitter and the host accessors use this
    /// one (REQ-CORE-033).
    #[must_use]
    pub fn ppu_peek(&self, address: u16, fetch: ChrFetch) -> u8 {
        self.ppu_peek_as::<AnyBoard>(address, fetch)
    }

    #[must_use]
        pub fn ppu_peek_as<K: BoardKind>(&self, address: u16, fetch: ChrFetch) -> u8 {
        K::board(&self.board).ppu_peek(&self.memory, address, fetch)
    }

    /// Writes CHR-RAM at `address` ($0000-$1FFF).
    pub fn ppu_write(&mut self, address: u16, value: u8) {
        self.ppu_write_as::<AnyBoard>(address, value)
    }

        pub fn ppu_write_as<K: BoardKind>(&mut self, address: u16, value: u8) {
        K::board_mut(&mut self.board).ppu_write(&mut self.memory, address, value);
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
        self.chr_latch_state_as::<AnyBoard>()
    }

    #[must_use]
        pub fn chr_latch_state_as<K: BoardKind>(&self) -> i32 {
        K::board(&self.board).chr_latch_state()
    }

    /// Restores the opaque CHR latch state (see [`Cartridge::chr_latch_state`]).
    pub fn set_chr_latch_state(&mut self, state: i32) {
        self.set_chr_latch_state_as::<AnyBoard>(state)
    }

        pub fn set_chr_latch_state_as<K: BoardKind>(&mut self, state: i32) {
        K::board_mut(&mut self.board).set_chr_latch_state(state);
    }

    /// Moves the CHR latches on as the fetch of pattern `tile` from `pattern_table` ($0000
    /// or $1000) would (REQ-CORE-039).
    pub fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        self.advance_chr_latch_as::<AnyBoard>(pattern_table, tile)
    }

        pub fn advance_chr_latch_as<K: BoardKind>(&mut self, pattern_table: i32, tile: i32) {
        K::board_mut(&mut self.board).advance_chr_latch(pattern_table, tile);
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
        self.on_ppu_address_bus_as::<AnyBoard>(address, ppu_clock)
    }

        pub fn on_ppu_address_bus_as<K: BoardKind>(&mut self, address: u16, ppu_clock: i64) {
        if !self.watches_ppu_reads {
            return;
        }

        K::board_mut(&mut self.board)
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
        self.on_ppu_address_bus_without_read_as::<AnyBoard>(address, ppu_clock)
    }

        pub fn on_ppu_address_bus_without_read_as<K: BoardKind>(&mut self, address: u16, ppu_clock: i64) {
        if !self.watches_ppu_bus_without_read {
            return;
        }

        K::board_mut(&mut self.board)
            .on_ppu_address_bus(&mut self.memory, address, ppu_clock);
    }

    /// Signals a write to PPUCTRL ($2000). MMC5 needs it: the sprite size decides whether
    /// its two CHR register sets are used separately (nesdev, "MMC5").
    pub fn on_ppu_control(&mut self, control: u8) {
        self.on_ppu_control_as::<AnyBoard>(control)
    }

        pub fn on_ppu_control_as<K: BoardKind>(&mut self, control: u8) {
        K::board_mut(&mut self.board).on_ppu_control(control);
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
        self.read_nametable_as::<AnyBoard>(address)
    }

    #[must_use]
        pub fn read_nametable_as<K: BoardKind>(&self, address: u16) -> u8 {
        K::board(&self.board).read_nametable(&self.memory, address)
    }

    /// [`Cartridge::read_nametable`] with the console's VRAM, which the MMC5 needs while it
    /// answers all four tables in extended attribute mode or with the split on — see
    /// [`Mapper::read_nametable_from_vram`]. Every other board forwards to
    /// [`Cartridge::read_nametable`] without reaching a dispatch.
    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
    pub fn read_nametable_from_vram(&self, address: u16, vram: &[u8]) -> u8 {
        self.board
            .read_nametable_from_vram(&self.memory, address, vram)
    }

    /// Where a write to a nametable the cartridge answers lands in console VRAM, or `None` when
    /// the board keeps or drops the byte (see [`Mapper::nametable_write_offset`]).
    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
    pub fn nametable_write_offset(&self, address: u16) -> Option<usize> {
        self.nametable_write_offset_as::<AnyBoard>(address)
    }

    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
        pub fn nametable_write_offset_as<K: BoardKind>(&self, address: u16) -> Option<usize> {
        K::board(&self.board).nametable_write_offset(address)
    }

    /// Whether the board substitutes a background tile's nametable byte, its attribute or its
    /// pattern bank (see [`Mapper::substitutes_background_tiles`]). `false` on every board but
    /// the MMC5 with either mode armed.
    ///
    /// **Not cached on `Cartridge`, and that was measured.** A `bool` field refreshed beside
    /// [`Cartridge::nametable_pages`] — the shape [`Cartridge::watches_ppu_reads`] has — made
    /// `Cartridge::refresh_rom_windows` too large to inline, which put **+610 864 instructions
    /// on `smb3` and +101 088 on `m1`** on a function that runs after every CPU write to the
    /// cartridge, and bought nothing: the capture's own figure did not move by an instruction.
    /// The capture asks once per visible line, and once the substituting body is behind a call
    /// (`Ppu2C02::capture_background_row_substituted`) that is free. Step 4 of
    /// `docs/progress/MF-LEG-2.md` has both measurements.
    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
    pub fn substitutes_background_tiles(&self) -> bool {
        self.substitutes_background_tiles_as::<AnyBoard>()
    }

    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
        pub fn substitutes_background_tiles_as<K: BoardKind>(&self) -> bool {
        K::board(&self.board).substitutes_background_tiles()
    }

    /// Opaque per-tile state of the MMC5's two substituting modes, which the tile capture holds
    /// while it walks a background row out of display order and puts back afterwards
    /// (REQ-CORE-033). Always 0 on every other board — see
    /// [`Mapper::background_capture_state`].
    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
    pub fn background_capture_state(&self) -> u64 {
        self.background_capture_state_as::<AnyBoard>()
    }

    #[cfg(feature = "board-mmc5-exattr-split")]
    #[must_use]
        pub fn background_capture_state_as<K: BoardKind>(&self) -> u64 {
        K::board(&self.board).background_capture_state()
    }

    /// Restores what [`Cartridge::background_capture_state`] took.
    #[cfg(feature = "board-mmc5-exattr-split")]
    pub fn set_background_capture_state(&mut self, state: u64) {
        self.set_background_capture_state_as::<AnyBoard>(state)
    }

    #[cfg(feature = "board-mmc5-exattr-split")]
        pub fn set_background_capture_state_as<K: BoardKind>(&mut self, state: u64) {
        K::board_mut(&mut self.board).set_background_capture_state(state);
    }

    /// Positions that state on the tile the capture is about to read (see
    /// [`Mapper::position_captured_background_tile`]).
    #[cfg(feature = "board-mmc5-exattr-split")]
    pub fn position_captured_background_tile(&mut self, address: u16, column: i32, line: i32) {
        self.board
            .position_captured_background_tile(address, column, line);
    }

    /// Writes a nametable byte the cartridge serves itself.
    pub fn write_nametable(&mut self, address: u16, value: u8) {
        self.write_nametable_as::<AnyBoard>(address, value)
    }

        pub fn write_nametable_as<K: BoardKind>(&mut self, address: u16, value: u8) {
        K::board_mut(&mut self.board).write_nametable(&mut self.memory, address, value);
    }

    /// Advances the cartridge by one CPU cycle.
    ///
    /// The clock itself always moves — the MMC1's shift register and three IRQ counters read it
    /// (see [`Cartridge::cpu_cycle`]) — and only the dispatch is behind the latch. See
    /// `clocks_on_cpu_cycle` for the eleven instructions a board with nothing to do used to pay
    /// here.
    pub fn step_cpu_cycle(&mut self) {
        self.step_cpu_cycle_as::<AnyBoard>()
    }

        pub fn step_cpu_cycle_as<K: BoardKind>(&mut self) {
        self.memory.cpu_cycle += 1;
        if self.clocks_on_cpu_cycle {
            K::board_mut(&mut self.board).on_cpu_cycle(&mut self.memory);
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

    /// A press on the console's **RESET** button, which the cartridge connector has no pin for:
    /// the mirroring the game selected, the M2 cycle counter the IRQ boards count on and every
    /// register of the board are **kept**. See [`Mapper::soft_reset`] for the hardware, and
    /// `NesCore::soft_reset` for the rest of the console's own after-reset state.
    ///
    /// The two cache refreshes are not a behaviour: they are what keeps the bus's fast windows in
    /// step with the board, and they are needed only because a reset-detecting multicart may have
    /// moved a bank. For every other board this call does nothing observable, which is the whole
    /// point of it.
    pub fn soft_reset(&mut self) {
        self.board.soft_reset(&mut self.memory);
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
        // The flash chip's changed sectors, and **nothing at all** for the other boards: this
        // call appends zero bytes unless `board_memory_sizes` gave the board a flash sector, so
        // every existing board's block is byte for byte what it was before MF-UNL-9 and the one
        // difference in its state file is the version number in the header.
        self.memory.save_flash_state(writer);
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
        // The flash, from the version of the format that first carried it. A state written before
        // MF-UNL-9 has no such bytes and loads with the chip holding what the dump holds, which
        // is also what a fresh power-on gives (REQ-CORE-091).
        if loaded.is_ok() && crate::NesCore::state_version_has_flash(reader.format_version()) {
            self.memory.load_flash_state(reader)?;
        }
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

    /// The cartridge of `mapper` / `submapper`, or `None` when the registry refuses that pair.
    ///
    /// **Mapper 268 is the one number that refuses a submapper**, and the sweeps below have to
    /// let it: it serves submappers 0 and 1 — the COOLBOY and MINDKIDS layouts of the AA6023 —
    /// and `docs/review/UNL-census.md` §7.2 asks that the PCB codes it does not serve be refused
    /// rather than run on a register layout that is not theirs (Q-157, MF-UNL-6 step 1). Any
    /// other refusal, and any refusal for another cause, is still a failure of the sweep.
    fn latched_cartridge(mapper: i32, submapper: u8) -> Option<Cartridge> {
        let rom = RomImage::try_load(&image(mapper, submapper), "latch.nes")
            .unwrap_or_else(|e| panic!("mapper {mapper}.{submapper}: {e:?}"));
        match Cartridge::create(rom) {
            Ok(cartridge) => Some(cartridge),
            Err(error) => {
                assert_eq!(
                    (268, crate::rom::RomLoadFailure::UnsupportedMapper),
                    (mapper, error.failure),
                    "mapper {mapper}.{submapper}: {error:?}"
                );
                None
            }
        }
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
    // The expected list below *is* one row per lot that added a board to either latch, each with
    // the page or the chip it came from -- a flat table, as `registry::board_memory_sizes` is.
    // MF-UNL-10's mapper 164 is the arm that crossed the threshold.
    #[allow(clippy::too_many_lines)]
    fn mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched() {
        let line = one_rendered_line();
        let mut moved = Vec::new();
        for &mapper in registry::SUPPORTED_MAPPERS {
            // Submapper 3 of mapper 4 is the Acclaim MC-ACC, a board of its own (MR-4-3).
            for submapper in [0u8, 3] {
                let Some(mut cartridge) = latched_cartridge(mapper, submapper) else {
                    continue;
                };
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
        // MF-UNL-8 steps 1 and 2: mappers 12 and 14 hold an `Mmc3` whose counter is the Huang-1's
        // own, so both move for mapper 116's reason -- which is the same chip's. **Twice each**:
        // submapper 3 is neither mapper 12's refused Magic Card 4M (that is submapper 1) nor
        // anything on mapper 14's page, so `latched_cartridge` builds both halves of each pair.
        #[cfg(feature = "board-huang1")]
        expected.extend_from_slice(&[12, 12, 14, 14]);
        // MF-UNL-4 step 3: the J.Y. Company ASIC keeps the previous PPU address whatever its IRQ
        // source is, so that a game selecting source 1 ("PPU A12 rise, unfiltered") mid-frame
        // does not read a stale edge. That one byte is what makes all four numbers move here.
        #[cfg(feature = "board-jy-asic")]
        expected.extend_from_slice(&[35, 35]);
        #[cfg(feature = "board-mmc3-multicart")]
        expected.extend_from_slice(&[37, 37, 47, 47]);
        expected.extend_from_slice(&[48, 48, 64, 64]);
        // MF-UNL-2 step 2: the other four of the TQROM's CHR-RAM family, for 119's own reason.
        #[cfg(feature = "board-mmc3-chrram")]
        expected.extend_from_slice(&[74, 74]);
        // MF-UNL-8 step 3: the Cony / Yoko ASIC keeps the previous PPU address whatever `$8318`
        // holds, so that a game selecting the A12 source mid-frame does not read a stale edge --
        // the J.Y. ASIC's own reason, and that one byte is what makes both numbers move here.
        // **Twice each**: submapper 3 is a PCB of its own on mapper 83 (the N-C44 4-in-1) and
        // nothing on 264, and neither is the submapper 2 this board refuses, so
        // `latched_cartridge` builds both halves of both pairs.
        #[cfg(feature = "board-cony")]
        expected.extend_from_slice(&[83, 83]);
        #[cfg(feature = "board-jy-asic")]
        expected.extend_from_slice(&[90, 90]);
        // MF-UNL-8 step 4: mapper 91 keeps the previous PPU address on both submappers, so both
        // halves of the pair move here even though only submapper 0's counter reads it.
        #[cfg(feature = "board-91")]
        expected.extend_from_slice(&[91, 91]);
        // MF-6c step 8: the Oeka Kids board latches its CHR bank on the entry into
        // $2000-$2FFF, which the rendered line below reaches.
        #[cfg(feature = "board-96")]
        expected.extend_from_slice(&[96, 96]);
        // MF-UNL-2 step 3: the six scrambled boards, each of which holds an `Mmc3`.
        #[cfg(feature = "board-mmc3-scramble")]
        expected.extend_from_slice(&[114, 114, 115, 115]);
        #[cfg(feature = "board-116")]
        expected.extend_from_slice(&[116, 116]);
        expected.extend_from_slice(&[118, 118]);
        #[cfg(feature = "board-119")]
        expected.extend_from_slice(&[119, 119]);
        #[cfg(feature = "board-mmc3-scramble")]
        expected.extend_from_slice(&[121, 121, 123, 123]);
        #[cfg(feature = "board-158")]
        expected.extend_from_slice(&[158, 158]);
        // MF-UNL-5 step 1: the Nanjing FC-001 keeps the previous PPU address whatever its CHR
        // switch is set to, because the page makes the latch a property of the address lines and
        // not of the enable -- so a game turning the switch on mid-frame does not read a stale
        // edge. That one byte pair is what makes the number move here, exactly as mapper 96's
        // does above.
        #[cfg(feature = "board-162")]
        expected.extend_from_slice(&[162, 162]);
        #[cfg(feature = "board-163")]
        expected.extend_from_slice(&[163, 163]);
        // MF-UNL-10 step 2: mapper 164 latches CHR A12 and CHR A3 on the rise of PPU A13, which
        // is 163's mechanism with a second line on it. **Twice**, the board refusing no
        // submapper.
        #[cfg(feature = "board-164")]
        expected.extend_from_slice(&[164, 164]);
        #[cfg(feature = "board-mmc3-scramble")]
        expected.extend_from_slice(&[182, 182, 187, 187]);
        // MF-UNL-2 steps 4 and 5: the UNL-KOF97 and the TXC 189, both of which hold an `Mmc3`.
        #[cfg(feature = "board-189")]
        expected.extend_from_slice(&[189, 189]);
        #[cfg(feature = "board-mmc3-chrram")]
        expected.extend_from_slice(&[192, 192, 194, 194, 195, 195]);
        // MF-UNL-11 step 2: mapper 208's board holds an `Mmc3`, so it moves for 114's reason.
        // **Twice**: submapper 1 is the copyright-stripped hack and the board refuses neither.
        #[cfg(feature = "board-208")]
        expected.extend_from_slice(&[208, 208]);
        #[cfg(feature = "board-jy-asic")]
        expected.extend_from_slice(&[209, 209, 211, 211]);
        // MF-UNL-11 step 1: the UNL-8237 and UNL-8237A hold an `Mmc3`, so the number moves for
        // 114's reason. **Twice**: submapper 1 is the UNL-8237A and the board refuses neither.
        #[cfg(feature = "board-215")]
        expected.extend_from_slice(&[215, 215]);
        // MF-UNL-11 step 3: the A9461 holds an `Mmc3`, so it moves for 114's reason. **Twice**:
        // the page gives the number no submapper and the board refuses neither half of the pair.
        #[cfg(feature = "board-219")]
        expected.extend_from_slice(&[219, 219]);
        // MF-UNL-12 step 1: the UNL-SHERO holds an `Mmc3`, so it moves for 114's reason.
        // **Twice**: the page gives the number no submapper and the board refuses neither half.
        #[cfg(feature = "board-262")]
        expected.extend_from_slice(&[262, 262]);
        #[cfg(feature = "board-263")]
        expected.extend_from_slice(&[263, 263]);
        #[cfg(feature = "board-cony")]
        expected.extend_from_slice(&[264, 264]);
        // MF-UNL-6: mapper 268's board holds an `Mmc3`, so it moves for 114's reason. **Once and
        // not twice**: submapper 3 is one of the seven PCB codes this board refuses, so
        // `latched_cartridge` skips that half of the pair (Q-157).
        #[cfg(feature = "board-268")]
        expected.push(268);
        // MF-UNL-12 step 2: the BMW8544 holds an `Mmc3`, so it moves for 114's reason. **Twice**:
        // the page gives the number no submapper and the board refuses neither half of the pair.
        #[cfg(feature = "board-292")]
        expected.extend_from_slice(&[292, 292]);
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
                let Some(mut cartridge) = latched_cartridge(mapper, submapper) else {
                    continue;
                };
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
                let Some(cartridge) = latched_cartridge(mapper, submapper) else {
                    continue;
                };
                assert!(
                    !cartridge.watches_ppu_bus_without_read || cartridge.watches_ppu_reads,
                    "mapper {mapper} submapper {submapper}: watches the PPU's writes without \
                     watching its reads"
                );
            }
        }
    }
}
