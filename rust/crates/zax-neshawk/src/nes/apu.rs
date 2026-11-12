//! The three APU members that reach into the machine.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/APU.cs`: `DMCUnit.Run` (which drives
//! `nes.cpu.RDY` and `nes.dmc_dma_exec`), `DMCUnit.Fetch` (which reads the CPU bus), and
//! `RunOneLast` (which strobes the joypads and publishes `nes._irq_apu`). They are `impl Bus`
//! for that reason; everything else is in [`crate::apu`].
//! MIT; see `LICENSE-BizHawk`.

use super::Bus;

impl Bus {
    /// `APU.RunOneFirst`.
    pub(crate) fn apu_run_one_first(&mut self) {
        self.apu.run_one_first_before_dmc();
        self.dmc_run();
        self.apu.run_one_first_after_dmc();
    }

    /// `DMCUnit.Run`.
    fn dmc_run(&mut self) {
        self.apu.dmc.timer_just_reloaded = false;
        if self.apu.dmc.timer > 0 {
            self.apu.dmc.timer -= 1;
        }
        if self.apu.dmc.timer == 0 {
            self.apu.dmc.timer = self.apu.dmc.timer_reload;
            let rec = &mut self.apu.recalculate;
            self.apu.dmc.clock(rec);
            self.apu.dmc.timer_just_reloaded = true;
        }

        // Any time the sample buffer is in an empty state and bytes remaining is not zero, the
        // following occur; also note that the halt for DMC DMA occurs on APU cycles only (hence
        // the timer check)
        if !self.apu.dmc.sample_buffer_filled
            && ((self.apu.dmc.sample_length > 0) || self.apu.dmc.fill_glitch_2)
            && (self.apu.dmc_dma_countdown == -1)
            && (self.apu.dmc.delay == 0)
        {
            if !self.apu.dmc.fill_glitch {
                if !self.apu.call_from_write {
                    // when called due to empty buffer while DMC running, there is no delay
                    self.rdy = false;
                    self.dmc_dma_exec = true;

                    if self.apu.dmc.fill_glitch_2 {
                        // this will only run for one cycle and not actually run a DMA
                        self.apu.dmc_dma_countdown = -2;
                        self.apu.dmc_rdy_check = -2;
                        self.apu.dmc.fill_glitch_2_end = true;
                    } else {
                        self.apu.dmc_dma_countdown = 3;
                        self.apu.dmc_rdy_check = 2;
                    }
                } else {
                    // when called from write, either a 2 or 3 cycle delay in activation.
                    if self.apu.dmc.timer % 2 == 0 {
                        self.apu.dmc.delay = 3;
                    } else {
                        self.apu.dmc.delay = 2;
                    }
                }
            } else {
                // if refill and empty happen simultaneously, do not do another refill and act as
                // though the sample buffer was filled
                self.apu.dmc.sample_buffer_filled = true;
                self.apu.dmc.fill_glitch = false;
            }
        }

        // VisualNES and test roms verify that DMC DMA is 1 cycle shorter for calls from writes;
        // however, if the third cycle lands on a write, there is a 2 cycle delay even if the
        // instruction only has a single write. Therefore, the RDY_check is 2 in both cases.
        if self.apu.dmc.delay != 0 {
            self.apu.dmc.delay -= 1;
            if self.apu.dmc.delay == 0 {
                self.apu.dmc_dma_countdown = 3;
                self.apu.dmc_rdy_check = 2;
                self.apu.call_from_write = false;
            }
        }
    }

    /// `APU.RunDMCFetch`.
    pub(crate) fn run_dmc_fetch(&mut self) {
        self.dmc_fetch();
    }

    /// `APU.RunDMCHaltFetch`.
    pub(crate) fn run_dmc_halt_fetch(&mut self) {
        let a = self.address_bus;
        self.read_memory(a);
    }

    /// `DMCUnit.Fetch`.
    fn dmc_fetch(&mut self) {
        if self.apu.dmc.sample_length != 0 {
            self.dmc_dma_controller_conflict = true;
            let addr = self.apu.dmc.sample_address as u16;
            self.apu.dmc.sample_buffer = i32::from(self.read_memory(addr));
            self.dmc_dma_controller_conflict = false;

            self.apu.dmc.sample_buffer_filled = true;
            // `(sample_address & 0x1F) == 0x4017` below can never hold; it is the C#'s own
            // defect (it means `0x17`), transcribed rather than "corrected".
            #[allow(clippy::bad_bit_mask)]
            if self.address_bus >= 0x4000 && self.address_bus <= 0x401F {
                if (self.apu.dmc.sample_address & 0x1F) == 0x15 {
                    self.db = self.apu.dmc.sample_buffer as u8;
                } else if (self.apu.dmc.sample_address & 0x1F) == 0x16
                    // the C# tests `== 0x4017` here against a value masked to 5 bits, which can
                    // never match; transcribed as it stands rather than "corrected".
                    || (self.apu.dmc.sample_address & 0x1F) == 0x4017
                {
                    // The bus conflict leaves the open bus bits of the controller filled with the
                    // bits from the sample buffer.
                    self.db = ((self.apu.dmc.sample_buffer as u8) & 0xE0) | (self.db & 0x1F);
                }
            }
            self.apu.dmc.sample_address =
                i32::from((self.apu.dmc.sample_address as u16).wrapping_add(1));

            //sample address wraps to 0x8000, even though this cannot be reached by write
            if self.apu.dmc.sample_address == 0 {
                self.apu.dmc.sample_address = 0x8000;
            }

            self.apu.dmc_reload_countdown = 3;
        }
    }

    /// `APU.RunOneLast`.
    pub(crate) fn apu_run_one_last(&mut self) {
        if self.apu.dmc.timer % 2 == 1 {
            // The controllers only get strobed when transitioning from a get cycle to a put cycle.
            if self.joypad_strobed {
                self.joypad_strobed = false;
                self.strobe_joyport();
            }
        } else {
            // The frame counter interrupt flag is only cleared when transitioning from a put
            // cycle to a get cycle.
            if self.apu.sequencer_irq_clear_pending {
                self.apu.sequencer_irq_clear_pending = false;
                self.apu.sequencer_irq_flag = false;
            }
        }

        // we need to predict if there will be a length clock here, because the sequencer ticks
        // last, but the timer reload shouldn't happen if length clock and write happen
        // simultaneously
        if (self.apu.sequencer_counter == self.apu.sequencer_check_1)
            || (self.apu.sequencer_counter == self.apu.sequencer_check_2)
        {
            self.apu.len_clock_active = true;
        }

        // writes on the same cycle as reload disable IRQ if it is set, so put this here before
        // writes
        if self.apu.dmc_reload_countdown > 0 {
            self.apu.dmc_reload_countdown -= 1;
            if self.apu.dmc_reload_countdown == 0 {
                self.apu.dmc.sample_length = self.apu.dmc.sample_length.wrapping_sub(1);

                if self.apu.dmc.sample_length == 0 {
                    if self.apu.dmc.loop_flag {
                        self.apu.dmc.sample_address = self.apu.dmc.user_address;
                        self.apu.dmc.sample_length = self.apu.dmc.user_length;
                    } else if self.apu.dmc.irq_enabled && !self.apu.dmc_irq_glitch {
                        self.apu.dmc_irq = true;
                    }
                }

                if self.apu.dmc.pending_disable {
                    self.apu.dmc.sample_length = 0;
                    self.apu.dmc.pending_disable = false;
                }

                self.apu.dmc_irq_glitch = false;
            }
        }

        // handle writes
        if self.apu.pending_reg != -1 {
            let reg = self.apu.pending_reg;
            let val = self.apu.pending_val;
            if reg == 0x4003 || reg == 0x4007 || reg == 0x4010 || reg == 0x4015 || reg == 0x4017 {
                self.apu.write_reg_internal(reg, val);
                self.apu.pending_reg = -1;
            } else if self.apu.dmc.timer % 2 == 0 {
                self.apu.write_reg_internal(reg, val);
                self.apu.pending_reg = -1;
            }
        }

        self.apu.len_clock_active = false;

        self.apu.sequencer_tick();
        let v = self.apu.seq_val;
        self.apu.sequencer_write_tick(v);
        self.apu.doing_tick_quarter = false;

        if self.apu.sequencer_irq_assert > 0 {
            self.apu.sequencer_irq_assert -= 1;
            if self.apu.sequencer_irq_assert == 0 {
                self.apu.sequencer_irq = true;
            }
        }

        self.apu.sync_irq();
        self.irq_apu = self.apu.irq_pending;

        // since the units run concurrently, the APU frame sequencer is ran last because it can
        // change the output values of the pulse/triangle channels; we want the changes to affect
        // it on the *next* cycle.
        if !self.apu.sequencer_irq_flag {
            self.apu.sequencer_irq = false;
        }
    }
}
