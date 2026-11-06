use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 64, Tengen RAMBO-1: MMC3-like with a third PRG register, 1 KiB CHR mode and a
/// scanline counter that can also count CPU cycles (REQ-CORE-100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rambo1 {
    banks: BankWindows,
    a12: A12Watcher,
    bank: [u8; 16],
    select: u8,
    irq_latch: u8,
    irq_counter: i32,
    cycle_divider: i32,
    cycle_mode: bool,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
    /// CPU cycles still to run before the counter's zero reaches the /IRQ line (MR-64-4), or 0
    /// when nothing is armed. nesdev ("RAMBO-1"): in scanline mode "the actual interrupt
    /// triggers slightly later. Specifically, it is delayed until M2 falls twice after the PPU
    /// A12 rise", and in cycle mode it "triggers one M2 cycle later than one would naively
    /// expect".
    irq_delay: i32,
}

impl Rambo1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            a12: A12Watcher::new(A12Watcher::RAMBO1_FILTER_DOTS),
            bank: [0; 16],
            select: 0,
            irq_latch: 0,
            irq_counter: 0,
            cycle_divider: 0,
            cycle_mode: false,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
            irq_delay: 0,
        }
    }

    fn update_banks(&mut self) {
        let b = |i: usize| i32::from(self.bank[i]);
        // MR-64-7. The PRG mode bit *swaps* R6 with R15 and leaves R7 at $A000; it does not
        // rotate the three switchable windows. nesdev ("RAMBO-1") gives R6/R7/RF for P = $00
        // and RF/R7/R6 for P = $40, corrected from the hardware tests of the "RAMBO-1 Mapper
        // Investigation" thread ("when P=1, $A000=R7, $C000=R6"). Mesen carries the old table.
        if self.select & 0x40 != 0 {
            self.banks.map_prg_8k(0, b(15));
            self.banks.map_prg_8k(1, b(7));
            self.banks.map_prg_8k(2, b(6));
        } else {
            self.banks.map_prg_8k(0, b(6));
            self.banks.map_prg_8k(1, b(7));
            self.banks.map_prg_8k(2, b(15));
        }

        self.banks.map_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        if self.select & 0x20 != 0 {
            // 1 KiB mode: registers 8 and 9 fill the odd kilobytes of the first two windows.
            self.banks.map_chr_1k(high, b(0));
            self.banks.map_chr_1k(high + 1, b(8));
            self.banks.map_chr_1k(high + 2, b(1));
            self.banks.map_chr_1k(high + 3, b(9));
        } else {
            self.banks.map_chr_2k(high, b(0) >> 1);
            self.banks.map_chr_2k(high + 2, b(1) >> 1);
        }

        for i in 0..4 {
            self.banks.map_chr_1k(low + i, b(2 + i));
        }
    }

    /// Clocks the 8-bit counter and, when it lands on zero with IRQs enabled, **arms** the
    /// delayed assertion of MR-64-4 rather than pulling the line at once: two M2 cycles in
    /// scanline mode, one in cycle mode.
    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_delay = if self.cycle_mode { 1 } else { 2 };
        }
    }
}

impl Mapper for Rambo1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_i32(self.cycle_divider);
        writer.write_bool(self.cycle_mode);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        self.a12.save_state(writer);
        // Written last, so a state from before this lot loads with no delay armed: the board
        // comes back with whatever `irq_pending` it had, which is where the countdown was
        // heading anyway (REQ-CORE-091).
        writer.write_i32(self.irq_delay);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.cycle_divider = reader.read_i32()?;
        self.cycle_mode = reader.read_bool()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        self.irq_delay = if reader.remaining() > 0 {
            reader.read_i32()?
        } else {
            0
        };
        Ok(())
    }

    fn chr_view<'m>(
        &self,
        _memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its fetches have side effects or another source: the PPU takes the full path.
        None
    }

    fn mapper_number(&self) -> i32 {
        64
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// MR-64-3's dividend. The board returned `None` — "catch the PPU up on every cycle" — and
    /// can now bound the wait exactly as the MMC3 does, over an A12 filter five times as long.
    ///
    /// In **cycle mode** the counter is clocked by M2 and not by the PPU at all, so no amount of
    /// PPU lag can move this line and the horizon is unbounded. In scanline mode the counter
    /// only moves on an A12 rise, and those come at least `min_rise_interval` dots apart.
    #[inline(never)]
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.cycle_mode || self.irq_pending || !self.irq_enabled {
            return Some(i64::MAX);
        }

        let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
            1 + i64::from(self.irq_latch)
        } else {
            i64::from(self.irq_counter)
        };
        Some((clocks_to_zero - 1) * self.a12.min_rise_interval())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 16];
        self.select = 0;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.cycle_divider = 0;
        self.cycle_mode = false;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_delay = 0;
        self.a12.reset();
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xE001 {
            0x8000 => {
                self.select = value;
                self.update_banks();
            }
            0x8001 => {
                self.bank[usize::from(self.select & 0x0F)] = value;
                self.update_banks();
            }
            0xA000 => memory.set_mirroring(if value & 0x01 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            0xC000 => self.irq_latch = value,
            0xC001 => {
                self.cycle_mode = value & 0x01 != 0;
                self.cycle_divider = 0;
                self.irq_reload = true;
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
                // An acknowledgement disarms a delay that had not landed yet: the chip has
                // nothing left to assert.
                self.irq_delay = 0;
            }
            0xE001 => self.irq_enabled = true,
            _ => {}
        }
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if !self.cycle_mode && self.a12.rose(address, clock) {
            self.clock_irq_counter();
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        // MR-64-4. The countdown runs in **both** modes, because a scanline-mode counter is
        // armed by an A12 rise and lands two M2 cycles later. One compare and one branch on a
        // path the five mapper-64 games already pay for, and nothing at all on any other board.
        if self.irq_delay > 0 {
            self.irq_delay -= 1;
            if self.irq_delay == 0 {
                self.irq_pending = true;
            }
        }

        if !self.cycle_mode {
            return;
        }

        self.cycle_divider += 1;
        if self.cycle_divider < 4 {
            return;
        }

        self.cycle_divider = 0;
        self.clock_irq_counter();
    }
}
