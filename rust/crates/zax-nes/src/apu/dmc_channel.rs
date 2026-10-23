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
            timer: 0,
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
    pub const fn needs_sample_fetch(&self) -> bool {
        self.buffer_empty && self.bytes_remaining > 0
    }

    /// Address the next DMA fetch reads.
    pub const fn fetch_address(&self) -> u16 {
        self.current_address
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
                self.rate_index = usize::from(value & 0x0F);

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
    pub fn set_enabled(&mut self, enabled: bool) {
        self.irq_flag = false;

        if !enabled {
            self.bytes_remaining = 0;
            return;
        }

        if self.bytes_remaining == 0 {
            self.restart();
        }
    }

    /// Clocks the output unit's timer; called once per CPU cycle.
    pub fn clock_timer(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
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
    }

    /// Hands the byte the DMA read over to the memory reader. Advances the address, which
    /// wraps from $FFFF back to $8000, and ends or loops the sample when the last byte has
    /// been consumed (nesdev).
    pub fn supply_byte(&mut self, value: u8) {
        self.sample_buffer = value;
        self.buffer_empty = false;
        self.current_address = if self.current_address == 0xFFFF {
            0x8000
        } else {
            self.current_address + 1
        };

        self.bytes_remaining -= 1;
        if self.bytes_remaining > 0 {
            return;
        }

        if self.looping {
            self.restart();
        } else if self.irq_enabled {
            self.irq_flag = true;
        }
    }

    fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    /// Back to the power-on state; the rate table stays with the region.
    pub fn reset(&mut self) {
        let rates = self.rates;
        *self = Self::default();
        self.rates = rates;
    }
}
