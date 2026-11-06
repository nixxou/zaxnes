use super::{Mapper, Mmc1, Mmc3, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 116, the Gouder **SOMARI-P** board and its Huang ASIC (REQ-CORE-101 optional,
/// REQ-CORE-102, Q-130 question 1, Q-132).
///
/// nesdev ("INES Mapper 116"): the ASIC "contains three different mapper implementations" — a
/// VRC2, an MMC3 and an MMC1 — with a **supervisor register** choosing which one answers, and
/// "three chips each keeping its own registers, so that when switching between modes using the
/// supervisor register, the entire mapper state changes, including PRG/CHR-ROM switchable and
/// fixed banks as well as IRQ state". So this board *holds* the three chips rather than
/// reimplementing any of them, which is [`super::NesEvent`]'s shape read three times over:
/// about 550 bytes of sub-board, against the 1 376 of `Board`'s largest variant, so the enum
/// cannot grow.
///
/// # The supervisor register
///
/// `$4100`, decoded as `(address & $E100) == $4100`, holding `[.... .CMM]`:
///
/// | field | meaning |
/// | --- | --- |
/// | `MM` = 0 | the VRC2 answers |
/// | `MM` = 1 | the MMC3 answers |
/// | `MM` = 2 or 3 | the MMC1 answers ("mode 3 = mode 2") |
/// | `C`, **bit 2** | the outer 256 KiB CHR-ROM bank, CHR A18 |
///
/// The page's own diagram puts `C` at bit 2 and Mesen agrees (`(_mode & 0x04) << 6`); a summary
/// of that page which says bit 3 is wrong. **Power-on is `$01`, MMC3 mode.**
///
/// The register is **below `$8000`**, which `Cartridge::cpu_write` reaches for all of
/// `$4020-$FFFF`, so [`Mapper::write_register`]'s usual `address < 0x8000` guard must *not* be
/// the first line of this board — and it is not; see that method.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], with no new `Board` discriminant, for the reason MF-6a3 step 1
/// measured: two more discriminants cost `mmc5` **+0.080 %** through the jump table of
/// `Board::expansion_audio_output`, which `NesBus::push_expansion_audio` inlines whole. This
/// board qualifies for that enum because its MMC3 mode asserts /IRQ — and **only** its MMC3
/// mode does, which the brief and Mesen agree on (Mesen gates the whole counter on
/// `(_mode & 0x03) == 1`), so `irq_asserted` and `on_ppu_address_bus` are mode-gated here.
///
/// # The four submappers
///
/// 1. **Submapper 0, the SOMARI-P PAL**, and **submapper 1, the SOMARI-W**: the `-W` PAL "resets
///    the MMC1's serial shift register when switching *to* MMC1 mode, equivalent to writing to
///    $8000-$FFFF with bit 7 set" and the `-P` does not. Every single-game cartridge is submapper
///    0 except the GK2 *Somari*, "and that game does not seem to rely on it" — so the reading
///    that puts the reset on submapper 1 is **not observable by any game in the sources**, and it
///    is the reading here. [`Mmc1::write_register`] already implements that reset as its bit-7
///    arm, so the switch drives it rather than reimplementing it.
/// 2. **Submapper 2, the Huang-2** (Q-132), "addresses 8 KiB rather than 16 KiB PRG-ROM banks, meaning
///    that the PRG bank register value is shifted by one bit to the left compared to what one
///    would write to a normal MMC1". That cannot go through [`Mmc1`]'s own `update_banks`, whose
///    windows are 16 KiB — so the board lays PRG out **on top of** it, which is the move MF-6b
///    step 3 already made for the NES-EVENT. See [`Somari::refresh_mmc1`], which also records
///    the one reading of that sentence this board had to choose between (**Q-132**).
/// 3. **Submapper 3** (Q-132) is "a reset-based multicart with the first game having 256 KiB of PRG- and
///    CHR-ROM each, the remaining ones 128 KiB", and the page **says nothing about how a reset
///    selects the game** — not the counter's width, not which address lines it drives, not
///    whether it wraps. Mesen does not implement it at all. **Q-132**: it loads as a Huang-1
///    with no reset-based outer bank, so the first game runs and the rest are unreachable, which
///    is strictly better than guessing an outer-bank rule and silently running the wrong game.
///
/// # The outer CHR bank
///
/// It cannot be expressed through the sub-boards, whose `update_banks` write **absolute** bank
/// numbers, so [`BankWindows::offset_chr_banks`] adds it to the active chip's windows after
/// every update. That method recovers each bank from its own offset, so re-applying it is
/// idempotent and the board does not have to remember what it last added. **The MMC1 mode
/// ignores the outer bank** (Mesen's `UpdateChr` case 2/3 does not apply it).
///
/// The titles are unlicensed: *Somari the Adventurer* (the Sonic port the board is named for),
/// *AV Mei Shao Nv Zhan Shi* and four more. There is **no image of this board under
/// `assets-free/test-roms`** (§2 of `docs/review/MF-6-inventory.md`), so nothing in the campaign
/// exercises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Somari {
    /// `MM` = 1, and the only mode with an IRQ.
    mmc3: Mmc3,
    /// `MM` = 2 or 3.
    mmc1: Mmc1,
    /// `MM` = 0, a **VRC2b**: see [`Vrc2Vrc4::new_vrc2b`].
    vrc2: Vrc2Vrc4,
    /// The supervisor register at `$4100`, `[.... .CMM]`, three bits wide.
    supervisor: u8,
    /// 0 (SOMARI-P), 1 (SOMARI-W), 2 (Huang-2) or 3 (the reset-based multicart, Q-132). See the
    /// type's own note.
    submapper: i32,
}

impl Somari {
    /// `$4100`'s decode: "Mask: $E100", so A15, A14, A13 and A8 alone.
    const SUPERVISOR_MASK: u16 = 0xE100;
    const SUPERVISOR_ADDRESS: u16 = 0x4100;

    /// The power-on supervisor register, **MMC3 mode**.
    const POWER_ON: u8 = 0x01;

    const MODE_VRC2: u8 = 0;
    const MODE_MMC3: u8 = 1;

    /// 1 KiB banks in the outer CHR bank `C` selects: 256 KiB.
    const OUTER_CHR_BANKS: i32 = 256;

    /// The VRC2's CHR registers at power-on; see [`Vrc2Vrc4::force_chr_registers`].
    const VRC2_POWER_ON_CHR: i32 = 0xFF;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let mut board = Self {
            // 116 and not 4: `Mmc3Board::new` reads a submapper only when the mapper number is
            // 4, which is the tightening MF-6a3 step 1 made for the TQROM and the two
            // multicarts, so this cannot be read as an MMC6 or an MMC3A whatever a mapper-116
            // image's submapper byte holds.
            mmc3: Mmc3::new(memory, 116),
            mmc1: Mmc1::new(memory),
            vrc2: Vrc2Vrc4::new_vrc2b(memory, 116),
            supervisor: Self::POWER_ON,
            submapper: if header.is_nes20 { header.submapper } else { 0 },
        };
        board.vrc2.force_chr_registers(Self::VRC2_POWER_ON_CHR);
        board.refresh();
        board
    }

    /// `MM` of the supervisor register, with 3 folded onto 2.
    const fn mode(&self) -> u8 {
        self.supervisor & 0x03
    }

    /// The outer CHR bank in 1 KiB units: `C` is bit 2 and selects a 256 KiB half.
    const fn outer_chr(&self) -> i32 {
        if self.supervisor & 0x04 == 0 {
            0
        } else {
            Self::OUTER_CHR_BANKS
        }
    }

    /// Recomputes the active chip's windows from its own registers and lays the outer CHR bank
    /// over them.
    ///
    /// Called after **every** write the board takes, which is what makes the three chips look
    /// like three whole mappers: each keeps its own registers and the one that answers is asked
    /// to recompute from them, so switching mode moves PRG, CHR, the fixed windows and the IRQ
    /// state together.
    ///
    /// Recomputing before adding the bank is what makes the call **idempotent**: the windows
    /// that go into [`BankWindows::offset_chr_banks`] never already carry an offset, so no write
    /// can count the bank twice. The MMC1 mode is a no-op here — it ignores the outer bank, and
    /// its own `update_banks` needs the memory (its mirroring register and its 512 KiB
    /// outer-PRG test), so it is driven from [`Somari::refresh_with_memory`] instead.
    fn refresh(&mut self) {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.refresh_banks(),
            Self::MODE_MMC3 => self.mmc3.refresh_banks(),
            _ => return,
        }

        let outer = self.outer_chr();
        if let Some(banks) = self.banks_mut() {
            banks.offset_chr_banks(outer);
        }
    }

    /// [`Somari::refresh`] for the paths that hold the memory, which is every one but a loaded
    /// state: only the MMC1 mode needs it.
    fn refresh_with_memory(&mut self, memory: &mut CartridgeMemory) {
        if self.mode() >= 2 {
            self.refresh_mmc1(memory);
        } else {
            self.refresh();
        }
    }

    /// The MMC1 mode, and submapper 2's one departure from it.
    ///
    /// **Q-132, second item.** The Huang-2's sentence is "the PRG bank register value is shifted
    /// by one bit to the left compared to what one would write to a normal MMC1", and it admits
    /// two readings: the **literal** one, that the board recovers the MMC1's own value as
    /// `register >> 1` and then lays out exactly as the MMC1 would — which is what is
    /// implemented here, because it is the sentence and nothing more — and a **wider** one, that
    /// 8 KiB banks are genuinely addressable so an odd register gives an *unaligned* 16 KiB view
    /// (`map_prg_8k(slot, r)` and `map_prg_8k(slot + 1, r + 1)` where a normal MMC1 maps `2r`
    /// and `2r + 1`). The page's "addresses 8 KiB rather than 16 KiB PRG-ROM banks" leans the
    /// second way; its own formula is the first. No image of submapper 2 is available to settle
    /// it, so the ticket records both and this takes the formula.
    fn refresh_mmc1(&mut self, memory: &mut CartridgeMemory) {
        self.mmc1.refresh_banks(memory);
        if self.submapper != 2 {
            return;
        }

        let control = self.mmc1.control_register();
        let bank = i32::from(self.mmc1.prg_register() & 0x0F) >> 1;
        let windows = self.mmc1.windows_mut();
        match (control >> 2) & 0x03 {
            // 32 KiB at $8000, the low bit of the bank number ignored, as on an MMC1.
            0 | 1 => windows.map_prg_32k(bank >> 1),
            // The first bank fixed at $8000, switchable at $C000.
            2 => {
                windows.map_prg_16k(0, 0);
                windows.map_prg_16k(2, bank);
            }
            // Switchable at $8000, the last bank fixed at $C000.
            _ => {
                windows.map_prg_16k(0, bank);
                windows.map_prg_16k(2, -1);
            }
        }
    }

    /// A write to the supervisor register at `$4100`.
    ///
    /// `#[inline(never)]`: this is one arm of a `Board::write_register` dispatch that
    /// `NesBus::write_internal` reaches, and MF-6a2 step 3 measured what an unguarded body costs
    /// there. The one cartridge of this board pays a `callq` per supervisor write and nothing
    /// else does.
    #[inline(never)]
    fn write_supervisor(&mut self, memory: &mut CartridgeMemory, value: u8) {
        let was_mmc1 = self.mode() >= 2;
        self.supervisor = value & 0x07;

        // Submapper 1, the SOMARI-W PAL: switching *to* MMC1 mode "resets the MMC1's serial
        // shift register ... equivalent to writing to $8000-$FFFF with bit 7 set". The chip's
        // own bit-7 arm is driven rather than reimplemented, and MR-1-1's rule applies to it
        // unchanged — "the bit 7 reset is never ignored", whatever the consecutive-write test
        // says.
        if self.submapper == 1 && !was_mmc1 && self.mode() >= 2 {
            self.mmc1.write_register(memory, 0x8000, 0x80);
        }

        self.refresh_with_memory(memory);
    }
}

impl Mapper for Somari {
    fn mapper_number(&self) -> i32 {
        116
    }

    fn banks(&self) -> Option<&BankWindows> {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.banks(),
            Self::MODE_MMC3 => self.mmc3.banks(),
            _ => self.mmc1.banks(),
        }
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.banks_mut(),
            Self::MODE_MMC3 => self.mmc3.banks_mut(),
            _ => self.mmc1.banks_mut(),
        }
    }

    /// The whole mapper state: the **supervisor register first**, then all three chips'
    /// complete blocks. The page requires each chip to keep its own registers across a mode
    /// switch, so all three are written whatever the mode is (REQ-CORE-090).
    ///
    /// **[`Mapper::save_state`] and [`Mapper::load_state`] are overridden rather than
    /// `save_registers`/`load_registers`, and the order is the reason.** The defaults write
    /// `banks()` and then the registers, and `banks()` on this board is *whichever chip the
    /// current mode names* — so a state restored into a board sitting in a different mode would
    /// have loaded one chip's windows into another's. Writing the supervisor register first and
    /// each chip's own `save_state` after it removes the ambiguity: every block has exactly one
    /// owner, and the mode is known before any of them is read.
    fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.supervisor);
        self.mmc3.save_state(writer);
        self.mmc1.save_state(writer);
        self.vrc2.save_state(writer);
    }

    fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.supervisor = reader.read_u8()?;
        self.mmc3.load_state(reader)?;
        self.mmc1.load_state(reader)?;
        self.vrc2.load_state(reader)?;
        // Both chips the outer CHR bank reaches are put back to their **register-derived**
        // windows before it is re-applied, so a state cannot come back with the bank counted
        // twice or not at all — the MMC3's own `load_registers` recomputes and the VRC2's does
        // not, and `refresh` wants them in the same state. The MMC1 ignores the bank, so its
        // restored windows are its own and are left alone.
        self.mmc3.refresh_banks();
        self.vrc2.refresh_banks();
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        self.mmc1.reset(memory);
        self.vrc2.reset(memory);
        self.vrc2.force_chr_registers(Self::VRC2_POWER_ON_CHR);
        self.supervisor = Self::POWER_ON;
        self.refresh_with_memory(memory);
    }

    /// **Only the MMC3 mode has an IRQ**: the brief says so and Mesen gates the whole counter on
    /// `(_mode & 0x03) == 1`. A VRC2 has no counter at all (that is the VRC4's), and an MMC1 has
    /// none either.
    fn irq_asserted(&self) -> bool {
        self.mode() == Self::MODE_MMC3 && self.mmc3.irq_asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.prg_ram_enabled(),
            Self::MODE_MMC3 => self.mmc3.prg_ram_enabled(),
            _ => self.mmc1.prg_ram_enabled(),
        }
    }

    fn prg_ram_writable(&self) -> bool {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.prg_ram_writable(),
            Self::MODE_MMC3 => self.mmc3.prg_ram_writable(),
            _ => self.mmc1.prg_ram_writable(),
        }
    }

    /// **The `address < 0x8000` guard is deliberately not the first line.** The supervisor
    /// register is at `$4100`, which `Cartridge::cpu_write` reaches for all of `$4020-$FFFF`, so
    /// a board that returned early below `$8000` could never be switched out of MMC3 mode.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address & Self::SUPERVISOR_MASK == Self::SUPERVISOR_ADDRESS {
            self.write_supervisor(memory, value);
            return;
        }

        if address < 0x8000 {
            return;
        }

        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.write_register(memory, address, value),
            Self::MODE_MMC3 => self.mmc3.write_register(memory, address, value),
            _ => self.mmc1.write_register(memory, address, value),
        }

        self.refresh_with_memory(memory);
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.ppu_peek(memory, address, fetch),
            Self::MODE_MMC3 => self.mmc3.ppu_peek(memory, address, fetch),
            _ => self.mmc1.ppu_peek(memory, address, fetch),
        }
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.chr_peek_view(memory),
            Self::MODE_MMC3 => self.mmc3.chr_peek_view(memory),
            _ => self.mmc1.chr_peek_view(memory),
        }
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match self.mode() {
            Self::MODE_VRC2 => self.vrc2.ppu_write(memory, address, value),
            Self::MODE_MMC3 => self.mmc3.ppu_write(memory, address, value),
            _ => self.mmc1.ppu_write(memory, address, value),
        }
    }

    /// Mode-gated with [`Somari::irq_asserted`]: the MMC3's A12 counter is the only one on this
    /// board, so in the other two modes the PPU's bus moves nothing at all.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.mode() == Self::MODE_MMC3 {
            self.mmc3.on_ppu_address_bus(memory, address, clock);
        }
    }
}
