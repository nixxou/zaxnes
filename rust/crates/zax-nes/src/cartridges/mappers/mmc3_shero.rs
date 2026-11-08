use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// NES 2.0 mapper **262**, the Sachen **UNL-SHERO** of *Street Heroes* and *侍魂 (Shìhún)*: an
/// MMC3 clone with **8 KiB of CHR-RAM beside its CHR-ROM**, a CHR-ROM/CHR-RAM select and four
/// high CHR bits at `$4100`, and **four-screen mirroring** (MF-UNL-12 step 1, REQ-CORE-102).
///
/// nesdev ("NES 2.0 Mapper 262"): "It uses an MMC3 clone with both CHR-ROM and 8 KiB of CHR-RAM,
/// as well 4 KiB of additional VRAM for four-screen mirroring. Its UNIF board name is
/// **UNL-SHERO**." `docs/review/UNL-census.md` §1.1 files it under Sachen and gives it the two
/// titles the jumper picks between.
///
/// # The register file, and the whole of it
///
/// ```text
/// Mask: probably $E001
///    $8000, $8001, $A000, $A001, $C000, $C001, $E000, $E001: As normal MMC3.
///
/// Mask: unknown, but probably $C100
///  $4100 (read): Read jumper setting      $4100 (write): High CHR bits and CHR-ROM/-RAM
///     Bit 7654 3210                         Bit 7654 3210
///         ---------                             ---------
///         J... ....                             .M.. 8421
///         +--------- Jumper value                |   |||+- 256 KiB CHR bank at $1000-$17FF
///                    0: Street Heroes            |   ||+-- 256 KiB CHR bank at $1800-$1FFF
///                    1: 侍魂 (Samurai Spirits)   |   |+--- 256 KiB CHR bank at $0800-$0FFF
///                                                |   +---- 256 KiB CHR bank at $0000-$07FF
///                                                +-------- Select CHR-ROM/-RAM
///                                                          0: CHR-ROM   1: CHR-RAM
/// ```
///
/// # Three of the page's own words are "probably" or "unknown", and this is which (**Q-172**)
///
/// §7.2 of the census says of this board that "several masks on the page are 'probably' or
/// 'unknown'; the implementation follows the page and the comment lists which masks are guesses".
/// There are exactly **two** such masks and they are listed here:
///
/// * **`Mask: probably $E001`** on the MMC3 half. `$E001` *is* the MMC3's own mask, so the guess
///   and the chip agree and nothing is decided by it: every `$8000-$FFFF` write is handed to the
///   [`Mmc3`] this board holds, which decodes it under exactly that mask.
/// * **`Mask: unknown, but probably $C100`** on `$4100`. Implemented as written —
///   `address & $C100 == $4100`, which is A15 clear, A14 set and A8 set — so the port answers
///   through the whole of `$4000-$5FFF` on odd pages, and inside `$6000-$7FFF` too wherever a
///   header declares no PRG-RAM. `Cartridge::cpu_write`'s own order is what settles the overlap:
///   a dump that declares PRG-RAM has that window as RAM and the register is reached at
///   `$4100-$5FFF` alone. The sibling Sachen boards of this build use `$E100`
///   ([`super::Nina03`], [`super::Sachen72008`]) and `$C101` ([`super::Sachen8Reg`]), so `$C100`
///   is the page's guess and not a family convention, and it is left exactly as the page has it.
///
/// # And two readings of the one diagram that a paraphrase gets wrong (**Q-172**)
///
/// Both come from the diagram's **column positions**, which is why this board was built from the
/// page's raw wikitext and not from rendered text:
///
/// * **the select is bit 6.** `.M.. 8421` sits under `7654 3210`, so `M` is at the position of
///   bit 6 and bit 7 is `.`. The `J` of the *read* diagram is the one at bit 7, and the two
///   diagrams are not symmetrical.
/// * **the four A18 lines are not in address order.** Each annotation's `+` names its bit, and
///   they read bit 3 → `$0000-$07FF`, bit 2 → `$0800-$0FFF`, bit 1 → **`$1800-$1FFF`**, bit 0 →
///   **`$1000-$17FF`**. The `8421` label is the four bits' weights; the bottom two 2 KiB regions
///   are crossed over against them. See [`Mmc3Shero::CHR_A18_LINES`].
///
/// # The CHR-RAM chip's own three address lines
///
/// The page gives the chip a size and no banking at all, and what addresses it follows from the
/// board: an 8 Ki × 8 SRAM has A0-A12, PPU A0-A9 drive its low ten and the MMC3's own CHR bank
/// lines A10-A12 the top three. So the chip is banked in 1 KiB amounts by **bits 0-2 of whatever
/// 1 KiB bank number the chip put in that window**, which is [`Mmc3Board::chr_ram_offset`]'s
/// arithmetic for the TQROM read through [`Mmc3Board::chr_bank_of_window`] — the accessor whose
/// own note says why the bank has to come from the register and not from the window table. That
/// is the same reading mapper 119 has carried since MF-6a3 step 1 and it is recorded in Q-172
/// because the page itself does not state it.
///
/// [`Mmc3Board::chr_ram_offset`]: super::Mmc3Board
/// [`Mmc3Board::chr_bank_of_window`]: super::Mmc3Board
///
/// # Four-screen, and what it does to `$A000`
///
/// The board carries the 4 KiB of nametable the arrangement needs, so the mirroring is
/// four-screen from the wiring and not from byte 6 of a header — the shape [`super::IremLrog017`]
/// already has, and `ppu::VRAM_SIZE` being 4 KiB is why this core needs nothing else for it.
///
/// The page's "`$A000`: As normal MMC3" and its own four-screen VRAM cannot both be literally
/// true: on a four-screen board the cartridge drives CIRAM A10 and A11 itself, so the chip's one
/// mirroring register reaches nothing. [`Mmc3Board::force_ignore_a000`] is already exactly that
/// condition — a four-screen *header* reaches it at load — and this board sets it from its own
/// wiring. Recorded in Q-172; a test pins that a `$A000` write leaves all four pages distinct.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**, for [`super::Mmc3Scramble`]'s reason: the board *holds* an
/// [`Mmc3`] and adds its own register, so inline it would set that enum's size. Its IRQ is the
/// chip's, which is that enum's entry condition, and `try_read_prg` — the jumper — is a method
/// that variant already forwards for five earlier boards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Shero {
    /// The chip itself. Every `$8000-$FFFF` write reaches it unchanged.
    mmc3: Mmc3,
    /// `$4100`'s write side: bit 6 the CHR-RAM select, bits 3-0 the four CHR A18 lines.
    ///
    /// Power-on zero, which the page does not state and which is this build's default for every
    /// register no page gives a power-up value: CHR-ROM, first 256 KiB half everywhere.
    register: u8,
}

impl Mmc3Shero {
    /// `$4100`'s window, the page's own "probably `$C100`" — see the type's note on the two
    /// guessed masks.
    const PORT_MASK: u16 = 0xC100;
    const PORT: u16 = 0x4100;

    /// `$4100` bit 6: "Select CHR-ROM/-RAM — 0: CHR-ROM, 1: CHR-RAM". **Bit 6 and not bit 7**;
    /// the type's note has the column arithmetic.
    const CHR_RAM_SELECT: u8 = 0x40;

    /// The CHR-RAM chip, "8 KiB of CHR-RAM", which `registry::board_memory_sizes` settles at load
    /// because no header field can state it — the TQROM's arm's reason in the same words.
    pub(crate) const CHR_RAM_SIZE: usize = 8 * 1024;

    /// The chip's own bank lines as a mask of the 1 KiB bank number: three of them on 8 KiB.
    const CHR_RAM_BANK_MASK: u8 = 0x07;

    /// Which bit of `$4100` drives CHR A18 over each of the eight 1 KiB windows.
    ///
    /// **The crossing of bits 1 and 0 is the page's, not a transcription slip**: its annotation
    /// lines give bit 1 the region `$1800-$1FFF` and bit 0 the region `$1000-$17FF`, so the four
    /// 2 KiB regions in address order take bits 3, 2, **0** and **1**. Per 1 KiB window that is
    /// bits 3, 3, 2, 2, 0, 0, 1, 1.
    const CHR_A18_LINES: [u8; BankWindows::CHR_WINDOWS] = [3, 3, 2, 2, 0, 0, 1, 1];

    /// CHR A18 as a 1 KiB bank number: 256 KiB is 256 banks of 1 KiB.
    const CHR_A18: i32 = 0x0100;

    /// The jumper a read of `$4100` answers in bit 7 (**Q-172**).
    ///
    /// The page makes it the board's **wiring** — "0: Game title is *Street Heroes*; 1: Game title
    /// is *侍魂 (Shìhún)*" — and no header field, iNES or NES 2.0, can state which of the two
    /// titles a dump is; the page defines no submapper either. So this build answers **0**, the
    /// page's first-listed title, and records the gap: a *Samurai Spirits* dump needs a submapper
    /// the page does not define, or a per-ROM table, and §5 of `docs/review/UNL-census.md` is why
    /// no such table can be built here — there is no ROM of this number to run.
    const JUMPER: u8 = 0x00;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let mut board = Self {
            // 262 and not 4, the tightening every MMC3 clone here makes.
            mmc3: Mmc3::new(memory, mapper),
            register: 0,
        };
        // The board's wiring drives CIRAM A10 and A11, so the chip's mirroring register reaches
        // nothing; see the type's own note.
        board.mmc3.force_ignore_a000();
        board.refresh();
        board
    }

    /// Recomputes the MMC3's windows from its own registers and lays the four CHR A18 lines over
    /// them.
    ///
    /// Called after every write the board takes, on [`super::Somari::refresh`]'s own idempotence
    /// argument: the chip keeps its registers and is asked to recompute from them, and the lines
    /// are ORed into bank numbers the chip can only drive eight bits of — so bit 8 is never
    /// already set and re-applying the overlay cannot count a line twice.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        self.apply_chr_a18();
    }

    /// The four high CHR bits of `$4100`, one per 2 KiB region, in the order
    /// [`Mmc3Shero::CHR_A18_LINES`] gives.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and every board of this family
    /// carries: this runs from one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches.
    #[inline(never)]
    fn apply_chr_a18(&mut self) {
        let register = self.register;
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        banks.or_chr_banks_each(Self::CHR_A18_LINES.map(|bit| {
            if register >> bit & 0x01 == 0 {
                0
            } else {
                Self::CHR_A18
            }
        }));
    }

    /// Whether the CHR-RAM chip is the one the PPU reaches: `$4100` bit 6.
    const fn chr_ram_selected(&self) -> bool {
        self.register & Self::CHR_RAM_SELECT != 0
    }

    /// Where in the CHR-RAM chip the 1 KiB window `window` starts: bits 0-2 of the bank number
    /// the chip put there, the chip's own three address lines above the kilobyte.
    fn chr_ram_offset(&self, window: usize) -> usize {
        usize::from(self.mmc3.chr_bank_of_window(window) & Self::CHR_RAM_BANK_MASK)
            * BankWindows::CHR_WINDOW_SIZE
    }

    /// The eight window offsets into the CHR-RAM chip, for [`Mmc3Shero::chr_peek_view`].
    fn chr_ram_offsets(&self) -> [usize; BankWindows::CHR_WINDOWS] {
        std::array::from_fn(|window| self.chr_ram_offset(window))
    }

    /// A CHR fetch while the RAM chip is selected.
    ///
    /// `#[inline(never)]` for [`Mmc3Board::chr_ram_read`]'s measured reason, which is the same
    /// body in the same dispatch: `Board::ppu_peek`'s arms are inlined into
    /// `Ppu2C02::background_fetch`, 7 % to 11 % of every ROM of lot PR-1's set, and MF-6a2 step 3
    /// priced an inlined CHR-RAM decode there at 8.0 M of set total.
    ///
    /// [`Mmc3Board::chr_ram_read`]: super::Mmc3Board
    #[inline(never)]
    fn chr_ram_read(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let index = usize::from(address) & 0x1FFF;
        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }

        let offset = self.chr_ram_offset(index / BankWindows::CHR_WINDOW_SIZE)
            + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        ram[super::super::wrap_offset(offset, ram.len())]
    }

    /// A PPU write while the RAM chip is selected. `Mapper::ppu_write`'s default drops every
    /// write on a board that has CHR-ROM, and this board has both chips — so without this the
    /// CHR-RAM would be unreachable. `#[inline(never)]` for the reason above, one dispatch over:
    /// `Board::ppu_write`'s arms are inlined into `NesBus::write_internal`.
    #[inline(never)]
    fn chr_ram_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let index = usize::from(address) & 0x1FFF;
        let offset = self.chr_ram_offset(index / BankWindows::CHR_WINDOW_SIZE)
            + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)] = value;
        }
    }

    /// A write the caller has already decoded as `$4100` under the page's "probably `$C100`".
    ///
    /// `#[inline(never)]`, for [`Mmc3Shero::apply_chr_a18`]'s reason.
    #[inline(never)]
    fn write_port(&mut self, value: u8) {
        self.register = value;
        self.refresh();
    }
}

impl Mapper for Mmc3Shero {
    fn mapper_number(&self) -> i32 {
        262
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's one register behind it. Nothing of the
    /// MMC3's own layout moves and no earlier state can name this number (REQ-CORE-090).
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        self.register = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    /// `Cartridge::reset` puts the header's mirroring back just before this runs, so the board has
    /// to state its own — and it is this call, and not [`Mmc3Shero::new`], that makes the board
    /// four-screen, because `Cartridge::create` resets the cartridge it has just built. The same
    /// shape [`super::Mmc3Gouder208::reset`] and [`super::IremLrog017::reset`] already have.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.register = 0;
        memory.set_mirroring(NametableMirroring::FourScreen);
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable()
    }

    /// The jumper, in bit 7 of a read of `$4100`; see [`Mmc3Shero::JUMPER`].
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address & Self::PORT_MASK == Self::PORT {
            return Some(Self::JUMPER);
        }

        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    /// **The `address < 0x8000` guard is deliberately not the first line**, which is
    /// [`super::Mmc3Txc189`]'s own note: the one port is at `$4100`, a range
    /// `Cartridge::cpu_write` reaches, so a board that returned early below `$8000` could never
    /// reach its CHR-RAM at all.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            if address & Self::PORT_MASK == Self::PORT {
                self.write_port(value);
            }

            return;
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: this board's CHR fetches clock the
    /// A12 counter.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// The chip the select names, through the windows that chip is banked by — so a 3D tile
    /// record captured while the RAM is selected carries the RAM's bytes (REQ-CORE-037).
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        let banks = self.banks()?;
        if self.chr_ram_selected() {
            return Some(
                ChrView::new(memory.chr_ram(), banks).with_offsets(self.chr_ram_offsets()),
            );
        }

        Some(ChrView::new(memory.rom().chr_rom(), banks))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.chr_ram_selected() {
            return self.chr_ram_read(memory, address);
        }

        self.banks()
            .map_or(0, |banks| banks.read_chr(address, memory.rom().chr_rom()))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.chr_ram_selected() {
            self.chr_ram_write(memory, address, value);
        }
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
