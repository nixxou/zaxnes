use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **178**: the 外星 (Wàixīng) **FS305**, the 南晶 (Nánjīng) **NJ0430**, and the
/// Jncota, Hengge Dianzi and GameStar / Belsonic boards that carry the same chipset
/// (REQ-CORE-102, MF-UNL-5 step 3).
///
/// nesdev's **"INES Mapper 178"**. Four registers, four PRG banking modes, mirroring from a
/// register, and an 8 KiB PRG-RAM window that is **banked**:
///
/// ```text
/// $4800: .... .SSM        $4801: .... .LLL        $4802: HHHH HHHH        $4803: BBBB BBBB
///              ||+- Nametable            +++- PRG            ++++-++++- PRG            ++++-++++- PRG-RAM
///              ||    mirroring               A16..A14            A24?..A17                A20?..A13
///              ||    0: Vertical
///              ||    1: Horizontal
///              ++-- PRG banking mode
///                    0: NROM-256/BNROM (PRG A14=CPU A14)
///                    1: UNROM (PRG A14..16=111b if CPU A14=1)
///                    2: NROM-128
///                    3: UNROM but with bit 0 of "fixed" bank selectable
///                       (PRG A15..16=11b if CPU A14=1)
/// ```
///
/// `$4801` is "an inner bank register for UNROM mode" and `$4802` "an outer bank register", so a
/// 16 KiB bank number is `L | H<<3` — A14 is the window's own least significant bit and the
/// question marks on A24 and A20 are the page's, not this build's.
///
/// *Banks*: "CPU $6000-$7FFF: 8 KiB switchable PRG-RAM bank", "CPU $8000-$FFFF: 16/32 KiB PRG-ROM
/// bank, switchable", "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM", "Nametable mirroring: Switchable
/// horizontal/vertical settings".
///
/// # This board has **no** PPU-driven CHR window, and that is Q-154
///
/// §7.2 of `docs/review/UNL-census.md` puts this number in a lot called "the Chinese-RPG boards
/// with a PPU-driven CHR-RAM window" and says of its three boards that "all three select a 4 KiB
/// CHR-RAM page **from the PPU address lines** rather than from a register". **This board does
/// not.** Its page gives it "8 KiB **unbanked** CHR-RAM", says the chipset "is designed for using
/// 8 KiB of chip-internal unbanked CHR-RAM, but can address external RAM/ROM as well, banked in
/// 8 KiB amounts", and **mentions PPU A9, PPU A13, a 4 KiB CHR window and CHR switching nowhere at
/// all**. MF-UNL-2's rule is that the number's own page wins, each case in a Q ticket, so:
///
/// * the board overrides **no** PPU hook, and the number is in **neither** of §4.3's watch lists;
/// * [`Mapper::chr_view`] is the `None` [`super::DecodeBoard`] answers for the whole variant, which
///   is not this board's choice and costs it the span fast path it could otherwise have had —
///   the one price it pays for sharing a variant with boards that do watch the bus;
/// * `$4800-$4803` is where its registers are, not `$5000-$5300`.
///
/// # The four PRG modes, as two windows
///
/// This core's PRG window table is four slots of 8 KiB, so each mode is two 16 KiB bank numbers:
///
/// | mode | $8000-$BFFF | $C000-$FFFF | the page's words |
/// | ---: | --- | --- | --- |
/// | 0 | `(L & 6) \| H<<3` | `(L & 6) \| 1 \| H<<3` | "NROM-256/BNROM (PRG A14=CPU A14)" — A14 is the CPU's, so the pair is one 32 KiB bank |
/// | 1 | `(L & 7) \| H<<3` | `7 \| H<<3` | "UNROM (PRG A14..16=111b if CPU A14=1)" — the last 16 KiB of the outer bank is fixed |
/// | 2 | `(L & 7) \| H<<3` | the same | "NROM-128" — one 16 KiB bank in both halves |
/// | 3 | `(L & 7) \| H<<3` | `(L & 1) \| 6 \| H<<3` | "UNROM but with bit 0 of \"fixed\" bank selectable (PRG A15..16=11b if CPU A14=1)" |
///
/// Mode 0's "PRG A14=CPU A14" is why its `$8000` window masks `L`'s bit 0 away: the line is the
/// CPU's and the register's bit 0 reaches nothing. Mode 3 is mode 1 with A14 of the fixed half
/// given back to the register, which is exactly what "bit 0 of the fixed bank selectable" says.
///
/// # The PRG-RAM window, which is the mechanism this board brings
///
/// "PRG-RAM is mapped to CPU $6000-$7FFF and is banked in 8 KiB amounts", from all eight bits of
/// `$4803` — up to 2 MiB on the page's own "PRG-RAM A20?..A13". That is
/// [`Mapper::prg_ram_window`], the hook lot MF-CART built so that no `Board` dispatch answers a
/// CPU read of that window: the offset is settled after each CPU write and cached on `Cartridge`.
/// The wrap is the chip's own length, so an image that declares 8 KiB sees every bank as the same
/// chip, which is what the hardware's missing address lines do.
///
/// **How much RAM an image gets is the header's business** and `registry::board_memory_sizes` says
/// only that the board has at least its one window; see that function's arm and Q-154.
///
/// # Three sentences of the page this build deliberately does not act on
///
/// 1. **The swapped-register dumps.** "ROM images of *宠物: 小精灵 IV* (CRC32 0xB0B13DBD) and
///    *Education Computer 32-in-1 (Game Star) [!]* (CRC32 0xF834F634) have the meaning of registers
///    $4801 and $4802 swapped, and an emulator must shift the value of $4802 one bit to the left
///    before ORing with $4801." §7.2 step 3 asks for this to be "recorded as a **dumping-error
///    note in a comment**, not worked around — the page attributes it to bad dumps, not to a
///    hardware variant", and this paragraph is that note. A CRC32 table in the registry would make
///    two bad dumps run and every good dump of the same titles stop.
/// 2. **The commit-on-$4801 theory.** "Supposedly, the new PRG bank is only committed when $4801
///    is written. **Emulating this supposed behavior however will break Ping Pong.**" The page
///    rejects it in its own next clause, so this board recomputes its windows on every register
///    write.
/// 3. **The FS305's hardwired A14.** "The Waixing FS305 circuit board connects PRG A14 directly to
///    CPU A14, making all PRG banking modes except BNROM unusable." That is a fact about one of the
///    PCBs the number covers, and nothing in a header distinguishes an FS305 from an NJ0430, so the
///    chipset is emulated and the board's wiring is not guessed at. A game on an FS305 only ever
///    selects mode 0 anyway, which is the mode in which the two are the same.
///
/// # Submappers 1 to 3 are deferred, and the page says what they are
///
/// Submapper 1 is "an Infrared Sensor that generates interrupt requests upon detecting movement",
/// submapper 2 "a TMS5220-compatible LPC speech chip … the same speech chip as the one used in
/// Mapper 241", and submapper 3 a two-bit "solder pad read-out" that replaces PRG-ROM reads with a
/// multicart menu selector. §7.2 scopes this lot to **submapper 0 only** and defers 178's speech
/// chip with the rest of the expansion audio and its infrared IRQ with it; the educational
/// computers that use the number also "require Dendy video timing", which is a console-timing
/// matter and out of this lot either way. So [`Mapper::expansion_audio_output`] stays `None`,
/// [`Mapper::irq_asserted`] stays `false`, and a submapper-1, -2 or -3 image runs as the board
/// without its extra chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaixingFs305 {
    banks: BankWindows,
    /// `$4800`: `.... .SSM`, the mirroring bit and the two-bit PRG mode.
    mode: u8,
    /// `$4801`: `.... .LLL`, the inner bank, PRG A16..A14.
    inner: u8,
    /// `$4802`: `HHHH HHHH`, the outer bank, PRG A24?..A17.
    outer: u8,
    /// `$4803`: `BBBB BBBB`, the PRG-RAM bank, PRG-RAM A20?..A13.
    ram_bank: u8,
}

impl WaixingFs305 {
    /// The four registers, at the four addresses the page names. **No mask is given**, and that is
    /// part of Q-154: the decode here is the four addresses themselves, which is what the page
    /// writes. A board that decoded less would answer a mirror no game is documented to use.
    const PORT_FIRST: u16 = 0x4800;
    const PORT_LAST: u16 = 0x4803;

    /// "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM" — the chip the chipset "is designed for". The
    /// figure is the one REQ-CORE-012's default gives an image with no CHR-ROM; it is stated
    /// because the page states it and no iNES 1.0 header field can.
    pub(crate) const CHR_RAM_SIZE: usize = 8 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            mode: 0,
            inner: 0,
            outer: 0,
            ram_bank: 0,
        };
        board.update_banks();
        board
    }

    /// The two 16 KiB PRG windows of the selected mode, and the 8 KiB CHR-RAM chip that never
    /// moves.
    fn update_banks(&mut self) {
        // "This can be considered an outer bank register for UNROM mode": `H` sits above A16, so
        // three bits up from the 16 KiB window's own least significant bit.
        let outer = i32::from(self.outer) << 3;
        let inner = i32::from(self.inner & 0x07);
        let (low, high) = match (self.mode >> 1) & 0x03 {
            // "NROM-256/BNROM (PRG A14=CPU A14)": the line is the CPU's, so the register's bit 0
            // reaches nothing and the pair is one 32 KiB bank.
            0 => (inner & 0x06, (inner & 0x06) | 0x01),
            // "UNROM (PRG A14..16=111b if CPU A14=1)": the outer bank's last 16 KiB is fixed.
            1 => (inner, 0x07),
            // "NROM-128": one 16 KiB bank, in both halves of the window.
            2 => (inner, inner),
            // "UNROM but with bit 0 of \"fixed\" bank selectable (PRG A15..16=11b if CPU A14=1)".
            _ => (inner, (inner & 0x01) | 0x06),
        };
        self.banks.map_prg_16k(0, outer | low);
        self.banks.map_prg_16k(2, outer | high);
        self.banks.map_chr_8k(0);
    }

    /// `$4800` bit 0: "Nametable mirroring — 0: Vertical, 1: Horizontal."
    fn mirroring(&self) -> NametableMirroring {
        if self.mode & 0x01 == 0 {
            NametableMirroring::Vertical
        } else {
            NametableMirroring::Horizontal
        }
    }
}

impl Mapper for WaixingFs305 {
    banked!();

    fn mapper_number(&self) -> i32 {
        178
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.mode);
        writer.write_u8(self.inner);
        writer.write_u8(self.outer);
        writer.write_u8(self.ram_bank);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The windows themselves are in the block `Mapper::save_state` wrote before this one.
        self.mode = reader.read_u8()?;
        self.inner = reader.read_u8()?;
        self.outer = reader.read_u8()?;
        self.ram_bank = reader.read_u8()?;
        Ok(())
    }

    /// The page gives no power-on values, so all four registers come up clear: mode 0 (BNROM) with
    /// vertical mirroring, outer bank 0, and the first 32 KiB of the image at $8000-$FFFF. That is
    /// also the configuration the FS305's hardwired A14 leaves usable, so a game on either PCB
    /// boots the same way.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mode = 0;
        self.inner = 0;
        self.outer = 0;
        self.ram_bank = 0;
        memory.set_mirroring(self.mirroring());
        self.update_banks();
    }

    /// "PRG-RAM is mapped to CPU $6000-$7FFF and is banked in 8 KiB amounts", from all eight bits
    /// of `$4803`.
    ///
    /// The wrap is the chip's own length, so an image that declares one 8 KiB chip sees every bank
    /// as that chip — the modulo standing in for the address lines the board does not have, which
    /// is `Fme7::prg_ram_window`'s own note in the same words.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        (
            usize::from(self.ram_bank) * BankWindows::PRG_WINDOW_SIZE,
            memory.prg_ram().len(),
        )
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if !(Self::PORT_FIRST..=Self::PORT_LAST).contains(&address) {
            return;
        }

        match address & 0x03 {
            0 => {
                self.mode = value;
                memory.set_mirroring(self.mirroring());
            }
            1 => self.inner = value,
            2 => self.outer = value,
            // `$4803` moves the PRG-RAM window and no PRG-ROM window; `Cartridge::cpu_write` asks
            // `Mapper::cpu_read_windows` after every write either way, so there is nothing to
            // special-case.
            _ => self.ram_bank = value,
        }

        self.update_banks();
    }
}
