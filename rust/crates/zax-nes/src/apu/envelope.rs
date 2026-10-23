use crate::state::{InvalidState, StateReader, StateWriter};

/// The volume envelope of a pulse channel or of the noise channel (nesdev, "APU Envelope").
///
/// The loop flag and the length counter's halt flag are the same bit of the same register;
/// they are kept separately here because they drive two different units.
#[derive(Debug, Clone, Default)]
pub(crate) struct Envelope {
    start: bool,
    divider: u8,
    decay: u8,
    /// Divider period, and the volume itself in constant-volume mode (0 to 15).
    pub period: u8,
    /// Constant volume flag: bit 4 of $4000 / $4004 / $400C.
    pub constant_volume: bool,
    /// Loop flag: bit 5 of the same register.
    pub looping: bool,
}

impl Envelope {
    /// Volume the channel is driven at, 0 to 15.
    pub const fn volume(&self) -> u8 {
        if self.constant_volume {
            self.period
        } else {
            self.decay
        }
    }

    /// Sets the start flag, which a write to the length register does.
    pub fn restart(&mut self) {
        self.start = true;
    }

    /// Clocks the envelope on a quarter-frame signal.
    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
            return;
        }

        if self.divider == 0 {
            self.divider = self.period;

            if self.decay > 0 {
                self.decay -= 1;
            } else if self.looping {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }
}

impl Envelope {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.period);
        writer.write_bool(self.constant_volume);
        writer.write_bool(self.looping);
        writer.write_bool(self.start);
        writer.write_u8(self.divider);
        writer.write_u8(self.decay);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.period = reader.read_u8()?;
        self.constant_volume = reader.read_bool()?;
        self.looping = reader.read_bool()?;
        self.start = reader.read_bool()?;
        self.divider = reader.read_u8()?;
        self.decay = reader.read_u8()?;
        Ok(())
    }
}
