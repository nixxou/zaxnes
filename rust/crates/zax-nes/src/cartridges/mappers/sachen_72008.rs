use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **133**: the Sachen 72008 (UNIF `UNL-SA-72008`) of *Jovial Race* and the Joy Van
/// release of *盜帥 — Master Chu and the Drunkard Hu* (REQ-CORE-102, MF-UNL-1 step 3).
///
/// One latch, and that is the whole board:
///
/// ```text
/// Mask: $E100
/// $4100: [.... .PCC] - 32 KiB PRG bank and 8 KiB CHR bank
/// ```
///
/// # The page has two decodes, and this is the 72-pin one
///
/// nesdev ("INES Mapper 133") documents the number twice, because two boards carry it:
///
/// * the **60-pin Sachen 3009**, a Namco-108 clone marked "ASLIC AX-24G" with a select/data pair
///   at $8000/$8001 under mask $E001, where `$8000: [..B. .B.B]` picks the register and
///   `$8001: [...X XPCC]` writes it — the `XX` field being an **analog feedback path** that maps
///   `$00/$08/$10/$18` onto registers `$04/$24/$01/$21` and "may cause rapid bank switching if
///   unstable";
/// * the **72-pin Sachen 72008**, the one latch above.
///
/// **This build implements the 72-pin decode and not the 60-pin one, and the page is what
/// settles it**, in its own closing sentence: *"Only the 72-pin variant requires emulation, as
/// the 60-pin version contains backwards-compatible bankswitching code."* The 60-pin board's
/// games write the 72-pin register as well, so one decode runs both; the analog feedback path
/// has no deterministic behaviour to implement and no game that needs it. No submapper and no
/// heuristic therefore selects between them — there is one decode — and a dump of either board
/// runs on it. Mesen 2 makes the same call (`Sachen_133.h` has only the $4100 latch).
///
/// # The mask, and what it keeps out of $6000-$7FFF
///
/// **$E100**, so the latch answers at $4100-$41FF, $4300-$43FF and so on up to $5F00-$5FFF, and
/// **nothing in $6000-$7FFF** (`$6100 & $E100` is `$6100`). Mesen decodes `(addr & $6100) ==
/// $4100` over a $4100-$FFFF window, one address line narrower, which also catches $C100-$DFFF;
/// the page's mask is used here. With the register confined below $6000 the board needs no
/// [`Mapper::intercept_cpu_write`] body and `registry::board_memory_sizes` leaves the
/// $6000-$7FFF window to the header, which is the mapper-97 rule the registry's own comment
/// states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sachen72008 {
    banks: BankWindows,
    /// The `[.... .PCC]` latch, kept whole: the CHR field is bits 1-0 and the PRG bit is bit 2.
    register: u8,
}

impl Sachen72008 {
    /// The latch's decode: "Mask: $E100", `$4100`.
    const PORT_MASK: u16 = 0xE100;
    const PORT: u16 = 0x4100;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            register: 0,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        // "[.... .PCC] - 32 KiB PRG bank and 8 KiB CHR bank".
        self.banks
            .map_prg_32k(i32::from((self.register >> 2) & 0x01));
        self.banks.map_chr_8k(i32::from(self.register & 0x03));
    }
}

impl Mapper for Sachen72008 {
    banked!();

    fn mapper_number(&self) -> i32 {
        133
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
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & Self::PORT_MASK == Self::PORT {
            self.register = value & 0x07;
            self.update_banks();
        }
        // Everything else, $8000-$FFFF included: the mask decodes A15-A13 and A8, and
        // `$8100 & $E100` is `$8100`, so no address above $5FFF can be the latch.
    }
}
