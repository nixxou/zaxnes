use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// NES 2.0 mapper **263**, the **UNL-KOF97** board: "an MMC3 clone with scrambled mapper address
/// and data bits" (MF-UNL-2 step 4, REQ-CORE-102).
///
/// Two games by **Rex Soft**: *The King of Fighters '97* and *Boogerman II: The Final
/// Adventure*.
///
/// # The two scrambles
///
/// nesdev ("NES 2.0 Mapper 263") gives both as tables, under **mask `$F000`**. The address one is
/// a sentence — "**A12 is used for the MMC3's A0**" — and its table is that sentence read out:
///
/// | written | `$8000` | `$9000` | `$A000` | `$B000` | `$C000` | `$D000` | `$E000` | `$F000` |
/// | --- | --- | --- | --- | --- | --- | --- | --- | --- |
/// | MMC3 | `$8000` | `$8001` | `$A000` | `$A001` | `$C000` | `$C001` | `$E000` | `$E001` |
///
/// So the board's own A12 — bit 12 of the CPU address, which an MMC3 ignores entirely — is wired
/// to the chip's A0, and the chip's own A0 is not connected. One line of arithmetic reproduces
/// the whole table, and [`Mmc3Kof97::mmc3_port`] is it.
///
/// The data one the page gives twice, as a bit map and as the expression:
///
/// ```text
/// Written bit 76543210
///             --------
/// Actual bit  76243051
///
/// Val = (Val & 0xD8) | ((Val & 0x20) >> 4) | ((Val & 4) << 3) | ((Val & 2) >> 1) | ((Val & 1) << 2);
/// ```
///
/// [`Mmc3Kof97::permute`] is the expression verbatim, and
/// `req_core_100_mapper_263_permutes_every_one_of_the_256_written_bytes` checks it against the
/// **bit map** over all 256 inputs — the two statements of the same thing, each proving the
/// other.
///
/// # What is not here
///
/// Everything else. The page documents no other register, no protection, no outer bank and no
/// IRQ rule, so this board is an [`Mmc3`] with two translations in front of it and nothing more:
/// it holds the chip ([`super::Somari`]'s shape) and forwards every method the chip implements.
///
/// Its own *Note* is about the dumps rather than the board, and is recorded because it explains
/// why the number is rare: "Most versions of *The King of Fighters '97* are hacked for Mapper 4.
/// Several of them also fail to disable the frame IRQ, freezing on emulators that accurately
/// power-on with the frame IRQ enabled." This core does power on with the frame IRQ enabled
/// (REQ-CORE-113), so such a hacked dump freezes here too — correctly, and on the APU rather
/// than on the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Kof97 {
    /// The chip itself; the board is two translations in front of it.
    mmc3: Mmc3,
}

impl Mmc3Kof97 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            // 263 and not 4: `Mmc3Board::new` reads a submapper only for mapper 4, which is the
            // tightening MF-6a3 step 1 made for the TQROM. NES 2.0 defines no submapper for this
            // number, and an image of it carries the high mapper nibble in byte 8 where the
            // submapper also lives, so the tightening matters here.
            mmc3: Mmc3::new(memory, 263),
        }
    }

    /// Which MMC3 register the address `address` means: "A12 is used for the MMC3's A0."
    ///
    /// The chip decodes `$E001` — A14, A13 and A0 — and on this board A0 is not connected while
    /// A12 is. So the chip sees A14 and A13 as it always does, and its A0 carries the written
    /// A12: `(address & $E000) | ((address >> 12) & 1)`, which walks the page's eight-row table
    /// exactly.
    const fn mmc3_port(address: u16) -> u16 {
        (address & 0xE000) | ((address >> 12) & 0x01)
    }

    /// The page's data permutation, as its own expression:
    ///
    /// ```text
    /// Val = (Val & 0xD8) | ((Val & 0x20) >> 4) | ((Val & 4) << 3) | ((Val & 2) >> 1) | ((Val & 1) << 2);
    /// ```
    ///
    /// which is the bit map `76543210 → 76243051` written out: bits 7, 6, 4 and 3 stay (`$D8`),
    /// bit 5 moves to bit 1, bit 2 to bit 5, bit 1 to bit 0 and bit 0 to bit 2.
    const fn permute(value: u8) -> u8 {
        (value & 0xD8)
            | ((value & 0x20) >> 4)
            | ((value & 0x04) << 3)
            | ((value & 0x02) >> 1)
            | ((value & 0x01) << 2)
    }
}

impl Mapper for Mmc3Kof97 {
    fn mapper_number(&self) -> i32 {
        263
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
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

    /// The two scrambles, and then the chip.
    ///
    /// The page's mask is `$F000`, so the whole of `$8000-$FFFF` is the register window and every
    /// address in a 4 KiB block means the same chip register — which is what makes A12 free to
    /// carry A0 in the first place.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let port = Self::mmc3_port(address);
        self.mmc3.write_register(memory, port, Self::permute(value));
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: the fetches clock the A12 counter,
    /// so the PPU takes the full path.
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
