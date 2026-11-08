use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 206, the Namco 108 / Namcot 109, 118 and 119 and the Tengen MIMIC-1 (DxROM,
/// DRROM, DE1ROM), with mappers 76, 88, 95 and 154 as its four parameters (REQ-CORE-102).
///
/// nesdev ("INES Mapper 206"): two bank registers behind $8000 (select, even) and $8001 (data,
/// odd), register mask **$E001**; `PRG Window: 8K + 8K + 16K fixed`, so R6 and R7 switch
/// $8000-$9FFF and $A000-$BFFF and "the last two 8KiB banks [are] fixed to the end";
/// `CHR Window: 2Kx2 + 1Kx4`, so the left pattern table takes R0 and R1 as two 2 KiB banks and
/// the right one R2-R5 as four 1 KiB banks. `Mirroring: Fixed H/V, or 4` — **hardwired, no
/// register touches it** — `IRQ: None`, and `PRG RAM: None`.
///
/// **The four variants are runtime fields and not `const` parameters**, which is the opposite of
/// the choice [`super::McAcc`] made and for the reason §3.2 of `docs/review/MF-6-inventory.md`
/// gives: the MC-ACC's difference is read by `on_ppu_address_bus` on every PPU fetch, where every
/// difference below is read only by [`Namcot108::update_banks`], i.e. on a CPU write to
/// $8000-$FFFF — "the rarest bus event there is". Five monomorphisations would have cost five
/// `Board` variants, five jump-table slots in every dispatch and five arms on
/// `Mapper::irq_asserted`, which §4.2 prices at +11 instructions in `Cpu6502::branch` and +47 in
/// `Cpu6502::resolve` each. One variant with four `bool`s costs one of each.
///
/// The variants, each from its own page:
///
/// * **76**, NAMCOT-3446 (*Megami Tensei*): "four CHR registers and two PRG registers", R2-R5
///   being **four 2 KiB windows** at $0000, $0800, $1000 and $1800 with a 6-bit data field, so
///   128 KiB of CHR is reachable where the 108 reaches 64 KiB.
/// * **88**, NAMCOT-3433/3443 (*Dragon Spirit*, *Quinty*): "CHR support is increased to 128 KiB
///   by connecting PPU's A12 line to the CHR ROM's A16 line", so the left pattern table reaches
///   only the first 64 KiB and the right one only the second. The page gives the implementation
///   as well — "masking CHR bank outputs by ANDing with $3F and ORing with $40 for Namco 108
///   registers 2-5" — and adds that "if the CHR ROM is 64 KiB or smaller, it is identical to
///   mapper 206", which is why the mask is applied unconditionally: on a 64 KiB CHR the `| $40`
///   wraps back onto the same bank.
/// * **95**, NAMCOT-3425 (*Dragon Buster*): the data register is `..ND DDDD`, so **bit 5 is a
///   nametable select** and the bank field is five bits. R0 drives "nametable at PPU
///   $2000-$27FF" and R1 "nametable at PPU $2800-$2FFF" — CHR A15 wired to CIRAM A10. "Only
///   horizontal mirroring and 1-screen mirroring are possible because the Namco 108 lacks the C
///   bit of MMC3."
/// * **154**, NAMCOT-3453 (*Devil Man*): the 88's CHR wiring plus `$8000-$FFFF: [.N.. ....]`,
///   N being one-screen select (0 = 1ScA, 1 = 1ScB). The page states the difference this board
///   is here to pin: "the nametable control bit is present over the entire 32kB range; unlike
///   the associated Namco 108 which is only present at $8000-$9FFF".
///
/// # The sixth number, which is not a Namco board at all
///
/// **112** is an unlicensed clone (MF-UNL-7 step 2) whose page says it is "quite similar to iNES
/// Mapper 206, which resembles an simplified MMC3; however, the registers appears to be scrambled
/// a bit". It is here, behind `board-112`, because the scramble is in the **ports and the register
/// order** and nothing else: the window table this board composes, `update_banks` included, is the
/// 206's to the line. See [`Namcot108::write_register_112`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namcot108 {
    banks: BankWindows,
    bank: [u8; 8],
    select: u8,
    /// 76, 88, 95, 154 or 206, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// What [`Mapper::nametable_page`] answers. `FOLLOW_MIRRORING` from one end of a run to the
    /// other on every number but 95, which is the only one in
    /// `Cartridge::nametable_pages_vary`'s list. Derived state: `load_registers` recomputes it
    /// and no savestate byte carries it (REQ-CORE-091), the `level` lesson of MF-5a step 3.
    nametable_pages: [i32; 4],
    /// Mapper 76: R2-R5 are four 2 KiB windows instead of R0/R1 two plus R2-R5 four.
    chr_2k_windows: bool,
    /// Mappers 88 and 154: PPU A12 drives CHR A16.
    chr_a16_from_a12: bool,
    /// Mapper 95: bit 5 of the data register is CIRAM A10, not a CHR bank bit.
    nametable_from_chr: bool,
    /// Mapper 154: bit 6 of **any** write to $8000-$FFFF is a one-screen select.
    one_screen_register: bool,
    /// Mapper 112: the select, data and mirroring ports are at $8000, $A000 and $E000 under mask
    /// $E001, and the eight register indices are the 206's rotated by six (MF-UNL-7 step 2).
    ///
    /// **Behind its own feature, so that with `board-112` off this struct and every function of it
    /// is byte for byte what MF-6a step 1 shipped**: the owner's rule is one feature per board
    /// that costs, and the five Namco numbers may not pay for a sixth that is not theirs.
    #[cfg(feature = "board-112")]
    scrambled_ports: bool,
}

impl Namcot108 {
    /// The bank field of mapper 95's data register, `..ND DDDD` losing its `N`. Every other
    /// number of the family uses the whole byte and lets [`BankWindows`] wrap it, which is the
    /// same answer as masking for a power-of-two ROM.
    const CHR_DATA_95: u8 = 0x1F;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            bank: [0; 8],
            select: 0,
            mapper_number,
            nametable_pages: [Cartridge::FOLLOW_MIRRORING; 4],
            chr_2k_windows: mapper_number == 76,
            chr_a16_from_a12: mapper_number == 88 || mapper_number == 154,
            nametable_from_chr: mapper_number == 95,
            one_screen_register: mapper_number == 154,
            #[cfg(feature = "board-112")]
            scrambled_ports: mapper_number == 112,
        }
    }

    /// Mapper 112's register decode, and the whole of what it adds to this board
    /// (nesdev, "INES Mapper 112", MF-UNL-7 step 2).
    ///
    /// Three ports under the page's own **mask $E001** where the 206 has two under $E001:
    ///
    /// ```text
    /// Bank select ($8000)     Bank data ($A000)     Nametable arrangement ($E000)
    /// xxxx xRRR               DDDD DDDD             xxxx xxxN
    /// ```
    ///
    /// The 206 reaches its pair at $8000 (even) and $8001 (odd); this board puts the data port at
    /// $A000 instead, which is the first half of the page's "the registers appears to be scrambled
    /// a bit". **$8001, $A001, $C000, $C001 and $E001 are inside the mask and are not registers**:
    /// the page names three addresses and this decode answers three.
    ///
    /// # The second half of the scramble is a rotation by six
    ///
    /// | `RRR` | what the page says it selects | the 206's register |
    /// | ---: | --- | ---: |
    /// | 0 | 8 KB PRG at $8000-$9FFF | R6 |
    /// | 1 | 8 KB PRG at $A000-$BFFF | R7 |
    /// | 2 | 2 KB CHR at PPU $0000-$07FF | R0 |
    /// | 3 | 2 KB CHR at PPU $0800-$0FFF | R1 |
    /// | 4-7 | 1 KB CHR at PPU $1000, $1400, $1800, $1C00 | R2-R5 |
    ///
    /// So `RRR` names the 206's `(RRR + 6) mod 8` and the windows are identical — which is why
    /// this number is a parameter of this board and not a board: nothing below the register file
    /// differs, and `Namcot108::update_banks` is reused to the line.
    ///
    /// # Two things the page does not say, and what this build does with them (Q-161)
    ///
    /// * **$C000-$DFFF.** The page lists eight `RRR` values covering two PRG windows and the whole
    ///   of CHR, and says nothing about the upper 16 KiB. The 206's "the last two 8KiB banks [are]
    ///   fixed to the end" is taken, because that is the board this one is "quite similar to" and
    ///   because a board with no vectors cannot run.
    /// * **the low bit of the two 2 KiB CHR registers.** The 206's page is explicit that "only bits
    ///   5-1 exist for 2KB CHR banks"; this page gives the data port a whole byte and uses the
    ///   MMC3's own wording, "Select 2 KB CHR bank", for selects 2 and 3. The 206's rule is taken —
    ///   the register drives CHR A11 upwards and the window's A10 comes from the PPU, which is a
    ///   property of the silicon the page says this is — so a 2 KiB select ignores bit 0.
    ///
    /// # And one it does say in a way worth quoting
    ///
    /// "Nametable arrangement (0: horizontal (A10); 1: vertical (A11))". The parentheses are what
    /// settle it: `horizontal (A10)` is CIRAM A10 driven from **PPU A10**, which is this core's
    /// [`NametableMirroring::Vertical`] — the two logical tables distinct across A10 — and `(A11)`
    /// is [`NametableMirroring::Horizontal`]. The numbering is then the MMC3's own
    /// "0: vertical; 1: horizontal" after all.
    #[cfg(feature = "board-112")]
    fn write_register_112(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xE001 {
            // The whole byte is latched and masked where it is read, which is this board's own
            // idiom one arm below and the MMC3's before it.
            0x8000 => self.select = value,
            0xA000 => {
                self.bank[usize::from(self.select.wrapping_add(6) & 0x07)] = value;
                self.update_banks();
            }
            0xE000 => memory.set_mirroring(if value & 0x01 == 0 {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            }),
            _ => {}
        }
    }

    /// Mapper 95's nametables: bit 5 of R0 for the two logical tables at $2000-$27FF and bit 5
    /// of R1 for the two at $2800-$2FFF (nesdev, "INES Mapper 095", the bank-data diagram and
    /// the bank-select list). Mesen 2 reads the same two bits (`Namco108_95.h:11-13`).
    ///
    /// Every other number of the family returns here at once, so its four pages stay
    /// `FOLLOW_MIRRORING` and `Mapper::nametable_page` answers what the trait's default answers.
    fn refresh_nametable_pages(&mut self) {
        if !self.nametable_from_chr {
            return;
        }

        let page = |register: usize| i32::from((self.bank[register] >> 5) & 0x01);
        self.nametable_pages = [page(0), page(0), page(1), page(1)];
    }

    fn update_banks(&mut self) {
        self.refresh_nametable_pages();

        // "8K + 8K + 16K fixed": R6 at $8000, R7 at $A000, the last two banks at $C000 and
        // $E000.
        self.banks.map_prg_8k(0, i32::from(self.bank[6]));
        self.banks.map_prg_8k(1, i32::from(self.bank[7]));
        self.banks.map_prg_8k(2, -2);
        self.banks.map_prg_8k(3, -1);

        if self.chr_2k_windows {
            // Mapper 76: R2 at $0000, R3 at $0800, R4 at $1000, R5 at $1800, each a 2 KiB bank
            // number in its own right (nesdev, "INES Mapper 076").
            for window in 0..4 {
                self.banks
                    .map_chr_2k(window * 2, i32::from(self.bank[2 + window]));
            }

            return;
        }

        // The 108's own layout. R0 and R1 hold a **1 KiB** bank number whose bit 0 does not
        // exist ("only bits 5-1 exist for 2KB CHR banks"), which is `>> 1` into a 2 KiB bank.
        let data = if self.nametable_from_chr {
            Self::CHR_DATA_95
        } else {
            0xFF
        };
        let left = |value: u8| {
            let value = value & data;
            i32::from(if self.chr_a16_from_a12 {
                value & 0x3F
            } else {
                value
            }) >> 1
        };
        self.banks.map_chr_2k(0, left(self.bank[0]));
        self.banks.map_chr_2k(2, left(self.bank[1]));

        for window in 0..4 {
            let value = self.bank[2 + window] & data;
            self.banks.map_chr_1k(
                4 + window,
                i32::from(if self.chr_a16_from_a12 {
                    value | 0x40
                } else {
                    value
                }),
            );
        }
    }
}

impl Mapper for Namcot108 {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        // The pages follow from the registers, so they are recomputed and not read back.
        self.refresh_nametable_pages();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 8];
        self.bank[7] = 1;
        self.select = 0;
        self.update_banks();
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// `#[inline(never)]`, so that this decode stays **out of** `NesBus::write_internal`.
    ///
    /// Inlined, the arm put 51 instructions into that function (3 020 → 3 071); out of line it
    /// takes 16 *off* it (3 020 → **3 004**), the board paying one `callq` per write to
    /// $8000-$FFFF, which is the rarest bus event there is. That is what keeps the five boards
    /// of MF-6a step 2 from compounding into the function every lot has had to watch.
    ///
    /// **It does not buy back what this board costs the MMC3**, and the figure is in
    /// `docs/progress/MF-6a.md` step 1 rather than hidden here: `write_internal` costs `m4`
    /// +66 921 instructions over 300 frames and `smb3` +16 561 — the two MMC3 ROMs of the set,
    /// and only those two — and the figure is **the same to the instruction** whether the arm is
    /// inlined,
    /// `#[inline(never)]` or `#[cold] #[inline(never)]`, which is what says it is the dispatch's
    /// target count and not this arm's shape. Every other ROM of lot PR-1's set is identical to
    /// the instruction, and a 33rd `Board` variant whose `write_register` is the *default* empty
    /// one costs all thirteen nothing at all (step 1's experiment X).
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        // Mapper 112's three ports, before the five Namco numbers' two: the board is the same
        // below the register file and nothing but this decode differs (MF-UNL-7 step 2). Behind
        // the feature, so that with `board-112` off this function is what MF-6a step 1 shipped.
        #[cfg(feature = "board-112")]
        if self.scrambled_ports {
            self.write_register_112(memory, address, value);
            return;
        }

        // Mapper 154's N, "present over the entire 32kB range", taken before the register
        // decode because it is not part of it: a write to $E000 sets the nametable and selects
        // nothing (nesdev, "INES Mapper 154").
        if self.one_screen_register {
            memory.set_mirroring(if value & 0x40 == 0 {
                NametableMirroring::SingleScreenA
            } else {
                NametableMirroring::SingleScreenB
            });
        }

        // Register mask $E001, i.e. the two registers live at $8000-$9FFF and the three MMC3
        // register pairs above them do not exist on this chip. The 154 page's "only present at
        // $8000-$9FFF" is the statement that A13 and A14 **are** decoded here; Mesen 2 decodes
        // `addr & $8001` instead (`Namco108.h:17`), which would make a write to $A001 a bank
        // write, and AMD-061 §1 puts the documented hardware above a reference implementation.
        match address & 0xE001 {
            // The whole byte is latched and masked where it is read, which is the MMC3's own
            // idiom (`Mmc3Board::write_register`): bits 7-6 are that chip's CHR and PRG mode,
            // and on this one they are simply not wired to anything. Masking at the index also
            // keeps `panic_bounds_check` out of the arm.
            0x8000 => self.select = value,
            0x8001 => {
                self.bank[usize::from(self.select & 0x07)] = value;
                self.update_banks();
            }
            _ => {}
        }
    }
}
