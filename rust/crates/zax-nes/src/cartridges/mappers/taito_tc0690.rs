use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 48, the Taito **TC0690** (REQ-CORE-102).
///
/// nesdev ("INES Mapper 048"): the chip is "a superset of TC0190+PAL16R4 abilities", so the
/// banking is [`super::TaitoTc0190`]'s — two switchable 8 KiB PRG banks with the last two fixed,
/// two 2 KiB CHR registers that "address full 512KB" and four 1 KiB ones limited to 256 KiB —
/// with the register mask widened from `$A003` to **`$E003`** to make room for the IRQ and for a
/// mirroring register of its own:
///
/// | register | what |
/// | --- | --- |
/// | `$8000` | PRG reg 0, 8 KiB at `$8000` |
/// | `$8001` | PRG reg 1, 8 KiB at `$A000` |
/// | `$8002` / `$8003` | the two 2 KiB CHR regs |
/// | `$A000`-`$A003` | the four 1 KiB CHR regs |
/// | `$C000` | IRQ reload value, **inverted** |
/// | `$C001` | IRQ clear |
/// | `$C002` | IRQ enable |
/// | `$C003` | IRQ acknowledge |
/// | `$E000` | `[.M.. ....]` mirroring |
///
/// # The IRQ: the MMC3's, with the page's own two exceptions
///
/// "IRQs on this mapper seem to behave exactly like MMC3's IRQs, with 2 exceptions: 1) The
/// written reload value is inverted. EG: Writing $06 to the reload reg on this mapper would be
/// like writing $F9 on MMC3. You can simply XOR the writes with $FF and it will work just like
/// MMC3. 2) The IRQ seems to trip a little later than it does on MMC3. It looks like about a 4
/// CPU cycle delay from the normal MMC3 IRQ time."
///
/// So the counter is clocked by A12 rises through the MMC3's filter
/// ([`A12Watcher::MMC3_FILTER_DOTS`]), reloads and asserts on MMC3 revision B's rule, and the
/// assertion is **armed rather than made** — [`TaitoTc0690::IRQ_DELAY_CYCLES`] counted down in
/// `on_cpu_cycle`, which is MR-64-4's shape for the RAMBO-1's own delay
/// ([`super::Rambo1`]'s `irq_delay`) and not a new mechanism.
///
/// `$C000`-`$C003` are the MMC3's `$C000`, `$C001`, `$E001` and `$E000` in that order, which is
/// what "exactly like MMC3's IRQs" leaves them to be: the page names the four registers and
/// gives no operational detail of its own.
///
/// # The counter is written out and not shared with [`super::Mmc3`], deliberately
///
/// Sharing it would mean lifting the MMC3's five IRQ fields into a struct of their own, and
/// **that is the one thing this step may not do**: §5 step 4's stop condition is `m4` or `smb3`
/// above the reference, and MF-6b step 1 measured what moving an IRQ board's fields by one byte
/// costs — `Cpu6502::branch` re-laid out and eleven of thirteen ROMs up by +0.009 % to
/// +0.027 %, with no code added at all. Thirty lines of counter here cost the MMC3 nothing, and
/// `mf_6b_the_tc0690_counts_the_same_lines_as_the_mmc3_and_trips_four_cycles_later` is what
/// says the two really do agree, in the shape
/// `mf_5a_the_vrc6_and_the_vrc4_share_one_irq_counter` gave that kind of test.
///
/// # Three readings where Mesen 2 and the page differ, and AMD-061 §1 settles all three
///
/// 1. **The delay is 4 CPU cycles, the page's figure.** `TaitoTc0690.h:32-34` uses **6 or 22**
///    with a comment that says what they are: "6 removes the shaking from The Jetsons, 21-22
///    seems to work for the Flintstones+Captain Saver". Those are tuned against games, not
///    measured off hardware, and the page gives one number.
/// 2. **`$C000` and `$C001` do not acknowledge.** Mesen clears the IRQ source in both
///    (`TaitoTc0690.h:60`, `:67`) with the comment "Flintstones expects either $C000 or $C001 to
///    clear the irq flag". The page's two exceptions do not include that, and on an MMC3 neither
///    `$C000` nor `$C001` acknowledges. **This is the one divergence here that a game can see**,
///    so it is named for R0: if *The Flintstones: The Rescue of Dino & Hoppy* shows a shaking or
///    mistimed status bar, this arm is where to look.
/// 3. **No submapper.** Mesen reads `_romInfo.SubMapperID == 1` for a variant that adds one to
///    the inverted reload value and shortens the delay to 6; the page documents no NES 2.0
///    submapper for mapper 48 at all, so there is none here.
///
/// The page's prose says bit 4 of `$E000` selects the mirroring and **its own register diagram
/// says `[.M.. ....]`, which is bit 6**. The diagram and Mesen 2 (`value & 0x40`) agree, so bit 6
/// it is; `req_core_100_the_tc0690_mirroring_is_bit_6_of_e000` pins the whole byte so the choice
/// is visible rather than implied.
///
/// The six games are *Bakushou!! Jinsei Gekijou 3*, *Bubble Bobble 2*, *Don Doko Don 2*,
/// *Captain Saver*, *The Jetsons: Cogswell's Caper!* and *The Flintstones: The Rescue of Dino &
/// Hoppy*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaitoTc0690 {
    banks: BankWindows,
    a12: A12Watcher,
    prg: [u8; 2],
    chr: [u8; 6],
    /// `$C000` as the board stores it: **already inverted**, so every other use of it below is
    /// the MMC3's own.
    irq_latch: u8,
    irq_counter: i32,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
    /// CPU cycles still to run before the counter's zero reaches the /IRQ line, or 0 when
    /// nothing is armed. The page's "about a 4 CPU cycle delay", in MR-64-4's shape.
    irq_delay: i32,
}

impl TaitoTc0690 {
    /// "It looks like about a 4 CPU cycle delay from the normal MMC3 IRQ time."
    const IRQ_DELAY_CYCLES: i32 = 4;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            a12: A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS),
            prg: [0, 1],
            chr: [0; 6],
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
            irq_delay: 0,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_8k(0, i32::from(self.prg[0]));
        self.banks.map_prg_8k(1, i32::from(self.prg[1]));
        self.banks.map_prg_8k(2, -2);
        self.banks.map_prg_8k(3, -1);

        // The whole byte for the 2 KiB registers, the LSB included — the TC0190's own note:
        // they "use the full written value (not dropping the LSB like MMC3)".
        self.banks.map_chr_2k(0, i32::from(self.chr[0]));
        self.banks.map_chr_2k(2, i32::from(self.chr[1]));
        for window in 0..4 {
            self.banks
                .map_chr_1k(4 + window, i32::from(self.chr[2 + window]));
        }
    }

    /// MMC3 revision B's rule, which is what "exactly like MMC3's IRQs" means here: reload from
    /// the latch when the counter is at zero or a `$C001` clear is pending, decrement otherwise,
    /// and assert when the result is zero with IRQs enabled. The assertion is the one thing that
    /// is not immediate — see [`TaitoTc0690::IRQ_DELAY_CYCLES`].
    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            // **One more than the page's figure, and the ordering inside `NesBus::tick` is
            // why.** That function runs `catch_up_ppu` — where the A12 rise and therefore this
            // arming happen — *before* `Cartridge::step_cpu_cycle`, so the very cycle the rise
            // lands in decrements the counter itself. An MMC3 asserting on that rise is
            // observable at the end of that cycle; "four cycles later" is four cycles after
            // that, so the field counts the arming cycle and then four more.
            // `Rambo1::irq_delay` encodes a different quantity the same way — its page counts
            // **M2 falls** from the rise, and the arming cycle's own fall is the first of them,
            // so its 2 is literally two falls where this 4 is four cycles of difference.
            // `mf_6b_the_tc0690_counts_the_same_lines_as_the_mmc3_and_trips_four_cycles_later`
            // is the measurement, against a real MMC3 on the same instrument.
            self.irq_delay = Self::IRQ_DELAY_CYCLES + 1;
        }
    }
}

impl Mapper for TaitoTc0690 {
    banked!();

    fn mapper_number(&self) -> i32 {
        48
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        writer.write_i32(self.irq_delay);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.irq_delay = reader.read_i32()?;
        self.a12.load_state(reader)
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.prg = [0, 1];
        self.chr = [0; 6];
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_delay = 0;
        self.a12.reset();
        // The mirroring register's own zero state, as on the TC0190: this board controls the
        // layout, so the header's bit never decides it.
        memory.set_mirroring(NametableMirroring::Vertical);
        self.update_banks();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// [`super::Mmc3`]'s bound, with this board's own filter: the line only rises when the
    /// counter reaches zero with IRQs on, the counter only moves on an A12 rise, and two rises
    /// are at least [`A12Watcher::min_rise_interval`] dots apart.
    ///
    /// The four-cycle delay does not enter it. The horizon bounds what the **PPU** can do to the
    /// line, and the delay lands from `on_cpu_cycle`; it can only make the assertion later than
    /// this bound, never sooner.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.irq_pending || !self.irq_enabled {
            return Some(i64::MAX);
        }

        let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
            1 + i64::from(self.irq_latch)
        } else {
            i64::from(self.irq_counter)
        };
        Some((clocks_to_zero - 1) * self.a12.min_rise_interval())
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xE003 {
            // Six bits of PRG, as on the TC0190.
            0x8000 => self.prg[0] = value & 0x3F,
            0x8001 => self.prg[1] = value & 0x3F,
            0x8002 => self.chr[0] = value,
            0x8003 => self.chr[1] = value,
            address @ 0xA000..=0xA003 => {
                self.chr[2 + usize::from(address & 0x03)] = value;
            }
            0xC000 => {
                // "The written reload value is inverted. [...] You can simply XOR the writes
                // with $FF and it will work just like MMC3." Stored inverted, so nothing below
                // has to remember.
                self.irq_latch = value ^ 0xFF;
                return;
            }
            0xC001 => {
                // The MMC3's `$C001`: clear the counter, reload on the next clock. It does
                // **not** acknowledge — see the type's note on Mesen 2.
                self.irq_counter = 0;
                self.irq_reload = true;
                return;
            }
            0xC002 => {
                self.irq_enabled = true;
                return;
            }
            0xC003 => {
                // The MMC3's `$E000`: disable and acknowledge. An acknowledgement disarms a
                // delay that had not landed yet, as `Rambo1`'s does.
                self.irq_enabled = false;
                self.irq_pending = false;
                self.irq_delay = 0;
                return;
            }
            // `[.M.. ....]`: bit 6, which is what the page's diagram and Mesen 2 agree on.
            // `$E001`-`$E003` are not registers.
            0xE000 => {
                memory.set_mirroring(if value & 0x40 == 0 {
                    NametableMirroring::Vertical
                } else {
                    NametableMirroring::Horizontal
                });
                return;
            }
            _ => return,
        }

        self.update_banks();
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.a12.rose(address, clock) {
            self.clock_irq_counter();
        }
    }

    /// `#[inline(never)]`, for the reason [`super::Vrc3::on_cpu_cycle`] gives.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.irq_delay > 0 {
            self.irq_delay -= 1;
            if self.irq_delay == 0 {
                self.irq_pending = true;
            }
        }
    }
}
