//! The fast core's APU path (lot FAST-1, `docs/design/FAST-CORE.md`).
//!
//! [`Apu2A03::run_cpu_cycles`] already skips the quiet cycles, the ones on which no channel moves
//! and no frame-counter step falls. What stops a skip is the next timer reload of **any**
//! channel, audible or not: a pulse channel with its length counter at 0 or its volume at 0 still
//! reloads its timer every period, the noise channel's shift register still turns every 4 to 4 068
//! cycles, and each of those reloads costs a whole stepped cycle and a new
//! [`Apu2A03::quiet_cycles`] for nothing, since the channel's output stays 0.
//!
//! Here a silent channel (see `PulseChannel::silent`, `NoiseChannel::silent`) does not bound the
//! skip; it is advanced through the skipped cycles in one go, to the very timer, sequencer and
//! shift register its reloads would have left. A channel only stops being silent on a register
//! write (the bus catches the APU up first) or a frame-counter step (which bounds the skip), so
//! its output is 0 on every skipped cycle and the level is unchanged. The machine is therefore
//! the same as with [`Apu2A03::run_cpu_cycles`], cycle for cycle.

use super::Apu2A03;

impl Apu2A03 {
    /// [`Self::run_cpu_cycles`] for the fast core: the same cycles and the same state, with the
    /// silent channels left out of the quiet stretches' bound.
    pub fn run_cpu_cycles_fast(&mut self, mut count: u32) {
        while count > 0 {
            let skip = if count > 1 {
                self.fast_quiet_cycles().min(count - 1)
            } else {
                0
            };
            if skip > 0 {
                self.skip_fast_cycles(skip);
                count -= skip;
            }
            self.step_cpu_cycle();
            count -= 1;
        }
    }

    /// [`Self::quiet_cycles`] with the silent channels left out.
    #[allow(clippy::float_cmp, clippy::cast_sign_loss)] // Exact level; positive counts.
    #[must_use]
    pub fn fast_quiet_cycles(&self) -> u32 {
        if self.frame_irq_pending || self.level_dirty || self.resampler.level() != self.level {
            return 0;
        }

        let mut quiet = u32::MAX;
        if self.pending_sequencer_reset > 0 {
            quiet = (self.pending_sequencer_reset - 1) as u32;
        }
        let next_step = self.sequence()[self.frame_step];
        if next_step > self.frame_counter {
            quiet = quiet.min((next_step - self.frame_counter - 1) as u32);
        }

        let parity = u32::from(self.apu_cycle);
        for pulse in [&self.pulse1, &self.pulse2] {
            if !pulse.silent() {
                quiet = quiet.min(
                    pulse
                        .quiet_clocks()
                        .saturating_mul(2)
                        .saturating_add(parity),
                );
            }
        }
        if !self.noise.silent() {
            quiet = quiet.min(self.noise.quiet_clocks());
        }

        quiet
            .min(self.triangle.quiet_clocks())
            .min(self.dmc.quiet_clocks())
    }

    /// `cycles` cycles (at most [`Self::fast_quiet_cycles`]) in one go.
    #[allow(clippy::cast_possible_wrap)] // At most a frame's cycles.
    fn skip_fast_cycles(&mut self, cycles: u32) {
        self.readable_status = self.compose_status(0);
        self.cycles += i64::from(cycles);
        self.frame_counter += cycles as i32;
        if self.pending_sequencer_reset > 0 {
            self.pending_sequencer_reset -= cycles as i32;
        }

        let pulse_clocks = if self.apu_cycle {
            cycles / 2
        } else {
            cycles.div_ceil(2)
        };
        self.apu_cycle ^= (cycles & 1) != 0;
        for pulse in [&mut self.pulse1, &mut self.pulse2] {
            if pulse.silent() {
                pulse.advance_silent(pulse_clocks);
            } else {
                pulse.skip_quiet_clocks(pulse_clocks);
            }
        }
        if self.noise.silent() {
            self.noise.advance_silent(cycles);
        } else {
            self.noise.skip_quiet_clocks(cycles);
        }
        self.triangle.skip_quiet_clocks(cycles);
        self.dmc.skip_quiet_clocks(cycles);

        let ring = &self.ring;
        self.resampler
            .skip_cycles(cycles, &mut |sample| super::lock(ring).enqueue(sample));
    }
}
