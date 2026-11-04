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
/// of its two squares *and* their length counters are clocked together at a fixed cadence of
/// about 240 Hz, every [`ENVELOPE_CYCLES`] CPU cycles. Neither follows `$4017`, which is the
/// difference a game would hear if the APU's sequencer were reused — hence the dedicated divider
/// below. AMD-061 §6.3 withdraws AMD-054's 120 Hz divergence on the length counters: "Length
/// counter operates twice as fast as the APU length counter" and both are "fixed to a 240Hz
/// update rate" (nesdev, "MMC5 audio").
///
/// **PCM channel.** Both modes, selected by bit 0 of $5010 (AMD-061 §6.3). In write mode a
/// non-zero write to $5011 sets the level and a write of 0 leaves the level alone and raises the
/// PCM IRQ flag; in read mode a CPU read of $8000-$BFFF puts the byte read into the level, and a
/// byte of 0 raises the flag. Reading $5010 returns the flag in bit 7 and acknowledges it. When
/// bit 7 of $5010 allows it, the flag **drives the CPU's /IRQ line**: nesdev's own pseudocode is
/// `Cart /IRQ line = NOT(irqTrip AND irqEnable)`, and §6.3 withdraws AMD-054's divergence that
/// had it drive nothing.
///
/// **Output.** `M = r(K · P(q1 + q2)) + r(K · T(pcm))`, on the APU's own two curves and its own
/// `K` (AMD-061 §6.4); the APU **subtracts** it from its `A`, because the board's polarity is
/// reversed (§6.2). See [`mixer`] for both.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the board's independent hardware flags.
pub(crate) struct Mmc5Audio {
    pulse1: Mmc5PulseChannel,
    pulse2: Mmc5PulseChannel,

    cycles: i64,
    half_rate: bool,
    divider: i32,

    pcm: u8,
    read_mode: bool,
    irq_enabled: bool,
    irq_flag: bool,
}

impl Mmc5Audio {
    /// A board's sound at power-on, silent (REQ-CORE-004).
    pub fn new() -> Self {
        Self::default()
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units, a **positive**
    /// quantity the APU subtracts (REQ-CORE-075 as amended by AMD-061 §6.2 and §6.4).
    ///
    /// The two squares read the APU's own pulse table on their own summed index, so one of them
    /// at volume 15 contributes what one APU pulse at volume 15 does and the two of them what
    /// two APU pulses do (§6.5's first two criteria); the PCM level reads the APU's own
    /// triangle/noise/DMC table, which is the DMC's curve, over a full byte.
    ///
    /// Q-115: §6.4's parenthesis says the pulse table is "appliquée voie par voie", which would
    /// give 9 780 for two squares at volume 15 where §6.5's second criterion asks for 8 462.
    /// The criteria win — they are what the amendment declares normative, and one index shared
    /// by both squares is what "equivalent in volume to the corresponding APU channels" means
    /// for a pair of APU pulses. The ticket carries the contradiction.
    pub fn output(&self) -> i32 {
        let index = self.pulse1.output() + self.pulse2.output();
        mixer::PULSE_TABLE[usize::try_from(index)
            .unwrap_or(0)
            .min(mixer::PULSE_INDEX_MAX)]
            + mixer::MMC5_PCM_TABLE[usize::from(self.pcm)]
    }

    /// Whether $5010 put the PCM channel in read mode, where the board takes its level from the
    /// CPU's reads of $8000-$BFFF instead of from writes to $5011 (AMD-061 §6.3).
    pub const fn pcm_read_mode(&self) -> bool {
        self.read_mode
    }

    /// Feeds the PCM channel the byte a CPU read of $8000-$BFFF put on the bus, which read mode
    /// turns into the level; a byte of 0 raises the PCM IRQ flag instead, exactly as a write of
    /// 0 does in write mode (AMD-061 §6.3).
    pub fn feed_pcm_read(&mut self, value: u8) {
        if value == 0 {
            self.irq_flag = true;
        } else {
            self.pcm = value;
        }
    }

    /// Whether the PCM IRQ is pulling the CPU's /IRQ line: nesdev's
    /// `Cart /IRQ line = NOT(irqTrip AND irqEnable)` (AMD-061 §6.3, which withdraws AMD-054's
    /// divergence that had the flag drive nothing).
    pub const fn irq_asserted(&self) -> bool {
        self.irq_flag && self.irq_enabled
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

        // AMD-061 §6.3: the length counters are clocked on *every* envelope clock, at the same
        // 240 Hz, where AMD-054 clocked them every other one at 120 Hz.
        self.divider = 0;
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.pulse1.length.clock(self.cycles);
        self.pulse2.length.clock(self.cycles);
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
                // (REQ-CORE-075).
                //
                // Q-115: AMD-061 §6.3 adds the read mode but leaves this clause of AMD-054
                // unamended, so the write still takes even in read mode. Mesen ignores it there
                // (`if(!_pcmReadMode)`), which is probably closer to the hardware; no known game
                // writes $5011 while in read mode, so the ticket records the choice rather than
                // guessing against the requirement's own words.
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
        // Reserved. AMD-061 §6.3 put the length counters on the envelope's own 240 Hz, so the
        // `length_turn` flag that used to pick every other clock is gone; its byte is kept so
        // that the block layout — and with it the savestate format version — does not move, in
        // either direction (REQ-CORE-091).
        writer.write_bool(false);
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
        reader.read_bool()?; // The reserved byte; see `save_state`.
        self.pcm = reader.read_u8()?;
        self.read_mode = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_flag = reader.read_bool()?;
        Ok(())
    }
}
