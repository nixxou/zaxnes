use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mappers **12** and **14**, the **Huang-1** ASIC's other two PCBs (REQ-CORE-102,
/// MF-UNL-8 steps 1 and 2).
///
/// The chip is mapper 116's — nesdev ("INES Mapper 012"): "a chip-on-board variant of the same
/// Huang-1 ASIC as mappers 14, 116 and 238 plus GAL16V8 programmable logic" — but neither of
/// these two boards is mapper 116's board, and the two are not each other's either. What they
/// share is the chip and one mechanism: **an outer CHR bank line, CHR A18, driven per PPU window
/// from a register the ASIC does not hold.** That is the whole of this type.
///
/// | | **12**, the Gouder SL-5020B | **14**, the Gouder SL-1632 (`UNL-SL1632`) |
/// | --- | --- | --- |
/// | the outer register | `$4132`, mask `$E100`, outside the ASIC | `$A131`, mask `$FFFF?`, **over the sub-board's own registers** |
/// | modes | **one**: "The Huang-1 ASIC functions exclusively as an MMC3A clone on this PCB" | **two**: MMC3 and VRC2 (no MMC1 mode) |
/// | CHR A18 bits | **two**, one per pattern table (bits 0 and 4) | **three**, over unequal window groups (bits 3, 5 and 7) |
/// | the counter | the MMC3's, **revision A** — "MMC3A-specific IRQ behavior is necessary for running *Dragon Ball Z 5*" | the MMC3's, as it comes |
/// | one title each | *Dragon Ball Z 5* (Rex Soft) | *Samurai Spirits*, the 8-character version (Rex Soft) |
///
/// # Why this is not [`super::Somari`] with a parameter
///
/// `docs/review/UNL-census.md` §7.2 files mapper 12 as "`Somari` with one more register", which
/// would make it the same three-chip delegation with a supervisor register. **Mapper 12's own page
/// says the opposite in one sentence** — the ASIC "functions exclusively as an MMC3A clone on this
/// PCB" — so that board has no supervisor register, no VRC2 mode and no MMC1 mode at all, and
/// mapper 14's page gives *its* board two modes and not three. MF-UNL-2's rule is that the
/// number's own page wins; **Q-164** records both cases.
///
/// So what is reused here is the **sub-chips** and not the board: an [`Mmc3`] through the
/// `pub(crate)` accessors mapper 116 already needed, and for mapper 14 a [`super::Vrc2Vrc4`] in the
/// VRC2b configuration through the constructor it already had. [`super::Somari`] is not touched by
/// a line — with `board-huang1` off every file of `board-116` is byte for byte what this lot's base
/// commit shipped, and with it on mapper 116's struct, its save-state block and its hot path are
/// unchanged.
///
/// # Where the board sits
///
/// In [`super::IrqBoard`], with no new `Board` discriminant: the board holds an `Mmc3` whose
/// counter is the chip's, so it answers **yes** to [`Mapper::irq_asserted`], which is that enum's
/// entry condition. The outer register of both numbers is **below `$8000`** — `$4132` on one and a
/// write the sub-board also sees on the other — so [`Mapper::write_register`]'s usual
/// `address < 0x8000` guard must not be the first line of this board, exactly as on mapper 116.
///
/// # The outer CHR bank, and why `BankWindows` needed one more method
///
/// MF-6b step 6 predicted it: "`BankWindows` has no way to express a 1 KiB window table whose A18
/// changes at `$1000` and again at `$1800` other than by computing it per window". Mapper 116's
/// [`BankWindows::offset_chr_banks`] adds one bank number to all eight windows and cannot say
/// that; [`BankWindows::or_chr_banks_each`] takes the eight values. It is an **OR** and not an
/// addend because both pages call the thing an address line, which is also what makes the call
/// idempotent — a window's bank may already carry the bit, so the board may re-apply it after
/// every register write without remembering what it last laid down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Huang1 {
    /// The only mode of mapper 12 and one of mapper 14's two.
    mmc3: Mmc3,
    /// `$4132` on mapper 12 and `$A131` on mapper 14. Both pages give it one byte and both use
    /// only some of its bits; what is latched is the whole byte, so a save state carries what a
    /// game wrote.
    supervisor: u8,
    /// 12 or 14.
    mapper_number: i32,
}

impl Huang1 {
    /// Mapper 12's outer register: "$4132", with "Mask: $E100" — A15, A14, A13 and A8 and
    /// nothing else, so the register repeats through the whole of `$4100`-`$5FFF` wherever A8 is
    /// set. `$6100 & $E100` is `$6100`, so **no address in $6000-$7FFF reaches it** and the
    /// board's PRG-RAM window is the header's; see `registry::board_memory_sizes`.
    const OUTER_MASK_12: u16 = 0xE100;
    const OUTER_ADDRESS_12: u16 = 0x4100;

    /// CHR A18 as a 1 KiB bank number: bit 0 of such a number is CHR A10, so A18 is bit 8.
    const CHR_A18: i32 = 1 << 8;

    /// What a read of mapper 12's `$4132` drives.
    ///
    /// nesdev ("INES Mapper 012"): "*Dragon Ball Z 5* reads the same address to select between
    /// Chinese and English text", `.... ...A`, and "It was previously thought that the value of
    /// this bit reflects a jumper or solder pad. Pictures of the circuit board however show no
    /// sign of either. Either such a solder pad is hidden underneath the GAL16V8, or the GAL16V8
    /// has the value hard-wired (to Chinese text in all known copies)."
    ///
    /// The page does **not** say which value is Chinese, so the figure is a choice and **Q-164**
    /// records it: zero, which is what the one dump of the one game that reads it is reported to
    /// show. Bits 7-1 are open bus on the hardware and zero here, which is this build's answer for
    /// every such read (the Sachen TCA-01's `[xxii iiii]` is the precedent).
    const LANGUAGE_BIT: u8 = 0x00;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            // The mapper number is this board's and not 4: `Mmc3::new` reads a submapper only
            // when the number is 4, which is MF-6a3 step 1's tightening, so neither of these
            // numbers can be read as an MMC6, an MMC3C with hard-wired mirroring or an MMC3A
            // whatever submapper byte an image carries. Mapper 12's own submapper 1 is the FFE
            // Magic Card 4M and is refused at load; see `Huang1::new_12`'s caller in the registry.
            mmc3: Mmc3::new(memory, mapper_number),
            // The page gives no power-on value for either register. **Q-164**: zero, so a board
            // powers up with no outer CHR bank — on mapper 12 that is the first 256 KiB of CHR,
            // and on mapper 14 it is also VRC2 mode, which `Huang1::mode_is_mmc3` reads the other
            // way round from mapper 116's documented `$01`; see that method.
            supervisor: 0,
            mapper_number,
        };

        if mapper_number == 12 {
            // "The Huang-1 ASIC functions exclusively as an MMC3A clone on this PCB", and
            // "MMC3A-specific IRQ behavior is necessary for running *Dragon Ball Z 5*". Mapper 4
            // reaches the revision A counter through NES 2.0 submapper 4; this board is not
            // mapper 4 and carries no such submapper, so the board that holds the chip sets it at
            // load — mappers 114 and 268's own precedent.
            board.mmc3.force_irq_revision_a();
        }

        board.refresh();
        board
    }

    /// CHR A18 for each of the eight 1 KiB windows, from the outer register.
    ///
    /// Mapper 12, `$4132`, `...B ...A` — bit 0 is "CHR A18 when PPU A12=0" and **bit 4** "CHR A18
    /// when PPU A12=1", so one line per pattern table and the two groups are equal.
    fn chr_a18_windows(&self) -> [i32; BankWindows::CHR_WINDOWS] {
        let line = |bit: u8| {
            if self.supervisor & bit == 0 {
                0
            } else {
                Self::CHR_A18
            }
        };
        let (low, high) = (line(0x01), line(0x10));
        [low, low, low, low, high, high, high, high]
    }

    /// Recomputes the active chip's windows from its own registers and lays the outer CHR bank
    /// over them.
    ///
    /// Called after **every** write the board takes, which is what makes
    /// [`BankWindows::or_chr_banks_each`]'s idempotence load-bearing: the windows that go into it
    /// are always the chip's own, so no write can see a bank that already carries the line from a
    /// previous call and no board state remembers what was laid down.
    ///
    /// The page's "Because this register is implemented outside the ASIC, it is not affected by
    /// MMC3 $8000 bit 7, and writes take effect immediately" is exactly this shape: $8000 bit 7
    /// swaps which registers drive which windows *inside* the chip, and the line is applied to the
    /// windows afterwards whatever that swap did. **FCEUX's reading is deliberately not
    /// implemented** — it "emulates it as a temporary latch register that takes effect on the next
    /// write to an MMC3 CHR bank register", and the page's own next sentence is that "the homebrew
    /// hack *Ultimate Mortal Kombat III* relies on FCEUX' behavior and would therefore not work on
    /// the original PCB". This build is the PCB (**Q-164**).
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        let lines = self.chr_a18_windows();
        if let Some(banks) = self.mmc3.banks_mut() {
            banks.or_chr_banks_each(lines);
        }
    }

    /// Whether `address` is mapper 12's outer register, under the page's own mask.
    const fn is_outer_register(&self, address: u16) -> bool {
        self.mapper_number == 12 && address & Self::OUTER_MASK_12 == Self::OUTER_ADDRESS_12
    }
}

impl Mapper for Huang1 {
    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The whole mapper state: the **outer register first**, then the chip's own complete block.
    ///
    /// [`Mapper::save_state`] and [`Mapper::load_state`] are overridden rather than
    /// `save_registers`/`load_registers`, on [`super::Somari`]'s own reason read for one chip: the
    /// defaults write `banks()` and then the registers, and the windows this board's `banks()`
    /// shows **carry the outer CHR line** while the chip's registers do not. `Mmc3`'s own
    /// `load_registers` recomputes the windows from its registers, so the line has to be laid back
    /// over them after it — which is the one line [`Huang1::load_state`] adds and which no default
    /// could have (REQ-CORE-090, REQ-CORE-091).
    fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.supervisor);
        self.mmc3.save_state(writer);
    }

    fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.supervisor = reader.read_u8()?;
        self.mmc3.load_state(reader)?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.supervisor = 0;
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

    /// Mapper 12's language bit, and PRG otherwise.
    ///
    /// The register is at `$4132`, which `Cartridge::default_cpu_read` reaches through
    /// [`Mapper::try_read_prg`] for all of `$4020`-`$FFFF`, so this is where the read side of the
    /// GAL goes; see [`Huang1::LANGUAGE_BIT`].
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if self.is_outer_register(address) {
            return Some(Self::LANGUAGE_BIT);
        }

        self.mmc3.try_read_prg(memory, address)
    }

    /// **The `address < 0x8000` guard is deliberately not the first line**, for mapper 116's
    /// reason: mapper 12's outer register is at `$4132`, which `Cartridge::cpu_write` reaches for
    /// all of `$4020`-`$FFFF`, so a board that returned early below `$8000` could never switch its
    /// CHR half at all.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.is_outer_register(address) {
            self.supervisor = value;
            self.refresh();
            return;
        }

        if address < 0x8000 {
            return;
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc3.ppu_write(memory, address, value);
    }

    /// The chip's own A12 counter, through the chip's own filter.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
