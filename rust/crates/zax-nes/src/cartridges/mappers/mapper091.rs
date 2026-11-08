use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **91**: the **JY830623C** / **YY840238C** (submapper 0) and the **EJ-006-1**
/// (submapper 1) boards (REQ-CORE-102, MF-UNL-8 step 4).
///
/// nesdev ("INES Mapper 091"): the number "was used by the original release of *Super Fighter III*
/// from an unknown publisher, but is more commonly known by a clone board from J.Y. Company", and
/// the two submappers are two boards:
///
/// > Submapper 0 has a **PA12-based IRQ counter and hard-wired mirroring**; submapper 1 an
/// > **M2-based IRQ counter and selectable mirroring**.
///
/// Four original titles on submapper 0 — *Street Fighter 3*, *Mortal Kombat II*, *Dragon Ball Z 2*
/// and *Mario & Sonic 2* — plus the two *1995 Super HIK 4-in-1* multicarts on the YY840238C.
/// **Submapper 1's *Super Fighter III* is not among them**, and the page says why: "The commonly
/// available ROM image of *Super Fighter III* is a mapper hack for INES Mapper 197." That is a
/// statement about the dumps in circulation, so the title is not evidence for this board and census
/// §7.2 leaves it out of the lot's roster; the board itself is implemented from the page because the
/// page documents it.
///
/// # The register map, and the one mask that is the submapper
///
/// Every register of the chip is in `$6000`-`$7FFF`, and the two boards differ by **one mask bit**:
/// `$F003` on submapper 0 and `$F007` on submapper 1. So the index of a register is
/// `address & 3` or `address & 7`, which is [`Mapper091::index_mask`], and the registers submapper
/// 1 adds are exactly the four that mask makes reachable.
///
/// | register | submapper 0 | submapper 1 |
/// | --- | --- | --- |
/// | `$6000`-`$6003` | four 2 KiB CHR banks | the same four |
/// | `$6004` / `$6005` | — (the index wraps onto CHR 0 and 1) | select horizontal / vertical mirroring, value ignored |
/// | `$6006` / `$6007` | — | the IRQ counter's LSB / MSB |
/// | `$7000`-`$7001` | two 8 KiB PRG banks | the same two |
/// | `$7006` | IRQ stop/acknowledge (index 2) | IRQ stop/acknowledge (index 6) |
/// | `$7007` | IRQ start/reset (index 3) | IRQ start/reset (index 7) |
/// | `$8000`-`$9FFF` | the outer bank register, **from the address** | — |
///
/// # The outer bank register, which is an address and not a datum
///
/// Submapper 0 alone, "Mask: unknown":
///
/// ```text
/// A~FEDC BA98 7654 3210
///   -------------------
///   .... .... .... .PPC
///                   ||+- Select outer 512 KiB CHR-ROM bank (CHR A19)
///                   ++-- Select outer 128 KiB PRG-ROM bank (PRG A17-A18)
/// ```
///
/// The diagram is an **`A~`** and not a `D~`: which address of `$8000`-`$9FFF` a game writes is the
/// register, and the byte it writes is not read at all. **Q-168** records the two things that
/// leaves open — which of A2 and A1 is PRG A18 (this build takes the diagram's own MSB-first column
/// order, the convention every other nesdev field uses) and what "Mask: unknown" means for the
/// addresses above A2.
///
/// # The two counters, which share only their start and stop ports
///
/// * **Submapper 0**: "a binary counter clocked by PPU A12, and simply counts a fixed number of
///   **64 unfiltered rises** of PPU A12". Six stages and the carry out is the IRQ, so it fires
///   every 64 rises.
/// * **Submapper 1**: "a binary counter clocked by the M2 signal with a **factor of 5/4**, meaning
///   counting down by five every fourth M2 cycle". A two-bit prescaler on M2 and a sixteen-bit
///   down counter, whose borrow past zero is the IRQ.
///
/// Both are **unfiltered**, which is the word that keeps submapper 0 away from
/// [`A12Watcher`](crate::cartridges::A12Watcher) exactly as it does the J.Y. ASIC and the Cony
/// board. Q-168 records what neither page sentence says: whether a counter stops at its
/// firing point, and whether `$7007` reloads anything.
///
/// # Where the board sits
///
/// In [`super::IrqBoard`]: both boards hold a counter that pulls /IRQ, which is that enum's entry
/// condition, and the two hooks the two submappers need — `on_cpu_cycle` and
/// `on_ppu_address_bus` — are both already forwarded there for the J.Y. ASIC and the Cony board.
/// `try_read_prg` is **not** needed: every register of this chip is write-only and the board's own
/// `board_memory_sizes` row leaves no RAM at `$6000`-`$7FFF`, so a read there is open bus through
/// `Mapper::try_read_prg`'s default.
///
/// **Not boxed**: the board is a [`BankWindows`] and sixteen bytes, where this enum already carries
/// an `Mmc3Board` inline for the TQROM, so it sets no new high-water mark — [`super::Mmc3Kof97`]'s
/// own reason.
///
/// **There is no ROM of this number under `assets-free/test-roms`** (§5 of
/// `docs/review/UNL-census.md`), so nothing in the campaign exercises it and every test is written
/// from the register table above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapper091 {
    banks: BankWindows,
    /// Submapper 1, the EJ-006-1: the `$F007` mask, the two mirroring ports, the counter register
    /// and the M2 clock. The one per-board parameter, and it carries all five differences.
    is_ej006: bool,

    /// `$6000`-`$6003`, four 2 KiB CHR banks ("CHR-ROM A18..A11" in effect: the page gives the
    /// register no diagram, so all eight bits are kept).
    chr: [u8; 4],
    /// `$7000`-`$7001`, the two 8 KiB PRG banks of `$8000`-`$BFFF`.
    prg: [u8; 2],
    /// Submapper 0's outer bank register, `.PPC` of the **address** of a write to `$8000`-`$9FFF`.
    outer: u8,

    /// Submapper 0: the six-stage A12 counter, 0 to 63. Submapper 1: the sixteen-bit down counter
    /// `$6006`/`$6007` writes.
    irq_counter: u16,
    /// Submapper 1's `÷4` M2 prescaler — "every fourth M2 cycle".
    irq_prescaler: u8,
    irq_enabled: bool,
    irq_pending: bool,
    /// The previous PPU address, for submapper 0's **unfiltered** A12 rise. Kept whatever the board
    /// is, so that the number is in the PPU watch lists for both submappers — the J.Y. ASIC's own
    /// reason.
    last_ppu_address: u16,
}

impl Mapper091 {
    /// Submapper 0's outer PRG bank, in 8 KiB units: "Select outer 128 KiB PRG-ROM bank (PRG
    /// A17-A18)", and bit 4 of an 8 KiB bank number is PRG A17.
    const OUTER_PRG_SHIFT: u32 = 4;
    /// The inner PRG bank submapper 0's outer register sits above: 128 KiB, sixteen 8 KiB banks.
    const INNER_PRG_MASK: i32 = 0x0F;
    /// Submapper 0's outer CHR bank: "Select outer 512 KiB CHR-ROM bank (CHR A19)", and bit 8 of a
    /// **2 KiB** bank number is CHR A19.
    const OUTER_CHR_A19: i32 = 1 << 8;

    /// Submapper 0's counter: "counts a fixed number of **64** unfiltered rises of PPU A12".
    const A12_RISES_PER_IRQ: u16 = 64;
    /// Submapper 1's prescaler and step: "counting down by **five** every **fourth** M2 cycle".
    const M2_PRESCALER: u8 = 4;
    const M2_STEP: u16 = 5;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let mut board = Self {
            banks: windows_for(memory),
            is_ej006: header.is_nes20 && header.submapper == 1,
            chr: [0; 4],
            prg: [0; 2],
            outer: 0,
            irq_counter: 0,
            irq_prescaler: 0,
            irq_enabled: false,
            irq_pending: false,
            last_ppu_address: 0,
        };
        board.update_banks();
        board
    }

    /// The register index inside `$6000`-`$6FFF` and `$7000`-`$7FFF`: "Mask: `$F003`/`$F007`
    /// (submapper 0/1)", so three address bits on the EJ-006-1 and two on the J.Y. board.
    ///
    /// This one mask **is** the submapper: with it at `$03`, `$6004` and `$6005` are CHR registers
    /// 0 and 1 again and `$6006`/`$6007` are registers 2 and 3, which is exactly a board with
    /// hard-wired mirroring and no counter register.
    const fn index_mask(&self) -> u16 {
        if self.is_ej006 {
            0x07
        } else {
            0x03
        }
    }

    /// Submapper 0's `.PPC` outer bank, as the two lines it drives.
    ///
    /// `C` is A0 and `PP` is A2-A1, read **MSB-first** as every other nesdev field diagram is:
    /// A2 is PRG A18 and A1 is PRG A17 (Q-168 records that the page's label writes the pair
    /// ascending, which is the one place this could be read the other way).
    const fn outer_prg(&self) -> i32 {
        ((self.outer as i32) >> 1) & 0x03
    }

    const fn outer_chr_a19(&self) -> i32 {
        if self.outer & 0x01 == 0 {
            0
        } else {
            Self::OUTER_CHR_A19
        }
    }

    fn update_banks(&mut self) {
        // "CPU $8000-$9FFF / $A000-$BFFF: 8 KiB switchable PRG-ROM bank" and "CPU $C000-$FFFF:
        // 16 KiB PRG-ROM bank, hard-wired to last bank".
        //
        // On submapper 0 the outer register is "PRG A17-A18" above a **128 KiB inner bank**, so
        // the registers drive A13-A16 and "the last bank" is the last of that inner bank -- the
        // same reading the Cony board's `$E000` window takes of the same words. On submapper 1
        // there is no outer register at all and the page gives the two bank registers no diagram,
        // so all eight bits reach the ROM and the last bank is the ROM's.
        if self.is_ej006 {
            self.banks.map_prg_8k(0, i32::from(self.prg[0]));
            self.banks.map_prg_8k(1, i32::from(self.prg[1]));
            self.banks.map_prg_16k(2, -1);
        } else {
            let outer = self.outer_prg() << Self::OUTER_PRG_SHIFT;
            for slot in 0..2 {
                let inner = i32::from(self.prg[slot]) & Self::INNER_PRG_MASK;
                self.banks.map_prg_8k(slot, outer | inner);
            }
            self.banks.map_prg_8k(2, outer | (Self::INNER_PRG_MASK - 1));
            self.banks.map_prg_8k(3, outer | Self::INNER_PRG_MASK);
        }

        // "PPU $0000-$07FF / $0800-$0FFF / $1000-$17FF / $1800-$1FFF: 2 KiB switchable CHR-ROM
        // bank", with submapper 0's CHR A19 above all four.
        let outer = if self.is_ej006 {
            0
        } else {
            self.outer_chr_a19()
        };
        for slot in 0..4 {
            self.banks
                .map_chr_2k(slot * 2, i32::from(self.chr[slot]) | outer);
        }
    }

    /// One clock of the counter this board has.
    ///
    /// Submapper 0: "a binary counter clocked by PPU A12, and simply counts a fixed number of 64
    /// unfiltered rises of PPU A12" — six stages whose carry out is the IRQ, so it fires on every
    /// sixty-fourth rise and keeps counting.
    ///
    /// Submapper 1: "a binary counter clocked by the M2 signal with a factor of 5/4, meaning
    /// counting down by five every fourth M2 cycle" — the prescaler here, and the borrow past zero
    /// is the IRQ.
    ///
    /// **Q-168** records what neither sentence says: that the counter keeps running past its firing
    /// point is this build's reading of "a binary counter", against a counter that stops there.
    fn tick_irq(&mut self) {
        if !self.irq_enabled {
            return;
        }

        if self.is_ej006 {
            self.irq_prescaler += 1;
            if self.irq_prescaler < Self::M2_PRESCALER {
                return;
            }

            self.irq_prescaler = 0;
            self.irq_pending |= self.irq_counter < Self::M2_STEP;
            self.irq_counter = self.irq_counter.wrapping_sub(Self::M2_STEP);
            return;
        }

        self.irq_counter = (self.irq_counter + 1) % Self::A12_RISES_PER_IRQ;
        self.irq_pending |= self.irq_counter == 0;
    }
}

impl Mapper for Mapper091 {
    banked!();

    fn mapper_number(&self) -> i32 {
        91
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_raw(&self.prg);
        writer.write_u8(self.outer);
        writer.write_i32(i32::from(self.irq_counter));
        writer.write_u8(self.irq_prescaler);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        writer.write_i32(i32::from(self.last_ppu_address));
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.prg)?;
        self.outer = reader.read_u8()?;
        self.irq_counter = u16::try_from(reader.read_i32()? & 0xFFFF).unwrap_or(0);
        self.irq_prescaler = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.last_ppu_address = u16::try_from(reader.read_i32()? & 0xFFFF).unwrap_or(0);
        // The windows again from the registers, over the PRG-ROM loaded (REQ-CORE-091). The
        // mirroring is the machine's and comes back with its own block.
        self.update_banks();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 4];
        self.prg = [0; 2];
        self.outer = 0;
        self.irq_counter = 0;
        self.irq_prescaler = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// The register file **is** the board's decode of `$6000`-`$7FFF` — "Mask: $F003/$F007" leaves
    /// no address in those 8 KiB that is not a register — so `registry::board_memory_sizes` gives
    /// the number no PRG-RAM and this answers `false` with it.
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x6000 {
            return;
        }

        if address >= 0x8000 {
            // Submapper 0's outer bank register, "$8000-$9FFF, write, submapper 0 only", whose
            // diagram is an **address**: the value written is not read at all. "Mask: unknown", so
            // the page's own range is the decode and A2-A0 are the register (Q-168).
            if !self.is_ej006 && (0x8000..=0x9FFF).contains(&address) {
                self.outer = u8::try_from(address & 0x07).unwrap_or(0);
                self.update_banks();
            }

            return;
        }

        let index = address & self.index_mask();
        if address < 0x7000 {
            match index {
                // "$6000: Select 2 KiB CHR-ROM bank at PPU $0000-$07FF", and the three after it.
                0..=3 => {
                    self.chr[usize::from(index)] = value;
                    self.update_banks();
                }
                // "$6004: Select horizontal mirroring, value ignored" and "$6005: Select vertical
                // mirroring, value ignored" -- submapper 1 only, and on submapper 0 the mask puts
                // these two addresses back on CHR registers 0 and 1.
                //
                // The page says *mirroring* and not *arrangement* here, where mapper 83's gives
                // both namings; this takes the word the page uses (Q-168).
                4 => memory.set_mirroring(NametableMirroring::Horizontal),
                5 => memory.set_mirroring(NametableMirroring::Vertical),
                // "$6006: Set IRQ Counter LSB" and "$6007: Set IRQ Counter MSB" -- submapper 1
                // only. There is no latch beside the counter, so these write it directly, which is
                // the page's own "Set IRQ Counter".
                6 => self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value),
                _ => self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8),
            }

            return;
        }

        match index {
            // "$7000: Select 8 KiB PRG-ROM bank at CPU $8000-$9FFF" and "$7001: ... $A000-$BFFF".
            0 | 1 => {
                self.prg[usize::from(index)] = value;
                self.update_banks();
            }
            // "IRQ Stop/Acknowledge ($7006, write) ... Value is ignored" -- index 2 under
            // submapper 0's mask and 6 under submapper 1's.
            2 if !self.is_ej006 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            // "IRQ Start/Reset ($7007, write) ... Value is ignored". The counter goes back to zero
            // and the prescaler with it; **nothing is reloaded**, because the page gives the board
            // no latch beside the counter -- `$6006`/`$6007` are the counter itself (Q-168).
            3 if !self.is_ej006 => {
                self.irq_enabled = true;
                self.irq_prescaler = 0;
                self.irq_counter = 0;
            }
            6 if self.is_ej006 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            7 if self.is_ej006 => {
                self.irq_enabled = true;
                self.irq_prescaler = 0;
            }
            // Submapper 1's $7002-$7005: the mask reaches them and the page documents nothing
            // there.
            _ => {}
        }
    }

    /// Submapper 1's clock: "the M2 signal with a factor of 5/4".
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.is_ej006 {
            self.tick_irq();
        }
    }

    /// Submapper 0's clock: "64 **unfiltered** rises of PPU A12".
    ///
    /// `last_ppu_address` is kept on both boards, so that the number may be in the PPU watch lists
    /// of `cartridges::mod` for either submapper without a stale edge on the first rise the
    /// counter sees — the J.Y. ASIC's own reason, read for a per-board clock instead of a
    /// per-register one.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if !self.is_ej006 && address & 0x1000 != 0 && self.last_ppu_address & 0x1000 == 0 {
            self.tick_irq();
        }

        self.last_ppu_address = address;
    }
}
