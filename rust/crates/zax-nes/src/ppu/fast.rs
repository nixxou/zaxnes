//! The fast core's PPU paths (lot FAST-1, `docs/design/FAST-CORE.md`).
//!
//! Step 1 of the PPU part: the visible span of a rendering line (dots 2 to 255) eight dots at a
//! time instead of one. The result is **the same machine** as [`Ppu2C02::run_visible_span`]:
//! the same pixels, the same latches, shifters, `v` and sprite evaluation state after the span,
//! so `golden-core` stays byte-identical and a fast span can stop anywhere a dot can.
//!
//! # Why eight dots at a time is exact
//!
//! Inside the span a dot does: shift the four background shifters, one step of the fetch
//! pipeline (phase `(dot - 1) & 7`: 0 reloads the shifters' low bytes and reads the nametable,
//! 2 the attribute, 4 and 6 the two pattern planes, 7 moves `v` to the next tile), one step of
//! sprite evaluation from dot 65, and one pixel. Take a group of eight dots starting at a phase-0
//! dot `d`. With `r` a shifter before dot `d` and `l` its latch, dot `d` makes it
//! `n = ((r << 1) & 0xFF00) | l`, and dot `d + j` shows bit `15 - fine_x` of `n << j`, that is bit
//! `15 - fine_x - j` of `n` (`fine_x + j` is at most 14). After the group the shifter is
//! `n << 7`. The fetches of the group only write the latches and `v`, which no pixel of the group
//! reads (the pixels read the shifters, loaded at dot `d` from the latches of the group before).
//! So the group's eight pixels come from `n`, its fetches run in their own order, and the sprite
//! evaluation, which looks at neither, runs its dots as before.

use super::{Ppu2C02, OPAQUE_PIXEL, SCREEN_WIDTH};
use crate::cartridges::{BoardKind, Cartridge};

/// `SPREAD[b]`: the eight bits of `b` spread over the eight bytes of a `u64`, the leftmost pixel
/// (bit 7) in the lowest byte, each byte 0 or 1.
const SPREAD: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut value = 0u64;
        let mut pixel = 0;
        while pixel < 8 {
            value |= (((byte >> (7 - pixel)) & 1) as u64) << (pixel * 8);
            pixel += 1;
        }
        table[byte] = value;
        byte += 1;
    }
    table
};

/// The eight 4-bit background entries (`palette << 2 | pixel`) of the two bytes of four
/// shifters, the high byte's pixels in the low half: `(high byte's, low byte's)`.
#[allow(clippy::inline_always)] // Once per eight pixels on the fast path.
#[inline(always)]
fn entries(low: u16, high: u16, attribute_low: u16, attribute_high: u16) -> u128 {
    let byte = |value: u16, shift: u16| usize::from((value >> shift) & 0xFF);
    let half = |shift: u16| {
        SPREAD[byte(low, shift)]
            | (SPREAD[byte(high, shift)] << 1)
            | (SPREAD[byte(attribute_low, shift)] << 2)
            | (SPREAD[byte(attribute_high, shift)] << 3)
    };
    u128::from(half(8)) | (u128::from(half(0)) << 64)
}

impl Ppu2C02 {
    /// [`Ppu2C02::run_visible_span`] for the fast core: the dots before the next phase-0 dot and
    /// after the last whole group run as before, the whole groups between run eight dots at a
    /// time. Same returns: `false`, having run nothing, when the cartridge's fetches cannot be
    /// read directly.
    pub(super) fn run_visible_span_fast<K: BoardKind>(
        &mut self,
        count: i64,
        cartridge: Option<&Cartridge>,
    ) -> bool {
        let Some(cartridge_ref) = cartridge else {
            return false;
        };
        if cartridge_ref.chr_view_as::<K>().is_none() {
            return false;
        }

        // Dots up to the next phase-0 dot (`dot + 1 ≡ 1 mod 8`, so `dot ≡ 0 mod 8`).
        let head = i64::from((8 - (self.dot & 7)) & 7).min(count);
        if head > 0 && !self.run_visible_span::<K>(head, cartridge) {
            return false;
        }
        let groups = (count - head) / 8;
        if groups > 0 {
            self.run_span_groups::<K>(groups, cartridge_ref);
        }
        let tail = count - head - groups * 8;
        if tail > 0 {
            self.run_visible_span::<K>(tail, cartridge);
        }
        true
    }

    /// `groups` whole groups of eight dots of a visible line, from a phase-0 dot.
    fn run_span_groups<K: BoardKind>(&mut self, groups: i64, cartridge: &Cartridge) {
        let Some(chr) = cartridge.chr_view_as::<K>() else {
            return;
        };

        // The start of each logical nametable in console VRAM, as `run_visible_span` finds it
        // (it accepted the cartridge, so no table is served by it).
        let layout = self.layout_of(Some(cartridge));
        let mut nametables = [0usize; 4];
        for (table, start) in (0u16..).zip(nametables.iter_mut()) {
            let page = self.nametable_page(0x2000 + (table << 10), Some(cartridge));
            *start = usize::try_from(page).map_or_else(
                |_| super::map_nametable(0x2000 + (table << 10), layout),
                |page| page << 10,
            );
        }
        let nametable_byte = |vram: &[u8], address: u16| {
            vram[nametables[usize::from((address >> 10) & 0x03)] | (usize::from(address) & 0x03FF)]
        };

        let background = (self.mask & 0x08) != 0;
        let sprites = (self.mask & 0x10) != 0;
        let color_mask = OPAQUE_PIXEL | if (self.mask & 0x01) != 0 { 0x30 } else { 0x3F };
        let fine_x = u32::from(self.fine_x & 0x07);
        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let row = self.scanline as usize * SCREEN_WIDTH;

        // The colour of a background entry with no sprite over it, for the whole span.
        let mut background_colors = [0u8; 16];
        for (entry, color) in (0u8..).zip(background_colors.iter_mut()) {
            let entry = if background && (entry & 0x03) != 0 { entry } else { 0 };
            *color = self.palette_marked[usize::from(entry)] & color_mask;
        }
        let evaluation_from = (self.dot + 1).max(65);

        for _ in 0..groups {
            let first = self.dot + 1;

            // Dot `first`'s shift and reload, then the group's eight pixels from them.
            let attribute_low = if (self.attribute_latch & 1) != 0 { 0xFF } else { 0x00 };
            let attribute_high = if (self.attribute_latch & 2) != 0 { 0xFF } else { 0x00 };
            let low = ((self.pattern_shift_low << 1) & 0xFF00) | u16::from(self.pattern_low_latch);
            let high =
                ((self.pattern_shift_high << 1) & 0xFF00) | u16::from(self.pattern_high_latch);
            let attr_low = ((self.attribute_shift_low << 1) & 0xFF00) | attribute_low;
            let attr_high = ((self.attribute_shift_high << 1) & 0xFF00) | attribute_high;
            #[allow(clippy::cast_possible_truncation)] // Eight bytes of the window.
            let window = (entries(low, high, attr_low, attr_high) >> (8 * fine_x)) as u64;
            self.pattern_shift_low = low << 7;
            self.pattern_shift_high = high << 7;
            self.attribute_shift_low = attr_low << 7;
            self.attribute_shift_high = attr_high << 7;

            if sprites && self.sprite_line_stale {
                self.rebuild_sprite_line();
            }
            #[allow(clippy::cast_sign_loss)] // first is 9..=241 here.
            let x0 = (first - 1) as usize;
            let entries = window.to_le_bytes();
            let mut cells = [0u8; 8];
            if sprites {
                cells.copy_from_slice(&self.sprite_line[x0..x0 + 8]);
            }
            let mut colors = [0u8; 8];
            if u64::from_le_bytes(cells) == 0 {
                for (color, entry) in colors.iter_mut().zip(entries) {
                    *color = background_colors[usize::from(entry & 0x0F)];
                }
            } else {
                for (j, color) in colors.iter_mut().enumerate() {
                    let entry = if background { entries[j] } else { 0 };
                    *color = self.compose_span_pixel(entry, cells[j], x0 + j, color_mask);
                }
            }
            if let Some(out) = self.pixels.get_mut(row + x0..row + x0 + 8) {
                out.copy_from_slice(&colors);
            }

            // The group's fetches (phases 0, 2, 4, 6, 7), into the latches and `v`.
            self.nametable_latch = nametable_byte(&self.vram, 0x2000 | (self.v & 0x0FFF));
            let address =
                0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
            let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
            self.attribute_latch = (nametable_byte(&self.vram, address) >> shift) & 0x03;
            self.pattern_low_latch = chr.read(self.pattern_address(0));
            self.pattern_high_latch = chr.read(self.pattern_address(8));
            self.increment_coarse_x();

            self.dot += 8;
            self.ppu_clock += 8;
        }

        // Sprite evaluation looks at none of the above (it reads OAM and writes the secondary
        // OAM, which no pixel of this line reads): its dots run after the groups, in order.
        if self.dot >= evaluation_from {
            self.run_sprite_evaluation_range(evaluation_from, self.dot);
        }
    }

    /// The colour of pixel `x` of a span from its background entry and its sprite cell, as
    /// `span_pixel` composes it.
    #[allow(clippy::inline_always)] // Once a pixel on the fast path.
    #[inline(always)]
    fn compose_span_pixel(&mut self, background: u8, sprite: u8, x: usize, color_mask: u8) -> u8 {
        let background_opaque = (background & 0x03) != 0;
        if (sprite & super::render::SPRITE_ZERO) != 0
            && background_opaque
            && x != SCREEN_WIDTH - 1
        {
            self.sprite_zero_hit = true;
        }
        let entry = if (sprite & super::render::SPRITE_OPAQUE) != 0
            && (!background_opaque || (sprite & super::render::SPRITE_BEHIND) == 0)
        {
            0x10 | (sprite & 0x0F)
        } else if background_opaque {
            background
        } else {
            0
        };
        self.palette_marked[usize::from(entry & 0x1F)] & color_mask
    }

    /// Dots `first` to `last` (both at least 65) of the sprite evaluation, as that many
    /// [`Ppu2C02::run_sprite_evaluation`] calls would run them. A whole pair of dots (odd, then
    /// even) of an aligned evaluation takes a sprite at once: one pair for a sprite not on the
    /// line, four for one that is copied. Anything else (dot 65's start, a misaligned address, a
    /// full secondary OAM, a pair cut by the range) runs dot by dot.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn run_sprite_evaluation_range(&mut self, first: i32, last: i32) {
        let mut dot = first;
        while dot <= last {
            let pair = (dot & 1) == 1 && dot > 65 && dot < last;
            if pair && self.eval_finished {
                // The copy is over: odd dots read OAM into the buffer, even dots move the sprite
                // half and read the secondary OAM back into it, so only the last pair shows.
                let pairs = (last - dot + 1) / 2;
                self.eval_sprite = (self.eval_sprite + pairs) & 0x3F;
                #[allow(clippy::cast_sign_loss)] // Masked to 0..32.
                let slot = (self.secondary_oam_addr & 0x1F) as usize;
                self.oam_eval_latch = self.secondary_oam[slot];
                dot += pairs * 2;
                continue;
            }
            if pair
                && self.eval_byte == 0
                && !self.sprite_in_range
                && self.secondary_oam_addr < 32
                && self.secondary_oam_addr & 3 == 0
            {
                #[allow(clippy::cast_sign_loss)] // 0..64.
                let sprite = self.eval_sprite as usize;
                let y = self.oam[sprite * 4];
                #[allow(clippy::cast_sign_loss)] // 0..32.
                let slot = self.secondary_oam_addr as usize;
                if !(self.in_sprite_range(y) && !self.sprite_in_margin()) {
                    self.oam_eval_latch = y;
                    self.secondary_oam[slot] = y;
                    self.next_sprite();
                    dot += 2;
                    continue;
                }
                if dot + 7 <= last {
                    if dot + 1 == 66 {
                        self.sprite_zero_found = true;
                    }
                    self.secondary_oam[slot..slot + 4]
                        .copy_from_slice(&self.oam[sprite * 4..sprite * 4 + 4]);
                    #[allow(clippy::cast_possible_truncation)] // 0..64.
                    if let Some(source) = self.secondary_source.get_mut(slot >> 2) {
                        *source = sprite as u8;
                    }
                    self.oam_eval_latch = self.oam[sprite * 4 + 3];
                    self.secondary_oam_addr += 4;
                    self.eval_copy_dot = dot + 7;
                    self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
                    self.eval_finished = self.eval_sprite == 0;
                    self.sprite_in_range = false;
                    self.eval_found = self.secondary_oam_addr >> 2;
                    dot += 8;
                    continue;
                }
            }
            self.run_sprite_evaluation(dot);
            dot += 1;
        }
    }

    /// The evaluation of a build without `ppu-exact-sprite-eval` has its own shape; it runs dot
    /// by dot here.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    fn run_sprite_evaluation_range(&mut self, first: i32, last: i32) {
        for dot in first..=last {
            self.run_sprite_evaluation(dot);
        }
    }
}
