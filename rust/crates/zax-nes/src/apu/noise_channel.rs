use super::envelope::Envelope;
use super::length_counter::LengthCounter;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::RegionTiming;

/// Timer periods in CPU cycles, NTSC (nesdev).
const NTSC_PERIODS: [i32; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// The letterless RP2A03's periods (lot FIDPLUS-C, FIDELITY-LEADS G4): its last entry is 2046,
/// not 4068, in every region (`BeesNES` `m_ui16NoisePeriodsOld`, MIT, from Kevin Horton's
/// measurements of the early chip).
const LETTERLESS_PERIODS: [i32; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 2046,
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
    /// A letterless RP2A03 has no short mode: bit 7 of `$400E` does nothing, and its own period
    /// table (`BeesNES` `SetModeFlag` and `OldNoiseTable`, its `bRp2A02` profiles). A setting of
    /// the console model, not state.
    letterless: bool,
    /// The region's table, kept to go back to when the letterless chip is left.
    region_periods: &'static [i32; 16],
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
            letterless: false,
            region_periods: &NTSC_PERIODS,
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
        self.region_periods = if timing.uses_pal_noise_and_dmc_periods {
            &PAL_PERIODS
        } else {
            &NTSC_PERIODS
        };
        self.select_periods();
    }

    fn select_periods(&mut self) {
        self.periods = if self.letterless {
            &LETTERLESS_PERIODS
        } else {
            self.region_periods
        };
    }

    /// Makes the channel a letterless RP2A03's, which has no short mode.
    pub fn set_letterless(&mut self, letterless: bool) {
        self.letterless = letterless;
        if letterless {
            self.short_mode = false;
        }
        self.select_periods();
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
                self.short_mode = (value & 0x80) != 0 && !self.letterless;
                self.period_index = usize::from(value & 0x0F);
            }
            3 => {
                self.length.load(value, cycle);
                self.envelope.restart();
            }
            _ => {}
        }
    }

    /// Timer clocks from now on which the timer only counts down: [`Self::clock_timer`]
    /// returns `false` on each of them.
    #[allow(clippy::cast_sign_loss)] // Checked non-negative.
    pub const fn quiet_clocks(&self) -> u32 {
        if self.timer < 0 {
            u32::MAX
        } else {
            self.timer as u32
        }
    }

    /// Counts the timer down by `clocks`, at most [`Self::quiet_clocks`].
    #[allow(clippy::cast_possible_wrap)] // At most a timer period.
    pub fn skip_quiet_clocks(&mut self, clocks: u32) {
        self.timer -= clocks as i32;
    }

    /// Whether the channel outputs 0 whatever its shift register does, until a register write or
    /// a frame-counter clock (lot FAST-1).
    pub const fn silent(&self) -> bool {
        !self.length.active() || self.envelope.volume() == 0
    }

    /// `clocks` timer clocks of a [`Self::silent`] channel in one go: the same timer and shift
    /// register as that many [`Self::clock_timer`] calls (lot FAST-1).
    #[allow(clippy::cast_possible_wrap)] // At most a frame's clocks.
    pub fn advance_silent(&mut self, clocks: u32) {
        let clocks = clocks as i32;
        if self.timer < 0 || clocks <= self.timer {
            self.timer -= clocks;
            return;
        }
        let rest = clocks - (self.timer + 1);
        let period = self.periods[self.period_index];
        let reloads = 1 + rest / period;
        self.timer = period - 1 - rest % period;
        let tap = if self.short_mode { 6 } else { 1 };
        for _ in 0..reloads {
            let feedback = (self.shift_register ^ (self.shift_register >> tap)) & 1;
            self.shift_register = (self.shift_register >> 1) | (feedback << 14);
        }
    }

    /// Clocks the timer; called every CPU cycle.
    /// Clocks the timer; `true` when the shift register moved (the output may have changed).
    pub fn clock_timer(&mut self) -> bool {
        if self.timer == 0 {
            self.timer = self.periods[self.period_index] - 1;

            // Feedback is bit 0 xor bit 6 in short mode, bit 0 xor bit 1 otherwise (nesdev).
            let feedback = (self.shift_register
                ^ (self.shift_register >> if self.short_mode { 6 } else { 1 }))
                & 1;
            self.shift_register = (self.shift_register >> 1) | (feedback << 14);
            true
        } else {
            self.timer -= 1;
            false
        }
    }

    /// Back to the power-on state; the period tables and the chip (letterless or not) stay: they
    /// are the console's, not state.
    pub fn reset(&mut self) {
        let (periods, region_periods, letterless) =
            (self.periods, self.region_periods, self.letterless);
        *self = Self::default();
        self.periods = periods;
        self.region_periods = region_periods;
        self.letterless = letterless;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NesRegion;

    #[test]
    fn fidplus_c_the_letterless_chip_has_its_own_last_period() {
        let mut noise = NoiseChannel::default();
        noise.set_timing(&RegionTiming::for_region(NesRegion::Pal));
        noise.write(2, 0x8F, 0);
        assert!(noise.short_mode);
        assert_eq!(3778, noise.periods[15]);
        noise.set_letterless(true);
        assert_eq!(2046, noise.periods[15]);
        noise.reset();
        assert_eq!(2046, noise.periods[15], "a reset keeps the chip");
        noise.write(2, 0x8F, 0);
        assert!(!noise.short_mode);
        noise.set_letterless(false);
        assert_eq!(3778, noise.periods[15], "back to the region's table");
    }
}
