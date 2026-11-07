use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mappers **79 and 146**: American Video Entertainment's **NINA-03** and **NINA-06**, and
/// the 60-pin Sachen **3015** and **SA-016** that carry the same logic (REQ-CORE-102,
/// MF-UNL-3 step 1).
///
/// One page, nesdev's **"NINA-003-006"**, documents both numbers and the whole board is one
/// register:
///
/// ```text
/// A~[010x xxx1 xxxx xxxx]   D~[.... PCCC]
///                                 |+++--  8 KiB CHR bank at PPU $0000-$1FFF
///                                 +-----  32 KiB PRG bank at CPU $8000-$FFFF
/// ```
///
/// The address pattern reads A15, A14, A13 and A8 and nothing else, which is **mask $E100**
/// against **$4100** — the window `$4100-$41FF, $4300-$43FF, $45xx, $47xx, …, $5Dxx, $5Fxx` the
/// page enumerates, and the same one [`super::Sachen72008`] decodes. `$6100 & $E100` is `$6100`,
/// so no address in $6000-$7FFF can reach it; the board therefore needs no
/// [`Mapper::intercept_cpu_write`] body, and `registry::board_memory_sizes` gives it no PRG-RAM
/// on the page's own "PRG RAM capacity: None" rather than because the register is in the way.
///
/// The two numbers differ in nothing this core can see: the NINA-03 and the NINA-06 "differ only
/// in CIC type", and 146 "denotes the 60-pin Sachen 3015 and SA-016 (chip-on-board) circuit
/// boards, **which are functionally identical**". So one board carries both numbers and
/// [`Mapper::mapper_number`] is the only field that tells them apart.
///
/// # The CNROM switch at $8000-$FFFF is deliberately not implemented
///
/// FCEUX gives this board a second register — a CNROM-style 8 KiB CHR bank from a write to
/// $8000-$FFFF — and this build has nothing there. **The page is why**: its last line is that
/// "iNES Mapper **148** has the same bit assignment in the data latch, but moves the latch
/// register to the CPU $8000-$FFFF register, **introducing bus conflicts**". The latch of *this*
/// number is at $4100 and the board's own row says "Bus conflicts: **No**", which is only true of
/// a board whose register the PRG-ROM does not drive. A CNROM switch at $8000 is therefore mapper
/// 148's board and not this one; what makes emulators carry it is that mapper-3 images of AVE
/// titles have circulated mislabelled, and the fix for a mislabelled image is its header. A
/// mapper-79 dump that wrote $8000 expecting a CHR switch would be such an image.
///
/// `req_core_102_mapper_79_ignores_a_write_at_8000` is the anti-test the census asks for, and it
/// is deliberate rather than an omission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nina03 {
    banks: BankWindows,
    /// The `[.... PCCC]` latch, kept whole: `CCC` is bits 2-0 and `P` is bit 3.
    register: u8,
    /// 79 or 146, for [`Mapper::mapper_number`].
    mapper_number: i32,
}

impl Nina03 {
    /// "This register is located at addresses: $4100-$41FF, $4300-$43FF, $45xx, $47xx, …, $5Dxx,
    /// and $5Fxx", which is `A~[010x xxx1 xxxx xxxx]` — A15, A14, A13 and A8.
    const PORT_MASK: u16 = 0xE100;
    const PORT: u16 = 0x4100;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            register: 0,
            mapper_number,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        // "+----- Select 32 KB PRG ROM bank for CPU $8000-$FFFF" — bit 3, one line, because the
        // board's 64 KiB of PRG-ROM is two 32 KiB banks.
        self.banks
            .map_prg_32k(i32::from((self.register >> 3) & 0x01));
        // "|+++-- Select 8 KB CHR ROM bank for PPU $0000-$1FFF" — bits 2-0, eight banks of the
        // board's 64 KiB of CHR-ROM.
        self.banks.map_chr_8k(i32::from(self.register & 0x07));
    }
}

impl Mapper for Nina03 {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The windows themselves are in the block `Mapper::save_state` wrote before this one.
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
            self.register = value & 0x0F;
            self.update_banks();
        }
        // Everything else, $8000-$FFFF included: `$8100 & $E100` is `$8100`, and the CNROM
        // switch FCEUX puts there is mapper 148's board — see the struct's own note.
    }
}
