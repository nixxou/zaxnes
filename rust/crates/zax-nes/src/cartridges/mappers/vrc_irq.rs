// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// Mesen keeps the same counter in one place for the same reason, `VrcIrq.h`; this file is the
// shape its `VrcIrq` has, with the register decode left to each board because the VRC4 and the
// VRC6 reach the four registers through different addresses. See THIRD-PARTY-NOTICES.md.

use crate::state::{InvalidState, StateReader, StateWriter};

/// The IRQ counter Konami put in the VRC4, the VRC6 and the VRC7 (nesdev, "VRC IRQ"): an 8-bit
/// up-counter that reloads from a latch and pulls /IRQ when it passes $FF, clocked either once
/// per CPU cycle or once per 341 PPU dots through a ÷3 prescaler.
///
/// **One type, not two copies.** `vrc2_vrc4.rs` carried these seven fields and these three
/// methods inline; the VRC6 needs exactly the same counter, differing only in how the four
/// registers are addressed — a VRC4 reaches them through two address lines and splits the latch
/// into nibbles, a VRC6 has them at `$F000`, `$F001` and `$F002` with the latch written whole.
/// So the decode stays on each board and the counter is here.
///
/// The save-state layout is **byte for byte** the one `Vrc2Vrc4::save_registers` wrote before the
/// extraction — latch, counter, prescaler, then the four flags in that order — so a mapper
/// 21/22/23/25/27 state written before this lot loads unchanged (REQ-CORE-091).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VrcIrq {
    /// The reload value. "Writing here sets the reload value for the IRQ counter."
    latch: u8,
    /// Held as `i32` rather than `u8` because the wrap test compares against $FF before the
    /// increment, which is how the hardware reloads on the step *out of* $FF.
    counter: i32,
    /// "a prescaler which divides the CPU clock by 341": counted down by 3 per CPU cycle and
    /// topped up by 341, so a step happens every 341/3 CPU cycles — one PPU scanline.
    prescaler: i32,
    enabled: bool,
    /// Bit 0 of the control register, `A`: what the enable becomes when the next acknowledge
    /// arrives.
    enable_after_ack: bool,
    /// Bit 2 of the control register, `M`: "1 = cycle mode", the prescaler bypassed.
    cycle_mode: bool,
    pending: bool,
}

impl VrcIrq {
    /// The prescaler's period, in thirds of a CPU cycle: 341 PPU dots to a scanline.
    const PRESCALER_PERIOD: i32 = 341;

    pub const fn new() -> Self {
        Self {
            latch: 0,
            counter: 0,
            prescaler: Self::PRESCALER_PERIOD,
            enabled: false,
            enable_after_ack: false,
            cycle_mode: false,
            pending: false,
        }
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub const fn reset(&mut self) {
        *self = Self::new();
    }

    /// Whether the counter is pulling /IRQ.
    pub const fn asserted(&self) -> bool {
        self.pending
    }

    /// The whole latch, which is how a VRC6 writes it (`$F000`, `LLLL LLLL`).
    pub const fn write_latch(&mut self, value: u8) {
        self.latch = value;
    }

    /// One nibble of the latch, which is how a VRC4 writes it: its two latch registers each
    /// carry four bits.
    pub const fn write_latch_nibble(&mut self, value: u8, high: bool) {
        if high {
            self.latch = (self.latch & 0x0F) | ((value & 0x0F) << 4);
        } else {
            self.latch = (self.latch & 0xF0) | (value & 0x0F);
        }
    }

    /// The control register, `.... .MEA`.
    ///
    /// "If E is set, the IRQ counter is reloaded with the latch value and the prescaler is
    /// reset", and a write here always acknowledges — Mesen clears the source unconditionally
    /// (`VrcIrq.h:76-88`), which is the reading `Vrc2Vrc4` already had.
    pub fn write_control(&mut self, value: u8) {
        self.enable_after_ack = value & 0x01 != 0;
        self.enabled = value & 0x02 != 0;
        self.cycle_mode = value & 0x04 != 0;
        if self.enabled {
            self.counter = i32::from(self.latch);
            self.prescaler = Self::PRESCALER_PERIOD;
        }

        self.pending = false;
    }

    /// The acknowledge register: "writing any value to this register will acknowledge the
    /// pending IRQ, and copy the A bit to the E bit".
    pub const fn acknowledge(&mut self) {
        self.pending = false;
        self.enabled = self.enable_after_ack;
    }

    /// One CPU cycle.
    ///
    /// `#[inline]` deliberately: this is `Board::on_cpu_cycle`'s arm for five mapper numbers
    /// already and now for two more, and MF-3b rule 4 is the story of what a new distinct arm
    /// body in that dispatch costs every other game. With the attribute the arm is the same
    /// inlined body it was before the extraction, which the post-LTO diff of step 1 checks.
    #[inline]
    pub fn step_cpu_cycle(&mut self) {
        if !self.enabled {
            return;
        }

        if self.cycle_mode {
            self.clock();
            return;
        }

        self.prescaler -= 3;
        if self.prescaler > 0 {
            return;
        }

        self.prescaler += Self::PRESCALER_PERIOD;
        self.clock();
    }

    /// "When the counter would increment past $FF, it is instead reloaded with the latch value
    /// and an IRQ is generated."
    fn clock(&mut self) {
        if self.counter == 0xFF {
            self.counter = i32::from(self.latch);
            self.pending = true;
            return;
        }

        self.counter += 1;
    }

    /// Appends the counter to `writer` (REQ-CORE-090), in the order `Vrc2Vrc4` used before the
    /// extraction — see the type's own note.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.latch);
        writer.write_i32(self.counter);
        writer.write_i32(self.prescaler);
        writer.write_bool(self.enabled);
        writer.write_bool(self.enable_after_ack);
        writer.write_bool(self.cycle_mode);
        writer.write_bool(self.pending);
    }

    /// Restores what [`VrcIrq::save_state`] wrote (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.latch = reader.read_u8()?;
        self.counter = reader.read_i32()?;
        self.prescaler = reader.read_i32()?;
        self.enabled = reader.read_bool()?;
        self.enable_after_ack = reader.read_bool()?;
        self.cycle_mode = reader.read_bool()?;
        self.pending = reader.read_bool()?;
        Ok(())
    }
}

/// The VRC3's counter, mapper 73 (nesdev, "VRC3"): **sixteen** bits, CPU cycles only, with the
/// same `.... .MEA` control register and the same acknowledge as [`VrcIrq`] above.
///
/// nesdev: "The counter is 16 bits instead of 8 bits, and there is no scanline mode, only CPU
/// cycle mode"; the latch arrives as **four** nibbles at `$8000`, `$9000`, `$A000` and `$B000`;
/// "when it overflows from $FFFF, an IRQ is generated, and the counter is reloaded with the
/// 16-bit latch value", and in 8-bit mode "IRQs are generated when the lower 8 bits overflow
/// from $FF". So the VRC3's `M` bit is a counter **width** where the VRC4's is a prescaler
/// bypass, and the VRC3 has no prescaler at all.
///
/// # Why this is a second type and not a width parameter on [`VrcIrq`]
///
/// It was built that way first — `VrcIrqCounter<const WIDE: bool>` with `VrcIrq` as `<false>`,
/// the shape MR-4-3 gave the MC-ACC and MF-6a gave mapper 155 — and **measured**: the 16-bit
/// latch needs one byte more than the narrow counter's, which lands in the padding after `latch`
/// and so costs the struct nothing, but it moves `pending` one byte along inside `Vrc2Vrc4`,
/// `Vrc6` and `Vrc7`. Those three offsets are *operands* in the thirteen per-board blocks
/// `Board::irq_asserted` is compiled to inside `Cpu6502::branch`, and moving them re-laid the
/// whole chain out: `branch` kept its 418 instructions and its dynamic count rose on **eleven of
/// lot PR-1's thirteen ROMs**, +0.009 % to +0.027 %, ending +9 886 794 on the set. MF-6b step 1
/// has the table.
///
/// So the VRC3's counter stands beside Konami's rather than inside it: the two share a file, a
/// control register and an acknowledge, and nothing a compiler can lay out. The 8-bit wrap rule
/// and the prescaler have no code in common anyway, which is the other half of the answer to
/// §5's question — and `mf_6b_the_vrc3_and_the_vrc4_fire_on_the_same_cycle` pins the one thing
/// they do share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vrc3Irq {
    /// The 16-bit reload value, four nibbles of it.
    latch: u16,
    /// Held as `i32` for the reason [`VrcIrq::counter`] is: the wrap is tested against the top
    /// value *before* the increment, which is how the hardware reloads on the step out of it.
    counter: i32,
    enabled: bool,
    /// Bit 0 of the control register, `A`.
    enable_after_ack: bool,
    /// Bit 2, `M`: "1 = 8-bit mode", only bits 0-7 of the counter taking part in the wrap.
    small: bool,
    pending: bool,
}

impl Vrc3Irq {
    pub const fn new() -> Self {
        Self {
            latch: 0,
            counter: 0,
            enabled: false,
            enable_after_ack: false,
            small: false,
            pending: false,
        }
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub const fn reset(&mut self) {
        *self = Self::new();
    }

    /// Whether the counter is pulling /IRQ.
    pub const fn asserted(&self) -> bool {
        self.pending
    }

    /// Nibble `nibble` of the latch: `$8000` is bits 0-3, `$9000` bits 4-7, `$A000` bits 8-11 and
    /// `$B000` bits 12-15. `nibble` is a literal at every call site, so the shift and the mask
    /// fold away to the single `or` the hardware's four registers are.
    pub const fn write_latch_nibble(&mut self, value: u8, nibble: u32) {
        let shift = nibble * 4;
        let field = 0x000F << shift;
        self.latch = (self.latch & !field) | ((value as u16 & 0x000F) << shift);
    }

    /// The control register, `.... .MEA`: "Any write to this register will acknowledge the
    /// pending IRQ. If this register is written to with the E bit set, the 16-bit IRQ counter is
    /// reloaded."
    pub const fn write_control(&mut self, value: u8) {
        self.enable_after_ack = value & 0x01 != 0;
        self.enabled = value & 0x02 != 0;
        self.small = value & 0x04 != 0;
        if self.enabled {
            self.counter = self.latch as i32;
        }

        self.pending = false;
    }

    /// The acknowledge register: it clears the pending IRQ and copies `A` into `E`, as on every
    /// other Konami chip.
    pub const fn acknowledge(&mut self) {
        self.pending = false;
        self.enabled = self.enable_after_ack;
    }

    /// One CPU cycle. The counter "is incremented each CPU cycle (if enabled)" and has no
    /// prescaler and no scanline mode at all.
    #[inline]
    pub const fn step_cpu_cycle(&mut self) {
        if !self.enabled {
            return;
        }

        // 8-bit mode leaves bits 8-15 of the counter alone and reloads the low byte from the low
        // byte of the latch, which is how Mesen reads it too (`VRC3.h:41-48`, whose own
        // `if(_smallCounter == 0)` tests the mode flag and not the byte it has just incremented,
        // so that branch never fires there; AMD-061 §1 puts the page first).
        let top = if self.small { 0xFF } else { 0xFFFF };
        if self.counter & top != top {
            self.counter += 1;
            return;
        }

        self.counter = if self.small {
            (self.counter & 0xFF00) | (self.latch as i32 & 0xFF)
        } else {
            self.latch as i32
        };
        self.pending = true;
    }

    /// Appends the counter to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(u8::try_from(self.latch & 0xFF).unwrap_or(0));
        writer.write_u8(u8::try_from(self.latch >> 8).unwrap_or(0));
        writer.write_i32(self.counter);
        writer.write_bool(self.enabled);
        writer.write_bool(self.enable_after_ack);
        writer.write_bool(self.small);
        writer.write_bool(self.pending);
    }

    /// Restores what [`Vrc3Irq::save_state`] wrote (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let low = reader.read_u8()?;
        let high = reader.read_u8()?;
        self.latch = u16::from(low) | (u16::from(high) << 8);
        self.counter = reader.read_i32()?;
        self.enabled = reader.read_bool()?;
        self.enable_after_ack = reader.read_bool()?;
        self.small = reader.read_bool()?;
        self.pending = reader.read_bool()?;
        Ok(())
    }
}
