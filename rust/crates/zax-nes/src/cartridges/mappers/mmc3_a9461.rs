use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **219**, the 卡聖 (Kǎshèng) **A9461**: an MMC3 clone with a second, **extended**
/// banking mode — four independent 8 KiB PRG windows and eight 1 KiB CHR windows reached through
/// a wider register index — above a 128 KiB outer bank of its own (MF-UNL-11 step 3,
/// REQ-CORE-102).
///
/// nesdev ("INES Mapper 219") gives the number two games: *Toy Story*, the port, and
/// *Super 1997 4-in-1* (NT-8029). `docs/review/UNL-census.md` §2.5 grades the page **partial**,
/// and what that grade is about is the one reading this board had to settle; see below.
///
/// # The register file
///
/// ```text
/// Mode Register ($8002)            Mask: $E003
///  CPMR RRRR
///  ||++-++++-*Select bank register number (0-7 in MMC3 mode, $08-$26 in Extended mode)
///  ||+------- Select mode   0: MMC3 mode   1: Extended mode
///  |+--------*1: PRG A14 inversion in MMC3 mode
///  +---------*1: CHR A12 inversion in MMC3 mode
///
/// Outer Bank Bit 0 ($5002)   .... ...B   bit 0 of the 128 KiB outer PRG-ROM and CHR-ROM bank
/// Outer Bank Bit 1 ($5003)   ..B. ....   bit 1 of the same
/// ```
///
/// and, "used only if `$8002` bit 5=1", four index patterns on the chip's own `$8000`/`$8001`
/// pair under mask `$E001`:
///
/// | `$8000` value | `$8001` value | what it does |
/// | --- | --- | --- |
/// | `..10 .RRR`, `$23-$26` | `..BB BB..` | an 8 KiB PRG bank, its four bits **reversed**, at the window `RRR` names: 6 → `$8000`, 5 → `$A000`, 4 → `$C000`, 3 → `$E000` |
/// | `..00 1..0` or `..01 ...0`, `$08-$1E` even | `.... .654` | the **CHR bank number latch**: bits 4-6 of every following CHR switch |
/// | `..00 1RR1`, `$09-$0F` odd | `BBBB BBB.` | a 1 KiB CHR bank at PPU `$0000-$0FFF`, `RR` picking the quarter — and bit 1 of the *index* is ORed into bit 0 of the bank, which is what makes it a "pseudo 2 KiB switch" |
/// | `..01 RR.1`, `$11-$1F` odd | `BBBB BBB.` | a 1 KiB CHR bank at PPU `$1000-$1FFF`, `RR` picking the quarter |
///
/// `$A000`, `$A001`, `$C000`, `$C001`, `$E000` and `$E001` are "as on regular MMC3" in both
/// modes, so the mirroring, the PRG-RAM protect and the whole scanline counter are the chip's.
///
/// # The mode bit is latched by `$8002` and the index by every even address (**Q-171**)
///
/// The `R RRRR` field is **five** bits and `$26` needs six, so the sixth is bit 5 — the mode bit
/// itself. That alone would be a contradiction: the extended PRG indices `$23-$26` have bit 5
/// **set** and all three CHR index patterns have it **clear**, yet every one of the four sections
/// says "used only if `$8002` bit 5=1". The two statements can only both hold if the mode bit is
/// latched by a write to **`$8002`** and the index by any even address of the window — and that
/// is also why the page annotates `$8002` bits 6 and 7 as "PRG A14 inversion" and "CHR A12
/// inversion", which are the MMC3's own `$8000` bits 6 and 7. `$8002` is the chip's `$8000` port
/// with bit 5 given a meaning.
///
/// So this board latches [`Mmc3A9461::index`] from every even write in `$8000-$9FFF`, latches
/// [`Mmc3A9461::extended`] from `$8002` alone, and passes every even write on to the chip — which
/// is where bits 6 and 7 have to land. In extended mode the **odd** writes are this board's four
/// registers and do not reach the chip at all, so the chip keeps the bank registers it had and
/// clearing the mode bit gives an MMC3 back without a reload.
///
/// # `Mask: $5003` is read as `$F003` (**Q-171**)
///
/// Taken literally, `$5003` decodes A14, A12, A1 and A0 and not A15 — so `$D002` and `$D003`,
/// which are the MMC3's **IRQ latch and reload**, would also be the outer bank register. The
/// window implemented is `$F003`, which is the two addresses the page's own section titles name
/// (`$5002`, `$5003`) together with their aliases inside `$5000-$5FFF` and nothing above.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**, for [`super::Mmc3Scramble`]'s reason: the board *holds* an
/// [`Mmc3`] and adds its own register file, so inline it would set that enum's size. Its IRQ is
/// the chip's, which is that enum's entry condition, and every method it needs is one the variant
/// already forwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3A9461 {
    /// The chip itself. Every even `$8000-$9FFF` write and the whole of `$A000-$FFFF` reach it;
    /// in extended mode the odd writes do not.
    mmc3: Mmc3,
    /// The last value written to an even address in `$8000-$9FFF`, six bits of it live: the bank
    /// register number the page quotes as "0-7 in MMC3 mode, `$08-$26` in Extended mode".
    index: u8,
    /// `$8002` bit 5: the extended mode.
    extended: bool,
    /// The four extended 8 KiB PRG banks, in window order `$8000`, `$A000`, `$C000`, `$E000`.
    ext_prg: [u8; 4],
    /// The eight extended 1 KiB CHR banks, in window order.
    ext_chr: [u8; 8],
    /// The CHR bank number latch, as written: its low three bits are bits 4-6 of every following
    /// CHR switch's bank number.
    chr_latch: u8,
    /// `$5002` and `$5003`: the two-bit 128 KiB outer bank, over PRG **and** CHR.
    ///
    /// Power-on zero; the page's "The Outer Bank register must be initialized to `$03` for the
    /// menu to appear" is a statement about the *menu's* code and not about the board, which is
    /// why nothing here presets it.
    outer: u8,
}

impl Mmc3A9461 {
    /// The outer bank window: `$5002` and `$5003` and their aliases inside `$5000-$5FFF`. See the
    /// type's note on why the page's `Mask: $5003` is read as `$F003`.
    const OUTER_MASK: u16 = 0xF003;
    const OUTER_BIT0: u16 = 0x5002;
    const OUTER_BIT1: u16 = 0x5003;

    /// The 8 KiB PRG banks the chip and the extended registers drive: A13-A16, **128 KiB**, which
    /// is what the page says of both modes ("PRG-ROM and CHR-ROM banks masked to 128 KiB" in MMC3
    /// mode, and four bits of bank number in extended mode).
    const PRG_INNER: i32 = 0x0F;
    /// And the 1 KiB CHR banks: A10-A16, 128 KiB, seven bits of bank number either way.
    const CHR_INNER: i32 = 0x7F;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let mut board = Self {
            // 219 and not 4, the tightening every MMC3 clone here makes.
            mmc3: Mmc3::new(memory, mapper),
            index: 0,
            extended: false,
            ext_prg: [0; 4],
            ext_chr: [0; 8],
            chr_latch: 0,
            outer: 0,
        };
        board.refresh();
        board
    }

    /// Recomputes the MMC3's windows from its own registers, lays the extended mode's twelve
    /// windows over them when it is on, and puts the outer bank over both halves either way.
    ///
    /// Called after every write the board takes, on [`super::Somari::refresh`]'s own idempotence
    /// argument: the chip keeps its registers and is asked to recompute from them.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        if self.extended {
            self.apply_extended();
        }
        self.apply_outer();
    }

    /// The extended mode's twelve windows, written absolutely: four 8 KiB PRG banks and eight
    /// 1 KiB CHR banks, which is what "four independent PRG windows" means — the chip's own
    /// `$8000` bit 6 and bit 7 reach nothing here because no window is left for them to swap.
    fn apply_extended(&mut self) {
        let prg = self.ext_prg;
        let chr = self.ext_chr;
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        for (slot, bank) in prg.into_iter().enumerate() {
            banks.map_prg_8k(slot, i32::from(bank));
        }
        for (slot, bank) in chr.into_iter().enumerate() {
            banks.map_chr_1k(slot, i32::from(bank));
        }
    }

    /// The 128 KiB outer bank of `$5002`/`$5003`, over PRG and CHR alike: one
    /// `(bank & inner) | outer` per window on both tables.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and every board of this family
    /// carries: this runs from one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches.
    #[inline(never)]
    fn apply_outer(&mut self) {
        let outer = i32::from(self.outer & 0x03);
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        // 128 KiB is 16 PRG banks of 8 KiB and 128 CHR banks of 1 KiB.
        banks.remap_prg_banks(Self::PRG_INNER, outer << 4);
        banks.remap_chr_banks(Self::CHR_INNER, outer << 7);
    }

    /// A write below `$8000`: the two outer-bank bits. `true` when this board decoded it.
    ///
    /// `#[inline(never)]`, for [`Mmc3A9461::apply_outer`]'s reason.
    #[inline(never)]
    fn write_outer(&mut self, address: u16, value: u8) -> bool {
        match address & Self::OUTER_MASK {
            // `$5002`: `.... ...B`.
            Self::OUTER_BIT0 => self.outer = (self.outer & 0x02) | (value & 0x01),
            // `$5003`: `..B. ....`.
            Self::OUTER_BIT1 => self.outer = (self.outer & 0x01) | ((value >> 4) & 0x02),
            _ => return false,
        }

        self.refresh();
        true
    }

    /// The extended mode's `$8001`: which of the four registers the latched index names, and what
    /// the written byte means to it. `true` when it was one of the four.
    ///
    /// `#[inline(never)]`, for [`Mmc3A9461::apply_outer`]'s reason.
    #[inline(never)]
    fn write_extended(&mut self, value: u8) -> bool {
        let index = self.index;
        // `..10 .RRR`, `$23-$26`: bit 5 set and bit 4 clear.
        if index & 0x30 == 0x20 {
            let window = index & 0x07;
            if !(3..=6).contains(&window) {
                return false;
            }

            // "6: $8000-$9FFF, 5: $A000-$BFFF, 4: $C000-$DFFF, 3: $E000-$FFFF."
            let slot = usize::from(6 - window);
            // "Bits 0-3 (reversed order) of 8 KiB PRG-ROM bank number", the field being
            // `..BB BB..`: the bank's bit 0 is the value's bit 5 and its bit 3 the value's bit 2.
            self.ext_prg[slot] = ((value >> 5) & 0x01)
                | ((value >> 3) & 0x02)
                | ((value >> 1) & 0x04)
                | ((value << 1) & 0x08);
            return true;
        }

        if index & 0x20 != 0 {
            return false;
        }

        // The three CHR patterns, which bit 0 of the index tells apart from each other and bit 4
        // splits by pattern table: `..00 1..0` / `..01 ...0` are the latch, `..00 1RR1` the
        // pseudo 2 KiB switch over PPU $0000-$0FFF and `..01 RR.1` the 1 KiB switch over
        // $1000-$1FFF.
        let high_table = index & 0x10 != 0;
        if index & 0x01 == 0 {
            // The latch. `..00 1..0` needs bit 3; `..01 ...0` needs nothing more than bit 4.
            if !high_table && index & 0x08 == 0 {
                return false;
            }

            // `.... .654`: the value's bits 0-2 are the bank's bits 4-6.
            self.chr_latch = value;
            return true;
        }

        let (window, bank) = if high_table {
            // `..01 RR.1`, `$11-$1F` odd: `RR` is bits 3-2, and no bit of the index reaches the
            // bank. The four quarters of PPU $1000-$1FFF.
            (4 + usize::from((index >> 2) & 0x03), (value >> 1) & 0x7F)
        } else {
            if index & 0x08 == 0 {
                return false;
            }

            // `..00 1RR1`, `$09-$0F` odd: `RR` is bits 2-1, and "the value is OR'd with bit 1
            // SHR 1 of the value written to $8000" — so the low bit of the quarter number is
            // also the low bit of the bank, which is the "pseudo 2 KiB" of the section's title.
            (
                usize::from((index >> 1) & 0x03),
                ((value >> 1) & 0x7F) | ((index >> 1) & 0x01),
            )
        };

        // "then OR'd with bits 0-2 SHL 4 of the value written to the CHR-ROM latch."
        self.ext_chr[window] = bank | ((self.chr_latch & 0x07) << 4);
        true
    }
}

impl Mapper for Mmc3A9461 {
    fn mapper_number(&self) -> i32 {
        219
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's register file behind it. Nothing of the
    /// MMC3's own layout moves and no earlier state can name this number (REQ-CORE-090).
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.index);
        writer.write_bool(self.extended);
        writer.write_raw(&self.ext_prg);
        writer.write_raw(&self.ext_chr);
        writer.write_u8(self.chr_latch);
        writer.write_u8(self.outer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        self.index = reader.read_u8()?;
        self.extended = reader.read_bool()?;
        reader.read_into(&mut self.ext_prg)?;
        reader.read_into(&mut self.ext_chr)?;
        self.chr_latch = reader.read_u8()?;
        self.outer = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.index = 0;
        self.extended = false;
        self.ext_prg = [0; 4];
        self.ext_chr = [0; 8];
        self.chr_latch = 0;
        self.outer = 0;
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

    /// The outer bank below `$8000`, then the chip's own window with the mode bit and the index
    /// taken out of it on the way past.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            self.write_outer(address, value);
            return;
        }

        if address < 0xA000 {
            if address & 0x01 == 0 {
                self.index = value;
                // `$8002` and its aliases: the mode bit, which no other even address latches.
                if address & 0x02 != 0 {
                    self.extended = value & 0x20 != 0;
                }

                // Bits 6 and 7 are the chip's own, which is what the page's two annotations on
                // `$8002` say, so an even write always reaches it.
                self.mmc3.write_register(memory, 0x8000, value);
                self.refresh();
                return;
            }

            if self.extended {
                self.write_extended(value);
                self.refresh();
                return;
            }
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: this board's CHR fetches clock the
    /// A12 counter.
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
