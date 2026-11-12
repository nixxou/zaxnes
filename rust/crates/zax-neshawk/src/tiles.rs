//! The 3D tile capture: the per-frame [`TileRecord`]s the renderer reads, taken off NesHawk's
//! machine.
//!
//! **This module is not a port of anything.** NesHawk has no tile capture — it is Zaxnes's own
//! addition, specified by SPEC-02 §4.1 and REQ-CORE-030 to REQ-CORE-046 and implemented for
//! Zaxnes's PPU in `zax_nes::ppu::tiles`. What is ported is the *machine* the capture reads; the
//! capture rule is the spec's, and this file follows `zax-nes`'s implementation of it so that the
//! two engines produce the same records for the same frame. That is what the parity measurement
//! of `zax-harness`'s `neshawk-tiles` checks, exactly as lot FCEUX-6's `fceux-tiles` did for
//! FCEUX.
//!
//! # The rule, in three sentences
//!
//! * **Sprites** of band `line / 8` are captured at the **first dot** of visible line `line`
//!   when `line & 7 == 7` and sprites are on, by OAM order (REQ-CORE-044).
//! * **The background row** of visible line `line` is captured at its **last dot**, when the
//!   background is on and the fine Y of the row-start shadow is 1 (AMD-021 R2-005): the row shows
//!   the state every CPU write made *during* that line left, which is why it is taken at the end.
//! * The row-start shadow is `v` shorn of the two end-of-line reloads — the dot-256 vertical
//!   increment and the dot-257 horizontal reload land in [`TileCapture::v_row_next`] instead and
//!   only become the shadow once the line is over. A coarse-X increment never reaches either.
//!
//! # What a capture must not do
//!
//! Change the machine. Every read here goes through `Board::peek_ppu`, the side-effect-free path
//! (REQ-CORE-033), so a frame runs identically whether records are built or not — which the
//! crate's own test asserts by running the same movie with the capture on and off and comparing
//! the work RAM.

use zax_core_api::TileRecord;

use crate::nes::Nes;

/// Visible scanlines, `zax_nes::ppu::VISIBLE_SCANLINES`.
const VISIBLE_SCANLINES: i32 = 240;

/// Screen width in pixels.
const SCREEN_WIDTH: i32 = 256;

/// Records a standard frame's background row holds: 32 columns, or 33 when fine X is not 0
/// (REQ-CORE-038).
const ROW_WIDTH: i32 = 33;

/// The capture's own state, which lives on the PPU because that is what it reads.
#[derive(Debug, Clone, Default)]
pub struct TileCapture {
    /// Whether records are being built. Off by default: a host that only wants a picture — the
    /// harness's tools, the test-ROM readers — pays nothing.
    pub enabled: bool,
    /// The frame's records, in capture order (REQ-CORE-044).
    pub records: Vec<TileRecord>,
    /// `v` shorn of the two end-of-line reloads: the address the row of the current line is read
    /// from (AMD-021 R2-005). See this module's header.
    pub v_row_start: u16,
    /// Where those two reloads go, and what [`Self::v_row_start`] becomes once the line is over.
    pub v_row_next: u16,
}

// `v_row_start`/`v_row_next` are machine state and are saved; `records` is a frame's output, like
// the frame buffer, and `enabled` is the host's choice, which a restored state must not change.
crate::state::sync_struct!(TileCapture {
    v_row_start, v_row_next,
} skip: {
    enabled, records,
});

impl crate::ppu::PpuRegs {
    /// The 15-bit `v` NesHawk keeps split across `fv`, `v`, `h`, `vt` and `ht`.
    ///
    /// Not [`crate::ppu::PpuRegs::get_2007access`], which masks fine Y to two bits because it is
    /// composing a PPU *address*; the capture wants the register, whose fine Y is three bits.
    #[must_use]
    pub const fn compose_v(&self) -> u16 {
        (((self.fv & 7) << 12)
            | ((self.v & 1) << 11)
            | ((self.h & 1) << 10)
            | ((self.vt & 31) << 5)
            | (self.ht & 31)) as u16
    }
}

impl crate::ppu::Ppu {
    /// A reload that reaches the row of the line it happens in: the `$2006` install, the `$2007`
    /// auto-increment, and the vertical reload of the pre-render line (which captures nothing, so
    /// it may as well take effect at once).
    pub(crate) fn capture_shadow_both(&mut self) {
        if !self.tiles.enabled {
            return;
        }
        let v = self.ppur.compose_v();
        self.tiles.v_row_start = v;
        self.tiles.v_row_next = v;
    }

    /// A reload that belongs to the **next** line: the vertical increment of dot 256 and the
    /// horizontal reload of dot 257 (AMD-021 R2-005).
    pub(crate) fn capture_shadow_next(&mut self) {
        if !self.tiles.enabled {
            return;
        }
        self.tiles.v_row_next = self.ppur.compose_v();
    }

    /// Whether the background is being shown (`PPUMASK` bit 3).
    const fn background_enabled(&self) -> bool {
        self.reg_2001.show_bg != 0
    }

    /// Whether sprites are being shown (`PPUMASK` bit 4).
    const fn sprites_enabled(&self) -> bool {
        self.reg_2001.show_obj != 0
    }
}

impl Nes {
    /// Whether the capture is running.
    #[must_use]
    pub const fn tile_capture_enabled(&self) -> bool {
        self.b.ppu.tiles.enabled
    }

    /// Turns the capture on or off. Turning it on seeds the row-start shadow from the live `v`,
    /// so that a capture switched on mid-frame does not read a stale address.
    pub fn set_tile_capture(&mut self, on: bool) {
        self.b.ppu.tiles.enabled = on;
        if on {
            let v = self.b.ppu.ppur.compose_v();
            self.b.ppu.tiles.v_row_start = v;
            self.b.ppu.tiles.v_row_next = v;
        } else {
            self.b.ppu.tiles.records.clear();
        }
    }

    /// The records of the frame just run, in capture order (REQ-CORE-030, REQ-CORE-044). Empty
    /// when the capture is off.
    #[must_use]
    pub fn tile_records(&self) -> &[TileRecord] {
        &self.b.ppu.tiles.records
    }

    /// Clears the frame's records, which `FrameAdvance` does at the start of a frame.
    pub(crate) fn begin_capture_frame(&mut self) {
        if self.b.ppu.tiles.enabled {
            self.b.ppu.tiles.records.clear();
        }
    }

    /// The captures owed by visible line `line` at its first dot: the sprite band.
    ///
    /// `zax_nes`'s `begin_capture_line` also computes the REQ-CORE-046 masks here; those are the
    /// margin decoder's, which is Zaxnes's bus, and a NesHawk frame reports 0 for both.
    pub(crate) fn begin_capture_line(&mut self, line: i32) {
        if !self.b.ppu.tiles.enabled {
            return;
        }
        if self.b.ppu.sprites_enabled() && (line & 7) == 7 {
            self.capture_sprites(line);
        }
    }

    /// The background row owed by visible line `line`, at its last dot (R2-005), and then the
    /// row-start shadow moving on to the next line.
    pub(crate) fn end_capture_line(&mut self, line: i32) {
        if !self.b.ppu.tiles.enabled {
            return;
        }
        if self.b.ppu.background_enabled() {
            self.capture_background_row(line);
        }
        self.b.ppu.tiles.v_row_start = self.b.ppu.tiles.v_row_next;
    }

    /// One byte of PPU address space, through the side-effect-free path (REQ-CORE-033).
    fn peek_ppu(&self, addr: i32) -> u8 {
        self.b.board.peek_ppu(addr & 0x3FFF, &self.b.ciram)
    }

    /// The 16 CHR bytes of the pattern at `address`, packed as REQ-CORE-034 requires.
    fn pattern_planes(&self, address: i32) -> (u64, u64) {
        let mut plane0 = 0u64;
        let mut plane1 = 0u64;
        for row in 0..8 {
            plane0 |= u64::from(self.peek_ppu(address + row)) << (8 * row);
            plane1 |= u64::from(self.peek_ppu(address + 8 + row)) << (8 * row);
        }
        (plane0, plane1)
    }

    /// `zax_nes::ppu::tiles`'s `capture_background_row`, on NesHawk's state.
    fn capture_background_row(&mut self, line: i32) {
        let shadow = self.b.ppu.tiles.v_row_start;
        // AMD-021 R2-005: the row is the left-edge shadow of the address as the line ends.
        if ((shadow >> 12) & 0x07) != 1 {
            return;
        }
        // The exception of the original: no row on line 0 under vertical mirroring
        // (REQ-CORE-037). NesHawk has no `NametableMirroring`, so the board's own mirroring map
        // is read — vertical is `0,1,0,1`, which is what `SetMirrorType(Vertical)` writes.
        if line == 0 && self.mirroring_is_vertical() {
            return;
        }

        let coarse_x = i32::from(shadow & 0x1F);
        let coarse_y = i32::from((shadow >> 5) & 0x1F);
        let nametable = i32::from((shadow >> 10) & 0x03);
        let fine_x_scroll = self.b.ppu.ppur.fh;
        let pattern_table = if self.b.ppu.reg_2000.bg_pattern_hi != 0 {
            0x1000
        } else {
            0x0000
        };
        let row_index = line / 8;
        let y = (line - 1) as i16;

        for k in 0..=32 {
            let x = (8 * k) - fine_x_scroll;
            if x >= SCREEN_WIDTH {
                // k = 32 with fine X at 0: a row has 32 records, otherwise 33 (REQ-CORE-038).
                continue;
            }

            let wrapped = (coarse_x + k).rem_euclid(64);
            let mut column = wrapped;
            let mut table = nametable;
            if column > 31 {
                // The horizontally neighbouring nametable, the board's mirroring then applied.
                table ^= 1;
                column -= 32;
            }

            let nametable_address = 0x2000 | (table << 10) | (coarse_y << 5) | column;
            let tile = i32::from(self.peek_ppu(nametable_address));
            let attribute_address =
                0x23C0 | (table << 10) | ((coarse_y & 0x1C) << 1) | (column >> 2);
            let attribute = self.peek_ppu(attribute_address);
            let shift = ((coarse_y & 0x02) << 1) | (column & 0x02);
            let palette = (attribute >> shift) & 0x03;

            let (plane0, plane1) = self.pattern_planes(pattern_table + (tile * 16));

            let record = TileRecord::new(
                plane0,
                plane1,
                x as i16,
                y,
                (row_index * ROW_WIDTH) + k,
                false,
                false,
                false,
                palette,
                true,
            );
            self.b.ppu.tiles.records.push(record);
        }
    }

    /// Whether the board's nametable map is the `0,1,0,1` of vertical mirroring, which
    /// REQ-CORE-037's exception is about.
    fn mirroring_is_vertical(&self) -> bool {
        let map = [
            self.b.board.base().apply_mirroring(0x2000) >> 10,
            self.b.board.base().apply_mirroring(0x2400) >> 10,
            self.b.board.base().apply_mirroring(0x2800) >> 10,
            self.b.board.base().apply_mirroring(0x2C00) >> 10,
        ];
        map == [0, 1, 0, 1]
    }

    /// `zax_nes::ppu::tiles`'s `capture_sprites`, on NesHawk's OAM.
    ///
    /// NesHawk's `allow_more_than_eight_sprites` does not reach this: the capture emits every
    /// sprite whose band this is, as `zax-nes`'s does, because a record the renderer cannot see
    /// is the renderer's business and not the capture's (REQ-CORE-036 — empty and hidden records
    /// are still emitted, and filtering is the consumer's job).
    fn capture_sprites(&mut self, line: i32) {
        let band = line / 8;
        let sprite_table = if self.b.ppu.reg_2000.obj_pattern_hi != 0 {
            0x1000
        } else {
            0x0000
        };
        let tall = self.b.ppu.reg_2000.obj_size_16 != 0;

        for n in 0..64usize {
            let oam_y = self.b.ppu.oam[n * 4];
            let top = i32::from(oam_y) + 1;
            if top >= VISIBLE_SCANLINES || top / 8 != band {
                continue;
            }

            let tile = i32::from(self.b.ppu.oam[(n * 4) + 1]);
            let attribute = self.b.ppu.oam[(n * 4) + 2];
            let oam_x = self.b.ppu.oam[(n * 4) + 3];
            let mirror_h = (attribute & 0x40) != 0;
            let mirror_v = (attribute & 0x80) != 0;
            let hidden = (attribute & 0x20) != 0;
            let palette = attribute & 0x03;
            let x = i16::from(oam_x);
            let top = i16::from(oam_y) + 1;
            let index = n as i32;

            if !tall {
                self.emit_sprite(
                    index,
                    x,
                    top,
                    sprite_table + (tile * 16),
                    mirror_h,
                    mirror_v,
                    hidden,
                    palette,
                );
                continue;
            }

            // 8x16: the table comes from bit 0 of the tile number and the pair is
            // (T & $FE, T | 1); a vertical flip swaps the halves as well as flipping each.
            let tall_table = if (tile & 1) != 0 { 0x1000 } else { 0x0000 };
            let upper = if mirror_v { tile | 1 } else { tile & 0xFE };
            let lower = if mirror_v { tile & 0xFE } else { tile | 1 };
            self.emit_sprite(
                index,
                x,
                top,
                tall_table + (upper * 16),
                mirror_h,
                mirror_v,
                hidden,
                palette,
            );
            if i32::from(top) + 8 < VISIBLE_SCANLINES {
                self.emit_sprite(
                    64 + index,
                    x,
                    top + 8,
                    tall_table + (lower * 16),
                    mirror_h,
                    mirror_v,
                    hidden,
                    palette,
                );
            }
        }
    }

    /// One sprite record.
    #[allow(clippy::too_many_arguments)] // The ten fields of SPEC-02 §4.1, as `zax-nes` has them.
    #[allow(clippy::fn_params_excessive_bools)] // Same.
    fn emit_sprite(
        &mut self,
        index: i32,
        x: i16,
        y: i16,
        pattern_address: i32,
        mirror_h: bool,
        mirror_v: bool,
        hidden: bool,
        palette: u8,
    ) {
        let (plane0, plane1) = self.pattern_planes(pattern_address);
        self.b.ppu.tiles.records.push(TileRecord::new(
            plane0, plane1, x, y, index, mirror_h, mirror_v, hidden, palette, false,
        ));
    }
}
