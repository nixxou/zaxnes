//! The flash chip a board can carry **instead of** a mask ROM, and the only memory of this build
//! that a game saves into by writing what the loader believes is read-only (MF-UNL-9 step 1).
//!
//! Two boards here have one. nesdev ("GTROM"):
//!
//! > Both of these mappers lack PRG RAM for game saves but the configuration of both boards make
//! > it possible to erase and rewrite sections of the chip using NES software.
//!
//! and, of the chip itself, "a Microchip SST39SF040 flash chip as the PRG ROM storage device".
//! The GTROM's seven commercial homebrew titles save that way and have no other way to save, so
//! persisting it is persisting their saves — which is why `docs/review/UNL-census.md` §7.2 grades
//! one mapper number an L and names this module's design "the step to design before coding".
//!
//! # Four decisions, and the reason for each
//!
//! 1. **Nothing here is on a read path.** A programmed byte goes into `prg_rom` exactly as the
//!    board already put it, so every PRG read stays the plain window read it was: this object only
//!    records *which sectors changed*. That is what lets a board carrying a flash chip stay in
//!    `default` — see `docs/review/BOARD-FEATURES.md`.
//! 2. **The dirty-sector set and the undo data are one object.** [`FlashRom::original`] holds the
//!    **pristine** contents of every sector a command has touched, taken on the first write to it,
//!    and the sectors with an entry *are* the dirty set. One `Vec` answers both "what do I save?"
//!    and "how do I put the chip back?", and the second question is not optional:
//!    [`FlashRom::load_state`] has to load a state into a core whose flash was written
//!    *differently*, which means reverting what this core did before applying what the state did.
//! 3. **The save file is the changed sectors**, which is the census's own wording, and not an
//!    image of the chip: a game that has flashed one 4 KiB sector of a 512 KiB chip writes about
//!    4 KiB and not 512. [`FlashRom::export_save`] is that container and it is self-describing —
//!    magic, version and sector size — because unlike a savestate it is a file a host keeps
//!    across versions of this build.
//! 4. **A sector is the chip's sector**, 4 KiB on the SST39SF040, and it is the unit of the erase
//!    command, of the dirty set and of the save file at once. The datasheet's chip erase and its
//!    software-ID commands are not implemented; see [`FlashCommandState::write`].

use crate::state::{InvalidState, StateReader, StateWriter};

/// What a finished command sequence asks the chip to do, the return of
/// [`FlashCommandState::write`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlashCommand {
    /// Nothing yet: the write advanced the sequence, or aborted it.
    None,
    /// Program the byte at the address written. Flash can only clear bits.
    Program,
    /// Erase to `$FF` the whole sector the address written falls in.
    EraseSector,
}

/// The SST39SF040's command state machine: how far through a sequence the chip is, and whether
/// the sequence under way is the erase one rather than the program one.
///
/// **The board owns this and not [`FlashRom`]**, which is deliberate: on the GTROM these two
/// values are already two bytes of that board's savestate block, written by MF-6c step 9, and
/// MF-UNL-9's instruction is to add the flash data "behind a version check that keeps old states
/// loading" and otherwise *not* to change the board's save-state format. Keeping the machine where
/// its bytes already are keeps that block byte for byte what it was; what moves here is the
/// decode, which is the chip's and not the board's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FlashCommandState {
    /// How many bytes of the sequence under way have landed.
    step: u8,
    /// Whether that sequence is the erase one.
    erasing: bool,
}

impl FlashCommandState {
    /// The two command addresses, on the **chip's** address lines (Q-117). On the GTROM they are
    /// reachable in PRG bank 0 at CPU $D555 and $AAAA, which is what the page's own
    /// "Specific unlock sequences written to addresses $D555 and $AAAA" names.
    const UNLOCK_5555: u32 = 0x5555;
    const UNLOCK_2AAA: u32 = 0x2AAA;

    /// The sequence so far, for a board that keeps these two values in its own savestate block.
    pub(crate) const fn step(self) -> u8 {
        self.step
    }

    /// Whether the sequence under way is the erase one, likewise.
    pub(crate) const fn erasing(self) -> bool {
        self.erasing
    }

    /// Puts back a sequence read from a savestate block.
    pub(crate) const fn restore(&mut self, step: u8, erasing: bool) {
        self.step = step;
        self.erasing = erasing;
    }

    /// Abandons any sequence under way (power-on, reset).
    pub(crate) const fn reset(&mut self) {
        self.step = 0;
        self.erasing = false;
    }

    /// Advances the machine by the write of `value` to chip address `chip_address`, and answers
    /// what to do when the last byte of a sequence lands.
    ///
    /// The sequences are the chip's and not the board's, so they are the datasheet's two:
    ///
    /// * erase a sector — `$5555:$AA`, `$2AAA:$55`, `$5555:$80`, `$5555:$AA`, `$2AAA:$55`,
    ///   `ADDR:$30`;
    /// * program a byte — `$5555:$AA`, `$2AAA:$55`, `$5555:$A0`, `ADDR:DATA`.
    ///
    /// Any other write aborts the sequence, which is the whole of the census's "an **incomplete**
    /// sequence does not [change a ROM byte]".
    ///
    /// **`$90` and `$F0`, the software-ID pair, are not implemented**, as on the shipped UNROM 512:
    /// a board answering a *read* with chip status needs a body in `Board::cpu_read_override`, and
    /// no game of either board's roster reads the chip's ID. The chip-erase sequence
    /// (`$5555:$10`) is left out for the same reason it is on that board: nothing asks for it.
    pub(crate) fn write(&mut self, chip_address: u32, value: u8) -> FlashCommand {
        match (self.step, chip_address, value) {
            (0 | 3, Self::UNLOCK_5555, 0xAA) | (1 | 4, Self::UNLOCK_2AAA, 0x55) => {
                self.step += 1;
                FlashCommand::None
            }
            (2, Self::UNLOCK_5555, 0x80) => {
                self.erasing = true;
                self.step = 3;
                FlashCommand::None
            }
            (2, Self::UNLOCK_5555, 0xA0) => {
                self.erasing = false;
                self.step = 6;
                FlashCommand::None
            }
            (5, _, 0x30) if self.erasing => {
                self.step = 0;
                FlashCommand::EraseSector
            }
            (6, _, _) => {
                self.step = 0;
                FlashCommand::Program
            }
            // Anything else aborts the sequence.
            _ => {
                self.step = 0;
                FlashCommand::None
            }
        }
    }
}

/// A flash chip standing in for a board's PRG-ROM: the sectors a game has written, and what they
/// held before it did.
///
/// The bytes themselves stay in the ROM image, where every read already finds them; this holds
/// only the per-sector pristine copies that make the writes undoable and savable. See the module
/// note for why that is the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashRom {
    /// The chip's erase unit, 4 KiB on the SST39SF040. Never zero.
    sector_size: usize,
    /// Pristine contents of every sector a command has touched, `None` while the sector still
    /// holds what the dump did. **The sectors with an entry are the dirty set**; see the module
    /// note, decision 2.
    original: Vec<Option<Box<[u8]>>>,
}

impl FlashRom {
    /// The SST39SF040's sector, "a Microchip SST39SF040 flash chip" being the chip both boards
    /// carry: 4 KiB, the unit of its sector-erase command.
    pub(crate) const SST39SF040_SECTOR: usize = 4 * 1024;

    /// Magic that opens a flash save file, `"ZXFL"`. A file format and not a savestate block, so
    /// it carries its own magic, version and sector size: a host keeps it across versions of this
    /// build, and a file whose sector size is not the chip's is refused rather than misread.
    const SAVE_MAGIC: u32 = 0x4C46_585A;

    /// Version of the save-file layout. One layout so far (MF-UNL-9).
    const SAVE_VERSION: u32 = 1;

    /// A chip of `rom_len` bytes in sectors of `sector_size`, nothing written yet.
    pub(crate) fn new(rom_len: usize, sector_size: usize) -> Self {
        let sector_size = sector_size.max(1);
        Self {
            sector_size,
            original: vec![None; rom_len.div_ceil(sector_size)],
        }
    }

    /// The sector `offset` falls in, or `None` when it is past the end of the chip.
    fn sector_of(&self, offset: usize) -> Option<usize> {
        let sector = offset / self.sector_size;
        if sector < self.original.len() {
            Some(sector)
        } else {
            None
        }
    }

    /// The byte range of sector `sector` inside a `rom_len`-byte chip. Clamped, because the last
    /// sector of a chip whose size is not a whole number of them is short.
    fn sector_range(&self, sector: usize, rom_len: usize) -> std::ops::Range<usize> {
        let start = (sector * self.sector_size).min(rom_len);
        let end = (start + self.sector_size).min(rom_len);
        start..end
    }

    /// Takes the pristine copy of `sector` if this is the first write to it. Idempotent: the copy
    /// is what the *dump* held, so a second write must not overwrite it with the first write's
    /// result.
    fn remember(&mut self, rom: &[u8], sector: usize) {
        if self.original.get(sector).is_some_and(Option::is_none) {
            let range = self.sector_range(sector, rom.len());
            self.original[sector] = Some(rom[range].to_vec().into_boxed_slice());
        }
    }

    /// Programs one byte, the `$A0` command.
    ///
    /// Flash can only clear bits, so the byte written is `ANDed` with what is already there — a game
    /// that has not erased the sector first sees exactly that, which is the chip and not an
    /// approximation of it.
    pub(crate) fn program(&mut self, rom: &mut [u8], offset: usize, value: u8) {
        let Some(sector) = self.sector_of(offset) else {
            return;
        };

        self.remember(rom, sector);
        if let Some(byte) = rom.get_mut(offset) {
            *byte &= value;
        }
    }

    /// Erases to `$FF` the sector `offset` falls in, the `$80`/`$30` command.
    pub(crate) fn erase_sector(&mut self, rom: &mut [u8], offset: usize) {
        let Some(sector) = self.sector_of(offset) else {
            return;
        };

        self.remember(rom, sector);
        let range = self.sector_range(sector, rom.len());
        rom[range].fill(0xFF);
    }

    /// Whether any sector has been written, i.e. whether there is anything to save.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.original.iter().any(Option::is_some)
    }

    /// How many sectors have been written.
    #[must_use]
    pub fn dirty_sectors(&self) -> usize {
        self.original.iter().filter(|slot| slot.is_some()).count()
    }

    /// Puts every written sector back to what the dump held, and empties the dirty set.
    fn revert(&mut self, rom: &mut [u8]) {
        let rom_len = rom.len();
        for sector in 0..self.original.len() {
            let Some(pristine) = self.original[sector].take() else {
                continue;
            };
            let range = self.sector_range(sector, rom_len);
            let count = range.len().min(pristine.len());
            rom[range.start..range.start + count].copy_from_slice(&pristine[..count]);
        }
    }

    /// Writes the changed sectors — index and current contents — to `writer`. The payload both
    /// the savestate block and the save file carry.
    fn write_sectors(&self, rom: &[u8], writer: &mut StateWriter) {
        writer.write_usize(self.dirty_sectors());
        for (sector, slot) in self.original.iter().enumerate() {
            if slot.is_none() {
                continue;
            }
            writer.write_usize(sector);
            writer.write_blob(&rom[self.sector_range(sector, rom.len())]);
        }
    }

    /// Reads a payload written by [`Self::write_sectors`] over a chip put back to the dump's
    /// contents first, so that the result is the sectors the payload names and no others.
    ///
    /// # Errors
    /// [`InvalidState`] when the payload is truncated or names a sector the chip has not got.
    fn read_sectors(
        &mut self,
        rom: &mut [u8],
        reader: &mut StateReader,
    ) -> Result<(), InvalidState> {
        let count = reader.read_usize()?;
        // Read the whole payload before touching the chip: a truncated one must leave the flash
        // as it was rather than half applied. `read_index` is what refuses a sector the chip has
        // not got, so a corrupt file cannot address outside the ROM.
        let mut sectors = Vec::with_capacity(count.min(self.original.len()));
        for _ in 0..count {
            let sector = reader.read_index(self.original.len())?;
            let contents = reader.read_blob()?;
            sectors.push((sector, contents));
        }

        self.revert(rom);
        let rom_len = rom.len();
        for (sector, contents) in sectors {
            self.remember(rom, sector);
            let range = self.sector_range(sector, rom_len);
            let bytes = range.len().min(contents.len());
            rom[range.start..range.start + bytes].copy_from_slice(&contents[..bytes]);
        }

        Ok(())
    }

    /// The changed sectors as a file for a host to keep (REQ-CORE-095's shape for a board whose
    /// save *is* its ROM). Empty when nothing has been written, so a host can skip the file
    /// altogether.
    #[must_use]
    pub fn export_save(&self, rom: &[u8]) -> Vec<u8> {
        if !self.is_dirty() {
            return Vec::new();
        }

        let mut writer =
            StateWriter::with_capacity(16 + self.dirty_sectors() * (self.sector_size + 8));
        writer.write_u32(Self::SAVE_MAGIC);
        writer.write_u32(Self::SAVE_VERSION);
        writer.write_u32(u32::try_from(self.sector_size).unwrap_or(u32::MAX));
        self.write_sectors(rom, &mut writer);
        writer.into_vec()
    }

    /// Applies a file written by [`Self::export_save`]. Returns `false` and leaves the chip
    /// untouched when `data` is not one, is of a layout this build does not read, or was written
    /// for a different sector size.
    pub fn import_save(&mut self, rom: &mut [u8], data: &[u8]) -> bool {
        let Ok(mut reader) = StateReader::new(data, 0) else {
            return false;
        };

        let header = (reader.read_u32(), reader.read_u32(), reader.read_u32());
        let (Ok(magic), Ok(version), Ok(sector_size)) = header else {
            return false;
        };
        if magic != Self::SAVE_MAGIC
            || version != Self::SAVE_VERSION
            || usize::try_from(sector_size) != Ok(self.sector_size)
        {
            return false;
        }

        self.read_sectors(rom, &mut reader).is_ok()
    }

    /// Appends the changed sectors to a savestate block (REQ-CORE-090).
    pub(crate) fn save_state(&self, rom: &[u8], writer: &mut StateWriter) {
        self.write_sectors(rom, writer);
    }

    /// Restores them (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or names a sector the chip has not got.
    pub(crate) fn load_state(
        &mut self,
        rom: &mut [u8],
        reader: &mut StateReader,
    ) -> Result<(), InvalidState> {
        self.read_sectors(rom, reader)
    }
}
