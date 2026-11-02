use super::mixer;
use super::mmc5_pulse_channel::Mmc5PulseChannel;
use crate::state::{InvalidState, StateReader, StateWriter};

/// CPU cycles between two envelope clocks: 1 789 773 / 7 458 is 239.98 Hz, the "about 240 Hz"
/// of REQ-CORE-075.
pub const ENVELOPE_CYCLES: i32 = 7458;

/// The MMC5's expansion audio: two square channels, a PCM channel and the $5010 / $5015
/// registers (REQ-CORE-075).
///
/// Written from the public nesdev page "MMC5 audio" and from REQ-CORE-075, which settles the
/// points the page leaves open or where the reference differs.
///
/// **No frame sequencer.** The board has no equivalent of the APU's frame counter: the envelopes
/// of its two squares are clocked at a fixed cadence of about 240 Hz, every [`ENVELOPE_CYCLES`]
/// CPU cycles, and their length counters every other time, so at about 120 Hz. Neither follows
/// `$4017`, which is the difference a game would hear if the APU's sequencer were reused — hence
/// the dedicated divider below. The public documentation gives the length counters 240 Hz as
/// well; REQ-CORE-075 asks for 120 Hz, which is the reference's, and records the divergence.
///
/// **PCM channel.** Write mode only. A non-zero write to $5011 sets the level; a write of 0
/// leaves the level alone and raises the PCM IRQ flag instead. Reading $5010 returns that flag
/// in bit 7 and clears it. The flag never pulls the CPU's /IRQ line — REQ-CORE-075 records that
/// as a divergence too, and it is what keeps a game that enables the IRQ from looping in a
/// handler the spec gives no other way out of. The read mode of $5010 (bit 0 set), where reads
/// of $8000-$BFFF feed the converter, is not emulated: the level simply stays at the last value
/// written.
///
/// **Output.** `M = r(Km · P(q1 + q2)) + 31 · pcm`, which the APU adds to its own `A` before the
/// sum is clipped to 32 767 (REQ-CORE-073).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the board's independent hardware flags.
pub(crate) struct Mmc5Audio {
    pulse1: Mmc5PulseChannel,
    pulse2: Mmc5PulseChannel,

    cycles: i64,
    half_rate: bool,
    divider: i32,
    /// Whether the next envelope clock also clocks the length counters. It starts `true` so
    /// that the first one does **not**: the counters then fall on the even envelope clocks,
    /// every 2 x [`ENVELOPE_CYCLES`] cycles from power-on, which is the phase the APU's own
    /// half-frame signal has relative to its quarter-frame signal.
    length_turn: bool,

    pcm: u8,
    read_mode: bool,
    irq_enabled: bool,
    irq_flag: bool,
}

impl Mmc5Audio {
    /// A board's sound at power-on, silent (REQ-CORE-004).
    pub fn new() -> Self {
        Self {
            length_turn: true,
            ..Self::default()
        }
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units (REQ-CORE-075).
    pub fn output(&self) -> i32 {
        let index = self.pulse1.output() + self.pulse2.output();
        mixer::MMC5_PULSE_TABLE[usize::try_from(index)
            .unwrap_or(0)
            .min(mixer::PULSE_INDEX_MAX)]
            + (mixer::MMC5_PCM_STEP * i32::from(self.pcm))
    }

    /// Back to the power-on state, silent (REQ-CORE-004).
    pub fn reset(&mut self) {
        self.pulse1.reset();
        self.pulse2.reset();
        *self = Self::new();
    }

    /// Advances the board's sound by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        self.cycles += 1;

        self.half_rate = !self.half_rate;
        if self.half_rate {
            // The square timers run at half the CPU clock, as the APU's pulses do.
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
        }

        self.divider += 1;
        if self.divider < ENVELOPE_CYCLES {
            return;
        }

        self.divider = 0;
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();

        self.length_turn = !self.length_turn;
        if self.length_turn {
            self.pulse1.length.clock(self.cycles);
            self.pulse2.length.clock(self.cycles);
        }
    }

    /// Writes one of the board's sound registers, $5000 to $5015.
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x5000..=0x5003 => self.pulse1.write(address - 0x5000, value, self.cycles),
            0x5004..=0x5007 => self.pulse2.write(address - 0x5004, value, self.cycles),
            0x5010 => {
                self.read_mode = (value & 0x01) != 0;
                self.irq_enabled = (value & 0x80) != 0;
            }
            0x5011 => {
                // A write of 0 does not change the level; it raises the PCM IRQ flag
                // (REQ-CORE-075). Writes keep working in read mode, which is not emulated.
                if value == 0 {
                    self.irq_flag = true;
                } else {
                    self.pcm = value;
                }
            }
            0x5015 => {
                self.pulse1.length.set_enabled((value & 0x01) != 0);
                self.pulse2.length.set_enabled((value & 0x02) != 0);
            }
            _ => {}
        }
    }

    /// Reads one of the board's two readable sound registers, $5010 or $5015, as a CPU read
    /// does: a read of $5010 **clears** the PCM IRQ flag (REQ-CORE-075). `None` when the board
    /// drives nothing at `address`.
    pub fn read(&mut self, address: u16) -> Option<u8> {
        let value = self.peek(address);
        if address == 0x5010 {
            self.irq_flag = false;
        }
        value
    }

    /// The same two registers with no side effect at all (REQ-CORE-081): the flag of $5010 is
    /// reported without being acknowledged under the game's feet, which is what the host and
    /// the Lua accessors need.
    pub fn peek(&self, address: u16) -> Option<u8> {
        match address {
            0x5010 => Some(if self.irq_flag { 0x80 } else { 0x00 }),
            0x5015 => Some(
                u8::from(self.pulse1.length.active())
                    | (u8::from(self.pulse2.length.active()) << 1),
            ),
            _ => None,
        }
    }

    /// Appends the sound state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.pulse1.save_state(writer);
        self.pulse2.save_state(writer);

        writer.write_i64(self.cycles);
        writer.write_bool(self.half_rate);
        writer.write_i32(self.divider);
        writer.write_bool(self.length_turn);
        writer.write_u8(self.pcm);
        writer.write_bool(self.read_mode);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_flag);
    }

    /// Restores the sound state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.pulse1.load_state(reader)?;
        self.pulse2.load_state(reader)?;

        self.cycles = reader.read_i64()?;
        self.half_rate = reader.read_bool()?;
        self.divider = reader.read_i32()?;
        self.length_turn = reader.read_bool()?;
        self.pcm = reader.read_u8()?;
        self.read_mode = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_flag = reader.read_bool()?;
        Ok(())
    }
}
