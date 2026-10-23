//! How big the offscreen buffer the scene is drawn into has to be, from the render method and
//! the resolution scale of SPEC-09 §4.1.1 (REQ-SCN-063, AMD-032 §3).
//!
//! The application always draws the scene into its own framebuffer and blits it onto the window.
//! The two settings only decide how big that framebuffer is: with `video.render_method` = 0,
//! "Direct", it has the size of the view and `video.res_scale` has no effect at all; with 1,
//! "Render Tex", it is the view scaled by `video.res_scale_factor` (1, 2/3, 1/2, 1/3 or 1/4) and
//! the blit stretches it back, filtered bilinearly (REQ-SCN-063).

/// The render method that draws at the view's own resolution.
pub const DIRECT_METHOD: i32 = 0;
/// The render method the resolution scale applies to.
pub const RENDER_TEXTURE_METHOD: i32 = 1;

/// Whether the resolution scale has any effect, which is what the options window greys the row
/// out on.
#[must_use]
pub const fn scale_applies(render_method: i32) -> bool {
    render_method == RENDER_TEXTURE_METHOD
}

/// The size of the offscreen buffer for a view of `view_width` x `view_height` pixels: at least
/// one pixel in each direction, whatever the factor.
#[must_use]
pub fn scene_size(
    view_width: i32,
    view_height: i32,
    render_method: i32,
    factor: f32,
) -> (i32, i32) {
    let width = view_width.max(1);
    let height = view_height.max(1);
    if !scale_applies(render_method) || !factor.is_finite() || factor >= 1.0 || factor <= 0.0 {
        return (width, height);
    }
    // Rounding rather than truncating keeps 2/3 of 1080 at 720 instead of 719, so the factors
    // stay the exact fractions SPEC-06 names wherever the view divides evenly.
    (scaled(width, factor), scaled(height, factor))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
// the C# `(int)MathF.Round(size * factor)` of a view size below 2^24
fn scaled(size: i32, factor: f32) -> i32 {
    ((size as f32 * factor).round() as i32).max(1)
}
