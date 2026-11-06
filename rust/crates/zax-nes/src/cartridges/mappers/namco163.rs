// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The register decode of `Namco163::write_register` follows `Namco163.h`'s own `addr & 0xF800`
// switch, which is the one reading of the nesdev register table that could be checked case by
// case against a second source; the three variants' split of $C000-$D800 and of $E000's top two
// bits is that file's as well. See THIRD-PARTY-NOTICES.md.

use super::{windows_for, Mapper};
use crate::apu::Namco163Audio;
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which of the three Namco ASICs an image carries. They share one register layout and differ
/// in what the chip does with four of its nineteen registers (nesdev, "INES Mapper 019" and
/// "INES Mapper 210").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamcoVariant {
    /// The Namco 163 of mapper 19: the IRQ counter, the nametable selects, the 128-byte
    /// internal RAM and the expansion audio.
    Namco163,
    /// The Namcot 175 of mapper 210 submapper 1: hardwired mirroring, a PRG-RAM enable at
    /// $C000, and none of the above.
    Namco175,
    /// The Namcot 340 of mapper 210 submapper 2: mirroring selected by $E000's top two bits,
    /// and no PRG-RAM at all.
    Namco340,
}

/// iNES mappers 19 and 210, the Namco 163 and the two cost-reduced ASICs cut from it
/// (nesdev, "INES Mapper 019", "INES Mapper 210", "Namco 163 audio"; the numbers are outside
/// REQ-CORE-100's table, which REQ-CORE-102 allows, and REQ-CORE-075 makes the chip's sound
/// optional).
///
/// "The mapper contains 19 registers spanning $4800-$5FFF and $8000-$FFFF, with each register
/// occupying an $800-byte range", so the decode is `address & 0xF800`:
///
/// | address | bits | the Namco 163 | the 175 | the 340 |
/// | --- | --- | --- | --- | --- |
/// | $4800 | `DDDD DDDD` | internal RAM data port | — | — |
/// | $5000 | `LLLL LLLL` | IRQ counter, low 8 | — | — |
/// | $5800 | `EHHH HHHH` | IRQ counter, high 7 and enable | — | — |
/// | $8000-$B800 | `BBBB BBBB` | eight 1 KiB CHR banks | the same | the same |
/// | $C000-$C800 | `BBBB BBBB` | nametable selects 0 and 1 | **PRG-RAM enable** ($C000) | — |
/// | $D000-$D800 | `BBBB BBBB` | nametable selects 2 and 3 | — | — |
/// | $E000 | `.SPP PPPP` | 8 KiB PRG bank at $8000, `S` disables sound | bank only | bank, and `MM` picks the mirroring |
/// | $E800 | `HLPP PPPP` | 8 KiB PRG bank at $A000, `H`/`L` disable CIRAM as CHR | bank only | bank only |
/// | $F000 | `..PP PPPP` | 8 KiB PRG bank at $C000 | the same | the same |
/// | $F800 | `KKKK DCBA` | write protect, and the RAM address port | — | — |
///
/// $E000-$FFFF is not switchable: the last 8 KiB bank is wired there.
///
/// **The 128-byte internal RAM lives in the last 128 bytes of PRG-RAM**, not in a field of this
/// struct, and that is a deliberate choice of layout rather than an accident. REQ-CORE-095 gives
/// the host one contiguous slice to persist, [`Cartridge::battery_ram`], and the chip's RAM has
/// to be in it: on this board the wavetable *is* the save file's second half. Mesen's `.sav` has
/// the same two parts in the same order (`Namco163.h:103-139` writes `_saveRam` then
/// `GetInternalRam()`), so a save file crosses between the two programs. The CPU's $6000-$7FFF
/// window never reaches the tail — see [`Mapper::prg_ram_window`].
///
/// **What this build does not serve, and why: Q-126.** Two of the chip's PPU modes need a
/// pattern or nametable fetch to come out of a memory this crate's fetch path cannot reach from
/// the cartridge, and the measurement that forbids the only available mechanism is Q-123's, taken
/// in step 1 of this same lot. Both are decoded, kept in the registers and in the savestate, and
/// answered with the alternative the chip itself offers:
///
/// * a **CHR register holding $E0-$FF** selects a CIRAM page as pattern data. Served as the
///   CHR-ROM bank the register names, i.e. exactly as though both of `$E800`'s disable bits were
///   set — which is the behaviour the chip has on the 256 KiB-CHR games that set them.
/// * a **nametable select below $E0** puts a CHR-ROM bank in a nametable. Served from CIRAM
///   through the cartridge's own mirroring, i.e. as an ordinary board's nametable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namco163 {
    banks: BankWindows,
    mapper_number: i32,
    variant: NamcoVariant,
    /// The chip's sound, on the Namco 163 alone. Its 128 bytes of register and wavetable RAM are
    /// not in here: see the struct's own note.
    audio: Namco163Audio,
    /// The 15-bit up-counter of $5000/$5800, with the enable in bit 15 — the layout the two
    /// registers read and write directly, so a read gives back what was written.
    irq_counter: u16,
    irq_pending: bool,
    /// R0-R7, the eight CHR registers of $8000-$B800, exactly as written.
    chr: [u8; 8],
    /// The four nametable selects of $C000-$D800, exactly as written. Meaningless on the 175,
    /// where $C000 is the PRG-RAM enable, and on the 340.
    nametable_select: [u8; 4],
    /// $E000-$F000's low six bits, the three switchable PRG banks.
    prg: [u8; 3],
    /// `$F800` on the 163 and `$C000` on the 175: the PRG-RAM write protection.
    write_protect: u8,
    /// `$E800`'s two top bits, which take CIRAM out of the CHR windows of $0000-$0FFF and
    /// $1000-$1FFF. Decoded and recorded; see the struct's note and Q-126.
    chr_ciram_disabled: [bool; 2],
    /// What [`Mapper::nametable_page`] answers for each logical table, recomputed whenever a
    /// nametable select moves.
    ///
    /// A field and not a `match`, for the reason the field doc of `Cartridge::nametable_pages`
    /// gives: that function is consulted **once per nametable fetch**.
    nametable_pages: [i32; 4],
    /// Precomputed from `write_protect`, so that `prg_ram_writable` — asked on the CPU's
    /// $6000-$7FFF decode — is one load and one test, like every other board's.
    prg_ram_on: bool,
}

impl Namco163 {
    /// The chip's own RAM: 128 bytes of channel registers and wavetable, "$80 bytes of sound RAM
    /// shared by channel registers and wavetable samples".
    pub(crate) const INTERNAL_RAM_SIZE: usize = Namco163Audio::RAM_SIZE;

    /// PRG-RAM a Namco 163 is given whatever the header says: the four 2 KiB windows of
    /// $6000-$7FFF, plus [`Self::INTERNAL_RAM_SIZE`] for the chip's own RAM behind them.
    pub(crate) const PRG_RAM_SIZE: usize = 4 * 0x800 + Namco163::INTERNAL_RAM_SIZE;

    /// Which ASIC `mapper` and `submapper` name.
    ///
    /// Mapper 19 is the Namco 163 and mapper 210 submapper 1 and 2 are the 175 and the 340
    /// (nesdev). **Submapper 0 of mapper 210 is a Namcot 175** here, which is a provisional
    /// choice the page does not make — it leaves the variant unstated, and Mesen guesses it from
    /// the first write the game makes (`Namco163.h:29-36`, `SetVariant`). This build does not
    /// guess, for one reason: the two variants differ in whether `$E000`'s top two bits *change
    /// the mirroring*, and a 175 image misread as a 340 has its mirroring moved by what are PRG
    /// bank bits to it, which is a visible fault on the first bank switch. A 340 image misread as
    /// a 175 keeps the header's mirroring, which a dump of a hardwired board carries correctly.
    /// So the safe default is the variant that leaves the header alone. See Q-126.
    // Q-126
    pub(crate) const fn variant_of(mapper: i32, submapper: i32) -> NamcoVariant {
        match (mapper, submapper) {
            (210, 2) => NamcoVariant::Namco340,
            (210, _) => NamcoVariant::Namco175,
            _ => NamcoVariant::Namco163,
        }
    }

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let header = memory.rom().header();
        let submapper = if header.is_nes20 { header.submapper } else { 0 };
        let mut board = Self {
            banks: windows_for(memory),
            mapper_number,
            variant: Self::variant_of(mapper_number, submapper),
            audio: Namco163Audio::new(),
            irq_counter: 0,
            irq_pending: false,
            chr: [0; 8],
            nametable_select: [0; 4],
            prg: [0; 3],
            write_protect: 0,
            chr_ciram_disabled: [false; 2],
            nametable_pages: [Cartridge::FOLLOW_MIRRORING; 4],
            prg_ram_on: false,
        };
        board.update_banks();
        board
    }

    /// Whether this image carries the Namco 163 proper, the only one of the three with an IRQ
    /// counter, nametable selects, internal RAM and sound.
    const fn is_163(&self) -> bool {
        matches!(self.variant, NamcoVariant::Namco163)
    }

    /// The chip's 128 bytes inside PRG-RAM. Empty when the board has no PRG-RAM at all, which is
    /// the Namcot 340 — it has no sound either, so nothing reads this there.
    fn internal_ram(memory: &CartridgeMemory) -> &[u8] {
        let ram = memory.prg_ram();
        ram.get(ram.len().saturating_sub(Self::INTERNAL_RAM_SIZE)..)
            .unwrap_or(&[])
    }

    /// The same 128 bytes, mutably.
    fn internal_ram_mut(memory: &mut CartridgeMemory) -> &mut [u8] {
        let len = memory.prg_ram().len();
        let start = len.saturating_sub(Self::INTERNAL_RAM_SIZE);
        memory.prg_ram_mut().get_mut(start..).unwrap_or(&mut [])
    }

    /// The three switchable PRG windows and the wired last one, from the registers as they stand.
    fn update_banks(&mut self) {
        for (slot, bank) in self.prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, i32::from(*bank));
        }

        // "$E000-$FFFF" is absent from the page's PRG table: the last bank is wired there.
        self.banks.map_prg_8k(3, -1);
        self.update_chr_banks();
    }

    /// The eight 1 KiB CHR windows.
    ///
    /// A register holding $E0-$FF asks for a CIRAM page rather than a CHR-ROM bank, unless the
    /// half's `$E800` disable bit is set. This build serves the CHR-ROM bank either way — the
    /// register is still decoded and `chr_ciram_disabled` is still kept, so a lot that implements
    /// the mode has the state it needs. See the struct's note and Q-126.
    // Q-126
    fn update_chr_banks(&mut self) {
        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }

    /// The four nametables, from the selects of $C000-$D800.
    ///
    /// "Values $E0-$FF can select nametable RAM": the low bit is the CIRAM page, which is what
    /// [`Mapper::nametable_page`] answers. A select below $E0 asks for a CHR-ROM nametable, which
    /// this build does not serve (Q-126) and which falls back to the cartridge's own mirroring —
    /// `FOLLOW_MIRRORING` — so that a board whose game has not written the registers yet behaves
    /// like an ordinary one.
    // Q-126
    fn derive_nametables(&self) -> [i32; 4] {
        if !self.is_163() {
            return [Cartridge::FOLLOW_MIRRORING; 4];
        }

        self.nametable_select.map(|select| {
            if select >= 0xE0 {
                i32::from(select & 0x01)
            } else {
                Cartridge::FOLLOW_MIRRORING
            }
        })
    }

    /// Stores what [`Self::derive_nametables`] computes.
    fn refresh_nametables(&mut self) {
        self.nametable_pages = self.derive_nametables();
    }

    /// The write protection, as each variant reads the one register it has for it.
    ///
    /// On the Namco 163 `$F800`'s "upper nybble must equal b0100 to enable writes" and bits A-D
    /// then protect the four 2 KiB windows one by one, which is finer than
    /// [`Mapper::prg_ram_writable`] can say — the per-window half is applied by
    /// [`Mapper::intercept_cpu_write`]. On the 175 the single bit 0 of `$C000` is the whole of
    /// it, and the 340 has no PRG-RAM.
    fn refresh_prg_ram_enable(&mut self) {
        self.prg_ram_on = match self.variant {
            NamcoVariant::Namco163 => (self.write_protect & 0xF0) == 0x40,
            NamcoVariant::Namco175 => (self.write_protect & 0x01) != 0,
            NamcoVariant::Namco340 => false,
        };
    }

    /// Whether the 2 KiB window of $6000-$7FFF that `address` falls in takes writes, on the
    /// Namco 163's four protection bits.
    fn window_writable(&self, address: u16) -> bool {
        let window = (usize::from(address) >> 11) & 0x03;
        (self.write_protect & (1 << window)) == 0
    }

    /// The 340's `MM`, "$E000 bits 7-6 control mirroring": the page's four rows in order.
    const fn mirroring_of(value: u8) -> NametableMirroring {
        match (value >> 6) & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::Vertical,
            2 => NametableMirroring::Horizontal,
            _ => NametableMirroring::SingleScreenB,
        }
    }

    /// One CPU cycle on the 15-bit up-counter of $5000/$5800.
    ///
    /// nesdev: it "increments on each CPU cycle while enabled", and "upon reaching $7FFF, an IRQ
    /// is fired". So the counter **stops** at $7FFF rather than wrapping — the IRQ stays asserted
    /// until the game writes either half, which is what acknowledges it. Mesen's
    /// `ProcessCpuClock` reads the same two sentences the same way (`Namco163.h:141-155`).
    fn step_irq(&mut self) {
        if (self.irq_counter & 0x8000) == 0 || (self.irq_counter & 0x7FFF) == 0x7FFF {
            return;
        }

        self.irq_counter += 1;
        if (self.irq_counter & 0x7FFF) == 0x7FFF {
            self.irq_pending = true;
        }
    }
}

impl Mapper for Namco163 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u16(self.irq_counter);
        writer.write_bool(self.irq_pending);
        writer.write_raw(&self.chr);
        writer.write_raw(&self.nametable_select);
        writer.write_raw(&self.prg);
        writer.write_u8(self.write_protect);
        writer.write_bool(self.chr_ciram_disabled[0]);
        writer.write_bool(self.chr_ciram_disabled[1]);
        // Written last, so that a state from before the sound existed would load with a silent
        // chip (REQ-CORE-091). No such state was ever shipped -- the board and its sound arrive
        // in the same lot -- and the ordering costs nothing; it is the shape `fme7.rs` and
        // `vrc6.rs` established. The 128 bytes themselves are PRG-RAM and are written by
        // `Cartridge::save_state` with the rest of it.
        self.audio.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.irq_counter = reader.read_u16()?;
        self.irq_pending = reader.read_bool()?;
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.nametable_select)?;
        reader.read_into(&mut self.prg)?;
        self.write_protect = reader.read_u8()?;
        self.chr_ciram_disabled[0] = reader.read_bool()?;
        self.chr_ciram_disabled[1] = reader.read_bool()?;
        if reader.remaining() > 0 {
            self.audio.load_state(reader)?;
        } else {
            self.audio.reset();
        }

        // Every derived half comes back from the registers (REQ-CORE-091). The **mirroring** is
        // not touched: `Cartridge::load_state` has just restored it from the state, and the 340's
        // `MM` would overwrite the saved one with whatever `$E000` implies.
        self.refresh_prg_ram_enable();
        self.update_banks();
        self.nametable_pages = self.derive_nametables();
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    /// Nothing on either page gives a power-on value for any register, so they are all 0: the
    /// three switchable PRG windows show bank 0, the CHR windows bank 0, the IRQ is disabled, the
    /// PRG-RAM is write-protected and the nametables follow the header. Mesen powers up the same
    /// way (`Namco163.h:90-101`), and these games write their banks before they draw.
    ///
    /// The chip's 128 bytes are **not** cleared here: they are PRG-RAM, which the core clears at
    /// power-on and keeps across a reset so that a host which reloaded a battery keeps it
    /// (REQ-CORE-095, and `Cartridge::reset`'s own note).
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.audio.reset();
        self.irq_counter = 0;
        self.irq_pending = false;
        self.chr = [0; 8];
        self.nametable_select = [0; 4];
        self.prg = [0; 3];
        self.write_protect = 0;
        self.chr_ciram_disabled = [false; 2];
        self.prg_ram_on = false;
        self.update_banks();
        self.refresh_nametables();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `M` of REQ-CORE-073, which the APU subtracts (AMD-061 §6.2). One load: the level is
    /// recomputed where the chip's channels move, which is the shape every expansion-audio board
    /// in this build now uses.
    fn expansion_audio_output(&self) -> Option<i32> {
        if self.is_163() {
            Some(self.audio.output())
        } else {
            None
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        // Reads are not gated on either variant: only the write protection is a register. The
        // 340 has no PRG-RAM, so there is nothing to read.
        !matches!(self.variant, NamcoVariant::Namco340)
    }

    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_on
    }

    /// $6000-$7FFF covers the four 2 KiB windows and **not** the 128 bytes behind them: the
    /// modulo is over the windows alone, so the chip's RAM is unreachable from the CPU bus except
    /// through $4800. See the struct's note on where that RAM lives.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let windows = memory
            .prg_ram()
            .len()
            .saturating_sub(Self::INTERNAL_RAM_SIZE)
            .max(1);
        (0, windows)
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// The registers below $8000 — the RAM data port and the two IRQ halves — and the per-window
    /// half of the PRG-RAM write protection.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !self.is_163() {
            return false;
        }

        match address {
            0x4800..=0x5FFF => {
                self.write_register(memory, address, value);
                true
            }
            // The global enable is `prg_ram_writable`; this is the four per-window bits, which
            // that method cannot express because it is not given the address.
            0x6000..=0x7FFF if !self.window_writable(address) => true,
            _ => false,
        }
    }

    /// $4800 reads the chip's RAM through the address port, and $5000/$5800 read the IRQ counter
    /// back. A read of $4800 auto-increments the port exactly as a write does, so this is a read
    /// with a side effect and [`Mapper::cpu_peek_override`] is implemented separately.
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !self.is_163() {
            return None;
        }

        match address & 0xF800 {
            0x4800 => {
                let value = self.audio.read_ram(Self::internal_ram(memory));
                Some(Some(value))
            }
            0x5000 => Some(Some((self.irq_counter & 0x00FF) as u8)),
            0x5800 => Some(Some((self.irq_counter >> 8) as u8)),
            _ => None,
        }
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !self.is_163() {
            return None;
        }

        match address & 0xF800 {
            0x4800 => Some(Some(self.audio.peek_ram(Self::internal_ram(memory)))),
            0x5000 => Some(Some((self.irq_counter & 0x00FF) as u8)),
            0x5800 => Some(Some((self.irq_counter >> 8) as u8)),
            _ => None,
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF800 {
            // "Audio expansion RAM access", and the port auto-increments.
            0x4800 if self.is_163() => {
                self.audio.write_ram(Self::internal_ram_mut(memory), value);
            }
            // "Writes acknowledge pending IRQs."
            0x5000 if self.is_163() => {
                self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value);
                self.irq_pending = false;
            }
            0x5800 if self.is_163() => {
                self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8);
                self.irq_pending = false;
            }
            // The eight 1 KiB CHR banks of $8000-$B800.
            0x8000..=0xB800 => {
                self.chr[usize::from((address >> 11) & 0x07)] = value;
                self.update_chr_banks();
            }
            // $C000 is the 175's PRG-RAM enable and the 163's first nametable select; the three
            // above it are nametable selects on the 163 and nothing at all on the other two.
            0xC000 if matches!(self.variant, NamcoVariant::Namco175) => {
                self.write_protect = value;
                self.refresh_prg_ram_enable();
            }
            0xC000..=0xD800 if self.is_163() => {
                self.nametable_select[usize::from((address >> 11) & 0x03)] = value;
                self.refresh_nametables();
            }
            // "Select 8 KiB PRG ROM bank at $8000-$9FFF", six bits, plus the sound disable on the
            // 163 and the mirroring on the 340.
            0xE000 => {
                self.prg[0] = value & 0x3F;
                self.banks.map_prg_8k(0, i32::from(self.prg[0]));
                match self.variant {
                    NamcoVariant::Namco163 => self.audio.set_disabled((value & 0x40) != 0),
                    NamcoVariant::Namco340 => memory.set_mirroring(Self::mirroring_of(value)),
                    NamcoVariant::Namco175 => {}
                }
            }
            0xE800 => {
                self.prg[1] = value & 0x3F;
                self.banks.map_prg_8k(1, i32::from(self.prg[1]));
                if self.is_163() {
                    // Decoded and recorded; see the struct's note and Q-126.
                    self.chr_ciram_disabled = [(value & 0x40) != 0, (value & 0x80) != 0];
                    self.update_chr_banks();
                }
            }
            0xF000 => {
                self.prg[2] = value & 0x3F;
                self.banks.map_prg_8k(2, i32::from(self.prg[2]));
            }
            // "Upper nybble must equal b0100 to enable writes", and the low nibble protects the
            // four windows. The same byte carries the RAM address port and its auto-increment.
            0xF800 if self.is_163() => {
                self.write_protect = value;
                self.refresh_prg_ram_enable();
                self.audio.set_ram_port(value);
            }
            _ => {}
        }
    }

    /// **Out of line, measured.** This is one arm of `Board::on_cpu_cycle`, whose thirty-odd arms
    /// share one prologue, so an arm that holds a value across a call needs a callee-saved
    /// register and the prologue that saves it is executed by **every** board on every CPU cycle.
    /// `vrc6.rs` carries the whole measurement — +17.9 M instructions over 300 frames on thirteen
    /// ROMs, twelve of them with no expansion audio — and the same attribute in the same place is
    /// what keeps this arm a tail call.
    ///
    /// Only mapper 19 is in `Cartridge::clocks_on_cpu_cycle`: the 175 and the 340 have neither an
    /// IRQ counter nor sound, so a mapper-210 image never reaches this at all.
    ///
    /// **What it costs the eleven other latched boards, measured and not hidden.** This is the
    /// first `on_cpu_cycle` arm in the build that reads anything out of `CartridgeMemory` -- the
    /// chip's 128 bytes are PRG-RAM -- and LLVM answers by promoting the `&mut CartridgeMemory`
    /// argument to the pointer and length it actually uses. The new convention propagates up
    /// through `Board::on_cpu_cycle` (both are internal, so it is free to) and the two loads land
    /// in **`NesBus::tick`**:
    ///
    /// ```text
    /// leaq  9592(%rbx), %rdi          leaq  9592(%rbx), %rdi
    ///                                 movq  9528(%rbx), %rsi
    ///                                 movq  9536(%rbx), %rdx
    /// ```
    ///
    /// They sit **after** the `clocks_on_cpu_cycle` test, so the twenty-six unlatched mapper
    /// numbers pay nothing; the twelve latched ones pay two instructions per CPU cycle. An extra
    /// `#[inline(never)]` boundary between this and the audio was built and **refuted** --
    /// argument promotion is transitive through `noinline`, so it moved the loads one frame down
    /// and left `tick` unchanged. What would stop it is a *store* through the pointer, which this
    /// board has no reason to make. See step 4 of `docs/progress/MF-5a.md` for the numbers and
    /// for the one design that would remove it.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        // The guard is not there to save the 175 and the 340 anything -- mapper 210 is not in
        // `Cartridge::clocks_on_cpu_cycle`, so they never arrive here at all. It is there so the
        // invariant holds for a *direct* call too, which is what
        // `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` makes: a chip with no IRQ
        // counter and no sound must not move on a CPU cycle, whoever asks it to.
        if !self.is_163() {
            return;
        }

        self.step_irq();
        self.audio.step_cpu_cycle(Self::internal_ram_mut(memory));
    }
}
