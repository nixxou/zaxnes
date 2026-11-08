use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// NES 2.0 mapper **292**, the **BMW8544** / UNL-DRAGONFIGHTER of *Dragon Fighter*: an MMC3 clone
/// whose CHR banking is **XORed with two registers a read fills**, both for protection and to
/// reach 512 KiB of CHR-ROM (MF-UNL-12 step 2, REQ-CORE-102).
///
/// nesdev ("NES 2.0 Mapper 292"): "used for the unlicensed game *Dragon Fighter* from 'Flying
/// Star', not to be confused with the licensed game from Natsume of the same name. Its PCB name is
/// 'BMW8544', its UNIF board name is **UNL-DRAGONFIGHTER**. It's an MMC3 clone with custom CHR
/// banking both for protection purposes and to access 512 KiB of CHR-ROM. **PRG banking, mirroring
/// and scanline IRQ work as on a normal MMC3.**"
///
/// # The four extra registers
///
/// "There are four extra registers that are filled by writing or reading (!) from the CPU
/// `$6000-$7FFF` address range **when PRG-RAM is enabled in the MMC3's `A001` register**."
///
/// ```text
/// Extra Index Register ($6000-$7FFF, write)      Extra Data Register #0 ($6000-$7FFF, read)
///  D~[ABR. ....]                                  D~[XXXX XXXX]
///     ||+-------- Select Extra Data Register         ++++-++++- XOR value for the 2 KiB CHR bank
///     ||          #0 or #1 on next read                        at PPU $0000-$07FF
///     ++--------- Always 1
///                                                Extra Data Register #1 ($6000-$7FFF, read)
/// Latch Register (write)                          D~[.BAA AAAA]
///  the datum of the last CPU write to **any**          |++-++++- 4 KiB CHR bank at $1000-$1FFF
///  address, zero page included                         |         (CHR A17..A12)
///                                                      +-------- Upper bit of the 2 KiB bank at
///                                                                $0800-$0FFF (CHR A18)
/// ```
///
/// `R` is bit **5**, `B` bit 6 and `A` bit 7: the annotation's two `+` columns sit under `A` and
/// `B` and the `+` of the `R` line under `R`. "It's hypothesized that clearing the A and B bits
/// may switch the two CHR pattern table halves to normal MMC3 CHR banking, but this has not yet
/// been verified on real hardware. The game keeps both bits enabled at all times." So this build
/// latches all three bits, acts on `R` alone and leaves the hypothesis alone — recorded in Q-172,
/// with a test that pins that clearing A and B changes nothing.
///
/// # The effective CHR banks, which is the whole board
///
/// ```text
/// PPU $0000-$07FF  A10 = PPU A10;  A11-A17 = Extra0 bits 0-6 XOR MMC3 A11-A17;  A18 = Extra0 b7
///                  chrBank2K = extraRegister[0] ^ (mmc3Register[0] >> 1);
///                  "when Bit 7 in the MMC3's index register is set, MMC3 registers 2/3 … apply"
/// PPU $0800-$0FFF  A10 = PPU A10;  A11-A17 = MMC3 A11-A17;  A18 = Extra1 bit 6
///                  chrBank2K = ((extraRegister[1] << 1) & 0x80) ^ (mmc3Register[1] >> 1);
///                  "… MMC3 registers 4/5 instead of register 1 apply"
/// PPU $1000-$1FFF  A10-A11 = PPU A10-A11;  A12-A17 = Extra1 bits 0-5;  A18 = always 0
///                  chrBank4K = extraRegister[1] & 0x3F;
/// ```
///
/// Two things fall out of reading it as *address lines* rather than as three bank numbers, and
/// both are what [`Mmc3Dragonfighter::apply_chr`] computes:
///
/// * **`CHR A10 = PPU A10` means the chip's own bank bit 0 is a don't-care** in the two 2 KiB
///   regions. The 1 KiB window inside each of them is the PPU's address line, so the pair is a
///   true 2 KiB window whatever odd bank number the chip put there.
/// * **the note about `$8000` bit 7 needs no code of its own.** With the MMC3's CHR inversion set,
///   what the chip drives at PPU `$0000-$07FF` *is* its registers 2 and 3 — that is what inversion
///   means — so taking the chip's own per-window 1 KiB bank number and shifting it right by one
///   gives "register 0" with the bit clear and "registers 2/3" with it set, which is exactly what
///   the two notes ask for. [`Mmc3Board::chr_bank_of_window`] is that number.
///
/// [`Mmc3Board::chr_bank_of_window`]: super::Mmc3Board
///
/// # The latch is a new hook, and §7.2's "no new hook" is the one thing its page overrides
///
/// `docs/review/UNL-census.md` §7.2 says of this step that "`Cartridge::cpu_read` already reaches
/// that window, so no new hook". The **read** needs none —
/// [`Mapper::cpu_read_override`] is a `&mut self` hook `Cartridge::try_cpu_read` already calls
/// first thing — but the *datum* does: the page's own *Latch Register* section makes it "the data
/// of the last CPU write to **any** address" and says "the game writes the desired values to zero
/// page". A zero-page write is RAM and never reaches a cartridge in this core, and
/// `NesBus::data_bus` is not it either — that byte is set by reads as well as writes, so at the
/// instant of the `$6000` read it holds the high byte of that very address.
///
/// So the hook is [`CartridgeMemory::latch_cpu_write`], one store in
/// `NesBus::write_internal` behind `board-292`. Recorded as **Q-172**.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**, for [`super::Mmc3Scramble`]'s reason: the board *holds* an
/// [`Mmc3`] and adds its own registers. It is the **first** board of that variant to need
/// [`Mapper::cpu_read_override`], so that forward is new — `#[inline(never)]`, which is the shape
/// [`super::IrqBoard::try_read_prg`] carries for the same inlined-jump-table reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Dragonfighter {
    /// The chip itself. Every `$8000-$FFFF` write reaches it unchanged: "PRG banking, mirroring
    /// and scanline IRQ work as on a normal MMC3".
    mmc3: Mmc3,
    /// The Extra Index Register, as written: `A`, `B` and `R` in bits 7, 6 and 5.
    index: u8,
    /// Extra Data Register #0, the eight XOR bits of the 2 KiB bank at PPU `$0000-$07FF`.
    data0: u8,
    /// Extra Data Register #1: bits 5-0 the 4 KiB bank at PPU `$1000-$1FFF`, bit 6 CHR A18 of the
    /// 2 KiB bank at PPU `$0800-$0FFF`.
    data1: u8,
}

impl Mmc3Dragonfighter {
    /// The window all four registers live in, written **and** read: "the CPU `$6000-$7FFF` address
    /// range".
    const WINDOW_FIRST: u16 = 0x6000;
    const WINDOW_LAST: u16 = 0x7FFF;

    /// The Extra Index Register's `R`: "Select Extra Data Register #0 or #1 on next
    /// `$6000-$7FFF` read". Bit **5**, the annotation's `||+` landing under the third character of
    /// `ABR.`.
    const INDEX_SELECT: u8 = 0x20;

    /// Extra Data Register #1 bit 6: CHR A18 of the 2 KiB bank at PPU `$0800-$0FFF`.
    const DATA1_CHR_A18: u8 = 0x40;

    /// Extra Data Register #1 bits 5-0: the 4 KiB bank at PPU `$1000-$1FFF`, "CHR A17..A12".
    const DATA1_BANK: u8 = 0x3F;

    /// Whether `address` is the board's window. `A` and `B` of the index register ("Always 1")
    /// are latched and acted on by nothing, so the decode is the range and the `$A001` enable.
    const fn in_window(address: u16) -> bool {
        address >= Self::WINDOW_FIRST && address <= Self::WINDOW_LAST
    }

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let mut board = Self {
            // 292 and not 4, the tightening every MMC3 clone here makes.
            mmc3: Mmc3::new(memory, mapper),
            index: 0,
            data0: 0,
            data1: 0,
        };
        board.refresh();
        board
    }

    /// Recomputes the MMC3's windows from its own registers and then **replaces** the CHR half of
    /// them with what the page's three blocks compute.
    ///
    /// Called after every write and every register-filling read, on [`super::Somari::refresh`]'s
    /// own idempotence argument: the chip keeps its registers and is asked to recompute from them,
    /// so the overlay never sees a window it has already written.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        self.apply_chr();
    }

    /// The page's three *Effective CHR banks* blocks, one per region, written as the eight 1 KiB
    /// windows this core banks in.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and every board of this family
    /// carries: this runs from one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches, and from one arm of `Board::cpu_read_override`.
    #[inline(never)]
    fn apply_chr(&mut self) {
        // The chip's own 1 KiB bank per window, taken before anything is written: with the CHR
        // inversion set these are registers 2-5 at $0000-$0FFF, which is what the page's two notes
        // ask for.
        let chip: [u8; BankWindows::CHR_WINDOWS] =
            std::array::from_fn(|window| self.mmc3.chr_bank_of_window(window));
        let (data0, data1) = (self.data0, self.data1);
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        for (window, &chip_bank) in chip.iter().enumerate() {
            // The window index as a bank number: 0 to 7, so the conversion cannot fail.
            let index = i32::try_from(window).unwrap_or(0);
            // "CHR A10: PPU A10" in the two 2 KiB regions, "CHR A10-A11: PPU A10-A11" in the
            // 4 KiB one: the low bits of the window index are the PPU's own address lines and no
            // register reaches them.
            let bank = match window {
                // PPU $0000-$07FF: `chrBank2K = extraRegister[0] ^ (mmc3Register[0] >> 1)`.
                0 | 1 => {
                    let bank2k = i32::from(data0 ^ (chip_bank >> 1));
                    (bank2k << 1) | (index & 0x01)
                }
                // PPU $0800-$0FFF: `((extraRegister[1] << 1) & 0x80) ^ (mmc3Register[1] >> 1)`,
                // which is CHR A11-A17 from the chip and CHR A18 from Extra1 bit 6. The chip's
                // own bank is eight bits wide, so its bit 7 after the shift is zero and the
                // page's `^` is an `|`.
                2 | 3 => {
                    let bank2k =
                        i32::from(chip_bank >> 1) | (i32::from(data1 & Self::DATA1_CHR_A18) << 1);
                    (bank2k << 1) | (index & 0x01)
                }
                // PPU $1000-$1FFF: `chrBank4K = extraRegister[1] & 0x3F`, and "CHR A18: Always 0,
                // i.e. restricted to the first 256 KiB CHR-ROM half". The chip drives nothing
                // here at all.
                _ => {
                    let bank4k = i32::from(data1 & Self::DATA1_BANK);
                    (bank4k << 2) | (index & 0x03)
                }
            };
            banks.map_chr_1k(window, bank);
        }
    }

    /// Whether the board's own window answers at all: "when PRG-RAM is enabled in the MMC3's
    /// `A001` register".
    ///
    /// `registry::board_memory_sizes` gives the number **no** PRG-RAM — the window *is* the
    /// register, which is 114's, 115's, 182's, 208's and the two MMC3 multicarts' arm in that
    /// function — so nothing is behind it for the enable to expose. The enable is still read,
    /// because the page makes it the condition.
    fn registers_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled()
    }

    /// The register fill a **read** of `$6000-$7FFF` performs: "It is filled with the value that
    /// had been latched during the last CPU write to any address."
    ///
    /// `#[inline(never)]`, for [`Mmc3Dragonfighter::apply_chr`]'s reason, and here it is also what
    /// keeps the arm this board contributes to the inlined `Board::cpu_read_override` jump table
    /// to one `callq`.
    #[inline(never)]
    fn fill_data_register(&mut self, memory: &CartridgeMemory) {
        let latched = memory.cpu_write_latch();
        if self.index & Self::INDEX_SELECT == 0 {
            self.data0 = latched;
        } else {
            self.data1 = latched;
        }

        self.refresh();
    }
}

impl Mapper for Mmc3Dragonfighter {
    fn mapper_number(&self) -> i32 {
        292
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's three register bytes behind it. Nothing of
    /// the MMC3's own layout moves and no earlier state can name this number (REQ-CORE-090).
    ///
    /// The **latch** is not here: it lives on [`CartridgeMemory`] and is a transient bus byte the
    /// next CPU write overwrites, which that field's own note argues. What has to survive a round
    /// trip is the two data registers, and they do.
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.index);
        writer.write_u8(self.data0);
        writer.write_u8(self.data1);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        self.index = reader.read_u8()?;
        self.data0 = reader.read_u8()?;
        self.data1 = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.index = 0;
        self.data0 = 0;
        self.data1 = 0;
        self.refresh();
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

    /// The read that fills a data register. `Some(None)` is "this board decoded the address and
    /// drives nothing on the bus", which is the right answer for a window with no RAM behind it:
    /// the caller keeps its open-bus value.
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !Self::in_window(address) || !self.registers_enabled() {
            return None;
        }

        self.fill_data_register(memory);
        Some(None)
    }

    /// The side-effect-free counterpart: the same decode, and **no fill**, so that a host
    /// accessor or a Lua script peeking at `$6000` cannot move a CHR bank under the game's feet
    /// (REQ-CORE-080, the reason this method exists at all).
    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        (Self::in_window(address) && self.registers_enabled()).then_some(None)
    }

    /// The Extra Index Register, and the chip for everything else.
    ///
    /// **The `address < 0x8000` guard is deliberately not the first line**, which is
    /// [`super::Mmc3Txc189`]'s own note: the index register is at `$6000-$7FFF`, a range
    /// `Cartridge::cpu_write` reaches whenever the cartridge has no PRG-RAM, so a board that
    /// returned early below `$8000` could never select a data register at all.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            if Self::in_window(address) && self.registers_enabled() {
                self.index = value;
            }

            return;
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: this board's CHR fetches clock the
    /// A12 counter.
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
