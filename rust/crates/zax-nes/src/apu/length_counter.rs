use crate::state::{InvalidState, StateReader, StateWriter};

/// The length counter the four tone channels share (nesdev, "APU Length Counter").
///
/// Two write-ordering quirks are visible to the CPU and are measured by blargg's frame
/// counter suite (REQ-CORE-113), so they are modelled here rather than approximated.
///
/// - **Halt.** A write to the halt flag on the very cycle the counter is clocked takes
///   effect *after* the clock (`10.len_halt_timing`). Nothing is needed for that: the bus
///   steps the APU for a cycle before handing the register write over.
/// - **Reload.** A reload written on the clock cycle is ignored outright when the counter
///   was non-zero before the clock (`11.len_reload_timing`). That needs the pre-clock value
///   and the cycle the clock happened on, which is what [`LengthCounter::clock`] records.
#[derive(Debug, Clone)]
pub(crate) struct LengthCounter {
    clocked_at_cycle: i64,
    value_before_clock: u8,
    value: u8,
    /// Halt flag: bit 5 of $4000 / $4004 / $400C, bit 7 of $4008.
    pub halt: bool,
    enabled: bool,
}

impl Default for LengthCounter {
    fn default() -> Self {
        Self {
            clocked_at_cycle: -1,
            value_before_clock: 0,
            value: 0,
            halt: false,
            enabled: false,
        }
    }
}

impl LengthCounter {
    /// Reload table, indexed by the top five bits of $4003 / $4007 / $400B / $400F.
    pub const TABLE: [u8; 32] = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96,
        22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];

    /// Whether the counter is running, which is what $4015 reports.
    pub const fn active(&self) -> bool {
        self.value > 0
    }

    /// Enables or disables the channel; disabling clears the counter at once.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.value = 0;
        }
    }

    /// Counts down on a half-frame signal, at APU cycle `cycle`.
    pub fn clock(&mut self, cycle: i64) {
        self.clocked_at_cycle = cycle;
        self.value_before_clock = self.value;

        if !self.halt && self.value > 0 {
            self.value -= 1;
        }
    }

    /// Loads the counter from the length register written at cycle `cycle`.
    pub fn load(&mut self, register: u8, cycle: i64) {
        if !self.enabled {
            // "When disabled via $4015, length shouldn't allow reloading" (apu_test 1-len_ctr).
            return;
        }

        if self.clocked_at_cycle == cycle && self.value_before_clock != 0 {
            // Reload written during a length clock, counter non-zero before it: ignored.
            return;
        }

        self.value = Self::TABLE[usize::from(register >> 3)];
    }
}

impl LengthCounter {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.value);
        writer.write_bool(self.halt);
        writer.write_bool(self.enabled);
        writer.write_i64(self.clocked_at_cycle);
        writer.write_u8(self.value_before_clock);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.value = reader.read_u8()?;
        self.halt = reader.read_bool()?;
        self.enabled = reader.read_bool()?;
        self.clocked_at_cycle = reader.read_i64()?;
        self.value_before_clock = reader.read_u8()?;
        Ok(())
    }
}
