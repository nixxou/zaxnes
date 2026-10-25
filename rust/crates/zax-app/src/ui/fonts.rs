//! The fonts of the interface (SPEC-10 REQ-UI-207, REQ-PUB-006): a system face at two sizes, or
//! the built-in one of `ImGui` when the system has none, and the remapping of the glyphs a face
//! lacks.

use std::path::PathBuf;

use imgui::{FontAtlas, FontConfig, FontGlyphRanges, FontId, FontSource};
use zax_platform::imgui_glyphs;

use super::system_fonts;

/// Size of the body text, in pixels.
pub const BODY_SIZE: f32 = 16.0;
/// Size of the headings (product name, about block), in pixels.
pub const HEADING_SIZE: f32 = 22.0;

/// The sizes of the two faces, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSizes {
    /// The body text.
    pub body: f32,
    /// The headings.
    pub heading: f32,
}

impl FontSizes {
    /// The sizes of the desktop interface.
    pub const DESKTOP: Self = Self {
        body: BODY_SIZE,
        heading: HEADING_SIZE,
    };
}

/// The system face (the `hint` file, or the platform's) and the bold one of its family, read;
/// none with `built_in_only`, or when the face is missing or unreadable.
fn read_faces(
    hint: Option<&str>,
    built_in_only: bool,
) -> Option<(PathBuf, Vec<u8>, Option<Vec<u8>>)> {
    if built_in_only {
        return None;
    }
    let files = system_fonts::locate(hint);
    let regular = files.regular.as_ref()?;
    let body = std::fs::read(regular).ok()?;
    let bold = files
        .bold
        .as_ref()
        .and_then(|bold| std::fs::read(bold).ok());
    Some((regular.clone(), body, bold))
}

/// U+25B8, the small triangle of "Fichier ▸ Ouvrir une ROM" (REQ-UI-207).
pub const SMALL_RIGHT_TRIANGLE: u32 = 0x25B8;
/// U+25BA, the pointer Segoe UI and Arial carry instead.
pub const RIGHT_POINTER: u32 = 0x25BA;
/// U+25B6, the larger triangle.
pub const RIGHT_TRIANGLE: u32 = 0x25B6;
/// The ASCII sign the amendment allows as the last substitute.
pub const ASCII_POINTER: u32 = 0x3E;

/// What the font atlas needs: Latin-1 (the accents and the guillemets), the ligature of "Cœur
/// d'émulation" (REQ-UI-202), the dashes (the "unbound" sign of DEC-08-5), the typographic
/// quotes, the ellipsis and the small triangles, as pairs of first and last code point, then
/// the zero that ends the list.
static GLYPH_RANGES: [u32; 13] = [
    0x0020, 0x00FF, 0x0152, 0x0153, 0x2010, 0x2015, 0x2018, 0x201F, 0x2026, 0x2026, 0x25B6, 0x25BA,
    0x0000,
];

/// The substitutes the built-in font gets: Latin-1 is there, so the French accents survive, but
/// neither the triangle of REQ-UI-207 nor the ellipsis of "Ouvrir une ROM…" is.
const ASCII_SUBSTITUTES: [(u32, u32); 4] = [
    (SMALL_RIGHT_TRIANGLE, ASCII_POINTER),
    (0x2026, 0x2E), // Horizontal ellipsis to a full stop.
    (0x2014, 0x2D), // Em dash to a hyphen.
    (0x2013, 0x2D), // En dash to a hyphen.
];

/// A face of the system at `size_pixels`, with the glyphs the interface needs.
fn face(data: &[u8], size_pixels: f32) -> FontSource<'_> {
    FontSource::TtfData {
        data,
        size_pixels,
        config: Some(FontConfig {
            glyph_ranges: FontGlyphRanges::from_slice(&GLYPH_RANGES),
            ..FontConfig::default()
        }),
    }
}

/// The fonts added to the atlas.
#[derive(Debug, Clone, Default)]
pub struct UiFonts {
    /// The heading face, when a system face was loaded.
    pub heading: Option<FontId>,
    /// The file of the body face, when a system face was loaded.
    pub body_file: Option<PathBuf>,
}

impl UiFonts {
    /// Adds the faces to `atlas`: the system face (the `hint` file, or the platform's) at the
    /// body size, then the bold one of the same family at the heading size. With no face, or
    /// `built_in_only`, the built-in font of `ImGui` is the only one.
    pub fn load(atlas: &mut FontAtlas, hint: Option<&str>, built_in_only: bool) -> Self {
        Self::load_sized(atlas, hint, built_in_only, FontSizes::DESKTOP)
    }

    /// [`Self::load`] with other sizes, which the handheld interface derives from its display.
    pub fn load_sized(
        atlas: &mut FontAtlas,
        hint: Option<&str>,
        built_in_only: bool,
        wanted: FontSizes,
    ) -> Self {
        let Some((body_file, body, bold)) = read_faces(hint, built_in_only) else {
            let config = (wanted != FontSizes::DESKTOP).then(|| FontConfig {
                size_pixels: wanted.body,
                ..FontConfig::default()
            });
            atlas.add_font(&[FontSource::DefaultFontData { config }]);
            return Self::default();
        };
        atlas.add_font(&[face(&body, wanted.body)]);
        // The headings use the bold file of the same family when the system has one, and the
        // regular face at the larger size otherwise.
        let heading = atlas.add_font(&[face(bold.as_deref().unwrap_or(&body), wanted.heading)]);
        Self {
            heading: Some(heading),
            body_file: Some(body_file),
        }
    }

    /// Adds the faces of the console mode of the desktop (Rust-only, owner request) after the
    /// desktop's: the body and the heading at `wanted`. Returns them, the heading none with the
    /// built-in font only.
    pub fn load_console(
        atlas: &mut FontAtlas,
        hint: Option<&str>,
        built_in_only: bool,
        wanted: FontSizes,
    ) -> (FontId, Option<FontId>) {
        let Some((_, body, bold)) = read_faces(hint, built_in_only) else {
            let config = Some(FontConfig {
                size_pixels: wanted.body,
                ..FontConfig::default()
            });
            return (
                atlas.add_font(&[FontSource::DefaultFontData { config }]),
                None,
            );
        };
        let body_font = atlas.add_font(&[face(&body, wanted.body)]);
        let heading = atlas.add_font(&[face(bold.as_deref().unwrap_or(&body), wanted.heading)]);
        (body_font, Some(heading))
    }

    /// Points the code points a face lacks at the nearest glyph the atlas carries (REQ-UI-207).
    /// Must run once the atlas is built, since the glyph tables exist from then on.
    ///
    /// A system face does not necessarily carry U+25B8: Segoe UI and Arial both have U+25BA, and
    /// `DejaVu` Sans has all three triangles. The string itself stays the amendment's.
    pub fn remap_missing_glyphs(&self, atlas: &mut FontAtlas) {
        if self.body_file.is_none() {
            // The built-in font of REQ-PUB-006's degraded mode, which reads better with an
            // ASCII sign than with the '?' box of `ImGui`.
            for (from, to) in ASCII_SUBSTITUTES {
                if !imgui_glyphs::carries(atlas, 0, from) && imgui_glyphs::carries(atlas, 0, to) {
                    imgui_glyphs::remap(atlas, 0, from, to);
                }
            }
            return;
        }
        for index in 0..imgui_glyphs::font_count(atlas) {
            if !imgui_glyphs::is_loaded(atlas, index)
                || imgui_glyphs::carries(atlas, index, SMALL_RIGHT_TRIANGLE)
            {
                continue;
            }
            if let Some(substitute) = [RIGHT_POINTER, RIGHT_TRIANGLE, ASCII_POINTER]
                .into_iter()
                .find(|code| imgui_glyphs::carries(atlas, index, *code))
            {
                imgui_glyphs::remap(atlas, index, SMALL_RIGHT_TRIANGLE, substitute);
            }
        }
    }
}
