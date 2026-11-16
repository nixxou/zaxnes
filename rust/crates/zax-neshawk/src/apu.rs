//! The 2A03's APU.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/APU.cs`. The five channel classes, the frame
//! sequencer and the mixer are here; the three members that reach into the machine —
//! `DMCUnit.Run`, `DMCUnit.Fetch` and `RunOneLast` — are `impl Nes` in [`crate::nes::apu`],
//! because in the C# they go through `nes.cpu`, `nes.ReadMemory` and `nes.strobe_joyport`.
//! MIT; see `LICENSE-BizHawk`.

#![allow(clippy::struct_excessive_bools)] // the C#'s own flags, one field each

pub(crate) const DMC_RATE_NTSC: [i32; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];
pub(crate) const DMC_RATE_PAL: [i32; 16] = [
    398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
];
const LENGTH_TABLE: [i32; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[rustfmt::skip]
const PULSE_DUTY: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // (12.5%)
    [0, 1, 1, 0, 0, 0, 0, 0], // (25%)
    [0, 1, 1, 1, 1, 0, 0, 0], // (50%)
    [1, 0, 0, 1, 1, 1, 1, 1], // (25% negated (75%))
];

#[rustfmt::skip]
const TRIANGLE_TABLE: [i32; 32] = [
    15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
];

const NOISE_TABLE_NTSC: [i32; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];
const NOISE_TABLE_PAL: [i32; 16] = [
    4, 7, 14, 30, 60, 88, 118, 148, 188, 236, 354, 472, 708, 944, 1890, 3778,
];

const SEQUENCER_LUT_NTSC: [&[i32]; 2] = [
    &[7457, 14913, 22371, 29830],
    &[7457, 14913, 22371, 29830, 37282],
];
const SEQUENCER_LUT_PAL: [&[i32]; 2] = [
    &[8313, 16627, 24939, 33254],
    &[8313, 16627, 24939, 33254, 41566],
];

// ------------------------------------------------------------------------------- PulseUnit

#[derive(Debug, Clone, Default)]
pub struct PulseUnit {
    pub unit: i32,

    // reg0
    duty_cnt: i32,
    env_loop: i32,
    env_constant: i32,
    env_cnt_value: i32,
    pub len_halt: bool,
    // reg1
    sweep_en: i32,
    sweep_divider_cnt: i32,
    sweep_negate: i32,
    sweep_shiftcount: i32,

    sweep_reload: bool,
    // reg2/3
    len_cnt: i32,
    pub timer_raw_reload_value: i32,
    pub timer_reload_value: i32,

    // misc..
    lenctr_en: i32,

    // state
    swp_divider_counter: i32,
    swp_silence: bool,
    duty_step: i32,
    timer_counter: i32,
    pub sample: i32,
    duty_value: bool,

    env_start_flag: i32,
    env_divider: i32,
    env_counter: i32,
    pub env_output: i32,
}

impl PulseUnit {
    #[must_use]
    pub fn new(unit: i32) -> Self {
        Self {
            unit,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn is_len_cnt_non_zero(&self) -> bool {
        self.len_cnt > 0
    }

    pub fn write_reg(&mut self, addr: i32, val: u8, len_clock_active: bool) {
        let v = i32::from(val);
        match addr {
            0 => {
                self.env_cnt_value = v & 0xF;
                self.env_constant = (v >> 4) & 1;
                self.env_loop = (v >> 5) & 1;
                self.duty_cnt = (v >> 6) & 3;
            }
            1 => {
                self.sweep_shiftcount = v & 7;
                self.sweep_negate = (v >> 3) & 1;
                self.sweep_divider_cnt = (v >> 4) & 7;
                self.sweep_en = (v >> 7) & 1;
                self.sweep_reload = true;
            }
            2 => {
                self.timer_reload_value = (self.timer_reload_value & 0x700) | v;
                self.timer_raw_reload_value = self.timer_reload_value * 2 + 2;
            }
            3 => {
                if len_clock_active {
                    if self.len_cnt == 0 {
                        self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize] + 1;
                    }
                } else {
                    self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize];
                }

                self.timer_reload_value = (self.timer_reload_value & 0xFF) | ((v & 0x07) << 8);
                self.timer_raw_reload_value = self.timer_reload_value * 2 + 2;
                self.duty_step = 0;
                self.env_start_flag = 1;

                // allow the lenctr_en to kill the len_cnt
                let e = self.lenctr_en;
                self.set_lenctr_en(e);
            }
            _ => {}
        }
    }

    pub fn set_lenctr_en(&mut self, value: i32) {
        self.lenctr_en = value;
        // if the length counter is not enabled, then we must disable the length system in this way
        if self.lenctr_en == 0 {
            self.len_cnt = 0;
        }
    }

    pub fn clock_length_and_sweep(&mut self) {
        // this should be optimized to update only when `timer_reload_value` changes
        let mut sweep_shifter = self.timer_reload_value >> self.sweep_shiftcount;
        if self.sweep_negate == 1 {
            sweep_shifter = -sweep_shifter - self.unit;
        }
        sweep_shifter += self.timer_reload_value;

        // this sweep logic is always enabled:
        self.swp_silence = self.timer_reload_value < 8 || sweep_shifter > 0x7FF;

        // does enable only block the pitch bend? does the clocking proceed?
        if self.sweep_en == 1 {
            // clock divider
            if self.swp_divider_counter != 0 {
                self.swp_divider_counter -= 1;
            }
            if self.swp_divider_counter == 0 {
                self.swp_divider_counter = self.sweep_divider_cnt + 1;

                // divider was clocked: process sweep pitch bend
                if self.sweep_shiftcount != 0 && !self.swp_silence {
                    self.timer_reload_value = sweep_shifter;
                    self.timer_raw_reload_value = (self.timer_reload_value << 1) + 2;
                }
            }

            // handle divider reload, after clocking happens
            if self.sweep_reload {
                self.swp_divider_counter = self.sweep_divider_cnt + 1;
                self.sweep_reload = false;
            }
        }

        // env_loop doubles as "halt length counter"
        if (self.env_loop == 0 || self.len_halt) && self.len_cnt > 0 {
            self.len_cnt -= 1;
        }
    }

    pub fn clock_env(&mut self) {
        if self.env_start_flag == 1 {
            self.env_start_flag = 0;
            self.env_divider = self.env_cnt_value;
            self.env_counter = 15;
        } else if self.env_divider != 0 {
            self.env_divider -= 1;
        } else {
            // `else if (env_divider == 0)` — always true here
            self.env_divider = self.env_cnt_value;
            if self.env_counter == 0 {
                if self.env_loop == 1 {
                    self.env_counter = 15;
                }
            } else {
                self.env_counter -= 1;
            }
        }
    }

    pub fn run(&mut self, recalculate: &mut bool) {
        if self.env_constant == 1 {
            self.env_output = self.env_cnt_value;
        } else {
            self.env_output = self.env_counter;
        }

        if self.timer_counter > 0 {
            self.timer_counter -= 1;
        }
        if self.timer_counter == 0 && self.timer_raw_reload_value != 0 {
            if self.duty_step == 7 {
                self.duty_step = 0;
            } else {
                self.duty_step += 1;
            }
            self.duty_value = PULSE_DUTY[self.duty_cnt as usize][self.duty_step as usize & 7] == 1;
            // reload timer
            self.timer_counter = self.timer_raw_reload_value;
        }

        let newsample = if self.duty_value {
            // high state of duty cycle
            if self.swp_silence || self.len_cnt == 0 {
                0 // silenced
            } else {
                self.env_output
            }
        } else {
            0 // duty cycle is 0, silenced.
        };

        if newsample != self.sample {
            *recalculate = true;
            self.sample = newsample;
        }
    }
}

// ------------------------------------------------------------------------------- NoiseUnit

#[derive(Debug, Clone)]
pub struct NoiseUnit {
    // reg0 (sweep)
    env_cnt_value: i32,
    env_loop: i32,
    env_constant: i32,
    pub len_halt: bool,

    // reg2 (mode and period)
    mode_cnt: i32,
    period_cnt: i32,

    // reg3 (length counter and envelop trigger)
    len_cnt: i32,

    // set from apu:
    lenctr_en: i32,

    // state
    shift_register: i32,
    timer_counter: i32,
    pub sample: i32,
    env_output: i32,
    env_start_flag: i32,
    env_divider: i32,
    env_counter: i32,
    noise_bit: bool,

    noise_table: &'static [i32; 16],
}

impl NoiseUnit {
    #[must_use]
    pub fn new(pal: bool) -> Self {
        Self {
            env_cnt_value: 0,
            env_loop: 0,
            env_constant: 0,
            len_halt: false,
            mode_cnt: 0,
            period_cnt: 0,
            len_cnt: 0,
            lenctr_en: 0,
            shift_register: 1,
            timer_counter: 0,
            sample: 0,
            env_output: 0,
            env_start_flag: 0,
            env_divider: 0,
            env_counter: 0,
            noise_bit: true,
            noise_table: if pal {
                &NOISE_TABLE_PAL
            } else {
                &NOISE_TABLE_NTSC
            },
        }
    }

    #[must_use]
    pub fn is_len_cnt_non_zero(&self) -> bool {
        self.len_cnt > 0
    }

    pub fn write_reg(&mut self, addr: i32, val: u8, len_clock_active: bool) {
        let v = i32::from(val);
        match addr {
            0 => {
                self.env_cnt_value = v & 0xF;
                self.env_constant = (v >> 4) & 1;
                // we want to delay a halt until after a length clock if they happen on the same cycle
                if self.env_loop == 0 && ((v >> 5) & 1) == 1 {
                    self.len_halt = true;
                }
                self.env_loop = (v >> 5) & 1;
            }
            1 => {}
            2 => {
                self.period_cnt = self.noise_table[(v & 0xF) as usize];
                self.mode_cnt = (v >> 7) & 1;
            }
            3 => {
                if len_clock_active {
                    if self.len_cnt == 0 {
                        self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize] + 1;
                    }
                } else {
                    self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize];
                }

                let e = self.lenctr_en;
                self.set_lenctr_en(e);
                self.env_start_flag = 1;
            }
            _ => {}
        }
    }

    pub fn set_lenctr_en(&mut self, value: i32) {
        self.lenctr_en = value;
        if self.lenctr_en == 0 {
            self.len_cnt = 0;
        }
    }

    pub fn clock_env(&mut self) {
        if self.env_start_flag == 1 {
            self.env_start_flag = 0;
            self.env_divider = self.env_cnt_value + 1;
            self.env_counter = 15;
        } else {
            // note: the C# tests `!= 0` then `== 0` in two *separate* ifs, so a divider that
            // reaches zero here is reloaded in the same call. Not an `else if`.
            if self.env_divider != 0 {
                self.env_divider -= 1;
            }
            if self.env_divider == 0 {
                self.env_divider = self.env_cnt_value + 1;
                if self.env_counter == 0 {
                    if self.env_loop == 1 {
                        self.env_counter = 15;
                    }
                } else {
                    self.env_counter -= 1;
                }
            }
        }
    }

    pub fn clock_length_and_sweep(&mut self) {
        if self.len_cnt > 0 && (self.env_loop == 0 || self.len_halt) {
            self.len_cnt -= 1;
        }
    }

    pub fn run(&mut self, recalculate: &mut bool) {
        if self.env_constant == 1 {
            self.env_output = self.env_cnt_value;
        } else {
            self.env_output = self.env_counter;
        }

        if self.timer_counter > 0 {
            self.timer_counter -= 1;
        }
        if self.timer_counter == 0 && self.period_cnt != 0 {
            // reload timer
            self.timer_counter = self.period_cnt;
            let feedback_bit = if self.mode_cnt == 1 {
                (self.shift_register >> 6) & 1
            } else {
                (self.shift_register >> 1) & 1
            };
            let feedback = feedback_bit ^ (self.shift_register & 1);
            self.shift_register >>= 1;
            self.shift_register &= !(1 << 14);
            self.shift_register |= feedback << 14;
            self.noise_bit = (self.shift_register & 1) != 0;
        }

        let newsample = if self.len_cnt == 0 {
            0
        } else if self.noise_bit {
            self.env_output // switched, was 0?
        } else {
            0
        };
        if newsample != self.sample {
            *recalculate = true;
            self.sample = newsample;
        }
    }
}

// ------------------------------------------------------------------------------- TriangleUnit

#[derive(Debug, Clone, Default)]
pub struct TriangleUnit {
    // reg0
    linear_counter_reload: i32,
    control_flag: i32,
    // reg2/3
    timer_cnt: i32,
    reload_flag: i32,
    len_cnt: i32,
    pub halt_2: bool,
    // misc..
    lenctr_en: i32,
    linear_counter: i32,
    timer: i32,
    timer_cnt_reload: i32,
    seq: i32,
    pub sample: i32,
}

impl TriangleUnit {
    #[must_use]
    pub fn is_len_cnt_non_zero(&self) -> bool {
        self.len_cnt > 0
    }

    pub fn set_lenctr_en(&mut self, value: i32) {
        self.lenctr_en = value;
        if self.lenctr_en == 0 {
            self.len_cnt = 0;
        }
    }

    pub fn write_reg(&mut self, addr: i32, val: u8, len_clock_active: bool) {
        let v = i32::from(val);
        match addr {
            0 => {
                self.linear_counter_reload = v & 0x7F;
                self.control_flag = (v >> 7) & 1;
            }
            1 => {}
            2 => {
                self.timer_cnt = (self.timer_cnt & !0xFF) | v;
                self.timer_cnt_reload = self.timer_cnt + 1;
            }
            3 => {
                self.timer_cnt = (self.timer_cnt & 0xFF) | ((v & 0x7) << 8);
                self.timer_cnt_reload = self.timer_cnt + 1;
                if len_clock_active {
                    if self.len_cnt == 0 {
                        self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize] + 1;
                    }
                } else {
                    self.len_cnt = LENGTH_TABLE[((v >> 3) & 0x1F) as usize];
                }
                self.reload_flag = 1;

                // allow the lenctr_en to kill the len_cnt
                let e = self.lenctr_en;
                self.set_lenctr_en(e);
            }
            _ => {}
        }
    }

    pub fn run(&mut self, recalculate: &mut bool) {
        // when clocked by timer, seq steps forward
        // except when linear counter or length counter is 0
        let en = self.len_cnt != 0 && self.linear_counter != 0;

        let mut do_clock = false;
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            do_clock = true;
            self.timer = self.timer_cnt_reload;
        }

        if en && do_clock {
            self.seq = (self.seq + 1) & 0x1F;

            let mut newsample = TRIANGLE_TABLE[self.seq as usize];

            // special hack: frequently, games will use the maximum frequency triangle in order to
            // mute it; the DAC then outputs a steady level at about 7.5
            if self.timer_cnt_reload == 1 {
                newsample = 8;
            }

            if newsample != self.sample {
                *recalculate = true;
                self.sample = newsample;
            }
        }
    }

    pub fn clock_length_and_sweep(&mut self) {
        // env_loop doubles as "halt length counter"
        if self.len_cnt > 0 && self.control_flag == 0 {
            self.len_cnt -= 1;
        }
    }

    pub fn clock_linear_counter(&mut self) {
        if self.reload_flag == 1 {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter != 0 {
            self.linear_counter -= 1;
        }

        if self.control_flag == 0 {
            self.reload_flag = 0;
        }
    }
}

// ------------------------------------------------------------------------------- DMCUnit

#[derive(Debug, Clone)]
pub struct DmcUnit {
    pub dmc_rate: &'static [i32; 16],

    pub irq_enabled: bool,
    pub loop_flag: bool,
    pub timer_reload: i32,

    /// dmc delay per visual 2a03
    pub delay: i32,

    /// this timer never stops, ever, so it is convenient to use for even/odd timing used elsewhere
    pub timer: i32,
    pub user_address: i32,
    pub user_length: u32,
    pub sample_length: u32,
    pub sample_address: i32,
    pub sample_buffer: i32,
    pub sample_buffer_filled: bool,

    pub out_shift: i32,
    pub out_bits_remaining: i32,
    pub out_deltacounter: i32,
    pub out_silence: bool,
    /// happens when buffer is filled and emptied at the same time
    pub fill_glitch: bool,
    /// happens when a write triggered refill that sets length to zero happens too close to an
    /// automatic DMA (causes 1-cycle blips in dmc_dma_start_test_v2)
    pub fill_glitch_2: bool,
    pub fill_glitch_2_end: bool,

    pub pending_disable: bool,

    pub timer_just_reloaded: bool,
}

impl DmcUnit {
    #[must_use]
    pub fn new(pal: bool) -> Self {
        let dmc_rate = if pal { &DMC_RATE_PAL } else { &DMC_RATE_NTSC };
        Self {
            dmc_rate,
            irq_enabled: false,
            loop_flag: false,
            timer_reload: dmc_rate[0],
            delay: 0,
            // confirmed in VisualNES, on console it seems the APU runs a couple cycles before
            // CPU exits reset
            timer: 1020,
            user_address: 0xC000,
            user_length: 1,
            sample_length: 0,
            sample_address: 0xC000,
            sample_buffer: 0,
            sample_buffer_filled: false,
            out_shift: 0,
            out_bits_remaining: 7, //confirmed in VisualNES
            out_deltacounter: 64,
            out_silence: true,
            fill_glitch: false,
            fill_glitch_2: false,
            fill_glitch_2_end: false,
            pending_disable: false,
            timer_just_reloaded: false,
        }
    }

    /// `DMCUnit.sample`.
    #[must_use]
    pub fn sample(&self) -> i32 {
        self.out_deltacounter
    }

    /// `DMCUnit.Clock`.
    pub fn clock(&mut self, recalculate: &mut bool) {
        // If the silence flag is clear, bit 0 of the shift register is applied to the counter
        if !self.out_silence {
            // apply current sample bit to delta counter
            if (self.out_shift & 1) != 0 {
                if self.out_deltacounter < 126 {
                    self.out_deltacounter += 2;
                }
            } else if self.out_deltacounter > 1 {
                self.out_deltacounter -= 2;
            }
            *recalculate = true;
        }

        // The right shift register is clocked.
        self.out_shift >>= 1;

        // The bits-remaining counter is decremented. If it becomes zero, a new cycle is started.
        if self.out_bits_remaining == 0 {
            // The bits-remaining counter is loaded with 8.
            self.out_bits_remaining = 7;
            // If the sample buffer is empty then the silence flag is set
            if !self.sample_buffer_filled {
                self.out_silence = true;
            } else {
                // otherwise, the silence flag is cleared and the sample buffer is emptied into
                // the shift register.
                self.out_silence = false;
                self.out_shift = self.sample_buffer;
                self.sample_buffer_filled = false;
            }
        } else {
            self.out_bits_remaining -= 1;
        }
    }

    #[must_use]
    pub fn is_len_cnt_non_zero(&self) -> bool {
        self.sample_length != 0
    }
}

// ------------------------------------------------------------------------------- APU

#[derive(Debug, Clone)]
pub struct Apu {
    pub m_vol: i32,

    pub dmc_dma_countdown: i32,
    pub dmc_rdy_check: i32,
    pub call_from_write: bool,

    pub recalculate: bool,

    pub pulse: [PulseUnit; 2],
    pub triangle: TriangleUnit,
    pub noise: NoiseUnit,
    pub dmc: DmcUnit,

    pub(crate) irq_pending: bool,
    pub dmc_reload_countdown: i32,
    pub dmc_irq: bool,
    pub dmc_irq_glitch: bool,
    pub(crate) pending_reg: i32,
    pub(crate) doing_tick_quarter: bool,
    pub(crate) pending_val: u8,
    pub seq_tick: i32,
    pub seq_val: u8,
    pub len_clock_active: bool,

    pub(crate) sequencer_counter: i32,
    pub(crate) sequencer_step: i32,
    pub(crate) sequencer_mode: i32,
    pub(crate) sequencer_irq_inhibit: i32,
    pub(crate) sequencer_irq_assert: i32,
    pub(crate) sequencer_irq: bool,
    /// `APU.sequence_reset_pending`, which the C# savestates and never reads. Kept so that the
    /// two structs line up field for field.
    #[allow(dead_code)]
    pub(crate) sequence_reset_pending: bool,
    pub(crate) sequencer_irq_clear_pending: bool,
    pub(crate) sequencer_irq_flag: bool,

    pub(crate) sequencer_lut: [&'static [i32]; 2],
    pub(crate) sequencer_check_1: i32,
    pub(crate) sequencer_check_2: i32,

    pub sampleclock: u32,
    pub(crate) oldmix: i32,
    pub(crate) cart_sound: i32,
    pub(crate) old_cart_sound: i32,
}

impl Apu {
    /// `new APU(this, old, pal)`. `old` only carries `m_vol` across a hard reset.
    #[must_use]
    pub fn new(old: Option<&Apu>, pal: bool) -> Self {
        Self {
            m_vol: old.map_or(1, |o| o.m_vol),
            dmc_dma_countdown: -1,
            dmc_rdy_check: 0,
            call_from_write: false,
            recalculate: false,
            pulse: [PulseUnit::new(1), PulseUnit::new(0)],
            triangle: TriangleUnit::default(),
            noise: NoiseUnit::new(pal),
            dmc: DmcUnit::new(pal),
            irq_pending: false,
            dmc_reload_countdown: 0,
            dmc_irq: false,
            dmc_irq_glitch: false,
            pending_reg: -1,
            doing_tick_quarter: false,
            pending_val: 0,
            seq_tick: 0,
            seq_val: 0,
            len_clock_active: false,
            sequencer_counter: 0,
            sequencer_step: 0,
            sequencer_mode: 0,
            sequencer_irq_inhibit: 0,
            sequencer_irq_assert: 0,
            sequencer_irq: false,
            sequence_reset_pending: false,
            sequencer_irq_clear_pending: false,
            sequencer_irq_flag: false,
            sequencer_lut: if pal {
                SEQUENCER_LUT_PAL
            } else {
                SEQUENCER_LUT_NTSC
            },
            sequencer_check_1: 0,
            sequencer_check_2: 0,
            sampleclock: 0,
            oldmix: 0,
            cart_sound: 0,
            old_cart_sound: 0,
        }
    }

    /// `DMCUnit.set_lenctr_en`. On the `Apu` rather than on `DmcUnit` because the C# body reaches
    /// back through `apu.` for six of its fields.
    pub fn dmc_set_lenctr_en(&mut self, en: bool) {
        if !en {
            // in these cases, the disable happens right as the reload begins, it is cancelled
            // similar to a write triggered reload cancelling an automatic reload, and has the
            // same timing (still uses fill_glitch_2)
            if (self.dmc.timer == 3 || self.dmc.timer == 2)
                && self.dmc.out_bits_remaining == 0
                && self.dmc.sample_length != 0
            {
                self.dmc.sample_length = 0;
                self.dmc.fill_glitch_2 = true;
                self.dmc_irq = false;
                self.sync_irq();
                return;
            }

            // in these cases the disable happens too late and the reload (and potential IRQ)
            // still happen
            if self.dmc.timer == 1
                && self.dmc.out_bits_remaining == 0
                && self.dmc.sample_length != 0
            {
                self.dmc.pending_disable = true;
                self.dmc_irq = false;
                self.sync_irq();
                return;
            }

            // if a fetch / reload is in progress, writing here has no immediate effect
            // but it seems IRQs can be cancelled after the fetch before IRQ
            if self.dmc_dma_countdown > 0 || self.dmc.delay != 0 || self.dmc_reload_countdown != 0 {
                if self.dmc_reload_countdown != 0 {
                    self.dmc_irq_glitch = true;
                }

                self.dmc.pending_disable = true;
            } else {
                self.dmc.sample_length = 0;
            }
        } else {
            // only start playback if playback is stopped
            if self.dmc.sample_length == 0 {
                self.dmc.sample_address = self.dmc.user_address;
                self.dmc.sample_length = self.dmc.user_length;
            }
            if !self.dmc.sample_buffer_filled {
                // apparently the dmc is different if called from a cpu write, let's try
                self.call_from_write = true;
            }
        }

        // irq is acknowledged or sure to be clear, in either case
        self.dmc_irq = false;
        self.sync_irq();
    }

    /// `DMCUnit.WriteReg`.
    fn dmc_write_reg(&mut self, addr: i32, val: u8) {
        match addr {
            0 => {
                self.dmc.irq_enabled = (val & 0x80) != 0;
                self.dmc.loop_flag = (val & 0x40) != 0;
                self.dmc.timer_reload = self.dmc.dmc_rate[(val & 0xF) as usize];
                if !self.dmc.irq_enabled {
                    self.dmc_irq = false;
                }
                self.sync_irq();
            }
            1 => {
                self.dmc.out_deltacounter = i32::from(val & 0x7F);
                self.recalculate = true;
            }
            2 => {
                self.dmc.user_address = 0xC000 | (i32::from(val) << 6);
            }
            3 => {
                self.dmc.user_length = (u32::from(val) << 4) + 1;
            }
            _ => {}
        }
    }

    /// `APU.sequencer_write_tick`.
    pub(crate) fn sequencer_write_tick(&mut self, val: u8) {
        if self.seq_tick > 0 {
            self.seq_tick -= 1;

            if self.seq_tick == 0 {
                self.sequencer_mode = (i32::from(val) >> 7) & 1;

                // check if we will be doing the extra frame ticks or not
                if self.sequencer_mode == 1 && !self.doing_tick_quarter {
                    self.quarter_frame();
                    self.half_frame();
                }

                self.sequencer_irq_inhibit = (i32::from(val) >> 6) & 1;
                if self.sequencer_irq_inhibit == 1 {
                    self.sequencer_irq_flag = false;
                }

                self.sequencer_counter = 0;
                self.sequencer_step = 0;

                if self.sequencer_mode == 0 {
                    self.sequencer_check_2 = self.sequencer_lut[0][3] - 2;
                } else {
                    self.sequencer_check_2 = self.sequencer_lut[1][4] - 2;
                }
            }
        }
    }

    /// `APU.sequencer_tick`.
    pub(crate) fn sequencer_tick(&mut self) {
        self.sequencer_counter += 1;
        if self.sequencer_mode == 0 && self.sequencer_counter == self.sequencer_lut[0][3] - 1 {
            if self.sequencer_irq_inhibit == 0 {
                self.sequencer_irq_assert = 2;
                self.sequencer_irq_flag = true;
            }

            self.half_frame();
        }
        if self.sequencer_mode == 0
            && self.sequencer_counter == self.sequencer_lut[0][3] - 2
            && self.sequencer_irq_inhibit == 0
        {
            self.sequencer_irq_flag = true;
        }
        if self.sequencer_mode == 1 && self.sequencer_counter == self.sequencer_lut[1][4] - 1 {
            self.half_frame();
        }
        if self.sequencer_lut[self.sequencer_mode as usize][self.sequencer_step as usize]
            != self.sequencer_counter
        {
            return;
        }
        self.sequencer_check();
    }

    pub fn sync_irq(&mut self) {
        self.irq_pending = self.sequencer_irq | self.dmc_irq;
    }

    /// `APU.sequencer_check`.
    fn sequencer_check(&mut self) {
        #[allow(clippy::match_like_matches_macro, clippy::single_match_else)]
        let (quarter, half, reset) = match self.sequencer_mode {
            0 => {
                // 4-step
                let quarter = true;
                let half = self.sequencer_step == 1;
                let reset = self.sequencer_step == 3;
                if reset && self.sequencer_irq_inhibit == 0 {
                    self.sequencer_irq_flag = true;
                }
                (quarter, half, reset)
            }
            _ => {
                // 5-step
                let quarter = self.sequencer_step != 3;
                let half = self.sequencer_step == 1;
                let reset = self.sequencer_step == 4;
                (quarter, half, reset)
            }
        };

        if reset {
            self.sequencer_counter = 0;
            self.sequencer_step = 0;
        } else {
            self.sequencer_step += 1;
        }

        if quarter {
            self.quarter_frame();
        }
        if half {
            self.half_frame();
        }
    }

    fn half_frame(&mut self) {
        self.doing_tick_quarter = true;
        self.pulse[0].clock_length_and_sweep();
        self.pulse[1].clock_length_and_sweep();
        self.triangle.clock_length_and_sweep();
        self.noise.clock_length_and_sweep();
    }

    fn quarter_frame(&mut self) {
        self.doing_tick_quarter = true;
        self.pulse[0].clock_env();
        self.pulse[1].clock_env();
        self.triangle.clock_linear_counter();
        self.noise.clock_env();
    }

    /// `APU.NESSoftReset`.
    pub fn nes_soft_reset(&mut self) {
        // need to study what happens to apu and stuff..
        self.sequencer_irq = false;
        self.sequencer_irq_flag = false;
        self.write_reg_internal(0x4015, 0);

        // for 4017, its as if the last value written gets rewritten
        self.sequencer_mode = (i32::from(self.seq_val) >> 7) & 1;
        self.sequencer_irq_inhibit = (i32::from(self.seq_val) >> 6) & 1;
        if self.sequencer_irq_inhibit == 1 {
            self.sequencer_irq_flag = false;
        }
        self.sequencer_counter = 0;
        self.sequencer_step = 0;

        self.sequencer_check_1 = self.sequencer_lut[0][1] - 1;

        if self.sequencer_mode == 0 {
            self.sequencer_check_2 = self.sequencer_lut[0][3] - 2;
        } else {
            self.sequencer_check_2 = self.sequencer_lut[1][4] - 2;
        }

        self.dmc.fill_glitch = false;
        self.dmc.fill_glitch_2 = false;
    }

    /// `APU.NESHardReset`.
    pub fn nes_hard_reset(&mut self) {
        // "at power on it is as if $00 was written to $4017 9-12 cycles before the reset vector"
        // DMC seems to run for a couple cycles after reset, so aim for the upper end (12)
        self.sequencer_counter = 2;

        self.sequencer_check_1 = self.sequencer_lut[0][1] - 1;

        if self.sequencer_mode == 0 {
            self.sequencer_check_2 = self.sequencer_lut[0][3] - 2;
        } else {
            self.sequencer_check_2 = self.sequencer_lut[1][4] - 2;
        }

        self.dmc.fill_glitch = false;
        self.dmc.fill_glitch_2 = false;
    }

    /// `APU.WriteReg`: a write is only latched, and applied by `RunOneLast`.
    pub fn write_reg(&mut self, addr: i32, val: u8) {
        self.pending_reg = addr;
        self.pending_val = val;
    }

    /// `APU._WriteReg`.
    pub fn write_reg_internal(&mut self, addr: i32, val: u8) {
        let index = addr - 0x4000;
        let reg = index & 3;
        let channel = index >> 2;
        let lca = self.len_clock_active;
        match channel {
            0 => self.pulse[0].write_reg(reg, val, lca),
            1 => self.pulse[1].write_reg(reg, val, lca),
            2 => self.triangle.write_reg(reg, val, lca),
            3 => self.noise.write_reg(reg, val, lca),
            4 => self.dmc_write_reg(reg, val),
            5 => {
                if addr == 0x4015 {
                    self.pulse[0].set_lenctr_en(i32::from(val) & 1);
                    self.pulse[1].set_lenctr_en((i32::from(val) >> 1) & 1);
                    self.triangle.set_lenctr_en((i32::from(val) >> 2) & 1);
                    self.noise.set_lenctr_en((i32::from(val) >> 3) & 1);
                    self.dmc_set_lenctr_en((val & 0x10) != 0);
                } else if addr == 0x4017 {
                    if self.dmc.timer % 2 == 0 {
                        self.seq_tick = 3;
                    } else {
                        self.seq_tick = 4;
                    }

                    self.seq_val = val;
                }
            }
            _ => {}
        }
    }

    /// `APU.PeekReg`.
    #[must_use]
    pub fn peek_reg(&self, addr: i32) -> u8 {
        if addr == 0x4015 {
            //notice a missing bit here. should properly emulate with empty / Data bus
            let dmc_nonzero = i32::from(self.dmc.is_len_cnt_non_zero());
            let noise_nonzero = i32::from(self.noise.is_len_cnt_non_zero());
            let tri_nonzero = i32::from(self.triangle.is_len_cnt_non_zero());
            let pulse1_nonzero = i32::from(self.pulse[1].is_len_cnt_non_zero());
            let pulse0_nonzero = i32::from(self.pulse[0].is_len_cnt_non_zero());
            let ret = (i32::from(self.dmc_irq) << 7)
                | (i32::from(self.sequencer_irq_flag) << 6)
                | (dmc_nonzero << 4)
                | (noise_nonzero << 3)
                | (tri_nonzero << 2)
                | (pulse1_nonzero << 1)
                | pulse0_nonzero;
            return ret as u8;
        }
        // don't return 0xFF here or SMB will break
        0x00
    }

    /// `APU.ReadReg`.
    pub fn read_reg(&mut self, addr: i32) -> u8 {
        if addr == 0x4015 {
            let ret = self.peek_reg(0x4015);
            self.sequencer_irq_clear_pending = true;
            self.sync_irq();
            return ret;
        }
        // don't return 0xFF here or SMB will break
        0x00
    }

    /// `APU.RunOneFirst`, minus `dmc.Run()`, which needs the machine. The caller runs the DMC
    /// immediately after, in the C#'s order.
    pub fn run_one_first_before_dmc(&mut self) {
        let rec = &mut self.recalculate;
        self.pulse[0].run(rec);
        self.pulse[1].run(rec);
        self.triangle.run(rec);
        self.noise.run(rec);
    }

    /// The tail of `RunOneFirst`, after `dmc.Run()`.
    pub fn run_one_first_after_dmc(&mut self) {
        self.pulse[0].len_halt = false;
        self.pulse[1].len_halt = false;
        self.noise.len_halt = false;
    }

    /// `APU.ExternalQueue`, which only a board with sound calls from `ClockCPU`.
    pub fn external_queue(&mut self, value: i32) {
        self.cart_sound = value + self.old_cart_sound;

        if self.cart_sound != self.old_cart_sound {
            self.recalculate = true;
            self.old_cart_sound = self.cart_sound;
        }
    }

    /// `APU.EmitSample`.
    pub fn emit_sample(&mut self) -> i32 {
        if self.recalculate {
            self.recalculate = false;

            let s_pulse0 = self.pulse[0].sample;
            let s_pulse1 = self.pulse[1].sample;
            let s_tri = self.triangle.sample;
            let s_noise = self.noise.sample;
            let s_dmc = self.dmc.sample();

            // more properly correct
            let pulse_out = if s_pulse0 == 0 && s_pulse1 == 0 {
                0.0f32
            } else {
                95.88f32 / ((8128.0f32 / (s_pulse0 + s_pulse1) as f32) + 100.0f32)
            };

            let tnd_out = if s_tri == 0 && s_noise == 0 && s_dmc == 0 {
                0.0f32
            } else {
                159.79f32
                    / (1.0f32
                        / ((s_tri as f32 / 8227.0f32)
                            + (s_noise as f32 / 12241.0f32)
                            + (s_dmc as f32 / 22638.0f32))
                        + 100.0f32)
            };

            let output = pulse_out + tnd_out;

            // this needs to leave enough headroom for straying DC bias due to the DMC unit
            // getting stuck outputs. smb3 is bad about that.
            self.oldmix =
                (20000.0f32 * output * (1 + self.m_vol / 5) as f32) as i32 + self.cart_sound;
        }

        self.oldmix
    }
}

// -------------------------------------------------------------------- the savestate

// `APU.SyncState` and the four units' own. The `skip:` groups are the three `&'static` lookup
// tables the C# holds as `static readonly` arrays — constants, not state.
crate::state::sync_struct!(PulseUnit {
    unit,
    duty_cnt,
    env_loop,
    env_constant,
    env_cnt_value,
    len_halt,
    sweep_en,
    sweep_divider_cnt,
    sweep_negate,
    sweep_shiftcount,
    sweep_reload,
    len_cnt,
    timer_raw_reload_value,
    timer_reload_value,
    lenctr_en,
    swp_divider_counter,
    swp_silence,
    duty_step,
    timer_counter,
    sample,
    duty_value,
    env_start_flag,
    env_divider,
    env_counter,
    env_output,
});

crate::state::sync_struct!(NoiseUnit {
    env_cnt_value, env_loop, env_constant, len_halt, mode_cnt, period_cnt,
    len_cnt, lenctr_en, shift_register, timer_counter, sample, env_output,
    env_start_flag, env_divider, env_counter, noise_bit,
} skip: {
    noise_table,
});

crate::state::sync_struct!(TriangleUnit {
    linear_counter_reload,
    control_flag,
    timer_cnt,
    reload_flag,
    len_cnt,
    halt_2,
    lenctr_en,
    linear_counter,
    timer,
    timer_cnt_reload,
    seq,
    sample,
});

crate::state::sync_struct!(DmcUnit {
    irq_enabled, loop_flag, timer_reload, delay, timer,
    user_address, user_length, sample_length, sample_address,
    sample_buffer, sample_buffer_filled,
    out_shift, out_bits_remaining, out_deltacounter, out_silence,
    fill_glitch, fill_glitch_2, fill_glitch_2_end, pending_disable, timer_just_reloaded,
} skip: {
    dmc_rate,
});

crate::state::sync_struct!(Apu {
    m_vol, dmc_dma_countdown, dmc_rdy_check, call_from_write, recalculate,
    pulse, triangle, noise, dmc,
    irq_pending, dmc_reload_countdown, dmc_irq, dmc_irq_glitch,
    pending_reg, doing_tick_quarter, pending_val, seq_tick, seq_val, len_clock_active,
    sequencer_counter, sequencer_step, sequencer_mode, sequencer_irq_inhibit,
    sequencer_irq_assert, sequencer_irq, sequence_reset_pending,
    sequencer_irq_clear_pending, sequencer_irq_flag,
    sequencer_check_1, sequencer_check_2,
    sampleclock, oldmix, cart_sound, old_cart_sound,
} skip: {
    sequencer_lut,
});
