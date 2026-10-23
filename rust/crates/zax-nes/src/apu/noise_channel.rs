use super::envelope::Envelope;
use super::length_counter::LengthCounter;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::RegionTiming;

/// Timer periods in CPU cycles, NTSC (nesdev).
const NTSC_PERIODS: [i32; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// Timer periods in CPU cycles, PAL (nesdev, REQ-CORE-015).
const PAL_PERIODS: [i32; 16] = [
    4, 8, 14, 30, 60, 88, 118, 148, 188, 236, 354, 472, 708, 944, 1890, 3778,
];

/// The noise channel: timer, 15-bit linear feedback shift register, envelope and length
/// counter (nesdev, "APU Noise").
#[derive(Debug, Clone)]
pub(crate) struct NoiseChannel {
    periods: &'static [i32; 16],
    period_index: usize,
    timer: i32,
    short_mode: bool,
    // The shift register is seeded with 1 at power-on; seeded with 0 it would never leave 0.
    shift_register: i32,
    pub envelope: Envelope,
    pub length: LengthCounter,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            periods: &NTSC_PERIODS,
            period_index: 0,
            timer: 0,
            short_mode: false,
            shift_register: 1,
            envelope: Envelope::default(),
            length: LengthCounter::default(),
        }
    }
}

impl NoiseChannel {
    /// Output level, 0 to 15.
    pub fn output(&self) -> i32 {
        if (self.shift_register & 1) != 0 || !self.length.active() {
            0
        } else {
            i32::from(self.envelope.volume())
        }
    }

    /// Selects the NTSC or the PAL period table (REQ-CORE-015).
    pub fn set_timing(&mut self, timing: &RegionTiming) {
        self.periods = if timing.uses_pal_noise_and_dmc_periods {
            &PAL_PERIODS
        } else {
            &NTSC_PERIODS
        };
    }

    /// Writes register `index` (0 to 3) at APU cycle `cycle`.
    pub fn write(&mut self, index: u16, value: u8, cycle: i64) {
        match index {
            0 => {
                self.length.halt = (value & 0x20) != 0;
                self.envelope.looping = (value & 0x20) != 0;
                self.envelope.constant_volume = (value & 0x10) != 0;
                self.envelope.period = value & 0x0F;
            }
            2 => {
                self.short_mode = (value & 0x80) != 0;
                self.period_index = usize::from(value & 0x0F);
            }
            3 => {
                self.length.load(value, cycle);
                self.envelope.restart();
            }
            _ => {}
        }
    }

    /// Clocks the timer; called every CPU cycle.
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.periods[self.period_index] - 1;

            // Feedback is bit 0 xor bit 6 in short mode, bit 0 xor bit 1 otherwise (nesdev).
            let feedback = (self.shift_register
                ^ (self.shift_register >> if self.short_mode { 6 } else { 1 }))
                & 1;
            self.shift_register = (self.shift_register >> 1) | (feedback << 14);
        } else {
            self.timer -= 1;
        }
    }

    /// Back to the power-on state; the period table stays with the region.
    pub fn reset(&mut self) {
        let periods = self.periods;
        *self = Self::default();
        self.periods = periods;
    }
}

impl NoiseChannel {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.envelope.save_state(writer);
        self.length.save_state(writer);

        writer.write_usize(self.period_index);
        writer.write_i32(self.timer);
        writer.write_bool(self.short_mode);
        writer.write_i32(self.shift_register);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.envelope.load_state(reader)?;
        self.length.load_state(reader)?;

        self.period_index = reader.read_index(self.periods.len())?;
        self.timer = reader.read_i32()?;
        self.short_mode = reader.read_bool()?;
        self.shift_register = reader.read_i32()?;
        Ok(())
    }
}
