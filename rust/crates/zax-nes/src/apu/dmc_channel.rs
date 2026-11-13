use crate::state::{InvalidState, StateReader, StateWriter};
use crate::RegionTiming;

/// Timer periods in CPU cycles per output bit, NTSC (nesdev).
const NTSC_RATES: [i32; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

/// Timer periods in CPU cycles per output bit, PAL (nesdev, REQ-CORE-015).
const PAL_RATES: [i32; 16] = [
    398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
];

/// The delta modulation channel: memory reader, output unit and IRQ (nesdev, "APU DMC").
///
/// The channel does not own the bus. When its one-byte sample buffer runs dry it raises
/// [`DmcChannel::needs_sample_fetch`]; the bus then halts the CPU for the DMA and hands the
/// byte back through [`DmcChannel::supply_byte`].
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the hardware's independent flags.
pub(crate) struct DmcChannel {
    rates: &'static [i32; 16],
    rate_index: usize,
    timer: i32,
    irq_enabled: bool,
    looping: bool,
    sample_address: u16,
    sample_length: i32,
    current_address: u16,
    bytes_remaining: i32,
    sample_buffer: u8,
    buffer_empty: bool,
    shift_register: u8,
    bits_remaining: i32,
    silence: bool,
    /// Output level, 0 to 127 ($4011, and the delta decoder).
    output: i32,
    /// Whether the channel is pulling /IRQ low (bit 7 of $4015).
    irq_flag: bool,
}

impl Default for DmcChannel {
    fn default() -> Self {
        Self {
            rates: &NTSC_RATES,
            rate_index: 0,
            // One full period before the first bit, not zero: see `reset`.
            timer: NTSC_RATES[0] - 1,
            irq_enabled: false,
            looping: false,
            sample_address: 0xC000,
            sample_length: 1,
            current_address: 0xC000,
            bytes_remaining: 0,
            sample_buffer: 0,
            buffer_empty: true,
            shift_register: 0,
            bits_remaining: 8,
            silence: true,
            output: 0,
            irq_flag: false,
        }
    }
}

impl DmcChannel {
    /// Output level, 0 to 127.
    pub const fn output(&self) -> i32 {
        self.output
    }

    /// Whether the channel is pulling /IRQ low (bit 7 of $4015).
    pub const fn irq_flag(&self) -> bool {
        self.irq_flag
    }

    /// Whether a sample is still playing (bit 4 of $4015).
    pub const fn active(&self) -> bool {
        self.bytes_remaining > 0
    }

    /// Whether the memory reader wants a byte: the sample buffer is empty and bytes remain.
    ///
    /// The **load** fetch's start delay is deliberately *not* asked about here. It is an absolute
    /// cycle held by the bus ([`Self::set_enabled`] returns it, `NesBus::after_apu_write` stores
    /// it), because this question is asked once per CPU cycle on every game and a third test on
    /// it cost 10 instructions a cycle through [`Self::quiet_clocks`] and [`Self::clock_timer`]
    /// (lot MF-4a-bis step 1).
    pub const fn needs_sample_fetch(&self) -> bool {
        self.buffer_empty && self.bytes_remaining > 0
    }

    /// Address the next DMA fetch reads.
    pub const fn fetch_address(&self) -> u16 {
        self.current_address
    }

    /// CPU cycles from the channel's own position until [`Self::needs_sample_fetch`] could next
    /// turn true, or `None` while no sample is playing.
    ///
    /// The memory reader asks for a byte on the cycle the output unit takes the last bit out of
    /// the shift register and finds the sample buffer is the only thing left to refill it with
    /// ([`Self::clock_timer`]): the next bit plays `timer + 1` cycles from here, since the timer
    /// counts down once per CPU cycle and the bit plays on the cycle it is already 0 on, and each
    /// of the `bits_remaining - 1` bits after it a whole period later.
    ///
    /// `None` is the answer for a channel with no bytes left, and it is an answer about the
    /// **future** and not only about now: `bytes_remaining` can only grow again through a write
    /// to `$4015` or through [`Self::supply_byte`], so while the CPU is halted and no fetch runs
    /// — which is exactly the window [`crate::nes_bus::NesBus`] asks this for — nothing can make
    /// the reader ask. For the same reason the count returned is a count the caller may rely on
    /// to the cycle rather than a lower bound that drifts.
    ///
    /// Used by the bus to decide how much of an OAM transfer may run in its fast loop; it is
    /// never on a per-cycle path.
    pub const fn cycles_until_fetch(&self) -> Option<u32> {
        if self.bytes_remaining <= 0 {
            return None;
        }
        // `timer` is non-negative and `bits_remaining` is 1 to 8 everywhere outside
        // `clock_timer`'s own body; anything else is answered with "now", which only ever makes
        // the caller more careful than it has to be.
        if self.buffer_empty || self.timer < 0 || self.bits_remaining <= 0 {
            return Some(0);
        }

        let period = self.rates[self.rate_index].unsigned_abs();
        let bits_after_the_next = self.bits_remaining.unsigned_abs().saturating_sub(1);
        Some(
            self.timer
                .unsigned_abs()
                .saturating_add(1)
                .saturating_add(bits_after_the_next.saturating_mul(period)),
        )
    }

    /// Selects the NTSC or the PAL rate table (REQ-CORE-015).
    pub fn set_timing(&mut self, timing: &RegionTiming) {
        self.rates = if timing.uses_pal_noise_and_dmc_periods {
            &PAL_RATES
        } else {
            &NTSC_RATES
        };
    }

    /// Writes register `index` (0 to 3): $4010 to $4013.
    pub fn write(&mut self, index: u16, value: u8) {
        match index {
            0 => {
                self.irq_enabled = (value & 0x80) != 0;
                self.looping = (value & 0x40) != 0;

                // **A write on the very cycle the timer reloads uses the new period.**
                // `AccuracyCoin`'s `DMC Reload Timing` walks a `$4010` write across the reload
                // one cycle at a time and keys the six outcomes: a write 3, 2, 1 or **0** cycles
                // before the reload takes the new rate, and one 1 or 2 cycles after it does not
                // ("if the write to $4010 happens on the same cycle as the timer reloading, then
                // the sample rate changes, and the DMC timer takes the new value").
                //
                // A write earlier than the reload needs nothing: [`Self::clock_timer`] reads
                // `rates[rate_index]` at the instant it reloads, not a cached period. Only the
                // same-cycle case is missing, and it is missing because the bus performs a write
                // *after* stepping the APU — so that reload has already happened and has taken
                // the old period.
                //
                // `timer` standing at exactly `period - 1` is what says the reload was this
                // cycle: the reload is the only thing that sets that value, and the timer leaves
                // it on the next clock, so nothing but a write on the reload's own cycle can see
                // it. Reloading again with the new period is then the whole of the rule, and it
                // costs a register write and nothing per cycle.
                // The index is masked to the table's sixteen entries so that this read needs no
                // bounds check. `rate_index` is always 0 to 15, but LLVM cannot see that here —
                // the field's last writer is another call — and without the mask it emits a
                // bounds check and a `core::panicking::panic_bounds_check` **call site inside
                // `NesBus::write_internal`**, which is MF-3b rule 1's stop-the-lot event:
                // measured at +5 instructions and a cold panic block. The mask folds away.
                let reloaded_this_cycle = self.timer == self.rates[self.rate_index & 0x0F] - 1;

                let index = usize::from(value & 0x0F);
                self.rate_index = index;
                if reloaded_this_cycle {
                    // `index` is 0 to 15 by construction, so this one is already checkless.
                    self.timer = self.rates[index] - 1;
                }

                // "Disabling IRQ flag should clear it" (apu_test 7-dmc_basics).
                if !self.irq_enabled {
                    self.irq_flag = false;
                }
            }
            1 => self.output = i32::from(value & 0x7F),
            2 => self.sample_address = 0xC000 + (u16::from(value) << 6),
            _ => self.sample_length = (i32::from(value) << 4) + 1,
        }
    }

    /// Applies bit 4 of a $4015 write: starts the sample when it is set and nothing is
    /// playing, stops it when it is clear. A $4015 write always clears the DMC IRQ flag
    /// (nesdev).
    ///
    /// Returns the **load** fetch's start delay in CPU cycles, 0 when the write queued no load.
    /// The fetch a start asks for is not asked for on the write's own cycle: it waits two cycles
    /// when the write lands on an even CPU cycle and three when it lands on an odd one, which is
    /// what makes a load DMA "typically take 3 cycles" against a reload's four (nesdev, "DMA").
    /// `apu_cycle` is this core's name for that parity — it is the same bit the `$4017` write
    /// delay reads (`apu_reset/4017_timing`). Mesen 2 does the same, from `AccuracyCoin`'s
    /// "Delta Modulation Channel" page; without it blargg's `sync_dmc` locks onto a phase three
    /// cycles away from the hardware's and `sprdma_and_dmc_dma` measures the DMC fetch in the
    /// wrong place.
    ///
    /// The delay is **returned** instead of being counted down here. The channel is clocked once
    /// per CPU cycle of every game, and a counter in it had to be tested in
    /// [`Self::clock_timer`], [`Self::quiet_clocks`] and [`Self::skip_quiet_clocks`] — about 10
    /// instructions a cycle whether a sample plays or not (lot MF-4a-bis step 1). The bus turns
    /// the delay into one absolute cycle at this write and gates its own DMA path on it, which
    /// costs the cycles that run no DMA nothing at all.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    pub fn set_enabled(&mut self, enabled: bool, apu_cycle: bool) -> u32 {
        self.irq_flag = false;

        if !enabled {
            self.bytes_remaining = 0;
            return 0;
        }

        if self.bytes_remaining == 0 {
            self.restart();
            return if apu_cycle { 2 } else { 3 };
        }

        0
    }

    /// Timer clocks from now on which the timer only counts down: [`Self::clock_timer`]
    /// returns `false` on each of them.
    #[allow(clippy::cast_sign_loss)] // Checked non-negative.
    pub const fn quiet_clocks(&self) -> u32 {
        if self.timer > 0 {
            self.timer as u32
        } else {
            0
        }
    }

    /// Counts the timer down by `clocks`, at most [`Self::quiet_clocks`].
    #[allow(clippy::cast_possible_wrap)] // At most a timer period.
    pub fn skip_quiet_clocks(&mut self, clocks: u32) {
        self.timer -= clocks as i32;
    }

    /// Clocks the output unit's timer; called once per CPU cycle.
    /// Clocks the timer; `true` when a bit was played (the output may have changed).
    pub fn clock_timer(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
            return false;
        }

        self.timer = self.rates[self.rate_index] - 1;

        if !self.silence {
            if (self.shift_register & 1) != 0 {
                if self.output <= 125 {
                    self.output += 2;
                }
            } else if self.output >= 2 {
                self.output -= 2;
            }
        }

        self.shift_register >>= 1;

        self.bits_remaining -= 1;
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;

            if self.buffer_empty {
                self.silence = true;
            } else {
                self.silence = false;
                self.shift_register = self.sample_buffer;
                self.buffer_empty = true;
            }
        }

        true
    }

    /// Hands the byte the DMA read over to the memory reader. Advances the address, which
    /// wraps from $FFFF back to $8000, and ends or loops the sample when the last byte has
    /// been consumed (nesdev).
    ///
    /// Returns `Some(cycles)` in the one case the hardware asks for **another** fetch it then takes
    /// back: a one-byte sample that does not loop, whose byte lands on the APU cycle before the
    /// output unit's bit counter starts over. "If this happens right before the bit counter
    /// resets, a DMA is triggered and aborted 1 cycle later (causing one halted CPU cycle)"
    /// (Mesen 2, `DeltaModulationChannel::SetDmcReadBuffer`; `AccuracyCoin`, `Implicit DMA
    /// Abort`). The count is the cycles until that counter starts over; the bus owns the DMA
    /// cycles and does the rest (`NesBus::arm_implicit_dmc_abort`).
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    pub fn supply_byte(&mut self, value: u8) -> Option<u32> {
        self.sample_buffer = value;
        self.buffer_empty = false;
        self.current_address = if self.current_address == 0xFFFF {
            0x8000
        } else {
            self.current_address + 1
        };

        self.bytes_remaining -= 1;
        if self.bytes_remaining > 0 {
            return None;
        }

        if self.looping {
            self.restart();
            return None;
        }
        if self.irq_enabled {
            self.irq_flag = true;
        }
        if self.sample_length == 1 && self.bits_remaining == 1 && (0..2).contains(&self.timer) {
            return Some(self.timer.unsigned_abs() + 1);
        }
        None
    }

    fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    /// Back to the power-on state; the rate table stays with the region.
    ///
    /// The timer starts a **whole period** from its first bit rather than firing on the first
    /// CPU cycle, which fixes the output unit's phase — and with it the phase of every sample
    /// fetch the memory reader asks for, since every entry of both rate tables is even.
    ///
    /// That phase is what decides whether a reload fetch needs its alignment cycle: a request
    /// raised on an odd CPU cycle is served three cycles later, one raised on an even cycle
    /// four (see [`NesBus::run_dma`]). The hardware takes four, which is what blargg's
    /// `sync_dmc` counts on — its fine-synchronisation loop is written to be one cycle longer
    /// than the DMC's 3424-cycle byte so that it drifts into alignment ("3421+4 clocks per
    /// iter"). Three would make the loop exactly 3424 and it would never converge, which hangs
    /// every ROM of `dmc_dma_during_read4` and `sprdma_and_dmc_dma`. Mesen 2 sets the same
    /// phase at reset for the same reason.
    ///
    /// [`NesBus::run_dma`]: crate::NesBus
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    pub fn reset(&mut self) {
        let rates = self.rates;
        *self = Self::default();
        self.rates = rates;
        self.timer = rates[0] - 1;
    }

    /// The documented *after reset* state of the DMC (AMD-044 REQ-CORE-122): the output level
    /// is `AND`ed with 1 — the upper six bits are cleared — and the IRQ flag goes away. The
    /// sample address, the length and the rate are kept, as nesdev describes; the channel's own
    /// "playing" state is stopped by the `$4015 = 0` the APU applies.
    pub fn soft_reset(&mut self) {
        self.output &= 1;
        self.irq_flag = false;
    }
}

impl DmcChannel {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_usize(self.rate_index);
        writer.write_i32(self.timer);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.looping);
        writer.write_u16(self.sample_address);
        writer.write_i32(self.sample_length);
        writer.write_u16(self.current_address);
        writer.write_i32(self.bytes_remaining);
        writer.write_u8(self.sample_buffer);
        writer.write_bool(self.buffer_empty);
        writer.write_u8(self.shift_register);
        writer.write_i32(self.bits_remaining);
        writer.write_bool(self.silence);
        writer.write_i32(self.output);
        writer.write_bool(self.irq_flag);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.rate_index = reader.read_index(self.rates.len())?;
        self.timer = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.looping = reader.read_bool()?;
        self.sample_address = reader.read_u16()?;
        self.sample_length = reader.read_i32()?;
        self.current_address = reader.read_u16()?;
        self.bytes_remaining = reader.read_i32()?;
        self.sample_buffer = reader.read_u8()?;
        self.buffer_empty = reader.read_bool()?;
        self.shift_register = reader.read_u8()?;
        self.bits_remaining = reader.read_i32()?;
        self.silence = reader.read_bool()?;
        self.output = reader.read_i32()?;
        self.irq_flag = reader.read_bool()?;
        Ok(())
    }
}
