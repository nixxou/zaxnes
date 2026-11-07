use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **189**: an MMC3 clone whose PRG-ROM is banked in **32 KiB** amounts by a register
/// at `$4100`, "like [`AxROM`], [`BNROM`] and [`GNROM`]" (MF-UNL-2 step 5, REQ-CORE-102).
///
/// [`AxROM`]: super::AxRom
/// [`BNROM`]: super::Mapper034
/// [`GNROM`]: super::GxRom
///
/// nesdev ("INES Mapper 189") names two games and their many variants:
///
/// * *Thunder Warrior — 轰天至尊* (TXC, **01-22017-000**, 60-pin) and *Gluk the Thunder Warrior*
///   (**01-22018-400**, 72-pin), which use "the 28-pin version of the PT8154 ASIC, … a
///   stripped-down MMC3 **without any IRQ functionality**";
/// * *Street Fighter II: The World Warrior* (Yoko Soft, **FC-001EM**/**FC-001EMC**) and its
///   variants *Super Fighter II´*, *Master Fighter II*, *Master Fighter III* and *Mario Fighter
///   III*, which "need[] a fully functional MMC3 clone, as the IRQ feature is utilized".
///
/// # The register
///
/// > ```text
/// > D~[..PP ....] A~[010. ...1 .... ....] (Address & $E100)==$4100
/// >      ++- PRG A16..15
/// > ```
/// > The original board variants switch four 32 KiB PRG-ROM banks based on bits 4 and 5 on
/// > writes to CPU `$4100-$5FFF` (address bit 9 must be set).
///
/// and, for the **oversize** board (`自動車床控制板 MACP 811023`, used only by the 256 KiB hack
/// *Mario Fighter III*), `D~[.PPP ....]` — one more bit, PRG A17..A15.
///
/// **"Address bit 9" is A8, the ninth line**, and the page's own two statements say so: its mask
/// `$E100` reads A15, A14, A13 and **A8**, its address diagram sets the bit in the `...1` nibble
/// that is bits 11-8, and the oversize game "still writes to the original `$4132` address" —
/// `$4132` has A8 set and A9 **clear**. So the decode is `(address & $E100) == $4100` and nothing
/// else, which is what [`Mmc3Txc189::REGISTER_MASK`] holds.
///
/// Three bits of the written byte are taken, not two: the oversize board's `[.PPP ....]` is the
/// original's `[..PP ....]` with A17 added above it, and on a 128 KiB image that bit wraps away
/// — [`BankWindows::map_prg_32k`] wraps the bank modulo the image's. One decode therefore runs
/// both boards, which is what the page's own *Generalized definition for emulation* recommends
/// when the generic pirate variants are to be left out: "respond to writes to `$4100-$5FFF` with
/// address bit 9 set, and only use the upper nibble of the data byte being written".
///
/// # The generic pirate boards are deliberately not built
///
/// The page lists two more decodes — **A-37**/**810809-C** and **K-3086**/**810836-C** — which
/// "connect the PRG-ROM Bank Register to the MMC3's PRG-RAM interface" at `$6000`, one reading
/// the bank from data bits 4-5 and the other from *address* bits 0-1. Its own paragraph is the
/// reason they are left out: "There are countless pirated variants of the (already unauthorized)
/// *Street Fighter II* 8-bit backport. … doing so requires **modifying the PRG-ROM content** to
/// change the write address". They are hacks of a hack, they are not the boards
/// `docs/review/UNL-census.md` §3.1 lists the number's titles for, and running them would mean
/// decoding the whole of `$4020-$7FFF` and ORing the two nibbles of every byte written there —
/// which is what Mesen 2's `MMC3_189` does (`RegisterStartAddress()` `$4120`, `(_prgReg |
/// _prgReg >> 4) & 0x07`). The same shape, and the same reason, as the 60-pin decode
/// [`super::Sachen72008`] records.
///
/// # The PT8154's missing counter is **not** a parameter
///
/// *Thunder Warrior*'s 28-pin PT8154 has no IRQ at all and *Street Fighter II*'s 40-pin pair
/// does. **Nothing in an image distinguishes them**: the page gives the number no submapper, and
/// the two boards differ in pin count rather than in anything a header records. So the counter is
/// built — the MMC3's, unchanged — and a *Thunder Warrior* dump simply never writes `$C000-$E001`,
/// which is the harmless direction of the two. Recorded here rather than guessed at with a
/// heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Txc189 {
    /// The chip itself. Every `$8000-$FFFF` register reaches it unchanged; only PRG leaves.
    mmc3: Mmc3,
    /// The 32 KiB PRG bank, three bits: PRG A17..A15.
    bank: u8,
}

impl Mmc3Txc189 {
    /// `(address & $E100) == $4100`: A15, A14, A13 and **A8**. See the type's own note on why
    /// the page's "address bit 9" is A8.
    const REGISTER_MASK: u16 = 0xE100;
    const REGISTER_ADDRESS: u16 = 0x4100;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            // 189 and not 4, for the reason `Mmc3Kof97::new` gives.
            mmc3: Mmc3::new(memory, 189),
            bank: 0,
        };
        board.refresh();
        board
    }

    /// Recomputes the MMC3's windows and lays the 32 KiB bank over the whole of `$8000-$FFFF`.
    ///
    /// The MMC3's own PRG registers are **gone** from the bus: the number exists because the
    /// board "bank[s] PRG-ROM in 32 KiB amounts", so the chip's PRG bank lines are not connected
    /// and `R6`/`R7` reach nothing. Its CHR banking, its mirroring and its counter are
    /// untouched, which is what "over a stock MMC3" means. `refresh` is called after every write
    /// the board takes, on [`super::Somari::refresh`]'s own idempotence argument.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        let bank = i32::from(self.bank);
        if let Some(banks) = self.mmc3.banks_mut() {
            banks.map_prg_32k(bank);
        }
    }

    /// A write to the bank register.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and [`super::Somari`]'s
    /// `write_supervisor` carries: this is one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches.
    #[inline(never)]
    fn write_bank(&mut self, value: u8) {
        // `[.PPP ....]`, PRG A17..A15 -- the oversize board's three bits, of which the original
        // board drives two and the third wraps away on its 128 KiB.
        self.bank = (value >> 4) & 0x07;
        self.refresh();
    }
}

impl Mapper for Mmc3Txc189 {
    fn mapper_number(&self) -> i32 {
        189
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.bank);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        self.bank = reader.read_u8()?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.bank = 0;
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

    /// **The `address < 0x8000` guard is deliberately not the first line**, which is
    /// [`super::Somari::write_register`]'s own note: the register is at `$4100`, a range
    /// `Cartridge::cpu_write` reaches, so a board that returned early below `$8000` could never
    /// be banked at all.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & Self::REGISTER_MASK == Self::REGISTER_ADDRESS {
            self.write_bank(value);
            return;
        }

        if address < 0x8000 {
            return;
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give.
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
