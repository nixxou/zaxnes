// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// `chr_address`'s choice between the two CHR register sets is Mesen's `chrA` expression
// (`Core/NES/Mappers/Nintendo/MMC5.h`), read back as "use set B"; see THIRD-PARTY-NOTICES.md.

use super::{windows_for, Mapper};
use crate::apu::Mmc5Audio;
use crate::cartridges::{wrap_offset, BankWindows, CartridgeMemory, ChrFetch};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NesCore;

/// iNES mapper 5, MMC5 (REQ-CORE-100): four PRG modes with RAM or ROM per window, four CHR
/// modes with two register sets (sprites and background), `ExRAM`, fill mode, a scanline
/// IRQ and a hardware multiplier (nesdev, "MMC5"). Extended attribute mode and the split
/// screen are not supported.
///
/// The **expansion sound** ($5000-$5015) is emulated as REQ-CORE-075 as amended by AMD-054
/// requires: two square channels, a PCM channel in write mode and the $5010 / $5015 registers,
/// all of it in [`Mmc5Audio`]. Its output is the `M` the APU's mixer adds to its own
/// (REQ-CORE-073); the bus reads it through [`Mapper::expansion_audio_output`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc5 {
    banks: BankWindows,
    audio: Mmc5Audio,
    ex_ram: [u8; Self::EX_RAM_SIZE],
    chr_a: [i32; 8],
    chr_b: [i32; 4],
    prg_bank: [u8; 4],
    prg_mode: i32,
    chr_mode: i32,
    chr_upper: i32,
    ex_ram_mode: i32,
    nametable_mapping: i32,
    fill_tile: u8,
    fill_attribute: u8,
    ram_bank: u8,
    protect1: i32,
    protect2: i32,
    last_set_was_b: bool,
    sprite_size16: bool,
    /// Which CHR register set a *background* fetch reads, as one bool instead of the three
    /// MR-5-1 rules recomputed per fetch (see [`Mmc5::refresh_background_chr_set`]). Derived
    /// from `sprite_size16`, `in_frame` and `last_set_was_b`, so it is not part of the
    /// savestate; [`Mapper::load_registers`] recomputes it.
    ///
    /// It must stay *before* `in_frame` in this struct, and `chr_address` must read it rather
    /// than `in_frame`: `in_frame` is the last `bool` of the board, so it is the niche rustc
    /// fills with [`super::super::board::Board`]'s discriminant. A `chr_address` that reads
    /// that byte reads the enum tag, which costs every other board 1-2 instructions per CHR
    /// fetch — see `docs/progress/MF-2.md`, step 0.
    background_uses_set_b: bool,
    irq_target: i32,
    irq_enabled: bool,
    irq_pending: bool,
    /// The board's single /IRQ line, latched rather than recomputed on every read: both the
    /// scanline IRQ of $5203/$5204 and the PCM IRQ of $5010 pull it (AMD-061 §6.3).
    ///
    /// It is a field, and `irq_asserted` is one load of it, because that method sits on the
    /// CPU's per-cycle interrupt check for **every** board. `Board::irq_asserted` is a jump
    /// table whose other twenty-five arms are a single `movzbl` each; an arm that instead read
    /// four bytes and branched grew the function by seven instructions, which was enough to
    /// change the inliner's mind inside `Cpu6502::step` and move work onto every game's
    /// per-instruction path. See `docs/progress/MF-2.md`, step 0, for the same trap on the CHR
    /// path, and [`Mmc5::refresh_irq_line`] for the five places this is kept up to date.
    irq_line: bool,
    in_frame: bool,
    /// Set by the scanline detection of [`Mmc5::on_ppu_address_bus`] and consumed by the next
    /// nametable fetch, which is what turns `in_frame` on (Mesen's `_needInFrame`).
    ///
    /// nesdev ("MMC5"): the three identical nametable reads are followed by "the next PPU read,
    /// regardless of address", which "is the point at which the scanline is detected" — that
    /// read arms this — and the flag then rises on the fetch that starts the line proper.
    need_in_frame: bool,
    scanline_counter: i32,
    /// CPU cycles left before the in-frame flag drops for want of a PPU read, reloaded to
    /// [`Mmc5::PPU_IDLE_CYCLES`] by every read and counted down by [`Mapper::on_cpu_cycle`].
    ///
    /// nesdev ("MMC5"): the flag is cleared when "3 CPU cycles pass without a PPU read having
    /// occurred (PPU /RD has not been low during the last 3 M2 rises)". Reads of **every** kind
    /// reload it, pattern fetches included: during rendering the longest silence on the PPU's bus
    /// is five dots, under two CPU cycles (measured, `docs/progress/MF-4c.md` step 2), so the
    /// counter never expires inside a frame; the 7 165-dot gap of vblank is where it fires.
    ppu_idle_counter: u8,
    /// How many times in a row the PPU has read the address in `last_ppu_read_addr`, capped at
    /// [`Mmc5::NT_READS_TO_DETECT`]. Dots 337, 338 and 340 of a line read the same nametable
    /// address, which is the three the chip looks for.
    nt_read_counter: u8,
    /// The address of the PPU's previous read, for `nt_read_counter`.
    last_ppu_read_addr: u16,
    multiplicand: u8,
    multiplier: u8,
}

impl Mmc5 {
    const EX_RAM_SIZE: usize = 1024;

    /// CPU cycles without a PPU read after which the in-frame flag drops (nesdev, "MMC5": "3
    /// CPU cycles pass without a PPU read having occurred").
    const PPU_IDLE_CYCLES: u8 = 3;

    /// Repeats of one nametable address that arm the detection: the counter is incremented by
    /// the second and the third of "three consecutive PPU reads from the same nametable
    /// address", so it reaches 2.
    const NT_READS_TO_DETECT: u8 = 2;

    /// What [`Mmc5::nt_read_counter`] is restored to by a state load, and why it is not 0.
    ///
    /// A state is written at a frame boundary, i.e. in vblank, and the last three reads before
    /// vblank are always dots 337, 338 and 340 of line 239 reading the same nametable address —
    /// so the counter is always 2 there. Restoring 2 makes the pre-render line's first read the
    /// detection point, exactly as it is without a load; restoring 0 would push the first
    /// detection to the end of the pre-render line and land every split one scanline late for
    /// that frame, which under run-ahead (a state load per frame, AMD-041) would be permanent.
    const NT_READS_AFTER_LOAD: u8 = Self::NT_READS_TO_DETECT;

    /// PRG-RAM the board is given regardless of the header (up to 64 KiB, banked).
    pub(crate) const MAX_PRG_RAM_SIZE: usize = 64 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            audio: Mmc5Audio::new(),
            ex_ram: [0; Self::EX_RAM_SIZE],
            chr_a: [0; 8],
            chr_b: [0; 4],
            prg_bank: [0, 0x80, 0x80, 0xFF],
            prg_mode: 3,
            chr_mode: 0,
            chr_upper: 0,
            ex_ram_mode: 0,
            nametable_mapping: 0,
            fill_tile: 0,
            fill_attribute: 0,
            ram_bank: 0,
            protect1: 0,
            protect2: 0,
            last_set_was_b: false,
            sprite_size16: false,
            background_uses_set_b: false,
            irq_target: 0,
            irq_enabled: false,
            irq_pending: false,
            irq_line: false,
            in_frame: false,
            need_in_frame: false,
            scanline_counter: 0,
            ppu_idle_counter: 0,
            nt_read_counter: 0,
            last_ppu_read_addr: 0,
            multiplicand: 0,
            multiplier: 0,
        }
    }

    fn status(&self) -> u8 {
        (if self.irq_pending { 0x80 } else { 0 }) | (if self.in_frame { 0x40 } else { 0 })
    }

    fn product(&self) -> u16 {
        u16::from(self.multiplicand) * u16::from(self.multiplier)
    }

    fn write_control(&mut self, address: u16, value: u8) {
        let v = i32::from(value);
        match address {
            0x5100 => {
                self.prg_mode = v & 0x03;
                self.update_banks();
            }
            0x5101 => self.chr_mode = v & 0x03,
            0x5102 => self.protect1 = v,
            0x5103 => self.protect2 = v,
            0x5104 => self.ex_ram_mode = v & 0x03,
            0x5105 => self.nametable_mapping = v,
            0x5106 => self.fill_tile = value,
            0x5107 => self.fill_attribute = value & 0x03,
            0x5113 => {
                self.ram_bank = value & 0x07;
                self.update_banks();
            }
            0x5114..=0x5117 => {
                self.prg_bank[usize::from(address - 0x5114)] = value;
                self.update_banks();
            }
            0x5120..=0x5127 => {
                self.chr_a[usize::from(address - 0x5120)] = v | (self.chr_upper << 8);
                self.last_set_was_b = false;
                self.refresh_background_chr_set();
            }
            0x5128..=0x512B => {
                self.chr_b[usize::from(address - 0x5128)] = v | (self.chr_upper << 8);
                self.last_set_was_b = true;
                self.refresh_background_chr_set();
            }
            0x5130 => self.chr_upper = v & 0x03,
            0x5203 => self.irq_target = v,
            0x5204 => {
                self.irq_enabled = value & 0x80 != 0;
                self.refresh_irq_line();
            }
            0x5205 => self.multiplicand = value,
            0x5206 => self.multiplier = value,
            _ => {}
        }
    }

    fn update_banks(&mut self) {
        match self.prg_mode {
            0 => self.map_prg_window(0, self.prg_bank[3], 4),
            1 => {
                self.map_prg_window(0, self.prg_bank[1], 2);
                self.map_prg_window(2, self.prg_bank[3], 2);
            }
            2 => {
                self.map_prg_window(0, self.prg_bank[1], 2);
                self.map_prg_window(2, self.prg_bank[2], 1);
                self.map_prg_window(3, self.prg_bank[3], 1);
            }
            _ => {
                for slot in 0..4 {
                    self.map_prg_window(slot, self.prg_bank[slot], 1);
                }
            }
        }
    }

    fn map_prg_window(&mut self, slot: usize, register: u8, windows: usize) {
        let rom = register & 0x80 != 0 || slot + windows == BankWindows::PRG_WINDOWS;
        let bank = i32::from(register & 0x7F);
        // The bank number counts in 8 KiB units, so a 16 or 32 KiB window drops the bits the
        // larger granularity does not use.
        let first = bank & !(i32::try_from(windows).unwrap_or(1) - 1);
        for i in 0..windows {
            let step = i32::try_from(i).unwrap_or(0);
            if rom {
                self.banks.map_prg_8k(slot + i, first + step);
            } else {
                self.banks.map_prg_ram_8k(slot + i, (first + step) & 0x07);
            }
        }
    }

    /// Recomputes [`Mmc5::irq_line`] from its four inputs: the scanline IRQ's flag and enable,
    /// and the PCM IRQ's. Called wherever one of them moves, never on a CPU cycle.
    fn refresh_irq_line(&mut self) {
        self.irq_line = (self.irq_pending && self.irq_enabled) || self.audio.irq_asserted();
    }

    /// One PPU read, as the board's own bus brings it: the scanline detection and the reload of
    /// the idle counter (nesdev, "MMC5"; finding MR-5-4).
    ///
    /// **`inline(never)`, and measured.** `Board::on_ppu_address_bus` is the out-of-line
    /// dispatch the three A12 boards take on every fetch — 17.3 M calls over 300 frames of
    /// *Super Mario Bros. 3*. Letting this body into the MMC5's arm raised the register pressure
    /// of the whole function and added **one instruction to the dispatch's own prologue**
    /// (`movl %eax, %r9d`, keeping the board tag alive across the jump-table index), which
    /// callgrind priced at **+16.6 M on SMB3**: a regression on a board this lot does not touch.
    /// Behind a call the arm is three instructions, the prologue is byte for byte the one at
    /// `a5994fd1`, and only the MMC5 pays the call.
    #[inline(never)]
    fn see_ppu_read(&mut self, address: u16) {
        // A nametable byte, as opposed to an attribute byte of the same $2xxx page: only this
        // fetch turns an armed detection into the flag, because it is the first fetch of the
        // line proper.
        let nametable_fetch = (0x2000..0x3000).contains(&address) && (address & 0x03FF) < 0x03C0;
        if nametable_fetch && self.need_in_frame {
            self.need_in_frame = false;
            self.in_frame = true;
            self.refresh_background_chr_set();
        }

        if self.nt_read_counter >= Self::NT_READS_TO_DETECT {
            self.detect_scanline();
        } else if (0x2000..0x3000).contains(&address) && self.last_ppu_read_addr == address {
            // "in the range $2xxx": an attribute address would count too, but the three in a row
            // only ever happen at dots 337, 338 and 340, which read the nametable byte.
            self.nt_read_counter += 1;
        }

        if self.last_ppu_read_addr != address {
            self.nt_read_counter = 0;
        }

        self.last_ppu_read_addr = address;
        self.ppu_idle_counter = Self::PPU_IDLE_CYCLES;
    }

    /// The point at which the scanline is detected, reached by the PPU read that follows three
    /// identical nametable reads (nesdev, "MMC5"; finding MR-5-4).
    ///
    /// "If the 'in-frame' flag (register $5204) was clear, it becomes set, and the internal
    /// 8-bit scanline counter is reset to zero; but if it was already set, the scanline counter
    /// is incremented, then compared against the value written to $5203." The becoming-set is
    /// one fetch away ([`Mmc5::need_in_frame`]), so the first detected line only arms and
    /// zeroes, with **no compare** — MR-5-3, which this keeps.
    ///
    /// **Q-121.** Which PPU line is the *first* detected one is not in the source, and it decides
    /// every MMC5 split to one line. The pre-render line fetches exactly like a visible one, the
    /// three identical nametable reads of its dots 337, 338 and 340 included, and nothing
    /// documented resets the repeat counter across V-blank — so it is detected, visible line 0 is
    /// the first *counted* line, and `$5203` = N fires on visible line N-1. That is Mesen's
    /// behaviour and it agrees with the page's "the MMC5's scanline IRQ occurs at PPU cycle 4".
    /// The ticket carries the alternative and its one-line shape.
    // Q-121
    fn detect_scanline(&mut self) {
        if !self.in_frame && !self.need_in_frame {
            self.need_in_frame = true;
            self.scanline_counter = 0;
            return;
        }

        self.scanline_counter = (self.scanline_counter + 1) & 0xFF;
        // "Value $00 is a special case that will not produce IRQ pending conditions" ($5203).
        if self.irq_target != 0 && self.scanline_counter == self.irq_target {
            self.irq_pending = true;
            self.refresh_irq_line();
        }
    }

    /// Drops the in-frame flag, which only the two clears of nesdev's description do: the idle
    /// counter running out, and the CPU reading the NMI vector at $FFFA/$FFFB.
    fn leave_frame(&mut self) {
        self.in_frame = false;
        self.need_in_frame = false;
        self.refresh_background_chr_set();
    }

    /// Recomputes [`Mmc5::background_uses_set_b`] from the three inputs MR-5-1 names. Called
    /// at each of the five places one of them changes, never on a fetch.
    fn refresh_background_chr_set(&mut self) {
        // MR-5-1. Three separate rules, not one (nesdev, "MMC5"): with 8x8 sprites
        // "$5128-$512B are completely ignored", so set A serves everything; with 8x16 sprites
        // the chip "keeps track of whether the PPU is fetching background tiles or sprite
        // tiles", and the sprite fetches take set A; and only outside rendering does "the last
        // set of registers written to" decide. We had been using the last-written set as *the*
        // 8x8 rule, so an 8x8 game that last wrote $5128-$512B rendered from set B throughout.
        self.background_uses_set_b = self.sprite_size16 && (self.in_frame || self.last_set_was_b);
    }

    fn chr_address(&self, address: u16, fetch: ChrFetch) -> usize {
        let index = i32::from(address & 0x1FFF);
        // The three MR-5-1 rules, precomputed: everything but the fetch kind is known before
        // the fetch. One bool load, and not the one holding `Board`'s tag (see the field).
        let use_b = self.background_uses_set_b && fetch == ChrFetch::Background;
        let offset = match self.chr_mode {
            0 => {
                let bank = if use_b { self.chr_b[3] } else { self.chr_a[7] };
                bank * 0x2000 + index
            }
            1 => {
                let window = usize::try_from((index >> 12) & 0x01).unwrap_or(0);
                let bank = if use_b {
                    self.chr_b[3]
                } else {
                    self.chr_a[3 + 4 * window]
                };
                bank * 0x1000 + (index & 0x0FFF)
            }
            2 => {
                let window = usize::try_from((index >> 11) & 0x03).unwrap_or(0);
                let bank = if use_b {
                    self.chr_b[1 + 2 * (window & 1)]
                } else {
                    self.chr_a[1 + 2 * window]
                };
                bank * 0x0800 + (index & 0x07FF)
            }
            _ => {
                let window = usize::try_from((index >> 10) & 0x07).unwrap_or(0);
                let bank = if use_b {
                    self.chr_b[window & 0x03]
                } else {
                    self.chr_a[window]
                };
                bank * 0x0400 + (index & 0x03FF)
            }
        };
        usize::try_from(offset).unwrap_or(0)
    }

    fn read_chr(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        let offset = self.chr_address(address, fetch);
        let chr_rom = memory.rom().chr_rom();
        if chr_rom.is_empty() {
            let ram = memory.chr_ram();
            return if ram.is_empty() {
                0
            } else {
                ram[wrap_offset(offset, ram.len())]
            };
        }

        chr_rom[wrap_offset(offset, chr_rom.len())]
    }

    fn nametable_mode(&self, address: u16) -> i32 {
        let table = i32::from(address.wrapping_sub(0x2000) >> 10) & 0x03;
        (self.nametable_mapping >> (table * 2)) & 0x03
    }
}

impl Mapper for Mmc5 {
    /// Its reads of $8000-$FFFF go through [`Mapper::cpu_read_override`].
    fn rom_windows(&self, _memory: &CartridgeMemory) -> [usize; 4] {
        [super::NO_ROM_WINDOW; 4]
    }

    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.ex_ram);
        for value in self.chr_a {
            writer.write_i32(value);
        }
        for value in self.chr_b {
            writer.write_i32(value);
        }
        writer.write_raw(&self.prg_bank);
        writer.write_i32(self.prg_mode);
        writer.write_i32(self.chr_mode);
        writer.write_i32(self.chr_upper);
        writer.write_i32(self.ex_ram_mode);
        writer.write_i32(self.nametable_mapping);
        writer.write_u8(self.fill_tile);
        writer.write_u8(self.fill_attribute);
        writer.write_u8(self.ram_bank);
        writer.write_i32(self.protect1);
        writer.write_i32(self.protect2);
        writer.write_bool(self.last_set_was_b);
        writer.write_bool(self.sprite_size16);
        writer.write_i32(self.irq_target);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        writer.write_bool(self.in_frame);
        writer.write_i32(self.scanline_counter);
        writer.write_u8(self.multiplicand);
        writer.write_u8(self.multiplier);

        // Last, so that a block written before AMD-054 (format version 4 to 7) reads back
        // without it (REQ-CORE-091).
        self.audio.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_blob_into(&mut self.ex_ram)?;
        for value in &mut self.chr_a {
            *value = reader.read_i32()?;
        }
        for value in &mut self.chr_b {
            *value = reader.read_i32()?;
        }
        reader.read_into(&mut self.prg_bank)?;
        self.prg_mode = reader.read_i32()?;
        self.chr_mode = reader.read_i32()?;
        self.chr_upper = reader.read_i32()?;
        self.ex_ram_mode = reader.read_i32()?;
        self.nametable_mapping = reader.read_i32()?;
        self.fill_tile = reader.read_u8()?;
        self.fill_attribute = reader.read_u8()?;
        self.ram_bank = reader.read_u8()?;
        self.protect1 = reader.read_i32()?;
        self.protect2 = reader.read_i32()?;
        self.last_set_was_b = reader.read_bool()?;
        self.sprite_size16 = reader.read_bool()?;
        self.irq_target = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.in_frame = reader.read_bool()?;
        self.scanline_counter = reader.read_i32()?;
        self.multiplicand = reader.read_u8()?;
        self.multiplier = reader.read_u8()?;

        if NesCore::state_version_has_mmc5_audio(reader.format_version()) {
            self.audio.load_state(reader)?;
        } else {
            // An older block carries no sound state: the board starts silent, as it does
            // after a reset, and the game's next write to $5000-$5015 sets it up again.
            self.audio.reset();
        }
        // The scanline detection's own state is **not** in the block and no format version
        // moves for it: it is re-derived within one line, and the one value a load has to get
        // right is `nt_read_counter` — see `Mmc5::NT_READS_AFTER_LOAD`. The idle counter starts
        // disarmed so that the load cannot drop a flag the block says is set; the first PPU read
        // after the load arms it again.
        self.need_in_frame = false;
        self.ppu_idle_counter = 0;
        self.nt_read_counter = Self::NT_READS_AFTER_LOAD;
        self.last_ppu_read_addr = 0;

        // Both are derived from registers the block already carries, so no older block has to
        // carry them and no format version moves.
        self.refresh_background_chr_set();
        self.refresh_irq_line();
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
        5
    }

    /// The scanline IRQ of $5203/$5204, and — since AMD-061 §6.3 — the PCM IRQ of $5010 as
    /// well: nesdev gives the board one /IRQ line, `NOT(irqTrip AND irqEnable)`, and both
    /// sources pull it. Latched in [`Mmc5::irq_line`], which that field's comment explains.
    fn irq_asserted(&self) -> bool {
        self.irq_line
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        None
    }

    /// `M` of REQ-CORE-073: the board's two square channels and its PCM level, in the mixer's
    /// integer units (REQ-CORE-075). Every other board leaves the trait's `None`, so nothing
    /// of this reaches the bus's cycle for them.
    fn expansion_audio_output(&self) -> Option<i32> {
        Some(self.audio.output())
    }

    /// The board's sound has no equivalent of the APU's frame sequencer, so it counts CPU
    /// cycles of its own (REQ-CORE-075). The MMC5 is one of the two boards that already step
    /// every cycle, so this costs nothing anywhere else.
    ///
    /// The in-frame flag's idle clear rides with it, for the same reason — finding MR-5-4,
    /// nesdev ("MMC5"): the flag "is cleared when the PPU is no longer rendering. This is
    /// detected when 3 CPU cycles pass without a PPU read having occurred". Every PPU read
    /// reloads [`Mmc5::ppu_idle_counter`]; the counter sits at 0 between frames and the test is
    /// then one compare.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.audio.step_cpu_cycle();

        if self.ppu_idle_counter > 0 {
            self.ppu_idle_counter -= 1;
            if self.ppu_idle_counter == 0 {
                self.leave_frame();
            }
        }
    }

    fn prg_ram_writable(&self) -> bool {
        (self.protect1 & 0x03) == 0x02 && (self.protect2 & 0x03) == 0x01
    }

    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        (usize::from(self.ram_bank) * BankWindows::PRG_WINDOW_SIZE + usize::from(address - 0x6000))
            % memory.prg_ram().len()
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.audio.reset();
        self.ex_ram = [0; Self::EX_RAM_SIZE];
        self.chr_a = [0; 8];
        self.chr_b = [0; 4];
        // Power-on: PRG mode 3 with the last bank at $E000, the documented state so that a
        // reset vector can be fetched.
        self.prg_mode = 3;
        self.prg_bank = [0, 0x80, 0x80, 0xFF];
        self.ram_bank = 0;
        self.chr_mode = 0;
        self.chr_upper = 0;
        self.ex_ram_mode = 0;
        self.nametable_mapping = 0;
        self.fill_tile = 0;
        self.fill_attribute = 0;
        self.protect1 = 0;
        self.protect2 = 0;
        self.last_set_was_b = false;
        self.sprite_size16 = false;
        self.irq_target = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_line = false;
        self.in_frame = false;
        self.need_in_frame = false;
        self.scanline_counter = 0;
        // Power-on: nothing has read the PPU bus yet, so the detection starts from scratch and
        // the idle counter is not armed (see `load_registers` for why a *load* differs).
        self.ppu_idle_counter = 0;
        self.nt_read_counter = 0;
        self.last_ppu_read_addr = 0;
        self.multiplicand = 0;
        self.multiplier = 0;
        self.refresh_background_chr_set();
        self.update_banks();
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        match address {
            // Finding MR-5-4, nesdev ("MMC5"): the in-frame flag is also reset when "the V-blank
            // NMI occurs, i.e. CPU reads the interrupt vector from addresses $FFFA and $FFFB" or
            // when "the user program intentionally reads from CPU addresses $FFFA or $FFFB". The
            // byte itself is not ours to answer — `None` sends the read on to the default PRG
            // path, which is where it was going anyway.
            //
            // This costs no other board anything and the MMC5 one compare it was already paying
            // for: `Mmc5::rom_windows` returns `NO_ROM_WINDOW` on all four windows, so every
            // MMC5 read of $8000-$FFFF already came through here.
            //
            // Only the flag is cleared. Mesen also zeroes the scanline counter, clears the
            // pending IRQ and forgets the last read address; the source says none of that, and
            // zeroing the counter is redundant — the next detection finds the flag clear and
            // zeroes it itself.
            0xFFFA | 0xFFFB => {
                self.leave_frame();
                None
            }
            0x5204 => {
                let value = self.status();
                self.irq_pending = false;
                self.refresh_irq_line();
                Some(Some(value))
            }
            // Reading $5010 acknowledges the PCM IRQ flag (REQ-CORE-075), so the host
            // accessor must not take this path (REQ-CORE-081).
            0x5010 | 0x5015 => {
                let value = self.audio.read(address);
                self.refresh_irq_line();
                Some(value)
            }
            // AMD-061 §6.3, the read mode of $5010: "une lecture CPU dans $8000-$BFFF place
            // l'octet lu dans le niveau PCM ; une lecture de valeur 0 lève le drapeau d'IRQ
            // PCM". The byte is the one the board would have driven anyway, so the read is
            // answered exactly as before and only observed on the way past.
            //
            // This costs no other board anything, and the MMC5 nothing it was not already
            // paying: `Mmc5::rom_windows` returns `NO_ROM_WINDOW` for all four windows, so
            // every MMC5 read of $8000-$FFFF already came through here rather than through
            // `Cartridge::read_rom_window`. Out of read mode — which is every MMC5 game that
            // does not drive PCM from reads — the guard is one bool test on that path.
            0x8000..=0xBFFF if self.audio.pcm_read_mode() => {
                let value = self.try_read_prg(memory, address);
                if let Some(byte) = value {
                    self.audio.feed_pcm_read(byte);
                    self.refresh_irq_line();
                }

                Some(value)
            }
            _ => self.cpu_peek_override(memory, address),
        }
    }

    fn cpu_peek_override(&self, _memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        match address {
            0x5204 => Some(Some(self.status())),
            // The side-effect-free read of the same two registers (REQ-CORE-081): the flag of
            // $5010 is reported without being acknowledged.
            0x5010 | 0x5015 => Some(self.audio.peek(address)),
            0x5205 => Some(Some(self.product().to_le_bytes()[0])),
            0x5206 => Some(Some(self.product().to_le_bytes()[1])),
            // ExRAM reads back only in the two RAM modes. MR-5-5, nesdev ("MMC5"), the ExRAM
            // table for modes %00 and %01: a CPU read "returns open bus"; we used to answer a
            // driven 0 there.
            0x5C00..=0x5FFF => Some(if self.ex_ram_mode >= 2 {
                Some(self.ex_ram[usize::from(address - 0x5C00)])
            } else {
                None
            }),
            _ => None,
        }
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if (0x5C00..=0x5FFF).contains(&address) {
            // MR-5-2, nesdev ("MMC5"), the ExRAM table. In the two nametable modes "writes are
            // only allowed when the PPU is rendering. If writes are attempted during V-blank,
            // they may be ignored or cause a corruption at that memory address" — so storing
            // the value, which is what we did in every mode but 3, is the one behaviour the
            // hardware does not have. Of the two the source allows we take **ignored**, the
            // cheaper and less surprising of them; Mesen writes 0, the "corruption" reading,
            // and the two are indistinguishable to a game that reads back what it wrote.
            // Mode 2 is the plain read/write RAM mode and mode 3 is read-only.
            let writable = match self.ex_ram_mode {
                0 | 1 => self.in_frame,
                2 => true,
                _ => false,
            };
            if writable {
                self.ex_ram[usize::from(address - 0x5C00)] = value;
            }

            return true;
        }

        if (0x5000..=0x5015).contains(&address) {
            // The board's own sound registers (REQ-CORE-075).
            self.audio.write(address, value);
            self.refresh_irq_line();
            return true;
        }

        if (0x5100..=0x5206).contains(&address) {
            self.write_control(address, value);
            return true;
        }

        if address >= 0x8000 {
            // A PRG window showing RAM takes writes; one showing ROM ignores them.
            let writable = self.prg_ram_writable();
            self.banks
                .try_write_prg_window(address, value, memory.prg_ram_mut(), writable);
            return true;
        }

        false
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.read_chr(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if memory.rom().chr_rom().is_empty() {
            let offset = self.chr_address(address, ChrFetch::Background);
            let ram = memory.chr_ram_mut();
            if !ram.is_empty() {
                let len = ram.len();
                ram[wrap_offset(offset, len)] = value;
            }
        }
    }

    fn nametable_page(&self, table: i32) -> i32 {
        match (self.nametable_mapping >> (table * 2)) & 0x03 {
            0 => 0,
            1 => 1,
            _ => -2,
        }
    }

    fn read_nametable(&self, _memory: &CartridgeMemory, address: u16) -> u8 {
        if self.nametable_mode(address) == 2 {
            // ExRAM as a nametable: only the two nametable modes show it to the PPU.
            return if self.ex_ram_mode < 2 {
                self.ex_ram[usize::from(address & 0x03FF)]
            } else {
                0
            };
        }

        // Fill mode: one tile everywhere, one attribute everywhere.
        if address & 0x03FF >= 0x03C0 {
            self.fill_attribute * 0x55
        } else {
            self.fill_tile
        }
    }

    fn write_nametable(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.nametable_mode(address) == 2 && self.ex_ram_mode < 2 {
            self.ex_ram[usize::from(address & 0x03FF)] = value;
        }
    }

    /// Every PPU read, which is how the chip finds the start of a scanline — **finding MR-5-4,
    /// the answer to Q-116 issue 3.**
    ///
    /// nesdev ("MMC5"): "The MMC5 detects scanlines by first looking for three consecutive PPU
    /// reads from the same nametable address in the range $2xxx. Once this has been seen, the
    /// next PPU read, regardless of address, is the point at which the scanline is detected."
    /// The three are dots 337, 338 and 340 of a line — the fetch that loads the first tile of the
    /// next line, and the two the hardware performs and discards — and "the next PPU read" is the
    /// attribute fetch of dot 3.
    ///
    /// This runs on **every** PPU read of an MMC5 game and on no other board's: the three cheap
    /// tests of a non-nametable read are what a pattern fetch pays. The read notification itself
    /// is not new — the PPU has always told the cartridge every address it reads, which is what
    /// Q-116 missed; see `docs/progress/MF-4c.md` step 2.
    // Q-116
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        self.see_ppu_read(address);
    }

    fn on_ppu_control(&mut self, control: u8) {
        self.sprite_size16 = control & 0x20 != 0;
        self.refresh_background_chr_set();
    }
}
