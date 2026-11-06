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
use crate::wide::WideExtension;
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

/// The bits of a byte of the picture that carry the NES colour index of REQ-CORE-048 (0 to 63).
pub const PIXEL_COLOR: u8 = 0x3F;

/// The bit of a byte of the picture that carries the opacity mask of AMD-050 REQ-CORE-049: 1
/// where the PPU produced at least one opaque pixel, 0 where the picture shows the universal
/// backdrop for want of one.
///
/// This is the internal layout the amendment's note allows ("Cette disposition n'est pas exigée
/// pour une interface interne"): the recommended 32-bytes-a-row bit array is for a native
/// interface, and carrying the bit in a spare bit of the picture is what makes the mask cost the
/// PPU nothing at all ([`Ppu2C02::palette_marked`]).
pub const OPAQUE_PIXEL: u8 = 0x40;

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

/// PPU dots after a `$2007` read's own `ppu_clock` from which the byte that read fetched is
/// the one a later read returns.
///
/// Two terms, both from `AccuracyCoin`'s `$2007 Stress Test` error code 2: "Reading from
/// $2007 should set up the PPU Read Buffer **two ppu cycles after the CPU Read ends**", and a
/// read presents the buffer at the *start* of its own CPU cycle, which is **three** dots before
/// the `ppu_clock` the bus hands [`Ppu2C02::read`] (the PPU is caught up to the end of the
/// cycle). So a read whose clock is less than five dots after another's still sees the older
/// buffer — which is every pair of `$2007` reads one CPU cycle apart, and no pair two cycles
/// apart.
///
/// That one-cycle pair is `dmc_dma_during_read4/double_2007_read`: `lda $20F7,x` with X=$10
/// dummy-reads `$2007` and then reads `$2107`, and its source lists the four outputs hardware
/// gives — all four have the second read returning the buffer as it stood *before* the first.
///
/// Two reads on the **same** dot are not a pair of CPU reads at all: only the time-free host
/// accessor `NesCore::ppu_read` can make them, and it means "read the register again", so the
/// window is open strictly after the previous read's dot.
const READ_BUFFER_READY_DOTS: i64 = 5;

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
/// The last dot of the fixed stretch of a visible line (dots 2 to 255: no line start, no
/// vertical increment, no reload) that [`Ppu2C02::run_visible_span`] runs.
const LAST_SPAN_DOT: i32 = 255;

/// The hardware's limit of eight sprites per line, in the picture. The evaluation, the overflow
/// flag and the pattern fetches the cartridge sees stay the hardware's whatever the choice: the
/// sprites past the eighth are only added to the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteLimit {
    /// Eight sprites per line, as on the console.
    #[default]
    Kept,
    /// Every sprite of a line shows: no flicker.
    Removed,
    /// Removed, but kept on a line where eight sprites or more at the same position follow
    /// each other in OAM: a game hiding the sprites after them there (The Legend of Zelda at
    /// its doors). Mesen's "automatically re-enable sprite limit as needed".
    Smart,
}

/// The picture processing unit.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // The chip's flags, one field each like the C#.
pub struct Ppu2C02 {
    vram: [u8; VRAM_SIZE],
    /// For each byte of `vram`, the rendered frame (`vram_clock`) it was last written in, for a
    /// margin decoder to tell what the game drew when (not saved: 0 after a state load).
    vram_written: Vec<u32>,
    vram_clock: u32,
    oam: [u8; OAM_SIZE],
    palette: [u8; PALETTE_SIZE],
    /// `palette` with the opacity bit of AMD-050 REQ-CORE-049 already set on every entry but
    /// entry 0: `palette[i] | 0x40` for `i ≥ 1`, `palette[0]` for entry 0.
    ///
    /// Derived, so it is **not** in the state blob (`ppu/state.rs` writes `palette` alone) and is
    /// rebuilt wherever `palette` is written — the one register write, the power-up fill and the
    /// state restore, none of them in a per-pixel path.
    ///
    /// It is what makes the opacity mask free. The pixel functions resolve a palette **index**
    /// that is 0 exactly where the pixel is transparent (see `render::span_pixel`), so reading
    /// this table instead of `palette`, with the colour mask widened to `0x7F`/`0x70`, carries
    /// the bit with the same indexed load and the same `AND` as before the amendment.
    palette_marked: [u8; PALETTE_SIZE],
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
    /// The byte the `$2007` read buffer held before the last read refilled it, for a read that
    /// comes too soon to see the refill (see [`READ_BUFFER_READY_DOTS`]).
    read_buffer_prev: u8,
    /// `ppu_clock` of the last `$2007` read, which is when `read_buffer` was refilled.
    /// Transient: the window in which the refill is not yet visible is shorter than two CPU
    /// cycles and no two `$2007` reads one cycle apart can straddle an instruction boundary, so
    /// it is not in the savestate.
    read_buffer_clock: i64,
    decay: u8,
    skip_latch: bool,
    vblank_just_started: bool,
    nmi_line_sample: bool,

    // Rendering state (render.rs).
    pixels: Vec<u8>,
    secondary_oam: [u8; 4 * SPRITES_PER_LINE],
    /// The primary OAM index of each sprite the evaluation copied to `secondary_oam`, for the
    /// widescreen extension: a sprite its high byte puts in a margin is left out of the picture
    /// (not saved: a state is taken between frames).
    secondary_source: [u8; SPRITES_PER_LINE],
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
    /// The sprite layer of the line being drawn, one cell per column, derived from the
    /// loaded slots above (see `rebuild_sprite_line`); not part of the saved state.
    sprite_line: [u8; SCREEN_WIDTH],
    /// True when the slots changed since `sprite_line` was built.
    sprite_line_stale: bool,
    /// Whether the picture shows more than eight sprites on a line (see [`SpriteLimit`]).
    sprite_limit: SpriteLimit,
    /// The sprites of the next line past the eight the hardware loads, when the limit is
    /// removed: their patterns, attributes and x, in OAM order after the eighth (not saved: a
    /// state is taken between frames).
    extra_pattern_low: [u8; 64],
    extra_pattern_high: [u8; 64],
    extra_attribute: [u8; 64],
    extra_x: [u8; 64],
    extra_loaded: usize,

    // Tile capture state (tiles.rs).
    tiles: TileBuffer,
    sprite_order: [usize; 64],
    mask_x: i32,
    mask_y: i32,

    // Widescreen extension (wide.rs); `frame_margin` is its margin latched when the frame
    // being captured began, so that every band of one frame has the same columns.
    wide: WideExtension,
    frame_margin: i32,
    /// The visible range of each band of the frame being captured (`FrameBatch::visible_rows`).
    visible_rows: crate::VisibleRows,
    /// The vertical widescreen margin of the frame being captured, latched with `frame_margin`.
    frame_vertical_margin: i32,
    /// The background rows of the frame being captured, by row index, for a margin decoder.
    captured_rows: [Option<tiles::CapturedRow>; 31],
    /// The lines of the screen whose centre the wide picture takes from the PPU's own pixels:
    /// all of them, but those of a row across the top or bottom edge whose centre a decoder gave
    /// (`decode_margins`), past which the game shows a row kept for the overscan.
    centre_lines: (i32, i32),
    /// The decoder of the frame just run asks its screen's top and bottom lines hidden with a
    /// vertical margin too ([`crate::wide_decode::MarginDecoder::hides_overscan`]).
    hides_overscan: bool,
    /// Per band of 8 lines, the x from which the centre of the wide picture is composed from the
    /// records rather than taken from the PPU (256: nowhere): the columns a margin decoder
    /// redraws at the right edge ([`crate::wide_decode::MarginDecoder::redrawn_right_columns`]).
    redrawn_from: [i16; 30],
    /// Per band of 8 lines, the x up to which the centre of the wide picture is composed from
    /// the records rather than taken from the PPU (0: nowhere): the columns a margin decoder
    /// redraws at the left edge
    /// ([`crate::wide_decode::MarginDecoder::redrawn_left_columns`]).
    redrawn_to: [i16; 30],
    /// The margin decoder's own background palette for the frame, for the decoded tiles in
    /// sub-palettes 4-7 ([`crate::wide_decode::MarginDecoder::margin_palette`]).
    margin_palette: Option<[u8; 16]>,
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
            vram_written: vec![0; VRAM_SIZE],
            vram_clock: 0,
            oam: [0; OAM_SIZE],
            palette: [0; PALETTE_SIZE],
            palette_marked: marked_palette(&[0; PALETTE_SIZE]),
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
            read_buffer_prev: 0,
            read_buffer_clock: 0,
            decay: 0,
            skip_latch: false,
            vblank_just_started: false,
            nmi_line_sample: false,
            pixels: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
            secondary_oam: [0; 4 * SPRITES_PER_LINE],
            secondary_source: [0; SPRITES_PER_LINE],
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
            sprite_line: [0; SCREEN_WIDTH],
            sprite_line_stale: true,
            sprite_limit: SpriteLimit::Kept,
            extra_pattern_low: [0; 64],
            extra_pattern_high: [0; 64],
            extra_attribute: [0; 64],
            extra_x: [0; 64],
            extra_loaded: 0,
            tiles: TileBuffer::new(),
            sprite_order: [0; 64],
            mask_x: 0,
            mask_y: 0,
            wide: WideExtension::new(),
            frame_margin: 0,
            visible_rows: crate::STANDARD_ROWS,
            captured_rows: [None; 31],
            redrawn_from: [256; 30],
            redrawn_to: [0; 30],
            frame_vertical_margin: 0,
            centre_lines: (0, 240),
            hides_overscan: false,
            margin_palette: None,
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

    /// Whether the picture shows more than eight sprites on a line; from the next line.
    pub fn set_sprite_limit(&mut self, limit: SpriteLimit) {
        self.sprite_limit = limit;
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

    /// Whether the PPU is actually fetching: rendering enabled **and** on a line that fetches —
    /// a visible one or the pre-render line.
    ///
    /// This is the condition the `$2004` register quirk of `AccuracyCoin`'s `Address $2004
    /// behavior` turns on. The pre-render line counts, and the ROM says so against its own test's
    /// title: `Address $2004 behavior` 6 is called "writing to `$2004` on a visible scanline" and
    /// its code notes that "the CPU write occurs on **dot 321 of the pre-render line**".
    ///
    /// The `$2007` quirk that shared this predicate until lot MF-4d-fix is deferred to lot MF-4e;
    /// see [`Self::increment_vram_address`] and `Q-125`.
    const fn fetching(&self) -> bool {
        self.rendering_enabled()
            && (self.scanline < VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline())
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

    /// The nametable layout in force: the cartridge's, unless the widescreen extension asks
    /// for four separate nametables.
    #[must_use]
    pub fn layout_of(&self, cartridge: Option<&Cartridge>) -> NametableMirroring {
        if self.wide.four_screen() {
            NametableMirroring::FourScreen
        } else {
            Self::mirroring_of(cartridge)
        }
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

    /// The margin decoder's own background palette for the frame just run, if any (the decoded
    /// tiles in sub-palettes 4-7).
    #[must_use]
    pub const fn margin_palette(&self) -> Option<[u8; 16]> {
        self.margin_palette
    }

    /// Rebuilds [`Self::palette_marked`] from `palette`. Called wherever the whole palette is
    /// replaced — the power-up fill and a state restore — and never per pixel.
    pub(super) fn refresh_marked_palette(&mut self) {
        self.palette_marked = marked_palette(&self.palette);
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
        self.refresh_marked_palette();
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
        self.read_buffer_prev = 0;
        self.read_buffer_clock = 0;
        self.decay = 0;
        self.skip_latch = false;
        self.vblank_just_started = false;
        self.nmi_line_sample = false;

        self.reset_render_state();
        self.wide.reset();
        self.reset_tile_state();

        // Just after a load or a reset the first frame starts at the beginning of vblank,
        // so the reset lands there (REQ-CORE-020, notes).
        self.scanline = self.timing.vblank_scanline;
        self.dot = 1;
    }

    /// The documented *after reset* state of the PPU (AMD-044 REQ-CORE-122): PPUCTRL and
    /// PPUMASK go to 0, the `$2005`/`$2006` write latch is cleared, the scroll register `t`
    /// and the `$2007` read buffer go to 0, and the odd-frame flag goes back to "even"
    /// (nesdev, "PPU power up state").
    ///
    /// Nothing else moves: PPUSTATUS, OAMADDR, the current address `v`, the nametable RAM, the
    /// OAM, the palette RAM and the CHR RAM are all unchanged, and the PPU keeps clocking from
    /// where it was — a RESET press does not restart the frame. `v_row_start` and `v_row_next`
    /// follow `v`, not `t`, so they stay as they are too. The ~29 658-cycle window during which
    /// the hardware ignores writes to PPUCTRL, PPUMASK, PPUSCROLL and PPUADDR is not modelled,
    /// no more than it is for the power-on reset (Q-078).
    // Q-078
    pub fn soft_reset(&mut self) {
        self.control = 0;
        self.mask = 0;
        self.write_toggle = false;
        self.t = 0;
        self.fine_x = 0;
        self.read_buffer = 0;
        self.read_buffer_prev = 0;
        self.read_buffer_clock = 0;
        self.odd_frame = false;
    }

    /// Advances the PPU by the dots of one CPU cycle of the current region.
    pub fn step_cpu_cycle(&mut self, mut cartridge: Option<&mut Cartridge>) {
        // accumulator / denominator by repeated subtraction: three or four rounds (3 or 3.2
        // dots per cycle) cost less than a division on every CPU cycle.
        let denominator = self.timing.dots_per_cycle_denominator;
        let mut accumulator = self.dot_accumulator + self.timing.dots_per_cycle_numerator;
        let mut dots = 0;
        while accumulator >= denominator {
            accumulator -= denominator;
            dots += 1;
        }
        self.dot_accumulator = accumulator;

        let sample_after = (dots - NMI_SAMPLE_LEAD).max(0);
        for dot in 0..dots {
            if dot == sample_after {
                self.nmi_line_sample = self.nmi_asserted();
            }

            self.step_dot(&mut cartridge);
        }
    }

    /// Advances the PPU by the dots of `cycles` CPU cycles, exactly as that many calls of
    /// [`Ppu2C02::step_cpu_cycle`] would.
    ///
    /// Only the NMI sample of the last cycle survives (each cycle overwrites it and nothing
    /// reads it in between), so the dots run in two stretches around that one sample.
    pub fn run_cpu_cycles(&mut self, cycles: u32, mut cartridge: Option<&mut Cartridge>) {
        if cycles == 0 {
            return;
        }

        let numerator = i64::from(self.timing.dots_per_cycle_numerator);
        let denominator = i64::from(self.timing.dots_per_cycle_denominator);
        let before = i64::from(self.dot_accumulator) + (i64::from(cycles) - 1) * numerator;
        let dots_before = before / denominator;
        let last = before % denominator + numerator;
        let dots_last = last / denominator;
        #[allow(clippy::cast_possible_truncation)] // A remainder of the i32 denominator.
        let accumulator = (last % denominator) as i32;
        self.dot_accumulator = accumulator;

        let sample_after = (dots_last - i64::from(NMI_SAMPLE_LEAD)).max(0);
        self.run_dots(dots_before + sample_after, &mut cartridge);
        self.nmi_line_sample = self.nmi_asserted();
        self.run_dots(dots_last - sample_after, &mut cartridge);
    }

    /// Runs `count` dots, skipping in one go the stretches where a dot does nothing but
    /// move the counters (or, on a visible line with rendering off, draw the backdrop).
    fn run_dots(&mut self, mut count: i64, cart: Cart<'_, '_>) {
        while count > 0 {
            let idle = self.idle_dots_ahead().min(count);
            if idle > 0 {
                #[allow(clippy::cast_possible_truncation)] // At most a line of dots.
                let idle_i32 = idle as i32;
                if self.scanline < VISIBLE_SCANLINES && self.dot < SCREEN_WIDTH_I32 {
                    self.fill_backdrop(self.dot + 1, (self.dot + idle_i32).min(SCREEN_WIDTH_I32));
                }

                self.dot += idle_i32;
                self.ppu_clock += idle;
                count -= idle;
                continue;
            }

            let span = self.visible_span_ahead(count);
            if span > 0 {
                if !self.run_visible_span(span, cart.as_deref()) {
                    self.run_visible_span_through_bus(span, cart);
                }
                count -= span;
            } else {
                self.step_dot(cart);
                count -= 1;
            }
        }
    }

    /// How many of the next dots lie in the stretch of a rendering visible line that
    /// [`Ppu2C02::run_visible_span`] can take (dots 2 to [`LAST_SPAN_DOT`]), at most `count`.
    fn visible_span_ahead(&self, count: i64) -> i64 {
        if self.scanline >= VISIBLE_SCANLINES
            || !self.rendering_enabled()
            || !(1..LAST_SPAN_DOT).contains(&self.dot)
        {
            return 0;
        }

        i64::from(LAST_SPAN_DOT - self.dot).min(count)
    }

    /// How many of the next dots of the current line have nothing to do but count (and draw
    /// the backdrop on a visible line with rendering off), so that
    /// [`Ppu2C02::run_dots`] may skip them.
    fn idle_dots_ahead(&self) -> i64 {
        let next = self.dot + 1;
        let visible = self.scanline < VISIBLE_SCANLINES;
        let pre_render = self.scanline == self.pre_render_scanline();
        if next < 2 || (self.rendering_enabled() && (visible || pre_render)) {
            return 0;
        }

        // Dot 340 closes the line; dot 338 of the pre-render line decides the odd frame's
        // skipped dot.
        let last_idle = if pre_render {
            SKIP_DECISION_DOT - 1
        } else {
            DOTS_PER_SCANLINE - 2
        };
        i64::from((last_idle - next + 1).max(0))
    }

    /// Dots from the current one to the next dot where the vblank flag may rise or fall on
    /// its own (dot 1 of the vblank line and of the pre-render line), at least 1.
    #[must_use]
    pub fn dots_until_nmi_change(&self) -> i64 {
        let frame = i64::from(self.timing.scanlines_per_frame) * i64::from(DOTS_PER_SCANLINE);
        let here = i64::from(self.scanline) * i64::from(DOTS_PER_SCANLINE) + i64::from(self.dot);
        let until = |scanline: i32| {
            let there = i64::from(scanline) * i64::from(DOTS_PER_SCANLINE) + 1;
            (there - here).rem_euclid(frame).max(1)
        };
        until(self.timing.vblank_scanline).min(until(self.pre_render_scanline()))
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
        if self.scanline == VISIBLE_SCANLINES {
            self.wide.arm_overclock();
        }

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
        if self.dot == SKIP_DECISION_DOT && self.scanline == self.pre_render_scanline() {
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

            // The sprite captures of SPEC-02 §3.5 run on the first dot of the visible line
            // that is always reached: the idle dot 0 of line 0 is dropped on a skipped odd
            // frame.
            //
            // **Nested inside the `dot == 1` test, not a second one of its own.** Both tests
            // read the same field and LLVM is free to merge them, and when lot MF-4c took the
            // MMC5's `on_ppu_scanline` out of this block it did: the two short-circuiting
            // branches became one branchless `setae`/`orb`/`jne` that evaluates the scanline
            // compare on **every** dot reaching this function, about 21 000 a frame.
            // Callgrind put that at +30 to +40 M instructions over 300 frames on every game,
            // NROM included — 0.5 % for a hook with nothing behind it.
            if self.scanline < VISIBLE_SCANLINES {
                self.begin_capture_line(self.scanline, cart);
            }
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

                // Dot 0 is where both of PPUSTATUS's timing quirks live, so the two share one
                // test on the dot and differ only in the scanline.
                if self.dot == 0 {
                    if self.scanline == self.timing.vblank_scanline {
                        // Reading the flag one dot before it would be raised suppresses it for
                        // the whole frame (nesdev; vbl_nmi_timing test 2, case 8).
                        self.suppress_vblank = true;
                    } else if self.scanline == self.pre_render_scanline() {
                        // **The sprite flags are sampled at the end of the read, the vblank flag
                        // at its start.** All three are cleared on dot 1 of the pre-render line,
                        // and `AccuracyCoin`'s `$2002 flag timing` says why a read still sees
                        // them differently: "Reads from $2002 will latch the state of the vblank
                        // flag at the beginning of the read (when M2 goes high), but the sprite
                        // flags are not latched, so the value you get back is the state of the
                        // sprite flags at the end of the read (when M2 goes low). […] the sprite
                        // flags are read approximately 1.875 PPU cycles after the vblank flag."
                        //
                        // So a read landing on dot 0 of this line samples the sprite flags at
                        // about dot 1.9 — past the clear — while its vblank bit is the one dot 0
                        // still had. That is the single byte of the ROM's key that moves: its
                        // four reads walk dots 339 and 340 of the line before, then dots 0 and 1
                        // of this one, and want `E0 E0 80 00` where every flag latched at the
                        // read's own dot would give `E0 E0 E0 00`. Dot 340 of the previous line
                        // is 1.9 dots short of the clear, so it keeps both bits, and by dot 1
                        // the clear has been seen by the vblank bit too.
                        //
                        // The ROM also accepts `E0 80 80 00`, a console whose sampling lands a
                        // dot earlier; this takes the first key.
                        status &= !0x60;
                    }
                }

                self.refresh_decay(status, 0xE0);

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
                // The buffer this read sees. A read refills it only `READ_BUFFER_READY_DOTS`
                // dots later, so a second read one CPU cycle behind still gets the old byte.
                //
                // Both bookkeeping fields are written **here**, before the fetch below, so that
                // nothing of this has to stay live across that call: a value kept across it
                // costs `read_internal` a callee-saved register — a `pushq`/`popq` pair on
                // every CPU read cycle of every game, measured and then removed.
                let since = self.ppu_clock - self.read_buffer_clock;
                let visible = if since > 0 && since < READ_BUFFER_READY_DOTS {
                    self.read_buffer_prev
                } else {
                    self.read_buffer
                };
                self.read_buffer_prev = visible;
                self.read_buffer_clock = self.ppu_clock;

                let value;
                if is_palette_address(address) {
                    // Palette reads are immediate and only drive six bits; the buffer still
                    // takes the mirrored nametable byte "underneath" the palette (nesdev).
                    value = (self.palette[palette_index(address)] & self.palette_read_mask())
                        | (self.decay & 0xC0);
                    self.refresh_decay(value, 0x3F);
                    self.read_buffer = self.read_vram_inner(
                        address & 0x2FFF,
                        PpuBusAccess::Cpu,
                        ChrFetch::Background,
                        &mut cartridge,
                    );
                } else {
                    value = visible;
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

    /// What the palette RAM drives onto the PPU's data bus: six bits, and only the two colour
    /// bits of a grey when PPUMASK's greyscale bit is set (REQ-CORE-047).
    ///
    /// The bit masks the palette's **output**, so it reaches the CPU through `$2007` exactly as
    /// it reaches the pixel — `AccuracyCoin`'s `Palette RAM Quirks` code 6, "with Greyscale Mode
    /// enabled, the lower four bits of the value read should all be zero", against its code 7,
    /// "the lower four bits of the value *written* should be unaffected". The stored byte is
    /// therefore left alone, which is why [`Ppu2C02::peek`] — the side-effect-free host accessor
    /// of REQ-CORE-081, which answers what was written — does not apply this.
    const fn palette_read_mask(&self) -> u8 {
        if (self.mask & 0x01) != 0 {
            0x30
        } else {
            0x3F
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
                } else if (1..READ_BUFFER_READY_DOTS)
                    .contains(&(self.ppu_clock - self.read_buffer_clock))
                {
                    self.read_buffer_prev
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

            4 => {
                if self.fetching() {
                    // **A `$2004` write made while the PPU is fetching does not reach OAM, and
                    // bumps the address by four.** `AccuracyCoin`'s `Address $2004 behavior` 6
                    // ("should increment the OAM address by 4"), 7 ("shouldn't write to OAM") and
                    // A ("increment the OAM address by 4, and then bitwise AND the OAM address
                    // with `$FC`"). Its test 7 writes `$8C` over the whole of OAM this way and
                    // then requires OAM `$00` to still hold the `$5A` a transfer put there.
                    //
                    // Only the register write is suppressed: an OAM transfer's bytes go through
                    // [`Self::write_oam_dma`] and keep writing, which is what every game that
                    // starts a transfer with rendering on depends on.
                    self.oam_address = self.oam_address.wrapping_add(4) & 0xFC;
                } else {
                    self.write_oam(value);
                }
            }

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
                    // MMC3 counter by hand, and what mmc3_test measures (REQ-CORE-114). It is
                    // the address lines alone — /RD stays high — so it goes through the
                    // without-read notification, which a board waiting on /RD does not take.
                    self.notify_address_bus_without_read(self.v & 0x3FFF, &mut cartridge);
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

    // Q-125: **a `$2007` access made while the PPU is fetching moves `v` by a coarse X increment
    // and a Y increment at once on hardware**, not by the 1 or 32 of PPUCTRL (nesdev, "PPU
    // registers": "it will update v in an odd way, triggering a coarse X increment and a Y
    // increment simultaneously"; `third_party/mesen2` takes the same branch, and
    // `AccuracyCoin`'s `$2007 Read w/ Rendering` 2 asks for the vertical half by name).
    //
    // That rule shipped in lot MF-4d step 6 and was **backed out here**, in lot MF-4d-fix: the
    // increment it produces is right only for the dot the access actually lands on, and applied
    // dot-blind it broke AMD-021 R2-005 criterion 2 (a background row on Super Mario Bros. 3's
    // split line on 4 986 of 5 000 frames) and moved the pictures of Super Mario Bros. 3 and
    // Zelda II in golden-core. R2-005 is a P1 requirement; one `AccuracyCoin` row is not.
    //
    // Lot MF-4e owns the per-dot work and must satisfy **both** the rule and R2-005. Until then
    // the ordinary PPUCTRL step applies on every line, which is `d96d73b7`'s behaviour.
    //
    // **No `#[inline]`, which is `d96d73b7`'s shape and was checked both ways.** MF-4d step 6
    // needed the attribute because its `fetching` branch pushed this body past the inliner's
    // budget; without the branch the body is small again and LLVM inlines it unasked — 0
    // definitions and 0 call sites in the post-LTO dump on both targets. Adding the attribute
    // back was measured and changes nothing: `ppu_vbl_nmi`, 300 frames, callgrind, whole
    // process, 3 331 792 209 without it against 3 331 799 091 with it.
    //
    // It is **not** the lever on the cost this back-out gives back, which is a block-layout flip
    // inside [`Self::read`] — see `docs/progress/MF-4d-fix.md` step 5.
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
            self.notify_address_bus_without_read(address, cart);
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
        let index = palette_index(address);
        self.palette[index] = value & 0x3F;
        // AMD-050 REQ-CORE-049: the shadow entry follows, here and not per pixel. A game writes
        // a handful of palette bytes a frame; the PPU reads 61 440 pixels.
        self.palette_marked[index] = mark(index, value & 0x3F);
    }

    /// Tells the cartridge which address the PPU is **reading**, so that an MMC3-style board can
    /// count its A12 rises (REQ-CORE-114) and the MMC5 can detect the start of a scanline on its
    /// own bus (nesdev, "MMC5"; finding MR-5-4).
    ///
    /// This is the notification of every fetch of a rendered line, nametable and attribute
    /// bytes included — see [`Cartridge::on_ppu_address_bus`]. It carries no cost for a board
    /// that does not ask for it: the test is a load of one `bool` settled at load time.
    fn notify_address_bus(&self, address: u16, cart: Cart<'_, '_>) {
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus(address, self.ppu_clock);
        }
    }

    /// The counterpart for the address lines moving with **/RD high**: a `$2007` write, or a
    /// `$2006` write loading `v`. A12 is an address line, so an A12 counter must see both, and a
    /// board whose rule is about /RD — the MMC5's in-frame detection — must not
    /// (`Cartridge::watches_ppu_bus_without_read`).
    pub(super) fn notify_address_bus_without_read(&self, address: u16, cart: Cart<'_, '_>) {
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus_without_read(address, self.ppu_clock);
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

    /// Where nametable `address` lies: the console VRAM page the cartridge maps it to,
    /// [`Cartridge::FOLLOW_MIRRORING`] or [`Cartridge::SERVED_BY_CARTRIDGE`]. With the widescreen
    /// extension's four nametables, one the cartridge maps to console VRAM follows them instead:
    /// an MMC5 picks its pages itself (`$5105`), the hack's layout must win over it.
    fn nametable_page(&self, address: u16, cartridge: Option<&Cartridge>) -> i32 {
        let page = cartridge_nametable_page(address, cartridge);
        if page >= 0 && self.wide.four_screen() {
            Cartridge::FOLLOW_MIRRORING
        } else {
            page
        }
    }

    /// Kept **out of line**. The cartridge's nametable read gained a `CartridgeMemory`
    /// argument in lot MF-2 (the UNROM 512 serves its four-screen nametables out of CHR-RAM,
    /// MR-30-6), which made this small enough for the inliner to fold into `peek_vram` — and
    /// from there into `read_pattern_planes`, where it hoisted `layout_of` above the loop and
    /// cost that function sixteen instructions a call for a branch pattern fetches never take.
    /// `read_pattern_planes` is the function R0's callgrind of MF-1 singled out; see
    /// `docs/progress/MF-2.md`, step 7.
    #[inline(never)]
    fn read_nametable_inner(&self, address: u16, cartridge: Option<&Cartridge>) -> u8 {
        self.read_nametable_as(address, cartridge, self.layout_of(cartridge))
    }

    /// A nametable byte as the PPU reads it under `layout`, the mirroring it had when a row was
    /// drawn (a game may switch it during the frame, a status bar's split for instance).
    ///
    /// **`inline(always)`, as the counterpart of `read_nametable_inner`'s `inline(never)`.** This
    /// is the body of every nametable fetch, and it must end up *inside* its out-of-line caller,
    /// not behind a second call. It is the same accident as `Cpu6502::end_cycle` (step 1) and it
    /// bit the same function twice: the field doc of `Cartridge::nametable_pages` records lot
    /// MF-2 finding this exact function out of line after the UNROM 512 became a second board
    /// with a nametable page of its own, and MF-3 put it back out of line — R0's callgrind of
    /// `42b541ef` has `Ppu2C02::read_nametable_as` at 0 → **3.18 M** exclusive instructions with
    /// `read_nametable_inner` +0.84 M, which is the `callq`/`retq` appearing between the two.
    ///
    /// The decision is pinned rather than left to the inliner because it is pure cost either
    /// way: `read_nametable_inner` stays `#[inline(never)]`, so folding this into it cannot
    /// reach `read_pattern_planes` — the function MF-2 built that shield for.
    ///
    /// Note that this one is **invisible in the crate's own assembly**: `cargo rustc --emit=asm`
    /// on `zax-nes` shows the same tail call at `7cbd9c52` and at `42b541ef`. The profile is
    /// `lto = "fat"`, and this is a decision LTO takes afterwards; see step 3.
    #[allow(clippy::inline_always)] // Measured: a `callq` here costs 4.0 M instructions.
    #[inline(always)]
    fn read_nametable_as(
        &self,
        address: u16,
        cartridge: Option<&Cartridge>,
        layout: NametableMirroring,
    ) -> u8 {
        let page = self.nametable_page(address, cartridge);
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            if let Some(cartridge) = cartridge {
                return cartridge.read_nametable(address);
            }
        }

        if let Ok(page) = usize::try_from(page) {
            return self.vram[(page << 10) | (usize::from(address) & 0x03FF)];
        }

        self.vram[map_nametable(address, layout)]
    }

    /// The rendered frame a nametable byte was last written in (0: not since power-on or the
    /// last state load; always 0 for nametables the cartridge serves), under `layout`.
    fn nametable_written_as(
        &self,
        address: u16,
        cartridge: Option<&Cartridge>,
        layout: NametableMirroring,
    ) -> u32 {
        let page = self.nametable_page(address, cartridge);
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            return 0;
        }
        let offset = usize::try_from(page).map_or_else(
            |_| map_nametable(address, layout),
            |page| (page << 10) | (usize::from(address) & 0x03FF),
        );
        self.vram_written.get(offset).copied().unwrap_or(0)
    }

    fn write_nametable_byte(&mut self, address: u16, value: u8, cart: Cart<'_, '_>) {
        let page = self.nametable_page(address, cart.as_deref());
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.write_nametable(address, value);
                return;
            }
        }

        let offset = if let Ok(page) = usize::try_from(page) {
            (page << 10) | (usize::from(address) & 0x03FF)
        } else {
            map_nametable(address, self.layout_of(cart.as_deref()))
        };
        self.vram[offset] = value;
        if let Some(written) = self.vram_written.get_mut(offset) {
            *written = self.vram_clock;
        }
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

    /// The widescreen extension.
    #[must_use]
    pub const fn wide(&self) -> &WideExtension {
        &self.wide
    }

    /// The widescreen extension, for the bus and the host.
    pub fn wide_mut(&mut self) -> &mut WideExtension {
        &mut self.wide
    }

    /// The widescreen margin, in pixels on each side, of the frame whose records are held: 0
    /// for a standard 256-pixel frame.
    #[must_use]
    pub const fn frame_margin(&self) -> i32 {
        self.frame_margin
    }

    /// The vertical widescreen margin, in pixels above and below, of the frame whose records
    /// are held.
    #[must_use]
    pub const fn frame_vertical_margin(&self) -> i32 {
        self.frame_vertical_margin
    }

    /// The visible range of each band of the frame whose records are held.
    #[must_use]
    pub const fn visible_rows(&self) -> crate::VisibleRows {
        self.visible_rows
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

/// The colour of palette entry `index` with the opacity bit of AMD-050 REQ-CORE-049 on it:
/// entry 0 is the universal backdrop, which is what a **transparent** pixel resolves to, so it
/// alone keeps a clear bit.
const fn mark(index: usize, color: u8) -> u8 {
    if index == 0 {
        color
    } else {
        color | OPAQUE_PIXEL
    }
}

/// [`Ppu2C02::palette_marked`] built from a palette.
const fn marked_palette(palette: &[u8; PALETTE_SIZE]) -> [u8; PALETTE_SIZE] {
    let mut marked = [0u8; PALETTE_SIZE];
    let mut index = 0;
    while index < PALETTE_SIZE {
        marked[index] = mark(index, palette[index]);
        index += 1;
    }
    marked
}
