use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **86**, Jaleco's JF-13 of *Moero!! Pro Yakyuu* (MF-6c step 7).
///
/// nesdev ("INES Mapper 086"), two ports below $8000:
///
/// ```text
/// $6000-$6FFF      7  bit  0
///                  ---- ----
///                  .CPP ..CC
///                   |||   ||
///                   |++------ Select 32 KiB PRG bank at $8000
///                   +-----++- Select 8 KiB CHR bank at $0000
///
/// $7000-$7FFF      7  bit  0
///                  ---- ----
///                  ..RP TTTT
///                    || ||||
///                    || ++++- Track number
///                    |+------ While 0 and no sound is currently playing, start (or restart)
///                    |        sound specified in lower 4 bits
///                    +------- If 0 for at least 19us=34cycles, the µPD7756C is reset and sound
///                             playback stops
/// ```
///
/// "The mapping follows a pattern similar to GNROM" — but with the CHR field split in two, bit 6
/// carrying its third bit above bits 1-0. 128 KiB of PRG is four 32 KiB banks for a two-bit field
/// and 64 KiB of CHR is eight 8 KiB banks for a three-bit one, so both fields are exactly as wide
/// as the board they are on. Mesen 2 reads the same split (`JalecoJf13.h:19-20`).
///
/// # The decode **is** $6000-$7FFF, and the accidental copies fall out of it
///
/// "No PRG RAM (mapper ports occupy $6000-$7FFF instead)", so `registry::board_memory_sizes` gives
/// the board none and `Cartridge::cpu_write` walks past its PRG-RAM branch to
/// [`Mapper::write_register`] with the address intact — the shape mappers 47 and 37 use for their
/// outer register, and the reason this board needs no [`Mapper::intercept_cpu_write`] and so adds
/// no body to that dispatch.
///
/// And then the page's design flaw is **one mask**:
///
/// > This board is subject to the same design flaw as iNES Mapper 038: accidental copies of the
/// > registers are present at $E000-$FFFF because /ROMSEL is high when M2 is low, not just when
/// > A15 is low.
///
/// `address & 0x7000` is $6000 for both $6000-$6FFF and $E000-$EFFF and $7000 for both
/// $7000-$7FFF and $F000-$FFFF, and it is **nothing a register answers to** for all of
/// $8000-$DFFF. So the two documented ports and their two accidental copies are the same two arms
/// of [`JalecoJf13::write_register`], which is what the hardware's missing address line amounts
/// to; `req_core_100_the_jf13_registers_have_accidental_copies_at_e000` pins both halves.
///
/// Writes are **not** ANDed with the ROM: the page gives this board no bus-conflict line, and
/// Mesen 2 does not give it one either (`JalecoJf13.h` overrides no `HasBusConflicts`).
///
/// # The µPD7756C is not implemented
///
/// §4.8 of `docs/review/MF-6-inventory.md` is the decision and [`super::JalecoJf17`]'s note has
/// it in full: the samples are in a mask ROM inside the ASIC, REQ-CORE-075's list of
/// expansion-audio chips does not include the chip, and those bytes are in no source this project
/// may read. So $7000-$7FFF is **decoded and dropped** —
/// [`Mapper::expansion_audio_output`] keeps its `None` default and
/// `mf_6c_the_three_jaleco_upd7756c_boards_are_silent` pins the silence. The sixteen samples are
/// an umpire's calls; the game is playable without them, and what is lost is recorded voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoJf13 {
    banks: BankWindows,
    /// The $6000-$6FFF byte. Kept whole, because the two fields it holds are not contiguous.
    register: u8,
}

impl JalecoJf13 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        // ".CPP ..CC": bits 5-4 are the 32 KiB PRG bank, bits 1-0 and **bit 6** the 8 KiB CHR one.
        self.banks
            .map_prg_32k(i32::from((self.register >> 4) & 0x03));
        self.banks.map_chr_8k(i32::from(
            (self.register & 0x03) | ((self.register >> 4) & 0x04),
        ));
    }
}

impl Mapper for JalecoJf13 {
    banked!();

    fn mapper_number(&self) -> i32 {
        86
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.register = 0;
        self.update_banks();
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    ///
    /// The `match` is the board's address decode and the page's design flaw at once; see the
    /// struct's own note on `address & 0x7000`.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & 0x7000 == 0x6000 {
            self.register = value;
            self.update_banks();
        }
        // Everything else falls here, and the two cases are different in the hardware and the
        // same in this build: $7000-$7FFF (with its $F000-$FFFF copy) is the µPD7756C's track,
        // start and reset, which this board does not implement, and all of $8000-$DFFF is no
        // register at all because `address & 0x7000` cannot be $6000 or $7000 there.
    }
}
