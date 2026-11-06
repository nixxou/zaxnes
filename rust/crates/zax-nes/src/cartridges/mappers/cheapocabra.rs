use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **111**, the Membler Industries GTROM / Cheapocabra board (MF-6c step 9).
///
/// nesdev ("GTROM"), one register at **$5000-$5FFF and $7000-$7FFF**, both ranges the same:
///
/// ```text
/// 7  bit  0
/// ---- ----
/// GRNC PPPP
/// |||| ||||
/// |||| ++++- Select 32 KiB PRG ROM bank
/// |||+------ Select 8 KiB CHR RAM bank
/// ||+------- Select 8 KiB nametable
/// |+-------- Red LED (0=On; 1=Off)
/// +--------- Green LED (0=On; 1=Off)
/// ```
///
/// "512K total capacity divided into sixteen 32KB pages", with **"no portion of the bank is
/// fixed"**; 32 KiB of RAM on the PPU side, split into two 8 KiB CHR banks and two 8 KiB nametable
/// banks. Seven homebrew titles, *Anguna: Scourge of the Goblin King*, *The Incident*,
/// *Project Blue* and *NEScape* among them.
///
/// # The nametables: four distinct tables, twice over, and the **fourth** `read_nametable` body
///
/// The page is categorical about what the board does and why no mirroring control exists:
///
/// > This provides unique memory in both banks for all 4 nametables. This means the mapper always
/// > uses 4-screen mirroring and can neither use nor benefit from a switch to control the
/// > nametable arrangement.
///
/// and about the region above them:
///
/// > On this board these addresses are not mirrored and instead are wired to real, independent
/// > addresses on the RAM chip.
///
/// So each of the two 8 KiB nametable banks covers the whole of PPU $2000-$3FFF: four 1 KiB
/// nametables and the 4 KiB of "bonus RAM" at $3000-$3EFF behind them. **Four screens this core's
/// own VRAM could serve; two sets of four it could not** — `ppu::VRAM_SIZE` is 4 KiB — so
/// [`Mapper::nametable_page`] answers `SERVED_BY_CARTRIDGE` and the board reads and writes the
/// bytes itself. That is the shape MR-30-6 gave the UNROM 512's four-screen mode, and it makes
/// this the **fourth** distinct body of the `Mapper::read_nametable` dispatch Q-123 once measured
/// a cap of two on.
///
/// §4.5 of `docs/review/MF-6-inventory.md` asked the lot doing this board to "re-measure rather
/// than quote" that cap, and MF-6c step 4 already did for the third body: `read_nametable_inner`
/// went **114 → 75** instructions, the dispatch leaving by a tail call, and the set total moved
/// +17 007. `docs/progress/MF-6c.md` step 9 has what the fourth costs.
///
/// # The 32 KiB, in one allocation
///
/// `registry::board_memory_sizes` gives the board **32 KiB of CHR-RAM**, which is the chip the page
/// describes, and the two halves are laid out in it: the two 8 KiB CHR banks first, the two 8 KiB
/// nametable banks behind them at [`Cheapocabra::nametable_base`]. One allocation and not two, for
/// the reason [`super::UnRom512`] takes its four-screen nametables out of the end of its own
/// CHR-RAM: the board has one RAM chip and the save state has one blob for it (REQ-CORE-090).
///
/// A consequence worth naming: `BankWindows` therefore sees **four** 8 KiB CHR banks where the
/// board has two, so a CHR bank number above 1 would address the nametable half. It cannot
/// happen — the page gives the CHR field one bit.
///
/// # The flash, and the two gaps
///
/// Like the UNROM 512 the board carries a "Microchip SST39SF040 flash chip" and the PRG-ROM is
/// what a game saves into: "Programs must execute from RAM while erasing and writing".
/// [`Cheapocabra::step_flash`] is the same state machine [`super::UnRom512`] got from R0's answer
/// to **Q-117**, on this board's chip addressing — the register's bank in the top bits, the 32 KiB
/// window below it — and with the same two scopes left out, which are the gaps MR-30-4 records for
/// that board in the same words:
///
/// * **persistence is out of scope.** The flash is changed in memory and a reload loses it
///   (Q-117, to be settled with REQ-CORE-095).
/// * **the software-ID commands ($90 and $F0) are not implemented**, exactly as on the shipped
///   UNROM 512: the sequences that matter for saving are the erase and the program, and a board
///   answering a *read* with chip status would need a body in `Board::cpu_read_override`, which
///   MF-6c step 2 measured at +0.066 % on `mmc5`.
///
/// And two of the board's own:
///
/// * **the two LEDs** (bits 7 and 6) are decoded and kept — they are in the register byte, so they
///   are in the save state for free — and drive nothing. This build has no host surface for an
///   indicator on the cartridge, the gap MF-6c step 8 records for the Oeka Kids tablet in the same
///   words.
/// * **"reading from the register effectively writes the value of open bus"**, which the page's own
///   sentence continues: "so bitwise manipulation will require keeping a copy of the mapper
///   register's value in RAM". The register is **write-only** here, and the argument is the owner's
///   rule 4, development cost out of proportion: a faithful implementation needs the *open-bus
///   value* inside `Mapper::cpu_read_override`, which no hook of this architecture carries, so it
///   would take new plumbing from `NesBus` into the cartridge read path — and the page itself tells
///   every program on this board never to depend on the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cheapocabra {
    banks: BankWindows,
    /// `GRNC PPPP`, whole: the two LED bits are in it and so are in the save state.
    register: u8,
    /// First byte of CHR-RAM the nametable half starts at, precomputed at load — 16 KiB in, for
    /// a board whose chip is the 32 KiB `board_memory_sizes` gives it.
    ///
    nametable_base: usize,
    /// How far through an SST39SF040 command sequence the chip is (Q-117, MR-30-4).
    flash_step: u8,
    /// Whether the sequence under way is the erase one rather than the program one.
    flash_erasing: bool,
}

impl Cheapocabra {
    /// The RAM chip the page gives the board: "The board contains 32KB of RAM".
    pub(crate) const CHR_RAM_SIZE: usize = 32 * 1024;

    /// The nametable half of it: two banks of 8 KiB, each covering PPU $2000-$3FFF whole.
    const NAMETABLE_SIZE: usize = 16 * 1024;

    /// The two SST39SF040 command addresses, on the **chip's** address lines (Q-117).
    const UNLOCK_5555: u32 = 0x5555;
    const UNLOCK_2AAA: u32 = 0x2AAA;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
            nametable_base: memory.chr_ram().len().saturating_sub(Self::NAMETABLE_SIZE),
            flash_step: 0,
            flash_erasing: false,
        }
    }

    fn update_banks(&mut self) {
        // "GRNC PPPP": four bits of 32 KiB PRG bank, bit 4 the 8 KiB CHR bank.
        self.banks.map_prg_32k(i32::from(self.register & 0x0F));
        self.banks
            .map_chr_8k(i32::from((self.register >> 4) & 0x01));
    }

    /// Offset in the RAM chip of PPU `address`, which is $2000-$3EFF here.
    ///
    /// Bit 5 of the register picks one of the two 8 KiB banks and the thirteen low bits of the
    /// address are the offset inside it: the four nametables fill the first 4 KiB and the bonus RAM
    /// of $3000-$3EFF follows them, which is the page's "not mirrored and instead ... wired to
    /// real, independent addresses on the RAM chip".
    const fn nametable_offset(&self, address: u16) -> usize {
        let bank = ((self.register >> 5) & 0x01) as usize;
        self.nametable_base + bank * 0x2000 + (address & 0x1FFF) as usize
    }

    /// The chip address of a CPU write in the flash window: the register's bank above the 32 KiB
    /// the window shows, which is Mesen 2's `(_prgReg << 15) | (addr & 0x7FFF)`.
    const fn chip_address(&self, address: u16) -> u32 {
        ((self.register & 0x0F) as u32) << 15 | (address & 0x7FFF) as u32
    }

    /// Advances the SST39SF040 command state machine and runs the command when its last byte
    /// lands (Q-117, MR-30-4).
    ///
    /// The chip recognises its unlock writes on **chip** addresses $5555 and $2AAA, which is R0's
    /// answer to Q-117 and what [`super::UnRom512::step_flash`]'s own note argues at length; on
    /// this board they are reachable in bank 0 at CPU $D555 and $AAAA. The sequences are the
    /// chip's and not the board's, so they are the same two:
    ///
    /// * erase a 4 KiB sector — `$5555:$AA`, `$2AAA:$55`, `$5555:$80`, `$5555:$AA`, `$2AAA:$55`,
    ///   `ADDR:$30`;
    /// * program a byte — `$5555:$AA`, `$2AAA:$55`, `$5555:$A0`, `ADDR:DATA`.
    ///
    /// `$90` and `$F0`, the software-ID pair, are not implemented; see the struct's own note.
    #[inline(never)]
    fn step_flash(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match (self.flash_step, self.chip_address(address), value) {
            (0 | 3, Self::UNLOCK_5555, 0xAA) | (1 | 4, Self::UNLOCK_2AAA, 0x55) => {
                self.flash_step += 1;
            }
            (2, Self::UNLOCK_5555, 0x80) => {
                self.flash_erasing = true;
                self.flash_step = 3;
            }
            (2, Self::UNLOCK_5555, 0xA0) => {
                self.flash_erasing = false;
                self.flash_step = 6;
            }
            (5, _, 0x30) if self.flash_erasing => {
                self.erase_sector(memory, address);
                self.flash_step = 0;
            }
            (6, _, _) => {
                self.program_byte(memory, address, value);
                self.flash_step = 0;
            }
            // Anything else aborts the sequence.
            _ => self.flash_step = 0,
        }
    }

    /// Erases the 4 KiB flash sector `address` falls in, to $FF.
    fn erase_sector(&self, memory: &mut CartridgeMemory, address: u16) {
        let Some(offset) = self.banks.prg_rom_offset(address) else {
            return;
        };

        let sector = offset & !0x0FFF;
        let rom = memory.prg_rom_mut();
        let end = (sector + 0x1000).min(rom.len());
        if sector < end {
            rom[sector..end].fill(0xFF);
        }
    }

    /// Programs one byte. Flash can only clear bits, so the byte written is ANDed with what is
    /// already there — a game that has not erased the sector first sees exactly that.
    fn program_byte(&self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let Some(offset) = self.banks.prg_rom_offset(address) else {
            return;
        };

        let rom = memory.prg_rom_mut();
        if let Some(byte) = rom.get_mut(offset) {
            *byte &= value;
        }
    }
}

impl Mapper for Cheapocabra {
    banked!();

    fn mapper_number(&self) -> i32 {
        111
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
        writer.write_u8(self.flash_step);
        writer.write_bool(self.flash_erasing);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        self.flash_step = reader.read_u8()?;
        self.flash_erasing = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        // The page gives the register no power-on value. Zero is PRG bank 0, CHR bank 0 and
        // nametable bank 0; a board with **no fixed PRG window** has to have usable vectors in
        // whichever bank it powers on in, and bank 0 is the one a dump is laid out from. Mesen 2
        // uses its configurable power-on byte here, which is the same choice when that is zero.
        self.register = 0;
        self.flash_step = 0;
        self.flash_erasing = false;
        // "The mapper always uses 4-screen mirroring": the four tables are served from the
        // cartridge whatever this says, and it says the same thing for anything that asks the
        // cartridge what its arrangement is.
        memory.set_mirroring(NametableMirroring::FourScreen);
        self.update_banks();
    }

    /// `SERVED_BY_CARTRIDGE`, for all four tables and always: both 8 KiB nametable banks hold
    /// four distinct tables, which is two sets of four, and this core's VRAM is one.
    ///
    /// A constant and not a field — the UNROM 512 precomputes its answer because the same board
    /// can be in one of four mirroring modes; this one cannot be in anything else.
    ///
    fn nametable_page(&self, _table: i32) -> i32 {
        super::super::Cartridge::SERVED_BY_CARTRIDGE
    }

    /// **The fourth distinct body of this dispatch** (§4.5, Q-123); see the struct's own note.
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        memory.read_chr_direct(self.nametable_offset(address))
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let offset = self.nametable_offset(address);
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)] = value;
        }
    }

    /// The register below $8000, and the flash chip above it.
    ///
    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1: this is a register
    /// decode and `Board::write_register`'s arms reach `NesBus::write_internal`.
    ///
    /// The board has no PRG-RAM (`registry::board_memory_sizes` gives it none), so
    /// `Cartridge::cpu_write` walks past its PRG-RAM branch and this method sees $7000-$7FFF with
    /// the address intact — the route mappers 47, 37 and 86 already take.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.step_flash(memory, address, value);
            return;
        }

        // "$5000-$5FFF and $7000-$7FFF", and nothing between or below them.
        if matches!(address & 0xF000, 0x5000 | 0x7000) {
            self.register = value;
            self.update_banks();
        }
    }
}
