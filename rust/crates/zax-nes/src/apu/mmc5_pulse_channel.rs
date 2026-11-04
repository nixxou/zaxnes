use super::envelope::Envelope;
use super::length_counter::LengthCounter;
use super::pulse_channel::DUTY_SEQUENCES;
use crate::state::{InvalidState, StateReader, StateWriter};

/// One of the MMC5's two square channels, $5000-$5003 and $5004-$5007 (REQ-CORE-075).
///
/// REQ-CORE-075 takes the waveform, the duty, the timer, the envelope and the length counter
/// straight from the APU's pulse channels, and drops **two** units the APU pulse has: the sweep,
/// which the board does not carry at all, and the cut below period 8 (AMD-061 §6.1). Both are
/// why this is a separate type rather than `PulseChannel` with a flag: an APU pulse with its
/// sweep register left at 0 mutes itself above period $3FF and goes silent below period 8, and an
/// MMC5 channel must do neither. Keeping the two types apart is also what keeps the difference
/// off the APU pulse's own per-cycle path, which every game runs.
///
/// The envelope and the length counter are **not** clocked from the APU's frame sequencer: the
/// board has no equivalent of it, and [`Mmc5Audio`](super::mmc5_audio::Mmc5Audio) drives both at
/// the same fixed 240 Hz instead (AMD-061 §6.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Mmc5PulseChannel {
    duty: usize,
    sequence_position: usize,
    timer_period: i32,
    timer: i32,
    pub envelope: Envelope,
    pub length: LengthCounter,
}

impl Mmc5PulseChannel {
    /// Current output, 0 to 15.
    ///
    /// AMD-061 §6.1: there is **no cut below period 8** here, where an APU pulse has one.
    /// "Frequency values less than 8 do not silence the MMC5 pulse channels; they can output
    /// ultrasonic frequencies" (nesdev, "MMC5 audio"). The cut stays in
    /// [`PulseChannel`](super::pulse_channel::PulseChannel), which is a separate type, so the
    /// APU's own pulses keep theirs and no game without an MMC5 is affected by this line.
    pub fn output(&self) -> i32 {
        if !self.length.active() || DUTY_SEQUENCES[(self.duty * 8) + self.sequence_position] == 0 {
            0
        } else {
            i32::from(self.envelope.volume())
        }
    }

    /// Writes register `index` (0 to 3) of the channel: 0 = $5000, 1 = $5001, 2 = $5002,
    /// 3 = $5003 (and $5004 upwards). `cycle` is the CPU cycle the write lands on, for the
    /// length reload quirk.
    pub fn write(&mut self, index: u16, value: u8, cycle: i64) {
        match index {
            0 => {
                self.duty = usize::from(value >> 6);
                self.length.halt = (value & 0x20) != 0;
                self.envelope.looping = (value & 0x20) != 0;
                self.envelope.constant_volume = (value & 0x10) != 0;
                self.envelope.period = value & 0x0F;
            }
            // The APU's sweep register. The MMC5 has no sweep unit, so the board ignores it
            // (nesdev, "MMC5 audio").
            1 => {}
            2 => {
                self.timer_period = (self.timer_period & 0x700) | i32::from(value);
            }
            _ => {
                self.timer_period = (self.timer_period & 0x0FF) | (i32::from(value & 0x07) << 8);
                self.length.load(value, cycle);

                // As on an APU pulse: the sequencer restarts at the first value of the
                // sequence and the envelope's start flag is set. The timer is not reloaded.
                self.sequence_position = 0;
                self.envelope.restart();
            }
        }
    }

    /// Clocks the duty sequencer's timer, once every two CPU cycles as the APU does.
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequence_position = (self.sequence_position + 1) & 7;
        } else {
            self.timer -= 1;
        }
    }

    /// Back to the power-on state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Appends the channel state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.envelope.save_state(writer);
        self.length.save_state(writer);

        writer.write_usize(self.duty);
        writer.write_usize(self.sequence_position);
        writer.write_i32(self.timer_period);
        writer.write_i32(self.timer);
    }

    /// Restores the channel state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.envelope.load_state(reader)?;
        self.length.load_state(reader)?;

        self.duty = reader.read_index(DUTY_SEQUENCES.len() / 8)?;
        self.sequence_position = reader.read_index(8)?;
        self.timer_period = reader.read_i32()?;
        self.timer = reader.read_i32()?;
        Ok(())
    }
}
