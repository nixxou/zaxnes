//! The glyph tables of a loaded `ImGui` font: which code points the atlas carries and the
//! remapping of one to another (REQ-UI-207). The safe wrapper offers neither, and the calls are
//! `unsafe`, so they live here.

use imgui::internal::RawCast as _;
use imgui::sys;
use imgui::FontAtlas;

/// A font of the atlas, by its position in the atlas.
fn font_at(atlas: &mut FontAtlas, index: usize) -> Option<*mut sys::ImFont> {
    // SAFETY: the raw atlas is the C++ structure the wrapper views; `Fonts` is its vector of
    // font pointers, valid while the atlas is not modified.
    unsafe {
        let raw: &mut sys::ImFontAtlas = atlas.raw_mut();
        let fonts = &raw.Fonts;
        let count = usize::try_from(fonts.Size).ok()?;
        (index < count).then(|| *fonts.Data.add(index))
    }
}

/// How many fonts the atlas holds.
#[must_use]
pub fn font_count(atlas: &mut FontAtlas) -> usize {
    // SAFETY: reads the size of the atlas's font vector.
    unsafe { usize::try_from(atlas.raw_mut().Fonts.Size).unwrap_or(0) }
}

/// Whether the font at `index` is loaded and has a glyph for `code`, without the fallback glyph.
#[must_use]
pub fn carries(atlas: &mut FontAtlas, index: usize, code: u32) -> bool {
    let Some(font) = font_at(atlas, index) else {
        return false;
    };
    // SAFETY: `font` points into the atlas, which outlives the call.
    unsafe { !sys::ImFont_FindGlyphNoFallback(font, code).is_null() }
}

/// Whether the font at `index` is loaded.
#[must_use]
pub fn is_loaded(atlas: &mut FontAtlas, index: usize) -> bool {
    let Some(font) = font_at(atlas, index) else {
        return false;
    };
    // SAFETY: `font` points into the atlas, which outlives the call.
    unsafe { sys::ImFont_IsLoaded(font) }
}

/// Makes the font at `index` draw `to` where `from` is asked for.
pub fn remap(atlas: &mut FontAtlas, index: usize, from: u32, to: u32) {
    if let Some(font) = font_at(atlas, index) {
        // SAFETY: `font` points into the atlas, which outlives the call.
        unsafe { sys::ImFont_AddRemapChar(font, from, to, true) };
    }
}
