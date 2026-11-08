use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which of the four wirings of the Sachen 8259 silicon a [`Sachen8259`] is (nesdev, "Sachen
/// 8259", the *CHR Rewiring* table).
///
/// **A runtime field and not a `const` parameter**, for the reason [`super::Namcot108`]'s and
/// [`super::SingleLatch`]'s notes both give: every difference between the four is read only by
/// [`Sachen8259::update_banks`], i.e. only on a CPU write to the index/data pair — "the rarest bus
/// event there is". Four monomorphisations would have bought four arms in every inner `match` of
/// [`super::DecodeBoard`]; one field with four values buys one.
///
/// The letters are the chip's own and the mapper numbers are **not** in letter order, which is why
/// both appear on every variant below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sachen8259Wiring {
    /// The 8259**A**, iNES mapper **141**: "Passes PPU A[11:0] through to CHR ROM A[11:0]" and
    /// "6 bits of chr reg to CHR ROM A[17:12]", so up to **256 KiB** of CHR.
    A,
    /// The 8259**B**, iNES mapper **138**: "Passes PPU A[10:0] through to CHR ROM A[10:0]" and
    /// "6 bits of chr reg to CHR ROM A[16:11]" — the one variant whose bank unit *is* its window,
    /// and up to **128 KiB** of CHR.
    B,
    /// The 8259**C**, iNES mapper **139**: "Passes PPU A[12:0] through to CHR ROM A[12:0]" and
    /// "6 bits of chr reg to CHR ROM A[18:13]", so the four windows are one contiguous 8 KiB bank
    /// and the chip reaches **512 KiB**.
    C,
    /// The 8259**D**, iNES mapper **137**, the one with a page of its own: four **1 KiB** windows
    /// over $0000-$0FFF and "PPU $1000-$1FFF: fixed to last 4KiB of CHR ROM".
    D,
}

impl Sachen8259Wiring {
    /// The wiring of `mapper_number`, from the *Variants* table of nesdev's "Sachen 8259".
    const fn of(mapper_number: i32) -> Self {
        match mapper_number {
            138 => Self::B,
            139 => Self::C,
            137 => Self::D,
            // 141, and the only number left.
            _ => Self::A,
        }
    }

    /// How far the 2 KiB window's bank number is shifted above the six bits of the CHR register,
    /// the low bits coming from the PPU address lines the chip passes through.
    ///
    /// The *CHR Rewiring* table says it as a wiring, and this is the same statement as an
    /// arithmetic one. Take the 8259A: the chip "passes PPU A[11:0]" and drives "chr reg to CHR ROM
    /// A[17:12]", so the CHR address of a fetch is `{chr_reg[5:0], A11..A0}` — a **4 KiB** unit —
    /// while the window that fetch falls in is 2 KiB and is chosen by "PPU A[12:11]". A11 is
    /// therefore both the window's low selector *and* an address line of the bank, which in 2 KiB
    /// units is `(chr_reg << 1) | (window & 1)`. The 8259C shifts by two and the 8259B not at all.
    ///
    /// The capacities on the page are the arithmetic's own check: six bits of register times the
    /// unit is 128 KiB on the B, 256 KiB on the A and 512 KiB on the C, which is exactly what the
    /// three rows say.
    /// The 8259D never reaches here — [`Sachen8259::update_banks`] takes its own branch first —
    /// so it shares the B's zero rather than naming a shift it has not got.
    const fn chr_shift(self) -> u32 {
        match self {
            Self::A => 1,
            Self::C => 2,
            Self::B | Self::D => 0,
        }
    }
}

/// iNES mappers **137, 138, 139 and 141**: the Sachen **8259**, one chip with four wirings
/// (REQ-CORE-102, MF-UNL-7 step 1).
///
/// # The caveat this board carries, in the page's own words
///
/// > The following information was gathered primarily from FCEU-mm and while it appears to be a
/// > good enough behavioral description to run many ROMs, it is likely incomplete.
///
/// And it governs all four numbers, because the four are one page plus mapper 137's. So this board
/// is the page's own *CHR Rewiring* and register tables and **nothing inferred past them**; where
/// mapper 137's page contradicts the family page the number's own page wins (Q-160), which is
/// MF-UNL-2's rule.
///
/// **The attribution gap is recorded and not guessed.** Neither NESdev nor Wikipedia attributes
/// the 8259 titles to 137/138/139/141 individually beyond two sentences, and those two are all
/// this build claims: mapper 137's page gives *The Great Wall* as the "only one known 8259D game"
/// with "32KiB PRG ROM and 32KiB CHR ROM", and the family page gives *Q-Boy*, "using a PCB
/// labelled **2M-RAM-COB**", as the CHR-RAM board "usually set to mapper 141".
///
/// # The two ports, which are [`super::Sachen8Reg`]'s
///
/// ```text
/// Register Select ($4100)      Register Data ($4101)
/// 7  bit  0                    7  bit  0
/// ---- ----                    ---- ----
/// .... .rrr                    .... .ddd
/// ```
///
/// "Registers should be masked with $C101" — A15, A14, A8 and A0 and nothing else — so the pair
/// repeats through the whole of $4100-$7FFF exactly as Sachen's eight-register ASIC does, and
/// `registry::board_memory_sizes` has to give all four numbers `prg_ram: Some(0)` for the same
/// reason: with no RAM behind $6000-$7FFF, `Cartridge::cpu_write` walks past its PRG-RAM branch
/// and [`Mapper::write_register`] sees the address intact.
///
/// **Unlike the eight-register ASIC there is no read port.** That page says "All three bits in all
/// eight registers are fully readable and writable"; this one says nothing of the kind, so the
/// data port is write-only here and the board leaves [`Mapper::try_read_prg`] to `Mapper`'s own
/// default — which is what keeps this board out of [`super::DecodeBoard::try_read_prg`]'s `cfg`
/// list.
///
/// # What each register drives
///
/// | register | the page's name | what it is |
/// | --- | --- | --- |
/// | R0-R3 | Chr Select | the lower 3 bits of a CHR block each, one register per block |
/// | R4 | Chr Outer Bank | "Select upper 3 bits of all four 2KiB CHR blocks" — and on the 8259D, one bit **per block** (see below) |
/// | R5 | Prg Bank | `.... .ppp`, "Select 32KiB PRG bank for CPU $8000" |
/// | R6 | — | nothing on the family page; **mapper 137's own page gives it `.... ...E`** |
/// | R7 | Mode and Mirroring Select | `.... .mms` |
///
/// All eight are stored, including R6 on the three numbers whose page does not mention it: the
/// index is three bits wide and a write has to land somewhere, and a board that dropped it would
/// differ from one that kept it only in a savestate this lot is writing for the first time.
///
/// # R7, and the one place the two pages disagree
///
/// `mms`: `mm` is the mirroring mode and `s` the "simple" mode. The four modes are
///
/// | `mm` | the family page (138, 139, 141) | mapper 137's page |
/// | ---: | --- | --- |
/// | 0 | `V` | `H` |
/// | 1 | `H` | `V` |
/// | 2 | `(0,1,1,1)` | `(0,1,1,1)` |
/// | 3 | `1scA` | `1scA` |
///
/// — **modes 0 and 1 are swapped between them**, and so is what `s` fixes the mirroring to ("V"
/// on the family page, "H" on 137's). Each number takes its own page, which is MF-UNL-2's rule,
/// and **Q-160** records the contradiction.
///
/// Mode 2, `(0,1,1,1)`, is not a [`NametableMirroring`]: one table on CIRAM page 0 and the other
/// three on page 1 is a per-table layout, so the board answers it through
/// [`Mapper::nametable_page`] — §4.5 of `docs/review/MF-6-inventory.md`'s cheap level, the
/// mechanism mappers 95, 118, 150, 158 and 207 already use, and **not** a `read_nametable` body
/// (Q-123's dispatch is untouched by this board).
///
/// # Simple mode
///
/// > When simple mode is enabled, mirroring is fixed to V, ignoring the m bits, and all 4 CHR
/// > banks use the CHR bank 0 register.
///
/// Both halves are implemented. Mapper 137's page says only "mirroring is fixed to _H_, and banks
/// become weird" — so on that number the mirroring follows its own page and the banks follow the
/// family page's concrete statement, there being no other; **Q-160** records that "weird" is all
/// the 8259D's page says.
///
/// # The CHR-RAM board
///
/// *Q-Boy*'s 2M-RAM-COB "replaces CHR-ROM with 8 KiB of unbanked CHR-RAM". An image with no
/// CHR-ROM is therefore a board with **no CHR bank lines at all**, and the CHR registers drive
/// nothing: `update_banks` leaves the 8 KiB window where `BankWindows::new` put it and a write to
/// R0-R4 or R6 changes no CHR read. The register bytes are still latched, because the index and
/// data ports are the chip's and not the CHR chip's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sachen8259 {
    banks: BankWindows,
    /// The eight three-bit registers, R0-R7.
    registers: [u8; 8],
    /// The register the $4100 port last selected.
    index: u8,
    /// 137, 138, 139 or 141, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// Which of the four wirings this is.
    wiring: Sachen8259Wiring,
    /// The 2M-RAM-COB board: no CHR-ROM on the image, so the CHR registers drive nothing.
    chr_ram_board: bool,
    /// What [`Mapper::nametable_page`] answers: `FOLLOW_MIRRORING` in three of R7's four modes and
    /// `[0, 1, 1, 1]` in the fourth. Derived state — `load_registers` recomputes it and no
    /// savestate byte carries it (REQ-CORE-091).
    nametable_pages: [i32; 4],
}

impl Sachen8259 {
    /// "Registers should be masked with $C101": A15, A14, A8 and A0.
    const PORT_MASK: u16 = 0xC101;
    const INDEX_PORT: u16 = 0x4100;
    const DATA_PORT: u16 = 0x4101;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            registers: [0; 8],
            index: 0,
            mapper_number,
            wiring: Sachen8259Wiring::of(mapper_number),
            chr_ram_board: memory.rom().chr_rom().is_empty(),
            nametable_pages: [Cartridge::FOLLOW_MIRRORING; 4],
        };
        board.refresh_nametable_pages();
        board.update_banks();
        board
    }

    /// `s`, bit 0 of R7.
    const fn simple_mode(&self) -> bool {
        self.registers[7] & 0x01 != 0
    }

    /// `mm`, bits 2-1 of R7.
    const fn mirroring_mode(&self) -> u8 {
        (self.registers[7] >> 1) & 0x03
    }

    /// The six bits of the CHR register of block `block` on the 8259A, B and C: "Select lower 3
    /// bits of 2KiB CHR block" from R0-R3 and "Select upper 3 bits of all four 2KiB CHR blocks"
    /// from R4.
    ///
    /// Simple mode makes every block read R0, which is the page's "all 4 CHR banks use the CHR
    /// bank 0 register". The outer bits are R4's either way: the sentence is about the *bank*
    /// registers.
    fn chr_register(&self, block: usize) -> i32 {
        let select = if self.simple_mode() { 0 } else { block };
        i32::from(self.registers[select] & 0x07) | (i32::from(self.registers[4] & 0x07) << 3)
    }

    /// The 1 KiB bank of window `window` on the **8259D**, whose R4 is one bit per block rather
    /// than three common upper bits (nesdev, "INES Mapper 137"):
    ///
    /// ```text
    /// ...0 0ccc   CHR register 0
    /// ...B 0ccc   CHR register 1
    /// ...C 0ccc   CHR register 2
    /// ...D Eccc   CHR register 3
    /// ```
    ///
    /// with R4 `.... .DCB` — `B` bit 4 of CHR register 1, `C` bit 4 of register 2, `D` bit 4 of
    /// register 3 — and R6 `.... ...E`, bit 3 of register 3 alone. So window 0 reaches eight
    /// banks, windows 1 and 2 sixteen with a hole in the middle, and **only window 3 reaches the
    /// whole 32 KiB** the page gives the chip, which is why its two extra bits exist.
    fn chr_bank_8259d(&self, window: usize) -> i32 {
        let low = i32::from(self.registers[if self.simple_mode() { 0 } else { window }] & 0x07);
        let outer = i32::from(self.registers[4] & 0x07);
        match window {
            0 => low,
            1 => low | ((outer & 0x01) << 4),
            2 => low | ((outer & 0x02) << 3),
            // `D` at bit 4 and `E`, R6's one bit, at bit 3.
            _ => low | ((outer & 0x04) << 2) | (i32::from(self.registers[6] & 0x01) << 3),
        }
    }

    fn update_banks(&mut self) {
        // "Select 32KiB PRG bank for CPU $8000", R5's three bits.
        self.banks.map_prg_32k(i32::from(self.registers[5] & 0x07));

        // The 2M-RAM-COB: "8 KiB of unbanked CHR-RAM", so there is nothing to bank.
        if self.chr_ram_board {
            return;
        }

        if matches!(self.wiring, Sachen8259Wiring::D) {
            // Four 1 KiB windows at $0000-$0FFF, and "PPU $1000-$1FFF: fixed to last 4KiB of CHR
            // ROM".
            for window in 0..4 {
                self.banks.map_chr_1k(window, self.chr_bank_8259d(window));
            }

            self.banks.map_chr_4k(4, -1);
            return;
        }

        // The 8259A, B and C: four 2 KiB blocks, chosen by "PPU A[12:11]", each at the bank the
        // wiring's shift puts the register at.
        let shift = self.wiring.chr_shift();
        let low_from_ppu = (1 << shift) - 1;
        for window in 0..4 {
            let bank = (self.chr_register(window) << shift)
                | (i32::try_from(window).unwrap_or(0) & low_from_ppu);
            self.banks.map_chr_2k(window * 2, bank);
        }
    }

    /// R7 mode 2 alone, `(0,1,1,1)`; every other mode leaves the four pages following
    /// [`Cartridge::mirroring`], which [`Sachen8259::update_mirroring`] has set.
    ///
    /// Simple mode "ignor[es] the m bits", so it leaves the pages following the mirroring too.
    fn refresh_nametable_pages(&mut self) {
        self.nametable_pages = if !self.simple_mode() && self.mirroring_mode() == 2 {
            [0, 1, 1, 1]
        } else {
            [Cartridge::FOLLOW_MIRRORING; 4]
        };
    }

    /// The two-screen layout `mm` mode **0** selects on this number, or mode 1's when `mode_zero`
    /// is false — the one place the two pages disagree (Q-160). The family page's mode 0 is `V`
    /// and mapper 137's is `H`, so the 8259D is the number whose pair is the other way round.
    const fn two_screen(&self, mode_zero: bool) -> NametableMirroring {
        let flipped = matches!(self.wiring, Sachen8259Wiring::D);
        if mode_zero == flipped {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        }
    }

    fn update_mirroring(&mut self, memory: &mut CartridgeMemory) {
        if self.simple_mode() {
            // "mirroring is fixed to V, ignoring the m bits" on the family page and to `H` on
            // mapper 137's — which is, on both pages, **that page's own mode 0**, so one call
            // says it for all four numbers.
            memory.set_mirroring(self.two_screen(true));
            return;
        }

        match self.mirroring_mode() {
            0 => memory.set_mirroring(self.two_screen(true)),
            1 => memory.set_mirroring(self.two_screen(false)),
            // `(0,1,1,1)` is the per-table layout above; the mirroring field is left as it is,
            // because `nametable_page` answers for all four tables and nothing consults it.
            2 => {}
            _ => memory.set_mirroring(NametableMirroring::SingleScreenA),
        }
    }
}

impl Mapper for Sachen8259 {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.registers);
        writer.write_u8(self.index);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.registers)?;
        self.index = reader.read_u8()?;
        // The pages follow from R7, so they are recomputed and not read back.
        self.refresh_nametable_pages();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        // The page gives the registers no power-on value, so they are zero: PRG bank 0, CHR block
        // 0 everywhere, simple mode off and mirroring mode 0.
        self.registers = [0; 8];
        self.index = 0;
        self.refresh_nametable_pages();
        self.update_mirroring(memory);
        self.update_banks();
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 0x03]
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & Self::PORT_MASK {
            Self::INDEX_PORT => self.index = value & 0x07,
            Self::DATA_PORT => {
                self.registers[usize::from(self.index & 0x07)] = value & 0x07;
                self.refresh_nametable_pages();
                self.update_mirroring(memory);
                self.update_banks();
            }
            // Everything else in $4020-$FFFF: the mask reads A15, A14, A8 and A0 only, so a
            // $8000-$FFFF write is `$8100`-shaped at most and never `$4100` or `$4101`.
            _ => {}
        }
    }
}
