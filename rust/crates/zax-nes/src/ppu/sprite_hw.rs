//! The sprite output units as the hardware has them, for the lines a rendering switched off and
//! on inside a frame has disturbed.
//!
//! A line that renders from end to end needs none of this: each of the eight sprites of a line
//! is drawn at its X, which is what [`Ppu2C02::rebuild_sprite_line`] paints. The hardware gets
//! there with a counter and a shift register per sprite, and what `AccuracyCoin` measures
//! (`Stale Sprite Shift Regs`, `Stale BG Shift Registers` 3 and 4) is what those do when the
//! rendering stops in the middle:
//!
//! * the counter is loaded with the sprite's X during the fetches of dots 257 to 320, and the
//!   shifter with its pattern;
//! * "if the ppu is rendering on dot 339, then the shifter counters are set to counting";
//!   otherwise they stay as they were, "which is likely halted";
//! * counting, the counter goes down by one on every dot of the picture, **rendering or not**,
//!   and at zero the unit is halted;
//! * halted, the unit draws and shifts — **only while the PPU renders**. A unit that reaches
//!   zero during a blanking keeps its pattern and draws it wherever the rendering comes back.
//!
//! The state below is that, kept for the eight units from the first switch-off on a line that
//! renders until a fetch window and a dot 339 have run with the rendering on, which puts every
//! unit back where an undisturbed line has it. It is only brought up to date at a `$2001` write
//! that switches the rendering and when a sprite line is rebuilt — nothing is tested per dot.

use super::{Ppu2C02, SCREEN_WIDTH, SPRITES_PER_LINE, VISIBLE_SCANLINES};

/// The eight sprite output units (see the module).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SpriteUnits {
    /// Whether the units are being followed; the rest is meaningless otherwise.
    disturbed: bool,
    /// Every unit is where an undisturbed line has it at its dot 0.
    clean: bool,
    /// Where the state stands: the PPU clock of dot 0 of its line, the line and the dot.
    line_clock: i64,
    scanline: i32,
    dot: i32,
    /// Dots left before the unit starts drawing.
    wait: [u16; SPRITES_PER_LINE],
    /// Pixels of the pattern already shifted out, 8 when nothing is left.
    used: [u8; SPRITES_PER_LINE],
    halted: [bool; SPRITES_PER_LINE],
    /// Units the fetch window in progress has reloaded.
    reloaded: u8,
}

impl SpriteUnits {
    /// One dot of a line, `visible` or not, of the picture (`renders`) or not.
    fn step(&mut self, visible: bool, renders: bool, dot: i32, enabled: bool, x: [u8; 8]) {
        if visible && (1..=256).contains(&dot) {
            for slot in 0..SPRITES_PER_LINE {
                if !self.halted[slot] {
                    if self.wait[slot] != 0 {
                        self.wait[slot] -= 1;
                        continue;
                    }
                    self.halted[slot] = true;
                }
                if enabled && self.used[slot] < 8 {
                    self.used[slot] += 1;
                }
            }
            return;
        }
        if dot == 257 {
            self.reloaded = 0;
        }
        if !(renders && enabled) {
            return;
        }
        if (257..=320).contains(&dot) && ((dot - 257) & 0x07) == 7 {
            #[allow(clippy::cast_sign_loss)] // 0..8 by the range.
            let slot = ((dot - 257) >> 3) as usize;
            self.wait[slot] = u16::from(x[slot]);
            self.used[slot] = 0;
            self.reloaded += 1;
        } else if dot == 339 {
            self.halted = [false; SPRITES_PER_LINE];
            self.clean = usize::from(self.reloaded) == SPRITES_PER_LINE;
        }
    }

    /// A whole line with the rendering in one state: what its 340 [`Self::step`]s leave.
    fn whole_line(&mut self, visible: bool, renders: bool, enabled: bool, x: [u8; 8]) {
        if visible {
            for slot in 0..SPRITES_PER_LINE {
                let drawn = if self.halted[slot] {
                    256
                } else {
                    256u16.saturating_sub(self.wait[slot])
                };
                self.halted[slot] = true;
                self.wait[slot] = 0;
                if enabled {
                    #[allow(clippy::cast_possible_truncation)] // At most 8.
                    let left = drawn.min(u16::from(8 - self.used[slot])) as u8;
                    self.used[slot] += left;
                }
            }
        }
        self.reloaded = 0;
        if renders && enabled {
            self.wait = x.map(u16::from);
            self.used = [0; SPRITES_PER_LINE];
            self.reloaded = 8;
            self.halted = [false; SPRITES_PER_LINE];
            self.clean = true;
        }
    }
}

impl Ppu2C02 {
    /// A `$2001` write has just switched the rendering, `on` being the new state.
    pub(super) fn note_sprite_switch(&mut self, on: bool, renders: bool) {
        if self.sprite_units.disturbed {
            let line_clock = self.ppu_clock - i64::from(self.dot);
            self.advance_sprite_units(line_clock, self.scanline, self.dot, !on);
        } else if !on && renders {
            self.start_sprite_units();
        } else {
            // Switched back on after a blanking that began outside the picture: the units are
            // left as an undisturbed line has them.
            return;
        }
        self.sprite_units.clean = false;
        if on {
            self.sprite_line_stale = true;
        }
    }

    /// Starts following the units, at a switch-off on a line that had rendered so far: where an
    /// undisturbed line has them at this dot.
    fn start_sprite_units(&mut self) {
        let visible = self.scanline < VISIBLE_SCANLINES;
        // From dot 257 the count of this line's sprites is the next line's already; the units
        // past it are then taken as loaded, which only matters to one whose stale X is over 248.
        #[allow(clippy::cast_sign_loss)] // Clamped to 0..=8.
        let loaded = if self.dot > 256 {
            SPRITES_PER_LINE
        } else {
            self.sprites_loaded.clamp(0, 8) as usize
        };
        let mut units = SpriteUnits {
            disturbed: true,
            line_clock: self.ppu_clock - i64::from(self.dot),
            scanline: self.scanline,
            dot: self.dot,
            reloaded: 8,
            ..SpriteUnits::default()
        };
        for slot in 0..SPRITES_PER_LINE {
            if visible && slot < loaded {
                units.wait[slot] = u16::from(self.sprite_x[slot]);
            } else {
                units.used[slot] = 8;
                units.halted[slot] = true;
            }
        }
        for dot in 1..=self.dot {
            units.step(visible, true, dot, true, self.sprite_x);
        }
        self.sprite_units = units;
    }

    /// Brings the units to `dot` of the line whose dot 0 is at `line_clock`, the rendering
    /// having been `enabled` or not all the way from where they stand.
    fn advance_sprite_units(&mut self, line_clock: i64, scanline: i32, dot: i32, enabled: bool) {
        let mut units = self.sprite_units;
        let x = self.sprite_x;
        let total = self.timing.scanlines_per_frame;
        let kind = |line: i32| {
            let visible = line < VISIBLE_SCANLINES;
            (visible, visible || line == total - 1)
        };

        if line_clock <= units.line_clock {
            let (visible, renders) = kind(scanline);
            for step in units.dot + 1..=dot {
                units.step(visible, renders, step, enabled, x);
            }
        } else {
            let (visible, renders) = kind(units.scanline);
            for step in units.dot + 1..=340 {
                units.step(visible, renders, step, enabled, x);
            }

            // The whole lines in between, a frame of them at most: past that nothing changes.
            let between =
                ((line_clock - units.line_clock + 170) / 341 - 1).clamp(0, i64::from(total));
            #[allow(clippy::cast_possible_truncation)] // At most a frame of lines.
            let between = between as i32;
            let mut line = (scanline - between).rem_euclid(total);
            for _ in 0..between {
                let (visible, renders) = kind(line);
                units.whole_line(visible, renders, enabled, x);
                line = (line + 1) % total;
            }

            let (visible, renders) = kind(scanline);
            for step in 1..=dot {
                units.step(visible, renders, step, enabled, x);
            }
        }

        units.line_clock = line_clock;
        units.scanline = scanline;
        units.dot = dot;
        self.sprite_units = units;
    }

    /// [`Ppu2C02::rebuild_sprite_line`] for a disturbed line: what the units still have to draw
    /// from where they stand. `false` when they are back where an undisturbed line has them, and
    /// the ordinary rebuild is the right one.
    pub(super) fn rebuild_disturbed_sprite_line(&mut self) -> bool {
        if !self.sprite_units.disturbed {
            return false;
        }
        let line_clock = self.ppu_clock - i64::from(self.dot);
        if self.sprite_units.line_clock != line_clock {
            self.advance_sprite_units(line_clock, self.scanline, 0, true);
        }
        let units = self.sprite_units;
        if units.clean && units.dot == 0 {
            self.sprite_units.disturbed = false;
            return false;
        }

        self.sprite_line_stale = false;
        self.sprite_line.fill(0);
        #[allow(clippy::cast_sign_loss)] // A dot of the line; clamped to 0..=8.
        let (base, loaded) = (
            units.dot.max(0) as usize,
            self.sprites_loaded.clamp(0, 8) as usize,
        );
        for slot in 0..loaded {
            if units.used[slot] >= 8 {
                continue;
            }
            let start = base
                + if units.halted[slot] {
                    0
                } else {
                    usize::from(units.wait[slot])
                };
            let flags = super::render::sprite_flags(
                self.sprite_attribute[slot],
                slot == 0 && self.sprite_zero_loaded,
            );
            let flipped = (self.sprite_attribute[slot] & 0x40) != 0;
            let (low, high) = (
                self.sprite_pattern_low[slot],
                self.sprite_pattern_high[slot],
            );
            for column in usize::from(units.used[slot])..8 {
                let at = start + column - usize::from(units.used[slot]);
                if at >= SCREEN_WIDTH {
                    break;
                }
                if self.sprite_line[at] != 0 {
                    continue;
                }
                let bit = if flipped { column } else { 7 - column };
                let pixel = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);
                if pixel != 0 {
                    self.sprite_line[at] = flags | pixel;
                }
            }
        }
        true
    }
}
