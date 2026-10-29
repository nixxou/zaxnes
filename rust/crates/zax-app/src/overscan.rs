//! The picture's edges a television did not show (Rust-only, owner request), as Mesen sets them:
//! per side, from the settings of the game's region (`video.overscan_ntsc_*`,
//! `video.overscan_pal_*`) unless the game has its own (`video.overscan_top` and the others),
//! and whether they are left out, the picture keeping its size ("Fit"), or the rest enlarged to
//! fill the view in 2D ("Resize", `video.overscan_mode`). Left out, not blacked out (owner
//! request): the scene's background shows there, as where the 3D scene's tiles are left out.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

/// The NES screen, in pixels.
const SCREEN_WIDTH: i32 = 256;
const SCREEN_HEIGHT: i32 = 240;

/// The sides cut, in pixels, and how.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overscan {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
    /// The rest enlarged to fill the view in 2D, rather than the cut sides blacked out.
    pub resize: bool,
}

impl Overscan {
    /// The overscan in force for a game of the region `ntsc` (or PAL): each side the game's own
    /// value, or its region's.
    #[must_use]
    pub fn from_settings(settings: &SettingsModel, ntsc: bool) -> Self {
        let side = |own: SettingId, ntsc_id: SettingId, pal_id: SettingId| {
            let value = settings.get(own).as_int();
            if value >= 0 {
                value
            } else {
                settings.get(if ntsc { ntsc_id } else { pal_id }).as_int()
            }
        };
        Self {
            top: side(
                SettingId::VideoOverscanTop,
                SettingId::VideoOverscanNtscTop,
                SettingId::VideoOverscanPalTop,
            ),
            bottom: side(
                SettingId::VideoOverscanBottom,
                SettingId::VideoOverscanNtscBottom,
                SettingId::VideoOverscanPalBottom,
            ),
            left: side(
                SettingId::VideoOverscanLeft,
                SettingId::VideoOverscanNtscLeft,
                SettingId::VideoOverscanPalLeft,
            ),
            right: side(
                SettingId::VideoOverscanRight,
                SettingId::VideoOverscanNtscRight,
                SettingId::VideoOverscanPalRight,
            ),
            resize: settings.get(SettingId::VideoOverscanMode).as_int() != 0,
        }
    }

    /// Whether no side is cut.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.top == 0 && self.bottom == 0 && self.left == 0 && self.right == 0
    }

    /// The sides cut for a picture with a widescreen `margin` on each side: the left and right
    /// ones only without a margin, which goes on past the screen's edges (cutting them would
    /// undo the widescreen). The top and bottom ones always (owner request: set, they apply),
    /// the vertical margin past them going with them.
    #[must_use]
    pub const fn in_force(self, margin: i32) -> Self {
        Self {
            top: self.top,
            bottom: self.bottom,
            left: if margin == 0 { self.left } else { 0 },
            right: if margin == 0 { self.right } else { 0 },
            resize: self.resize,
        }
    }

    /// Whether the tile at screen `(x, y)` (its top left pixel) is shown: not half or more in a
    /// cut side.
    #[must_use]
    pub fn keeps(&self, x: i16, y: i16) -> bool {
        let (x, y) = (i32::from(x) + 4, i32::from(y) + 4);
        y > self.top
            && y < SCREEN_HEIGHT - self.bottom
            && (self.left == 0 || x > self.left)
            && (self.right == 0 || x < SCREEN_WIDTH - self.right)
    }

    /// Leaves the cut sides out of `picture` (colour indices, `height` rows, as wide and as tall
    /// as the widescreen margins make it, the screen's 240 lines in the middle), and the vertical
    /// margin past the top and bottom ones: the picture ends where it does without a vertical
    /// margin. The left and right sides are cut on a picture without a horizontal margin only.
    /// A pixel left out is `zax_render::interop::HIDDEN_PIXEL`, which is not drawn.
    pub fn leave_out(&self, picture: &mut [u8], height: usize) {
        const HIDDEN: u8 = zax_render::interop::HIDDEN_PIXEL;
        let height_i = i32::try_from(height).unwrap_or(0);
        if height_i < SCREEN_HEIGHT || picture.is_empty() {
            return;
        }
        let width = picture.len() / height;
        let length = picture.len();
        let top = usize::try_from((height_i - SCREEN_HEIGHT) / 2).unwrap_or(0);
        if self.top > 0 {
            let start = (top + usize::try_from(self.top).unwrap_or(0)) * width;
            picture[..start.min(length)].fill(HIDDEN);
        }
        if self.bottom > 0 {
            let end = (top + usize::try_from(SCREEN_HEIGHT - self.bottom).unwrap_or(0)) * width;
            picture[end.min(length)..].fill(HIDDEN);
        }
        let (left, right) = (
            usize::try_from(self.left).unwrap_or(0),
            usize::try_from(self.right).unwrap_or(0),
        );
        if (left > 0 || right > 0) && width == usize::try_from(SCREEN_WIDTH).unwrap_or(0) {
            for row in picture.chunks_mut(width) {
                row[..left.min(width)].fill(HIDDEN);
                row[width.saturating_sub(right)..].fill(HIDDEN);
            }
        }
    }

    /// The sides the 2D picture's head-on view leaves out in Resize, `(left, bottom, right,
    /// top)` in scene units, as the profile's clipping (a pixel each): none in Fit. The clipping
    /// counts from a frame 240 high, while the picture's rows lie 8 higher, row 239 at y = 8 to
    /// 9 and row 0 at 247 to 248 (REQ-E3D-410): `bottom` lines cut end at y = 8 + `bottom`, and
    /// `top` lines at 248 - `top`, 8 under the frame's top (the frame leaves the top 8 lines out
    /// by itself).
    #[must_use]
    pub fn crop(&self) -> [f32; 4] {
        if !self.resize {
            return [0.0; 4];
        }
        let bottom = if self.bottom > 0 { self.bottom + 8 } else { 0 };
        #[allow(clippy::cast_precision_loss)] // -8..=40.
        [
            self.left as f32,
            bottom as f32,
            self.right as f32,
            (self.top - 8) as f32,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIDDEN: u8 = zax_render::interop::HIDDEN_PIXEL;

    fn sides(top: i32, bottom: i32, left: i32, right: i32) -> Overscan {
        Overscan {
            top,
            bottom,
            left,
            right,
            resize: false,
        }
    }

    #[test]
    fn a_taller_picture_ends_where_the_screen_does_without_a_margin() {
        // 2 columns, a vertical margin of 4 lines above and below the 240 of the screen.
        let height = 248;
        let mut picture = vec![1u8; 2 * height];
        sides(8, 8, 0, 0).leave_out(&mut picture, height);
        let line = |y: usize| &picture[y * 2..y * 2 + 2];
        assert_eq!(line(0), [HIDDEN, HIDDEN], "the margin above");
        assert_eq!(line(11), [HIDDEN, HIDDEN], "the screen's eighth line");
        assert_eq!(line(12), [1, 1], "its ninth");
        assert_eq!(line(235), [1, 1], "its ninth from the bottom");
        assert_eq!(line(236), [HIDDEN, HIDDEN], "its eighth from the bottom");
        assert_eq!(line(247), [HIDDEN, HIDDEN], "the margin below");
    }

    #[test]
    fn each_side_is_cut_on_its_own() {
        let mut picture = vec![1u8; 256 * 240];
        sides(2, 0, 3, 5).leave_out(&mut picture, 240);
        let at = |x: usize, y: usize| picture[y * 256 + x];
        assert_eq!(HIDDEN, at(100, 1), "top");
        assert_eq!(1, at(100, 2));
        assert_eq!(1, at(100, 239), "no bottom");
        assert_eq!(HIDDEN, at(2, 100), "left");
        assert_eq!(1, at(3, 100));
        assert_eq!(1, at(250, 100));
        assert_eq!(HIDDEN, at(251, 100), "right");
    }

    #[test]
    fn resize_cuts_the_picture_rows_where_they_lie_in_the_scene() {
        let mut overscan = sides(8, 8, 6, 0);
        assert_eq!([0.0; 4], overscan.crop(), "Fit");
        overscan.resize = true;
        // The bottom 8 rows end at y = 16, the top 8 at 240: the frame's own top.
        assert_eq!([6.0, 16.0, 0.0, 0.0], overscan.crop());
        assert_eq!(
            [0.0, 0.0, 0.0, -8.0],
            Overscan {
                resize: true,
                ..Overscan::default()
            }
            .crop()
        );
    }

    #[test]
    fn a_margin_keeps_the_sides_it_goes_on_past() {
        let overscan = sides(8, 8, 8, 8);
        assert_eq!(sides(8, 8, 0, 0), overscan.in_force(64));
        assert_eq!(overscan, overscan.in_force(0));
    }

    #[test]
    fn a_tile_half_in_a_cut_side_is_left_out() {
        let overscan = sides(8, 8, 8, 0);
        assert!(!overscan.keeps(100, 4), "half in the top 8 lines");
        assert!(overscan.keeps(100, 5));
        assert!(overscan.keeps(100, 227));
        assert!(!overscan.keeps(100, 228), "half in the bottom 8 lines");
        assert!(!overscan.keeps(4, 100), "half in the left 8 columns");
        assert!(overscan.keeps(252, 100), "no right side");
    }
}
