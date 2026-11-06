use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 67, the Sunsoft-3 (REQ-CORE-100, REQ-CORE-102).
///
/// nesdev ("INES Mapper 067"), the whole board: "CPU $8000-$BFFF: 16 KiB switchable PRG ROM
/// bank", "CPU $C000-$FFFF: 16 KiB PRG ROM bank, fixed to the last bank", four switchable 2 KiB
/// CHR banks, mapper-controlled mirroring, "PRG RAM: Unused, but mapper IC provides RAM enables"
/// and "Subject to bus conflicts: No".
///
/// | range | mask | what |
/// | --- | --- | --- |
/// | `$8000` | `$8800` | interrupt acknowledge |
/// | `$8800` / `$9800` / `$A800` / `$B800` | `$F800` | the four 2 KiB CHR banks |
/// | `$C800` | `$F800` | IRQ count, **written twice**, high byte first |
/// | `$D800` | `$F800` | `...P ....` IRQ enable |
/// | `$E800` | `$F800` | `.... ..MM` mirroring |
/// | `$F800` | `$F800` | `...X PPPP` 16 KiB PRG bank |
///
/// **The acknowledge's mask is `$8800` and the rest are `$F800`**, which decides the whole
/// decode in one line: A11 clear anywhere above `$8000` is the acknowledge, A11 set picks one of
/// the other seven registers by A12-A15. So `$8000-$87FF`, `$9000-$97FF` and every other
/// A11-clear page are all the same acknowledge register.
///
/// The counter is a plain CPU-cycle down counter — "while bit 4 is true, the 16-bit count
/// decreases by 1 every CPU cycle. Whenever the count wraps from $0000 to $FFFF, the mapper
/// asserts an IRQ and pauses itself" — and `$C800` "directly affects the current count, not a
/// reload value", in PPUADDR's shape: "the first write sets the **high** 8 bits of the IRQ
/// counter, and the second write sets the **low** 8 bits". Any write to `$D800` "will reset the
/// toggle so that the next write to $C800 will be the first write".
///
/// # Where this board is **not** the Irem H3001 of mapper 65
///
/// The two are the same mechanism — a 16-bit counter, one step per CPU cycle, no prescaler and no
/// scanline mode — and they differ on the two things an implementation conflates:
///
/// * **the wrap.** The H3001 "stops at 0" and fires *at* zero; the Sunsoft-3 fires on the step
///   **past** it, "$0000 → $FFFF". So the same count takes one more cycle here, which is what
///   `mf_6b_mapper_65_fires_at_zero_and_mapper_67_one_cycle_past_it` pins.
/// * **what the firing does.** The H3001 leaves its enable bit alone and stops the counter; this
///   chip "pauses itself", i.e. clears the enable bit and leaves the count running on from
///   $FFFF when a `$D800` write starts it again.
///
/// # Two divergences from Mesen 2, and AMD-061 §1 settles both for the page
///
/// * **`Sunsoft3.h` has no acknowledge at all.** Its `WriteRegister` switches on `addr & 0xF800`
///   and has no arm for the A11-clear pages, so the `$8000` register is unreachable there.
/// * **`$D800` acknowledges in Mesen** (`Sunsoft3.h:57-61`, `ClearIrqSource`), where the page
///   says in as many words: "Despite previous notes, writes to this register do not acknowledge
///   the IRQ."
///
/// The page also notes that "if counting is enabled, the External IRQ pin is also capable of
/// asserting an IRQ. No existing hardware uses this functionality", so there is nothing to wire.
///
/// The games are *Fantasy Zone II* (J) and *Mito Koumon II - Sekai Manyuu Ki*; *Vs. Platoon* is
/// the same chip on a VS System board, which §6 of `docs/review/MF-6-inventory.md` sets aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sunsoft3 {
    banks: BankWindows,
    /// The four 2 KiB CHR banks, six bits each: "the hardware only has six pins for CHR banking,
    /// for a limit of 128KB of CHR".
    chr: [u8; 4],
    /// `$F800` bits 0-3. Bit 4 is "a 1 bit latch, present but unused", so it is not stored.
    prg: u8,
    /// The 16-bit count itself; `$C800` writes into it directly.
    irq_counter: u16,
    /// `$D800` bit 4, `P`.
    irq_enabled: bool,
    /// The `$C800` write-twice toggle: false while the next write is the **high** byte, which is
    /// what a `$D800` write resets it to.
    irq_low_next: bool,
    irq_pending: bool,
}

impl Sunsoft3 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            chr: [0; 4],
            prg: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_low_next: false,
            irq_pending: false,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);
        for slot in 0..4 {
            self.banks.map_chr_2k(slot * 2, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for Sunsoft3 {
    banked!();

    fn mapper_number(&self) -> i32 {
        67
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg);
        writer.write_u16(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_low_next);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg = reader.read_u8()?;
        self.irq_counter = reader.read_u16()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_low_next = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.update_banks();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 4];
        self.prg = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_low_next = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        // "Interrupt Acknowledge ($8000), Mask: $8800. Any write to this address or any of its
        // mirrors acknowledges a pending IRQ." A11 clear is the whole of that mask above $8000.
        if address & 0x0800 == 0 {
            self.irq_pending = false;
            return;
        }

        match address & 0xF800 {
            0x8800 => self.chr[0] = value & 0x3F,
            0x9800 => self.chr[1] = value & 0x3F,
            0xA800 => self.chr[2] = value & 0x3F,
            0xB800 => self.chr[3] = value & 0x3F,
            0xC800 => {
                // PPUADDR's shape: the high byte first, then the low one, into the live count.
                if self.irq_low_next {
                    self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value);
                } else {
                    self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8);
                }

                self.irq_low_next = !self.irq_low_next;
                return;
            }
            0xD800 => {
                // "0: Pause counter; 1: Count", and the write resets the $C800 latch — but it
                // does **not** acknowledge, which is the page's own correction of Disch.
                self.irq_enabled = value & 0x10 != 0;
                self.irq_low_next = false;
                return;
            }
            0xE800 => {
                // ".... ..MM: Nametable mirroring (0=vertical, 1=horizontal, 2=1scA, 3=1scB)".
                memory.set_mirroring(match value & 0x03 {
                    0 => NametableMirroring::Vertical,
                    1 => NametableMirroring::Horizontal,
                    2 => NametableMirroring::SingleScreenA,
                    _ => NametableMirroring::SingleScreenB,
                });
                return;
            }
            // "...X PPPP": four bits of bank, and `X` the unused latch above them.
            _ => self.prg = value & 0x0F,
        }

        self.update_banks();
    }

    /// `#[inline(never)]`, for the reason [`super::Vrc3::on_cpu_cycle`] gives.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }

        // "Whenever the count wraps from $0000 to $FFFF, the mapper asserts an IRQ and pauses
        // itself." The wrap is the event, not the zero — see the type's note on mapper 65.
        self.irq_counter = self.irq_counter.wrapping_sub(1);
        if self.irq_counter == 0xFFFF {
            self.irq_enabled = false;
            self.irq_pending = true;
        }
    }
}
