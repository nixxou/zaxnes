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
