use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which board a [`Jv001`] is: the one chip's six wirings (MF-UNL-1 step 1).
///
/// The variants are the five Mesen 2 classes for the same silicon — `Txc22000` (36),
/// `Txc22211A` (132), `Sachen_136`, `Sachen_147`, `Txc22211B` (172) and `Txc22211C` (173) —
/// collapsed onto one board, which is what §6.1 of `docs/review/UNL-census.md` means by "the
/// same custom IC on six boards from four publishers".
///
/// **A runtime field and not a `const` parameter**, for the reason [`super::Namcot108`]'s own
/// note gives for its four: every difference below is read by [`Jv001::update_banks`] and by the
/// $4100 decode, i.e. only on a CPU access to $4100-$4103, $4200 or $8000-$FFFF — "the rarest
/// bus event there is". Six monomorphisations would have cost six jump-table slots in every
/// `Board` dispatch, which §4.2 of `docs/review/MF-6-inventory.md` prices per slot; one variant
/// with one field costs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jv001Board {
    /// Mapper 36, TXC's PCB 01-22000-400: the chip on CPU D4-D5, with the 8 KiB CHR bank in a
    /// latch of the board's own at $4200 rather than in the chip's Output.
    Txc36,
    /// Mapper 132, the three TXC PCBs of that number: the chip on CPU D0-D3, Output bit 2 the
    /// 32 KiB PRG bank and bits 1-0 the 8 KiB CHR bank.
    Txc132,
    /// Mapper 136, the Sachen 3011: the chip on CPU D0-D5, Output bits 2-0 the 8 KiB CHR bank
    /// and bit 4 the 32 KiB PRG bank.
    Sachen136,
    /// Mapper 147, the Sachen 3018's 60-pin release: the chip on CPU **D2-D7**, so every value
    /// is the byte rotated by two.
    Sachen147,
    /// Mapper 172, the Super Mega P-4070: the chip on CPU D0-D5 with the **bit order
    /// reversed**, and the one number of the six whose $8000 write drives the mirroring.
    Txc172,
    /// Mapper 173, Idea-Tek's board: the chip on CPU D0-D3, no PRG banking at all, and CHR A14
    /// driven from the **inverse of Invert** rather than from Output.
    Txc173,
}

/// iNES mappers **36, 132, 136, 147, 172 and 173**: the TXC 05-00002-010 / Sachen JV001
/// protection ASIC, one chip on six boards from four publishers (REQ-CORE-102, MF-UNL-1 step 1).
///
/// # One chip, described twice
///
/// nesdev describes the same silicon in two vocabularies. 36's, 132's and 173's pages give it
/// six registers — "'P', 'R', Output (3 bits each), 'S', inCrement and inVert (1 bit each)" —
/// where 136's, 147's and 172's give it five — "Input (6 bits), Output (6 bits), Register
/// (6 bits), Mode (1 bit) and Invert (1 bit)". The two accounts are one chip: `S` is the bit
/// above `R`, so the first vocabulary's `S`+`RRR` is the second's four-bit incrementable
/// `Register`, and `P`+`S` is its `Input`. This build therefore has **one** board and one
/// [`Jv001::accumulator_mask`], `$07` where a page says three bits and `$0F` where one says
/// four. Mesen 2 makes the same reduction with its `TxcChip(bool isJv001)`, whose `_mask` is the
/// same two values.
///
/// The registers, in this build's names:
///
/// | this build | the three-bit pages | the six-bit pages |
/// | --- | --- | --- |
/// | `accumulator` | `S` and `R` | `Register` |
/// | `staging` | `P` | `Input`, low half |
/// | `inverter` | — (the undriven bits) | `Input`, high half |
/// | `output` | Output | Output |
/// | `invert` | inVert | Invert |
/// | `increment` | inCrement | Mode |
///
/// # The register window, and the two things a write does
///
/// **Mask $E103**, so the four ports are $4100-$4103 and their mirrors every four bytes up to
/// $5FFF; $6000-$7FFF cannot reach them (`$6100 & $E103` is `$6100`), which is why this board
/// needs no [`Mapper::intercept_cpu_write`] body and why `registry::board_memory_sizes` leaves
/// the $6000-$7FFF window to the header.
///
/// ```text
/// write $4100 (mask $E103): Increment set -> accumulator + 1
///                           Increment clear, Invert clear -> accumulator := staging
///                           Increment clear, Invert set   -> accumulator := ~staging
/// write $4101:              [.... ...V] Invert
/// write $4102:              [..PP PPPP] staging (and the bits above the mask into inverter)
/// write $4103:              [.... ...C] Increment
/// write $8000-$FFFF:        Output := accumulator, with the inverter's bits above it
/// read  $4100 (mask $E100): the accumulator, with the inverter's bits above it inverted
///                           while Invert is set
/// ```
///
/// A **$8000-$FFFF write latches Output and nothing else** — the value written is not used at
/// all, which is what makes the board a CNROM-alike to a game that has already loaded the bank
/// number into the chip. The same write drives the mirroring on mapper 172 and only there.
///
/// # The protection, and the width each board checks
///
/// 132's page: "Games will also check the lower three or four bits of $4100 for the correct
/// value after several increment and inversion operations as a copy-protection measure"; 136's
/// says "the lower four or six bits" and 172's "the lower six bits". The *width* is a property
/// of the wiring, not of the chip: the chip always answers its four or three accumulator bits
/// with the inverter's bits above them, and the board decides which CPU data pins those reach.
/// [`Jv001::cpu_to_chip`] and [`Jv001::chip_to_cpu`] are that wiring, one pair per board:
///
/// | # | CPU pins | `cpu_to_chip` | `chip_to_cpu` |
/// | ---: | --- | --- | --- |
/// | 36 | D4-D5 | `(v >> 4) & 3` | `(x << 4) & $30` |
/// | 132, 173 | D0-D3 | `v & $0F` | `x & $0F` |
/// | 136 | D0-D5 | `v & $3F` | `x & $3F` |
/// | 147 | D2-D7 | `v` rotated right 2 | `x` rotated left 2 |
/// | 172 | D0-D5, reversed | D0↔D5, D1↔D4, D2↔D3 | the same involution |
///
/// The bits no board drives read as **0** and not as open bus: [`Mapper::try_read_prg`] returns
/// a whole byte and cannot see the CPU data bus. `Multicart225::cpu_peek_override` already makes
/// that compromise for the same reason, and no page asks a game to read past the bits its board
/// drives.
///
/// # Why `try_read_prg` and not `cpu_peek_override`
///
/// [`super::SmallBoard::try_read_prg`]'s own note is the whole argument, and it applies
/// unchanged: a board whose register is below $8000 answers from this hook so that the arm
/// `Board::try_read_prg`'s `#[inline(always)]` folds into `Cartridge::default_cpu_read` is one
/// `callq` to [`super::DecodeBoard::try_read_prg`], which carries `#[inline(never)]`. What makes
/// it possible here is that **the read has no side effect**: Mesen recomputes a `_yFlag` on
/// every chip read, and this build does not have that flag at all — see the note on
/// [`Jv001::chr_bank`].
///
/// # Mapper 173's CHR, and the one image that disables it
///
/// 173's page gives the bank two ways. The wiring: "copy bottom two bits of internal register
/// 'RRR' to CHR A15 and CHR A13 banking pins", with $4101's "value … immediately inverted and
/// used as CHR A14". And a formula, `8 KiB CHR-ROM bank number := (Output &1) | (~Invert <<1)`,
/// which is the same thing with CHR A15 left out. This build takes the wiring, all three bits of
/// it, which is also Mesen's reading.
///
/// The page then names an image where the third bit cannot exist: *小瑪琍* "uses the Mapper 173
/// circuit board but replaces the 32 KiB CHR-ROM chip with a SGS M2764A 8 KiB UVEPROM. Its pin
/// 27 function is /PGM instead of the normal A14. Setting Invert on this board just disables
/// CHR-ROM." So on an image with 8 KiB of CHR-ROM or less, Invert set means **no CHR at all**:
/// [`Jv001::chr_disabled`] answers the pattern fetches with 0 and [`Mapper::chr_peek_view`] with
/// `None`, which is as close to a floating PPU bus as this core gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jv001 {
    banks: BankWindows,
    /// `S`+`R`, the chip's incrementable register, masked to [`Jv001::accumulator_mask`] where
    /// it is read and latched.
    accumulator: u8,
    /// `P`, the value a $4100 write copies (or copies inverted) into the accumulator.
    staging: u8,
    /// The bits of a $4102 write **above** the accumulator mask. They are not a register of
    /// either page's account: they are the part of the Input latch the accumulator does not
    /// reach, and the chip answers a read and latches an Output with them in place. Mesen's
    /// `_inverter`, and the mechanism mapper 147's PRG bit 5 and mapper 136's PRG bit 4 are
    /// driven through.
    inverter: u8,
    /// Output, latched from the accumulator by a $8000-$FFFF write.
    output: u8,
    /// inVert / Invert. **Set at power-on on the three numbers whose page calls the register
    /// six bits wide** (136, 147, 172) and clear on the other three, which is Mesen's
    /// `_invert = isJv001` and the only power-on state either account gives.
    invert: bool,
    /// inCrement / Mode.
    increment: bool,
    /// Mapper 36's own 8 KiB CHR latch at $4200, outside the ASIC: `[.... CCCC]`, mask $E200.
    chr_latch: u8,
    /// 36, 132, 136, 147, 172 or 173, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// Which of the six boards this is.
    board: Jv001Board,
    /// Mapper 173 on an image with no CHR-ROM above 8 KiB: the whole CHR-ROM is disabled while
    /// Invert is set. Derived from [`Jv001::invert`] by [`Jv001::update_banks`]; no savestate
    /// byte carries it (REQ-CORE-091).
    chr_disabled: bool,
    /// Whether the image has 8 KiB of CHR-ROM or less, which is what makes mapper 173 the
    /// *小瑪琍* board. Settled at load from the ROM and not a register.
    small_chr: bool,
}

impl Jv001 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let board = match mapper_number {
            36 => Jv001Board::Txc36,
            132 => Jv001Board::Txc132,
            136 => Jv001Board::Sachen136,
            147 => Jv001Board::Sachen147,
            172 => Jv001Board::Txc172,
            // 173, and the arm the registry never reaches with anything else.
            _ => Jv001Board::Txc173,
        };
        let mut jv001 = Self {
            banks: windows_for(memory),
            accumulator: 0,
            staging: 0,
            inverter: 0,
            output: 0,
            invert: Self::power_on_invert(board),
            increment: false,
            chr_latch: 0,
            mapper_number,
            board,
            chr_disabled: false,
            // An image with no CHR-ROM at all is a CHR-RAM one and not 小瑪琍's UVEPROM, so it
            // is deliberately not small: the disable is a /CE line on a ROM chip this board
            // would not have.
            small_chr: {
                let chr_rom = memory.rom().chr_rom().len();
                chr_rom != 0 && chr_rom <= 0x2000
            },
        };
        jv001.update_banks();
        jv001
    }

    /// `$0F` on the three numbers whose page calls the register six bits wide and `$07` on the
    /// three that call it three, which is the one difference between the two accounts of the
    /// chip (see the struct's own note).
    const fn accumulator_mask(board: Jv001Board) -> u8 {
        match board {
            Jv001Board::Sachen136 | Jv001Board::Sachen147 | Jv001Board::Txc172 => 0x0F,
            Jv001Board::Txc36 | Jv001Board::Txc132 | Jv001Board::Txc173 => 0x07,
        }
    }

    /// Invert at power-on: set on the same three numbers, clear on the other three. Mesen's
    /// `TxcChip::TxcChip`, `_invert = isJv001`.
    const fn power_on_invert(board: Jv001Board) -> bool {
        matches!(
            board,
            Jv001Board::Sachen136 | Jv001Board::Sachen147 | Jv001Board::Txc172
        )
    }

    /// Mapper 172's reversed data bus: D0↔D5, D1↔D4, D2↔D3. An involution, so the same function
    /// serves both directions (nesdev, "INES Mapper 172": "the bit order D0-D5 is *reversed*").
    const fn reverse6(value: u8) -> u8 {
        ((value & 0x01) << 5)
            | ((value & 0x02) << 3)
            | ((value & 0x04) << 1)
            | ((value & 0x08) >> 1)
            | ((value & 0x10) >> 3)
            | ((value & 0x20) >> 5)
    }

    /// What the chip sees when the CPU writes `value`: the board's wiring of the data bus onto
    /// the chip's six data pins. The struct's own note has the table.
    const fn cpu_to_chip(&self, value: u8) -> u8 {
        match self.board {
            // "write $4102: [..PP ....]" and the three one-bit ports on D4 — the chip's pins
            // are CPU D4-D5 (nesdev, "INES Mapper 036").
            Jv001Board::Txc36 => (value >> 4) & 0x03,
            Jv001Board::Txc132 | Jv001Board::Txc173 => value & 0x0F,
            Jv001Board::Sachen136 => value & 0x3F,
            // "$4102: Set Input from bits 2-7", "$4101: Set Invert flag from bit 2", "$4103: Set
            // Mode from bit 2" (nesdev, "INES Mapper 147"): chip D0 is CPU D2, so the whole byte
            // is rotated right by two.
            Jv001Board::Sachen147 => value.rotate_right(2),
            Jv001Board::Txc172 => Self::reverse6(value),
        }
    }

    /// The counterpart on a read: the chip's answer `value` back onto the CPU data bus, with
    /// every pin the board does not drive left at 0 (see the struct's own note).
    const fn chip_to_cpu(&self, value: u8) -> u8 {
        match self.board {
            Jv001Board::Txc36 => (value << 4) & 0x30,
            Jv001Board::Txc132 | Jv001Board::Txc173 => value & 0x0F,
            Jv001Board::Sachen136 => value & 0x3F,
            // "Returns Register bits 0-5 on CPU data bus bits 2-7" (nesdev, "INES Mapper 147").
            Jv001Board::Sachen147 => value.rotate_left(2),
            Jv001Board::Txc172 => Self::reverse6(value),
        }
    }

    /// What the chip drives on a read of $4100: the accumulator, with the bits the accumulator
    /// does not reach taken from the inverter and complemented while Invert is set.
    ///
    /// 173's page gives the shape — `[xxxx SRRR]`, "S XOR V and RRR copied to bus" — and 136's,
    /// 147's and 172's the same thing as "Bits 4-5 are inverted if Invert==1", the two sentences
    /// differing only in where the mask falls. Mesen's `TxcChip::Read` is this line.
    const fn chip_read(&self) -> u8 {
        let mask = Self::accumulator_mask(self.board);
        let high = if self.invert {
            !self.inverter
        } else {
            self.inverter
        };
        (self.accumulator & mask) | (high & !mask)
    }

    /// The 8 KiB CHR bank each board selects.
    ///
    /// **Mapper 173 reads `invert` here and not a latched flag**, which is this build's one
    /// deliberate divergence from Mesen on these six numbers. Mesen keeps a `_yFlag` set to
    /// `!_invert || (value & $10)` on every chip read *and* write; on 173 the data bus is four
    /// bits wide, so bit 4 of a written value is always clear and the flag is `!_invert` after
    /// every write — while after a *read* it is unconditionally true, bit 4 of the answer being
    /// the complement the inverter half supplies. The page has no such flag and no such
    /// asymmetry: "This value is immediately inverted and used as CHR A14". Reading `invert`
    /// where the bank is computed is that sentence, and it is what leaves the protection read
    /// with no side effect at all.
    // Mappers 132 and 172 answer `Output & 3` for two different reasons, each with its own page
    // quotation on the arm; merging them would lose one of the two.
    #[allow(clippy::match_same_arms)]
    fn chr_bank(&self) -> i32 {
        let output = i32::from(self.output);
        match self.board {
            // "write $4200: [.... CCCC] - Select 8 KiB CHR bank" — the board's own latch, not the
            // chip (nesdev, "INES Mapper 036").
            Jv001Board::Txc36 => i32::from(self.chr_latch & 0x0F),
            // "Copy R register to PRG A15, CHR A14, CHR A13 banking pins sequentially" (nesdev,
            // "INES Mapper 132").
            Jv001Board::Txc132 => output & 0x03,
            // "CHR ROM bank = (Output & 7)" (nesdev, "INES Mapper 136").
            Jv001Board::Sachen136 => output & 0x07,
            // Register bits 1-4 are "CHR-ROM bank select (A13-A16)" (nesdev, "INES Mapper 147").
            Jv001Board::Sachen147 => (output & 0x1E) >> 1,
            // "8 KiB CHR-ROM bank number := (Output &3)" (nesdev, "INES Mapper 172"). Mesen
            // passes the whole Output and lets its page table wrap, which is the same answer on
            // the board's 32 KiB CHR; the page masks, so this does.
            Jv001Board::Txc172 => output & 0x03,
            // CHR A13 from Output bit 0, CHR A14 from the inverse of Invert, CHR A15 from Output
            // bit 1 (nesdev, "INES Mapper 173").
            Jv001Board::Txc173 => {
                (output & 0x01) | i32::from(!self.invert) << 1 | ((output & 0x02) << 1)
            }
        }
    }

    /// The 32 KiB PRG bank each board selects. Two of the six have no PRG bank line at all.
    fn prg_bank(&self) -> i32 {
        let output = i32::from(self.output);
        match self.board {
            // "write $8000: copy internal register 'RR' to PRG banking pins" (nesdev, "INES
            // Mapper 036").
            Jv001Board::Txc36 => output & 0x03,
            // Output bit 2 is PRG A15; see the CHR arm's quotation.
            Jv001Board::Txc132 => (output >> 2) & 0x01,
            // "PRG ROM bank = (Output >> 4) & 1" (nesdev, "INES Mapper 136"), which the page
            // attaches to *Mahjong Companion* alone. **Mesen's `Sachen_136` does not implement
            // this line**; the page does, so this build does, and on the 32 KiB images of the
            // number's other three titles the one bank makes it a no-op.
            Jv001Board::Sachen136 => (output >> 4) & 0x01,
            // Register bit 5 is PRG A16 and bit 0 PRG A15 (nesdev, "INES Mapper 147", the
            // Output diagram).
            Jv001Board::Sachen147 => ((output & 0x20) >> 4) | (output & 0x01),
            // 172's page gives the board 32 KiB of PRG-ROM and no PRG bank line; 173's page
            // gives the Idea-Tek board the same. Bank 0 is the whole of it.
            Jv001Board::Txc172 | Jv001Board::Txc173 => 0,
        }
    }

    fn update_banks(&mut self) {
        // "Setting Invert on this board just disables CHR-ROM" — mapper 173 on an image whose
        // CHR-ROM is the 8 KiB UVEPROM of 小瑪琍 (nesdev, "INES Mapper 173").
        self.chr_disabled =
            matches!(self.board, Jv001Board::Txc173) && self.small_chr && self.invert;
        self.banks.map_prg_32k(self.prg_bank());
        self.banks.map_chr_8k(self.chr_bank());
    }

    /// The $8000-$FFFF half of the decode: Output takes the accumulator, and on mapper 172 the
    /// mirroring follows Invert.
    ///
    /// The high bits come from the inverter on the four-bit boards and from **bit 3 of it moved
    /// up one** on the three-bit ones, which is Mesen's `TxcChip::Write` else-branch; on those
    /// three the inverter cannot hold anything above bit 3 anyway (`cpu_to_chip` masks to four bits
    /// or fewer), so the move is what puts mapper 147's PRG A16 and mapper 136's PRG A15 within
    /// reach on the boards that have them and is inert on the rest.
    fn latch_output(&mut self, memory: &mut CartridgeMemory) {
        self.output = if Self::accumulator_mask(self.board) == 0x0F {
            (self.accumulator & 0x0F) | (self.inverter & 0xF0)
        } else {
            (self.accumulator & 0x0F) | ((self.inverter & 0x08) << 1)
        };

        // "$8000-$FFFF: Output ← Register; nametable mirroring changes (Horizontal if
        // Invert==0, Vertical if Invert==1)" — mapper 172 and no other number of the six
        // (nesdev, "INES Mapper 172"). 136's, 147's and 173's boards hardwire it and 132's and
        // 36's pages give it no register.
        if matches!(self.board, Jv001Board::Txc172) {
            memory.set_mirroring(if self.invert {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            });
        }

        self.update_banks();
    }
}

impl Mapper for Jv001 {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.accumulator);
        writer.write_u8(self.staging);
        writer.write_u8(self.inverter);
        writer.write_u8(self.output);
        writer.write_bool(self.invert);
        writer.write_bool(self.increment);
        writer.write_u8(self.chr_latch);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.accumulator = reader.read_u8()?;
        self.staging = reader.read_u8()?;
        self.inverter = reader.read_u8()?;
        self.output = reader.read_u8()?;
        self.invert = reader.read_bool()?;
        self.increment = reader.read_bool()?;
        self.chr_latch = reader.read_u8()?;
        // `chr_disabled` is derived, not stored (REQ-CORE-091).
        self.chr_disabled =
            matches!(self.board, Jv001Board::Txc173) && self.small_chr && self.invert;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.accumulator = 0;
        self.staging = 0;
        self.inverter = 0;
        self.output = 0;
        self.invert = Self::power_on_invert(self.board);
        self.increment = false;
        self.chr_latch = 0;
        // The mirroring a power-on 172 shows is the one its Invert says, which `latch_output`
        // is where this board says; every other number leaves the header's.
        if matches!(self.board, Jv001Board::Txc172) {
            memory.set_mirroring(NametableMirroring::Vertical);
        }
        self.update_banks();
    }

    /// PRG-ROM above $8000, and the protection read at $4100 below it.
    ///
    /// **Mask $E100 and not $E103**: 36's, 132's and 173's pages all give the read its own,
    /// wider mask, so $4100-$41FF answers and not only $4100. Mesen reads `(addr & $103) ==
    /// $100` within a $4020-$5FFF window, which is the same set of addresses.
    ///
    /// See the struct's own note on why this hook and not [`Mapper::cpu_peek_override`], and on
    /// why the bits the board does not drive read as 0.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if address & 0xE100 == 0x4100 {
            return Some(self.chip_to_cpu(self.chip_read()));
        }

        None
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            // The value written is not used: the write only clocks Output out of the
            // accumulator, which is what lets a game load a bank number into the chip and then
            // select it with a CNROM-style store.
            self.latch_output(memory);
            return;
        }

        // Mapper 36's CHR latch, outside the ASIC: "Mask: $E200, write $4200: [.... CCCC]"
        // (nesdev, "INES Mapper 036"). Mesen decodes it as `(addr & $F200) == $4200`, one bit
        // narrower; the page's mask is used here. The two decodes overlap the chip's own window
        // — $4300 is both — and on the hardware both latches see the write, so neither arm is
        // an `else`.
        if matches!(self.board, Jv001Board::Txc36) && address & 0xE200 == 0x4200 {
            self.chr_latch = value & 0x0F;
        }

        let value = self.cpu_to_chip(value);
        match address & 0xE103 {
            0x4100 => {
                if self.increment {
                    self.accumulator = self.accumulator.wrapping_add(1);
                } else {
                    let mask = Self::accumulator_mask(self.board);
                    let loaded = (self.accumulator & !mask) | (self.staging & mask);
                    self.accumulator = if self.invert { !loaded } else { loaded };
                }
            }
            0x4101 => self.invert = value & 0x01 != 0,
            0x4102 => {
                let mask = Self::accumulator_mask(self.board);
                self.staging = value & mask;
                self.inverter = value & !mask;
            }
            0x4103 => self.increment = value & 0x01 != 0,
            // $4000-$40FF, $4104-$41FF and everything else below $8000 that is not one of the
            // four ports: no register, and on mapper 36 possibly the CHR latch above.
            _ => {}
        }

        // Mapper 36's CHR latch and mapper 173's CHR A14 both move below $8000, so the windows
        // are recomputed here and not only at the Output latch. Idempotent on the other four.
        self.update_banks();
    }

    /// 0 while mapper 173's CHR-ROM is disabled, and the banked read otherwise.
    ///
    /// The hardware leaves the PPU data bus floating when the UVEPROM's /CE is not driven; 0 is
    /// what this core answers for an absent CHR memory everywhere else ([`ChrView::read`]), so
    /// it is what the disable answers too.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.chr_disabled {
            return 0;
        }

        self.banks.read_chr(address, chr_of(memory))
    }

    /// `None` while mapper 173's CHR-ROM is disabled, so that `Ppu2C02::read_row_planes` falls
    /// back on [`Jv001::ppu_peek`] and captures the blank the board is showing; the banked view
    /// otherwise, which is what [`super::DecodeBoard::chr_peek_view`]'s own note wants of a
    /// plain CHR-ROM board.
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.chr_disabled {
            return None;
        }

        Some(ChrView::new(chr_of(memory), &self.banks))
    }
}

/// The CHR memory the pattern fetches read: the CHR-ROM, or the CHR-RAM on an image that has
/// none. The same choice [`Mapper::ppu_peek`]'s own default makes, written once because this
/// board overrides both the peek and the view.
fn chr_of(memory: &CartridgeMemory) -> &[u8] {
    let chr_rom = memory.rom().chr_rom();
    if chr_rom.is_empty() {
        memory.chr_ram()
    } else {
        chr_rom
    }
}
