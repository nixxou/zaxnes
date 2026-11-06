use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 64 (Tengen RAMBO-1) and 158 (Tengen 800037): MMC3-like with a third PRG
/// register, 1 KiB CHR mode and a scanline counter that can also count CPU cycles
/// (REQ-CORE-100, REQ-CORE-102).
///
/// **`TLSROM` is mapper 158, and it is a type parameter and not a field** — MR-4-3's rule, the
/// one [`super::Mmc3Board`]'s `MC_ACC` follows. nesdev ("INES Mapper 158") gives the whole board
/// in one sentence: it "modifies RAMBO-1 (mapper 64) in the exact same way that TLSROM (118)
/// differs from MMC3 (4), by which we mean the RAMBO-1's CHR A17 output is connected to CIRAM
/// A10". So bit 7 of the CHR bank registers picks the nametable, `$A000` is bypassed, and
/// nothing else moves.
///
/// A `const` parameter rather than a `bool` because the difference would otherwise be **two new
/// fields on the RAMBO-1** — a flag and the four pages — and MF-6b step 1 measured what moving
/// an IRQ board's fields costs: one byte of displacement on `VrcIrq::pending` re-laid out
/// `Cpu6502::branch` and cost eleven of lot PR-1's thirteen ROMs +0.009 % to +0.027 %, with no
/// code added at all. Monomorphised, `Rambo1` is `Rambo1Board<false>` and comes out of the
/// compiler byte-identical, which `cmp4c.py` reads on `Rambo1::update_banks`.
///
/// The pages are **computed and not cached**, which is the one place this board differs from the
/// TxSROM's shape. `Mmc3Board` holds a `nametable_pages: [i32; 4]` field because it predates the
/// cache `Cartridge` now keeps; with `Cartridge::nametable_pages_vary` naming mapper 158,
/// `Mapper::nametable_page` is asked four times per register write and never on a fetch, so a
/// field here would buy nothing and cost the RAMBO-1 its layout.
///
/// The one game is *Alien Syndrome* (Tengen, 1989), on Tengen's board 800037.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rambo1Board<const TLSROM: bool> {
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

/// Mapper 64, the RAMBO-1 proper.
pub type Rambo1 = Rambo1Board<false>;

/// Mapper 158, Tengen's 800037: the same chip with CHR A17 wired to CIRAM A10.
pub type Tengen800037 = Rambo1Board<true>;

impl<const TLSROM: bool> Rambo1Board<TLSROM> {
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

impl<const TLSROM: bool> Mapper for Rambo1Board<TLSROM> {
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
        if TLSROM {
            158
        } else {
            64
        }
    }

    /// Mapper 158's nametables, the TxSROM's rule on this chip's CHR layout
    /// ([`super::Mmc3Board::refresh_nametable_pages`] is the same six lines for the MMC3).
    ///
    /// It is the registers mapped to **the first pattern table** that drive CIRAM A10, and bit 7
    /// of `$8000` is what chooses them: with it clear the two 2 KiB registers R0 and R1 are at
    /// PPU `$0000-$0FFF` and take two nametables each, and with it set the four 1 KiB registers
    /// R2-R5 are there and take one each. That is the same bit, read the same way round, that
    /// `update_banks` uses for its `high`/`low` swap, and Mesen 2 reads it identically
    /// (`Rambo1_158.h:10-32`).
    ///
    /// Registers 8 and 9, which fill the odd kilobytes of 1 KiB mode, do **not** reach a
    /// nametable: only four pages exist and the four 1 KiB registers already name them. The
    /// page's own note that "Nestopia-1.4.0's implementation looks incomplete; it seems to
    /// ignore bank registers 8 and 9, as well as the K bit" is about its *banking*, which
    /// `update_banks` has had since MF-3.
    fn nametable_page(&self, table: i32) -> i32 {
        if !TLSROM {
            return -1;
        }

        let page = |register: usize| i32::from(self.bank[register] >> 7);
        let table = usize::try_from(table).unwrap_or(0) & 3;
        if self.select & 0x80 == 0 {
            page(table >> 1)
        } else {
            page(2 + table)
        }
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
            // Mapper 158 drives CIRAM A10 off its CHR registers, so this register "is bypassed
            // by the configuration described above, so writing here has no effect" — nesdev's
            // own words for the TxSROM (`INES Mapper 118`), which this board copies. Mesen 2
            // filters the same address out (`Rambo1_158.h:35`).
            0xA000 if !TLSROM => memory.set_mirroring(if value & 0x01 != 0 {
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
