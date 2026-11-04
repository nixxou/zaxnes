use super::mixer;
use crate::state::{InvalidState, StateReader, StateWriter};

/// The Sunsoft 5B's expansion audio: the YM2149F (an AY-3-8910) the chip carries — three square
/// channels, a noise generator and an envelope generator (REQ-CORE-075, optional; Q-118 for the
/// level).
///
/// Written from the public nesdev page "Sunsoft 5B audio". **Mesen 2 was not adaptable here**: its
/// `Sunsoft5bAudio` keeps the sixteen registers and the three tone generators but its
/// `UpdateOutputLevel` only ever looks at the tone, so it has neither the envelope nor the noise
/// (`Sunsoft5bAudio.h:63-74`). Only its volume table agrees with the page, and that agreement is
/// recorded in [`mixer::SUNSOFT_5B_TABLE`].
///
/// **Two addresses, sixteen registers.** `$C000-$DFFF` selects one of the chip's registers and
/// `$E000-$FFFF` writes it:
///
/// | register | bits | meaning |
/// | --- | --- | --- |
/// | `$00`/`$01`, `$02`/`$03`, `$04`/`$05` | `LLLL LLLL` / `---- HHHH` | 12-bit period of channel A, B, C |
/// | `$06` | `---P PPPP` | noise period |
/// | `$07` | `--CB Acba` | noise disable on C/B/A, tone disable on c/b/a |
/// | `$08`-`$0A` | `---E VVVV` | envelope enable and volume of A, B, C |
/// | `$0B`/`$0C` | `LLLL LLLL` / `HHHH HHHH` | 16-bit envelope period |
/// | `$0D` | `---- CAaH` | envelope reset and shape: continue, attack, alternate, hold |
/// | `$0E`, `$0F` | — | I/O ports, unused |
///
/// **The clock, and why one prescaler serves all three generators.** The page gives the tone and
/// the noise as `Clock / (32 × Period)` and the envelope *step* as `Clock / (16 × Period)`, with
/// the clock at 1.789773 MHz — the CPU's. Those are the AY's own datasheet relations for a chip
/// clocked at half that: a tone toggles every 16 × period CPU cycles, the noise shifts every
/// 32 × period, and the envelope steps every 16 × period. So a single divide-by-16 of the CPU
/// clock drives everything, and on each of its ticks a tone counter is one step, the noise counter
/// two, and the envelope counter one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sunsoft5bAudio {
    /// The sixteen registers, exactly as written.
    registers: [u8; 16],
    /// What `$C000` selected; values above $0F select nothing writable.
    selected: u8,
    /// Divide-by-16 of the CPU clock: the chip's own tick.
    prescaler: u8,
    /// One down-counter and one square output per channel.
    tone_counter: [u16; 3],
    tone_high: [bool; 3],
    /// The noise down-counter, in chip ticks, and the "17-bit linear feedback shift register with
    /// taps at bits 16 and 13".
    noise_counter: u8,
    noise_shift: u32,
    envelope_counter: u16,
    /// The envelope's "5-bit series of output levels", 0 to 31.
    envelope_level: u8,
    /// Whether the envelope is on a rising ramp (the shape register's `A`, then flipped by `a`).
    envelope_rising: bool,
    /// Whether the shape machine has stopped on its final value (`H`, or the end of a `C = 0`
    /// shape).
    envelope_holding: bool,
    /// `M` of REQ-CORE-073 as it stands, recomputed only when a generator or a register moves —
    /// so [`Sunsoft5bAudio::output`], which the bus reads every CPU cycle, is one load.
    level: i32,
}

impl Sunsoft5bAudio {
    /// CPU cycles per chip tick: the divide-by-16 above.
    const PRESCALER: u8 = 16;

    /// The LFSR's two taps, bits 16 and 13 of a 17-bit register.
    const NOISE_TAPS: u32 = (1 << 16) | (1 << 13);

    /// A board's sound at power-on (REQ-CORE-004). Every register is 0, which leaves all three
    /// volumes at 0 — silence — whatever `$07` says.
    pub fn new() -> Self {
        Self {
            registers: [0; 16],
            selected: 0,
            prescaler: Self::PRESCALER,
            tone_counter: [0; 3],
            tone_high: [false; 3],
            noise_counter: 0,
            // "produces a 1-bit random output": an all-zero shift register would never leave
            // zero, so the chip's comes up with a bit set.
            noise_shift: 1,
            envelope_counter: 0,
            envelope_level: 0,
            envelope_rising: false,
            envelope_holding: false,
            level: 0,
        }
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units, a **positive**
    /// quantity the APU subtracts (Q-118).
    pub const fn output(&self) -> i32 {
        self.level
    }

    /// 12-bit period of channel `channel`; 0 behaves as 1, as the AY's counters do.
    const fn tone_period(&self, channel: usize) -> u16 {
        let low = self.registers[channel * 2] as u16;
        let high = (self.registers[channel * 2 + 1] & 0x0F) as u16;
        let period = (high << 8) | low;
        if period == 0 {
            1
        } else {
            period
        }
    }

    /// `$06`, five bits, doubled because the noise shifts every 32 × period CPU cycles where a
    /// tone toggles every 16 × period.
    const fn noise_period_ticks(&self) -> u8 {
        let period = self.registers[6] & 0x1F;
        if period == 0 {
            2
        } else {
            period * 2
        }
    }

    /// The 16-bit envelope period of `$0B`/`$0C`, in chip ticks; 0 behaves as 1.
    const fn envelope_period(&self) -> u16 {
        let period = ((self.registers[0x0C] as u16) << 8) | self.registers[0x0B] as u16;
        if period == 0 {
            1
        } else {
            period
        }
    }

    /// Advances the board's sound by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        self.prescaler -= 1;
        if self.prescaler != 0 {
            return;
        }

        self.prescaler = Self::PRESCALER;
        self.step_tick();
    }

    /// One tick of the chip's divide-by-16 clock.
    fn step_tick(&mut self) {
        for channel in 0..3 {
            if self.tone_counter[channel] == 0 {
                self.tone_counter[channel] = self.tone_period(channel);
                self.tone_high[channel] = !self.tone_high[channel];
            }

            self.tone_counter[channel] -= 1;
        }

        if self.noise_counter == 0 {
            self.noise_counter = self.noise_period_ticks();
            self.step_noise();
        }

        self.noise_counter -= 1;

        if !self.envelope_holding {
            if self.envelope_counter == 0 {
                self.envelope_counter = self.envelope_period();
                self.step_envelope();
            }

            self.envelope_counter -= 1;
        }

        self.refresh_level();
    }

    /// "a 17-bit linear feedback shift register with taps at bits 16 and 13".
    fn step_noise(&mut self) {
        let feedback = (self.noise_shift & Self::NOISE_TAPS).count_ones() & 1;
        self.noise_shift = ((self.noise_shift << 1) | feedback) & 0x0001_FFFF;
    }

    /// One step of the envelope's shape machine.
    ///
    /// `$0D` is `---- CAaH` — "envelope reset and shape: continue (C), attack (A), alternate (a),
    /// hold (H)". The ramp runs to one end of the 0-31 range and then the three remaining bits
    /// decide what happens, which is the AY's eight documented shapes:
    ///
    /// * `C = 0`: one ramp, then 0 for ever, whichever way the ramp went;
    /// * `C = 1`, `H = 1`: stop, after `a` has had its chance to flip the direction — so `a`
    ///   decides whether it stops at the far end or back at the start;
    /// * `C = 1`, `H = 0`: go again, reversed if `a` and from the same end if not.
    fn step_envelope(&mut self) {
        let shape = self.registers[0x0D];
        let continues = shape & 0x08 != 0;
        let alternate = shape & 0x02 != 0;
        let hold = shape & 0x01 != 0;

        let last = if self.envelope_rising { 31 } else { 0 };
        if self.envelope_level != last {
            self.envelope_level = if self.envelope_rising {
                self.envelope_level + 1
            } else {
                self.envelope_level - 1
            };
            return;
        }

        if !continues {
            self.envelope_level = 0;
            self.envelope_holding = true;
            return;
        }

        if alternate {
            self.envelope_rising = !self.envelope_rising;
        }

        if hold {
            self.envelope_level = if self.envelope_rising { 31 } else { 0 };
            self.envelope_holding = true;
        } else {
            self.envelope_level = if self.envelope_rising { 0 } else { 31 };
        }
    }

    /// Recomputes `M` from the three channels.
    ///
    /// Per channel the chip gates its amplitude with the tone and the noise: `$07` holds a
    /// *disable* per generator per channel — "A bit of 0 enables the noise/tone on the specified
    /// channel, and a bit of 1 disables it" — and a disabled generator reads as a permanent 1, so
    /// "If both bits are 1, the channel outputs a constant signal at the specified volume". The
    /// amplitude is the envelope's own five-bit level when `$08`-`$0A` bit 4 says so, and
    /// `2V + 1` otherwise (Q-118).
    fn refresh_level(&mut self) {
        let mixer_register = self.registers[7];
        let noise_high = self.noise_shift & 1 != 0;
        let mut level = 0;
        for channel in 0..3 {
            let tone_disabled = mixer_register >> channel & 0x01 != 0;
            let noise_disabled = mixer_register >> (channel + 3) & 0x01 != 0;
            let open = (tone_disabled || self.tone_high[channel]) && (noise_disabled || noise_high);
            if !open {
                continue;
            }

            let volume = self.registers[8 + channel];
            let index = if volume & 0x10 != 0 {
                self.envelope_level as usize
            } else {
                mixer::sunsoft_5b_level(volume)
            };
            level += mixer::SUNSOFT_5B_TABLE[index.min(mixer::SUNSOFT_5B_LEVELS - 1)];
        }

        self.level = level;
    }

    /// A CPU write to `$C000-$FFFF`: the register select and the register write of the board
    /// (nesdev, "Sunsoft 5B audio").
    pub fn write(&mut self, address: u16, value: u8) {
        if address & 0x2000 == 0 {
            self.selected = value;
            return;
        }

        // "chooses one of 16 internal audio registers": a select above $0F names none of them,
        // and the write is lost.
        let register = usize::from(self.selected);
        if register > 0x0F {
            return;
        }

        self.registers[register] = value;

        // "envelope reset and shape": a write to $0D restarts the ramp at the end the attack bit
        // names, and lets a held envelope go again.
        if register == 0x0D {
            self.envelope_rising = value & 0x04 != 0;
            self.envelope_level = if self.envelope_rising { 0 } else { 31 };
            self.envelope_holding = false;
            self.envelope_counter = self.envelope_period();
        }

        self.refresh_level();
    }

    /// Appends the sound state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.registers);
        writer.write_u8(self.selected);
        writer.write_u8(self.prescaler);
        for channel in 0..3 {
            writer.write_u16(self.tone_counter[channel]);
            writer.write_bool(self.tone_high[channel]);
        }

        writer.write_u8(self.noise_counter);
        writer.write_u32(self.noise_shift);
        writer.write_u16(self.envelope_counter);
        writer.write_u8(self.envelope_level);
        writer.write_bool(self.envelope_rising);
        writer.write_bool(self.envelope_holding);
    }

    /// Restores what [`Sunsoft5bAudio::save_state`] wrote (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.registers)?;
        self.selected = reader.read_u8()?;
        self.prescaler = reader.read_u8()?;
        for channel in 0..3 {
            self.tone_counter[channel] = reader.read_u16()?;
            self.tone_high[channel] = reader.read_bool()?;
        }

        self.noise_counter = reader.read_u8()?;
        self.noise_shift = reader.read_u32()?;
        self.envelope_counter = reader.read_u16()?;
        self.envelope_level = reader.read_u8()?;
        self.envelope_rising = reader.read_bool()?;
        self.envelope_holding = reader.read_bool()?;

        // A prescaler of 0 would underflow on the next cycle, and an envelope level above 31
        // would index past the table; neither can be written by this build.
        if self.prescaler == 0 || self.prescaler > Self::PRESCALER || self.envelope_level > 31 {
            return Err(InvalidState::corrupt(
                "Sunsoft 5B audio: impossible prescaler or envelope level.",
            ));
        }

        self.refresh_level();
        Ok(())
    }
}
