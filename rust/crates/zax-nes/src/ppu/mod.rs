//! The 2C02 PPU (port of `src/Zax.Nes/Ppu`).
//!
//! Frame timing, registers, VRAM, OAM, palette, rendering and the 3dSen tile contract
//! (REQ-CORE-015, REQ-CORE-020, SPEC-02 §3.5).
//!
//! Written from the public nesdev documentation ("PPU rendering", "PPU frame timing", "PPU
//! scrolling", "PPU sprite evaluation", "PPU registers", "PPU palettes"). The type is split
//! over three files: this one carries the registers, the PPU bus and the dot clock;
//! `render.rs` carries the rendering pipeline and the 256 x 240 indexed frame; `tiles.rs`
//! carries the tile-record emission of SPEC-02 §3.5 and the colour composition the Zapper
//! needs.
//!
//! **Open bus.** The PPU has its own data-bus latch, separate from the CPU's (nesdev, "PPU
//! registers"; `ppu_open_bus`). Writing any register loads it; reads of the write-only
//! registers return it without refreshing it, and a bit that has not been refreshed for
//! about 600 ms decays to 0.
//!
//! **Cartridge.** The bus owns the cartridge, so every PPU call that may reach it takes it
//! as an `Option<&mut Cartridge>` instead of holding a reference.

mod ppu_bus_access;
mod render;
mod state;
mod tile_buffer;
mod tiles;

pub use ppu_bus_access::PpuBusAccess;
pub use tile_buffer::TileBuffer;

use crate::cartridges::{Cartridge, ChrFetch};
use crate::{NametableMirroring, RegionTiming, TileRecord};

/// The cartridge as the internal PPU paths pass it around.
type Cart<'a, 'c> = &'a mut Option<&'c mut Cartridge>;

/// Size of nametable VRAM: 4 KiB, enough for four-screen mode (REQ-CORE-090).
pub const VRAM_SIZE: usize = 4096;

/// Size of OAM.
pub const OAM_SIZE: usize = 256;

/// Size of palette RAM, in bytes (REQ-CORE-045).
pub const PALETTE_SIZE: usize = 32;

/// Upper bound on the bytes the tile records of one frame occupy in a savestate, so that
/// the host can announce a state size before serialising (REQ-ARCH-013).
pub const MAX_TILE_RECORD_STATE_SIZE: usize = 12 + TileBuffer::CAPACITY * 26;

/// Width of the optional 2D image, in pixels (REQ-CORE-048).
pub const SCREEN_WIDTH: usize = 256;

/// Height of the optional 2D image, in pixels (REQ-CORE-048).
pub const SCREEN_HEIGHT: usize = 240;

/// Number of sprites the hardware can show on one line (nesdev).
pub const SPRITES_PER_LINE: usize = 8;

/// PPU dots after which an unrefreshed bit of the data-bus latch decays to 0. About 600 ms
/// at the NTSC dot rate (nesdev; `ppu_open_bus` requires the decay to have happened within
/// one second).
const DECAY_DOTS: i64 = 3_200_000;

/// PPU dots between the /NMI sampling point and the end of the CPU cycle (see
/// [`Ppu2C02::nmi_line_sample`]).
const NMI_SAMPLE_LEAD: i32 = 2;

/// Dot of the pre-render line at which PPUMASK decides whether the idle dot (0, 0) of the
/// next frame is skipped (REQ-CORE-015).
///
/// nesdev describes the effect ("with rendering enabled, each odd PPU frame is one PPU
/// clock shorter: the first idle tick of the first visible scanline is skipped") but not
/// the dot at which the decision is latched. `ppu_vbl_nmi` test 10 measures it to one PPU
/// clock by enabling and disabling background rendering around it, and it only passes with
/// the mask read two dots before the end of the pre-render line.
const SKIP_DECISION_DOT: i32 = 338;

const VISIBLE_SCANLINES: i32 = RegionTiming::VISIBLE_SCANLINES;
const SCREEN_WIDTH_I32: i32 = 256;
const SPRITES_PER_LINE_I32: i32 = 8;
const DOTS_PER_SCANLINE: i32 = RegionTiming::DOTS_PER_SCANLINE;

/// The picture processing unit.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // The chip's flags, one field each like the C#.
pub struct Ppu2C02 {
    vram: [u8; VRAM_SIZE],
    oam: [u8; OAM_SIZE],
    palette: [u8; PALETTE_SIZE],
    decay_stamp: [i64; 8],

    timing: RegionTiming,

    dot: i32,
    scanline: i32,
    dot_accumulator: i32,
    odd_frame: bool,
    ppu_clock: i64,

    control: u8,
    mask: u8,
    vblank_flag: bool,
    sprite_zero_hit: bool,
    sprite_overflow: bool,
    suppress_vblank: bool,
    oam_address: u8,

    v: u16,
    t: u16,

    /// Shadow of `v` that ignores the coarse-X increments of the tile fetch pipeline, so
    /// that it always holds the scroll of the *left edge* of the next line to be drawn. The
    /// tile capture of REQ-CORE-037 reads it; `v` itself is already two tiles ahead by the
    /// time a visible line starts, because dots 321 to 336 of the previous line prefetch its
    /// first two tiles.
    ///
    /// Q-004: REQ-CORE-038 asks for "cx, cy, nametable" without saying at which instant to
    /// read them, and the raw register would place every background row 16 pixels to the
    /// left. It is refreshed wherever the hardware reloads the address — the horizontal
    /// reload at dot 257, the vertical reload at dots 280 to 304 of the pre-render line, the
    /// second $2006 write and the $2007 auto-increment — but never by a coarse-X increment.
    ///
    /// AMD-021 R2-005: the row of line L reflects the state *after* every CPU write made
    /// during line L, so the two reloads the hardware performs at the end of the line — the
    /// vertical increment at dot 256 and the horizontal reload at dot 257 — must not reach
    /// it. They land in `v_row_next` instead, which becomes this value once the line is
    /// over.
    v_row_start: u16,

    /// Row-start shadow the *next* line will use: where the end-of-line reloads of the
    /// hardware go (AMD-021 R2-005).
    v_row_next: u16,

    fine_x: u8,
    write_toggle: bool,
    read_buffer: u8,
    decay: u8,
    skip_latch: bool,
    vblank_just_started: bool,
    nmi_line_sample: bool,

    // Rendering state (render.rs).
    pixels: Vec<u8>,
    secondary_oam: [u8; 4 * SPRITES_PER_LINE],
    sprite_pattern_low: [u8; SPRITES_PER_LINE],
    sprite_pattern_high: [u8; SPRITES_PER_LINE],
    sprite_attribute: [u8; SPRITES_PER_LINE],
    sprite_x: [u8; SPRITES_PER_LINE],
    nametable_latch: u8,
    attribute_latch: u8,
    pattern_low_latch: u8,
    pattern_high_latch: u8,
    pattern_shift_low: u16,
    pattern_shift_high: u16,
    attribute_shift_low: u16,
    attribute_shift_high: u16,
    oam_eval_latch: u8,
    eval_sprite: i32,
    eval_byte: i32,
    eval_found: i32,
    overflow_copy: i32,
    eval_finished: bool,
    sprite_zero_found: bool,
    sprites_loaded: i32,
    sprite_zero_loaded: bool,

    // Tile capture state (tiles.rs).
    tiles: TileBuffer,
    sprite_order: [usize; 64],
    mask_x: i32,
    mask_y: i32,
}

impl Default for Ppu2C02 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu2C02 {
    /// Creates a PPU; the core calls [`Ppu2C02::reset`] before running it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            palette: [0; PALETTE_SIZE],
            decay_stamp: [0; 8],
            timing: RegionTiming::default(),
            dot: 0,
            scanline: 0,
            dot_accumulator: 0,
            odd_frame: false,
            ppu_clock: 0,
            control: 0,
            mask: 0,
            vblank_flag: false,
            sprite_zero_hit: false,
            sprite_overflow: false,
            suppress_vblank: false,
            oam_address: 0,
            v: 0,
            t: 0,
            v_row_start: 0,
            v_row_next: 0,
            fine_x: 0,
            write_toggle: false,
            read_buffer: 0,
            decay: 0,
            skip_latch: false,
            vblank_just_started: false,
            nmi_line_sample: false,
            pixels: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
            secondary_oam: [0; 4 * SPRITES_PER_LINE],
            sprite_pattern_low: [0; SPRITES_PER_LINE],
            sprite_pattern_high: [0; SPRITES_PER_LINE],
            sprite_attribute: [0; SPRITES_PER_LINE],
            sprite_x: [0; SPRITES_PER_LINE],
            nametable_latch: 0,
            attribute_latch: 0,
            pattern_low_latch: 0,
            pattern_high_latch: 0,
            pattern_shift_low: 0,
            pattern_shift_high: 0,
            attribute_shift_low: 0,
            attribute_shift_high: 0,
            oam_eval_latch: 0,
            eval_sprite: 0,
            eval_byte: 0,
            eval_found: 0,
            overflow_copy: 0,
            eval_finished: false,
            sprite_zero_found: false,
            sprites_loaded: 0,
            sprite_zero_loaded: false,
            tiles: TileBuffer::new(),
            sprite_order: [0; 64],
            mask_x: 0,
            mask_y: 0,
        }
    }

    /// Timings in force.
    #[must_use]
    pub const fn timing(&self) -> RegionTiming {
        self.timing
    }

    /// Changes the timings; takes effect on the next frame (REQ-CORE-016).
    pub fn set_timing(&mut self, timing: RegionTiming) {
        self.timing = timing;
    }

    /// Current dot inside the scanline, 0 to 340.
    #[must_use]
    pub const fn dot(&self) -> i32 {
        self.dot
    }

    /// Current scanline; 0 to 239 are visible, the last one is the pre-render line.
    #[must_use]
    pub const fn scanline(&self) -> i32 {
        self.scanline
    }

    /// Index of the pre-render line of the region in force.
    #[must_use]
    pub const fn pre_render_scanline(&self) -> i32 {
        self.timing.scanlines_per_frame - 1
    }

    /// The `VBlank` flag, bit 7 of PPUSTATUS.
    #[must_use]
    pub const fn vblank_flag(&self) -> bool {
        self.vblank_flag
    }

    /// Sprite 0 hit, bit 6 of PPUSTATUS (REQ-CORE-112).
    #[must_use]
    pub const fn sprite_zero_hit(&self) -> bool {
        self.sprite_zero_hit
    }

    /// Sprite overflow, bit 5 of PPUSTATUS (REQ-CORE-116).
    #[must_use]
    pub const fn sprite_overflow(&self) -> bool {
        self.sprite_overflow
    }

    /// Set for one step when the frame boundary is crossed; "run one frame" stops on it
    /// (REQ-CORE-020).
    ///
    /// It marks the start of vblank whether or not the `VBlank` *flag* was actually raised:
    /// a read of $2002 one dot earlier suppresses the flag (nesdev, `vbl_nmi_timing` test 2)
    /// but must not stop the host loop from seeing the frame boundary.
    #[must_use]
    pub const fn vblank_just_started(&self) -> bool {
        self.vblank_just_started
    }

    /// Whether /NMI is being pulled low right now: `VBlank` flag set and NMI enabled.
    #[must_use]
    pub const fn nmi_asserted(&self) -> bool {
        self.vblank_flag && (self.control & 0x80) != 0
    }

    /// The /NMI line as the CPU sees it: sampled near the *start* of the CPU cycle, two PPU
    /// dots before the register access of that cycle takes effect.
    ///
    /// The 6502 latches /NMI during the cycle, not at its end, so a write that disables NMI
    /// or a read of $2002 that clears the `VBlank` flag cannot undo an assertion that
    /// happened two dots or more earlier. That offset is what the one-PPU-clock resolution
    /// of `ppu_vbl_nmi` tests 5 to 8 and `vbl_nmi_timing` tests 5 to 7 measures
    /// (REQ-CORE-116).
    #[must_use]
    pub const fn nmi_line_sample(&self) -> bool {
        self.nmi_line_sample
    }

    /// PPUCTRL ($2000).
    #[must_use]
    pub const fn control(&self) -> u8 {
        self.control
    }

    /// PPUMASK ($2001).
    #[must_use]
    pub const fn mask(&self) -> u8 {
        self.mask
    }

    /// Whether background rendering is on (bit 3 of PPUMASK, REQ-CORE-037).
    #[must_use]
    pub const fn background_enabled(&self) -> bool {
        (self.mask & 0x08) != 0
    }

    /// Whether sprite rendering is on (bit 4 of PPUMASK, REQ-CORE-041).
    #[must_use]
    pub const fn sprites_enabled(&self) -> bool {
        (self.mask & 0x10) != 0
    }

    /// Whether either layer is being rendered, which gates the odd-frame dot skip.
    #[must_use]
    pub const fn rendering_enabled(&self) -> bool {
        (self.mask & 0x18) != 0
    }

    /// Whether the leftmost column is masked for background or sprites (bits 1 and 2 of
    /// PPUMASK), which forces maskX to at least 8 (REQ-CORE-046).
    #[must_use]
    pub const fn left_column_masked(&self) -> bool {
        (self.mask & 0x02) == 0 || (self.mask & 0x04) == 0
    }

    /// The nametable layout `cartridge` currently selects; horizontal without one.
    #[must_use]
    pub fn mirroring_of(cartridge: Option<&Cartridge>) -> NametableMirroring {
        cartridge.map_or(NametableMirroring::Horizontal, Cartridge::mirroring)
    }

    /// Object attribute memory.
    #[must_use]
    pub const fn oam(&self) -> &[u8; OAM_SIZE] {
        &self.oam
    }

    /// Nametable VRAM, 4 KiB.
    #[must_use]
    pub const fn vram(&self) -> &[u8; VRAM_SIZE] {
        &self.vram
    }

    /// OAM address register ($2003).
    #[must_use]
    pub const fn oam_address(&self) -> u8 {
        self.oam_address
    }

    /// Coarse X of the internal scroll register (SPEC-02 §2).
    #[must_use]
    pub const fn coarse_x(&self) -> i32 {
        (self.v & 0x1F) as i32
    }

    /// Coarse Y of the internal scroll register.
    #[must_use]
    pub const fn coarse_y(&self) -> i32 {
        ((self.v >> 5) & 0x1F) as i32
    }

    /// Nametable selection bits of the internal scroll register.
    #[must_use]
    pub const fn nametable_select(&self) -> i32 {
        ((self.v >> 10) & 0x03) as i32
    }

    /// Fine Y scroll of the internal scroll register.
    #[must_use]
    pub const fn fine_y(&self) -> i32 {
        ((self.v >> 12) & 0x07) as i32
    }

    /// Fine X scroll (the "x" register of nesdev).
    #[must_use]
    pub const fn fine_x(&self) -> i32 {
        self.fine_x as i32
    }

    /// Returns the 32 palette bytes of the batch, mirrors filled in (REQ-CORE-045).
    #[must_use]
    pub const fn snapshot_palette(&self) -> [u8; PALETTE_SIZE] {
        let mut out = self.palette;

        // $3F10, $3F14, $3F18 and $3F1C mirror $3F00, $3F04, $3F08 and $3F0C (nesdev).
        out[0x10] = self.palette[0x00];
        out[0x14] = self.palette[0x04];
        out[0x18] = self.palette[0x08];
        out[0x1C] = self.palette[0x0C];
        out
    }

    /// Returns the machine to its power-on state: nametable RAM, OAM and palette RAM
    /// cleared (REQ-CORE-004). The palette starts at 0, which is why the public
    /// `power_up_palette` test is excluded (REQ-CORE-112).
    pub fn reset(&mut self) {
        self.vram.fill(0);
        self.oam.fill(0);
        self.palette.fill(0);
        self.decay_stamp.fill(0);
        self.pixels.fill(0);

        self.dot = 0;
        self.dot_accumulator = 0;
        self.odd_frame = false;
        self.ppu_clock = 0;
        self.control = 0;
        self.mask = 0;
        self.vblank_flag = false;
        self.sprite_zero_hit = false;
        self.sprite_overflow = false;
        self.suppress_vblank = false;
        self.oam_address = 0;
        self.v = 0;
        self.t = 0;
        self.v_row_start = 0;
        self.v_row_next = 0;
        self.fine_x = 0;
        self.write_toggle = false;
        self.read_buffer = 0;
        self.decay = 0;
        self.skip_latch = false;
        self.vblank_just_started = false;
        self.nmi_line_sample = false;

        self.reset_render_state();
        self.reset_tile_state();

        // Just after a load or a reset the first frame starts at the beginning of vblank,
        // so the reset lands there (REQ-CORE-020, notes).
        self.scanline = self.timing.vblank_scanline;
        self.dot = 1;
    }

    /// Advances the PPU by the dots of one CPU cycle of the current region.
    pub fn step_cpu_cycle(&mut self, mut cartridge: Option<&mut Cartridge>) {
        let accumulator = self.dot_accumulator + self.timing.dots_per_cycle_numerator;
        let dots = accumulator / self.timing.dots_per_cycle_denominator;
        self.dot_accumulator = accumulator - (dots * self.timing.dots_per_cycle_denominator);

        let sample_after = (dots - NMI_SAMPLE_LEAD).max(0);
        for dot in 0..dots {
            if dot == sample_after {
                self.nmi_line_sample = self.nmi_asserted();
            }

            self.step_dot(&mut cartridge);
        }
    }

    /// Clears the one-step frame boundary flag; the core calls this each frame.
    pub fn clear_frame_boundary(&mut self) {
        self.vblank_just_started = false;
    }

    fn step_dot(&mut self, cart: Cart<'_, '_>) {
        self.advance_counters();
        self.process_dot(cart);
    }

    fn advance_counters(&mut self) {
        self.ppu_clock += 1;
        self.dot += 1;

        if self.dot < DOTS_PER_SCANLINE {
            return;
        }

        self.dot = 0;
        self.scanline += 1;

        if self.scanline < self.timing.scanlines_per_frame {
            return;
        }

        self.scanline = 0;

        if self.odd_frame && self.timing.skips_dot_on_odd_frames && self.skip_latch {
            // With rendering on, the idle dot (0, 0) is skipped on odd frames, which makes
            // the frame one dot shorter (NTSC only, REQ-CORE-015). The mask is read here,
            // at the dot that is dropped, and not one dot earlier: that single dot is what
            // ppu_vbl_nmi test 10 measures.
            self.dot = 1;
        }

        self.odd_frame = !self.odd_frame;
    }

    fn process_dot(&mut self, cart: Cart<'_, '_>) {
        if self.scanline == self.pre_render_scanline() && self.dot == SKIP_DECISION_DOT {
            self.skip_latch = self.rendering_enabled();
        }

        if self.dot == 1 {
            if self.scanline == self.timing.vblank_scanline {
                // The frame boundary always fires; the flag itself can be suppressed by a
                // read of $2002 one dot earlier (nesdev, "PPU frame timing").
                self.vblank_just_started = true;
                if !self.suppress_vblank {
                    self.vblank_flag = true;
                }

                self.suppress_vblank = false;
            } else if self.scanline == self.pre_render_scanline() {
                self.vblank_flag = false;
                self.sprite_zero_hit = false;
                self.sprite_overflow = false;
                self.suppress_vblank = false;
                self.begin_rendered_frame();
            }

            // Boards that count rendered lines rather than A12 rises (MMC5) are told where
            // the scan is.
            let rendering = self.rendering_enabled();
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.on_ppu_scanline(self.scanline, rendering);
            }
        }

        // The sprite captures of SPEC-02 §3.5 run on the first dot of the visible line that
        // is always reached: the idle dot 0 of line 0 is dropped on a skipped odd frame.
        if self.scanline < VISIBLE_SCANLINES && self.dot == 1 {
            self.begin_capture_line(self.scanline, cart);
        }

        self.run_render_dot(cart);

        // The background row of line L is captured once the line is over, so that it shows
        // the state left by every CPU write made during the line (R2-005). The two
        // end-of-line reloads of the hardware are kept out of it by v_row_next, which only
        // becomes the row start of the line that follows.
        if self.dot == DOTS_PER_SCANLINE - 1 {
            if self.scanline < VISIBLE_SCANLINES {
                self.end_capture_line(self.scanline, cart);
            }

            self.v_row_start = self.v_row_next;
        }
    }

    // ---------------------------------------------------------------- registers

    /// Reads register `register` (0-7, $2000 + register) with its side effects.
    pub fn read(&mut self, register: u16, mut cartridge: Option<&mut Cartridge>) -> u8 {
        self.apply_decay();

        match register & 0x07 {
            2 => {
                // Bits 0 to 4 of PPUSTATUS come from the PPU's own bus latch and are not
                // refreshed by the read; bits 5 to 7 are the flags and do refresh it
                // (ppu_open_bus tests 6 and 7).
                let mut status = self.decay & 0x1F;
                if self.vblank_flag {
                    status |= 0x80;
                }

                if self.sprite_zero_hit {
                    status |= 0x40;
                }

                if self.sprite_overflow {
                    status |= 0x20;
                }

                self.refresh_decay(status, 0xE0);

                // Reading the flag one dot before it would be raised suppresses it for the
                // whole frame (nesdev; vbl_nmi_timing test 2, case 8).
                if self.scanline == self.timing.vblank_scanline && self.dot == 0 {
                    self.suppress_vblank = true;
                }

                self.vblank_flag = false;
                self.write_toggle = false;
                status
            }

            4 => {
                let value = self.read_oam();
                self.refresh_decay(value, 0xFF);
                value
            }

            7 => {
                let address = self.v & 0x3FFF;
                let value;
                if is_palette_address(address) {
                    // Palette reads are immediate and only drive six bits; the buffer still
                    // takes the mirrored nametable byte "underneath" the palette (nesdev).
                    value = (self.palette[palette_index(address)] & 0x3F) | (self.decay & 0xC0);
                    self.refresh_decay(value, 0x3F);
                    self.read_buffer = self.read_vram_inner(
                        address & 0x2FFF,
                        PpuBusAccess::Cpu,
                        ChrFetch::Background,
                        &mut cartridge,
                    );
                } else {
                    value = self.read_buffer;
                    self.refresh_decay(value, 0xFF);
                    self.read_buffer = self.read_vram_inner(
                        address,
                        PpuBusAccess::Cpu,
                        ChrFetch::Background,
                        &mut cartridge,
                    );
                }

                self.increment_vram_address(&mut cartridge);
                value
            }

            // $2000, $2001, $2003, $2005 and $2006 are write-only: the read returns the
            // PPU's bus latch and does not refresh it (ppu_open_bus tests 2 and 5).
            _ => self.decay,
        }
    }

    /// Reads a register without any side effect, for the host memory accessor
    /// (REQ-CORE-081).
    #[must_use]
    pub fn peek(&self, register: u16) -> u8 {
        match register & 0x07 {
            2 => {
                (self.decay & 0x1F)
                    | if self.vblank_flag { 0x80 } else { 0x00 }
                    | if self.sprite_zero_hit { 0x40 } else { 0x00 }
                    | if self.sprite_overflow { 0x20 } else { 0x00 }
            }
            4 => self.oam[usize::from(self.oam_address)],
            7 => {
                let address = self.v & 0x3FFF;
                if is_palette_address(address) {
                    self.palette[palette_index(address)]
                } else {
                    self.read_buffer
                }
            }
            _ => self.decay,
        }
    }

    /// Writes register `register` (0-7, $2000 + register).
    pub fn write(&mut self, register: u16, value: u8, mut cartridge: Option<&mut Cartridge>) {
        self.apply_decay();

        // A write to any PPU register, $2002 included, loads the whole bus latch
        // (ppu_open_bus test 2).
        self.refresh_decay(value, 0xFF);

        match register & 0x07 {
            0 => {
                self.control = value;
                self.t = (self.t & 0xF3FF) | (u16::from(value & 0x03) << 10);
                if let Some(cartridge) = cartridge.as_deref_mut() {
                    cartridge.on_ppu_control(value);
                }
            }

            1 => self.mask = value,

            3 => self.oam_address = value,

            4 => self.write_oam(value),

            5 => {
                if self.write_toggle {
                    self.t = (self.t & 0x8FFF) | (u16::from(value & 0x07) << 12);
                    self.t = (self.t & 0xFC1F) | (u16::from(value & 0xF8) << 2);
                } else {
                    self.t = (self.t & 0xFFE0) | u16::from(value >> 3);
                    self.fine_x = value & 0x07;
                }

                self.write_toggle = !self.write_toggle;
            }

            6 => {
                if self.write_toggle {
                    self.t = (self.t & 0xFF00) | u16::from(value);
                    self.v = self.t;

                    // A CPU write reaches the row of the line it is made in (R2-005).
                    self.v_row_start = self.v;
                    self.v_row_next = self.v;

                    // The new address reaches the cartridge: this is how a game clocks an
                    // MMC3 counter by hand, and what mmc3_test measures (REQ-CORE-114).
                    self.notify_address_bus(self.v & 0x3FFF, &mut cartridge);
                } else {
                    self.t = (self.t & 0x00FF) | (u16::from(value & 0x3F) << 8);
                }

                self.write_toggle = !self.write_toggle;
            }

            7 => {
                self.write_vram_inner(self.v & 0x3FFF, value, PpuBusAccess::Cpu, &mut cartridge);
                self.increment_vram_address(&mut cartridge);
            }

            // $2002 is read-only; the write only loaded the bus latch.
            _ => {}
        }
    }

    /// Writes one OAM byte, used by OAM DMA ($4014).
    pub fn write_oam_dma(&mut self, value: u8) {
        self.write_oam(value);
    }

    /// Reads OAM through $2004.
    ///
    /// Bits 2 to 4 of a sprite's attribute byte do not exist in the chip, so the masked
    /// value is what was stored (see `write_oam`). During the 64 dots in which the secondary
    /// OAM is cleared, reads return $FF (nesdev, "PPU sprite evaluation").
    fn read_oam(&self) -> u8 {
        if self.rendering_enabled()
            && self.scanline < VISIBLE_SCANLINES
            && (1..=64).contains(&self.dot)
        {
            return 0xFF;
        }

        self.oam[usize::from(self.oam_address)]
    }

    /// Writes OAM at the current address and post-increments it.
    ///
    /// The three unimplemented bits of a sprite's byte 2 are dropped, so they read back as 0
    /// (nesdev, "PPU OAM"; `oam_read` and `ppu_open_bus` test 10).
    fn write_oam(&mut self, value: u8) {
        self.oam[usize::from(self.oam_address)] = if (self.oam_address & 0x03) == 0x02 {
            value & 0xE3
        } else {
            value
        };
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    fn increment_vram_address(&mut self, cart: Cart<'_, '_>) {
        let step = if (self.control & 0x04) != 0 { 32 } else { 1 };
        self.v = self.v.wrapping_add(step) & 0x7FFF;

        // A CPU write reaches the row of the line it is made in (R2-005).
        self.v_row_start = self.v;
        self.v_row_next = self.v;

        // The incremented address goes back on the PPU bus, so a $2007 access that steps
        // across $1000 clocks an MMC3 counter — which is what mmc3_irq_tests "A12 clocking"
        // measures (tests 5 and 6).
        self.notify_address_bus(self.v & 0x3FFF, cart);
    }

    // ---------------------------------------------------------------- bus latch decay

    fn apply_decay(&mut self) {
        for bit in 0..8 {
            if self.ppu_clock - self.decay_stamp[bit] > DECAY_DOTS {
                self.decay &= !(1u8 << bit);
            }
        }
    }

    fn refresh_decay(&mut self, value: u8, mask: u8) {
        self.decay = (self.decay & !mask) | (value & mask);

        for bit in 0..8 {
            if (mask & (1u8 << bit)) != 0 {
                self.decay_stamp[bit] = self.ppu_clock;
            }
        }
    }

    // ---------------------------------------------------------------- PPU memory

    /// Reads the PPU bus at `address` (masked to 14 bits).
    ///
    /// `access` says why the access happens: [`PpuBusAccess::Render`] and
    /// [`PpuBusAccess::Cpu`] put the address on the cartridge's bus and may have mapper side
    /// effects; the tile emitter passes [`PpuBusAccess::Inert`] (REQ-CORE-033). `fetch` is
    /// the pattern fetch the access belongs to (MMC5 bank sets).
    pub fn read_vram_through_bus(
        &mut self,
        address: u16,
        access: PpuBusAccess,
        fetch: ChrFetch,
        mut cartridge: Option<&mut Cartridge>,
    ) -> u8 {
        self.read_vram_inner(address, access, fetch, &mut cartridge)
    }

    fn read_vram_inner(
        &self,
        address: u16,
        access: PpuBusAccess,
        fetch: ChrFetch,
        cart: Cart<'_, '_>,
    ) -> u8 {
        let address = address & 0x3FFF;

        if access != PpuBusAccess::Inert {
            self.notify_address_bus(address, cart);
        }

        if address < 0x2000 {
            return match cart.as_deref_mut() {
                None => 0,
                Some(cartridge) if access == PpuBusAccess::Inert => {
                    cartridge.ppu_peek(address, fetch)
                }
                Some(cartridge) => cartridge.ppu_read(address, fetch),
            };
        }

        if !is_palette_address(address) {
            return self.read_nametable_inner(address, cart.as_deref());
        }

        self.palette[palette_index(address)]
    }

    /// Side-effect-free read of the PPU bus, for the tile emitter: the cartridge is only
    /// peeked (REQ-CORE-033).
    fn peek_vram(&self, address: u16, fetch: ChrFetch, cartridge: Option<&Cartridge>) -> u8 {
        let address = address & 0x3FFF;
        if address < 0x2000 {
            return cartridge.map_or(0, |cartridge| cartridge.ppu_peek(address, fetch));
        }

        if !is_palette_address(address) {
            return self.read_nametable_inner(address, cartridge);
        }

        self.palette[palette_index(address)]
    }

    /// Writes the PPU bus at `address` (masked to 14 bits); a [`PpuBusAccess::Cpu`] write
    /// also puts the address on the cartridge's bus.
    pub fn write_vram_through_bus(
        &mut self,
        address: u16,
        value: u8,
        access: PpuBusAccess,
        mut cartridge: Option<&mut Cartridge>,
    ) {
        self.write_vram_inner(address, value, access, &mut cartridge);
    }

    fn write_vram_inner(
        &mut self,
        address: u16,
        value: u8,
        access: PpuBusAccess,
        cart: Cart<'_, '_>,
    ) {
        let address = address & 0x3FFF;

        if access != PpuBusAccess::Inert {
            self.notify_address_bus(address, cart);
        }

        if address < 0x2000 {
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.ppu_write(address, value);
            }

            return;
        }

        if !is_palette_address(address) {
            self.write_nametable_byte(address, value, cart);
            return;
        }

        // Palette entries are 6 bits wide.
        self.palette[palette_index(address)] = value & 0x3F;
    }

    /// Tells the cartridge which address the PPU is driving, so that an MMC3-style board can
    /// count its A12 rises (REQ-CORE-114).
    fn notify_address_bus(&self, address: u16, cart: Cart<'_, '_>) {
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus(address, self.ppu_clock);
        }
    }

    /// Reads a nametable byte, from console VRAM or from the cartridge when the board
    /// supplies its own nametable (MMC5 `ExRAM` and fill mode).
    ///
    /// There is no side effect to worry about here, so the tile capture of REQ-CORE-037
    /// goes through the same path as rendering. The dot loop must hold with no cartridge
    /// attached (Q-062).
    #[must_use]
    pub fn read_nametable_byte(&self, address: u16, cartridge: Option<&Cartridge>) -> u8 {
        self.read_nametable_inner(address, cartridge)
    }

    fn read_nametable_inner(&self, address: u16, cartridge: Option<&Cartridge>) -> u8 {
        let page = cartridge_nametable_page(address, cartridge);
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            if let Some(cartridge) = cartridge {
                return cartridge.read_nametable(address);
            }
        }

        if let Ok(page) = usize::try_from(page) {
            return self.vram[(page << 10) | (usize::from(address) & 0x03FF)];
        }

        self.vram[map_nametable(address, Self::mirroring_of(cartridge))]
    }

    fn write_nametable_byte(&mut self, address: u16, value: u8, cart: Cart<'_, '_>) {
        let page = cartridge_nametable_page(address, cart.as_deref());
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.write_nametable(address, value);
                return;
            }
        }

        if let Ok(page) = usize::try_from(page) {
            self.vram[(page << 10) | (usize::from(address) & 0x03FF)] = value;
            return;
        }

        self.vram[map_nametable(address, Self::mirroring_of(cart.as_deref()))] = value;
    }

    /// Maps a nametable address to an offset in the 4 KiB VRAM, using the layout
    /// `mirroring` (SPEC-02 §2).
    #[must_use]
    pub const fn map_nametable(address: u16, mirroring: NametableMirroring) -> usize {
        map_nametable(address, mirroring)
    }

    /// Records of the frame, used by the frame runner and the tests.
    #[must_use]
    pub fn tile_records(&self) -> &[TileRecord] {
        self.tiles.records()
    }
}

fn cartridge_nametable_page(address: u16, cartridge: Option<&Cartridge>) -> i32 {
    cartridge.map_or(Cartridge::FOLLOW_MIRRORING, |cartridge| {
        cartridge.nametable_page(i32::from((address.wrapping_sub(0x2000) >> 10) & 0x03))
    })
}

const fn map_nametable(address: u16, mirroring: NametableMirroring) -> usize {
    let index = (address.wrapping_sub(0x2000) & 0x0FFF) as usize;
    let table = index >> 10;
    let offset = index & 0x03FF;

    let physical = match mirroring {
        NametableMirroring::Horizontal => table >> 1,
        NametableMirroring::Vertical => table & 1,
        NametableMirroring::SingleScreenA => 0,
        NametableMirroring::SingleScreenB => 1,
        NametableMirroring::FourScreen => table,
    };

    (physical << 10) | offset
}

/// Whether `address` falls in palette RAM, $3F00-$3FFF.
const fn is_palette_address(address: u16) -> bool {
    (address & 0x3F00) == 0x3F00
}

/// Index in palette RAM of `address`: $3F10, $3F14, $3F18 and $3F1C are redirected to
/// $3F00, $3F04, $3F08 and $3F0C (REQ-CORE-045).
const fn palette_index(address: u16) -> usize {
    let mut index = (address & 0x1F) as usize;
    if (index & 0x13) == 0x10 {
        index &= 0x0F;
    }

    index
}
