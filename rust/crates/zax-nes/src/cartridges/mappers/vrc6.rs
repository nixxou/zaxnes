// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// `Vrc6::refresh_nametables` is the arrangement of `VRC6.h:107-225` (`UpdatePpuBanking`), which
// is the one translation of the nesdev `$B003` tables into this crate's mirroring vocabulary that
// could be checked row by row; see the function's own note and THIRD-PARTY-NOTICES.md.

use super::vrc_irq::VrcIrq;
use super::{windows_for, Mapper};
use crate::apu::Vrc6Audio;
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 24 and 26, Konami VRC6 (nesdev, "VRC6"; the numbers are outside REQ-CORE-100's
/// table, which REQ-CORE-102 allows, and REQ-CORE-075 makes the chip's sound optional).
///
/// "Only address lines 0, 1, and 12-15 are used for registers", so every register has four
/// mirrors and the decode is `address & 0xF003`:
///
/// | address | bits | effect |
/// | --- | --- | --- |
/// | $8000-$8003 | `.... PPPP` | 16 KiB PRG bank at $8000-$BFFF |
/// | $9000-$9002 | — | pulse 1 ([`Vrc6Audio`]) |
/// | $9003 | `.... .ADH` | frequency scale and halt, all three channels |
/// | $A000-$A002 | — | pulse 2 |
/// | $B000-$B002 | — | sawtooth |
/// | $B003 | `W.PN MMDD` | PPU banking style, mirroring, and the PRG-RAM enable |
/// | $C000-$C003 | `...P PPPP` | 8 KiB PRG bank at $C000-$DFFF |
/// | $D000-$D003 | `BBBB BBBB` | CHR registers R0-R3 |
/// | $E000-$E003 | `BBBB BBBB` | CHR registers R4-R7 |
/// | $F000 | `LLLL LLLL` | IRQ latch |
/// | $F001 | `.... .MEA` | IRQ control |
/// | $F002 | — | IRQ acknowledge |
///
/// $E000-$FFFF is not switchable: the last 8 KiB bank is wired there. $6000-$7FFF is 8 KiB of
/// PRG-RAM, which `W` enables.
///
/// **Mapper 26 is the same chip with A0 and A1 swapped** ("VRC6b"), which is one `address`
/// rewrite at the top of [`Mapper::write_register`] and nothing else — see `swap_address_lines`.
///
/// The IRQ is Konami's, shared with the VRC4 as [`VrcIrq`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc6 {
    banks: BankWindows,
    mapper_number: i32,
    /// Whether this is the VRC6b of mapper 26, whose A0 and A1 reach the chip the other way
    /// round. Load-time, so the rewrite below is one test and not a `match` on the number.
    swap_address_lines: bool,
    /// The chip's three extra channels. Carried by every mapper-24 and -26 image: the VRC6
    /// always has them, and a game that writes no sound register leaves every `E` bit clear,
    /// which is silence (REQ-CORE-075, optional).
    audio: Vrc6Audio,
    irq: VrcIrq,
    /// `$8000-$8003`, the 16 KiB bank at $8000-$BFFF: four bits.
    ///
    /// Kept as a register and not only as a mapped window, so that [`Mapper::reset`] puts it
    /// back — `BankWindows` has no memory of what was written into it, and a reset that only
    /// re-mapped the fixed window would leave a VRC6 on whatever bank the game had selected
    /// (AMD-061 §5 exempts mapper 28 and no other board).
    prg_16k: u8,
    /// `$C000-$C003`, the 8 KiB bank at $C000-$DFFF: five bits.
    prg_8k: u8,
    /// `$B003`, exactly as written.
    banking_mode: u8,
    /// R0-R7, the eight CHR registers of $D000-$E003, exactly as written.
    chr: [u8; 8],
    /// What [`Mapper::nametable_page`] answers for each logical table, recomputed whenever
    /// `$B003` or a CHR register moves.
    ///
    /// A field and not a `match`, for the reason the field doc of `Cartridge::nametable_pages`
    /// gives: that function is consulted **once per nametable fetch**, and a board whose answer
    /// costs more than a load pushes `Ppu2C02::read_nametable_as` out of line and charges every
    /// game a call per fetch (MF-2, then MF-3; see MF-3b step 3).
    nametable_pages: [i32; 4],
    /// The 1 KiB CHR bank each logical nametable shows when `N` says the nametables come from
    /// CHR-ROM. Meaningless while they come from CIRAM.
    nametable_chr: [u8; 4],
    /// `$B003` bit 7, `W`: precomputed so that `prg_ram_enabled` — asked on the CPU's
    /// $6000-$7FFF decode — is one load and one test, like every other board's.
    prg_ram_on: bool,
}

impl Vrc6 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            mapper_number,
            // nesdev, "VRC6": "VRC6b (mapper 026) has A0 and A1 swapped". Mesen reads it the
            // same way (`VRC6.h:231-233`).
            swap_address_lines: mapper_number == 26,
            audio: Vrc6Audio::new(),
            irq: VrcIrq::new(),
            prg_16k: 0,
            // Nothing on the page gives a power-on value for either PRG register. 0 and 2 are
            // the banks the windows would show with no board logic at all — $8000-$DFFF laid
            // out linearly — which is what Mesen comes up with too (`VRC6.h:29-38` maps only
            // the fixed window and leaves `BaseMapper`'s linear default under it). Every VRC6
            // game writes both registers in its reset handler, so this is not observable.
            prg_8k: 2,
            banking_mode: 0,
            chr: [0; 8],
            nametable_pages: [Cartridge::FOLLOW_MIRRORING; 4],
            nametable_chr: [0; 4],
            prg_ram_on: false,
        };
        board.update_banks();
        board
    }

    /// The four PRG windows and the eight CHR windows, from the registers as they stand.
    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg_16k));
        self.banks.map_prg_8k(2, i32::from(self.prg_8k));
        // "$E000-$FFFF" is absent from the page's PRG table: the last bank is wired there.
        self.banks.map_prg_8k(3, -1);
        self.update_chr_banks();
    }

    /// The eight 1 KiB CHR windows, which `$B003`'s `D` and `P` bits arrange four ways.
    ///
    /// `D` picks the granularity — mode 0 is eight 1 KiB banks from R0-R7, mode 1 four 2 KiB
    /// banks from R0-R3, modes 2 and 3 four 1 KiB banks from R0-R3 then two 2 KiB banks from R4
    /// and R5 — and `P` decides what the low bit of a 2 KiB bank's register means: with `P` set
    /// the two halves are `R & $FE` and `R | 1`, i.e. PPU A10 passes through, and with `P` clear
    /// both halves take the whole register, i.e. "CHR A10 … according to the latched value".
    fn update_chr_banks(&mut self) {
        let pass_a10 = self.banking_mode & 0x20 != 0;
        // The two halves of a 2 KiB window, as `P` defines them.
        let pair = |register: u8| -> [i32; 2] {
            if pass_a10 {
                [i32::from(register & 0xFE), i32::from(register | 0x01)]
            } else {
                [i32::from(register), i32::from(register)]
            }
        };

        match self.banking_mode & 0x03 {
            0 => {
                for slot in 0..BankWindows::CHR_WINDOWS {
                    self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
                }
            }
            1 => {
                for group in 0..4 {
                    let [low, high] = pair(self.chr[group]);
                    self.banks.map_chr_1k(group * 2, low);
                    self.banks.map_chr_1k(group * 2 + 1, high);
                }
            }
            _ => {
                for slot in 0..4 {
                    self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
                }

                for group in 0..2 {
                    let [low, high] = pair(self.chr[4 + group]);
                    self.banks.map_chr_1k(4 + group * 2, low);
                    self.banks.map_chr_1k(5 + group * 2, high);
                }
            }
        }
    }

    /// The four nametables, from `$B003` and the CHR registers.
    ///
    /// **The vocabulary, because the page and this crate use two.** nesdev's `$B003` tables name
    /// the four fixed arrangements of mode 0 "vertical / horizontal / 1-screen A / 1-screen B"
    /// *and* give the CIRAM A10 source of each — "PPU A10 / PPU A11 / Ground / Vcc". CIRAM A10
    /// driven by PPU A10 is what every board page in this crate calls **vertical mirroring**
    /// (`fme7.rs` carries the same note), so for mode 0 the two namings agree. For **mode 3**
    /// they do not: the page calls `$B003 & $F == $3` "vertical pairs 2 KiB spread", and its own
    /// table for that column is `R6 even / R7 even / R6 odd / R7 odd`, which puts $2000 and
    /// $2400 on one CIRAM page and $2800 and $2C00 on the other — **horizontal** mirroring in
    /// this crate's sense. So the page's mode-3 names are arrangement names, and the table rows
    /// are what this function implements. Mesen reads every row the same way
    /// (`VRC6.h:174-194`), and the four rows were re-derived here one by one from the page's own
    /// even/odd columns:
    ///
    /// | `$B003 & $2F` | page's table rows | CIRAM pages | this crate |
    /// | --- | --- | --- | --- |
    /// | $20, $27 | R6e R6o R7e R7o / R6e R6o R7e R7o | 0 1 0 1 | `Vertical` |
    /// | $23, $24 | R6e R7e R6o R7o | 0 0 1 1 | `Horizontal` |
    /// | $28, $2F | R6e R6e R7e R7e | 0 0 0 0 | `SingleScreenA` |
    /// | $2B, $2C | R6o R7o R6o R7o | 1 1 1 1 | `SingleScreenB` |
    ///
    /// Everything else — modes 1 and 2, and every mode with `P` clear — takes a page per table
    /// from the low bit of a CHR register, which is the "according to the latched value" case,
    /// and the same three groupings serve the CHR-ROM nametables with the whole register.
    fn derive_nametables(&self) -> ([i32; 4], [u8; 4], Option<NametableMirroring>) {
        let mode = self.banking_mode;
        let (r4, r5, r6, r7) = (self.chr[4], self.chr[5], self.chr[6], self.chr[7]);

        // The four arrangements `P` fixes: a mirroring for the CIRAM case, and an even/odd
        // pattern of R6 and R7 for the CHR-ROM case. `(R & $FE) | 1` is `R | 1`.
        let fixed = match mode & 0x2F {
            0x20 | 0x27 => Some((
                NametableMirroring::Vertical,
                [r6 & 0xFE, r6 | 0x01, r7 & 0xFE, r7 | 0x01],
            )),
            0x23 | 0x24 => Some((
                NametableMirroring::Horizontal,
                [r6 & 0xFE, r7 & 0xFE, r6 | 0x01, r7 | 0x01],
            )),
            0x28 | 0x2F => Some((
                NametableMirroring::SingleScreenA,
                [r6 & 0xFE, r6 & 0xFE, r7 & 0xFE, r7 & 0xFE],
            )),
            0x2B | 0x2C => Some((
                NametableMirroring::SingleScreenB,
                [r6 | 0x01, r7 | 0x01, r6 | 0x01, r7 | 0x01],
            )),
            _ => None,
        };

        // Mode 1 is the page's "4-screen" row and takes R4-R7; mode 2 and mode 3 with `P` clear
        // pair the two tables of each half; mode 0 with `P` clear gives each half one table.
        let selected = match fixed {
            Some((_, selected)) => selected,
            None => match mode & 0x07 {
                1 | 5 => [r4, r5, r6, r7],
                2..=4 => [r6, r7, r6, r7],
                _ => [r6, r6, r7, r7],
            },
        };

        // `N` set means "Nametables come from CHRROM", and this build does **not** serve them:
        // the four tables stay on CIRAM, laid out by the same registers. The reason is measured
        // and is not a reading of the page — a third board overriding
        // `Mapper::read_nametable` reshapes the dispatch that `Ppu2C02::read_nametable_inner`
        // folds in and costs **every board** two instructions and 8 bytes of stack per
        // nametable fetch. The whole measurement is on `<Board as Mapper>::read_nametable` and
        // in step 1 of `docs/progress/MF-5a.md`.
        // Q-123
        let _ = mode & 0x10;

        // A fixed arrangement goes through the cartridge's mirroring, like every other board's;
        // the rest names a page per table from the latched low bit.
        match fixed {
            Some((mirroring, _)) => ([Cartridge::FOLLOW_MIRRORING; 4], selected, Some(mirroring)),
            None => (
                selected.map(|register| i32::from(register & 0x01)),
                selected,
                None,
            ),
        }
    }

    /// Stores what [`Vrc6::derive_nametables`] computes, and applies the mirroring a fixed
    /// arrangement names.
    fn refresh_nametables(&mut self, memory: &mut CartridgeMemory) {
        let (pages, chr, mirroring) = self.derive_nametables();
        self.nametable_pages = pages;
        self.nametable_chr = chr;
        if let Some(mirroring) = mirroring {
            memory.set_mirroring(mirroring);
        }
    }
}

impl Mapper for Vrc6 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.prg_16k);
        writer.write_u8(self.prg_8k);
        writer.write_u8(self.banking_mode);
        writer.write_raw(&self.chr);
        self.irq.save_state(writer);
        // Written last, so that a mapper-24 state from before the sound existed would load with
        // a silent chip -- which is a VRC6 before its first sound register write (REQ-CORE-091).
        // No such state was ever shipped: the board and its sound arrive in the same lot. The
        // ordering costs nothing and is the shape `fme7.rs` established.
        self.audio.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg_16k = reader.read_u8()?;
        self.prg_8k = reader.read_u8()?;
        self.banking_mode = reader.read_u8()?;
        reader.read_into(&mut self.chr)?;
        self.irq.load_state(reader)?;
        if reader.remaining() > 0 {
            self.audio.load_state(reader)?;
        } else {
            self.audio.reset();
        }

        // Every derived half comes back from the registers (REQ-CORE-091). The **mirroring** is
        // not touched: `Cartridge::load_state` has just restored it from the state, and
        // recomputing it here would overwrite the saved one with whatever `$B003` implies — the
        // same value in practice, but the state is authoritative. `Cartridge::load_state` then
        // calls `refresh_nametable_pages`, which reads the field this sets.
        self.prg_ram_on = self.banking_mode & 0x80 != 0;
        self.update_banks();
        let (pages, chr, _) = self.derive_nametables();
        self.nametable_pages = pages;
        self.nametable_chr = chr;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    /// Nothing on the page gives the power-on value of `$B003`, so it is 0: mode 0, nametables
    /// from CIRAM in the header's arrangement, and the PRG-RAM disabled. Mesen powers up the
    /// same way (`VRC6.h:29-38`), and the board's games set the register before they draw.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.audio.reset();
        self.irq.reset();
        self.prg_16k = 0;
        self.prg_8k = 2;
        self.banking_mode = 0;
        self.chr = [0; 8];
        self.nametable_chr = [0; 4];
        self.prg_ram_on = false;
        self.update_banks();
        self.refresh_nametables(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.irq.asserted()
    }

    /// `M` of REQ-CORE-073, which the APU subtracts (AMD-061 §6.2; the page's own "except
    /// inverted"). One load: the level is recomputed where the chip's generators move.
    fn expansion_audio_output(&self) -> Option<i32> {
        Some(self.audio.output())
    }

    fn prg_ram_enabled(&self) -> bool {
        self.prg_ram_on
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    // No `read_nametable` and no `write_nametable`: `$B003`'s `N` is Q-123, and the reason the
    // board may not have an arm there is measured on `<Board as Mapper>::read_nametable`.

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        // "VRC6b (mapper 026) has A0 and A1 swapped."
        let address = if self.swap_address_lines {
            (address & 0xFFFC) | ((address & 0x01) << 1) | ((address & 0x02) >> 1)
        } else {
            address
        };

        match address & 0xF003 {
            // "Select 16 KB PRG ROM bank at $8000-$BFFF", four bits.
            0x8000..=0x8003 => {
                self.prg_16k = value & 0x0F;
                self.banks.map_prg_16k(0, i32::from(self.prg_16k));
            }
            0xB003 => {
                self.banking_mode = value;
                self.prg_ram_on = value & 0x80 != 0;
                self.update_chr_banks();
                self.refresh_nametables(memory);
            }
            // "Select 8 KB PRG ROM bank at $C000-$DFFF", five bits.
            0xC000..=0xC003 => {
                self.prg_8k = value & 0x1F;
                self.banks.map_prg_8k(2, i32::from(self.prg_8k));
            }
            // R0-R3 at $D000-$D003 and R4-R7 at $E000-$E003.
            0xD000..=0xD003 | 0xE000..=0xE003 => {
                let group = usize::from(address & 0xF000 == 0xE000) * 4;
                self.chr[group | usize::from(address & 0x03)] = value;
                self.update_chr_banks();
                self.refresh_nametables(memory);
            }
            0xF000 => self.irq.write_latch(value),
            0xF001 => self.irq.write_control(value),
            0xF002 => self.irq.acknowledge(),
            // The three sound channels, and their shared frequency scaling at $9003.
            0x9000..=0x9003 | 0xA000..=0xA002 | 0xB000..=0xB002 => {
                self.audio.write(address, value);
            }
            // $F003 is not a register at all, and neither is $A003.
            _ => {}
        }
    }

    /// **Out of line, measured, and the attribute belongs on *this* function and not only on
    /// the audio's own body.** This is one arm of `Board::on_cpu_cycle`, whose thirty arms share
    /// one prologue. An arm that holds a value across a call needs a callee-saved register, and
    /// the prologue that saves it is executed by every board on every CPU cycle.
    ///
    /// `#[inline(never)]` on [`Vrc6Audio::step_cpu_cycle`] alone fixed the Windows build — the
    /// prologue went back to `pushq %rsi / subq $32, %rsp` — and left the Linux one costing
    /// `Board::on_cpu_cycle` **+17.9 M instructions over 300 frames, +2 per CPU cycle, on all
    /// thirteen ROMs of the measured set and every one of them on a board with no expansion
    /// audio at all**. The two ABIs allocate registers differently, so an attribute chosen
    /// against one dump is not evidence about the other: `cargo rustc --emit=asm` runs on the
    /// Windows host and callgrind on the Linux one, and this is the first place in the lots where
    /// they disagreed. With the attribute here the arm is a tail call and holds nothing.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.audio.step_cpu_cycle();
        self.irq.step_cpu_cycle();
    }
}
