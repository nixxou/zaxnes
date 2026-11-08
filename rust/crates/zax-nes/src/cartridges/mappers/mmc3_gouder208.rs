use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **208**, the unnamed Gouder board of *快打傳説 — Street Fighter IV*: an MMC3 clone
/// whose PRG-ROM is banked in **32 KiB** amounts by a register of its own, with a **256-byte
/// protection lookup table** behind four registers (MF-UNL-11 step 2, REQ-CORE-102).
///
/// nesdev ("INES Mapper 208"): "This mapper is used on exactly one known game: 快打傳説 Street
/// Fighter IV. It uses MMC3-compatible registers for CHR ROM bankswitching and IRQ control, and
/// additional registers for PRG ROM bankswitching, mirroring control, and protection." The board
/// carries no name on the page; the copyright notice on the title screen is **Gouder**'s, which is
/// what `docs/review/UNL-census.md` §2.5 files it under.
///
/// # The registers
///
/// ```text
/// PRG Bank and Mirroring ($4800-$4FFF, $6800-$6FFF, write-only)
///  7  bit  0
///  ---- ----
///  ..MP ...P
///    || |
///    |+----+- Select 32KB page of PRG-ROM at $8000
///    +------- Select mirroring
///               0: Vertical
///               1: Horizontal
///  The initial value of this register is $11.
/// ```
///
/// * **Protection Table Index (`$5000-$57FF`, write-only)**: "Specifies the index into the
///   256-byte table that will be used for the next write to `$5800-$5FFF`."
/// * **Protection Data Access (`$5800-$5FFF`, write)**: "Writes to one of the four protection
///   registers determined by the two lower bits of the address being written to. The value that
///   is written to the protection register is the CPU data XORed with the table entry indexed by
///   the last write to `$5000-$57FF`."
/// * **Protection Data Access (`$5800-$5FFF`, read)**: "Returns the content of one of the four
///   protection registers determined by the two lower bits of the address being read. The correct
///   result is checked for by executing code at `$5800`, which in the case of proper emulation of
///   the protection scheme will result in the execution of a JMP opcode to a desired address."
/// * `$C000-$FFFF`, the four IRQ ports, "identical to the respective MMC3 registers", and the
///   chip's `$8000`/`$8001` for the six CHR windows the page's *Banks* section lists.
///
/// The 32 KiB page is register **bits 4 and 0**, in that order, which is what the diagram's
/// connector joins; the initial `$11` is therefore page **3** with vertical mirroring.
///
/// # `$A000` is inert on submapper 0, and not on submapper 1 (**Q-171**)
///
/// The page's register list has **no `$A000` section** and puts "mirroring control" among the
/// "additional registers", so the chip's own mirroring register drives nothing on this board —
/// [`Mmc3Board::force_ignore_a000`] is exactly that condition and the board sets it at load, the
/// shape mapper 14 already uses.
///
/// Submapper 1 says the opposite in its own words, so it does **not**: "CHR ROM bankswitching,
/// mirroring and IRQ control are performed using the normal MMC3 registers."
///
/// # Submapper 1, the copyright-stripped hack
///
/// > A version of the game that removes the "Gouder Co. Ltd." copyright notice from the title
/// > screen uses a different MMC3-like mapper that is incompatible to the description above. …
/// > There are no additional registers, even as the ROM image retains some leftovers of code
/// > writing to `$5800`. Instead of switching PRG ROM banks MMC3-style, there is only one 32k PRG
/// > ROM bank at CPU `$8000`. Its value is determined by MMC3 bank register 6; the actual bank
/// > number is the bank register value SHR 2.
///
/// So on that submapper every one of this board's four ports is dead — the `$5800` leftovers
/// included, which is why [`Mapper::try_read_prg`] answers nothing there — and the 32 KiB page
/// comes from the chip's own `R6`. The number is still not mapper 4, because the PRG window is
/// 32 KiB and an MMC3's is two eights.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**: the board holds an [`Mmc3`] and its own register file, the
/// 256-byte table being a `const` and not a field, so inline it would set that enum's size — the
/// reason [`super::Mmc3Scramble`] is boxed. Its IRQ is the MMC3's, which is that enum's entry
/// condition, and the one method it adds to what the variant already forwards is none.
///
/// [`Mmc3Board::force_ignore_a000`]: super::Mmc3Board::force_ignore_a000
/// [`Mmc3Board`]: super::Mmc3Board
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Gouder208 {
    /// The chip itself. `$8000`, `$8001` and the four IRQ ports reach it unchanged; its PRG bank
    /// lines are not connected, exactly as on [`super::Mmc3Txc189`].
    mmc3: Mmc3,
    /// `$4800-$4FFF` / `$6800-$6FFF`, as written: `..MP ...P`. Power-on `$11`.
    bank: u8,
    /// `$5000-$57FF`: the index into [`Mmc3Gouder208::PROTECTION_TABLE`] the next `$5800` write
    /// will use.
    index: u8,
    /// `$5800-$5FFF`: the four protection registers, indexed by the low two address bits.
    protection: [u8; 4],
    /// Submapper 1, the copyright-stripped hack: no additional register at all, and the 32 KiB
    /// page from the chip's own `R6`.
    is_stripped: bool,
}

impl Mmc3Gouder208 {
    /// `(address & $D800) == $4800`, which is **exactly** the page's two windows `$4800-$4FFF`
    /// and `$6800-$6FFF` and nothing else: A14, A12 and A11 are decoded and A13 is not, so
    /// `$4800` and `$6800` both match while `$5800` and `$7800` — A11 set, A12 clear — do not.
    const BANK_MASK: u16 = 0xD800;
    const BANK_WINDOW: u16 = 0x4800;

    /// `$5000-$57FF` and `$5800-$5FFF`, each a 2 KiB window of its own under mask `$F800`.
    const PROTECTION_MASK: u16 = 0xF800;
    const INDEX_WINDOW: u16 = 0x5000;
    const DATA_WINDOW: u16 = 0x5800;

    /// "The initial value of this register is $11" — 32 KiB page **3**, vertical mirroring.
    const BANK_POWER_ON: u8 = 0x11;

    /// The page's own 256-byte table, `static uint8 lut[256]`, a write to `$5800-$5FFF` being
    /// XORed against the entry the last `$5000-$57FF` write indexed.
    ///
    /// Sixteen rows of sixteen, in the page's order. **The transcription check is the table's own
    /// regularity**: it is four groups of four rows, every group has one layout for its even rows
    /// and another for its odd ones, and within a row the eight bytes repeat after four. The
    /// sixteen first bytes are `59 59 59 59 00 08 00 08 59 59 59 59 01 09 01 09`, which is that
    /// structure read down the left edge, and
    /// `req_core_102_mapper_208_xors_a_write_against_the_pages_own_table` walks the whole array
    /// against it rather than trusting the four corners.
    const PROTECTION_TABLE: [u8; 256] = [
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19,
        0x09, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11,
        0x01, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19,
        0x09, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11,
        0x01, //
        0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18,
        0x08, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10,
        0x00, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18,
        0x08, //
        0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10,
        0x00, //
        0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, //
        0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let header = memory.rom().header();
        let is_stripped = header.is_nes20 && header.submapper == 1;
        let mut board = Self {
            // 208 and not 4, the tightening every MMC3 clone here makes: `Mmc3Board::new` reads
            // a submapper only for mapper 4, so this number's own submapper 1 cannot be taken
            // for an MMC6, an MMC3A or a hard-wired-mirroring MMC3C.
            mmc3: Mmc3::new(memory, mapper),
            bank: Self::BANK_POWER_ON,
            index: 0,
            protection: [0; 4],
            is_stripped,
        };

        // The page gives the board's own mirroring register and no `$A000` section; see the
        // type's note and **Q-171**. Submapper 1 keeps the chip's.
        if !is_stripped {
            board.mmc3.force_ignore_a000();
        }

        board.refresh();
        board
    }

    /// The 32 KiB PRG page this board shows at `$8000-$FFFF`.
    ///
    /// Submapper 0: the register's bits 4 and 0, bit 4 above bit 0, which is the diagram's
    /// `..MP ...P` with one connector joining the two. Submapper 1: "its value is determined by
    /// MMC3 bank register 6; the actual bank number is the bank register value SHR 2", so the
    /// chip's `R6` — an 8 KiB bank number — read as a 32 KiB one.
    fn page(&self) -> i32 {
        if self.is_stripped {
            i32::from(self.mmc3.bank_register(6) >> 2)
        } else {
            i32::from(((self.bank >> 3) & 0x02) | (self.bank & 0x01))
        }
    }

    /// Recomputes the MMC3's windows from its own registers and lays the 32 KiB page over the
    /// whole of `$8000-$FFFF`.
    ///
    /// The chip's PRG bank lines are not connected — the board banks 32 KiB at a time, so `R6`
    /// and `R7` reach nothing on submapper 0 and `R6` reaches *this* on submapper 1. Its CHR
    /// banking and its counter are untouched, which is what the page's "MMC3-compatible registers
    /// for CHR ROM bankswitching and IRQ control" means. Called after every write the board
    /// takes, on [`super::Somari::refresh`]'s own idempotence argument.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        let page = self.page();
        if let Some(banks) = self.mmc3.banks_mut() {
            banks.map_prg_32k(page);
        }
    }

    /// A write to one of the four ports below `$8000`. `true` when this board decoded it.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and every board of this family
    /// carries: this is one arm of a `Board::write_register` dispatch `NesBus::write_internal`
    /// reaches.
    #[inline(never)]
    fn write_low(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        // "There are no additional registers" on submapper 1 — the `$5800` leftovers the page
        // mentions included.
        if self.is_stripped {
            return false;
        }

        if address & Self::BANK_MASK == Self::BANK_WINDOW {
            self.bank = value;
            // "Select mirroring: 0: Vertical, 1: Horizontal."
            memory.set_mirroring(if value & 0x20 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            });
            self.refresh();
            return true;
        }

        match address & Self::PROTECTION_MASK {
            Self::INDEX_WINDOW => self.index = value,
            // "The value that is written to the protection register is the CPU data XORed with
            // the table entry indexed by the last write to $5000-$57FF."
            Self::DATA_WINDOW => {
                self.protection[usize::from(address & 0x03)] =
                    value ^ Self::PROTECTION_TABLE[usize::from(self.index)];
            }
            _ => return false,
        }

        true
    }
}

impl Mapper for Mmc3Gouder208 {
    fn mapper_number(&self) -> i32 {
        208
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's register file behind it. Nothing of the
    /// MMC3's own layout moves and no earlier state can name this number (REQ-CORE-090).
    ///
    /// `is_stripped` is the image's submapper, settled at load — wiring and not state, the reason
    /// `Mmc3Board::dots_per_clock` gives for itself.
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.bank);
        writer.write_u8(self.index);
        writer.write_raw(&self.protection);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        self.bank = reader.read_u8()?;
        self.index = reader.read_u8()?;
        reader.read_into(&mut self.protection)?;
        self.refresh();
        Ok(())
    }

    /// Back to the page's own initial value, `$11`, with the mirroring it implies.
    ///
    /// `Cartridge::reset` puts the header's mirroring back just before this runs, so the board
    /// has to state its own — and it is this call, and not [`Mmc3Gouder208::new`], that makes the
    /// board's power-on mirroring vertical, because `Cartridge::create` resets the cartridge it
    /// has just built.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.bank = Self::BANK_POWER_ON;
        self.index = 0;
        self.protection = [0; 4];
        if !self.is_stripped {
            memory.set_mirroring(NametableMirroring::Vertical);
        }
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

    /// The protection read at `$5800-$5FFF`: "Returns the content of one of the four protection
    /// registers determined by the two lower bits of the address being read."
    ///
    /// Submapper 1 answers nothing here — it has no such register, and the leftover code the page
    /// says the image still contains must read open bus rather than this board's state.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if !self.is_stripped && address & Self::PROTECTION_MASK == Self::DATA_WINDOW {
            return Some(self.protection[usize::from(address & 0x03)]);
        }

        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    /// **The `address < 0x8000` guard is deliberately not the first line**, which is
    /// [`super::Mmc3Txc189`]'s own note: two of the four ports are at `$4800` and `$5000`, a range
    /// `Cartridge::cpu_write` reaches, so a board that returned early below `$8000` could never be
    /// banked at all.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            self.write_low(memory, address, value);
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
