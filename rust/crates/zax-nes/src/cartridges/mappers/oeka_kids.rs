use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **96**, the Bandai Oeka Kids board (MF-6c step 8).
///
/// nesdev ("INES Mapper 096"), one CPU register and one thing that is not a register at all:
///
/// ```text
/// CPU $8000-$FFFF   7  bit  0
///                   ---- ----
///                   xxxx xCPP
///                         |||
///                         |++- Select 32KiB PRG bank
///                         +--- Select 16KiB outer CHR bank
///
/// PPU $2000-$2FFF  13  PPU address  0
///                   DD xxCC xxxx xxxx
///                   ||   ||
///                   ||   ++---------- Select 4 KiB inner CHR RAM bank for PPU $0000-$0FFF
///                   ++--------------- Latch CC bits when 'DD' changes from anything else to '10'
/// ```
///
/// "PRG ROM capacity: 128 KiB", "PRG RAM capacity: No", "CHR capacity: 32 KiB RAM", "Hardwired
/// Vertical mirroring". The two titles are *Oeka Kids: Anpanman no Hiragana Daisuki* and
/// *Oeka Kids: Anpanman to Oekaki Shiyou!!*.
///
/// # The PPU address bus is the whole board, and §4.4 said so before it was written
///
/// §4.4 of `docs/review/MF-6-inventory.md` calls this "the one PPU-read-shaped hook in the
/// inventory, and it is not this hook" — not `Mapper::ppu_read`, because the board reacts to
/// nametable **addresses** and not to pattern fetches, and the page is explicit about what that
/// means:
///
/// > This latching occurs regardless of whether the PPU or game code initiated the address change,
/// > enabling software to manually select CHR pages by writing specific values to $2006.
///
/// So the board is in **both** of §4.3's latches — [`Cartridge::watches_ppu_reads`] for the
/// rendering's own fetches and [`Cartridge::watches_ppu_bus_without_read`] for a bare `$2006`
/// write with no fetch behind it — and that is this step's answer to the open question MR-1
/// records against the MMC2 and classifies *unknown*. **The two cases are not the same question.**
/// What MR-1 leaves open is whether the MMC2's *latch*, whose rule is about /RD, should move on a
/// bare address change; REQ-CORE-033 and REQ-CORE-039 constrain that one and this step does not
/// touch it. What this board needs is written on its own page in the words quoted above, so it
/// gets it, and the distinction §4.4 calls "not academic" is settled **per board and from each
/// board's own documentation** rather than by one rule for both.
///
/// [`Cartridge::watches_ppu_reads`]: super::super::Cartridge
/// [`Cartridge::watches_ppu_bus_without_read`]: super::super::Cartridge
///
/// The latch is an **edge** on the two top address bits and not a decode of them: `DD` must change
/// "from anything else to '10'", so walking inside $2000-$2FFF changes nothing and only the entry
/// into it does. [`OekaKids::last_address`] is what remembers the other side of that edge, and it
/// is in the save state because nothing derives it. Mesen 2 reads the same edge the same way
/// (`OekaKids.h`, `NotifyVramAddressChange`).
///
/// The price is the cost class §4.4 predicted: an A12 board's. The board has **no IRQ**, so
/// [`Mapper::ppu_irq_horizon`] keeps `Mapper`'s own `Some(i64::MAX)` default — its /IRQ line cannot
/// change through anything the PPU does, because it has none — and what it does pay is
/// [`Mapper::chr_view`] answering `None` for the whole [`super::DecodeBoard`] variant, which
/// `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` requires of any board in that
/// latch: `Ppu2C02::run_visible_span` takes a whole line of dots without telling the cartridge
/// anything, and a board that must hear every address cannot afford that path.
///
/// # The tablet, recorded as a gap
///
/// Both titles are drawing programs for the **Oeka Kids tablet**, a peripheral on the expansion
/// port. This build has no host surface for an input device of that kind — the gap MR-30-4 records
/// for the UNROM 512's flash, MF-6c step 3's for the Datach barcode scanner and MF-6c step 4's for
/// *Nantettatte!! Baseball*'s external ROM, in the same words. The board is implemented and the
/// games boot; what cannot be done with them is draw. The brief of this lot names the peripheral
/// as a recorded gap and not a deliverable.
///
/// # One reading taken, and it is **Q-135**
///
/// The page's infobox says "Bus conflicts: **partly**", which is the one word in it that does not
/// decide anything: this build has a single `bus_conflicts` bool per cartridge (MR-BUS-1), so
/// "partly" has to become yes or no. It is **yes** here, which is Mesen 2's reading
/// (`HasBusConflicts` true) — see Q-135 for the two readings and why this one was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OekaKids {
    banks: BankWindows,
    /// Bit 2 of the CPU register, kept **as a 4 KiB page offset** (0 or 4) rather than as the bit:
    /// the 16 KiB outer bank is four of the 4 KiB pages the board banks in, so `outer | inner` is
    /// the page number and no shift is needed on the PPU's side of the board. Mesen 2 keeps it the
    /// same way (`_outerChrBank = value & 0x04`).
    outer_chr: u8,
    /// The two bits last latched off PPU A9-A8.
    inner_chr: u8,
    /// The previous PPU address, which is the other side of the edge on bits 13-12.
    last_address: u16,
}

impl OekaKids {
    /// "CHR capacity: 32 KiB RAM" — eight of the 4 KiB pages the board banks in. The same figure
    /// REQ-CORE-012's default would give an image with no CHR-ROM, stated here because the page
    /// states it and the header cannot.
    pub(crate) const CHR_RAM_SIZE: usize = 32 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            outer_chr: 0,
            inner_chr: 0,
            last_address: 0,
        }
    }

    /// The two 4 KiB CHR windows: the latched page at $0000-$0FFF, and the outer bank's **last**
    /// page wired at $1000-$1FFF (Mesen 2's `SelectChrPage(1, _outerChrBank | 0x03)`).
    fn update_chr(&mut self) {
        self.banks
            .map_chr_4k(0, i32::from(self.outer_chr | self.inner_chr));
        self.banks.map_chr_4k(4, i32::from(self.outer_chr | 0x03));
    }
}

impl Mapper for OekaKids {
    banked!();

    fn mapper_number(&self) -> i32 {
        96
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.outer_chr);
        writer.write_u8(self.inner_chr);
        writer.write_u16(self.last_address);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.outer_chr = reader.read_u8()?;
        self.inner_chr = reader.read_u8()?;
        self.last_address = reader.read_u16()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.outer_chr = 0;
        self.inner_chr = 0;
        self.last_address = 0;
        memory.set_mirroring(NametableMirroring::Vertical);
        self.banks.map_prg_32k(0);
        self.update_chr();
    }

    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and every board of this shape has carried the attribute
    /// since.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, _address: u16, value: u8) {
        // "xxxx xCPP": two bits of 32 KiB PRG bank and the outer CHR bank above them.
        self.banks.map_prg_32k(i32::from(value & 0x03));
        self.outer_chr = value & 0x04;
        self.update_chr();
    }

    /// The latch, on the rising edge of `DD` into `'10'` — the entry into PPU $2000-$2FFF.
    ///
    /// **`#[inline(never)]`, and the reason is the arms this one shares a dispatch with.**
    /// `Board::on_ppu_address_bus` is behind two latches, so no board outside them reaches it at
    /// all (MF-3b step 2 made that free); but the boards that *do* reach it are the three A12
    /// counters and the MMC5's in-frame detection, whose arms run on 57 600 fetches a frame. Out of
    /// line, this body is one `callq` per PPU address change of an Oeka Kids cartridge and nothing
    /// at all for them.
    #[inline(never)]
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if self.last_address & 0x3000 != 0x2000 && address & 0x3000 == 0x2000 {
            self.inner_chr = ((address >> 8) & 0x03) as u8;
            self.update_chr();
        }

        self.last_address = address;
    }
}
