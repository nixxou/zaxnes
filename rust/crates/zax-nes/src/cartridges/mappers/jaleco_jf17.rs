use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mappers **72** and **92**, Jaleco's JF-17 and JF-19 (MF-6c step 7).
///
/// nesdev ("INES Mapper 072"), one port at $8000-$FFFF with two latches in it:
///
/// ```text
/// 7  bit  0    PRG bank select
/// Pxxx DDDD
/// |    ||||
/// |    |+++- 16 KB PRG ROM bank for CPU $8000-$BFFF
/// |    +---- Required by mapper 92 games only
/// +--------- When a 1 is written after a 0 was previously written, load the bank specified by D
///
/// 7  bit  0    CHR bank select
/// xCxx DDDD
///  |   ||||
///  |   ++++- 8 KB CHR ROM bank for PPU $0000-$1FFF
///  +-------- When a 1 is written after a 0 was previously written, load the bank specified by D
/// ```
///
/// and nesdev ("INES Mapper 092"): "This mapper is identical to mapper 072 except for the
/// different PRG Setup." The two setups are the page's own diagrams:
///
/// ```text
///   mapper 072     $8000   $A000   $C000   $E000
///                +---------------+---------------+
///                |    PRG Reg    |     { -1}     |
///                +---------------+---------------+
///
///   mapper 092     $8000   $A000   $C000   $E000
///                +---------------+---------------+
///                |     { 0 }     |    PRG Reg    |
///                +---------------+---------------+
/// ```
///
/// So the difference is **one `bool`** — which window the register drives, and with it the width
/// of the field, because bit 3 is "Required by mapper 92 games only". A field and not a `const`
/// parameter, for the reason [`super::Sunsoft2`] and [`super::Bandai74161`] are fields: one
/// monomorphisation is one body in every dispatch it reaches, and two are two.
///
/// Mirroring is "Cuttable trace selects vertical or horizontal", so it comes from the header and
/// there is no mirroring register; "Bus conflicts: Yes", which is one arm of
/// [`super::super::registry::has_bus_conflicts`] and nothing in this file.
///
/// # The two edges, which are the whole board
///
/// The port is **not** a bank register: it is two 74174 latches behind a 74161 counter, and each
/// loads on the **rising edge** of its own bit of the data bus. "When a 1 is written after a 0 was
/// previously written" is a rising-edge detector, so the previous state of bits 7 and 6 is the
/// board's state — [`JalecoJf17::prg_bit`] and [`JalecoJf17::chr_bit`] — and the bank number in
/// the same write is loaded only when that bit was 0 before. Mesen 2 reads it the same way
/// (`JalecoJf17_19.h:40-54`), and a write with the bit already set changes nothing, which is what
/// `req_core_100_the_jf17_latches_load_on_a_rising_edge_of_their_own_bit` pins.
///
/// # The µPD7756C is not implemented, and §4.8 of `docs/review/MF-6-inventory.md` is the decision
///
/// Bits 5 and 4 of the same port drive an ADPCM speech chip: bit 5 holds it in reset ("Hold this
/// low for at least 34 cycles") and bit 4 starts a sample whose number comes from **the address
/// lines**, `1... .... ...A AAAA`. The samples are in a mask ROM inside the ASIC. REQ-CORE-075's
/// list of expansion-audio chips does not include this one and those bytes are in no source this
/// project may read, so the boards are implemented and the audio is not:
/// [`Mapper::expansion_audio_output`] keeps its `None` default and
/// `mf_6c_the_three_jaleco_upd7756c_boards_are_silent` pins the silence so that it is a choice and
/// not a bug. The two bits are **not** stored: there is no chip for them to be the state of, and a
/// byte in the save-state block for a device this build does not have would be weight with no
/// reader. Mesen 2 also ignores them ("Note: Audio bits not supported", `JalecoJf17_19.h:5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoJf17 {
    banks: BankWindows,
    /// Mapper 92, the JF-19: the register drives $C000-$FFFF and its field is four bits wide.
    is_jf19: bool,
    /// Bit 7 of the last write, so that the next one can see a rising edge.
    prg_bit: bool,
    /// Bit 6 of the last write.
    chr_bit: bool,
}

impl JalecoJf17 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        Self {
            banks: windows_for(memory),
            is_jf19: mapper == 92,
            prg_bit: false,
            chr_bit: false,
        }
    }

    /// The power-on window layout, which is also what a reset restores.
    ///
    /// `$C000`-`$FFFF` is the image's **last** bank on both numbers. On mapper 72 that is the
    /// page's `{ -1}` and it is wired; on mapper 92 the page makes that window the register's and
    /// **gives the register no power-on value**, so this is a choice, and it is the one Mesen 2
    /// makes (`SelectPrgPage(1, -1)` in its `InitMapper`, whatever the mode): the last bank is
    /// where a cartridge's vectors are, and $FFFA-$FFFF is inside this window on mapper 92.
    fn reset_banks(&mut self) {
        self.banks.map_prg_16k(0, 0);
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_8k(0);
    }
}

impl Mapper for JalecoJf17 {
    banked!();

    fn mapper_number(&self) -> i32 {
        if self.is_jf19 {
            92
        } else {
            72
        }
    }

    /// The two edge-detector bits. The banks themselves are in [`BankWindows`], which
    /// `Mapper::save_state` writes for every board (REQ-CORE-090), so there is nothing else to
    /// append — and nothing derived to recompute on the way back in, which is MF-5a step 3's
    /// `level` lesson applied in the other direction.
    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_bool(self.prg_bit);
        writer.write_bool(self.chr_bit);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg_bit = reader.read_bool()?;
        self.chr_bit = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg_bit = false;
        self.chr_bit = false;
        self.reset_banks();
    }

    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and every board of this shape has carried the attribute
    /// since.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, _address: u16, value: u8) {
        let prg_bit = value & 0x80 != 0;
        let chr_bit = value & 0x40 != 0;

        // "When a 1 is written after a 0 was previously written, load the bank specified by D."
        if prg_bit && !self.prg_bit {
            if self.is_jf19 {
                // Mapper 92: the fourth bit is the one the page marks "Required by mapper 92
                // games only", and the window is $C000-$FFFF.
                self.banks.map_prg_16k(2, i32::from(value & 0x0F));
            } else {
                self.banks.map_prg_16k(0, i32::from(value & 0x07));
            }
        }

        if chr_bit && !self.chr_bit {
            self.banks.map_chr_8k(i32::from(value & 0x0F));
        }

        // Bits 5 and 4 are the µPD7756C's reset and start, and this build has no such chip; see
        // the struct's own note and §4.8 of `docs/review/MF-6-inventory.md`.
        self.prg_bit = prg_bit;
        self.chr_bit = chr_bit;
    }
}
