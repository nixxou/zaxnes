// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The three generators are the shape of `Vrc6Pulse.h`, `Vrc6Saw.h` and `Vrc6Audio.h`, which
// transcribe the same nesdev page and agree with it line for line; the one place Mesen is the
// deciding source is the sawtooth's "every 2 clocks", read below. See THIRD-PARTY-NOTICES.md.

use super::mixer;
use crate::state::{InvalidState, StateReader, StateWriter};

/// One of the VRC6's two pulse channels (nesdev, "VRC6 audio").
///
/// | register | bits | meaning |
/// | --- | --- | --- |
/// | +0 | `MDDD VVVV` | mode (ignore duty), duty, 4-bit volume |
/// | +1 | `LLLL LLLL` | low 8 bits of the 12-bit period |
/// | +2 | `E... HHHH` | enable, high 4 bits of the period |
///
/// "The CPU clock rate (1.79 MHz) drives the 12-bit divider F. Every cycle the divider counts
/// down until it reaches zero, at which point the divider resets and the duty cycle generator is
/// clocked." The generator has sixteen steps and "When the current step is less than or equal to
/// the given duty cycle D, the channel volume V is output, otherwise 0 is output" — so `D` of 0
/// to 7 is a duty of 1/16 to 8/16, the page's "6.25% to 50%". "When the mode bit M is true, the
/// channel ignores the duty cycle generator and outputs the current volume regardless."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vrc6Pulse {
    volume: u8,
    duty: u8,
    ignore_duty: bool,
    period: u16,
    enabled: bool,
    /// The divider. `i32` because it is compared against zero after the decrement, which is how
    /// the hardware's "counts down until it reaches zero" reads with a reload of `period + 1`.
    timer: i32,
    step: u8,
}

impl Vrc6Pulse {
    const fn new() -> Self {
        Self {
            volume: 0,
            duty: 0,
            ignore_duty: false,
            period: 0,
            enabled: false,
            timer: 1,
            step: 0,
        }
    }

    /// A write to +0, +1 or +2 of this channel's group.
    fn write(&mut self, register: u16, value: u8) {
        match register & 0x03 {
            0 => {
                self.volume = value & 0x0F;
                self.duty = (value & 0x70) >> 4;
                self.ignore_duty = value & 0x80 != 0;
            }
            1 => self.period = (self.period & 0x0F00) | u16::from(value),
            2 => {
                self.period = (self.period & 0x00FF) | (u16::from(value & 0x0F) << 8);
                self.enabled = value & 0x80 != 0;
                if !self.enabled {
                    // "When the channel is disabled by clearing the E bit, output is forced to
                    // 0, and the duty cycle is immediately reset and halted; it will resume from
                    // the beginning when E is once again set."
                    self.step = 0;
                }
            }
            _ => {}
        }
    }

    /// One CPU cycle; true when the duty generator stepped, so that the caller only recomputes
    /// the mix when it can have changed.
    fn step_cpu_cycle(&mut self, shift: u8) -> bool {
        if !self.enabled {
            return false;
        }

        self.timer -= 1;
        if self.timer > 0 {
            return false;
        }

        self.step = (self.step + 1) & 0x0F;
        self.timer = i32::from(self.period >> shift) + 1;
        true
    }

    /// The channel's 4-bit contribution to the DAC, 0 to 15.
    const fn output(&self) -> usize {
        if !self.enabled {
            0
        } else if self.ignore_duty || self.step <= self.duty {
            self.volume as usize
        } else {
            0
        }
    }

    fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.volume);
        writer.write_u8(self.duty);
        writer.write_bool(self.ignore_duty);
        writer.write_u16(self.period);
        writer.write_bool(self.enabled);
        writer.write_i32(self.timer);
        writer.write_u8(self.step);
    }

    fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.volume = reader.read_u8()?;
        self.duty = reader.read_u8()?;
        self.ignore_duty = reader.read_bool()?;
        self.period = reader.read_u16()?;
        self.enabled = reader.read_bool()?;
        self.timer = reader.read_i32()?;
        self.step = reader.read_u8()?;
        Ok(())
    }
}

/// The VRC6's sawtooth channel (nesdev, "VRC6 audio").
///
/// | register | bits | meaning |
/// | --- | --- | --- |
/// | +0 | `..AA AAAA` | 6-bit accumulator rate |
/// | +1 | `LLLL LLLL` | low 8 bits of the period |
/// | +2 | `E... HHHH` | enable, high 4 bits of the period |
///
/// "the accumulator only reacts on every 2 clocks. When clocked, the rate value A is added to an
/// internal 8-bit accumulator. The high 5 bits of the accumulator are then output. […] After A
/// has been added 6 times, on the 7th clock, instead of A being added, the internal accumulator
/// is reset to zero." So the phase runs over **fourteen** divider steps — six additions on the
/// even ones and a reset at the top — which is why "it divides the clock by 14 instead of 16,
/// unlike the pulse channels" and `f = CPU / (14 × (t + 1))`.
///
/// Mesen is the deciding source for *which* fourteen steps add: `_step = (_step + 1) % 14`, then
/// reset at step 0 and add on every even step (`Vrc6Saw.h:53-68`). That gives the page's six
/// additions and its one reset per fourteen, and it is the only reading of "every 2 clocks" that
/// does.
///
/// "If A is more than 42 (floor(255 / 6)), the accumulator will wrap, resulting in distorted
/// sound" — so the addition is a wrapping one on eight bits, deliberately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vrc6Saw {
    rate: u8,
    accumulator: u8,
    period: u16,
    enabled: bool,
    timer: i32,
    step: u8,
}

impl Vrc6Saw {
    /// The phase's fourteen divider steps.
    const STEPS: u8 = 14;

    const fn new() -> Self {
        Self {
            rate: 0,
            accumulator: 0,
            period: 0,
            enabled: false,
            timer: 1,
            step: 0,
        }
    }

    fn write(&mut self, register: u16, value: u8) {
        match register & 0x03 {
            0 => self.rate = value & 0x3F,
            1 => self.period = (self.period & 0x0F00) | u16::from(value),
            2 => {
                self.period = (self.period & 0x00FF) | (u16::from(value & 0x0F) << 8);
                self.enabled = value & 0x80 != 0;
                if !self.enabled {
                    // "The phase of the saw generator can be mostly reset by clearing and
                    // immediately setting E. Clearing E does not reset the frequency divider,
                    // however, so the first step of the reset saw may appear shortened" — the
                    // phase and the accumulator go, the divider stays.
                    self.accumulator = 0;
                    self.step = 0;
                }
            }
            _ => {}
        }
    }

    fn step_cpu_cycle(&mut self, shift: u8) -> bool {
        if !self.enabled {
            return false;
        }

        self.timer -= 1;
        if self.timer > 0 {
            return false;
        }

        self.step = (self.step + 1) % Self::STEPS;
        self.timer = i32::from(self.period >> shift) + 1;
        if self.step == 0 {
            self.accumulator = 0;
        } else if self.step & 0x01 == 0 {
            self.accumulator = self.accumulator.wrapping_add(self.rate);
        }

        true
    }

    /// "The high 5 bits of the accumulator are then output (provided the channel is enabled by
    /// having the E bit set)."
    const fn output(&self) -> usize {
        if self.enabled {
            (self.accumulator >> 3) as usize
        } else {
            0
        }
    }

    fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.rate);
        writer.write_u8(self.accumulator);
        writer.write_u16(self.period);
        writer.write_bool(self.enabled);
        writer.write_i32(self.timer);
        writer.write_u8(self.step);
    }

    fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.rate = reader.read_u8()?;
        self.accumulator = reader.read_u8()?;
        self.period = reader.read_u16()?;
        self.enabled = reader.read_bool()?;
        self.timer = reader.read_i32()?;
        self.step = reader.read_u8()?;
        Ok(())
    }
}

/// The VRC6's expansion audio: "3 extra channels for sound: two pulse waves, and one sawtooth"
/// (REQ-CORE-075, optional; the level is [`mixer::VRC6_TABLE`], which the page fixes — see
/// Q-118 for the question it does *not* leave open here).
///
/// The register map, all of it reached through `address & 0xF003`:
///
/// | address | channel |
/// | --- | --- |
/// | `$9000`-`$9002` | pulse 1 |
/// | `$9003` | frequency scaling and halt, for all three |
/// | `$A000`-`$A002` | pulse 2 |
/// | `$B000`-`$B002` | sawtooth |
///
/// `$9003` is `.... .ADH`: "Halt (bit 0) halts all oscillators, stopping them in their current
/// state", "16x frequency (bit 1)" is a "4 octave increase" and "256x frequency (bit 2)" an
/// "8 octave increase", with "The halt flag overrides the other flags. The 256x flag overrides
/// the 16x flag." A 16× rise of the frequency is the period shifted right by 4 and a 256× rise by
/// 8, which is how the scaling reaches the two generators: as a shift applied to the period at
/// reload time, not as a second divider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Vrc6Audio {
    pulse1: Vrc6Pulse,
    pulse2: Vrc6Pulse,
    saw: Vrc6Saw,
    /// `$9003` bit 0.
    halted: bool,
    /// 0, 4 or 8, from `$9003` bits 1 and 2.
    shift: u8,
    /// `M` of REQ-CORE-073 as it stands, recomputed only when a generator steps or a register
    /// moves — so [`Vrc6Audio::output`], which the bus reads every CPU cycle, is one load. The
    /// three generators run off the CPU clock itself, so "only when something moved" is what
    /// keeps the per-cycle path to three tests on a quiet chip.
    level: i32,
}

impl Vrc6Audio {
    /// A board's sound at power-on (REQ-CORE-004): every `E` bit clear, so silence whatever the
    /// volumes say.
    pub const fn new() -> Self {
        Self {
            pulse1: Vrc6Pulse::new(),
            pulse2: Vrc6Pulse::new(),
            saw: Vrc6Saw::new(),
            halted: false,
            shift: 0,
            level: 0,
        }
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units, a **positive**
    /// quantity the APU subtracts — the page's "(except inverted)".
    pub const fn output(&self) -> i32 {
        self.level
    }

    /// Advances the three generators by one CPU cycle.
    ///
    /// **Out of line, measured.** `Vrc6::on_cpu_cycle` is one arm of `Board::on_cpu_cycle`, and
    /// the thirty arms of that dispatch **share one prologue** — so a register this body needs
    /// is a register every board saves on every CPU cycle. Inlined, it took the prologue from
    /// `pushq %rsi / subq $32, %rsp` to `pushq %rsi / pushq %rdi / subq $40, %rsp`: one more
    /// push and pop per CPU cycle for an NROM that has no expansion audio at all, which is
    /// MF-3b rule 3 ("on a dispatch, the arm that matters is the empty one") in its exact form.
    ///
    /// The Sunsoft 5B escapes this by accident: its generators run off a divide-by-16, so its
    /// own `step_cpu_cycle` is three instructions and the work is behind a second function. The
    /// VRC6's three oscillators are clocked by the CPU clock itself, so there is no cheap
    /// early-out to inline in its place and the whole body has to stay out.
    ///
    /// What it costs the board that wants it: one `callq`/`retq` per CPU cycle, which is the
    /// same trade `NesBus::push_expansion_audio` and `NesBus::run_dma` carry their own notes
    /// about.
    #[inline(never)]
    pub fn step_cpu_cycle(&mut self) {
        if self.halted {
            return;
        }

        let shift = self.shift;
        // Not `||`: all three must be clocked, so the short circuit would be a bug. Written as
        // three statements rather than `a | b | c` so that is obvious.
        let mut stepped = self.pulse1.step_cpu_cycle(shift);
        stepped |= self.pulse2.step_cpu_cycle(shift);
        stepped |= self.saw.step_cpu_cycle(shift);
        if stepped {
            self.refresh_level();
        }
    }

    /// Recomputes `M` from the 6-bit DAC.
    fn refresh_level(&mut self) {
        // 15 + 15 + 31 = 61 by construction, so the clamp is unreachable; it is there because
        // it is what makes the index provably in range and keeps a bounds check off this path
        // (the trap MF-4d step 2 met in `NesBus::write_internal`).
        let dac = self.pulse1.output() + self.pulse2.output() + self.saw.output();
        self.level = mixer::VRC6_TABLE[dac.min(mixer::VRC6_LEVELS - 1)];
    }

    /// A CPU write to `$9000`-`$9003`, `$A000`-`$A002` or `$B000`-`$B002`.
    pub fn write(&mut self, address: u16, value: u8) {
        match address & 0xF003 {
            0x9003 => {
                self.halted = value & 0x01 != 0;
                // "The 256x flag overrides the 16x flag."
                self.shift = if value & 0x04 != 0 {
                    8
                } else if value & 0x02 != 0 {
                    4
                } else {
                    0
                };
            }
            0x9000..=0x9002 => self.pulse1.write(address, value),
            0xA000..=0xA002 => self.pulse2.write(address, value),
            0xB000..=0xB002 => self.saw.write(address, value),
            _ => return,
        }

        self.refresh_level();
    }

    /// Appends the sound state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.pulse1.save_state(writer);
        self.pulse2.save_state(writer);
        self.saw.save_state(writer);
        writer.write_bool(self.halted);
        writer.write_u8(self.shift);
    }

    /// Restores what [`Vrc6Audio::save_state`] wrote (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.pulse1.load_state(reader)?;
        self.pulse2.load_state(reader)?;
        self.saw.load_state(reader)?;
        self.halted = reader.read_bool()?;
        self.shift = reader.read_u8()?;

        // A shift this build never writes would divide the period by more than 256, and a saw
        // phase past its fourteenth step would never reach the reset again.
        if !matches!(self.shift, 0 | 4 | 8) || self.saw.step >= Vrc6Saw::STEPS {
            return Err(InvalidState::corrupt(
                "VRC6 audio: impossible frequency scaling or saw phase.",
            ));
        }

        self.refresh_level();
        Ok(())
    }
}
