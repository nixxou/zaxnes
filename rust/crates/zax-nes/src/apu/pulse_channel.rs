use super::envelope::Envelope;
use super::length_counter::LengthCounter;

/// The duty sequences of nesdev, "APU Pulse", eight steps each.
const DUTY_SEQUENCES: [u8; 32] = [
    0, 1, 0, 0, 0, 0, 0, 0, //
    0, 1, 1, 0, 0, 0, 0, 0, //
    0, 1, 1, 1, 1, 0, 0, 0, //
    1, 0, 0, 1, 1, 1, 1, 1,
];

/// A pulse channel: timer, duty sequencer, sweep, envelope and length counter (nesdev,
/// "APU Pulse", "APU Sweep").
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the hardware's independent flags.
pub(crate) struct PulseChannel {
    is_first: bool,
    duty: usize,
    sequence_position: usize,
    timer_period: i32,
    timer: i32,
    sweep_enabled: bool,
    sweep_negate: bool,
    sweep_reload: bool,
    sweep_period: i32,
    sweep_shift: i32,
    sweep_divider: i32,
    pub envelope: Envelope,
    pub length: LengthCounter,
}

impl PulseChannel {
    /// Creates pulse 1 (`is_first`, one's complement negate) or pulse 2.
    pub fn new(is_first: bool) -> Self {
        Self {
            is_first,
            duty: 0,
            sequence_position: 0,
            timer_period: 0,
            timer: 0,
            sweep_enabled: false,
            sweep_negate: false,
            sweep_reload: false,
            sweep_period: 0,
            sweep_shift: 0,
            sweep_divider: 0,
            envelope: Envelope::default(),
            length: LengthCounter::default(),
        }
    }

    /// Output level, 0 to 15.
    pub fn output(&self) -> i32 {
        if self.muted()
            || !self.length.active()
            || DUTY_SEQUENCES[(self.duty * 8) + self.sequence_position] == 0
        {
            0
        } else {
            i32::from(self.envelope.volume())
        }
    }

    /// A period below 8 or a sweep target above $7FF mutes the channel (nesdev).
    fn muted(&self) -> bool {
        self.timer_period < 8 || self.sweep_target() > 0x7FF
    }

    /// Writes register `index` (0 to 3) at APU cycle `cycle`.
    pub fn write(&mut self, index: u16, value: u8, cycle: i64) {
        match index {
            0 => {
                self.duty = usize::from(value >> 6);
                self.length.halt = (value & 0x20) != 0;
                self.envelope.looping = (value & 0x20) != 0;
                self.envelope.constant_volume = (value & 0x10) != 0;
                self.envelope.period = value & 0x0F;
            }
            1 => {
                self.sweep_enabled = (value & 0x80) != 0;
                self.sweep_period = i32::from((value >> 4) & 0x07);
                self.sweep_negate = (value & 0x08) != 0;
                self.sweep_shift = i32::from(value & 0x07);
                self.sweep_reload = true;
            }
            2 => {
                self.timer_period = (self.timer_period & 0x700) | i32::from(value);
            }
            _ => {
                self.timer_period = (self.timer_period & 0x0FF) | (i32::from(value & 0x07) << 8);
                self.length.load(value, cycle);

                // The sequencer restarts at the first value of the sequence and the envelope
                // is restarted (nesdev). The timer itself is not reloaded.
                self.sequence_position = 0;
                self.envelope.restart();
            }
        }
    }

    /// Clocks the timer; called every other CPU cycle.
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequence_position = (self.sequence_position + 1) & 7;
        } else {
            self.timer -= 1;
        }
    }

    /// Clocks the sweep unit on a half-frame signal.
    pub fn clock_sweep(&mut self) {
        let target = self.sweep_target();

        if self.sweep_divider == 0
            && self.sweep_enabled
            && self.sweep_shift > 0
            && self.timer_period >= 8
            && target <= 0x7FF
        {
            self.timer_period = target;
        }

        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    fn sweep_target(&self) -> i32 {
        let mut change = self.timer_period >> self.sweep_shift;
        if self.sweep_negate {
            change = -change;
            if self.is_first {
                change -= 1;
            }
        }

        self.timer_period + change
    }

    /// Back to the power-on state.
    pub fn reset(&mut self) {
        *self = Self::new(self.is_first);
    }
}
