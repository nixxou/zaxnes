use super::length_counter::LengthCounter;
use crate::state::{InvalidState, StateReader, StateWriter};

/// The 32-step staircase of nesdev, "APU Triangle".
const SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// The triangle channel: timer, linear counter, length counter and sequencer.
#[derive(Debug, Clone)]
pub(crate) struct TriangleChannel {
    // Power-on step. nesdev does not fix the sequencer's power-on position; step 15 is
    // picked because its output is 0, so a machine that has just been powered on or reset
    // is genuinely silent (REQ-CORE-004) instead of holding the staircase's top step.
    sequence_position: usize,
    timer_period: i32,
    timer: i32,
    control: bool,
    linear_reload_value: u8,
    linear_counter: u8,
    linear_reload: bool,
    pub length: LengthCounter,
}

impl Default for TriangleChannel {
    fn default() -> Self {
        Self {
            sequence_position: 15,
            timer_period: 0,
            timer: 0,
            control: false,
            linear_reload_value: 0,
            linear_counter: 0,
            linear_reload: false,
            length: LengthCounter::default(),
        }
    }
}

impl TriangleChannel {
    /// Output level, 0 to 15.
    pub fn output(&self) -> i32 {
        i32::from(SEQUENCE[self.sequence_position])
    }

    /// Writes register `index` (0 to 3) at APU cycle `cycle`.
    pub fn write(&mut self, index: u16, value: u8, cycle: i64) {
        match index {
            0 => {
                self.control = (value & 0x80) != 0;
                self.length.halt = self.control;
                self.linear_reload_value = value & 0x7F;
            }
            2 => {
                self.timer_period = (self.timer_period & 0x700) | i32::from(value);
            }
            3 => {
                self.timer_period = (self.timer_period & 0x0FF) | (i32::from(value & 0x07) << 8);
                self.length.load(value, cycle);
                self.linear_reload = true;
            }
            _ => {}
        }
    }

    /// Clocks the timer; called every CPU cycle. The sequencer is held while either counter
    /// is zero, and at ultrasonic periods (below 2), where it would only add a DC offset.
    /// Clocks the timer; `true` when the sequencer moved (the output may have changed).
    pub fn clock_timer(&mut self) -> bool {
        if self.timer_period < 2 || self.linear_counter == 0 || !self.length.active() {
            return false;
        }

        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequence_position = (self.sequence_position + 1) & 31;
            true
        } else {
            self.timer -= 1;
            false
        }
    }

    /// Clocks the linear counter on a quarter-frame signal.
    pub fn clock_linear_counter(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_reload_value;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.control {
            self.linear_reload = false;
        }
    }

    /// Back to the power-on state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl TriangleChannel {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.length.save_state(writer);

        writer.write_usize(self.sequence_position);
        writer.write_i32(self.timer_period);
        writer.write_i32(self.timer);
        writer.write_bool(self.control);
        writer.write_u8(self.linear_reload_value);
        writer.write_u8(self.linear_counter);
        writer.write_bool(self.linear_reload);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.length.load_state(reader)?;

        self.sequence_position = reader.read_index(SEQUENCE.len())?;
        self.timer_period = reader.read_i32()?;
        self.timer = reader.read_i32()?;
        self.control = reader.read_bool()?;
        self.linear_reload_value = reader.read_u8()?;
        self.linear_counter = reader.read_u8()?;
        self.linear_reload = reader.read_bool()?;
        Ok(())
    }
}
