use super::{Mapper, Mmc1};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 105, the Nintendo **NES-EVENT** board (REQ-CORE-102).
///
/// nesdev ("NES-EVENT", "INES Mapper 105"): "the MMC1-based board used for *Nintendo World
/// Championships*, a multicart including timed versions of *Super Mario Bros.*, *Rad Racer*, and
/// *Tetris*. Like SUROM and SXROM, it repurposes the MMC1 CHR address bits". Its parts are "256K
/// of PRG ROM as two 27C010 UV EPROMs", "8K of CHR RAM", "8K of PRG RAM" and "several
/// 4000-series and 7400-series discrete ICs" — and those discrete ICs are the whole of this
/// file: the MMC1 runs exactly as it does on a mapper-1 cartridge, and its PRG outputs are then
/// taken through logic that reads `$A000` as a chip select instead of a CHR bank.
///
/// # Why it holds an [`Mmc1`] instead of parameterising one
///
/// §5 step 3 of `docs/review/MF-6-inventory.md` calls this "the step where `const`-vs-field
/// matters most in the whole lot, because mapper 1 is on five of R0's thirteen ROMs", and the
/// answer here is **neither**. A `const` parameter still puts the 30-bit counter and its three
/// flags in the struct for both monomorphisations, and MF-6b step 1 measured what moving an
/// MMC1-family board's fields costs: one byte of displacement on `VrcIrq::pending` re-laid out
/// `Cpu6502::branch` and cost eleven of thirteen ROMs +0.009 % to +0.027 %. A runtime `bool` is
/// worse still — MF-6a2 step 2 measured `Mmc1::update_banks` paying **+8 001 instructions over
/// 600 level-2 frames of Mega Man 2** for one test on the commit path.
///
/// Holding the chip rather than becoming it costs mapper 1 **nothing at all**: `Mmc1`'s fields,
/// its layout and its code are untouched, which `cmp4c.py` reads as `Mmc1::update_banks`
/// byte-identical. What the NES-EVENT pays for that is one redundant MMC1 bank computation per
/// write to `$8000-$FFFF` — the MMC1 lays out its own PRG and this board writes over it — and
/// §4.10's rule is that the board which needs a thing is the board that pays for it. There is
/// one cartridge of this board in the world.
///
/// # The registers, as Disch's notes give them
///
/// "Registers are internal and not accessed directly", so the serial protocol, the
/// consecutive-write rule and the five-bit shift register are [`Mmc1`]'s and are not repeated
/// here:
///
/// | register | NES-EVENT meaning |
/// | --- | --- |
/// | `$8000-$9FFF` | `[.... PSMM]`, "same as MMC1 (but CHR mode bit isn't used)" |
/// | `$A000-$BFFF` | `[...I OAA.]`: `I` timer control and initialisation toggle, `O` chip select, `AA` PRG reg *A* |
/// | `$C000-$DFFF` | unused — the register exists (PPU A12 is still wired to the MMC1) but nothing it holds can move 8 KiB of unbanked CHR-RAM |
/// | `$E000-$FFFF` | `[...W BBBB]`: `W` WRAM disable "same as MMC1", `B` PRG reg *B* |
///
/// PRG, from Disch's table and the page's own register diagram, which agree: "there are 2 PRG
/// chips, each 128k. The `O` bit selects between the chips, and also determines which PRG Reg is
/// used to select the page" — `O=0` is the first chip with a 32 KiB swap from `AA`, `O=1` the
/// second chip with the MMC1's own `P`/`S` modes from `B`. "In addition, if the mapper has not
/// been 'unlocked', the first 32k of the first chip is always selected regardless (as if $A000
/// contained $00)."
///
/// # Two things the sources leave open, and what this board does
///
/// 1. **The power-on `I` bit.** The page says "the first 32 KiB is hardwired until the timer is
///    started (write 0 then 1 to bit 4) for the first time", and Disch "PRG cannot be swapped
///    until the mapper has been 'initialized' by setting the `I` bit to 0, then to `1`". That
///    sequence only means something if `I` starts **set**, which no page states; an MMC1 powers
///    its CHR register on at zero and the "write 0" would then be a no-op. Mesen 2 resolves it
///    the same way, with `_chrReg0 |= 0x10; //Set I bit to 1` (`MMC1_105.h:24`), and that is the
///    reading here: [`NesEvent::POWER_ON_CHR0`], so the documented two-step unlock is the
///    sequence that unlocks.
/// 2. **The DIP switches.** "DIP switches set bits 28-25 of the counter's initial value. This
///    results in a range of roughly 5 to 10 minutes on an NTSC console", and there is no host
///    surface for them in this build. The documented default is the page's own: "in the official
///    tournament, `C` was closed, and the others were open, so the counter had to reach
///    $2800000" — $28000000, the page having dropped a digit, which its own formula confirms.
///    [`NesEvent::DIP_SWITCHES`] is that setting and nothing else reads it.
///
/// The one game is *Nintendo World Championships 1990* (Nintendo, 1990), 1 in NesCartDB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NesEvent {
    /// The chip itself, driven unchanged: the shift register, the four registers, the
    /// mirroring. Its own PRG layout is computed and then overwritten by
    /// [`NesEvent::update_prg`], which is what the board's discrete logic does to its outputs.
    mmc1: Mmc1,
    /// "A 30-bit up counter clocked by M2". It is stopped at [`NesEvent::irq_target`] and so
    /// never reaches 2^30; the width is the comparator's, not a wrap this has to model.
    counter: u32,
    /// The unlock sequence: 0 while `I` has not been seen clear, 1 once it has, 2 once it has
    /// been seen set again. PRG only moves at 2.
    init_state: u8,
    /// `I` clear: "the IRQ counter counts up every cycle".
    counting: bool,
    irq_pending: bool,
}

impl NesEvent {
    /// "8K of CHR RAM", unbanked — "this mapper has 8k CHR-RAM, and it is not swappable".
    /// Settled at load by `registry::board_memory_sizes`, for the reason
    /// [`super::Vrc3::CHR_RAM_SIZE`] is: REQ-CORE-012's default for an image with no CHR-ROM is
    /// 32 KiB, which this board has no line to reach.
    pub(crate) const CHR_RAM_SIZE: usize = 0x2000;

    /// `DCBA`, the four switches at bits 28-25 of the target: the official tournament's "`C`
    /// closed, and the others open". See the type's own note.
    const DIP_SWITCHES: u32 = 0b0100;

    /// `$A000` at power-on, `I` set. See the type's own note on why it is not zero.
    const POWER_ON_CHR0: u8 = 0x10;

    /// The 128 KiB of a chip, in 16 KiB banks: the `O` bit's base.
    const CHIP_BANKS_16K: i32 = 8;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            mmc1: Mmc1::new(memory),
            counter: 0,
            init_state: 0,
            counting: false,
            irq_pending: false,
        };
        board.mmc1.force_chr0_register(Self::POWER_ON_CHR0);
        board.refresh();
        board
    }

    /// "The IRQ counter is 30 bits wide.. when it reaches the following value, an IRQ is fired:
    /// `[1D CBAx xxxx xxxx xxxx xxxx xxxx xxxx]`" — bit 29 always set, bits 28-25 the switches.
    /// "So if all dipswitches are open (use `0` above), the counter must reach $20000000."
    const fn irq_target() -> u32 {
        0x2000_0000 | (Self::DIP_SWITCHES << 25)
    }

    /// The init state machine and the timer control, both driven by `I` and both re-read after
    /// every commit, as Mesen 2's `UpdateState` is (`MMC1_105.h:55-66`).
    fn refresh(&mut self) {
        let timer_reset = self.mmc1.chr0_register() & 0x10 != 0;
        if self.init_state == 0 && !timer_reset {
            self.init_state = 1;
        } else if self.init_state == 1 && timer_reset {
            self.init_state = 2;
        }

        // "When cleared, the IRQ counter counts up every cycle. When set, the IRQ counter is
        // reset to 0 and stays there (does not count), and the pending IRQ is acknowledged."
        self.counting = !timer_reset;
        if timer_reset {
            self.counter = 0;
            self.irq_pending = false;
        }

        self.update_prg();
    }

    /// The discrete logic, written over whatever layout the MMC1 just computed.
    fn update_prg(&mut self) {
        let chr0 = self.mmc1.chr0_register();
        let control = self.mmc1.control_register();
        let prg = self.mmc1.prg_register();
        let unlocked = self.init_state == 2;
        let windows = self.mmc1.windows_mut();

        if !unlocked {
            // "On powerup and reset, the first 32k of PRG (from the first PRG chip) is selected
            // at $8000 *no matter what*."
            windows.map_prg_32k(0);
        } else if chr0 & 0x08 == 0 {
            // `O=0`: "use first PRG chip (first 128k), use `A` PRG Reg, 32k swap" — `AA` being
            // bits 2-1, so four 32 KiB banks over the chip's 128 KiB.
            windows.map_prg_32k(i32::from((chr0 >> 1) & 0x03));
        } else {
            // `O=1`: "use second PRG chip (second 128k), use `B` PRG Reg, MMC1 style swap".
            // `B` is three bits of bank inside the chip, above the chip's own base.
            let bank = i32::from(prg & 0x07) | Self::CHIP_BANKS_16K;
            match (control >> 2) & 0x03 {
                // `P=0`, 32 KiB: the low bit of the bank number is ignored, as on an MMC1.
                0 | 1 => windows.map_prg_32k(bank >> 1),
                // `P=1, S=0`: the chip's first bank fixed at $8000, `B` switchable at $C000.
                2 => {
                    windows.map_prg_16k(0, Self::CHIP_BANKS_16K);
                    windows.map_prg_16k(2, bank);
                }
                // `P=1, S=1`: `B` switchable at $8000, the chip's last bank fixed at $C000.
                _ => {
                    windows.map_prg_16k(0, bank);
                    windows.map_prg_16k(2, (Self::CHIP_BANKS_16K * 2) - 1);
                }
            }
        }

        // 8 KiB of CHR-RAM and no bank line to it, whatever the MMC1's CHR registers hold.
        windows.map_chr_8k(0);
    }
}

impl Mapper for NesEvent {
    fn mapper_number(&self) -> i32 {
        105
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc1.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc1.banks_mut()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc1.save_registers(writer);
        writer.write_u32(self.counter);
        writer.write_u8(self.init_state);
        writer.write_bool(self.counting);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc1.load_registers(reader)?;
        self.counter = reader.read_u32()?;
        self.init_state = reader.read_u8()?;
        self.counting = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc1.reset(memory);
        self.mmc1.force_chr0_register(Self::POWER_ON_CHR0);
        self.counter = 0;
        self.init_state = 0;
        self.counting = false;
        self.irq_pending = false;
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `W` of `$E000`, "WRAM disable (same as MMC1)" — **and not [`Mmc1::prg_ram_enabled`]**,
    /// which this board must not delegate. A *Nintendo World Championships* dump is 256 KiB of
    /// PRG, no CHR-ROM and 8 KiB of PRG-RAM, which is exactly `Mmc1::is_snrom`'s signature, so
    /// the chip would also read bit 4 of `$A000` as a second chip enable — and bit 4 of `$A000`
    /// is this board's timer control. Starting the timer would switch the save RAM off.
    fn prg_ram_enabled(&self) -> bool {
        self.mmc1.prg_register() & 0x10 == 0
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc1.write_register(memory, address, value);
        self.refresh();
    }

    /// `#[inline(never)]`, for the reason [`super::Vrc3::on_cpu_cycle`] gives.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.counting {
            return;
        }

        self.counter += 1;
        if self.counter >= Self::irq_target() {
            // Mesen 2 stops the counter here as well (`MMC1_105.h:44`). Nothing in the sources
            // says the comparator latches, but the counter would otherwise keep climbing with
            // the output high for the ≈400 M cycles to 2^30 and nothing could distinguish the
            // two: the pending IRQ is only ever cleared by a write that also zeroes the counter.
            self.counting = false;
            self.irq_pending = true;
        }
    }
}
