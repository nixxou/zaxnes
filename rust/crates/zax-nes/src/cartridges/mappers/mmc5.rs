// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// `chr_address`'s choice between the two CHR register sets is Mesen's `chrA` expression
// (`Core/NES/Mappers/Nintendo/MMC5.h`), read back as "use set B"; see THIRD-PARTY-NOTICES.md.

#[cfg(feature = "board-mmc5-exattr-split")]
use super::CpuReadWindows;
use super::{windows_for, Mapper};
use crate::apu::Mmc5Audio;
#[cfg(feature = "board-mmc5-exattr-split")]
use crate::cartridges::ChrView;
use crate::cartridges::{wrap_offset, BankWindows, CartridgeMemory, ChrFetch};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NesCore;

/// iNES mapper 5, MMC5 (REQ-CORE-100): four PRG modes with RAM or ROM per window, four CHR
/// modes with two register sets (sprites and background), `ExRAM`, fill mode, a scanline
/// IRQ and a hardware multiplier (nesdev, "MMC5").
///
/// **Extended attribute mode (`$5104` = %01) and the vertical split (`$5200`-`$5202`) are not
/// here**: a game that arms one of them stops running this board and runs [`Mmc5Ext`] instead,
/// from the write that arms it (lot MF-LEG-3, the owner's decision of 2026-11-09; the banner
/// above that struct has the whole story). So **this** board is the one report entry MMC5-1
/// describes — the entry's verdict DIFFERS, five emulators to one and against the page — and it
/// is `04731c86`'s code to the instruction, feature on or off, which is what *Castlevania III*
/// runs: it uses neither mode, and lot MF-LEG-2 had measured it at **+2.992 %** for them.
///
/// With the feature **off** [`Mmc5Ext`] does not exist, the `Board` variant that reaches it is
/// not compiled, and `$5104` mode %01 renders exactly as mode %00 does.
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

    /// Whether this build has extended attribute mode and the vertical split, i.e. whether
    /// `board-mmc5-exattr-split` is on (lots MF-LEG-2 and MF-LEG-3).
    ///
    /// `pub` and in **every** build, so that one test can assert both sides of the owner's
    /// decision rather than two tests asserting one each: with it `false`, `$5200`-`$5202` are
    /// ignored, `$5104` mode %01 renders exactly as mode %00 does and no write can ever bring
    /// [`Mmc5Ext`] into being — which is what report entry MMC5-1's verdict DIFFERS describes.
    pub const EXATTR_SPLIT: bool = cfg!(feature = "board-mmc5-exattr-split");

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

    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        (
            usize::from(self.ram_bank) * BankWindows::PRG_WINDOW_SIZE,
            memory.prg_ram().len(),
        )
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

// ==========================================================================================
// Extended attribute mode and the vertical split — feature `board-mmc5-exattr-split`
// ==========================================================================================
//
// nesdev ("MMC5"), the two features report entry MMC5-1 is about: lot **MF-LEG-2** implemented
// them inside [`Mmc5`] and lot **MF-LEG-3** moved them out into the board below, which the
// enum reaches through its own variant. Both substitute data on the **background** fetch path
// and nowhere else: sprite patterns, `$2007` accesses and every CPU read are untouched ("CHR
// mode always applies to sprite pattern fetches").
//
// **Extended attribute mode, `$5104` = %01.** "Each byte of Expansion RAM is used to enhance
// the tile at the corresponding address in every nametable", as `AACC CCCC`: bits 7-6 are the
// palette of the whole 8x8 tile, replacing the attribute byte, and bits 5-0 pick a 4 KiB CHR
// bank for the tile's two pattern fetches, with `$5130`'s two bits as bank bits 7-6 ("the
// pattern fetches ignore the standard CHR banking bits"). The nametable fetch itself is **not**
// replaced — "nametable fetches are processed normally, and can come from CIRAM nametables,
// fill mode, or even Expansion RAM".
//
// **The vertical split, `$5200`-`$5202`.** "Based on nametable fetch count (horizontally) and
// scanline count (vertically), the MMC5 makes substitutions to the nametable data, attribute
// table data, and the lowest 3 address bits of the pattern data." Inside the region the
// nametable byte **and** the attribute byte come from `ExRAM` — "MMC5 internal extended RAM is
// always used as the nametable in split screen mode" — and the pattern from `$5202`'s bank.
// Outside it, mode 1 still applies if it is on: Nestopia composes the two in `FetchNtExtSplit`
// (`NstBoardMmc5.cpp:938-955`) exactly that way, and the page agrees — "the non-split region
// gains extended attributes, and the split region does not".
//
// **Where the five emulators disagree, and what this build does.**
//
// * *The region test.* `$5200` bit 6 is the side the split region is on. Left is
//   `column < threshold` and right is `column >= threshold` in FCEUX (`mmc5.cpp:186-193`),
//   Nestopia (`:856-857`) and Mesen (`MMC5.h:487-495`, a toggle at the delimiter starting
//   inside at column 0); puNES alone uses `column <= threshold` on the left
//   (`mapper_005.c:475`). **Four to one**, so this build is strict.
// * *The split's row.* The page says the counter "resets to the vertical scrolling value in
//   `$5201` each v-blank", so visible line L shows row `$5201 + L`: puNES (`:396-405`) and
//   Nestopia (`:681-692`) do exactly that, Mesen is one line later (`MMC5.h:483-484`,
//   `_scanlineCounter + 1`) and FCEUX one earlier and to tile granularity (`:243` with its own
//   `kHack = -1` comment, and `$5201` stored as `(V >> 3) & 0x1F` at `:569`). This build takes
//   the page and the two that agree with it; **Q-177** records the disagreement.
// * *The fine vertical scroll.* Mesen substitutes the pattern address's low three bits from the
//   split's row (`MMC5.h:513`), which is the page's "and the lowest 3 address bits of the
//   pattern data"; puNES (`:452`) and FCEUX (`:208`) keep the PPU's, so their split regions
//   scroll in whole tiles only. This build follows the **page** and Mesen.
//
// ------------------------------------------------------------------------------------------
// **What a game that arms neither mode pays: nothing, because it is not this board.**
//
// MF-LEG-2 put both modes inside [`Mmc5`] behind one `bool` test on the fetch path, and the
// figures are what parked the feature: *Castlevania III*, which uses neither, paid **+2.992 %**
// and the `mmc5` ROM +2.522 %, while every other board paid up to **+0.110 %** because the
// MMC5's arm of `Board::ppu_peek` grew by that test and the dispatch's prologue grew with it
// (`docs/progress/MF-LEG-2.md` step 4; seven shapes were measured and none moved that figure).
//
// So the two modes are **armed at run time** (lot MF-LEG-3, the owner's decision of 2026-11-09).
// A game cannot reach either without first writing `$5104` with mode %01 or `$5200` with bit 7
// set, and both writes go through this board: until one of them arrives the cartridge holds
// [`super::super::board::Board::Mmc5`], whose code is `04731c86`'s to the instruction, and the
// write replaces it with [`super::super::board::Board::Mmc5Ext`] — this board — carrying every
// register, memory and counter across. **No write ever switches back**: [`Mmc5Ext`] is correct
// with both modes off, which is what
// `mf_leg_3_the_full_variant_renders_like_the_light_one_with_neither_mode` pins by running the
// same twenty frames through both boards and comparing every pixel. Three things do move the
// variant, and only three: a reset (back to the light board, as a power-on is), a **savestate**
// load (`NesCore::restore_state` builds a new cartridge, so the variant is a pure function of the
// block — which is what makes rewind and run-ahead deterministic) and a **core-state** restore
// (rewind's own path, which loads into the attached cartridge: a light board is promoted if the
// block says a mode is armed, and a full board stays full).
//
// **The three registers of a board that has not switched.** `$5201` and `$5202` say nothing
// until the split is enabled, and `$5200`'s low bits say nothing until its own bit 7 is set —
// which is the same write. So a light board has no use for any of them, and the switch is armed
// by a write to **`$5201` or `$5202` as well**: the light board then drops nothing, and the
// hardware's "write the bank and the scroll, then enable" order (the natural one) cannot lose a
// register. `$5200` with bit 7 **clear** does not switch, and neither does `$5104` = 0, 2 or 3.

/// The MMC5 with **extended attribute mode and the vertical split**: [`Mmc5`] verbatim plus the
/// ten fields the two modes need, reached through [`super::super::board::Board::Mmc5Ext`] from
/// the write that arms one of them (see the banner above, and [`Mmc5Ext::write_arms_substitution`]).
///
/// Every method that the two modes do not touch forwards to `base`, so there is **one**
/// implementation of the MMC5's PRG decode, sound, IRQ, scanline detection and bank windows and
/// this board cannot drift from it. The three that are not a forward — `ppu_peek`,
/// `on_ppu_address_bus` and `read_nametable_from_vram` — are each `#[inline(never)]`, so the arm
/// this board adds to a `Board` dispatch is a load and a call: the growth MF-LEG-2 measured at
/// one instruction per CHR fetch *for every board* was the size of the MMC5's arm, and an arm
/// that is a call does not have it.
#[cfg(feature = "board-mmc5-exattr-split")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc5Ext {
    /// The light board, **verbatim and whole**: the switch moves it here field for field and
    /// every forward below reads it, so no register, memory, counter or flag is duplicated.
    base: Mmc5,
    /// `$5200`: bit 7 enables the vertical split, bit 6 picks the side (0 left, 1 right) and
    /// bits 4-0 are the threshold tile count (nesdev, "MMC5"; `ESxW WWWW`).
    split_control: u8,
    /// `$5201`: "all eight bits specify the vertical scroll value to use in split region".
    split_scroll: u8,
    /// `$5202`: "all eight bits select a 4 KB CHR bank", the one the whole split region's
    /// patterns come from.
    split_bank: u8,
    /// The low ten bits of the last background **nametable** fetch — the tile's offset in
    /// whichever nametable, which is the index of its byte in `ExRAM`: "each byte of the MMC5's
    /// internal extended RAM is used to enhance the background tile at the corresponding
    /// nametable address".
    ///
    /// Latched because the *attribute* address does not carry it (it keeps coarse Y bits 4-2 and
    /// coarse X bits 4-2 only), and the attribute fetch is where mode 1 has to answer. Not in
    /// the savestate: it is re-derived by the next nametable fetch, i.e. within one tile.
    ex_attr_index: u16,
    /// `ExRAM` index of the split tile being fetched, `((split_row & 0xF8) << 2) | column` —
    /// puNES `mapper_005.c:477`, Nestopia `NstBoardMmc5.cpp:860`, Mesen `MMC5.h:502`, the three
    /// agreeing to the expression.
    split_tile: u16,
    /// The split region's current pixel row, 0 to 239: `$5201` plus the line, the region
    /// scrolling independently of the rest of the screen. See [`Mmc5Ext::reload_split_row`] for
    /// the one line of it this build had to choose.
    split_row: u8,
    /// The split's own tile counter, 5 bits as the hardware's threshold field is: the screen tile
    /// column of the tile being fetched. Reset at the dot where the PPU starts a line's fetch
    /// sequence and pre-incremented by each counted nametable fetch, so the first tile of a line
    /// is column 0. Not in the savestate, for the reason `Mmc5::nt_read_counter` is not: a line
    /// re-derives it.
    split_column: u8,
    /// Pattern fetches ($0000-$1FFF) seen since the last nametable-space read, capped at
    /// [`Mmc5Ext::CHR_READS_BEFORE_ROW_START`]: how the board finds, **on its own bus**, the dot
    /// where the PPU begins a line's fetch sequence.
    ///
    /// Inside a tile group the PPU reads the nametable byte, the attribute byte and the two
    /// pattern planes, so exactly **two** pattern fetches separate one nametable-space read from
    /// the next. Between the last nametable fetch of a line and the first of the next line's
    /// prefetch lie the **sixteen** sprite pattern fetches of dots 257-320. So a nametable fetch
    /// that follows four or more pattern fetches is the first of a line's sequence, and nothing
    /// else is.
    ///
    /// Counting it here rather than taking a hook from the PPU is not a stylistic choice: a
    /// `dot == 321` test inside `Ppu2C02::run_rendering_dot` moved **+5.4 M to +13.6 M
    /// instructions onto `Ppu2C02::run_dots` for every ROM of PR-1's set** — the function the
    /// per-dot pipeline inlines into — by changing the inliner's mind, which is
    /// `docs/progress/MF-2.md` step 0's trap one function further out. See
    /// `docs/progress/MF-LEG-2.md` step 4.
    chr_reads_since_nt: u8,
    /// Whether the tile being fetched is in the split region, decided by the counter above at the
    /// nametable fetch and read again by the attribute and the two pattern fetches of the same
    /// tile (puNES's `split_in_reg`, Nestopia's `spliter.inside`).
    split_in_region: bool,
    /// Whether the per-tile state above was positioned by the **tile capture** rather than by a
    /// rendered fetch (see [`Mmc5Ext::position_captured_tile`]).
    ///
    /// It changes one thing: the split's pattern fetch keeps the PPU's own low three address bits
    /// instead of substituting the region's fine vertical scroll. The capture reads a whole 8x8
    /// tile — its eight rows at once, through `Ppu2C02::read_pattern_planes` — and substituting
    /// one row number into all eight would collapse the tile to a single line. The record it
    /// emits is therefore the split's tile as a tile, which is what a 3D block is; the sub-tile
    /// offset of a fine-scrolled region is the one thing the 3D view does not carry, and
    /// `docs/progress/MF-LEG-2.md` step 2 says so.
    ///
    /// Not in the savestate: the capture sets it and puts it back within one line.
    split_capture: bool,
}

#[cfg(feature = "board-mmc5-exattr-split")]
impl Mmc5Ext {
    /// Pattern fetches that must precede a nametable fetch for it to be the first of a line's
    /// fetch sequence: two separate the nametable-space reads of one tile group, and sixteen
    /// (eight sprites, two planes) separate the last group of a line from the first of the next
    /// line's prefetch. Four is the midpoint and any value from three to sixteen gives the same
    /// answer. See [`Mmc5Ext::chr_reads_since_nt`].
    const CHR_READS_BEFORE_ROW_START: u8 = 4;

    // --------------------------------------------------------------- the run-time switch

    /// Whether the CPU write of `value` to `address` **arms** one of the two modes, i.e. whether
    /// the cartridge must stop being [`super::super::board::Board::Mmc5`] and become
    /// [`super::super::board::Board::Mmc5Ext`]. The one test the light board's write arm makes,
    /// and the only thing lot MF-LEG-3 puts on a path a game that arms neither mode walks.
    ///
    /// `$5104` mode %01 and `$5200` bit 7 are the two the owner's decision names, and they are
    /// the two the hardware needs: nothing of either mode happens without one of them. `$5201`
    /// and `$5202` arm it as well, for the reason the banner's last paragraph gives — a light
    /// board has nowhere to keep them, and "bank, scroll, then enable" is the order a game
    /// writes them in. `$5200` with bit 7 **clear** does not arm anything: its other bits only
    /// mean something while bit 7 is set, and the write that sets bit 7 carries them.
    pub(crate) const fn write_arms_substitution(address: u16, value: u8) -> bool {
        match address {
            0x5104 => value & 0x03 == 1,
            0x5200 => value & 0x80 != 0,
            0x5201 | 0x5202 => true,
            _ => false,
        }
    }

    /// Whether the block a **light** board has just loaded says that a mode is armed, i.e.
    /// whether the state was written by this board and the variant has to be restored with it.
    ///
    /// The MMC5's savestate block is the **same length in both variants** — which is what lets a
    /// state written before this lot, packs' idle savestates included, load unchanged — because
    /// the three registers ride in the spare bits of the `i32` that carries `$5104`'s two
    /// (see [`Mmc5Ext::packed_ex_ram_mode`]). A light board stores that `i32` whole, as
    /// `04731c86` does, so this test reads it off a board that has not switched yet: mode %01 in
    /// the low two bits, or any of the three registers above bit 7.
    pub(crate) const fn state_arms_substitution(base: &Mmc5) -> bool {
        (base.ex_ram_mode & 0x03) == 1 || (base.ex_ram_mode >> 8) != 0
    }

    /// The full board, built from the light one by the write that armed a mode.
    ///
    /// **`inline(never)`, and it matters.** `Board::intercept_cpu_write` is inlined into
    /// `Cartridge::cpu_write` and `NesBus::write_internal` — one body as far as every
    /// measurement of this project is concerned — so a 1.4 KiB temporary here would be 1.4 KiB
    /// of stack frame on **every** board's write path. Behind a call it is this function's own
    /// frame, entered once in the life of a game.
    #[inline(never)]
    pub(crate) fn armed_by_write(base: &Mmc5, address: u16, value: u8) -> Box<Self> {
        let mut board = Self::from_light(base.clone());
        // $5104's write has already been taken by the light board — it is a register both
        // variants have. The split's three are not, so the arming write is applied here.
        board.write_split_register(address, value);
        Box::new(board)
    }

    /// The full board, built from the light one that has just loaded a state this board wrote
    /// (see [`Mmc5Ext::state_arms_substitution`]). `inline(never)` for `armed_by_write`'s reason.
    #[inline(never)]
    pub(crate) fn armed_by_state(base: &Mmc5) -> Box<Self> {
        let mut board = Self::from_light(base.clone());
        let packed = board.base.ex_ram_mode;
        board.base.ex_ram_mode = packed & 0x03;
        board.unpack_split_registers(packed);
        Box::new(board)
    }

    /// The light board this one was built from, for the reset that puts the cartridge back on it
    /// (`Board::reset`): power-on and reset both start on the light variant, so a game that armed
    /// a mode and was then reset has to arm it again, exactly as it has to write every other
    /// register again.
    pub(crate) fn to_light(&self) -> Mmc5 {
        self.base.clone()
    }

    fn from_light(base: Mmc5) -> Self {
        Self {
            base,
            split_control: 0,
            split_scroll: 0,
            split_bank: 0,
            ex_attr_index: 0,
            split_tile: 0,
            split_row: 0,
            split_column: 0,
            chr_reads_since_nt: 0,
            split_in_region: false,
            split_capture: false,
        }
    }

    /// The per-tile state the two modes derive inside one line, put back where a power-on, a
    /// reset or a state load leaves it. The region flag starts **clear**, so neither a load nor a
    /// mid-frame switch can substitute a tile before the line that owns it has started.
    fn clear_derived_state(&mut self) {
        self.ex_attr_index = 0;
        self.split_tile = 0;
        self.split_row = 0;
        self.split_column = 0;
        self.chr_reads_since_nt = 0;
        self.split_in_region = false;
        self.split_capture = false;
    }

    /// `$5200`-`$5202`, the three registers the light board has nowhere to keep.
    fn write_split_register(&mut self, address: u16, value: u8) {
        match address {
            0x5200 => self.split_control = value,
            0x5201 => self.split_scroll = value,
            0x5202 => self.split_bank = value,
            _ => {}
        }
    }

    /// `$5104`'s two bits with the split's three registers in the twenty-four above them: what
    /// this board writes where the light one writes `ex_ram_mode`, so that **the block is the
    /// same length in both variants** and a state crosses the switch in either direction.
    ///
    /// `$5104` only ever holds 0 to 3 (`write_control` masks it), so bits 31-8 of that `i32` have
    /// been written as zero by every build there has ever been; a light board reads this value
    /// back whole and [`Mmc5Ext::state_arms_substitution`] then sees that the state is this
    /// board's. The alternative — three bytes appended, as MF-LEG-2 wrote them — makes the block
    /// three bytes longer than `04731c86`'s and no existing state loads.
    fn packed_ex_ram_mode(&self) -> i32 {
        self.base.ex_ram_mode
            | (i32::from(self.split_control) << 8)
            | (i32::from(self.split_scroll) << 16)
            | (i32::from(self.split_bank) << 24)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // The three low bytes.
    fn unpack_split_registers(&mut self, packed: i32) {
        self.split_control = (packed >> 8) as u8;
        self.split_scroll = (packed >> 16) as u8;
        self.split_bank = (packed >> 24) as u8;
    }

    // ----------------------------------------------------------------- the two modes

    /// Whether a **background** fetch has to consult `ExRAM` at all. MF-LEG-2 cached this in a
    /// field refreshed at every register write, because the light board's own fetch path read it;
    /// here it is computed, because only a game that armed a mode runs this board at all and the
    /// five refresh points were five chances to leave it stale.
    ///
    /// Both features read `ExRAM` through the PPU, which only the two nametable modes allow: in
    /// modes %10 and %11 the 1 KiB is the CPU's RAM and the PPU cannot see it. Mesen gates both
    /// on `_extendedRamMode <= 1` (`MMC5.h:480`) and FCEUX the split on `!(MMC5HackCHRMode & 2)`
    /// (`mmc5.cpp:181`), which is the same test.
    const fn background_substitutes(&self) -> bool {
        self.base.in_frame
            && self.base.ex_ram_mode <= 1
            && (self.base.ex_ram_mode == 1 || self.split_enabled())
    }

    /// `$5200` bit 7.
    const fn split_enabled(&self) -> bool {
        (self.split_control & 0x80) != 0
    }

    /// Whether the screen tile column `column` lies in the split region: `$5200` bit 6 picks the
    /// side and bits 4-0 the threshold.
    const fn split_covers(&self, column: u8) -> bool {
        let threshold = self.split_control & 0x1F;
        if (self.split_control & 0x40) != 0 {
            column >= threshold
        } else {
            column < threshold
        }
    }

    /// Reloads the split's row at the start of a line's fetch sequence, and puts the tile counter
    /// one before column 0. [`Mmc5Ext::see_ppu_read`] is the only caller, which is where that dot
    /// is recognised — see [`Mmc5Ext::chr_reads_since_nt`].
    ///
    /// The row is `$5201 + L` for visible line L, modulo 240 — the page's "resets to the vertical
    /// scrolling value in `$5201` each v-blank" plus one per line. `scanline_counter` is already
    /// that L: it is zeroed on the pre-render line's detection and incremented once per line
    /// afterwards (see `Mmc5::detect_scanline` and Q-121), and this runs at the dot where the PPU
    /// begins fetching the tiles of the **next** line, so the counter read here is that line's
    /// index — 0 on the pre-render line, whose fetches feed visible line 0.
    // Q-177
    fn reload_split_row(&mut self) {
        // 0x1F, so that the first counted nametable fetch pre-increments to column 0 — puNES
        // `mapper_005.c:396` and Nestopia `NstBoardMmc5.cpp:681` both reset to the same 0x1F for
        // the same reason.
        self.split_column = 0x1F;
        self.split_in_region = false;
        let row =
            u32::try_from(self.base.scanline_counter).unwrap_or(0) + u32::from(self.split_scroll);
        #[allow(clippy::cast_possible_truncation)] // 0..240.
        {
            self.split_row = (row % 240) as u8;
        }
    }

    /// One counted background nametable fetch: moves the split's tile counter on and decides
    /// whether this tile is in the region.
    fn advance_split_column(&mut self) {
        // Five bits, as the threshold field is: a line's 34th tile is column 33 and wraps to 1,
        // which is the hardware's own counter and what puNES and Nestopia do.
        self.split_column = (self.split_column + 1) & 0x1F;
        self.split_in_region = self.split_covers(self.split_column);
        if self.split_in_region {
            self.split_tile =
                ((u16::from(self.split_row) & 0xF8) << 2) | u16::from(self.split_column);
        }
    }

    /// The attribute byte of the split tile being fetched, taken from the attribute table of the
    /// `ExRAM` nametable and copied into all four quadrants so that the PPU's own shift picks it
    /// whatever the tile's position (Nestopia `GetSpliterAttribute`, puNES
    /// `mapper_005.c:479-480`, Mesen `MMC5.h:506-509`, all three the same expression).
    fn split_attribute(&self) -> u8 {
        let tile = self.split_tile;
        let index = 0x03C0 | ((tile >> 4) & 0x38) | ((tile >> 2) & 0x07);
        let shift = ((tile >> 4) & 0x04) | (tile & 0x02);
        let byte = self.base.ex_ram[usize::from(index)];
        ((byte >> shift) & 0x03) * 0x55
    }

    /// The `ExRAM` byte of the tile mode 1 is enhancing, `AACC CCCC`.
    fn ex_attr_byte(&self) -> u8 {
        self.base.ex_ram[usize::from(self.ex_attr_index)]
    }

    /// One PPU read, as the board's own bus brings it — `Mmc5::see_ppu_read` with the two modes'
    /// per-tile state latched in the middle of it.
    ///
    /// **The one body of this file that is a copy rather than a forward**, because the latching
    /// has to happen between two halves of the light board's own work: after the fetch that
    /// raises `in_frame` (the substitution is gated on it) and before the repeat counter consumes
    /// `last_ppu_read_addr` (the split's column is gated on that address being a new one). The
    /// copy is `04731c86`'s `Mmc5::see_ppu_read` line for line, and what keeps the two in step is
    /// a test rather than a comment:
    /// `mf_leg_3_the_full_variant_renders_like_the_light_one_with_neither_mode` runs the same
    /// frames through both boards and compares every pixel, which a drifted copy of the scanline
    /// detection cannot pass.
    ///
    /// `inline(never)` for the reason `Mmc5::see_ppu_read` carries it: this is what
    /// `Board::on_ppu_address_bus` calls on every PPU read, and an arm that is a call does not
    /// grow the dispatch's prologue.
    #[inline(never)]
    fn see_ppu_read(&mut self, address: u16) {
        // A nametable byte, as opposed to an attribute byte of the same $2xxx page: only this
        // fetch turns an armed detection into the flag, because it is the first fetch of the
        // line proper.
        let nametable_fetch = (0x2000..0x3000).contains(&address) && (address & 0x03FF) < 0x03C0;
        if nametable_fetch && self.base.need_in_frame {
            self.base.need_in_frame = false;
            self.base.in_frame = true;
            self.base.refresh_background_chr_set();
        }

        // The per-tile state of the two substituting modes, latched where the board already sees
        // every read.
        //
        // `address != last_ppu_read_addr` leaves out the **two discarded nametable fetches** of
        // dots 338 and 340, which repeat dot 337's address: they are not tiles, and counting them
        // would put the split's column three ahead by the end of a line. It is the same test the
        // repeat detection below makes.
        if self.background_substitutes() {
            if address < 0x2000 {
                // A pattern fetch: the run this counts is what finds the start of a line's fetch
                // sequence (see `Mmc5Ext::chr_reads_since_nt`).
                self.chr_reads_since_nt = self
                    .chr_reads_since_nt
                    .saturating_add(1)
                    .min(Self::CHR_READS_BEFORE_ROW_START);
            } else {
                if nametable_fetch && address != self.base.last_ppu_read_addr {
                    if self.chr_reads_since_nt >= Self::CHR_READS_BEFORE_ROW_START
                        && self.split_enabled()
                    {
                        self.reload_split_row();
                    }

                    // "Each byte of Expansion RAM is used to enhance the tile at the
                    // corresponding address in every nametable": the tile's ten-bit offset,
                    // which the attribute address that follows does not carry.
                    self.ex_attr_index = address & 0x03FF;
                    if self.split_enabled() {
                        self.advance_split_column();
                    }
                }

                self.chr_reads_since_nt = 0;
            }
        }

        if self.base.nt_read_counter >= Mmc5::NT_READS_TO_DETECT {
            self.base.detect_scanline();
        } else if (0x2000..0x3000).contains(&address) && self.base.last_ppu_read_addr == address {
            // "in the range $2xxx": an attribute address would count too, but the three in a row
            // only ever happen at dots 337, 338 and 340, which read the nametable byte.
            self.base.nt_read_counter += 1;
        }

        if self.base.last_ppu_read_addr != address {
            self.base.nt_read_counter = 0;
        }

        self.base.last_ppu_read_addr = address;
        self.base.ppu_idle_counter = Mmc5::PPU_IDLE_CYCLES;
    }

    /// The byte the board puts on the nametable data bus for `address`: the split's tile and
    /// attribute, mode 1's attribute, and otherwise the byte the PPU would have read on its own.
    ///
    /// `vram` is the console's, which the board needs because `$5105` may map a table to CIRAM
    /// page 0 or 1 while this board is answering every fetch — see
    /// [`Mapper::read_nametable_from_vram`].
    fn substituted_nametable_byte(
        &self,
        memory: &CartridgeMemory,
        address: u16,
        vram: &[u8],
    ) -> u8 {
        let attribute_fetch = (address & 0x03FF) >= 0x03C0;
        if self.split_enabled() && self.split_in_region {
            return if attribute_fetch {
                self.split_attribute()
            } else {
                self.base.ex_ram[usize::from(self.split_tile)]
            };
        }

        if self.base.ex_ram_mode == 1 && attribute_fetch {
            // "Attribute fetches are replaced by data from Expansion RAM": the two bits, in all
            // four quadrants so that any shift yields them.
            return (self.ex_attr_byte() >> 6) * 0x55;
        }

        self.plain_nametable_byte(memory, address, vram)
    }

    /// The nametable byte the PPU would have read for `address` had the board not been answering
    /// all four tables: `$5105`'s choice of CIRAM page 0, CIRAM page 1, `ExRAM` or fill mode.
    ///
    /// The CIRAM pages are the one source the board cannot reach without `vram`; the other two
    /// are `Mmc5::read_nametable`'s own body, **called rather than copied**.
    fn plain_nametable_byte(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        let page = self.base.nametable_mode(address);
        if page >= 2 {
            return self.base.read_nametable(memory, address);
        }

        // This reproduces `Ppu2C02::read_nametable_as`'s own
        // `self.vram[(page << 10) | (address & 0x03FF)]`: the MMC5 never answers
        // `FOLLOW_MIRRORING`, so there is no mirroring left to apply, and the two CIRAM modes of
        // `$5105` are pages 0 and 1 (`Mmc5Ext::nametable_page`).
        let index = (usize::try_from(page).unwrap_or(0) << 10) | usize::from(address & 0x03FF);
        vram.get(index).copied().unwrap_or(0)
    }

    /// The CHR offset of a background pattern fetch the split or mode 1 answers, or `None` when
    /// neither does and the light board's ordinary windows apply.
    fn substituted_chr_offset(&self, address: u16) -> Option<usize> {
        if self.split_enabled() && self.split_in_region {
            // "$5202: all eight bits select a 4 KB CHR bank", with the page's "and the lowest 3
            // address bits of the pattern data" substituted from the split's own row, which is
            // the region's fine vertical scroll (Mesen `MMC5.h:513`).
            //
            // The tile capture reads the tile's eight rows at once and so keeps the PPU's own
            // three bits: see [`Mmc5Ext::split_capture`].
            let bank = usize::from(self.split_bank);
            let within = if self.split_capture {
                usize::from(address) & 0x0FFF
            } else {
                (usize::from(address) & 0x0FF8) | usize::from(self.split_row & 0x07)
            };
            return Some((bank << 12) | within);
        }

        if self.base.ex_ram_mode == 1 {
            // "The pattern fetches ignore the standard CHR banking bits, and instead use the top
            // two bits of $5130 and the bottom 6 bits from Expansion RAM to choose a 4KB bank to
            // select the tile from."
            let bank = usize::from(self.ex_attr_byte() & 0x3F)
                | (usize::try_from(self.base.chr_upper).unwrap_or(0) << 6);
            return Some((bank << 12) | (usize::from(address) & 0x0FFF));
        }

        None
    }

    /// The light board's CHR offset, with the two modes' substitution ahead of it and only for a
    /// background fetch: "CHR mode always applies to sprite pattern fetches".
    fn chr_address(&self, address: u16, fetch: ChrFetch) -> usize {
        if fetch == ChrFetch::Background && self.background_substitutes() {
            if let Some(offset) = self.substituted_chr_offset(address) {
                return offset;
            }
        }

        self.base.chr_address(address, fetch)
    }

    /// `Mmc5::read_chr` with the offset above: the four lines that choose between CHR-ROM and
    /// CHR-RAM, which have no behaviour of their own to keep in step.
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

    // ------------------------------------------- the tile capture (REQ-CORE-037, the 3D view)
    //
    // The tile records the 3D conversion is built from are emitted by
    // `Ppu2C02::capture_background_row`, which walks the row **out of display order** at the end
    // of the line and reads each tile's nametable byte, its attribute byte and its two pattern
    // planes through the inert path. Both substituting modes are per tile, so the board's
    // per-tile state has to be positioned per captured column — and then put back, because the
    // tile whose nametable byte was fetched at dot 337 has its attribute and pattern fetches on
    // the *next* line and would otherwise find the capture's leftovers.
    //
    // That is exactly the save/walk/restore the capture already performs for the MMC2 and MMC4
    // latches (REQ-CORE-039, REQ-CORE-033: "the emulation is identical whether or not the host
    // registered a video callback"), so these three methods are the same shape as
    // `chr_latch_state` / `advance_chr_latch` / `set_chr_latch_state` without disturbing that
    // pair's own contract — this board's `chr_latch_state` is the light board's, the trait's 0.

    /// The per-tile state of the two substituting modes, packed so that the capture can hold it
    /// in one value. `u64` and not `i32`: the six fields are 34 bits.
    fn capture_state(&self) -> u64 {
        u64::from(self.ex_attr_index)
            | (u64::from(self.split_tile) << 10)
            | (u64::from(self.split_row) << 20)
            | (u64::from(self.split_column) << 28)
            | (u64::from(self.split_in_region) << 33)
            | (u64::from(self.split_capture) << 34)
    }

    /// Puts back what [`Mmc5Ext::capture_state`] took.
    #[allow(clippy::cast_possible_truncation)] // Each field is masked to its own width.
    fn restore_capture_state(&mut self, state: u64) {
        self.ex_attr_index = (state & 0x03FF) as u16;
        self.split_tile = ((state >> 10) & 0x03FF) as u16;
        self.split_row = ((state >> 20) & 0xFF) as u8;
        self.split_column = ((state >> 28) & 0x1F) as u8;
        self.split_in_region = (state >> 33) & 1 != 0;
        self.split_capture = (state >> 34) & 1 != 0;
    }

    /// Positions the per-tile state on the tile the capture is about to read: `address` is its
    /// nametable address, `column` its screen tile column and `line` the screen line its top row
    /// is drawn on.
    ///
    /// `column` may be negative or above 31 — the widescreen extension captures `extra` columns
    /// on each side — and the split region is a property of the 32 columns the PPU draws, so a
    /// column outside them is never in it.
    fn position_captured_tile(&mut self, address: u16, column: i32, line: i32) {
        self.ex_attr_index = address & 0x03FF;
        if !self.split_enabled() {
            return;
        }

        // The page's row for that line, as `Mmc5Ext::reload_split_row` computes it for a rendered
        // one. `rem_euclid`, because the row of the band the capture emits is the one drawn on its
        // **top** line, which is `line - 1` for the first band of a frame.
        let row = (line + i32::from(self.split_scroll)).rem_euclid(240);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..240.
        {
            self.split_row = row as u8;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..32.
        let column5 = (column & 0x1F) as u8;
        self.split_column = column5;
        self.split_in_region = (0..32).contains(&column) && self.split_covers(column5);
        self.split_capture = true;
        if self.split_in_region {
            self.split_tile = ((u16::from(self.split_row) & 0xF8) << 2) | u16::from(column5);
        }
    }
}

/// Thirty-eight of the forty-one methods below are a **forward** to [`Mmc5Ext::base`], so the
/// MMC5's PRG decode, its sound, its IRQ, its scanline detection, its bank windows and its
/// savestate format have exactly one implementation and this board cannot drift from the light
/// one. The three that are not — `ppu_peek`, `on_ppu_address_bus` and `read_nametable_from_vram`
/// — plus the four write and state methods are where the two modes live.
#[cfg(feature = "board-mmc5-exattr-split")]
impl Mapper for Mmc5Ext {
    fn mapper_number(&self) -> i32 {
        self.base.mapper_number()
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.base.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.base.banks_mut()
    }

    /// The light board's block with the split's three registers in the spare bits of `$5104`'s
    /// `i32` — [`Mmc5Ext::packed_ex_ram_mode`] says why there and not in three bytes of its own.
    ///
    /// The clone is this board's one indulgence: `save_registers` takes `&self` and the one
    /// function that knows the block's layout is the light board's, so the value it has to write
    /// is handed to it through a copy of the board. It costs 1.4 KiB of memcpy per savestate —
    /// a run-ahead frame or a rewind sample — against a second copy of a twenty-five-field
    /// format that a future lot would have to keep in step.
    fn save_registers(&self, writer: &mut StateWriter) {
        let mut light = self.base.clone();
        light.ex_ram_mode = self.packed_ex_ram_mode();
        light.save_registers(writer);
    }

    /// The light board's parser, then the three registers out of the `i32` it stored whole.
    ///
    /// A state written by the **light** variant has zeros above bit 7, so this unpacks the three
    /// registers to 0 — which is the light board's own state, since a write to any of them is
    /// what would have armed this board (see [`Mmc5Ext::write_arms_substitution`]). This board
    /// does **not** switch back for such a state: both modes off is a state it renders exactly as
    /// the light one does, which
    /// `mf_leg_3_the_full_variant_renders_like_the_light_one_with_neither_mode` pins.
    ///
    /// Reached only through `NesCore::restore_core_state` — rewind's and run-ahead's path, which
    /// loads into the cartridge already attached (REQ-LOOP-067). A **savestate** load builds a new
    /// cartridge, so it always starts on the light board and `Board::load_registers` promotes it
    /// from the block.
    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.base.load_registers(reader)?;
        let packed = self.base.ex_ram_mode;
        self.base.ex_ram_mode = packed & 0x03;
        self.unpack_split_registers(packed);
        // Derived within one line, as `Mmc5::load_registers` says of `nt_read_counter`.
        self.clear_derived_state();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.base.reset(memory);
        // Power-on: the split is off, so nothing of it is armed and `$5201`'s row counter starts
        // where a write will put it. `Board::reset` then puts the cartridge back on the light
        // variant, so these three are the state of a board that is about to stop existing.
        self.split_control = 0;
        self.split_scroll = 0;
        self.split_bank = 0;
        self.clear_derived_state();
    }

    fn irq_asserted(&self) -> bool {
        self.base.irq_asserted()
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        self.base.ppu_irq_horizon()
    }

    fn expansion_audio_output(&self) -> Option<i32> {
        self.base.expansion_audio_output()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.base.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.base.prg_ram_writable()
    }

    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        self.base.prg_ram_window(memory)
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        self.base.rom_windows(memory)
    }

    fn cpu_read_windows(&self, memory: &CartridgeMemory) -> CpuReadWindows {
        self.base.cpu_read_windows(memory)
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        self.base.try_read_prg(memory, address)
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.base.cpu_read_override(memory, address)
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.base.cpu_peek_override(memory, address)
    }

    /// `$5200`-`$5202`, and the light board's decode for everything else. These three are the
    /// only registers of the MMC5 that exist on this board alone, which is why a write to one of
    /// them is what brings the board into being.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if (0x5200..=0x5202).contains(&address) {
            self.write_split_register(address, value);
            return true;
        }

        self.base.intercept_cpu_write(memory, address, value)
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.base.write_register(memory, address, value);
    }

    fn chr_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.base.chr_view(memory)
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.base.chr_peek_view(memory)
    }

    /// The light board's CHR read with the two modes' substitution ahead of it.
    ///
    /// `inline(never)`: `Board::ppu_peek` and `Board::ppu_read` are the dispatches MF-LEG-2
    /// measured at **+4 528 000 instructions on `metroid`** — one per CHR fetch, on a board that
    /// has nothing to do with the MMC5 — when the MMC5's arm grew by a single `bool` test. An arm
    /// that is a load and a call cannot grow the prologue that way, and only a game that armed a
    /// mode ever takes it.
    #[inline(never)]
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

    fn chr_latch_state(&self) -> i32 {
        self.base.chr_latch_state()
    }

    fn set_chr_latch_state(&mut self, state: i32) {
        self.base.set_chr_latch_state(state);
    }

    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        self.base.advance_chr_latch(pattern_table, tile);
    }

    /// Every PPU read — [`Mmc5Ext::see_ppu_read`], which is the light board's body with the two
    /// modes' per-tile state latched inside it.
    // Q-116
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        self.see_ppu_read(address);
    }

    fn on_ppu_control(&mut self, control: u8) {
        self.base.on_ppu_control(control);
    }

    fn on_ppu_scanline(&mut self, memory: &mut CartridgeMemory, scanline: i32, rendering: bool) {
        self.base.on_ppu_scanline(memory, scanline, rendering);
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        self.base.on_cpu_cycle(memory);
    }

    /// Where `$5105` puts logical table `table`, and **`SERVED_BY_CARTRIDGE` for all four as soon
    /// as either mode is armed**, so that every nametable-space fetch reaches
    /// [`Mapper::read_nametable_from_vram`] — which is how the attribute override and the split's
    /// tiles get onto the bus without a single test being added to any other board's fetch path
    /// (lot MF-LEG-2).
    ///
    /// The condition is the two **register** bits alone, never `in_frame`:
    /// `Cartridge::nametable_pages` is a cache refreshed by a CPU write to the cartridge, and a
    /// condition that moved inside a frame would leave it stale. The in-frame half of the test is
    /// made where the byte is answered. The switch itself refreshes the cache: it happens inside
    /// `Board::intercept_cpu_write`, and `Cartridge::cpu_write` calls `refresh_rom_windows` —
    /// which refreshes the pages of a board whose tables vary, mapper 5 among them — on every
    /// write the board takes.
    fn nametable_page(&self, table: i32) -> i32 {
        if self.base.ex_ram_mode == 1 || self.split_enabled() {
            return -2;
        }

        self.base.nametable_page(table)
    }

    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        self.base.read_nametable(memory, address)
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.base.write_nametable(memory, address, value);
    }

    /// Every nametable-space byte the PPU fetches while either mode is armed: the split's tile
    /// and attribute, mode 1's per-tile attribute, and otherwise what the PPU would have read on
    /// its own, `$5105`'s CIRAM pages included — which is why `vram` is here (lot MF-LEG-2).
    ///
    /// `inline(never)` for `ppu_peek`'s reason: `Board::read_nametable_from_vram` is on every
    /// nametable fetch of every board.
    #[inline(never)]
    fn read_nametable_from_vram(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        if self.background_substitutes() {
            self.substituted_nametable_byte(memory, address, vram)
        } else {
            // Outside rendering nothing is substituted: the page's two modes are about what the
            // chip does to the PPU's *fetches*, and a `$2007` access is "treated as sprite
            // accesses" or as a plain nametable read. Mesen's gate is the same `_ppuInFrame`
            // (`MMC5.h:480`).
            self.plain_nametable_byte(memory, address, vram)
        }
    }

    /// Where a **write** to nametable-space `address` lands in console VRAM while the board is
    /// answering all four tables, or `None` when the board keeps the byte itself (`ExRAM`) or
    /// drops it (fill mode).
    ///
    /// Only the two substituting modes make this necessary: without them a table the board
    /// answers is never a CIRAM table, so `Ppu2C02::write_nametable_byte`'s
    /// `SERVED_BY_CARTRIDGE` arm is right as it stands — which is why the light board leaves the
    /// trait's `None`. With them, a `$2007` write to a CIRAM table must still reach CIRAM, a
    /// detail no cross-checked emulator has to think about, their PPUs holding one pointer per
    /// table rather than a page number.
    fn nametable_write_offset(&self, address: u16) -> Option<usize> {
        let page = self.base.nametable_mode(address);
        if page >= 2 {
            return None;
        }

        Some((usize::try_from(page).unwrap_or(0) << 10) | usize::from(address & 0x03FF))
    }

    fn substitutes_background_tiles(&self) -> bool {
        // The two **register** bits, as `Mmc5Ext::nametable_page` reads them: the capture asks
        // once per row, before the loop, and `in_frame` is settled by then anyway.
        self.base.ex_ram_mode == 1 || self.split_enabled()
    }

    fn background_capture_state(&self) -> u64 {
        self.capture_state()
    }

    fn set_background_capture_state(&mut self, state: u64) {
        self.restore_capture_state(state);
    }

    fn position_captured_background_tile(&mut self, address: u16, column: i32, line: i32) {
        self.position_captured_tile(address, column, line);
    }
}
