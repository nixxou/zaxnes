use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 99, the Nintendo VS. System board (REQ-CORE-102, lot MF-VS step 4).
///
/// nesdev ("INES Mapper 099"). The board has **one bit** of register and it does not live on the
/// cartridge at all: it is bit 2 of a `$4016` write, one of the 2A03's own output pins.
///
/// | address | what it holds |
/// | --- | --- |
/// | CPU `$6000-$7FFF` | 2 KiB RAM (shared between the two CPUs of a `DualSystem`; open bus when it is the other CPU's) |
/// | CPU `$8000-$9FFF` | 8 KiB switchable PRG-ROM bank, **on the 40 KiB images only** |
/// | CPU `$A000-$FFFF` | 24 KiB fixed PRG-ROM |
/// | PPU `$0000-$1FFF` | 8 KiB switchable CHR-ROM bank |
///
/// The bank select is the page's: `$4016` write bit 2, "Select both PRG ROM and CHR ROM bank".
/// nesdev's "Vs. System" gives the same bit from the other side, in its `$4016` write layout:
/// "Bit 2: 8 KiB CHR ROM bank selection (Mapper 99 only)". The two other low bits of that write
/// are the controller strobe and the `DualSystem` line, which is why the page adds that
/// "developers typically maintain a separate copy of this value".
///
/// **The PRG half only exists above 32 KiB.** The page: "Only *Vs. Gumshoe* uses the 40 KiB PRG
/// variant; in the iNES encapsulation, the 8 KiB banks are arranged as 0, 1, 2, 3, 0 alternate,
/// empty", the hardware doing it "through an additional wire connecting the 2A03's OUT2 pin to
/// address line A13 on the EPROM socket". So the bit picks 8 KiB bank 0 or bank **4** at `$8000`
/// and the three windows above it never move. On a 32 KiB image there is no fifth bank and the
/// window stays at 0, which is also what the `DualSystem` 48 and 64 KiB images want (this build
/// runs them as the main console: a recorded gap).
///
/// Two things the page states that this board does **not** do, both recorded as gaps in
/// `docs/progress/MF-VS.md`:
///
/// * "unlike all other mappers, an undersize mapper 99 image implies open bus instead of
///   mirroring" — a dump with 8 KiB of CHR shows nothing while the bank bit is high. This core
///   wraps bank numbers in [`BankWindows`] for every board, so such an image shows bank 0 twice.
///   Mesen 2 says the same of itself (`VsSystem.h`: "it doesn't look like any game actually rely
///   on this behavior? So not implemented for now").
/// * the `DualSystem`'s swap of the 2 KiB at `$6000-$7FFF` between the two CPUs, which needs the
///   second CPU this build does not have.
///
/// The board is a register decode and nothing else — no IRQ, no bus conflicts, no mirroring
/// register (every VS image is four-screen) — so it is a [`super::DecodeBoard`] variant and costs
/// the `Board` enum no discriminant (`PERFORMANCE-HISTORY.md` §5.15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsBoard {
    banks: BankWindows,
    /// Bit 2 of the last `$4016` write: the PRG **and** CHR bank select.
    select: bool,
    /// Whether this image has a fifth 8 KiB PRG bank for the select bit to reach, settled at
    /// load: true for the 40 KiB *Vs. Gumshoe* layout and false for every 32 KiB image.
    banked_prg: bool,
}

impl VsBoard {
    /// iNES mapper number of the board.
    pub const MAPPER: i32 = 99;

    /// The RAM at `$6000-$7FFF`: "2 KiB RAM" on the page, and not the iNES default of 8 KiB.
    ///
    /// The window mirrors it four times, which is what the missing address lines of the board do.
    /// On a `DualSystem` the chip is shared between the two CPUs and reads open bus while it is
    /// the other one's — that needs the second CPU this build does not have, a recorded gap.
    pub const PRG_RAM_SIZE: usize = 2 * 1024;

    /// PRG-ROM above this size has the alternate bank the select bit reaches.
    const BANKED_PRG_THRESHOLD: usize = 32 * 1024;

    /// The 8 KiB bank the select bit puts at `$8000` when it is set: the page's "0 alternate",
    /// the fifth bank of the 40 KiB layout.
    const ALTERNATE_PRG_BANK: i32 = 4;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            select: false,
            banked_prg: memory.rom().prg_rom().len() > Self::BANKED_PRG_THRESHOLD,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        if self.banked_prg {
            self.banks.map_prg_8k(
                0,
                if self.select {
                    Self::ALTERNATE_PRG_BANK
                } else {
                    0
                },
            );
        } else {
            self.banks.map_prg_8k(0, 0);
        }

        // $A000-$FFFF is the fixed 24 KiB, whatever the select bit says.
        self.banks.map_prg_8k(1, 1);
        self.banks.map_prg_8k(2, 2);
        self.banks.map_prg_8k(3, 3);
        self.banks.map_chr_8k(i32::from(self.select));
    }
}

impl Mapper for VsBoard {
    banked!();

    fn mapper_number(&self) -> i32 {
        Self::MAPPER
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_bool(self.select);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.select = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.select = false;
        self.update_banks();
    }

    /// The `$4016` write, forwarded by `NesBus::write_internal` for a VS image and by nothing
    /// else — that address is a CPU register and never reaches a cartridge otherwise.
    ///
    /// `#[inline(never)]`, as the other boards of this enum: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address != Self::PORT {
            return;
        }

        // "Select both PRG ROM and CHR ROM bank" (nesdev, "INES Mapper 099"), bit 2 of the write
        // (nesdev, "Vs. System", $4016 write layout).
        let select = value & 0x04 != 0;
        if select != self.select {
            self.select = select;
            self.update_banks();
        }
    }
}

impl VsBoard {
    /// The CPU address the board's one register is written through.
    pub const PORT: u16 = 0x4016;
}
