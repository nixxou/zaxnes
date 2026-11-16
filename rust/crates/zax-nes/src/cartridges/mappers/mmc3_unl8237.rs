use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **215**, the **UNL-8237** and **UNL-8237A**: an MMC3 clone whose register
/// addresses and bank indices are permuted by **one of eight run-time selectable patterns**,
/// with outer PRG and CHR banks and an NROM-like PRG override (MF-UNL-11 step 1, REQ-CORE-102).
///
/// nesdev ("INES Mapper 215"): the number "is used for later single-game as well as multicart
/// releases from **Sugar Softec**. They use an MMC3 clone with a selectable scrambling pattern."
/// Submapper 0 is the **UNL-8237** and submapper 1 the **UNL-8237A**, which differ in one
/// register's bit assignment and nothing else.
///
/// Four original titles — *Earthworm Jim 2*, *Mortal Kombat 3*, *Mortal Kombat 3 Extra 60* and
/// *Pocahontas Part 2* — plus the eight multicarts the chipset was made for
/// (`docs/review/UNL-census.md` §2.5).
///
/// # The register file: `$5000`, `$5001` and `$5007`, all under mask `$F007`
///
/// ```text
/// NROM Override/Mode Register ($5000)        Scrambling Pattern Register ($5007)
///  D~7654 3210                                D~7654 3210
///    MCS. BBBb                                  .... .MMM
///    |||  ++++- Select 16 KiB PRG-ROM bank             +++- Select MMC3 register address
///    |||        at CPU $8000-$BFFF and $C000-$FFFF          and index scrambling mode (0-7)
///    ||+------- 0: Do not replace bit 0 (NROM-128)
///    ||         1: Replace bit 0 with CPU A14 (NROM-256)
///    |+-------- 0: Do not replace PRG A17 and CHR A17 from the MMC3 (256 KiB outer bank)
///    |          1: Replace PRG A17 and CHR A17 with $5001 bits 4 and 5, respectively
///    |             (128 KiB outer bank)
///    +--------- 0: Use PRG bank from MMC3; ignore bits 0-3/5
///               1: Ignore PRG bank from MMC3; apply bits 0-3/5
///  Power-up value: $00                        Power-up value: $00
///
/// Outer Bank Register ($5001), submapper 0   Outer Bank Register ($5001), submapper 1
///  D~7654 3210                                D~7654 3210
///    ..cp CCPP                                  ..cp P.PP
///      || ||++- 256 KiB outer PRG (A18/A19)           CCC.
///      || ++--- 256 KiB outer CHR (A18/A19)        || +|++- 256 KiB outer PRG (A18-A20)
///      |+------ 128 KiB outer PRG (A17)             || +++-- 256 KiB outer CHR (A18-A20)
///      +------- 128 KiB outer CHR (A17)             |+------ 128 KiB outer PRG (A17)
///                                                   +------- 128 KiB outer CHR (A17)
///  Power-up value: $xF                        Power-up value: $xF
/// ```
///
/// The mask is `$F007`, so the three registers are `$5000`, `$5001` and `$5007` repeating every
/// eight bytes through the whole of `$5000-$5FFF`; `$5002`-`$5006` are not decoded.
///
/// **The page gives `$5001` no readable counterpart and this board answers no read.** Its own
/// *Similar Mappers* line is the evidence: "[NES 2.0 Mapper 258] adds readable protection
/// registers in the `$5000-$5007` range" — *adds*, so 215 has none.
///
/// # Submapper 1's `$5001` is the one ambiguous diagram in this lot (**Q-171**)
///
/// The UNL-8237A's diagram is **two bit rows overlaid**, `P.PP` over `CCC.`, and four register
/// bits (3-0) carry six outer lines between them. The reading implemented is the one both
/// annotation lines support — the PRG line is `+|++-`, whose `|` at bit 2 passes the connector
/// down to the CHR line's `+++--` — so
///
/// * **PRG A20, A19, A18 = `$5001` bits 3, 1, 0**, and
/// * **CHR A20, A19, A18 = `$5001` bits 3, 2, 1**,
///
/// which makes both fields three bits wide as their own text ("A18-A20") says, with bits 3 and 1
/// shared between the two. Q-171 records the alternative — the CHR row read one place right, bits
/// 2, 1, 0 — and that the page cannot distinguish them. Submapper 0's diagram is unambiguous:
/// `CCPP` is PRG A19/A18 at bits 1-0 and CHR A19/A18 at bits 3-2.
///
/// # The two scramble tables
///
/// The page gives the whole of `$8000-$FFFF`'s behaviour as two tables indexed by `$5007`, which
/// are [`Mmc3Unl8237::ADDRESS_SCRAMBLE`] and [`Mmc3Unl8237::DATA_SCRAMBLE`]. **Five of the eight
/// address rows and four of the eight data rows are the identity**: modes 0, 5, 6 and 7 scramble
/// nothing at all, mode 2 scrambles data only, and modes 1, 3 and 4 scramble both. That is not a
/// transcription slip — it is what makes a multicart's menu, which runs before any `$5007` write,
/// an ordinary MMC3.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**, for [`super::Mmc3Scramble`]'s own reason: the board *holds*
/// an [`Mmc3`] and adds its own registers, so inline it would set that enum's size, and MF-6b
/// step 1 priced one byte of displacement in that payload at +0.009 % to +0.027 % on eleven of
/// lot PR-1's thirteen ROMs. Its IRQ **is** the MMC3's, counter included, which is that enum's
/// entry condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Unl8237 {
    /// The chip itself. Every `$8000-$FFFF` write reaches it — with its address and, on `$8000`,
    /// its low three data bits permuted — and what leaves are the bank numbers its registers
    /// produce, which [`Mmc3Unl8237::refresh`] lays this board's banking over.
    mmc3: Mmc3,
    /// `$5000`, as written: `MCS. BBBb`.
    mode: u8,
    /// `$5001`, as written. Power-on `$0F`; see [`Mmc3Unl8237::OUTER_POWER_ON`].
    outer: u8,
    /// `$5007`'s low three bits: which row of the two scramble tables is in force.
    scramble: u8,
    /// Submapper 1, the **UNL-8237A**: the second reading of [`Mmc3Unl8237::outer`].
    ///
    /// A field and not a `const` parameter, for [`super::Mmc3Scramble`]'s reason: it is read only
    /// by [`Mmc3Unl8237::outer_lines`], which runs on a register write and on no fetch or cycle
    /// path, so a second monomorphisation would buy nothing and cost a second set of bodies.
    is_8237a: bool,
}

impl Mmc3Unl8237 {
    /// Mask `$F007` with the high nibble `$5`: the register window is the whole of `$5000-$5FFF`
    /// and which register a write lands on is `address & $07`.
    const WINDOW_MASK: u16 = 0xF000;
    const WINDOW: u16 = 0x5000;

    /// `$5001`'s power-up value. The page gives **`$xF`** — the low nibble fixed and the high one
    /// unknown — so the two A17 lines in bits 4 and 5 are the `x`.
    ///
    /// `$0F` is the value used, and nothing can observe the choice out of reset: those two bits
    /// are read only while `$5000` bit 6 is set and `$5000` powers up at `$00`. **Q-171** records
    /// the `x`.
    ///
    /// The page's own *Note* makes this the **reset** value as well: "The Outer Bank Register is
    /// reset to its power-up value if a Reset has been detected via an interruption of the M2
    /// signal." See [`Mapper::reset`] below.
    const OUTER_POWER_ON: u8 = 0x0F;

    /// The eight addresses of an MMC3's register window, indexed by
    /// [`Mmc3Unl8237::port_index`] — the column headings of the page's address table.
    const MMC3_PORTS: [u16; 8] = [
        0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001,
    ];

    /// Which MMC3 register each of the eight written addresses means, as
    /// [`Mmc3Unl8237::MMC3_PORTS`] indices, one row per `$5007` value — nesdev ("INES Mapper
    /// 215"), *MMC3-compatible registers*:
    ///
    /// ```text
    ///  $5007  -------------address written-----------
    ///  value  8000 8001 A000 A001 C000 C001 E000 E001
    ///  0      8000 8001 A000 A001 C000 C001 E000 E001
    ///  1      A001 A000 8000 C000 8001 C001 E000 E001
    ///  2      8000 8001 A000 A001 C000 C001 E000 E001
    ///  3      C001 8000 8001 A000 A001 E001 E000 C000
    ///  4      A001 8001 8000 C000 A000 C001 E000 E001
    ///  5      8000 8001 A000 A001 C000 C001 E000 E001
    ///  6      8000 8001 A000 A001 C000 C001 E000 E001
    ///  7      8000 8001 A000 A001 C000 C001 E000 E001
    /// ```
    ///
    /// `$E000` — the IRQ disable — is fixed in every row, and `$E001` is moved by **mode 3 and
    /// only mode 3**, where a write to `$C001` means `$E001` and a write to `$E001` means `$C000`.
    const ADDRESS_SCRAMBLE: [[usize; 8]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [3, 2, 0, 4, 1, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [5, 0, 1, 2, 3, 7, 6, 4],
        [3, 1, 0, 4, 2, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
    ];

    /// Which MMC3 bank register the low three bits of a `$8000` value mean, one row per `$5007`
    /// value — the page's second table:
    ///
    /// ```text
    ///  $5007  -value written-
    ///  value  0 1 2 3 4 5 6 7
    ///  0      0 1 2 3 4 5 6 7
    ///  1      0 2 6 1 7 3 4 5
    ///  2      0 5 4 1 7 2 6 3
    ///  3      0 6 3 7 5 2 4 1
    ///  4      0 2 5 3 6 1 7 4
    ///  5      0 1 2 3 4 5 6 7
    ///  6      0 1 2 3 4 5 6 7
    ///  7      0 1 2 3 4 5 6 7
    /// ```
    ///
    /// Index 0 is fixed in every row — the 2 KiB CHR register R0 is never moved — and mode 2 is
    /// the one row that permutes data while leaving the addresses alone.
    const DATA_SCRAMBLE: [[u8; 8]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 2, 6, 1, 7, 3, 4, 5],
        [0, 5, 4, 1, 7, 2, 6, 3],
        [0, 6, 3, 7, 5, 2, 4, 1],
        [0, 2, 5, 3, 6, 1, 7, 4],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 1, 2, 3, 4, 5, 6, 7],
    ];

    /// The 8 KiB PRG banks the MMC3 still drives with `$5000` bit 6 **clear**: A13-A17, 256 KiB.
    const PRG_INNER_256K: i32 = 0x1F;
    /// And with it set: A13-A16, 128 KiB, A17 coming from `$5001` bit 4.
    const PRG_INNER_128K: i32 = 0x0F;
    /// The 1 KiB CHR banks the MMC3 still drives with `$5000` bit 6 clear: A10-A17, 256 KiB.
    const CHR_INNER_256K: i32 = 0xFF;
    /// And with it set: A10-A16, 128 KiB, A17 coming from `$5001` bit 5.
    const CHR_INNER_128K: i32 = 0x7F;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let header = memory.rom().header();
        let mut board = Self {
            // 215 and not 4, the tightening MF-6a3 step 1 made for the TQROM and every MMC3
            // clone since: `Mmc3Board::new` reads a submapper only for mapper 4, so this
            // number's own submapper 1 cannot be taken for an MMC6, an MMC3A or a hard-wired
            // -mirroring MMC3C. It is read here, where it belongs.
            mmc3: Mmc3::new(memory, mapper),
            mode: 0,
            outer: Self::OUTER_POWER_ON,
            scramble: 0,
            is_8237a: header.is_nes20 && header.submapper == 1,
        };
        board.refresh();
        board
    }

    /// Where in [`Mmc3Unl8237::MMC3_PORTS`] the written address `address` sits, which is
    /// `$E001`'s three live bits packed: A13, A14 and A0.
    fn port_index(address: u16) -> usize {
        usize::from(((address >> 12) & 0x06) | (address & 0x01))
    }

    /// The outer bank lines of `$5001`, as `(prg, chr)` **256 KiB bank numbers** — two bits on the
    /// UNL-8237 and three on the UNL-8237A.
    ///
    /// See the type's own note on the UNL-8237A's overlaid diagram and **Q-171**.
    fn outer_lines(&self) -> (i32, i32) {
        let outer = self.outer;
        if self.is_8237a {
            // `P.PP` over `CCC.`: PRG A20/A19/A18 = bits 3/1/0, CHR A20/A19/A18 = bits 3/2/1.
            (
                i32::from((outer & 0x03) | ((outer >> 1) & 0x04)),
                i32::from((outer >> 1) & 0x07),
            )
        } else {
            // `CCPP`: PRG A19/A18 = bits 1/0, CHR A19/A18 = bits 3/2.
            (i32::from(outer & 0x03), i32::from((outer >> 2) & 0x03))
        }
    }

    /// Recomputes the MMC3's windows from its own registers, lays the NROM override over the PRG
    /// half and then puts the outer banks over both halves.
    ///
    /// Called after **every** write the board takes, on [`super::Somari::refresh`]'s own
    /// argument: the chip keeps its registers and is asked to recompute from them, so the overlay
    /// is never counted twice and the board does not have to remember what it last applied.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        self.apply_override();
        self.apply_outer();
    }

    /// `$5000`'s NROM-like PRG override: bit 7 set replaces the MMC3's two PRG windows with one
    /// 16 KiB bank at `$8000-$BFFF` **and** `$C000-$FFFF`, bit 5 replacing the bank's bit 0 with
    /// CPU A14.
    ///
    /// The same mechanism, and the same two sentences, as mappers 114, 115, 123 and 187
    /// ([`super::Mmc3Scramble::apply_override`]) — the bits sit where 114's and 182's do, `M.S.
    /// BBBb` with bit 6 given a second meaning this board needs and those do not.
    ///
    /// The bank is written **raw**; [`Mmc3Unl8237::apply_outer`] then masks it to the inner
    /// region and ORs the outer lines in, exactly as it does the chip's own bank numbers. So the
    /// four-bit field addresses 256 KiB with bit 6 clear and 128 KiB with it set, its top bit
    /// then being A17 from `$5001` — which is what "ignore bits 0-3/5" and "replace PRG A17"
    /// mean together.
    fn apply_override(&mut self) {
        let register = self.mode;
        // "0: Use PRG bank from MMC3; ignore bits 0-3/5."
        if register & 0x80 == 0 {
            return;
        }

        let bank = i32::from(register & 0x0F);
        let nrom_256 = register & 0x20 != 0;
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        if nrom_256 {
            // "Replace bit 0 with CPU A14": a 32 KiB view, the two halves of one even/odd pair.
            let base = bank & 0x0E;
            banks.map_prg_16k(0, base);
            banks.map_prg_16k(2, base | 1);
        } else {
            // "Do not replace bit 0": the same 16 KiB at both windows.
            banks.map_prg_16k(0, bank);
            banks.map_prg_16k(2, bank);
        }
    }

    /// The outer PRG and CHR banks of `$5001`, and `$5000` bit 6's two A17 lines.
    ///
    /// One `(bank & inner) | outer` per window on both tables, which is what both of the page's
    /// phrasings describe: the 256 KiB outer bank sits **above** everything the MMC3 drives, and
    /// bit 6 set "replace[s] PRG A17 and CHR A17" — a line taken away from the chip and given to
    /// `$5001`, so the inner mask narrows by one bit as the line is ORed back in.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and every board of this family
    /// carries: this runs from one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches, and a cartridge of this number pays a `callq` per
    /// register write where nobody else pays anything.
    #[inline(never)]
    fn apply_outer(&mut self) {
        let narrow = self.mode & 0x40 != 0;
        let (prg_outer, chr_outer) = self.outer_lines();
        let (prg_inner, chr_inner) = if narrow {
            (Self::PRG_INNER_128K, Self::CHR_INNER_128K)
        } else {
            (Self::PRG_INNER_256K, Self::CHR_INNER_256K)
        };

        // The 256 KiB outer bank is 32 PRG banks of 8 KiB and 256 CHR banks of 1 KiB above the
        // inner region; the two A17 lines are one bank bit below it on each table.
        let mut prg_bits = prg_outer << 5;
        let mut chr_bits = chr_outer << 8;
        if narrow {
            prg_bits |= i32::from((self.outer >> 4) & 0x01) << 4;
            chr_bits |= i32::from((self.outer >> 5) & 0x01) << 7;
        }

        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        banks.remap_prg_banks(prg_inner, prg_bits);
        banks.remap_chr_banks(chr_inner, chr_bits);
    }

    /// A write inside `$5000-$5FFF`: the three registers under mask `$F007`.
    ///
    /// `#[inline(never)]`, for [`Mmc3Unl8237::apply_outer`]'s reason.
    #[inline(never)]
    fn write_low(&mut self, address: u16, value: u8) {
        match address & 0x07 {
            0 => self.mode = value,
            1 => self.outer = value,
            // `.... .MMM`.
            7 => self.scramble = value & 0x07,
            // $5002-$5006 are not decoded: the mask reads A2-A0 and names three of the eight.
            _ => return,
        }

        self.refresh();
    }
}

impl Mapper for Mmc3Unl8237 {
    fn mapper_number(&self) -> i32 {
        215
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's three registers behind it. Nothing of the
    /// MMC3's own layout moves and no earlier state can name this number (REQ-CORE-090).
    ///
    /// `is_8237a` is **not** written: it is the image's submapper, settled at load, which is
    /// wiring and not state — the reason `Mmc3Board::dots_per_clock` gives for itself.
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.mode);
        writer.write_u8(self.outer);
        writer.write_u8(self.scramble);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The chip's own `load_registers` recomputes its windows from its registers, which is
        // what `refresh` below wants to find: the overlay is never applied twice.
        self.mmc3.load_registers(reader)?;
        self.mode = reader.read_u8()?;
        self.outer = reader.read_u8()?;
        self.scramble = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    /// Back to the power-up values, which for `$5001` is what the page's *Note* states in so many
    /// words: "The Outer Bank Register is reset to its power-up value if a Reset has been detected
    /// via an interruption of the M2 signal. This makes it difficult to dump these cartridges
    /// using a Kazzo device."
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.mode = 0;
        self.outer = Self::OUTER_POWER_ON;
        self.scramble = 0;
        self.refresh();
    }

    /// **The one board of this build that reacts to a press on RESET**, and the page says so in
    /// the same breath as it gives the power-up value: *"The Outer Bank Register is reset to its
    /// power-up value if a Reset has been detected via an interruption of the M2 signal. This
    /// makes it difficult to dump these cartridges using a Kazzo device."*
    ///
    /// So `$5001` alone goes back, and that is the whole of it: the page says nothing about the
    /// mode register, the scramble register or the MMC3 chip behind them, and the connector has
    /// no reset pin for them to see (see [`Mapper::soft_reset`]). A reset-based multicart is
    /// exactly a board that changes the game it offers when the player presses RESET, so taking
    /// the outer bank away is the behaviour and not a side effect of one.
    fn soft_reset(&mut self, _memory: &mut CartridgeMemory) {
        self.outer = Self::OUTER_POWER_ON;
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

    /// The three registers, then the chip with its address and its `$8000` data permuted.
    ///
    /// **Bits 3-7 of a `$8000` write pass through.** The page states only that "bits 6 and 7 of
    /// register `$8000` are kept as they are", which this satisfies; bits 3-5 are don't-cares on
    /// an MMC3 and only three data lines are permuted, so passing them keeps the chip's `select`
    /// byte — and with it the save-state — independent of which scramble mode happens to be in
    /// force. `$8001` and `$A000-$FFFF` are "never scrambled".
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & Self::WINDOW_MASK == Self::WINDOW {
            self.write_low(address, value);
            return;
        }

        if address < 0x8000 {
            return;
        }

        let row = usize::from(self.scramble);
        let port = Self::MMC3_PORTS[Self::ADDRESS_SCRAMBLE[row][Self::port_index(address)]];
        let value = if port == 0x8000 {
            (value & 0xF8) | Self::DATA_SCRAMBLE[row][usize::from(value & 0x07)]
        } else {
            value
        };

        self.mmc3.write_register(memory, port, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: this board's CHR fetches clock the
    /// A12 counter, so the PPU takes the full path.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc3.ppu_write(memory, address, value);
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
