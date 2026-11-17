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

mod edges;
mod emphasis;
mod fast;
mod palette_corruption;
mod ppu_bus_access;
mod render;
mod sprite_hw;
mod state;
mod tile_buffer;
mod tiles;

pub use ppu_bus_access::PpuBusAccess;
pub use tile_buffer::TileBuffer;

use crate::cartridges::{AnyBoard, BoardKind, Cartridge, ChrFetch};
use crate::wide::WideExtension;
use crate::{NametableMirroring, NesRegion, RegionTiming, TileRecord};
use std::marker::PhantomData;

/// The cartridge as the internal PPU paths pass it around.
type Cart<'a, 'c, K> = &'a mut CartSlot<'c, K>;

/// The cartridge, with the [`BoardKind`] the frame runs on (lot CORE-2): the PPU's internal paths
/// take it so that the compiler knows which board is behind it, and it reads as the
/// `Option<&mut Cartridge>` it wraps.
pub(crate) struct CartSlot<'c, K> {
    cartridge: Option<&'c mut Cartridge>,
    kind: PhantomData<K>,
}

impl<'c, K> CartSlot<'c, K> {
    #[allow(clippy::inline_always)] // Lot CORE-2: the specialised paths rest on these being inlined.
    #[inline(always)]
    pub(crate) const fn new(cartridge: Option<&'c mut Cartridge>) -> Self {
        Self {
            cartridge,
            kind: PhantomData,
        }
    }
}

impl<'c, K> std::ops::Deref for CartSlot<'c, K> {
    type Target = Option<&'c mut Cartridge>;

    #[allow(clippy::inline_always)] // Lot CORE-2: the specialised paths rest on these being inlined.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.cartridge
    }
}

impl<K> std::ops::DerefMut for CartSlot<'_, K> {
    #[allow(clippy::inline_always)] // Lot CORE-2: the specialised paths rest on these being inlined.
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cartridge
    }
}

/// Size of nametable VRAM: 4 KiB, enough for four-screen mode (REQ-CORE-090).
pub const VRAM_SIZE: usize = 4096;

/// Size of OAM.
pub const OAM_SIZE: usize = 256;

/// The bits of a `$2002` read that come from the PPU's own bus latch on a 2C02: bits 0 to 4,
/// which the flags of bits 5 to 7 leave alone (nesdev, "PPU registers"; `ppu_open_bus` tests 6
/// and 7). An RC2C05 drives them with an identifier instead
/// (`NesBus::vs_status`, lot MAPPERS-N).
const STATUS_LATCH_BITS: u8 = 0x1F;

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
///
/// That was 338 while a `$2001` write took effect at once. Since lot FIDELITY the switch of the
/// rendering arrives two dots after its write (`Ppu2C02::switch_rendering`), so the same writes
/// are seen by the last dot of the line — which is also where Mesen 2 reads it, one dot earlier
/// with its own one-dot delay.
const SKIP_DECISION_DOT: i32 = 340;

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

/// One dot of a per-dot trace (feature `ppu-dot-trace`, lot FIDPLUS-2): where the scan is after
/// the dot ran, the two address registers, and the last address the PPU put on its bus during
/// the dot (`0xFFFF` when it put none). Compared with the transistor netlist dot by dot.
#[cfg(feature = "ppu-dot-trace")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotRecord {
    /// Scanline after the dot.
    pub scanline: i16,
    /// Dot after the dot ran.
    pub dot: i16,
    /// `v` after the dot.
    pub v: u16,
    /// `t` after the dot.
    pub t: u16,
    /// Last address put on the PPU bus during the dot, `0xFFFF` for none.
    pub bus: u16,
    /// CPU access to a PPU register on this dot (the access dot, as the core places it):
    /// `$2000-$2007`, bit 15 set for a write, with its value in bits 16-23; 0 for none.
    pub access: u32,
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
    /// Whether the OAM row corruption of [`Self::rendering_switched`] is emulated. A core setting,
    /// on by default (lot FIDELITY), and not part of the state: it describes the machine, not the
    /// game.
    oam_corruption: bool,
    /// The OAM rows a switch-off of the rendering marked, one bit a row, waiting for the rendering
    /// to come back on a line that renders. Transient: a state load drops it.
    oam_corrupt_rows: u32,
    /// The row the last switch-off seeded, which a switch-off at the end of the clear (dot 65)
    /// copies again without taking a new one (`TriCNES`). Transient like `oam_corrupt_rows`.
    oam_seed_row: u32,
    /// Whether the 2C02's OAMADDR copy at the start of rendering is emulated
    /// ([`Self::oamaddr_row_copy`]). A core setting, on by default; not part of the state.
    oamaddr_bug: bool,
    /// The last `$2007` write outside rendering: the PPU clock it came on and the address it
    /// stored to (lot FIDPLUS-2, [`Self::rewrite_pending_vram_write`]). Not part of the state:
    /// the two writes it pairs belong to one instruction, and a state is never taken inside one.
    last_data_write: (i64, u16),
    /// The next dot 257 makes no horizontal copy: rendering was switched on with the switch
    /// landing on dot 256 (lot FIDPLUS-2, A5, [`Self::switch_rendering`]). Consumed on dot 257;
    /// not part of the state (a `$2001` write ends its instruction and the dot follows at once).
    copy_257_suppressed: bool,
    /// Fidélité+ (`edges`): the state of its own loop, kept apart (boxed) so the standard core's
    /// layout stays its own (lot FIDPLUS-A, perf). Not part of the state but for the edges.
    fid: Box<edges::FidState>,
    /// Per-dot trace (feature `ppu-dot-trace`): on, the PPU runs every dot through
    /// [`Self::step_dot`] (no span, no idle skip) and records it. Not part of the state.
    #[cfg(feature = "ppu-dot-trace")]
    dot_trace: Option<Vec<DotRecord>>,
    /// Last address put on the bus in the current dot, for [`DotRecord::bus`].
    #[cfg(feature = "ppu-dot-trace")]
    dot_trace_bus: std::cell::Cell<u16>,
    /// Whether OAM decays while the rendering is off ([`Self::refresh_oam_row`]). A core
    /// setting, off by default; not part of the state.
    oam_decay: bool,
    /// Whether a power-on puts the PPU at the top of the picture (line 0, dot 11) rather than at
    /// the start of vblank ([`Self::reset`]). A core setting, off by default; read at the reset.
    power_on_top: bool,
    /// Whether the console wires the PPU's /RST pin to the reset line, as the front-loading NES
    /// (NES-001) does ([`Self::after_reset`]). A core setting, not in the state.
    reset_line: bool,
    /// The low byte of the address the PPU's octal latch last took (its last ALE, lot FIDELITY
    /// step 39): the low eight address lines share their pins with the data, and the latch keeps
    /// them through the read. Only read when the rendering comes back on an odd dot in alignments
    /// 2 and 3 ([`Self::stale_first_fetch`]); kept up to date only where the bus stops moving —
    /// a switch off, the end of a rendered frame, a `$2007` access — never per fetch.
    ale_low: u8,
    /// Whether this is a 2C02 older than the G revision (lot FIDELITY step 37): `$2007` reads of
    /// `$3F00-$3FFF` behave as the rest of the PPU memory, buffered, instead of returning the
    /// palette (nesdev "PPU registers": palette reads are "supported by the 2C02G, 2C02H, and
    /// PAL PPUs"; "On PPUs that do not support reading palette RAM, this memory range behaves
    /// the same as the rest of PPU memory"), and `$2003` writes do not corrupt OAM. A core
    /// setting, not in the state.
    pre_g_revision: bool,
    /// Whether, on a 2C02G, a `$2003` write copies the OAM row the CPU's previous bus byte points
    /// to over the row it writes ([`Self::oamaddr_write_corruption`]). A core setting.
    oamaddr_write_corruption: bool,
    /// Set by a reset (power-on or the Reset button) when [`Self::reset_line`] is on, cleared on
    /// dot 1 of the pre-render line: until then writes to `$2000`, `$2001`, `$2005` and `$2006`
    /// are ignored (nesdev, "PPU power up state", ~29 658 cycles; the transistor netlist's
    /// `after_reset_reg`, set while /RES is low and cleared there). In the state, as bit 1 of the
    /// write toggle's byte.
    pub(super) after_reset: bool,
    /// Study `study/access-phase`: dots added to the power-on position, which moves the CPU/PPU
    /// alignment (0 to 2).
    phase_offset: i32,
    /// The clock of the last refresh of each 8-byte OAM row by an access: `$2004`, OAM DMA, or the
    /// rendering coming back. Transient, like `oam_render_off`: a state load stamps them all with
    /// the clock it restores, so a row never decays right after a load.
    oam_row_refresh: [i64; 32],
    /// The clock at which the rendering last went off; while it is on, it refreshes every row on
    /// every line it evaluates the sprites of.
    oam_render_off: i64,
    /// Whether a `$2007` read and a `$2006` write made while the PPU is fetching meet the fetches
    /// as on the hardware: their effect a few dots later, and the fetch they collide with
    /// disturbed ([`Self::read_buffer_from_fetch`], [`Self::delay_address_load`]). A core setting,
    /// on by default (a PAL console video of *Super Mario Bros. 3* shows the damage it predicts,
    /// `docs/review/FIDELITY-LEADS.md` I1), and not part of the state; off, both act at once as
    /// before lot FIDELITY.
    bus_conflicts: bool,
    /// Whether the second `$2006` write loads `v` three dots later, with the hybrid address of the
    /// fetch it meets ([`Self::delay_address_load`]). Measured on the transistor netlists
    /// (`docs/review/FIDELITY-LEADS.md`, A3c): `v` loads in the middle of the third dot after
    /// the dot a read of the same cycle samples. A core setting, on by default; not in the state.
    address_load_delay: bool,
    /// Whether a `$2006` load that lands on a coarse-X increment, or on dot 256, ANDs with the
    /// incremented address ([`Self::address_load_collision`]). On by default.
    glitch_2006: bool,
    /// Whether a `$2000` write that starts on dot 257 lets the dot-257 copy take the nametable-X
    /// bit from the CPU's open bus ([`Self::control_write_glitch`]), and whether, in alignment 3,
    /// a `$2005` write takes the open bus for the copy of `t` it lands on
    /// ([`Self::scroll_write_on_copy`]) and for the fine X of its dot's pixel
    /// ([`Self::fine_x_write_mid_line`]). On by default.
    glitch_2000: bool,
    /// Whether, in alignment 2, switching rendering off on the first two dots of a nametable
    /// fetch with `v` at `$3C00` or above, or a `$2006` load taking `v` out of palette RAM on a
    /// visible dot, corrupts the background palettes ([`Self::corrupt_palettes`], `TriCNES`'s
    /// model, D5). On by default; a core setting, not in the state.
    palette_corruption: bool,
    /// Which of the four CPU/PPU clock alignments this PPU follows, 0 to 3, numbered as
    /// `AccuracyCoin` and `TriCNES` number them: the CPU's access one master clock later in the dot
    /// from 1 to 2, 3 and 0 (`docs/review/FIDELITY-LEADS.md`, A4 and D1). The transistor netlist
    /// with the CPU's hold time (step 33; its phases a2, a0, a6, a4 are alignments 0 to 3) sets the
    /// timings, `TriCNES` (MIT, Chris Siebert, `AccuracyCoin`'s author, calibrated on his consoles)
    /// what the netlist cannot show: a `$2007` read latches its buffer 6 dots after the access in
    /// alignments 2 and 3, 5 in 0 and 1 ([`Self::read_buffer_from_fetch`], both agree); the
    /// `$2000` dot-257 glitch and the `$2005` open bus happen in alignment 3 only
    /// ([`Self::scroll_write_on_copy`]); `$2001` acts two dots after the write and `$2006` loads
    /// three in all four, with the picture's edges of [`Self::switch_pixel_edge`]; the OAM row
    /// corruption (`TriCNES`: the netlist only shows it at some phases) is seeded one dot later in
    /// alignments 1 and 2. A core setting, not part of the state; 3 by default, the alignment this
    /// core always followed.
    alignment: u8,
    /// The CPU data bus before the write in progress: what the PPU's data pins see for the first
    /// half master clocks of the write, before the CPU drives its value. Set by the bus.
    cpu_open_bus: u8,
    /// Dots [`Self::switch_rendering`] ran ahead of the CPU, taken off the next catch-up. Never
    /// outstanding at a frame boundary (the switch only runs ahead on a line that renders), so
    /// not in the state.
    prepaid_dots: i32,
    /// A pixel a `$2001` switch on leaves at the backdrop, written when its line is over
    /// ([`Self::switch_pixel_edge`]): index in the picture and colour. Never pending at a frame
    /// boundary, so not in the state.
    pending_backdrop_pixel: Option<(usize, u8)>,
    /// The high plane's serial input, kept off the hot path (see `shift_background`): the 1s it
    /// had shifted in at the clock `serial_since` (the last switch of the rendering), at most 16.
    /// A reload after that clock starts the count again; the reloads themselves record nothing,
    /// so that the fetch loop keeps its registers (`Self::serial_ones`). Transient, like the rest
    /// of what a switch leaves: a state load drops it, which only matters if it lands between a
    /// switch-off and a switch-on.
    serial_since: i64,
    serial_base: u32,
    /// The sprite output units while a switched rendering has them off their usual course (see
    /// [`sprite_hw`]). Transient, like the rest of what a switch leaves.
    sprite_units: sprite_hw::SpriteUnits,
    /// The line [`Self::note_fetch_pause`] is counting for (the clock of its dot 0), the steps of
    /// the secondary OAM address it lost so far, and the dot the rendering went off on, −1 while
    /// it is on. Transient, like the row corruption.
    fetch_line: i64,
    fetch_missed: i32,
    fetch_off_dot: i32,
    /// The secondary OAM's "overflowed" flag as [`Self::note_oam2_overflow`] left it when the
    /// rendering went off. Transient.
    oam2_overflowed: bool,
    /// Palette RAM **as the game wrote it**, six bits an entry: what a `$2007` read of
    /// `$3F00`-`$3F1F` gives back and what the state blob carries, because palette RAM sits
    /// before the chip's colour decoder and a read never sees that decoder.
    ///
    /// On a NES image it is equal to [`Self::palette`] byte for byte, written in the same breath
    /// by the one register write below; the two differ only on a VS. System whose PPU has a
    /// scrambled master palette (lot MF-VS step 2).
    palette_raw: [u8; PALETTE_SIZE],
    /// Palette RAM **as the PPU shows it**: `palette_raw` with the VS. System's master-palette
    /// remap applied, and the identity on every NES image.
    ///
    /// This is the entry the whole output of the core resolves — the pixel image of
    /// REQ-CORE-048, the tile records, the backdrop and the frame batch's 32 bytes — so applying
    /// the remap *here*, once per palette write, is what keeps a VS palette out of every
    /// per-pixel path (see [`Self::remap_palette`]).
    palette: [u8; PALETTE_SIZE],
    /// `palette` with the opacity bit of AMD-050 REQ-CORE-049 already set on every entry but
    /// entry 0: `palette[i] | 0x40` for `i ≥ 1`, `palette[0]` for entry 0.
    ///
    /// Derived, so it is **not** in the state blob (`ppu/state.rs` writes `palette_raw` alone)
    /// and is rebuilt wherever palette RAM is written — the one register write, the power-up
    /// fill and the state restore, none of them in a per-pixel path.
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
    /// The OAM copy buffer: the byte the evaluation latched on the last odd dot, which is also
    /// what a `$2004` read sees while the PPU is fetching (see [`Ppu2C02::read_oam`]).
    oam_eval_latch: u8,
    /// The high six bits of the evaluation's OAM address — the sprite it stands on. mesen2's
    /// `_spriteAddrH`.
    eval_sprite: i32,
    /// The low two bits — the byte within that sprite. mesen2's `_spriteAddrL`. It is **not**
    /// always a byte index: an evaluation that starts misaligned (`$2003` left a value that is not
    /// a multiple of four) reads its first byte at that offset and interprets it as a Y
    /// coordinate, which is the whole of `AccuracyCoin`'s `Misaligned OAM behavior`.
    eval_byte: i32,
    /// The write pointer into `secondary_oam`, 0 to 32, counted in **bytes and not sprites**
    /// (mesen2's `_secondaryOamAddr`). At 32 the secondary OAM is full and its writes turn into
    /// reads, which is `AccuracyCoin`'s `Frozen OAM2 Increment`.
    ///
    /// **Not in the savestate, and the format does not move for it.** Like `secondary_source`
    /// above, it is alive only between dots 65 and 256 of a visible line: dot 65 assigns it
    /// unconditionally and nothing outside that window reads it. A state is taken between frames,
    /// so there is no such window to be in the middle of, and adding a field would have meant a
    /// new format version for a value that is always 0 where a block can be written.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    secondary_oam_addr: i32,
    /// Whether the evaluation is in the middle of copying a sprite it has judged in range. It is
    /// **latched**, not recomputed per byte: the range test runs on every even dot, but once it
    /// has said yes the four bytes are copied without asking again.
    ///
    /// Not in the savestate, for the reason given on `secondary_oam_addr`.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    sprite_in_range: bool,
    /// The last dot the evaluation copied a byte on, for the OAM row corruption of alignment 2,
    /// which differs on the dot of a copy (lot FIDELITY step 29). Not in the savestate, like
    /// `secondary_oam_addr`.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    eval_copy_dot: i32,
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
    /// Lot FAST-1: the frame being run is not shown (run-ahead, catch-up), so the fast core's
    /// line renderer leaves its pixels unpainted. The picture is not in the state and nothing
    /// else reads a frame's pixels (the Zapper and the capture read the tile records), so the
    /// machine is the same. Not in the state.
    skip_pixels: bool,
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
    /// The picture line the rows the margin decoder decoded end at (the playfield's bottom: a
    /// status bar under it), `None` when it decoded none.
    playfield_end: Option<i16>,
    /// Whether [`Ppu2C02::playfield_end`] is found without margins too.
    wants_playfield_end: bool,
    /// The scroll of each visible line as it was drawn (x on the 512-pixel plane, y on the
    /// 480-line one), kept only while [`Ppu2C02::set_tracks_line_scroll`] asks: the smooth
    /// scrolling finds the band that scrolls in it.
    line_scroll: Vec<Option<(u16, u16)>>,
    tracks_line_scroll: bool,
    /// The OAM sprites of the frame drawn in the "asleep" tint, one bit each
    /// ([`crate::wide::WideExtension::sprite_tinted`]).
    tinted_sprites: u64,
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
    /// The palette and PPUMASK as the frame just drawn ended (its vblank's first line), which
    /// the wide 2D picture is coloured with. Not saved: with run-ahead the host composes that
    /// picture once the real frame's state is back, whose palette is a frame older than the
    /// pixels and records it keeps; the first column, composed with it, showed a fade a step
    /// late.
    ///
    /// Lot KIRBY-MERGE kept these two **here**, at the cold end of the struct, and not beside
    /// [`Self::palette`] where the branch wrote them: 33 bytes inserted there move the offset of
    /// `decay_stamp`, `timing`, `dot` and every other hot field, which is `docs/progress/MF-2.md`
    /// step 0's trap in `Ppu2C02::run_dots`. The copy below costs the same wherever they sit.
    /// They are the *frame's last* palette, unrelated to [`Self::shown_palette_entry`], which is
    /// the VS. System remap of one written byte.
    frame_palette: [u8; PALETTE_SIZE],
    frame_mask: u8,
    /// The emphasis bits (PPUMASK 5-7) the frame being drawn started with, and the writes that
    /// changed them since, by first pixel reached (`emphasis.rs`, lot FIDPLUS-D2b). Not saved,
    /// like the picture they go with. Here at the cold end for the reason `frame_palette` gives.
    emphasis_start: u8,
    emphasis_log: Vec<(u32, u8)>,
    /// The colour subcarrier's phase (0-11, in half master clocks) at the start of the frame
    /// being drawn, for the composite decoder (`crate::composite`, lot FIDPLUS-D2b). Not saved:
    /// a picture property, and the phase a console starts at is arbitrary.
    video_phase: u8,
}

impl Default for Ppu2C02 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu2C02 {
    /// Creates a PPU; the core calls [`Ppu2C02::reset`] before running it.
    ///
    /// Lot KIRBY-MERGE: both sides' new fields took the body to 101 lines of clippy's 100, as
    /// LUA-MERGE's `EmulationHost::new` was taken past it. There a tail could be given to another
    /// function; here the body is a **single struct literal**, one line per field, so the only way
    /// to shorten it is to break `Ppu2C02` into sub-structs — which moves the field offsets the
    /// mapper lots measured (`docs/progress/MF-2.md` step 0). Hence the allow, in the idiom
    /// `cpu::opcode_table` already uses for a flat list.
    #[must_use]
    #[allow(clippy::too_many_lines)] // One line per field, kept as a flat literal.
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_SIZE],
            vram_written: vec![0; VRAM_SIZE],
            vram_clock: 0,
            oam: [0; OAM_SIZE],
            oam_corruption: true,
            oam_corrupt_rows: 0,
            oam_seed_row: 0,
            oamaddr_bug: true,
            last_data_write: (i64::MIN, 0),
            copy_257_suppressed: false,
            fid: Box::new(edges::FidState::NEW),
            #[cfg(feature = "ppu-dot-trace")]
            dot_trace: None,
            #[cfg(feature = "ppu-dot-trace")]
            dot_trace_bus: std::cell::Cell::new(0xFFFF),
            oam_decay: false,
            power_on_top: true,
            reset_line: false,
            ale_low: 0,
            pre_g_revision: false,
            oamaddr_write_corruption: false,
            after_reset: false,
            phase_offset: 0,
            oam_row_refresh: [0; 32],
            oam_render_off: 0,
            bus_conflicts: true,
            address_load_delay: true,
            glitch_2006: true,
            glitch_2000: true,
            palette_corruption: true,
            alignment: 3,
            cpu_open_bus: 0,
            prepaid_dots: 0,
            pending_backdrop_pixel: None,
            serial_since: 0,
            serial_base: 0,
            sprite_units: sprite_hw::SpriteUnits::default(),
            fetch_line: -1,
            fetch_missed: 0,
            fetch_off_dot: -1,
            oam2_overflowed: false,
            palette_raw: [0; PALETTE_SIZE],
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
            #[cfg(feature = "ppu-exact-sprite-eval")]
            secondary_oam_addr: 0,
            #[cfg(feature = "ppu-exact-sprite-eval")]
            eval_copy_dot: -1,
            #[cfg(feature = "ppu-exact-sprite-eval")]
            sprite_in_range: false,
            eval_found: 0,
            overflow_copy: 0,
            eval_finished: false,
            sprite_zero_found: false,
            sprites_loaded: 0,
            sprite_zero_loaded: false,
            sprite_line: [0; SCREEN_WIDTH],
            sprite_line_stale: true,
            skip_pixels: false,
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
            playfield_end: None,
            wants_playfield_end: false,
            line_scroll: Vec::new(),
            tracks_line_scroll: false,
            tinted_sprites: 0,
            margin_palette: None,
            frame_palette: [0; PALETTE_SIZE],
            frame_mask: 0,
            emphasis_start: 0,
            emphasis_log: Vec::new(),
            video_phase: 0,
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
    /// Turns the emulation of the OAM row corruption on or off (see
    /// [`Self::rendering_switched`]).
    /// Turns the 2C02's OAMADDR copy at the start of rendering on or off (see
    /// [`Self::oamaddr_row_copy`]).
    pub const fn set_oamaddr_bug(&mut self, enabled: bool) {
        self.oamaddr_bug = enabled;
    }

    /// The 2C02G/H's OAMADDR defect: "if OAMADDR is not less than eight when rendering starts,
    /// the eight bytes starting at OAMADDR & 0xF8 are copied to the first eight bytes of OAM"
    /// (nesdev, "PPU registers", OAMADDR). On the 2C03, 2C04, 2C05 and 2C07 "OAMADDR works as
    /// intended", so a PAL console does not do it; the Dendy does, "required for 2C02
    /// compatibility". Rendering starts at dot 1 of the pre-render line, once a frame: this costs
    /// the dots nothing. `BeesNES` does the same (`CPpu2C0X::PreRenderOamGlitches`, MIT).
    #[cold]
    #[inline(never)]
    fn oamaddr_row_copy(&mut self) {
        let row = usize::from(self.oam_address & 0xF8);
        self.oam.copy_within(row..row + 8, 0);
    }

    /// Turns the decay of OAM with the rendering off on or off (see [`Self::refresh_oam_row`]).
    pub fn set_oam_decay(&mut self, enabled: bool) {
        self.oam_decay = enabled;
        self.oam_row_refresh = [self.ppu_clock; 32];
        self.oam_render_off = self.ppu_clock;
    }

    /// Stamps every OAM row as refreshed now: after a state load, whose clock the stamps did not
    /// travel with.
    pub(super) fn restamp_oam_rows(&mut self) {
        self.oam_row_refresh = [self.ppu_clock; 32];
        self.oam_render_off = self.ppu_clock;
    }

    /// The PPU dots in a second, or `None` on the 2C07, which refreshes OAM itself while the
    /// rendering is off (nesdev, "PPU OAM": the PAL PPU "refreshes OAM" in its long vertical
    /// blank, and "OAM decay does not occur" on it).
    const fn oam_dot_rate(&self) -> Option<i64> {
        match self.timing.region {
            NesRegion::Pal => None,
            NesRegion::Dendy => Some(5_320_342),
            _ => Some(5_369_318),
        }
    }

    /// The microseconds since `last` (a PPU clock stamp), or `None` where OAM does not decay.
    fn oam_elapsed_us(&self, last: i64) -> Option<i64> {
        let rate = self.oam_dot_rate()?;
        Some((self.ppu_clock - last) * 1_000_000 / rate)
    }

    /// OAM is dynamic memory that the sprite evaluation refreshes, eight bytes a row, on every line
    /// the rendering is on; with the rendering off a row decays unless something refreshes it
    /// (nesdev, "PPU OAM": "OAM is a dynamic memory ... if rendering is off, it will decay").
    ///
    /// Lot FIDPLUS-E, speculative model (`docs/review/FIDELITY-LEADS.md`, "OAM decay"). What a
    /// console shows (`FireSide`, 2026-11-15, an original NES): OAM intact for about seven frames,
    /// then single bits lost at random, more the longer the rendering stays off; each bit falls to
    /// its own resting value, 0 or 1; the first eight bytes never decay. Modelled as:
    ///
    /// - every bit has its own retention, drawn once from the losses measured on that console with
    ///   `oamdecay.nes` (2026-11-16, [`OAM_RETENTION_US`]): none before 125 ms (that console lost
    ///   2 bits of 1838 at 7 frames, but blargg's `oam_stress`, which passes on hardware, leaves
    ///   rows unrefreshed for up to ~123 ms and fails below this floor), half by about 50 frames
    ///   (0.85 s), all by 90 (1.5 s);
    /// - every bit has its own resting value ([`oam_bit_rest`]): 0 for 76 %, 1 for 22 %, and 2 %
    ///   lose whatever they held;
    /// - both are fixed for the console, drawn from the bit's index, so a run replays identically;
    /// - the row `OAMADDR` points at is held refreshed (assumed from the first eight bytes, with
    ///   `OAMADDR` at 0 after a frame): it is settled when the address enters it and stamped when
    ///   the address leaves it;
    /// - no decay on the 2C07 ([`Self::oam_dot_rate`]).
    ///
    /// One console at one temperature: retention varies with both, so this is that console, not
    /// every NES. The held row is confirmed by the same run (row 31, where `OAMADDR` was parked,
    /// lost nothing). Called on every CPU access to a row and, for all rows,
    /// when the rendering comes back; the dots themselves do nothing.
    #[cold]
    #[inline(never)]
    fn refresh_oam_row(&mut self, row: usize) {
        if !self.rendering_enabled() && row != usize::from(self.oam_address >> 3) {
            let last = self.oam_row_refresh[row].max(self.oam_render_off);
            if let Some(elapsed) = self.oam_elapsed_us(last) {
                self.decay_oam_row(row, elapsed);
            }
        }
        self.oam_row_refresh[row] = self.ppu_clock;
    }

    /// `OAMADDR` moves to `value` with the decay on: the row it leaves was refreshed until now,
    /// and the row it enters settles what it lost while nothing refreshed it.
    #[cold]
    #[inline(never)]
    fn move_oam_address(&mut self, value: u8) {
        let (from, to) = (usize::from(self.oam_address >> 3), usize::from(value >> 3));
        if from != to && !self.rendering_enabled() {
            self.oam_row_refresh[from] = self.ppu_clock;
            let last = self.oam_row_refresh[to].max(self.oam_render_off);
            if let Some(elapsed) = self.oam_elapsed_us(last) {
                self.decay_oam_row(to, elapsed);
            }
            self.oam_row_refresh[to] = self.ppu_clock;
        }
        self.oam_address = value;
    }

    /// The bits of `row` whose retention is shorter than `elapsed_us` fall to their resting value.
    fn decay_oam_row(&mut self, row: usize, elapsed_us: i64) {
        if elapsed_us <= i64::from(OAM_RETENTION_US[0]) {
            return;
        }
        for index in row * 8..row * 8 + 8 {
            let mut byte = self.oam[index];
            for bit in 0..8 {
                let draw = oam_bit_draw(index * 8 + bit);
                if elapsed_us > i64::from(OAM_RETENTION_US[(draw & 0xFF) as usize]) {
                    let mask = 1u8 << bit;
                    byte = match oam_bit_rest(draw) {
                        OamRest::Zero => byte & !mask,
                        OamRest::One => byte | mask,
                        OamRest::Flips => byte ^ mask,
                    };
                }
            }
            self.oam[index] = if (index & 0x03) == 0x02 { byte & 0xE3 } else { byte };
        }
    }

    /// The rendering comes back: every row is evaluated from here on, so whatever decayed while it
    /// was off is settled now, before the first evaluation reads it.
    #[cold]
    #[inline(never)]
    fn refresh_oam_rows_on(&mut self) {
        let held = usize::from(self.oam_address >> 3);
        for row in 0..32 {
            let last = self.oam_row_refresh[row].max(self.oam_render_off);
            if row != held {
                if let Some(elapsed) = self.oam_elapsed_us(last) {
                    self.decay_oam_row(row, elapsed);
                }
            }
            self.oam_row_refresh[row] = self.ppu_clock;
        }
    }

    pub fn set_oam_corruption(&mut self, enabled: bool) {
        self.oam_corruption = enabled;
        if !enabled {
            self.oam_corrupt_rows = 0;
        }
    }

    /// A `$2001` write that turns the rendering on or off.
    ///
    /// **The switch takes effect two dots after the write.** `AccuracyCoin` measures it from
    /// several sides — `OAM Corruption` 2 ("if rendering is disabled on dot 9 (2 cycle delay) row 4
    /// will become corrupt"; three dots on two of the four CPU/PPU alignments), `Misaligned OAM2
    /// Address` 4 and `Frozen OAM2 Increment` 2, which turns the rendering back on "on dot 256"
    /// so that the vertical increment of that dot does **not** happen — and Mesen 2 has the same
    /// delay ("rendering enabled flag is apparently set with a 1 cycle delay").
    ///
    /// The two dots are run here, at once, under the old mask, and owed back to the next catch-up
    /// ([`Self::prepaid_dots`]): nothing is tested per dot. Only on a line that renders; elsewhere
    /// the switch has nothing to be late for.
    #[cold]
    #[inline(never)]
    fn switch_rendering<K: BoardKind>(&mut self, value: u8, cart: Cart<'_, '_, K>) {
        let renders =
            self.scanline < VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline();
        // Two dots in every alignment: the netlist with the CPU's hold time (step 33) stops and
        // starts the fetches and `v`'s increments on the third dot after the write in all four;
        // TriCNES's third dot in alignment 2 does not appear there. The PAL 2C07 takes four
        // (Visual 2C07 netlist, lot FIDPLUS-C C3c, `g2001px`: a switch on written on dot 12 leaves
        // dot 16's increment undone and fetches again from dot 17; 8 170 differing dots a frame
        // with four, 54 000 to 97 000 with two, three or five).
        // On PAL three on the cycles whose dot accumulator is 0, 1 or 2, four on 3 and 4: the
        // split of the `$2007` read's latch (Visual 2C07 netlist, `oam` and `g2001px` at the four
        // reset phases e0/e3/e6/e9, which between them switch on and off on every accumulator:
        // no differing run longer than 2 dots in any aligned frame; the earlier {0, 2} -> 3 left
        // 58 311 at e0, switch on with accumulator 1, and 371 915 at e9, switch off with 1).
        // With the 2A07's M2 duty (19/32, bench `benchdotpal57`, adversarial review 2026-11-16):
        // four on accumulator 4 only (`oam`, `g2001px` at the four phases, 0 long runs; the
        // 15/24 bench's {3, 4} split leaves 890 605 differing dots there).
        let delay = if self.timing.region == NesRegion::Pal {
            if self.dot_accumulator == 4 {
                4
            } else {
                3
            }
        } else {
            2
        };
        #[cfg(feature = "ppu-dot-trace")]
        let delay = pal_switch_override(self.timing.region, self.dot_accumulator, value, delay);
        let corrupts = (value & 0x18) == 0 && self.switch_off_corrupts_palettes();
        if renders && self.prepaid_dots == 0 {
            if corrupts {
                // It strikes the pixel the next dot draws, rendering still on.
                let color = self.next_pixel_color_index();
                self.corrupt_palettes(color);
            }
            self.run_dots(i64::from(delay), cart);
            self.prepaid_dots = delay;
        } else if corrupts {
            // Line 240 or a pre-render line already run: the next pixel is line 0's first,
            // drawn with rendering off, colour index 0.
            self.corrupt_palettes(0);
        }
        let ran_ahead = renders && self.prepaid_dots == delay;
        let write_dot = self.dot - if ran_ahead { delay } else { 0 };
        // A switch on dot 256 (written on 254): the horizontal copy of dot 257 still follows the
        // old state — the netlist copies after a switch off and does not after a switch on, in
        // alignments 2 and 3 (A5; alignments 0 and 1 hit the switch-on race of step 33). Both are
        // settled here, on the write, so that nothing is tested per dot.
        // The 2C07 does the same with its four dots (a write on dot 252 switches on dot 257).
        let late_switch = ran_ahead
            && self.dot == 256
            && self.alignment >= 2
            && (self.netlist_measured_region() || self.timing.region == NesRegion::Pal);
        let was_rendering = self.rendering_enabled();
        self.mask = value;
        if late_switch {
            let now_rendering = self.rendering_enabled();
            if was_rendering && !now_rendering {
                // Nothing reads `v` on dot 257 with rendering off: copying now is the same.
                self.transfer_address_x();
            } else if !was_rendering && now_rendering {
                self.copy_257_suppressed = true;
            }
        }
        let on = (value & 0x18) != 0;
        self.rendering_switched(on);
        if ran_ahead {
            self.switch_pixel_edge(on, write_dot);
        }
        if ran_ahead && renders {
            if on {
                self.first_fetch_on(self.dot, cart);
                self.skip_background_reload(cart);
            } else {
                self.note_last_ale(self.dot);
            }
        }
    }

    /// Takes the latch's last address when the rendering stops (step 39). The netlist with the
    /// CPU's hold time (`benchhold`, `aleoff.py`) shows no ALE at all while the rendering is off,
    /// and a switch off on dot `h` leaves the ALE of `h` itself when `h` is odd (it comes before
    /// the switch within the dot, in every phase), else that of `h - 1`. In this core's dots the
    /// ALE of the fetch read on odd dot `z` is the netlist's dot `z`, and `first_off` is the
    /// netlist's `h`. The address is worked out from the state, which nothing has moved since.
    #[cold]
    #[inline(never)]
    fn note_last_ale(&mut self, first_off: i32) {
        let dot = first_off - (1 - (first_off & 1));
        if let Some(address) = self.fetch_address_at(dot) {
            self.ale_low = address.to_le_bytes()[0];
        }
    }

    /// The address the fetch read on odd dot `dot` of a line that renders puts on the bus, from
    /// the current state; `None` past the line's last fetch.
    fn fetch_address_at(&self, dot: i32) -> Option<u16> {
        let nametable = 0x2000 | (self.v & 0x0FFF);
        Some(match dot {
            1..=256 | 321..=336 => match (dot - 1) & 0x07 {
                0 => nametable,
                2 => self.attribute_address(),
                4 => self.pattern_address(0),
                _ => self.pattern_address(8),
            },
            257..=320 => {
                #[allow(clippy::cast_sign_loss)] // dot is 257..=320 here, so the slot is 0..=7.
                let slot = (((dot - 257) >> 3) & 0x07) as usize;
                match (dot - 257) & 0x07 {
                    0 | 2 => nametable,
                    4 => self.sprite_pattern_address(slot),
                    _ => self.sprite_pattern_address(slot) + 8,
                }
            }
            337..=340 => nametable,
            _ => return None,
        })
    }

    /// The fetch of the dot the rendering comes back on (lot FIDELITY step 39,
    /// `docs/review/FIDELITY-LEADS.md` D5).
    ///
    /// The netlist with the CPU's hold time (`bench2007h`, `onlag.py`, `staleana.py`) releases
    /// the rendering on the dot `D + 2` of a write on `D`, in all four phases: the dot this core
    /// has just run under the old mask, so its fetch was skipped. When that dot is odd it is the
    /// dot of a fetch's ALE, and the hardware does that fetch. In phases a0 and a2 (alignments 1
    /// and 0) the ALE comes after the release within the dot and the fetch is the usual one; in
    /// a4 and a6 (alignments 3 and 2) it came before, with the rendering still off, so the read
    /// of the next dot uses the high byte of the address on the pins and the low byte the octal
    /// latch kept, that of the last ALE ([`Self::ale_low`]). `TriCNES` models the same read,
    /// `(PAR & $FF00) |` its octal latch. The fetch read on odd dot `z` has its ALE on the
    /// netlist's dot `z`, so `first_on` is the dot itself.
    #[cold]
    #[inline(never)]
    fn first_fetch_on<K: BoardKind>(&mut self, first_on: i32, cart: Cart<'_, '_, K>) {
        if first_on & 1 == 0 {
            return;
        }
        let background = matches!(first_on, 3..=255 | 321..=337);
        let sprite = matches!(first_on, 257..=320) && (first_on - 257) & 0x07 >= 4;
        if !background && !sprite {
            return;
        }
        let Some(intended) = self.fetch_address_at(first_on) else {
            return;
        };
        let address = if self.alignment >= 2 {
            (intended & 0xFF00) | u16::from(self.ale_low)
        } else {
            intended
        };
        self.ale_low = intended.to_le_bytes()[0];
        let attribute_shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
        let data = self.read_vram_inner(
            address,
            PpuBusAccess::Render,
            if background {
                ChrFetch::Background
            } else {
                ChrFetch::Sprite
            },
            cart,
        );
        if background {
            match (first_on - 1) & 0x07 {
                0 => self.nametable_latch = data,
                2 => self.attribute_latch = (data >> attribute_shift) & 0x03,
                4 => self.pattern_low_latch = data,
                _ => self.pattern_high_latch = data,
            }
            return;
        }
        #[allow(clippy::cast_sign_loss)] // first_on is 257..=320 here, so the slot is 0..=7.
        let slot = (((first_on - 257) >> 3) & 0x07) as usize;
        if self.fetched_sprite_in_margin(slot) {
            return;
        }
        if (first_on - 257) & 0x07 == 4 {
            self.sprite_pattern_low[slot] = data;
        } else {
            self.sprite_pattern_high[slot] = data;
        }
        self.sprite_line_stale = true;
    }

    /// The pixel the netlist draws differently from the dot the switch acts on (step 33).
    ///
    /// The fetches and `v` follow the switch from the third dot after the write `D` in every
    /// alignment, which this core draws from pixel `D + 2`. The picture does not quite follow:
    /// a switch on still shows the backdrop on pixel `D + 2`, every alignment, the tiles from
    /// `D + 3`; a switch off shows the backdrop from pixel `D + 1` in alignments 0 and 1, whose
    /// write the PPU stores in the first half of its dot. The pixel is set to what rendering off
    /// draws: pixel `D + 1` at once, it is drawn already; pixel `D + 2` when its line is over
    /// ([`Self::process_dot`]), as it is drawn later. Only the colour: the sprite 0 hit and
    /// everything else stay with the dot.
    #[cold]
    #[inline(never)]
    fn switch_pixel_edge(&mut self, on: bool, write_dot: i32) {
        if self.scanline >= VISIBLE_SCANLINES || (!on && self.alignment >= 2) {
            return;
        }
        let x = write_dot + if on { 2 } else { 1 };
        if !(0..SCREEN_WIDTH_I32).contains(&x) {
            return;
        }
        let mut color = self.palette[self.backdrop_entry()];
        if (self.mask & 0x01) != 0 {
            color &= 0x30;
        }
        #[allow(clippy::cast_sign_loss)] // Visible line, 0..256.
        let index = self.scanline as usize * SCREEN_WIDTH + x as usize;
        if on {
            self.pending_backdrop_pixel = Some((index, color));
        } else if let Some(pixel) = self.pixels.get_mut(index) {
            *pixel = color;
        }
    }

    /// The reload of the background shift registers that a rendering just switched back on comes
    /// too late for.
    ///
    /// The registers are reloaded every eight dots, and that reload reads the rendering one dot
    /// later than the rest: switched on for the very dot of a reload, the PPU fetches again but
    /// the registers only shift. `AccuracyCoin`'s `BG Serial In` rests on it — rendering off "just
    /// before the data would be loaded" and on "just after", on an empty nametable, and the 1s the
    /// high plane shifts in must still be there under a sprite zero at X = $92, which they are
    /// only if the reload of dot 137 is lost with those of dots 121 and 129.
    ///
    /// That dot is run here with the two of [`Self::switch_rendering`] and the registers put back
    /// to what the shift alone leaves: nothing is tested per dot.
    #[cold]
    #[inline(never)]
    fn skip_background_reload<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        let next = self.dot + 1;
        if (next & 0x07) != 1 || !matches!(next, 9..=257 | 329 | 337) {
            return;
        }
        let shifted = [
            self.pattern_shift_low << 1,
            (self.pattern_shift_high << 1) | 1,
            self.attribute_shift_low << 1,
            self.attribute_shift_high << 1,
        ];
        let base = self.serial_base;
        self.run_dots(1, cart);
        self.prepaid_dots += 1;
        [
            self.pattern_shift_low,
            self.pattern_shift_high,
            self.attribute_shift_low,
            self.attribute_shift_high,
        ] = shifted;
        // The reload that dot made is taken back; the serial count goes on from before it.
        self.serial_base = (base + 1).min(16);
        self.serial_since = self.ppu_clock;
    }

    /// The 1s the high background plane has shifted in since its last reload, which
    /// `shift_background` leaves out: brought into the register when the rendering comes back.
    ///
    /// The lowest *k* bits of the high plane are always the *k* bits shifted in since the reload,
    /// and the reload every eight dots keeps *k* under eight — out of sight of the pixel, which
    /// reads bits 8 to 15 — on any line that renders without a break. Only a reload that does not
    /// happen lets *k* pass eight, and a reload only fails to happen with the rendering off (or
    /// switched on for its very dot, `skip_background_reload`). So the count is taken when the
    /// rendering goes off and the bits are set when it comes back; nothing is done per dot.
    #[cold]
    #[inline(never)]
    fn serial_ones(&mut self, on: bool) {
        if on {
            let base = self.serial_base;
            if base != 0 {
                self.pattern_shift_high |= if base >= 16 {
                    0xFFFF
                } else {
                    (1u16 << base) - 1
                };
            }
            self.serial_since = self.ppu_clock;
            return;
        }
        let (shifted, reloaded) = self.shift_dots_since(self.serial_since);
        let base = if reloaded { 0 } else { self.serial_base };
        self.serial_base = (base + shifted).min(16);
        self.serial_since = self.ppu_clock;
    }

    /// The dots that shifted the background registers after the clock `since` and up to now, the
    /// rendering having been on all along (dots 2 to 257 and 321 to 337 of a line that renders),
    /// counted back from the current dot to the last reload (dots 9 to 257 by eights, 321, 329
    /// and 337, whose shift comes before the load) and at most 16; `true` with the count when such
    /// a reload came after `since`. A cold-path helper: the reloads record nothing themselves.
    fn shift_dots_since(&self, since: i64) -> (u32, bool) {
        let mut steps = self.ppu_clock - since;
        let (mut line, mut dot) = (self.scanline, self.dot);
        let last_line = self.pre_render_scanline();
        let mut count = 0;
        while steps > 0 {
            if line < VISIBLE_SCANLINES || line == last_line {
                if matches!(dot, 9..=257 | 321 | 329 | 337) && (dot - 1).trailing_zeros() >= 3 {
                    return (count, true);
                }
                if matches!(dot, 2..=257 | 321..=337) {
                    count = (count + 1).min(16);
                }
            }
            steps -= 1;
            dot -= 1;
            if dot < 0 {
                dot = 340;
                line = if line == 0 { last_line } else { line - 1 };
            }
        }
        (count, false)
    }

    /// Turns the fetches disturbed by a colliding `$2007` read or `$2006` write on or off (see
    /// [`Self::read_buffer_from_fetch`] and [`Self::delay_address_load`]).
    pub const fn set_bus_conflicts(&mut self, enabled: bool) {
        self.bus_conflicts = enabled;
    }

    /// See `address_load_delay`.
    pub const fn set_address_load_delay(&mut self, enabled: bool) {
        self.address_load_delay = enabled;
    }

    /// See `glitch_2006`.
    pub const fn set_glitch_2006(&mut self, enabled: bool) {
        self.glitch_2006 = enabled;
    }

    /// See `alignment`; values above 3 are taken modulo 4.
    pub const fn set_cpu_ppu_alignment(&mut self, alignment: u8) {
        self.alignment = alignment & 3;
    }

    /// See `palette_corruption`.
    pub const fn set_palette_corruption(&mut self, enabled: bool) {
        self.palette_corruption = enabled;
    }

    /// See `glitch_2000`.
    pub const fn set_glitch_2000(&mut self, enabled: bool) {
        self.glitch_2000 = enabled;
    }

    /// The CPU data bus as it stood before the write the bus is about to make (see
    /// `cpu_open_bus`).
    pub const fn note_cpu_open_bus(&mut self, value: u8) {
        self.cpu_open_bus = value;
    }

    /// What switching the rendering off or on inside a frame does to OAM, `on` being the new
    /// state.
    ///
    /// "If rendering is disabled during a visible scanline, OAM is going to be corrupted on the
    /// next visible pixel. 8 bytes of OAM get replaced with the first 8 bytes of OAM. The 8 bytes
    /// that get replaced correlate to the secondary OAM address at the moment rendering was
    /// disabled" (`AccuracyCoin`, `OAM Corruption`), two or three dots after the write depending
    /// on the CPU/PPU alignment — two here. The row is that address: dot / 2 during the clear,
    /// the secondary OAM address rounded up to a sprite during the evaluation, and the byte the
    /// fetch stands on from dot 257 (Mesen 2, `NesPpu::SetOamCorruptionFlags`, whose note says the
    /// model "still [is] pending more research" — which is why this is a setting). Row 0 copied over itself changes nothing.
    ///
    /// Reached from a `$2001` write that changes the rendering bits and from nowhere else, so it
    /// costs the dots nothing.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[cold]
    #[inline(never)]
    fn rendering_switched(&mut self, on: bool) {
        let renders =
            self.scanline < VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline();
        if renders {
            self.note_fetch_pause(on);
            self.note_oam2_overflow(on);
        }
        self.serial_ones(on);
        self.note_sprite_switch(on, renders);
        if self.oam_decay {
            if on {
                self.refresh_oam_rows_on();
            } else {
                self.oam_render_off = self.ppu_clock;
            }
        }
        if !self.oam_corruption {
            return;
        }
        if on {
            if renders {
                if matches!(self.alignment, 1 | 2) {
                    // Back on outside vblank: the pending seed is lost in these alignments
                    // (TriCNES); only what was copied at the switch itself remains.
                    self.oam_corrupt_rows = 0;
                } else {
                    self.corrupt_oam_rows();
                }
            }
            return;
        }
        if !renders {
            return;
        }

        // The seed is taken one dot later in alignments 1 and 2 than the switch itself (TriCNES:
        // 3 dots after the write, the rendering 2; AccuracyCoin's `OAM Corruption`, measured on
        // consoles: "three dots on two of the four alignments").
        let dot = self.dot + i32::from(matches!(self.alignment, 1 | 2));
        // TriCNES (`docs/review/FIDELITY-LEADS.md`, D1), measured against it dot by dot: during
        // the clear, a switch whose seed lands on an even dot (on every dot in alignment 2) also
        // corrupts the row before — the seed taken at once while the rendering still counts as
        // on, besides the one taken when it stops; and in alignment 1 the sprite fetches seed the
        // row after, their increment coming before it is used.
        // During the evaluation the row is the secondary OAM address rounded up to a sprite —
        // the address before this dot's increment on an even dot (on every dot in alignment 2),
        // and in alignment 2 the row after it as well on an even dot. Dot 256 seeds row 0.
        // In alignment 2, and on an even dot in alignment 1, the evaluation row is copied at
        // once (the evaluation still sees the rendering on), so a re-enable outside vblank, which
        // drops the pending seed in these alignments, keeps it; measured against TriCNES with a
        // re-enable ten lines on (lot FIDELITY step 29).
        // TriCNES also copies the clear's extra row at once, but then fails AccuracyCoin's own
        // `OAM Corruption` test 3 (OAM intact right after the switch, measured on a console in
        // every alignment) in alignment 2: the console wins, that row waits like the main one.
        let mut rows = 0u32;
        let mut deferred = 0u32;
        let mut seed = None;
        let even = dot % 2 == 0;
        #[allow(clippy::cast_sign_loss)] // Each arm is 0..32 by its range.
        let row = match dot {
            0 => 1,
            1..=64 => {
                let row = (dot >> 1) as u32 & 0x1F;
                if even || (self.alignment == 2 && dot > 1) {
                    deferred |= 1 << ((row + 31) & 0x1F);
                }
                row
            }
            65 => {
                // The clear's last step marks the corruption without taking a row: the one the
                // previous switch-off seeded is copied at once.
                rows |= 1 << self.oam_seed_row;
                seed = Some(self.oam_seed_row);
                0
            }
            66..=255 => {
                // Alignment 1, which seeds a dot late, always reads the address as it stands.
                // Alignment 2 seeds a dot late too, but TriCNES takes its seed after that dot
                // has run with the rendering on: the copy of the dot it seeds on is foreseen
                // (the evaluation copies on every even dot of a sprite it copies).
                let foreseen = self.alignment == 2 && even && self.copied_within(dot - 2, 1);
                let behind = if self.alignment == 2 {
                    !foreseen
                } else {
                    even && self.alignment != 1
                };
                let row = self.evaluation_row(behind);
                match self.alignment {
                    2 => {
                        rows |= 1 << row;
                        seed = Some(row);
                        if foreseen {
                            (row + 1) & 0x1F
                        } else {
                            0
                        }
                    }
                    1 if self.dot % 2 == 0 => {
                        rows |= 1 << row;
                        seed = Some(row);
                        0
                    }
                    _ => row,
                }
            }
            256 if self.alignment == 2 => {
                let row = self.evaluation_row(true);
                rows |= 1 << row;
                seed = Some(row);
                0
            }
            257..=320 => {
                let since = dot - 256 + i32::from(self.alignment == 1);
                if (dot == 257 && self.alignment == 1) || (dot == 258 && self.alignment == 2) {
                    rows |= 1 << 1;
                }
                if since > 64 {
                    // Alignment 1's last fetch dot seeds as the dots after the fetches do.
                    0
                } else {
                    (((since >> 3) * 4) + (since & 0x07).min(3)) as u32 & 0x1F
                }
            }
            340 if self.alignment == 1 => 1,
            _ => 0,
        };
        // The extra rows come from the seed taken while the rendering still counts as on, and
        // that one is copied at once; the main row waits for the rendering to come back.
        self.copy_row_zero_over(rows);
        self.oam_corrupt_rows |= deferred | (1 << row);
        self.oam_seed_row = seed.unwrap_or(row);
    }

    /// Keeps count of the steps the secondary OAM address misses when the rendering is off for part
    /// of a line's sprite fetches.
    ///
    /// From dot 257 the address climbs four times in every eight dots — on the first three, which
    /// read the sprite's Y, tile and attribute, and on the last — and it only climbs on a dot that
    /// renders. A line whose fetches all ran leaves it at 32, which is 0 again; a line that lost
    /// some leaves it short, and that is where a `$2004` read of dots 321 to 340 then looks
    /// (`AccuracyCoin`, `Misaligned OAM2 Address` 4: off from dot 268 to dot 285, eight steps
    /// lost, `$06` read from index `$18`). The write takes effect two dots on, as for the row
    /// corruption above.
    fn note_fetch_pause(&mut self, on: bool) {
        let line = self.ppu_clock - i64::from(self.dot);
        let dot = self.dot;
        if !on {
            if self.fetch_line != line {
                self.fetch_line = line;
                self.fetch_missed = 0;
            }
            self.fetch_off_dot = dot;
        } else if self.fetch_line == line && self.fetch_off_dot >= 0 {
            let first = self.fetch_off_dot.max(257);
            let last = (dot - 1).min(320);
            let mut missed = 0;
            for step in first..=last {
                if matches!((step - 257) & 0x07, 0 | 1 | 2 | 7) {
                    missed += 1;
                }
            }
            self.fetch_missed += missed;
            self.fetch_off_dot = -1;
        }
    }

    /// The secondary OAM's "overflowed" flag across a rendering switched off and on.
    ///
    /// The flag is raised whenever the secondary OAM address overflows — the evaluation filling
    /// it, and every line's last fetch at dot 321 — and "is cleared if rendering is enabled during
    /// dots 63, 255, and 339"; while it is up the address cannot move. With the rendering on all
    /// along it is always down again before it could matter, so it is only worked out here, at the
    /// two moments that can leave it up: it is read when the rendering goes off, and if the
    /// rendering comes back after dot 255 with it still up, "the sprite fetch stage will
    /// continuously read from index 0 of OAM2 for every object" (`AccuracyCoin`, `Frozen OAM2
    /// Increment` 2 to 4) — which is the secondary OAM holding that one byte everywhere, for the
    /// fetches of this line; the next line's clear rebuilds it.
    fn note_oam2_overflow(&mut self, on: bool) {
        let dot = self.dot;
        if !on {
            self.oam2_overflowed = match dot {
                321..=338 => true,
                64..=254 => self.secondary_oam_full(),
                _ => false,
            };
        } else if std::mem::take(&mut self.oam2_overflowed) && (256..=320).contains(&dot) {
            let first = self.secondary_oam[0];
            self.secondary_oam.fill(first);
        }
    }

    /// Whether the evaluation has filled the secondary OAM.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    const fn secondary_oam_full(&self) -> bool {
        self.secondary_oam_addr >= 32
    }

    /// Without the exact evaluation, eight sprites found is the same statement.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    const fn secondary_oam_full(&self) -> bool {
        self.eval_found >= 8
    }

    /// The row [`Self::rendering_switched`] marks during the evaluation: the secondary OAM
    /// address "ceilinged to the nearest multiple of 4", and 0 once the secondary OAM is full;
    /// `behind` takes the address one step back (the value before this dot's increment, which is
    /// the same rounding when nothing was copied).
    #[cfg(feature = "ppu-exact-sprite-eval")]
    #[allow(clippy::cast_sign_loss)] // 0..=32 before the mask.
    const fn evaluation_row(&self, behind: bool) -> u32 {
        let address = if behind && self.secondary_oam_addr > 0 {
            self.secondary_oam_addr - 1
        } else {
            self.secondary_oam_addr
        };
        ((address + 3) & !3) as u32 & 0x1F
    }

    /// Without the exact evaluation there is no secondary OAM address to read; row 0 is the one
    /// that does nothing.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    const fn evaluation_row(&self, _behind: bool) -> u32 {
        0
    }

    /// Whether the evaluation copied a byte within the `window` dots up to `dot`.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    const fn copied_within(&self, dot: i32, window: i32) -> bool {
        self.eval_copy_dot <= dot && dot - self.eval_copy_dot < window
    }

    /// Without the exact evaluation nothing is copied dot by dot.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    const fn copied_within(&self, _dot: i32, _window: i32) -> bool {
        false
    }

    /// Copies OAM row 0 over every row [`Self::rendering_switched`] marked, and the first byte of
    /// the secondary OAM over the byte of the same index.
    #[cold]
    #[inline(never)]
    fn corrupt_oam_rows(&mut self) {
        let rows = std::mem::take(&mut self.oam_corrupt_rows);
        self.copy_row_zero_over(rows);
    }

    /// Copies OAM row 0 over every row of `rows` (bit n = row n), and the first byte of the
    /// secondary OAM over the byte of the same index.
    fn copy_row_zero_over(&mut self, rows: u32) {
        for row in 1..32usize {
            if (rows >> row) & 1 != 0 {
                self.oam.copy_within(0..8, row * 8);
                self.secondary_oam[row] = self.secondary_oam[0];
            }
        }
    }

    pub fn set_sprite_limit(&mut self, limit: SpriteLimit) {
        self.sprite_limit = limit;
    }

    /// Current dot inside the scanline, 0 to 340.
    #[must_use]
    pub const fn dot(&self) -> i32 {
        self.dot
    }

    /// The current VRAM address `v` (host and test accessor, no side effect).
    #[must_use]
    pub const fn vram_v(&self) -> u16 {
        self.v
    }

    /// The temporary VRAM address `t` (host and test accessor, no side effect).
    #[must_use]
    pub const fn vram_t(&self) -> u16 {
        self.t
    }

    /// Current scanline; 0 to 239 are visible, the last one is the pre-render line.
    #[must_use]
    pub const fn scanline(&self) -> i32 {
        self.scanline
    }

    /// Lot MAPPERS-P (mapper 532): the picture, lent to the font overlay and given back with
    /// [`Self::restore_pixels`].
    #[cfg(feature = "board-532")]
    pub(crate) const fn set_skip_pixels(&mut self, skip: bool) {
        self.skip_pixels = skip;
    }

    pub(crate) fn take_pixels(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pixels)
    }

    /// Gives the picture back after [`Self::take_pixels`].
    #[cfg(feature = "board-532")]
    pub(crate) fn restore_pixels(&mut self, pixels: Vec<u8>) {
        self.pixels = pixels;
    }

    /// The palette RAM's colours as the picture holds them, and as written (`$3F00-$3F1F`).
    #[cfg(feature = "board-532")]
    pub(crate) const fn palette_colors(&self) -> ([u8; PALETTE_SIZE], [u8; PALETTE_SIZE]) {
        (self.palette, self.palette_raw)
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

    /// The nametable address the last background fetch of a frame read, for a board restored
    /// from a state: a state is written at a frame boundary, where `v` has not moved since the
    /// discarded fetch of line 239's dot 339. `None` with the rendering off, which made none.
    pub(crate) const fn last_nametable_read(&self) -> Option<u16> {
        if self.rendering_enabled() {
            Some(0x2000 | (self.v & 0x0FFF))
        } else {
            None
        }
    }

    /// Whether the PPU is actually fetching: rendering enabled **and** on a line that fetches —
    /// a visible one or the pre-render line.
    ///
    /// This is the condition the `$2004` register quirk of `AccuracyCoin`'s `Address $2004
    /// behavior` turns on. The pre-render line counts, and the ROM says so against its own test's
    /// title: `Address $2004 behavior` 6 is called "writing to `$2004` on a visible scanline" and
    /// its code notes that "the CPU write occurs on **dot 321 of the pre-render line**".
    ///
    /// It is also, **word for word**, the condition the nesdev "PPU scrolling" page states for
    /// `$2007`'s odd increment: "on the pre-render line and the visible lines 0-239, provided
    /// either background or sprite rendering is enabled". The two quirks share this predicate
    /// **in the build that has the `$2007` rule**, as they did before lot MF-4d-fix; that rule is
    /// behind `ppu-2007-render-read` and out of `default` since lot MF-4e-gate, so in the default
    /// build this predicate serves the `$2004` quirk alone. See [`Self::increment_vram_address`],
    /// Q-125, Q-137 and Q-139.
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

    /// Object attribute memory, writable: for tests and studies that set OAM directly (the OAM
    /// corruption probe of lot FIDELITY step 29), never for the emulation itself.
    #[doc(hidden)]
    pub const fn oam_mut(&mut self) -> &mut [u8; OAM_SIZE] {
        &mut self.oam
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

    /// Fine Y of the address the row of the **current** line would be captured from — the
    /// `v_row_start` of AMD-021 R2-005, which is `v` shorn of the two end-of-line reloads and of
    /// the increment a `$2007` access borrows from the fetch pipeline (Q-137).
    ///
    /// It is the field [`Ppu2C02::capture_background_row`] tests against 1 to decide whether a row
    /// is emitted at all, so it is the one observable that separates MF-4e's rule from the one
    /// MF-4d shipped — and, since lot MF-4e-gate, the one that says which of the two builds is
    /// running: with `ppu-2007-render-read` off this field follows `v` through a `$2007` access on
    /// every line, which is `11407930`'s behaviour (Q-139).
    #[must_use]
    pub const fn captured_row_fine_y(&self) -> i32 {
        ((self.v_row_start >> 12) & 0x07) as i32
    }

    /// Fine Y of the address that becomes the **next** line's captured row — the `v_row_next` of
    /// AMD-021 R2-005, which the two end-of-line reloads and the increment a `$2007` access
    /// borrows from the fetch pipeline all do reach (Q-137).
    ///
    /// Together with [`Self::captured_row_fine_y`] it is the pair that decides *which* line emits
    /// a row, and the two are what separates MF-4e's rule from the three variants it rejected —
    /// and, with `ppu-2007-render-read` off, what shows that both fields follow `v` again (Q-139).
    #[must_use]
    pub const fn next_row_fine_y(&self) -> i32 {
        ((self.v_row_next >> 12) & 0x07) as i32
    }

    /// The margin decoder's own background palette for the frame just run, if any (the decoded
    /// tiles in sub-palettes 4-7).
    #[must_use]
    pub const fn margin_palette(&self) -> Option<[u8; 16]> {
        self.margin_palette
    }

    /// Shows palette RAM through a VS. System PPU's master-palette remap (lot MF-VS step 2).
    ///
    /// **The PPU holds no VS. System state since lot MAPPERS-N** (owner's go-ahead,
    /// 2026-11-15): its four fields — this remap, the two bytes of an RC2C05's `$2002`
    /// identifier and the `$2000`/`$2001` swap — changed `Ppu2C02`'s layout, and with it the code
    /// of `run_dots` for every game: callgrind of clean builds, Mega Man 2 and Zelda II (MMC1),
    /// `core` **+0.090 %** / **+0.103 %** with `board-vs-system` on, all of it in
    /// `run_dots::<Mmc1Kind>`, two adjacent bytes of the tile pipeline no longer loaded by one
    /// instruction. `NesBus` now applies the three quirks from the `VsSystem` it already holds,
    /// and calls this after every palette write of a VS image, the power-on fill and a state
    /// load: 32 table lookups where a game writes a few palette bytes a frame.
    #[cfg(feature = "board-vs-system")]
    pub fn remap_palette(&mut self, lut: &[u8; crate::vs::LUT_LEN]) {
        for index in 0..PALETTE_SIZE {
            self.palette[index] = lut[usize::from(self.palette_raw[index] & 0x3F)];
        }
        self.palette_marked = marked_palette(&self.palette);
    }

    /// The bits of a `$2002` read that are **not** the three flags: the PPU's own bus latch on a
    /// 2C02 (`ppu_open_bus` tests 6 and 7), and an RC2C05's identifier instead of it.
    ///
    /// Two field reads, an `and` and an `or`, with no branch and no model to test: on every PPU
    /// but an RC2C05 the mask is [`STATUS_LATCH_BITS`] and the identifier 0, so the value is the
    /// `decay & 0x1F` this register always answered.
    /// Without the VS. System there is no identifier and this is the expression the register
    /// always had.
    #[inline]
    const fn status_open_bus(&self) -> u8 {
        self.decay & STATUS_LATCH_BITS
    }

    /// The `$2000` write: PPUCTRL, the two nametable bits of `t`, and the cartridge's hook.
    ///
    /// Its own function only so that the RC2C05's swap can reach it from the `$2001` arm as
    /// well; `#[inline]`, so the `$2000` arm is the body it always was.
    #[inline]
    fn write_control<K: BoardKind>(&mut self, value: u8, cartridge: Option<&mut Cartridge>) {
        // Alignment 3 only, on dot 257: the netlist with the CPU's hold time shows it there and
        // nowhere else (step 33); TriCNES's second case, alignment 0 on dot 258, does not appear.
        // The PAL 2C07 has none, at any of its five CPU/PPU phases (Visual 2C07 netlist, lot
        // FIDPLUS-C C3c, `g2000`: 23 322 differing dots without it, 31 212 to 37 140 with it).
        if self.alignment == 3
            && self.dot == 257
            && self.glitch_2000
            && self.timing.region != NesRegion::Pal
            && self.fetching()
        {
            self.control_write_glitch();
        }
        self.control = value;
        self.t = (self.t & 0xF3FF) | (u16::from(value & 0x03) << 10);
        if self.dot == 304 && self.netlist_measured_region() {
            self.control_write_on_vertical_copy(value);
        }
        if let Some(cartridge) = cartridge {
            cartridge.on_ppu_control_as::<K>(value);
        }
    }

    /// The colour index the PPU shows for the index `written` into palette RAM: the VS. System's
    /// master-palette remap, and the identity everywhere else.
    ///
    /// `written` is already six bits ([`Self::write_vram_inner`] masks it), so the lookup cannot
    /// leave the 64-entry table.
    /// Without the VS. System there is no remap, and this is the identity the optimiser removes.
    /// It keeps the receiver so that the one call site reads the same in both configurations.
    #[allow(clippy::unused_self)]
    const fn shown_palette_entry(&self, written: u8) -> u8 {
        written
    }

    /// Puts power-on contents in the palette RAM, OAM and nametable RAM (after [`Self::reset`],
    /// which clears them): a palette byte keeps its 6 bits, and the four entries `$3F10`,
    /// `$3F14`, `$3F18`, `$3F1C` follow the ones they mirror.
    pub fn fill_power_on_memories(&mut self, palette: &[u8; 32], oam: &[u8; 256], vram: &[u8]) {
        for (index, &byte) in palette.iter().enumerate() {
            self.palette_raw[index] = byte & 0x3F;
        }
        for mirror in [0x10, 0x14, 0x18, 0x1C] {
            self.palette_raw[mirror] = self.palette_raw[mirror - 0x10];
        }
        self.refresh_palette_views();
        self.oam.copy_from_slice(oam);
        let length = vram.len().min(self.vram.len());
        self.vram[..length].copy_from_slice(&vram[..length]);
    }

    /// Rebuilds [`Self::palette`] from [`Self::palette_raw`] and [`Self::palette_marked`] from
    /// that. Called wherever the whole palette is replaced — the power-up fill, a state restore
    /// and the choice of a VS PPU — and never per pixel.
    pub(super) fn refresh_palette_views(&mut self) {
        for index in 0..PALETTE_SIZE {
            self.palette[index] = self.shown_palette_entry(self.palette_raw[index]);
        }
        self.palette_marked = marked_palette(&self.palette);
    }

    /// Returns the 32 palette bytes of the batch, mirrors filled in (REQ-CORE-045).
    ///
    /// The bytes are the ones the PPU *shows*, so a VS. System's master-palette remap is already
    /// in them and the frame batch needs no knowledge of it (lot MF-VS step 2).
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
        self.palette_raw.fill(0);
        self.refresh_palette_views();
        self.decay_stamp.fill(0);
        self.pixels.fill(0);
        self.emphasis_start = 0;
        self.emphasis_log.clear();

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

        self.after_reset = self.reset_line;
        self.place_after_reset();
    }

    /// Where a reset leaves the PPU: the top of the picture ([`Self::power_on_top`]) or the start
    /// of vblank.
    fn place_after_reset(&mut self) {
        if self.power_on_top {
            // nesdev, "PPU power up state": the PPU starts at the top of the picture, not in
            // vblank, so the first `$2002` poll waits most of a frame. Dot 11 is BeesNES's choice
            // (`CPpu2C0X::ResetToKnown`, `LSNPpu2C0X.h:247-256`, MIT): the phase is arbitrary on
            // the hardware, and that one puts blargg's `test_apu_2` polls on the phase its `$4017`
            // writes expect and the OAM corruption of `sprite_overflow_tests/5.Emulator` on a
            // harmless row.
            self.scanline = 0;
            self.dot = 11 + self.phase_offset;
            return;
        }
        // Just after a load or a reset the first frame starts at the beginning of vblank,
        // so the reset lands there (REQ-CORE-020, notes).
        self.scanline = self.timing.vblank_scanline;
        self.dot = 1 + self.phase_offset;
    }

    /// Diagnostic (lot FIDPLUS-C, C3c): starts the dot accumulator at `phase`, so that on PAL
    /// (16 dots for 5 CPU cycles, the cycle of 4 dots every fifth) the 4-dot cycle falls where the
    /// netlist's does. Taken modulo the region's denominator; 0 on NTSC changes nothing.
    pub const fn set_dot_accumulator_phase(&mut self, phase: i32) {
        self.dot_accumulator = phase.rem_euclid(self.timing.dots_per_cycle_denominator);
    }

    /// Study `study/access-phase`: see `phase_offset`.
    pub const fn set_phase_offset(&mut self, dots: i32) {
        self.phase_offset = dots;
    }

    /// Chooses where a power-on puts the PPU (see [`Self::reset`]); takes effect at the next one.
    pub const fn set_power_on_top(&mut self, enabled: bool) {
        self.power_on_top = enabled;
    }

    /// Whether the reset line still shuts `$2000`, `$2001`, `$2005` and `$2006` (see
    /// [`Self::after_reset`]).
    #[must_use]
    pub const fn reset_window_shut(&self) -> bool {
        self.after_reset
    }

    /// Shuts or opens that window by hand: for host tools that fill the PPU's memories through
    /// its registers as if at power-on, and put the window back as they found it.
    pub const fn set_reset_window_shut(&mut self, shut: bool) {
        self.after_reset = shut;
    }

    /// Chooses a 2C02 older than the G revision (see [`Self::pre_g_revision`]).
    pub const fn set_pre_g_revision(&mut self, enabled: bool) {
        self.pre_g_revision = enabled;
    }

    /// Whether a 2C02G's `$2003` writes corrupt OAM (see [`Self::oamaddr_write_corruption`]).
    pub const fn set_oamaddr_write_corruption(&mut self, enabled: bool) {
        self.oamaddr_write_corruption = enabled;
    }

    /// A `$2003` write on a 2C02G: "writes to OAMADDR corrupt OAM", which "usually seems to copy
    /// sprites 8 and 9 (address $20) over the 8-byte row at the target address", the source
    /// "from the previous value on the CPU BUS" (nesdev "PPU registers", D4). The transistor
    /// netlist (a 2C02G) shows that copy in one CPU/PPU phase and the reverse one, from the old
    /// address's row to the bus's, in another: a race; this follows the console description.
    #[cold]
    #[inline(never)]
    fn oamaddr_write_corruption(&mut self, value: u8) {
        let source = usize::from(self.cpu_open_bus & 0xF8);
        let target = usize::from(value & 0xF8);
        if source != target {
            self.oam.copy_within(source..source + 8, target);
        }
    }

    /// Whether resets reach the PPU's /RST pin (see [`Self::after_reset`]); takes effect at the
    /// next reset, except that switching it off opens a write window still shut.
    pub const fn set_reset_line(&mut self, enabled: bool) {
        self.reset_line = enabled;
        if !enabled {
            self.after_reset = false;
        }
    }

    /// The documented *after reset* state of the PPU (AMD-044 REQ-CORE-122): PPUCTRL and
    /// PPUMASK go to 0, the `$2005`/`$2006` write latch is cleared, the scroll register `t`
    /// and the `$2007` read buffer go to 0, and the odd-frame flag goes back to "even"
    /// (nesdev, "PPU power up state").
    ///
    /// Nothing else moves: PPUSTATUS, OAMADDR, the current address `v`, the nametable RAM, the
    /// OAM, the palette RAM and the CHR RAM are all unchanged. `v_row_start` and `v_row_next`
    /// follow `v`, not `t`, so they stay as they are too. With the reset line
    /// ([`Self::reset_line`], off by default; on, the front-loading NES) the PPU also goes back to its
    /// power-on position and ignores writes to PPUCTRL, PPUMASK, PPUSCROLL and PPUADDR until the
    /// pre-render line (the ~29 658-cycle window of Q-078, lot FIDELITY step 34); without it it
    /// keeps clocking from where it was, as on the top-loading NES and the Famicom.
    // Q-078
    pub fn soft_reset(&mut self) {
        if self.reset_line {
            // The Reset button pulls /RST too: the PPU stops at line 0 dot 0 while it is held
            // and starts again from there with the CPU's reset sequence, as at power-on (the
            // transistor netlist, lot FIDELITY step 34), its write window shut until the
            // pre-render line.
            self.after_reset = true;
            self.prepaid_dots = 0;
            self.pending_backdrop_pixel = None;
            self.place_after_reset();
        }
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
    pub fn step_cpu_cycle(&mut self, cartridge: Option<&mut Cartridge>) {
        self.step_cpu_cycle_as::<AnyBoard>(cartridge);
    }

    /// [`Self::step_cpu_cycle`] with the board reached through `K` (lot CORE-2).
    pub fn step_cpu_cycle_as<K: BoardKind>(&mut self, cartridge: Option<&mut Cartridge>) {
        let mut cartridge = CartSlot::<K>::new(cartridge);
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
        let mut first = 0;
        if self.prepaid_dots != 0 {
            first = self.take_prepaid(dots);
            if first > sample_after {
                self.nmi_line_sample = self.nmi_asserted();
            }
        }
        for dot in first..dots {
            if dot == sample_after {
                self.nmi_line_sample = self.nmi_asserted();
            }

            self.step_dot(&mut cartridge);
        }
    }

    /// The dots of the coming `dots` that [`Self::switch_rendering`] already ran.
    #[cold]
    #[inline(never)]
    fn take_prepaid(&mut self, dots: i32) -> i32 {
        let taken = self.prepaid_dots.min(dots);
        self.prepaid_dots -= taken;
        taken
    }

    /// Advances the PPU by the dots of `cycles` CPU cycles, exactly as that many calls of
    /// [`Ppu2C02::step_cpu_cycle`] would.
    ///
    /// Only the NMI sample of the last cycle survives (each cycle overwrites it and nothing
    /// reads it in between), so the dots run in two stretches around that one sample.
    pub fn run_cpu_cycles(&mut self, cycles: u32, cartridge: Option<&mut Cartridge>) {
        self.run_cpu_cycles_as::<AnyBoard>(cycles, cartridge);
    }

    /// [`Self::run_cpu_cycles`] with the board reached through `K` (lot CORE-2).
    pub fn run_cpu_cycles_as<K: BoardKind>(
        &mut self,
        cycles: u32,
        cartridge: Option<&mut Cartridge>,
    ) {
        let mut cartridge = CartSlot::<K>::new(cartridge);
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
        let mut before_sample = dots_before + sample_after;
        let mut after_sample = dots_last - sample_after;
        if self.prepaid_dots != 0 {
            #[allow(clippy::cast_possible_truncation)] // At most the two prepaid dots matter.
            let total = (before_sample + after_sample).min(i64::from(i32::MAX)) as i32;
            let taken = i64::from(self.take_prepaid(total));
            let from_before = taken.min(before_sample);
            before_sample -= from_before;
            after_sample -= taken - from_before;
        }
        self.run_dots(before_sample, &mut cartridge);
        self.nmi_line_sample = self.nmi_asserted();
        self.run_dots(after_sample, &mut cartridge);
    }

    /// Runs `count` dots, skipping in one go the stretches where a dot does nothing but
    /// move the counters (or, on a visible line with rendering off, draw the backdrop).
    fn run_dots<K: BoardKind>(&mut self, count: i64, cart: Cart<'_, '_, K>) {
        if K::FIDELITY_PLUS {
            // Fidélité+ (`edges`): its own loop, which may post an edge from a dot's own work. A
            // constant: the standard copies of this function are `run_dots_standard` alone.
            self.run_dots_with_edges(count, cart);
            return;
        }
        self.run_dots_standard(count, cart);
    }

    /// The standard core's dots: `run_dots`'s own body. The Fidélité+ loop runs its dots through
    /// this function too, one at a time, and through nothing else of the per-dot path: lot
    /// FIDPLUS-A first called `step_dot`, then the spans, from that loop, and every extra caller
    /// made LLVM outline one of them from this loop (callgrind: +2.3 % on `metroid`, and pinning
    /// them inline moved the outlining to their own callees, +3.3 % on `duelito`).
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn run_dots_standard<K: BoardKind>(&mut self, mut count: i64, cart: Cart<'_, '_, K>) {
        #[cfg(feature = "ppu-dot-trace")]
        if self.dot_trace.is_some() {
            while count > 0 {
                self.dot_trace_bus.set(0xFFFF);
                self.step_dot(cart);
                self.record_traced_dot();
                count -= 1;
            }
            return;
        }
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

            if K::FAST {
                let run = self.run_line_end_fast::<K>(count, cart);
                if run > 0 {
                    count -= run;
                    continue;
                }
            }

            let span = self.visible_span_ahead(count);
            if span > 0 {
                let done = if K::FAST {
                    self.run_visible_span_fast::<K>(span, cart)
                } else {
                    self.run_visible_span::<K>(span, cart.as_deref())
                };
                if !done {
                    self.run_visible_span_through_bus(span, cart);
                }
                count -= span;
            } else {
                self.step_dot(cart);
                count -= 1;
            }
        }
    }

    /// Records the dot just run in the per-dot trace, when one is on.
    #[cfg(feature = "ppu-dot-trace")]
    fn record_traced_dot(&mut self) {
        let record = DotRecord {
            #[allow(clippy::cast_possible_truncation)] // Below 312.
            scanline: self.scanline as i16,
            #[allow(clippy::cast_possible_truncation)] // Below 341.
            dot: self.dot as i16,
            v: self.v,
            t: self.t,
            bus: self.dot_trace_bus.get(),
            access: 0,
        };
        if let Some(trace) = self.dot_trace.as_mut() {
            trace.push(record);
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

    fn step_dot<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        self.advance_counters();
        self.process_dot(cart);
    }

    /// Once a dot, on the hottest path: inlined by hand since the module's growth in lot FIDPLUS-C
    /// made the compiler call it out of line (callgrind 2026-11-16: +60 M instructions over
    /// 300 frames of Super Mario Bros. 3, the whole of the core's +0.4 %).
    #[allow(clippy::inline_always)] // measured: out of line it costs the core 0.4 %
    #[inline(always)]
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
            if (self.mask & 0x18) != 0 {
                // The rendered frame's last ALE, the nametable fetch of dot 339 (step 39).
                self.ale_low = self.v.to_le_bytes()[0];
            }
        }

        if self.scanline < self.timing.scanlines_per_frame {
            return;
        }

        self.scanline = 0;
        if self.emphasis_start != (self.mask & 0xE0) || !self.emphasis_log.is_empty() {
            self.restart_emphasis();
        }

        if self.odd_frame && self.timing.skips_dot_on_odd_frames && self.skip_latch {
            // With rendering on, the idle dot (0, 0) is skipped on odd frames, which makes
            // the frame one dot shorter (NTSC only, REQ-CORE-015). The mask is read here,
            // at the dot that is dropped, and not one dot earlier: that single dot is what
            // ppu_vbl_nmi test 10 measures.
            //
            // Q-136 question 3: **a VS. System PPU is treated as a 2C02 here**, and no allowed
            // page says whether an RP2C03, an RP2C04 or an RC2C05 skips the dot. Mesen 2
            // restricts the skip to the 2C02 and gives no reason; the provisional choice is to
            // change nothing, so a lot that settles the question changes this one condition and
            // no structure.
            self.dot = 1;
        }

        self.odd_frame = !self.odd_frame;
        // The frame just begun starts on dot 1 when the odd frame skipped dot 0.
        let phase =
            crate::composite::next_frame_phase(usize::from(self.video_phase), self.dot == 1);
        self.video_phase = u8::try_from(phase).unwrap_or(0);
    }

    fn process_dot<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        if self.dot == SKIP_DECISION_DOT && self.scanline == self.pre_render_scanline() {
            self.skip_latch = self.rendering_enabled();
        }

        if self.dot == 1 {
            if self.scanline == self.timing.vblank_scanline {
                // The frame boundary always fires; the flag itself can be suppressed by a
                // read of $2002 one dot earlier (nesdev, "PPU frame timing").
                self.vblank_just_started = true;
                // Written **inline**, and it has to stay inline (lot KIRBY-MERGE step 4): the
                // body runs once a frame, and putting it behind an `#[inline(never)]` call —
                // the MF-LEG-2 remedy for cold code in a hot function — made every ROM of the
                // PR-1 set **worse**, the set total going from +0.040 % to +0.088 % and
                // `metroid` from +0.049 % to +0.125 %. A 33-byte copy guarded by a test
                // `process_dot` already makes is cheaper than the call that would avoid it.
                self.frame_palette = self.palette;
                self.frame_mask = self.mask;
                if !self.suppress_vblank {
                    self.vblank_flag = true;
                }

                self.suppress_vblank = false;
            } else if self.scanline == self.pre_render_scanline() {
                if self.oam_corrupt_rows != 0 && self.rendering_enabled() {
                    // The rendering came back during vblank: the rows marked are corrupted on the
                    // first dot of the first line that renders.
                    self.corrupt_oam_rows();
                }
                if self.oam_address >= 8
                    && self.oamaddr_bug
                    && self.rendering_enabled()
                    && self.timing.region != NesRegion::Pal
                {
                    self.oamaddr_row_copy();
                }
                self.vblank_flag = false;
                self.sprite_zero_hit = false;
                self.sprite_overflow = false;
                self.suppress_vblank = false;
                self.begin_rendered_frame();
                self.after_reset = false;
            } else if K::WATCHES_BUS
                && self.scanline == VISIBLE_SCANLINES
                && self.rendering_enabled()
            {
                self.post_render_bus(cart);
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
            if let Some((index, color)) = self.pending_backdrop_pixel.take() {
                if let Some(pixel) = self.pixels.get_mut(index) {
                    *pixel = color;
                }
            }
            if self.scanline < VISIBLE_SCANLINES {
                self.end_capture_line(self.scanline, cart);
            }

            self.v_row_start = self.v_row_next;
        }
    }

    // ---------------------------------------------------------------- registers

    /// Reads register `register` (0-7, $2000 + register) with its side effects.
    pub fn read(&mut self, register: u16, cartridge: Option<&mut Cartridge>) -> u8 {
        self.read_as::<AnyBoard>(register, cartridge)
    }

    /// [`Self::read`] with the board reached through `K` (lot CORE-2).
    pub fn read_as<K: BoardKind>(
        &mut self,
        register: u16,
        cartridge: Option<&mut Cartridge>,
    ) -> u8 {
        let mut cartridge = CartSlot::<K>::new(cartridge);
        self.apply_decay();
        #[cfg(feature = "ppu-dot-trace")]
        self.trace_access(0x2000 | u32::from(register & 7));

        match register & 0x07 {
            2 => {
                // Bits 0 to 4 of PPUSTATUS come from the PPU's own bus latch and are not
                // refreshed by the read; bits 5 to 7 are the flags and do refresh it
                // (ppu_open_bus tests 6 and 7).
                let mut status = self.status_open_bus();
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
                if self.oam_decay {
                    self.refresh_oam_row(usize::from(self.oam_address >> 3));
                }
                let value = self.read_oam();
                self.refresh_decay(value, 0xFF);
                value
            }

            7 => self.read_data(&mut cartridge),

            // $2000, $2001, $2003, $2005 and $2006 are write-only: the read returns the
            // PPU's bus latch and does not refresh it (ppu_open_bus tests 2 and 5).
            _ => self.decay,
        }
    }

    /// The `$2007` arm of [`Ppu2C02::read`]: the read buffer, the palette's immediate path, the
    /// refill fetch and the address increment.
    ///
    /// **Out of line, and that is the whole point of it being a function.** It is the only arm of
    /// the register dispatch that calls anything, so it is the only one that needs callee-saved
    /// registers; inlined, LLVM gives up on shrink-wrapping and hoists its
    /// `pushq %r15 / %r14 / %r12 / %rbx / %rax` **above** the dispatch, where every read of every
    /// register pays five pushes and five pops. Callgrind priced that at **+11 instructions per
    /// register read** on `ppu_vbl_nmi` (304 710 calls, 19.2 M against 22.6 M instructions) and
    /// **+0.035 %** over the ten ROMs of PR-1's set whose behaviour had not moved — see Q-125 and
    /// `docs/progress/MF-4d-fix.md` step 5, which measured the cost and left the fix to a lot of
    /// its own. Behind a call, the prologue is back inside this body and `$2002`, `$2004` and the
    /// write-only arms `retq` without touching the stack, which is `d96d73b7`'s shape.
    ///
    /// `#[cold]` is **not** used: it would move the body to a different section and cost the
    /// `$2007`-heavy ROMs (`ppu_read_buffer`) a far branch for no gain — `#[inline(never)]` alone
    /// restores the shrink-wrap.
    ///
    /// **What it still costs, named exactly.** The arm compiles to `movq %rdx, %rsi` and a
    /// `jmp`: a tail call, so the five pushes are the ones [`Self::read`] used to make anyway,
    /// but the one `movq` that moves `cart` down an argument position is paid at `read`'s entry,
    /// and callgrind prices it at **exactly +304 710 instructions on `ppu_vbl_nmi` for exactly
    /// 304 710 calls** — one instruction per *register* read. Giving this function `read`'s own
    /// `(&mut self, register, cart)` signature to keep the arguments in place was built and
    /// **refuted**: the parameter is unused, LLVM drops it from this internal function's own
    /// convention, re-derives `cart` into `%rsi` and emits the same `movq`. The residual is
    /// +0.0068 % on `ppu_vbl_nmi` and +0.0144 % on `ppu_read_buffer` against `fff53974`, against
    /// the +0.161 % the hoisted prologue cost; the rest of the way is a fight with the register
    /// allocator, which `docs/progress/MF-5a.md` records rather than takes.
    #[inline(never)]
    fn read_data<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) -> u8 {
        if self.edges_on_for::<K>() {
            return self.read_data_with_edges(cart);
        }
        let address = self.v & 0x3FFF;
        // A `$2007` access is an ALE of `v` (step 39).
        self.ale_low = address.to_le_bytes()[0];
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
            if self.pre_g_revision {
                // Before the G revision the palette is not readable: the range reads like the
                // rest of the PPU memory, the buffer first.
                value = visible;
                self.refresh_decay(value, 0xFF);
            } else {
                // Palette reads are immediate and only drive six bits; the buffer still
                // takes the mirrored nametable byte "underneath" the palette (nesdev).
                value = (self.palette_raw[palette_index(address)] & self.palette_read_mask())
                    | (self.decay & 0xC0);
                self.refresh_decay(value, 0x3F);
            }
            self.read_buffer = self.read_vram_inner(
                address & 0x2FFF,
                PpuBusAccess::Cpu,
                ChrFetch::Background,
                cart,
            );
        } else {
            value = visible;
            self.refresh_decay(value, 0xFF);
            self.read_buffer =
                self.read_vram_inner(address, PpuBusAccess::Cpu, ChrFetch::Background, cart);
        }

        if self.bus_conflicts
            && (self.fetching() || self.read_latches_on_pre_render())
            && self.read_buffer_from_fetch(cart)
        {
            // The increment was made inside, where the hardware makes it.
            return value;
        }
        self.increment_vram_address(cart);
        value
    }

    /// A `$2007` read on the last dots of the line before the pre-render line, rendering on: its
    /// latch (5 or 6 dots later) lands on the pre-render line, which fetches, so the read takes the
    /// fetching path, X+Y increment and fetched byte, not the blanking +1 (Visual 2C07, `r2007` at
    /// e3 on the 19/32 bench, adversarial review item 8: a read on 310:338 turns `v` = `$04FE`
    /// into `$14FF` on 311:2). Cheap: the dot test fails on every other dot.
    fn read_latches_on_pre_render(&self) -> bool {
        self.dot >= 335
            && self.scanline == self.pre_render_scanline() - 1
            && self.rendering_enabled()
    }

    /// The read buffer of a `$2007` read made while the PPU is fetching: **what the PPU was
    /// reading anyway**.
    ///
    /// The read of `$2007` does not fill the buffer itself; it starts a small state machine that
    /// raises the PPU's read line four dots after the CPU's read ends, and the buffer
    /// takes whatever is on the bus then. In a blanking that is the byte at `v`. With the PPU
    /// fetching, the bus is busy with the fetch of that dot — every dot of a line that renders is
    /// either setting an address up (odd dots) or reading (even dots) — so "if the background
    /// nametable fetch reads $02 on the same cycle the PPU read buffer is supposed to be updated,
    /// then $02 goes into the buffer" (`AccuracyCoin`, `$2007 Stress Test`, which reads on every
    /// dot of a line and checks the half the analogue behaviour leaves stable).
    ///
    /// The dots up to that read are run here and owed back ([`Self::prepaid_dots`]), so nothing is
    /// tested per dot.
    ///
    /// **Measured on the transistor netlists of the 2A03 and 2C02** (`docs/review/FIDELITY-LEADS.md`,
    /// A3c, step 25): the buffer latches at the very start of dot X — `D + 6` in the CPU/PPU
    /// alignment the rest of this core's timing follows (`D + 5` in the three others), `D` being
    /// the dot a read of the same cycle samples — and takes **what the PPU data bus holds then**:
    /// on an even X the byte the fetch of X reads, on an odd X the byte the fetch of X - 1 read.
    /// `v` takes the rendering increment (X and Y) in the middle of X. On an odd X, the address
    /// latch is open while that bus is read, so the fetch whose address X sets up reads with the
    /// buffered byte as its low byte, its high lines already from the incremented `v` (`AccuracyCoin`,
    /// `ALE + Read`: a low plane read from `$0FFF` instead of `$0F03`).
    ///
    /// Returns whether the increment of `v` was made here (the odd case, where it comes before
    /// the disturbed fetch); otherwise the caller makes it, after X, which is the same thing for
    /// every fetch.
    #[cold]
    #[inline(never)]
    fn read_buffer_from_fetch<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) -> bool {
        let latch_dots = self.data_latch_dots();
        if self.prepaid_dots != 0 {
            return false;
        }
        self.run_dots(i64::from(latch_dots - 1), cart);
        self.prepaid_dots = latch_dots - 1;
        let latch_dot = self.dot + 1;
        if !self.fetching() {
            self.run_dots(1, cart);
            self.prepaid_dots = latch_dots;
            return false;
        }
        if (latch_dot & 1) == 0 {
            // An even dot: its own fetch is the byte.
            let before_latch = self.v;
            self.run_dots(1, cart);
            self.prepaid_dots = latch_dots;
            self.read_buffer = self.fetch_bus_value(cart.as_deref());
            if (self.netlist_measured_region() || self.timing.region == NesRegion::Pal)
                && self.scanline == self.pre_render_scanline()
                && self.vertical_copy_dots().contains(&latch_dot)
            {
                self.read_increment_meets_vertical_copy(before_latch);
                self.notify_address_bus(self.v & 0x3FFF, cart);
                return true;
            }
            // Lot FIDPLUS-2 (A5, netlist): the read's X and Y increments are the pipeline's own
            // signals, so on a dot that increments coarse X anyway (every eighth, 328, 336) they
            // merge with it: one X increment, plus Y; on dot 256, which makes both, nothing more.
            // Checked on Zelda II's title split, replayed on the netlist (A5).
            if matches!(latch_dot, 8..=256 | 328 | 336)
                && latch_dot % 8 == 0
                && (self.netlist_measured_region() || self.timing.region == NesRegion::Pal)
            {
                if latch_dot != 256 {
                    self.increment_y();
                    self.v_row_next = self.v;
                }
                self.notify_address_bus(self.v & 0x3FFF, cart);
                return true;
            }
            return false;
        }

        // An odd dot: the bus still holds the byte the fetch of X - 1 read.
        let residue = self.fetch_bus_value(cart.as_deref());
        self.read_buffer = residue;
        let before_latch = self.v;
        self.run_dots(1, cart);
        if (self.netlist_measured_region() || self.timing.region == NesRegion::Pal)
            && self.scanline == self.pre_render_scanline()
            && self.vertical_copy_dots().contains(&latch_dot)
        {
            self.read_increment_meets_vertical_copy(before_latch);
            self.notify_address_bus(self.v & 0x3FFF, cart);
        } else if latch_dot == 257 && (self.netlist_measured_region() || self.timing.region == NesRegion::Pal) {
            // On dot 257 the horizontal copy and the read's coarse X increment meet: `v` and `t`
            // keep their AND (FIDELITY-LEADS I7).
            self.increment_meets_horizontal_copy(before_latch);
            self.notify_address_bus(self.v & 0x3FFF, cart);
        } else {
            self.increment_vram_address(cart);
        }
        self.run_dots(1, cart);
        self.prepaid_dots = latch_dots + 1;
        if self.fetching() {
            self.refetch_with_low_byte(residue, cart.as_deref());
        }
        true
    }

    /// The increment of `v` after a `$2007` **write** made while the PPU is fetching.
    ///
    /// The write starts the same state machine as the read ([`Self::read_buffer_from_fetch`]):
    /// `v` takes its rendering increment (X and Y) on the same dot X, `D + 6` in this core's
    /// alignment (`D + 5` in the two others), and merges with the pipeline's own increments the
    /// same way: on a dot that increments coarse X anyway, one X increment plus Y; on 256 nothing
    /// more; on 257 `v` and `t` keep the AND of their horizontal bits
    /// ([`Self::increment_meets_horizontal_copy`]). Measured on the transistor
    /// netlists of the 2A03 and 2C02 (audit `scr` at reset phase e4, `docs/review/FIDELITY-LEADS.md`
    /// I6): line 8, a write on dot 112 leaves `v` = `$652F` until dot 117 and makes it `$7530` on
    /// 118; line 41, a write on dot 66 turns `$1539` into `$253A` on 72. Before this the increment
    /// was made at the access, so 136 of 150 writes differed from the netlist for five dots, and a
    /// write whose X fell on an eighth dot kept the extra coarse X increment for the whole line.
    ///
    /// The byte itself is still written at the access; the netlist trace compares `v` and `t`, not
    /// the memory a write during rendering reaches.
    #[cold]
    #[inline(never)]
    fn write_increment_from_fetch<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        let latch_dots = self.data_latch_dots();
        if self.prepaid_dots != 0 {
            // The PPU already runs ahead of this cycle (two `$2007` accesses in a row): no room
            // to run it further, the increment is made now.
            self.increment_vram_address(cart);
            return;
        }
        self.run_dots(i64::from(latch_dots - 1), cart);
        let latch_dot = self.dot + 1;
        let before_latch = self.v;
        self.run_dots(1, cart);
        self.prepaid_dots = latch_dots;
        let measured = self.netlist_measured_region() || self.timing.region == NesRegion::Pal;
        if !self.fetching() || !measured {
            self.increment_vram_address(cart);
            return;
        }
        if self.scanline == self.pre_render_scanline()
            && self.vertical_copy_dots().contains(&latch_dot)
        {
            self.read_increment_meets_vertical_copy(before_latch);
        } else if latch_dot == 257 {
            self.increment_meets_horizontal_copy(before_latch);
        } else if matches!(latch_dot, 8..=256 | 328 | 336) && latch_dot % 8 == 0 {
            if latch_dot != 256 {
                self.increment_y();
                self.v_row_next = self.v;
            }
        } else {
            self.increment_vram_address(cart);
            return;
        }
        self.notify_address_bus(self.v & 0x3FFF, cart);
    }

    /// X - D for a `$2007` access made while fetching: the dots from the dot a read of the same
    /// cycle samples to the dot whose start latches the data and whose middle increments `v`.
    fn data_latch_dots(&self) -> i32 {
        // X - D: 6 in alignments 2 and 3, 5 in 0 and 1.
        // On the PAL 2C07 (16 dots for 5 CPU cycles) the read lands at a different point of its
        // dot from one cycle to the next: X - D is 6 on the two cycles whose dot accumulator is
        // 3 or 4, 5 on the three others (Visual 2C07 netlist, lot FIDPLUS-C C3c, `r2007`: 5 in
        // 60 % of the reads and 6 in 40 %, whatever the dot; every other split of the five
        // phases leaves 49 000 to 102 000 differing dots a frame, this one 8 400). That bench
        // drove the 2C07 with the 2A03's M2 duty (15/24); with the 2A07's 19/32 (nesdev "CPU
        // ALL", adversarial review 2026-11-16, bench `benchdotpal57`) M2 falls earlier and
        // only accumulator 4 takes 6: `r2007` and `g2007v` at e0/e3/e6/e9, 0 long runs, against
        // 1 185 385 differing dots with the old split.
        let latch_dots: i32 = if self.timing.region == NesRegion::Pal {
            if self.dot_accumulator == 4 {
                6
            } else {
                5
            }
        } else if self.alignment >= 2 {
            6
        } else {
            5
        };
        #[cfg(feature = "ppu-dot-trace")]
        let latch_dots = pal_latch_override(self.timing.region, self.dot_accumulator, latch_dots);
        latch_dots
    }

    /// A `$2007` read whose X+Y increment lands on one of the pre-render line's vertical copy dots
    /// (2C07 and 2C02 alike): the incremented `v` and `t` meet on the vertical bits and **both keep
    /// their AND** (the low side wins, as in the `$2006` load collision), while the horizontal
    /// bits keep the increment. Visual 2C07 netlist, lot FIDPLUS-C C3c, `r2007` at reset phase
    /// e0: a read latching on dot 278 turns `v` = `$7040`, `t` = `$2000` into `v` = `$0001`,
    /// `t` = `$0000` (the increment of `$7040` is `$0061`); at the other phases the same read
    /// latches on 277, before the copy, and `t` is left alone. A read latching on dot 300
    /// (every phase) fits too: `$2000` AND `$3002` keeps `t`. Odd latch dots alike (`g2007v`, a
    /// program aimed at the window: at e3 a read latching on 289 turns `$7BC2` / `$7BC0` into
    /// `$0BC3` / `$0BC0`, one on 285 `$5A81` / `$5A80` into `$4A82` / `$4A80`). `before_latch` is
    /// `v` before the copy of that dot. The NTSC 2C02 does the same (transistor netlist, `g2007n`
    /// at reset phases e0/e2/e4/e6: the nine events of e0 and e4 fit, dots 280-292, the first
    /// copy dot included).
    #[cold]
    #[inline(never)]
    fn read_increment_meets_vertical_copy(&mut self, before_latch: u16) {
        const VERTICAL: u16 = 0x7BE0;
        self.v = before_latch;
        self.increment_coarse_x();
        self.increment_y();
        let kept = self.t & self.v & VERTICAL;
        self.t = (self.t & !VERTICAL) | kept;
        self.v = (self.v & !VERTICAL) | kept;
        self.v_row_next = self.v;
    }

    /// A `$2007` read or write whose X+Y increment lands on dot 257, the horizontal copy: as on
    /// the vertical copy dots ([`Self::read_increment_meets_vertical_copy`]) the incremented `v`
    /// and `t` meet on the horizontal bits and **both keep their AND**, the vertical bits keep the
    /// increment. 2C02 netlist, audit `scr` at reset phase e4 (FIDELITY-LEADS I7): a write on dot
    /// 251 (X = 257) turns `v` = `$0131`, `t` = `$3F6F` into `v` = `$1122`, `t` = `$3B62`; a read
    /// on 251, `$5111` / `$316F` into `$6102` / `$3162`. The rule this replaces (lot FIDPLUS-2,
    /// A5: the copy wins, only Y is left) is the case where the AND gives `t`'s bits, as on Zelda
    /// II's title split. `before_latch` is `v` before the copy of dot 257.
    #[cold]
    #[inline(never)]
    pub(super) fn increment_meets_horizontal_copy(&mut self, before_latch: u16) {
        const HORIZONTAL: u16 = 0x041F;
        self.v = before_latch;
        self.increment_coarse_x();
        self.increment_y();
        let kept = self.t & self.v & HORIZONTAL;
        self.t = (self.t & !HORIZONTAL) | kept;
        self.v = (self.v & !HORIZONTAL) | kept;
        self.v_row_next = self.v;
    }

    /// The byte the fetch of the dot just run (an even one, where the fetches read) put on the
    /// PPU bus.
    fn fetch_bus_value(&self, cartridge: Option<&Cartridge>) -> u8 {
        let dot = self.dot;
        match dot {
            1..=256 | 321..=336 => match (dot - 1) & 0x07 {
                1 => self.nametable_latch,
                3 => self.peek_vram::<AnyBoard>(
                    self.attribute_address(),
                    ChrFetch::Background,
                    cartridge,
                ),
                5 => self.pattern_low_latch,
                _ => self.pattern_high_latch,
            },
            // The sprite fetches read the nametable twice where the background reads it and the
            // attributes; the first of them, at dot 258, is the read dot 257 set up, before the
            // horizontal bits of `v` were reloaded.
            258 => self.nametable_latch,
            257..=320 => {
                #[allow(clippy::cast_sign_loss)] // 0..8 by the range.
                let slot = ((dot - 257) >> 3) as usize;
                match (dot - 257) & 0x07 {
                    5 => self.sprite_pattern_low[slot],
                    7 => self.sprite_pattern_high[slot],
                    _ => self.peek_vram::<AnyBoard>(
                        0x2000 | (self.v & 0x0FFF),
                        ChrFetch::Background,
                        cartridge,
                    ),
                }
            }
            _ => self.nametable_latch,
        }
    }

    const fn attribute_address(&self) -> u16 {
        0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07)
    }

    /// Redoes the background fetch of the dot just run with `low` as the low byte of its address
    /// (see [`Self::read_buffer_from_fetch`]). The sprite fetches are left alone.
    fn refetch_with_low_byte(&mut self, low: u8, cartridge: Option<&Cartridge>) {
        let dot = self.dot;
        if !matches!(dot, 1..=256 | 321..=336) {
            return;
        }
        let with_low = |address: u16| (address & 0xFF00) | u16::from(low);
        match (dot - 1) & 0x07 {
            1 => {
                let address = with_low(0x2000 | (self.v & 0x0FFF));
                self.nametable_latch =
                    self.peek_vram::<AnyBoard>(address, ChrFetch::Background, cartridge);
            }
            3 => {
                let address = with_low(self.attribute_address());
                let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                self.attribute_latch =
                    (self.peek_vram::<AnyBoard>(address, ChrFetch::Background, cartridge) >> shift)
                        & 0x03;
            }
            5 => {
                let address = with_low(self.pattern_address(0));
                self.pattern_low_latch =
                    self.peek_vram::<AnyBoard>(address, ChrFetch::Background, cartridge);
            }
            _ => {
                let address = with_low(self.pattern_address(8));
                self.pattern_high_latch =
                    self.peek_vram::<AnyBoard>(address, ChrFetch::Background, cartridge);
            }
        }
    }

    /// The second `$2006` write while the PPU is fetching: `v` takes the new address **four dots
    /// later**, three on the hardware (Mesen 2 has the same delay) and the one by which a
    /// register access of this core leads it.
    ///
    /// And when those four dots end between the two dots of a nametable or attribute fetch (the
    /// address set up with the old `v`, the read still to come) that read is made at a **hybrid
    /// address**: "the high byte will be determined by new value of the v register, and the low
    /// byte will be determined by the octal latch set on the previous cycle" (`AccuracyCoin`,
    /// `Hybrid Addresses`: `$2F00` written while the bus holds `$2C19` reads the tile of
    /// `$2F19`).
    ///
    /// The dots are run here and owed back ([`Self::prepaid_dots`]); `write` sets `v` on return.
    #[cold]
    #[inline(never)]
    fn delay_address_load<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        if self.prepaid_dots != 0 {
            return;
        }
        // Three dots in every alignment (transistor netlist with the CPU's hold time, step 33;
        // TriCNES's fourth dot in alignment 2 does not appear there). The PAL 2C07 loads two
        // dots after the write on the cycle whose dot accumulator is 0, three on the others
        // (Visual 2C07 netlist, lot FIDPLUS-C C3c, `g2006x`: 2 in 20 % of the loads, 3 in 80 %).
        let delay = if matches!(self.timing.region, NesRegion::Pal) && self.dot_accumulator == 0 {
            2
        } else {
            3
        };
        self.run_dots(i64::from(delay), cart);
        self.prepaid_dots = delay;
        if self.glitch_2006 {
            self.address_load_collision();
        }
        let next = self.dot + 1;
        if self.fetching() && matches!(next, 1..=256 | 321..=336) {
            let hybrid = |address: u16, old: u16| (address & 0xFF00) | (old & 0x00FF);
            match (next - 1) & 0x07 {
                1 => {
                    let address = hybrid(0x2000 | (self.t & 0x0FFF), self.v);
                    self.nametable_latch =
                        self.peek_vram::<K>(address, ChrFetch::Background, cart.as_deref());
                }
                3 => {
                    let address = hybrid(0x23C0 | (self.t & 0x0C00), self.attribute_address());
                    let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                    self.attribute_latch =
                        (self.peek_vram::<K>(address, ChrFetch::Background, cart.as_deref())
                            >> shift)
                            & 0x03;
                }
                0 | 2 => {
                    // The next dot starts a nametable or attribute fetch (8k+1, 8k+3): the
                    // netlist loads `v` in the middle of this dot, so that fetch's ALE already
                    // carries the new address (transistor netlist, `g2006x` e4, bus comparison
                    // 2026-11-16: a write on dot 95 puts `$2BED` on the bus on dot 99, not the
                    // old `$23D3`). This core reads the byte on that first dot, so the tile or
                    // attribute of that fetch came from the old address too. No increment falls
                    // on these dots.
                    self.v = self.t;
                }
                _ => {}
            }
        } else if self.fetching()
            && (matches!(next, 337 | 339)
                || ((257..=320).contains(&next) && matches!((next - 257) & 0x07, 0 | 2)))
        {
            // The dummy nametable fetches of the line's end and the sprite phase's garbage ones
            // take the new address the same way (bus comparison, `g2006w`/`g2006x` e4: writes on
            // 333 and 335 show on 337 and 339, one on 269 on 273). Their bytes feed no tile — dot
            // 1 reads the next line's — so only `v` moves.
            self.v = self.t;
        }
        self.run_dots(1, cart);
        self.prepaid_dots = delay + 1;
    }

    /// A second `$2006` write on a copy dot: the copy reads `t` while the write's low byte reaches
    /// it, and `v` keeps what the copy took until the delayed load (FIDELITY-LEADS A8, C3c).
    ///
    /// * 2C02 (netlist, `g2006x`, `$08` and `$D7` written on dots 257 and 304): the horizontal copy
    ///   of dot 257 and the pre-render line's last vertical copy (304) take, for the bits the low
    ///   byte drives, the written byte in alignment 2 and the CPU data bus as it stood before the
    ///   write in alignment 3 (`$20`, the address's high byte), the open bus of the `$2005` write
    ///   on these dots and of the `$2000` glitch, under the same switch. Alignments 0 and 1 take
    ///   the written byte. Fidelite+ has had it since FIDPLUS-A (`post_address_load`); the
    ///   standard core since 2026-11-16, a rule on a cold write path.
    /// * 2C07 (Visual 2C07, `g2006x` at reset phases e0, e3, e6): dot 257 the same, the open bus on
    ///   the cycle whose dot accumulator is 4 ([`Self::open_bus_on_copies`]); its vertical copy
    ///   ends on dot 302: `g2006w` writes there (adversarial review 2026-11-16), a second write
    ///   only leaves a 3-dot transient before the load, left alone; the first write is
    ///   [`Self::first_address_write_on_vertical_copy`].
    #[cold]
    #[inline(never)]
    fn address_write_on_copy(&mut self) {
        let pal = matches!(self.timing.region, NesRegion::Pal);
        if !(pal || self.netlist_measured_region()) {
            return;
        }
        let vertical = self.dot != 257;
        if vertical && (pal || self.scanline != self.pre_render_scanline()) {
            return;
        }
        let source = if self.open_bus_on_copies() {
            self.cpu_open_bus
        } else {
            self.t.to_le_bytes()[0]
        };
        self.v = if vertical {
            (self.v & !0x00E0) | u16::from(source & 0xE0)
        } else {
            (self.v & !0x041F) | (self.t & 0x0400) | u16::from(source & 0x1F)
        };
    }

    /// The netlist loads `v` in the middle of dot D+3 (`delay_address_load`), so the increment of
    /// dot D+4, which this core has already run with the old `v`, belongs to the new one (lot
    /// FIDPLUS-2, A5, `g2006x`): on a coarse-X increment dot the loaded address is incremented,
    /// on dot 256 in X and Y. NTSC and PAL (the 2C07 the same, Visual 2C07 `g2006x`).
    #[cold]
    #[inline(never)]
    fn increment_after_delayed_load(&mut self) {
        let dot = self.dot;
        let increments = (dot % 8 == 0 && (8..=256).contains(&dot)) || dot == 328 || dot == 336;
        if !increments
            || !self.fetching()
            || !(self.netlist_measured_region() || matches!(self.timing.region, NesRegion::Pal))
        {
            return;
        }
        self.increment_coarse_x();
        if dot == 256 {
            self.increment_y();
        }
    }

    /// A `$2006` load that lands on a dot whose increment of `v` it collides with.
    ///
    /// nesdev ("PPU registers", PPUADDR): "at most one axis of scrolling will be set to the
    /// bitwise AND of the written value and the current value". Measured on the transistor
    /// netlists of the 2A03 and 2C02 (`docs/review/FIDELITY-LEADS.md`, A3c), and more than the
    /// page says:
    /// * on a coarse-X increment (dots 8 to 248 by eights, 328, 336) the coarse X of the address
    ///   loaded is the written one AND the incremented one, every other field the written one;
    /// * on dot 256, which increments X and Y, the whole address is the written one AND the
    ///   incremented one;
    /// * **`t` takes the same value**: the load drives the shared bus of both registers, so the
    ///   copy of dot 257 and the next lines carry the damage until the game writes `t` again.
    ///
    /// Called with the three dots of the delayed load run, `self.dot` being the dot the load
    /// lands in and `self.v` already incremented by it. This is the netlist's behaviour in three
    /// of the four CPU/PPU alignments (in the fourth, the load lands a dot earlier), which are
    /// also the alignments the rest of this core's timing matches.
    #[cold]
    #[inline(never)]
    fn address_load_collision(&mut self) {
        let dot = self.dot;
        let increments = (dot % 8 == 0 && (8..=256).contains(&dot)) || dot == 328 || dot == 336;
        if !increments || !self.fetching() {
            return;
        }
        // The 2C07 (Visual 2C07, `g2006w`, 193 collisions at reset phases e0/e3/e6/e9, lot
        // FIDPLUS-C C3c-collision): a coarse-X collision ANDs nametable X as well as coarse X,
        // and `t` keeps the written nametable bits (put back by the `$2006` write after `v`
        // takes this value, [`Self::write`]). The 2C02 ANDs nametable X too (transistor netlist,
        // `g2006w` at reset phases e0/e4/e6, adversarial review 2026-11-16: a write of `$2D5A`
        // colliding with `$2ADC` leaves `$2958`, 66 of 66 coarse-X collisions; the first corpus,
        // `g2006x`, never wrote nametable X = 1). The clones keep the old mask, unmeasured.
        let x_fields = if self.netlist_measured_region() || matches!(self.timing.region, NesRegion::Pal) {
            0x041F
        } else {
            0x001F
        };
        self.t = if dot == 256 {
            self.t & self.v
        } else {
            (self.t & !x_fields) | (self.t & self.v & x_fields)
        };
    }

    /// A `$2000` write that starts on dot 257: for the first half master clocks of the write the
    /// PPU's data pins still carry the CPU's open bus, "which will almost always be the upper
    /// byte of the address" (nesdev, "PPU registers", PPUCTRL), and the copy of dot 257 takes
    /// the nametable-X bit from it, so "only the next scanline [is] erroneously drawn from the
    /// left nametable". Only "for specific CPU-PPU alignments": in the netlist measurement, one
    /// of four, the one this core's timing matches (`docs/review/FIDELITY-LEADS.md`, A3c). `t`
    /// gets the written value right after, so the damage is that one line.
    #[cold]
    #[inline(never)]
    fn control_write_glitch(&mut self) {
        self.v = (self.v & !0x0400) | (u16::from(self.cpu_open_bus & 0x01) << 10);
    }

    /// Whether switching rendering off now corrupts the palettes (see `palette_corruption`):
    /// `TriCNES`'s condition — the write on the first two dots of a fetch, at most dot 250, on a
    /// rendered line or line 240, `v` at `$3C00` or above. Its write dot is this core's (measured
    /// frame by frame on a switch-off sliding a dot per frame), but its `v` is the one before
    /// this dot's coarse-X increment, which this core has already made.
    #[cold]
    #[inline(never)]
    fn switch_off_corrupts_palettes(&self) -> bool {
        let dot = self.dot;
        let fetching_line =
            self.scanline < VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline();
        if !(self.palette_corruption
            && self.alignment == 2
            && self.rendering_enabled()
            && (fetching_line || self.scanline == VISIBLE_SCANLINES)
            && (dot & 0x07) < 2
            && dot <= 250)
        {
            return false;
        }
        let mut v = self.v;
        if fetching_line && dot.trailing_zeros() >= 3 && dot >= 8 {
            v = if v.trailing_zeros() >= 5 {
                (v | 0x1F) ^ 0x0400
            } else {
                v - 1
            };
        }
        (v & 0x3FFF) >= 0x3C00
    }

    /// A `$2006` load that took `v` out of palette RAM, `previous` being the address before it
    /// (see `palette_corruption`): `TriCNES` corrupts the palettes when the load lands on a
    /// visible dot and the old address did not end in 0, at the pixel of that dot, which here is
    /// the next one ([`Self::delay_backdrop_address`] ran the dots before it).
    #[cold]
    #[inline(never)]
    fn palette_load_corruption(&mut self, previous: u16) {
        if self.palette_corruption
            && self.alignment == 2
            && self.scanline < VISIBLE_SCANLINES
            && self.dot < SCREEN_WIDTH_I32
            && (previous & 0x000F) != 0
            && (self.v & 0x3FFF) < 0x3F00
        {
            let color = self.next_pixel_color_index();
            self.corrupt_palettes(color);
        }
    }

    /// The second `$2006` write on a visible line with rendering off: the pixels keep showing
    /// the old `v`'s backdrop for a few more dots (see `backdrop_entry` in the renderer, the
    /// palette background).
    ///
    /// Measured on the transistor netlist (`VisualNes`, rendering off, `v` loaded with `$3F05` and
    /// back with `$2000` every 81 dots across visible lines, pixel colours read where `VisualNes`
    /// reads them): a load made on dot `D` shows from pixel `D + 2` in the alignment this core
    /// follows by default and in another, which this core drew from pixel `D`; the two dots are
    /// run with the old `v` and owed back ([`Self::prepaid_dots`]), in every alignment
    /// (`docs/review/FIDELITY-LEADS.md`, D5; `TriCNES` gives 2 to 4 dots by alignment).
    /// Only the picture moves: outside the visible lines, or with no palette address on either
    /// side, the load has nothing to show and stays immediate.
    #[cold]
    #[inline(never)]
    fn delay_backdrop_address<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        let palette = |address: u16| (address & 0x3F00) == 0x3F00;
        if self.prepaid_dots != 0 || !(palette(self.v) || palette(self.t)) {
            return;
        }
        let delay = 2;
        self.run_dots(i64::from(delay), cart);
        self.prepaid_dots = delay;
    }

    /// A first `$2005` write on a visible line with rendering on, in alignment 3: the pixel of
    /// the dot it lands on takes fine X from the CPU's open bus.
    ///
    /// This core draws pixel `x` on dot `x + 1`, so a write made on dot `D` changes fine X from
    /// pixel `D` on. Measured on the transistor netlist (`VisualNes`, 2A03 + 2C02, a first write
    /// sliding across the line, pixel colours read where `VisualNes` reads them,
    /// `docs/review/FIDELITY-LEADS.md`, A4): pixel `D - 1` keeps the old fine X in all four
    /// alignments; pixel `D` shows the new one in the others, but in the alignment this core
    /// follows by default — the one whose `$2000` write on dot 257 takes the open bus — the
    /// fine X of the data bus as it stood before the write, the new one from pixel `D + 1`. The
    /// same open bus as [`Self::scroll_write_on_copy`]. Its dot is run here with that fine X and
    /// owed back ([`Self::prepaid_dots`]), under the `$2000` glitch's switch. (`TriCNES` puts that
    /// pixel in its alignment 2 and moves alignment 0 a pixel earlier; the netlist shows neither.)
    #[cold]
    #[inline(never)]
    fn fine_x_write_mid_line<K: BoardKind>(&mut self, value: u8, cart: Cart<'_, '_, K>) {
        if self.open_bus_on_copies()
            && self.prepaid_dots == 0
            && (0..SCREEN_WIDTH_I32).contains(&self.dot)
        {
            self.fine_x = self.cpu_open_bus & 0x07;
            self.run_dots(1, cart);
            self.prepaid_dots = 1;
            self.fine_x = value & 0x07;
        }
    }

    /// A `$2005` write that lands on the copy of `t` into `v` this core has just made: the
    /// horizontal copy of dot 257 (first write) or the last vertical copy of the pre-render line,
    /// dot 304 (second write).
    ///
    /// A register access of this core leads the hardware's effect by about a dot, so the copy
    /// this core already made with the old `t` took, on the console, the written value — or, in
    /// the alignment this core follows by default (3), the CPU data bus as it stood before the
    /// write, the same open bus as the `$2000` glitch of that dot. Measured on the transistor
    /// netlist (`VisualNes`, a first write of `$F8` or `$00` with `$20` on the bus, sliding across
    /// dot 257): written on dot 257, coarse X 31 or 0 in three alignments, 4 (`$20 >> 3`) in the
    /// fourth, the one where `$2000` glitches; on dot 258, the old value everywhere
    /// (`docs/review/FIDELITY-LEADS.md`, A4). Mesen 2 and `BeesNES` apply the same open bus on dot
    /// 257 in the one alignment they emulate. The vertical copy of dot 304 is taken by the same
    /// mechanism, not measured. The bus value is the `$2000` glitch's switch; off, the written
    /// value.
    #[cold]
    #[inline(never)]
    fn scroll_write_on_copy(&mut self, value: u8) {
        let source = if self.open_bus_on_copies() {
            self.cpu_open_bus
        } else {
            value
        };
        if !self.write_toggle && self.dot == 257 {
            self.v = (self.v & !0x001F) | u16::from(source >> 3);
            self.v_row_next = self.v;
        } else if self.write_toggle
            && self.dot == *self.vertical_copy_dots().end()
            && self.scanline == self.pre_render_scanline()
        {
            self.v = (self.v & !0x73E0)
                | (u16::from(source & 0xF8) << 2)
                | (u16::from(source & 0x07) << 12);
            self.v_row_start = self.v;
            self.v_row_next = self.v;
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
                    self.palette_raw[palette_index(address)]
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
    pub fn write(&mut self, register: u16, value: u8, cartridge: Option<&mut Cartridge>) {
        self.write_as::<AnyBoard>(register, value, cartridge);
    }

    /// [`Self::write`] with the board reached through `K` (lot CORE-2).
    #[allow(clippy::too_many_lines)] // One arm per register, as the chip; the trace adds its mark.
    pub fn write_as<K: BoardKind>(
        &mut self,
        register: u16,
        value: u8,
        cartridge: Option<&mut Cartridge>,
    ) {
        let mut cartridge = CartSlot::<K>::new(cartridge);
        self.apply_decay();
        #[cfg(feature = "ppu-dot-trace")]
        self.trace_access(0x2000 | 0x8000 | u32::from(register & 7) | (u32::from(value) << 16));

        // A write to any PPU register, $2002 included, loads the whole bus latch
        // (ppu_open_bus test 2).
        self.refresh_decay(value, 0xFF);

        match register & 0x07 {
            // **The RC2C05 swaps the meanings of `$2000` and `$2001`** (nesdev, "Vs. System"),
            // which is why these two arms ask one `bool` settled at load and the five arms below
            // ask nothing: a swap of the register *numbers* would have cost every write a
            // compare, and `$2007` is written thousands of times a frame.
            0 => {
                if self.after_reset {
                    // The reset line still holds the register (`emu.ppu_reset_line`).
                    return;
                }
                self.write_control::<K>(value, cartridge.as_deref_mut());
            }

            1 => {
                if self.after_reset {
                    // The reset line still holds the register (`emu.ppu_reset_line`).
                    return;
                }
                if ((value ^ self.mask) & 0xE0) != 0 {
                    self.note_emphasis(value);
                }
                let was_rendering = self.rendering_enabled();
                if was_rendering == ((value & 0x18) != 0) {
                    self.mask = value;
                } else if self.edges_on_for::<K>() {
                    self.post_switch_rendering(value);
                } else {
                    self.switch_rendering(value, &mut cartridge);
                }
            }

            3 => {
                if self.oamaddr_write_corruption && !self.pre_g_revision {
                    self.oamaddr_write_corruption(value);
                }
                if self.oam_decay {
                    self.move_oam_address(value);
                } else {
                    self.oam_address = value;
                }
            }

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
                if self.after_reset {
                    // The reset line still holds the register (`emu.ppu_reset_line`).
                    return;
                }
                if self.write_toggle {
                    self.t = (self.t & 0x8FFF) | (u16::from(value & 0x07) << 12);
                    self.t = (self.t & 0xFC1F) | (u16::from(value & 0xF8) << 2);
                } else {
                    self.t = (self.t & 0xFFE0) | u16::from(value >> 3);
                    self.fine_x = value & 0x07;
                }
                if (self.dot == 257 || self.dot == *self.vertical_copy_dots().end())
                    && self.fetching()
                {
                    self.scroll_write_on_copy(value);
                }
                if !self.write_toggle
                    && self.alignment == 3
                    && self.scanline < VISIBLE_SCANLINES
                    && self.rendering_enabled()
                {
                    self.fine_x_write_mid_line(value, &mut cartridge);
                }

                self.write_toggle = !self.write_toggle;
            }

            6 => {
                if self.after_reset {
                    // The reset line still holds the register (`emu.ppu_reset_line`).
                    return;
                }
                if self.write_toggle && self.edges_on_for::<K>() {
                    self.t = (self.t & 0xFF00) | u16::from(value);
                    self.post_address_load();
                } else if self.write_toggle {
                    self.t = (self.t & 0xFF00) | u16::from(value);
                    if (self.dot == 257 || self.dot == *self.vertical_copy_dots().end())
                        && self.fetching()
                    {
                        self.address_write_on_copy();
                    }
                    let written = self.t;
                    if (self.address_load_delay || self.bus_conflicts) && self.fetching() {
                        self.delay_address_load(&mut cartridge);
                    } else if self.address_load_delay
                        && self.scanline < VISIBLE_SCANLINES
                        && !self.rendering_enabled()
                    {
                        self.delay_backdrop_address(&mut cartridge);
                    }
                    let previous = self.v;
                    // Three dots and the one run after them, or two on a PAL phase-0 cycle.
                    let delayed = self.prepaid_dots == 4
                        || (self.prepaid_dots == 3 && matches!(self.timing.region, NesRegion::Pal));
                    self.v = self.t;
                    if self.t != written && matches!(self.timing.region, NesRegion::Pal) {
                        // C3c-collision: the 2C07's `t` keeps the written nametable bits.
                        self.t |= written & 0x0C00;
                        // A load that collided on dot 256: the dot after it, 257, copies `t`'s
                        // horizontal bits into the `v` it left, which the prepaid run made
                        // before this assignment (Visual 2C07, `g2006w`: a write on dot 254
                        // ends at `$2C58`, not `$2858`). On the 2C02 `t` = `v` after the
                        // collision, so the copy changes nothing there.
                        if self.dot == 257 && self.fetching() {
                            self.v = (self.v & !0x041F) | (self.t & 0x041F);
                        }
                    }
                    if delayed {
                        self.increment_after_delayed_load();
                    }
                    if (previous & 0x3F00) == 0x3F00 {
                        self.palette_load_corruption(previous);
                    }

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
                    if self.dot == 257
                        && self.fetching()
                        && (self.netlist_measured_region()
                            || matches!(self.timing.region, NesRegion::Pal))
                    {
                        self.first_address_write_on_copy(value);
                    } else if self.scanline == self.pre_render_scanline()
                        && self.dot == *self.vertical_copy_dots().end()
                        && self.fetching()
                        && (self.netlist_measured_region()
                            || matches!(self.timing.region, NesRegion::Pal))
                    {
                        self.first_address_write_on_vertical_copy(value);
                    }
                }

                self.write_toggle = !self.write_toggle;
            }

            7 => {
                // A `$2007` access is an ALE of `v` (step 39).
                self.ale_low = self.v.to_le_bytes()[0];
                if self.edges_on_for::<K>() {
                    if !self.rendering_now() {
                        self.post_data_store(value);
                        return;
                    }
                    self.rewrite_pending_vram_write(value, &mut cartridge);
                    self.write_vram_inner(self.v & 0x3FFF, value, PpuBusAccess::Cpu, &mut cartridge);
                    self.post_write_increment();
                    return;
                }
                self.rewrite_pending_vram_write(value, &mut cartridge);
                self.write_vram_inner(self.v & 0x3FFF, value, PpuBusAccess::Cpu, &mut cartridge);
                if self.bus_conflicts && (self.fetching() || self.read_latches_on_pre_render()) {
                    self.write_increment_from_fetch(&mut cartridge);
                } else {
                    self.increment_vram_address(&mut cartridge);
                }
            }

            // $2002 is read-only; the write only loaded the bus latch.
            _ => {}
        }
    }

    /// Rendering on, on a line that renders (visible or pre-render).
    const fn rendering_now(&self) -> bool {
        self.rendering_enabled()
            && (self.scanline < VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline())
    }

    /// A first `$2006` write on dot 257 of a fetching line, PAL 2C07 (Visual 2C07, `g2006w`, reset
    /// phases e0/e3/e6/e9, lot FIDPLUS-C C3c): the horizontal copy of that dot already takes the
    /// nametable-X bit the write puts in `t` (bit 2 of the byte), or the CPU's open bus on the
    /// cycle [`Self::open_bus_on_copies`] names, as the second write and `$2005` do there. A write
    /// on dot 258 comes after the copy and leaves `v` alone, which is what this core does anyway.
    #[cold]
    #[inline(never)]
    fn first_address_write_on_copy(&mut self, value: u8) {
        let source = if self.open_bus_on_copies() {
            self.cpu_open_bus
        } else {
            value
        };
        self.v = (self.v & !0x0400) | (u16::from(source & 0x04) << 8);
    }

    /// A `$2000` write on the 2C02's last vertical copy dot (304) of the pre-render line: that
    /// copy already takes the nametable-Y bit the write puts in `t`, as the first `$2006` write
    /// does there ([`Self::first_address_write_on_vertical_copy`]). 2C02 netlist, audit `scr` at
    /// reset phase e4, alignment 3: `$2000` = `$00` on 261:304 leaves `v` = `$316F` with `t` =
    /// `$316F`, where Zaxnes kept the bit the copy took before the write (`$396F`), and the
    /// difference ran through the first five lines of the next frame (FIDELITY-LEADS I7).
    #[cold]
    #[inline(never)]
    fn control_write_on_vertical_copy(&mut self, value: u8) {
        if self.scanline != self.pre_render_scanline() || !self.fetching() {
            return;
        }
        let source = if self.open_bus_on_copies() {
            self.cpu_open_bus
        } else {
            value
        };
        self.v = (self.v & !0x0800) | (u16::from(source & 0x02) << 10);
    }

    /// A first `$2006` write on the pre-render line's last vertical copy dot (302 on the 2C07, 304
    /// on the 2C02): that copy already takes the vertical bits the write puts in `t`'s high byte,
    /// from the written byte or the CPU's open bus as [`Self::open_bus_on_copies`] says (Visual
    /// 2C07, `g2006w` at e3, adversarial review 2026-11-16: `$25` on dot 302, open bus `$20`,
    /// turns `v` = `$2D5A` into `$245A`). The 2C02 form is unmeasured; its second write on 304
    /// follows the same copy ([`Self::address_write_on_copy`]).
    #[cold]
    #[inline(never)]
    fn first_address_write_on_vertical_copy(&mut self, value: u8) {
        const VERTICAL: u16 = 0x7BE0;
        let source = if self.open_bus_on_copies() {
            self.cpu_open_bus
        } else {
            value
        };
        let seen = (self.t & 0x00FF) | (u16::from(source & 0x3F) << 8);
        self.v = (self.v & !VERTICAL) | (seen & VERTICAL);
    }

    /// Whether a register write landing on a copy of `t` into `v` (or on a pixel) takes the CPU's
    /// open bus instead of the written value: alignment 3 on the 2C02 (A4), under the `$2000`
    /// glitch's switch. On the PAL 2C07 (16 dots for 5 CPU cycles) only on the cycle whose dot
    /// accumulator is 4, one in five (Visual 2C07 netlist, lot FIDPLUS-C C3c, `g2005` /
    /// `g2005px`: a first `$2005` write on dot 257 gives the written coarse X on the other cycles;
    /// of the 32 sets of phases, {4} alone leaves the fewest differing dots).
    const fn open_bus_on_copies(&self) -> bool {
        self.alignment == 3
            && self.glitch_2000
            && (!matches!(self.timing.region, NesRegion::Pal) || self.dot_accumulator == 4)
    }

    /// The dots of the pre-render line that copy `t`'s vertical bits into `v`: 280 to 304 on the
    /// 2C02, two dots earlier on the PAL 2C07 (Visual 2C07 netlist, lot FIDPLUS-C C3c: the copy is
    /// in `v` at the end of dot 278 in every frame of `g2000` and `g2001px`). The end of the PAL
    /// window is taken as shifted too; no corpus write has landed there yet.
    pub(super) const fn vertical_copy_dots(&self) -> std::ops::RangeInclusive<i32> {
        if matches!(self.timing.region, NesRegion::Pal) {
            278..=302
        } else {
            280..=304
        }
    }

    /// Whether the rules measured on the NTSC 2C02 netlist apply (lot FIDPLUS-2, A5): the PAL
    /// 2C07 (3.2 dots a cycle) and the clones keep the previous model, which their golden pictures
    /// follow (Zelda II Europe moved with them), until a measurement of their own.
    const fn netlist_measured_region(&self) -> bool {
        matches!(self.timing.region, NesRegion::Ntsc | NesRegion::Multi)
    }

    /// Two `$2007` writes on consecutive CPU cycles (the dummy write and the write of a
    /// read-modify-write instruction, `INC $2007`): the PPU stores a write to VRAM about four dots
    /// after the CPU's, from its data latch, and the second CPU write, three dots after the first,
    /// has already replaced that latch, so **the first address receives the second value** too
    /// (lot FIDPLUS-2, transistor netlist, the four alignments: `INC $2007` over 03 04 leaves
    /// 01 01, not 00 01; `TriCNES` agrees in its alignments 0 and 1;
    /// `docs/review/FIDELITY-LEADS.md` A5). Rendering off only: during rendering the write path
    /// is another one (C1), not measured here; palette entries are internal to the PPU and
    /// not measured either, so they keep one write each.
    fn rewrite_pending_vram_write<K: BoardKind>(&mut self, value: u8, cart: Cart<'_, '_, K>) {
        let rendering = self.rendering_now();
        let now = self.ppu_clock;
        let (then, address) = self.last_data_write;
        // One CPU cycle is 3 dots (3.2 on PAL): "the previous cycle" is 4 dots at most.
        if !rendering
            && then != i64::MIN
            && (1..=4).contains(&(now - then))
            && !is_palette_address(address)
        {
            self.write_vram_inner(address, value, PpuBusAccess::Inert, cart);
        }
        self.last_data_write = if rendering {
            (i64::MIN, 0)
        } else {
            (now, self.v & 0x3FFF)
        };
    }

    /// Writes one OAM byte, used by OAM DMA ($4014).
    pub fn write_oam_dma(&mut self, value: u8) {
        self.write_oam(value);
    }

    /// Reads OAM through `$2004`.
    ///
    /// Bits 2 to 4 of a sprite's attribute byte do not exist in the chip, so the masked
    /// value is what was stored (see `write_oam`).
    ///
    /// # During rendering it is not OAM that answers, it is the copy buffer
    ///
    /// On a visible line with rendering on, the address `$2003` holds is not what the chip is
    /// reading: the sprite evaluation owns the OAM address counter for the whole line, so a
    /// `$2004` read sees **the byte the evaluation last latched** — the "OAM copy buffer" — and not
    /// `oam[oam_address]` (nesdev, "PPU sprite evaluation"; `third_party/mesen2`'s
    /// `_oamCopybuffer`). Three windows, which is what `AccuracyCoin`'s `Address $2004 behavior`
    /// walks:
    ///
    /// * **dots 1 to 64**, the secondary-OAM clear: `$FF`, which is what the clear is writing and
    ///   therefore what the buffer holds. Test 4, "will always read `$FF`".
    /// * **dots 65 to 256**, the evaluation proper: [`Self::oam_eval_latch`], refreshed on every
    ///   odd dot from the sprite and byte the evaluation stands on. Test 8 reads at about dot 130
    ///   with OAM holding `oam[i] == i` and requires a value that is **neither `$00` nor `$FF`** —
    ///   the address is "changing every other ppu cycle", so the only wrong answers are the two
    ///   that mean "nothing is moving".
    /// * **dot 0 and dots 257 to 340**, the sprite fetches: the secondary OAM, at the byte the
    ///   fetch of that dot is reading, `((dot − 257) / 8) * 4 + min((dot − 257) % 8, 3)`. Test 9
    ///   reads at about dot 310 of line 0 and requires `$FF`, which is what the clear left there
    ///   because evaluation does not run on the pre-render line.
    ///
    /// Deriving the window from the dot costs the per-dot path **nothing**: the evaluation already
    /// maintains `oam_eval_latch`, and the secondary-OAM index is arithmetic on `dot` done only
    /// when a read actually lands. `third_party/mesen2` takes the same shortcut and says why —
    /// "this is done here for performance. It's faster to only do this here when it's needed".
    ///
    /// [`Self::peek`] keeps answering `oam[oam_address]`: it is the side-effect-free host accessor
    /// of REQ-CORE-081 and must report what the program wrote, not what the chip is clocking.
    ///
    /// Behind `ppu-exact-sprite-eval` with the evaluation it reads from: the copy buffer only
    /// carries the right byte per dot if the byte counter is the hardware's. The default build
    /// keeps `11407930`'s answer, below.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn read_oam(&self) -> u8 {
        if self.rendering_enabled() && self.scanline < VISIBLE_SCANLINES {
            if (1..=64).contains(&self.dot) {
                return 0xFF;
            }

            if (65..=256).contains(&self.dot) {
                return self.oam_eval_latch;
            }

            // After the last fetch the secondary OAM address is back at 0, where it stays until
            // the clear: dots 321 to 340, and dot 0 after them (`AccuracyCoin`, `$2004 Stress
            // Test` 2 and `Misaligned OAM2 Address` 2: "between dots 321 and 340 [a read]
            // should, under normal conditions, read from index 0 of OAM2").
            if self.dot > 320 {
                // Index 0, unless part of this line's fetches did not run (`note_fetch_pause`).
                let line = self.ppu_clock - i64::from(self.dot);
                let missed = if self.fetch_line == line {
                    self.fetch_missed
                } else {
                    0
                };
                #[allow(clippy::cast_sign_loss)] // Masked to 0..32.
                return self.secondary_oam[((32 - missed) & 0x1F) as usize];
            }
            let since = (self.dot - 257).clamp(0, 63);
            #[allow(clippy::cast_sign_loss)] // Clamped to 0..=63.
            let index = (((since / 8) * 4) + (since % 8).min(3)) as usize;
            return self.secondary_oam[index];
        }

        self.oam[usize::from(self.oam_address)]
    }

    /// Reads OAM through `$2004`, **the default build's answer** — `11407930`'s body to the
    /// instruction.
    ///
    /// Bits 2 to 4 of a sprite's attribute byte do not exist in the chip, so the masked value is
    /// what was stored (see `write_oam`). During the 64 dots in which the secondary OAM is cleared,
    /// reads return `$FF` (nesdev, "PPU sprite evaluation"); the rest of the time they answer from
    /// the OAM address, which is the approximation the exact arm above replaces.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
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
        if self.oam_decay {
            self.refresh_oam_row(usize::from(self.oam_address >> 3));
        }
        self.oam[usize::from(self.oam_address)] = if (self.oam_address & 0x03) == 0x02 {
            value & 0xE3
        } else {
            value
        };
        let next = self.oam_address.wrapping_add(1);
        if self.oam_decay {
            self.move_oam_address(next);
        } else {
            self.oam_address = next;
        }
    }

    /// The address increment a `$2007` access owes, which is **not the same increment** while the
    /// PPU is fetching — **behind `ppu-2007-render-read`, and out of `default`** (REQ-CORE-112,
    /// Q-125, Q-137, Q-139).
    ///
    /// nesdev, "PPU scrolling", under "$2007 (PPUDATA) reads and writes": "During rendering (on
    /// the pre-render line and the visible lines 0-239, provided either background or sprite
    /// rendering is enabled), it will update *v* in an odd way, triggering a coarse X increment
    /// and a Y increment simultaneously", because "the carry inputs to sections of *v* are
    /// configured differently during rendering". That condition is [`Self::fetching`] word for
    /// word — **it is not a function of the dot** — and `third_party/mesen2` takes the same
    /// branch. `AccuracyCoin`'s `$2007 read w/ rendering` 2 asks for the vertical half by name.
    ///
    /// # Why this shipped, broke a P1, and is back
    ///
    /// Lot MF-4d step 6 delivered the rule and lot MF-4d-fix backed it out: it broke AMD-021
    /// R2-005 criterion 2 — Super Mario Bros. 3 emitted a background row on its split line on
    /// 4 986 of 5 000 frames — and cut the owner's widescreen ground row in half. Q-125 read that
    /// as the increment being "blind to the dot" and deferred the per-dot work to lot MF-4e.
    ///
    /// **The collision was never on the dot, and never on `v`.** It was on `v_row_start`. MF-4d's
    /// body set `v_row_start = v` *after* the branch, so the Y half of the odd increment moved the
    /// captured row's fine Y — and [`Ppu2C02::capture_background_row`] emits a row exactly when
    /// that fine Y is 1. One split line whose fine Y was 0 became one that was, which is the
    /// spurious row per frame R2-005 counted and the half-block shift the margin decoder drew.
    ///
    /// So the two requirements are reconciled by **which of the two row fields the increment
    /// reaches**, not by when it happens. The increment a `$2007` access borrows during rendering
    /// *is the fetch pipeline's own* — the page says so: the same carry chain, differently
    /// configured — and R2-005 holds the pipeline's vertical increment out of **line L's** row
    /// while letting it reach **line L + 1's**. That is already exactly how the dot-256 increment
    /// is handled: [`Self::increment_y`] touches neither field, and `transfer_address_x` at dot
    /// 257 then carries the incremented `v` into `v_row_next` alone. This path does the same, so
    /// `v_row_next` follows `v` and `v_row_start` does not.
    ///
    /// # The four possibilities, and why this is the one
    ///
    /// Measured on Super Mario Bros. 3, whose MMC3 split writes `$2006` at dot 278 of line 193 and
    /// then touches `$2007` at dot 290 — in the horizontal blank, with rendering still on, so the
    /// odd increment applies. Which line emits the banner's first background row:
    ///
    /// | the increment reaches | line 193 | line 194 | R2-005 criterion 2 |
    /// | --- | --- | --- | --- |
    /// | `v` by the PPUCTRL step, both fields (`11407930`) | — | — | row at **195**, a line late |
    /// | both fields (MF-4d step 6) | **row** | row | **violated**, the P1 regression |
    /// | `v` only, neither field | — | — | the row is **lost** altogether |
    /// | **`v` and `v_row_next`** | — | **row** | **exact** |
    ///
    /// The criterion reads "Super Mario Bros. 3 … n'émet aucune rangée de fond à la ligne où le
    /// split écrit `$2006` ; **la première rangée du bandeau est émise à la ligne suivante**".
    /// Only the last row of that table satisfies both halves of the sentence, and it is **stricter
    /// than the baseline**, which emitted that row at line 195.
    ///
    /// R2-005's text excludes "l'incrément vertical **de fin de ligne**" and this one is not at
    /// the end of a line, so that is a reading of the requirement rather than its letter:
    /// **Q-137** files it with the provisional choice marked.
    ///
    /// # Why it is behind a feature and out of `default` (lot MF-4e-gate, Q-139)
    ///
    /// Not because it is dear — step 4 measured it at +0.000 % to +0.005 % on the eleven ROMs of
    /// PR-1's set whose behaviour does not move, which is well inside the owner's tolerance — but
    /// because of **what it changes on screen**. R0's dense A/B of the 3D packs against v0.4.2
    /// found it moving the picture in exactly the two games the owner watches most: *Super Mario
    /// Bros. 3* (5 dumps, 58 of 681 sampled frames differ) and *Zelda II* (2 dumps, 18-19 of
    /// 1 001), with and without a pack. SMB3's split line under the widescreen hack is where the
    /// owner saw a bug the last time this rule shipped, and lot MF-4d-fix backed it out for that.
    ///
    /// So the rule waits on the owner's eye, and until then the default build renders this path as
    /// `11407930` does — which is the `else` arm, and is `11407930`'s whole body. The two arms are
    /// two `#[cfg]` bodies rather than one body with a `cfg!` in it so that the default build's
    /// token stream is the baseline's exactly: MF-4d-fix step 5 measured that giving this arm a
    /// body of its own re-lays [`Self::read`] by about eleven instructions per register read, so
    /// the presence of the branch is itself a measurable thing and the gate must remove it rather
    /// than fold it.
    ///
    /// `ppu-2007-render-read` is a **read** in name only: both `$2007` paths come here, the write
    /// of register 7 as much as [`Self::read_data`], and the rule applies to both. The name
    /// follows the handle the owner and `AccuracyCoin` share for it — the row it wins back is
    /// called `PPU Misc. / $2007 read w/ rendering`.
    #[cfg(feature = "ppu-2007-render-read")]
    fn increment_vram_address<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        if self.fetching() {
            // Q-137: the pipeline's own increment. It does **not** reach `v_row_start` — the row
            // of the line the access is made in, which is R2-005's exception — and it **does**
            // reach `v_row_next`, which becomes the row start of the next line, exactly as dot
            // 257 carries the dot-256 increment forward.
            self.increment_coarse_x();
            self.increment_y();
            self.v_row_next = self.v;
        } else {
            let step = if (self.control & 0x04) != 0 { 32 } else { 1 };
            self.v = self.v.wrapping_add(step) & 0x7FFF;

            // A CPU write reaches the row of the line it is made in (R2-005).
            self.v_row_start = self.v;
            self.v_row_next = self.v;
        }

        // The incremented address goes back on the PPU bus, so a $2007 access that steps
        // across $1000 clocks an MMC3 counter — which is what mmc3_irq_tests "A12 clocking"
        // measures (tests 5 and 6).
        self.notify_address_bus(self.v & 0x3FFF, cart);
    }

    /// The default build's increment: the 1 or 32 of PPUCTRL **on every line**, fetching or not,
    /// reaching both captured-row fields — which is `11407930`'s body line for line, and so
    /// v0.4.2's behaviour on this path.
    ///
    /// The hardware rule, why it is right, and why the owner's eye gates it are all on the
    /// `ppu-2007-render-read` body above; this arm carries no argument of its own beyond being the
    /// baseline. `cargo build -p zax-nes --features ppu-2007-render-read` is the one flag that
    /// swaps them, and `docs/review/BOARD-FEATURES.md` has the row (Q-139).
    #[cfg(not(feature = "ppu-2007-render-read"))]
    fn increment_vram_address<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
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
        cartridge: Option<&mut Cartridge>,
    ) -> u8 {
        self.read_vram_through_bus_as::<AnyBoard>(address, access, fetch, cartridge)
    }

    /// [`Self::read_vram_through_bus`] with the board reached through `K` (lot CORE-2).
    pub fn read_vram_through_bus_as<K: BoardKind>(
        &mut self,
        address: u16,
        access: PpuBusAccess,
        fetch: ChrFetch,
        cartridge: Option<&mut Cartridge>,
    ) -> u8 {
        let mut cartridge = CartSlot::<K>::new(cartridge);
        self.read_vram_inner(address, access, fetch, &mut cartridge)
    }

    #[allow(clippy::inline_always)]
    // Lot CORE-2: called from every board kind's copy of the hot path, LLVM stops inlining it (measured, +2.8 % on metroid for span_pixel alone).
    #[inline(always)]
    fn read_vram_inner<K: BoardKind>(
        &self,
        address: u16,
        access: PpuBusAccess,
        fetch: ChrFetch,
        cart: Cart<'_, '_, K>,
    ) -> u8 {
        let address = address & 0x3FFF;

        if access != PpuBusAccess::Inert {
            #[cfg(feature = "ppu-dot-trace")]
            self.dot_trace_bus.set(address);
            self.notify_address_bus(address, cart);
        }

        if address < 0x2000 {
            return match cart.as_deref_mut() {
                None => 0,
                Some(cartridge) if access == PpuBusAccess::Inert => {
                    cartridge.ppu_peek_as::<K>(address, fetch)
                }
                Some(cartridge) => cartridge.ppu_read_as::<K>(address, fetch),
            };
        }

        if !is_palette_address(address) {
            return self.read_nametable_inner::<K>(address, cart.as_deref());
        }

        self.palette_raw[palette_index(address)]
    }

    /// Side-effect-free read of the PPU bus, for the tile emitter: the cartridge is only
    /// peeked (REQ-CORE-033).
    fn peek_vram<K: BoardKind>(
        &self,
        address: u16,
        fetch: ChrFetch,
        cartridge: Option<&Cartridge>,
    ) -> u8 {
        let address = address & 0x3FFF;
        if address < 0x2000 {
            return cartridge.map_or(0, |cartridge| cartridge.ppu_peek_as::<K>(address, fetch));
        }

        if !is_palette_address(address) {
            return self.read_nametable_inner::<K>(address, cartridge);
        }

        self.palette_raw[palette_index(address)]
    }

    /// Writes the PPU bus at `address` (masked to 14 bits); a [`PpuBusAccess::Cpu`] write
    /// also puts the address on the cartridge's bus.
    pub fn write_vram_through_bus(
        &mut self,
        address: u16,
        value: u8,
        access: PpuBusAccess,
        cartridge: Option<&mut Cartridge>,
    ) {
        self.write_vram_through_bus_as::<AnyBoard>(address, value, access, cartridge);
    }

    /// [`Self::write_vram_through_bus`] with the board reached through `K` (lot CORE-2).
    pub fn write_vram_through_bus_as<K: BoardKind>(
        &mut self,
        address: u16,
        value: u8,
        access: PpuBusAccess,
        cartridge: Option<&mut Cartridge>,
    ) {
        let mut cartridge = CartSlot::<K>::new(cartridge);
        self.write_vram_inner(address, value, access, &mut cartridge);
    }

    fn write_vram_inner<K: BoardKind>(
        &mut self,
        address: u16,
        value: u8,
        access: PpuBusAccess,
        cart: Cart<'_, '_, K>,
    ) {
        let address = address & 0x3FFF;

        if access != PpuBusAccess::Inert {
            self.notify_address_bus_without_read(address, cart);
        }

        if address < 0x2000 {
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.ppu_write_as::<K>(address, value);
            }

            return;
        }

        if !is_palette_address(address) {
            self.write_nametable_byte(address, value, cart);
            return;
        }

        // Palette entries are 6 bits wide.
        let index = palette_index(address);
        let written = value & 0x3F;
        self.palette_raw[index] = written;
        // The colour the chip shows for that entry: the identity on a NES image, and a VS.
        // System's master-palette permutation otherwise (lot MF-VS step 2). Here, for the reason
        // the shadow entry below is here.
        let shown = self.shown_palette_entry(written);
        self.palette[index] = shown;
        // AMD-050 REQ-CORE-049: the shadow entry follows, here and not per pixel. A game writes
        // a handful of palette bytes a frame; the PPU reads 61 440 pixels.
        self.palette_marked[index] = mark(index, shown);
    }

    /// Tells the cartridge which address the PPU is **reading**, so that an MMC3-style board can
    /// count its A12 rises (REQ-CORE-114) and the MMC5 can detect the start of a scanline on its
    /// own bus (nesdev, "MMC5"; finding MR-5-4).
    ///
    /// This is the notification of every fetch of a rendered line, nametable and attribute
    /// bytes included — see [`Cartridge::on_ppu_address_bus`]. It carries no cost for a board
    /// that does not ask for it: the test is a load of one `bool` settled at load time.
    fn notify_address_bus<K: BoardKind>(&self, address: u16, cart: Cart<'_, '_, K>) {
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus_as::<K>(address, self.ppu_clock);
        }
    }

    /// The post-render line's first two dots on the bus, raised from dot 1 (the branch every
    /// line already takes there) with dot 0's own clock for the first. 2C02 netlist
    /// (`benchpins`, `g2000` e4): dot 0 of line 240 gives an ALE with the coming low-plane
    /// pattern address (`1000`, /RD high), like every dot 0 after a fetching line; from dot 1
    /// the pins show `v` (`0802`) and hold it through vblank until the pre-render line's first
    /// fetch. Address lines only, so only an A12 watcher sees them.
    #[cold]
    #[inline(never)]
    fn post_render_bus<K: BoardKind>(&self, cart: Cart<'_, '_, K>) {
        let pattern = self.pattern_address(0);
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus_without_read_as::<K>(pattern, self.ppu_clock - 1);
            cartridge.on_ppu_address_bus_without_read_as::<K>(self.v & 0x3FFF, self.ppu_clock);
        }
        #[cfg(feature = "ppu-dot-trace")]
        self.dot_trace_bus.set(self.v & 0x3FFF);
    }

    /// A read whose byte nothing uses — the sprite phase's garbage nametable fetches — put on
    /// the bus for the trace and the boards that watch it, without reading the memory.
    pub(super) fn notify_discarded_read<K: BoardKind>(&self, address: u16, cart: Cart<'_, '_, K>) {
        #[cfg(feature = "ppu-dot-trace")]
        self.dot_trace_bus.set(address);
        self.notify_address_bus(address, cart);
    }

    /// The counterpart for the address lines moving with **/RD high**: a `$2007` write, or a
    /// `$2006` write loading `v`. A12 is an address line, so an A12 counter must see both, and a
    /// board whose rule is about /RD — the MMC5's in-frame detection — must not
    /// (`Cartridge::watches_ppu_bus_without_read`).
    pub(super) fn notify_address_bus_without_read<K: BoardKind>(
        &self,
        address: u16,
        cart: Cart<'_, '_, K>,
    ) {
        #[cfg(feature = "ppu-dot-trace")]
        self.dot_trace_bus.set(address);
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.on_ppu_address_bus_without_read_as::<K>(address, self.ppu_clock);
        }
    }

    /// Overrides the PAL `$2001` switch delays (feature `ppu-dot-trace`, measurement only): ten
    /// values, switch on for dot accumulators 0-4 then switch off for 0-4; 0 keeps the model.
    #[cfg(feature = "ppu-dot-trace")]
    pub fn set_pal_switch_override(delays: [i32; 10]) {
        for (slot, value) in PAL_SWITCH_OVERRIDE.iter().zip(delays) {
            slot.store(value, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Overrides the PAL `$2007` read latch (feature `ppu-dot-trace`, measurement only): five
    /// values, dots for accumulators 0-4; 0 keeps the model.
    #[cfg(feature = "ppu-dot-trace")]
    pub fn set_pal_latch_override(dots: [i32; 5]) {
        for (slot, value) in PAL_LATCH_OVERRIDE.iter().zip(dots) {
            slot.store(value, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Starts a per-dot trace (feature `ppu-dot-trace`), dropping any previous one.
    #[cfg(feature = "ppu-dot-trace")]
    pub fn start_dot_trace(&mut self) {
        self.dot_trace = Some(Vec::new());
    }

    /// Marks the last traced dot with a CPU register access (feature `ppu-dot-trace`).
    #[cfg(feature = "ppu-dot-trace")]
    fn trace_access(&mut self, access: u32) {
        // Bits 24-27: the dot accumulator at the access, plus one (PAL dot-phase studies).
        #[allow(clippy::cast_sign_loss)] // 0..5.
        let phase = ((self.dot_accumulator + 1) as u32 & 0x0F) << 24;
        if let Some(last) = self.dot_trace.as_mut().and_then(|trace| trace.last_mut()) {
            last.access = access | phase;
        }
    }

    /// Inverts the odd-frame parity (feature `ppu-dot-trace`): a console's power-on parity is
    /// not chosen by the program, and the differential must match the netlist's.
    #[cfg(feature = "ppu-dot-trace")]
    pub fn flip_frame_parity(&mut self) {
        self.odd_frame = !self.odd_frame;
    }

    /// Stops the per-dot trace and returns its records.
    #[cfg(feature = "ppu-dot-trace")]
    pub fn take_dot_trace(&mut self) -> Vec<DotRecord> {
        self.dot_trace.take().unwrap_or_default()
    }

    /// Reads a nametable byte, from console VRAM or from the cartridge when the board
    /// supplies its own nametable (MMC5 `ExRAM` and fill mode).
    ///
    /// There is no side effect to worry about here, so the tile capture of REQ-CORE-037
    /// goes through the same path as rendering. The dot loop must hold with no cartridge
    /// attached (Q-062).
    #[must_use]
    pub fn read_nametable_byte(&self, address: u16, cartridge: Option<&Cartridge>) -> u8 {
        self.read_nametable_inner::<AnyBoard>(address, cartridge)
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
    ///
    /// **Generic over the board kind since lot MAPPERS-N** (owner's go-ahead, 2026-11-15; this
    /// file was outside that lot's scope, so the backport should re-read this note). Before, there
    /// was one copy for every path and it held the whole `Board::read_nametable_from_vram`
    /// dispatch: each board that serves its own nametables added an arm there, and every game
    /// paid for it on every nametable fetch. Callgrind of clean builds, `core` of Mega Man 2 and
    /// Zelda II (MMC1, specialised path) over 300 frames: mapper 284's arm **+0.045 %**, mapper
    /// 682's alone **+0.025 %**, all of it in this function (+3.1 M instructions), against the
    /// +0.02 % a board may cost. Now a specialised kind gets a copy whose served-nametable branch
    /// calls its own board ([`Cartridge::read_nametable_from_vram_as`]) and the arms stay on the
    /// `AnyBoard` copy, the generic path of the boards without a specialised loop. Still
    /// `#[inline(never)]`, one copy per kind, for the MF-2 reason above.
    #[inline(never)]
    fn read_nametable_inner<K: BoardKind>(
        &self,
        address: u16,
        cartridge: Option<&Cartridge>,
    ) -> u8 {
        self.read_nametable_as::<K>(address, cartridge, self.layout_of(cartridge))
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
    fn read_nametable_as<K: BoardKind>(
        &self,
        address: u16,
        cartridge: Option<&Cartridge>,
        layout: NametableMirroring,
    ) -> u8 {
        let page = self.nametable_page(address, cartridge);
        // Lot MAPPERS-N: `K::SERVES_NAMETABLES` is a constant, so on a kind whose board never
        // serves its tables (MMC1, MMC3, NROM...) this whole branch is compiled out; see
        // `BoardKind::SERVES_NAMETABLES` and `read_nametable_inner`.
        if K::SERVES_NAMETABLES && page == Cartridge::SERVED_BY_CARTRIDGE {
            if let Some(cartridge) = cartridge {
                // Lot MF-LEG-2: the MMC5 answers all four tables in extended attribute mode
                // and with the vertical split on, CIRAM's included, so it is handed the VRAM
                // it would otherwise not be able to read. Every other board's arm is
                // `read_nametable`'s, unchanged — see `Mapper::read_nametable_from_vram`. This
                // is the **whole** of the two features' footprint on the PPU's fetch path, and
                // a board that does not take this branch cannot see it.
                #[cfg(feature = "board-mmc5-exattr-split")]
                return cartridge.read_nametable_from_vram_as::<K>(address, &self.vram);
                #[cfg(not(feature = "board-mmc5-exattr-split"))]
                return cartridge.read_nametable_as::<K>(address);
            }
        }

        if let Ok(page) = usize::try_from(page) {
            return self.vram[(page << 10) | (usize::from(address) & 0x03FF)];
        }

        self.vram[map_nametable(address, layout)]
    }

    /// The rendered frame a nametable byte was last written in (0: not since power-on or the
    /// last state load; always 0 for nametables the cartridge serves), under `layout`.
    fn nametable_written_as<K: BoardKind>(
        &self,
        address: u16,
        cartridge: Option<&Cartridge>,
        layout: NametableMirroring,
    ) -> u32 {
        let page = self.nametable_page(address, cartridge);
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            // Lot MF-LEG-2, and the same reason as in `write_nametable_byte`: a table the MMC5
            // answers may still be CIRAM's, and a margin decoder asking when a byte was last
            // written has to be told about it rather than given the 0 that means "never".
            #[cfg(feature = "board-mmc5-exattr-split")]
            if let Some(offset) =
                cartridge.and_then(|cartridge| cartridge.nametable_write_offset_as::<K>(address))
            {
                return self.vram_written.get(offset).copied().unwrap_or(0);
            }

            return 0;
        }
        let offset = usize::try_from(page).map_or_else(
            |_| map_nametable(address, layout),
            |page| (page << 10) | (usize::from(address) & 0x03FF),
        );
        self.vram_written.get(offset).copied().unwrap_or(0)
    }

    fn write_nametable_byte<K: BoardKind>(
        &mut self,
        address: u16,
        value: u8,
        cart: Cart<'_, '_, K>,
    ) {
        let page = self.nametable_page(address, cart.as_deref());
        if page == Cartridge::SERVED_BY_CARTRIDGE {
            // Lot MF-LEG-2: with the MMC5 answering every table, a `$2007` write to one that
            // CIRAM still backs must reach CIRAM all the same — the board says where
            // (`Mapper::nametable_write_offset`), because the page number the PPU would have
            // used is the one it has just been told not to trust. A cold path: a game writes a
            // nametable byte a few hundred times a frame, against the 17 000 fetches of one.
            #[cfg(feature = "board-mmc5-exattr-split")]
            if let Some(offset) = cart
                .as_deref()
                .and_then(|cartridge| cartridge.nametable_write_offset_as::<K>(address))
            {
                self.vram[offset] = value;
                if let Some(written) = self.vram_written.get_mut(offset) {
                    *written = self.vram_clock;
                }

                return;
            }

            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.write_nametable_as::<K>(address, value);
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

/// Lot FIDPLUS-E: an OAM bit's retention without a refresh, in microseconds, at the quantiles
/// `(k + 0.5) / 256` of the losses measured on `FireSide`'s NES with `oamdecay.nes` (2026-11-16;
/// both passes pooled, 1838 losses, read every 1-30 frames and linear in between; NTSC frames),
/// with a 125 ms floor for blargg's `oam_stress`.
/// See [`Ppu2C02::refresh_oam_row`].
const OAM_RETENTION_US: [u32; 256] = [
    125_000, 140_795, 167_413, 175_120, 182_827, 190_535, 198_242, 204_306,
    209_995, 215_683, 221_372, 227_061, 232_750, 238_439, 244_127, 250_438,
    271_771, 293_104, 314_437, 333_731, 340_480, 347_230, 353_979, 360_729,
    367_478, 374_228, 380_977, 387_727, 394_476, 401_225, 407_975, 414_724,
    421_474, 428_223, 434_973, 441_722, 448_472, 455_221, 461_971, 468_720,
    475_469, 482_219, 488_968, 495_718, 501_368, 505_859, 510_350, 514_841,
    519_332, 523_824, 528_315, 532_806, 537_297, 541_788, 546_279, 550_771,
    555_262, 559_753, 564_244, 568_735, 573_227, 577_718, 582_209, 586_700,
    591_191, 595_682, 600_174, 604_665, 609_156, 613_647, 618_138, 622_630,
    627_121, 631_612, 636_103, 640_594, 645_085, 649_577, 654_068, 658_559,
    663_050, 667_541, 672_032, 676_524, 681_015, 685_506, 689_997, 694_488,
    698_980, 703_471, 707_962, 712_453, 716_944, 721_435, 725_927, 730_418,
    734_909, 739_400, 743_891, 748_383, 752_225, 756_005, 759_786, 763_566,
    767_347, 771_127, 774_908, 778_688, 782_469, 786_250, 790_030, 793_811,
    797_591, 801_372, 805_152, 808_933, 812_713, 816_494, 820_274, 824_055,
    827_836, 831_616, 835_397, 839_177, 842_958, 846_738, 850_519, 854_299,
    858_080, 861_861, 865_641, 869_422, 873_202, 876_983, 880_763, 884_544,
    888_324, 892_105, 895_885, 899_666, 903_447, 907_227, 911_008, 914_788,
    918_569, 922_349, 926_130, 929_910, 933_691, 937_472, 941_252, 945_033,
    948_813, 952_594, 956_374, 960_155, 963_935, 967_716, 971_496, 975_277,
    979_058, 982_838, 986_619, 990_399, 994_180, 997_960, 1_003_298, 1_008_820,
    1_014_342, 1_019_865, 1_025_387, 1_030_909, 1_036_432, 1_041_954, 1_047_476, 1_052_998,
    1_058_521, 1_064_043, 1_069_565, 1_075_088, 1_080_610, 1_086_132, 1_091_654, 1_097_177,
    1_102_699, 1_108_221, 1_113_743, 1_119_266, 1_124_788, 1_130_310, 1_135_833, 1_141_355,
    1_146_877, 1_152_399, 1_157_922, 1_163_444, 1_168_966, 1_174_489, 1_180_011, 1_185_533,
    1_191_055, 1_196_578, 1_202_100, 1_207_622, 1_213_145, 1_218_667, 1_224_189, 1_229_711,
    1_235_234, 1_240_756, 1_246_278, 1_251_801, 1_257_323, 1_262_845, 1_268_367, 1_273_890,
    1_279_412, 1_284_934, 1_290_457, 1_295_979, 1_301_501, 1_307_023, 1_312_546, 1_318_068,
    1_323_590, 1_329_113, 1_334_635, 1_340_157, 1_345_679, 1_351_202, 1_356_724, 1_362_246,
    1_367_768, 1_373_291, 1_378_813, 1_384_335, 1_389_858, 1_395_380, 1_400_902, 1_406_424,
    1_411_947, 1_417_469, 1_422_991, 1_428_514, 1_434_036, 1_439_558, 1_445_080, 1_450_603,
    1_456_125, 1_461_647, 1_467_170, 1_472_692, 1_478_214, 1_483_736, 1_489_259, 1_494_781,
];

/// Where a decayed OAM bit settles.
#[derive(Clone, Copy)]
enum OamRest {
    Zero,
    One,
    /// Lost whatever it held: found changed after both an all-0 and an all-1 fill.
    Flips,
}

/// The resting class of a bit drawn by [`oam_bit_draw`], in `FireSide`'s proportions (1800 bits
/// outside row 31: 1406 lost to 0, 432 to 1, so 38 in both passes): 1368 / 394 / 38 of 1800 as
/// 778 / 224 / 22 of 1024.
const fn oam_bit_rest(draw: u64) -> OamRest {
    match (draw >> 8) & 0x3FF {
        0..778 => OamRest::Zero,
        778..1002 => OamRest::One,
        _ => OamRest::Flips,
    }
}

/// A fixed draw for OAM bit `bit`: its retention quantile in the low byte, its resting class in
/// bits 8-17 ([`oam_bit_rest`]) (`SplitMix64`).
const fn oam_bit_draw(bit: usize) -> u64 {
    let mut z = (bit as u64).wrapping_add(0x9E37_79B9_7F4A_7C15).wrapping_mul(0x2C07);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(feature = "ppu-dot-trace")]
static PAL_SWITCH_OVERRIDE: [std::sync::atomic::AtomicI32; 10] =
    [const { std::sync::atomic::AtomicI32::new(0) }; 10];

/// The measurement override of [`Ppu2C02::set_pal_switch_override`].
#[cfg(feature = "ppu-dot-trace")]
fn pal_switch_override(region: NesRegion, accumulator: i32, value: u8, delay: i32) -> i32 {
    if region != NesRegion::Pal {
        return delay;
    }
    #[allow(clippy::cast_sign_loss)] // 0..5.
    let slot = accumulator as usize + if (value & 0x18) == 0 { 5 } else { 0 };
    match PAL_SWITCH_OVERRIDE[slot].load(std::sync::atomic::Ordering::Relaxed) {
        0 => delay,
        forced => forced,
    }
}

#[cfg(feature = "ppu-dot-trace")]
static PAL_LATCH_OVERRIDE: [std::sync::atomic::AtomicI32; 5] =
    [const { std::sync::atomic::AtomicI32::new(0) }; 5];

/// The measurement override of [`Ppu2C02::set_pal_latch_override`].
#[cfg(feature = "ppu-dot-trace")]
fn pal_latch_override(region: NesRegion, accumulator: i32, dots: i32) -> i32 {
    if region != NesRegion::Pal {
        return dots;
    }
    #[allow(clippy::cast_sign_loss)] // 0..5.
    match PAL_LATCH_OVERRIDE[accumulator as usize].load(std::sync::atomic::Ordering::Relaxed) {
        0 => dots,
        forced => forced,
    }
}
