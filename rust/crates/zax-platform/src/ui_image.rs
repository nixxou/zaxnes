//! The pictures the interface shows besides its text (Rust-only): a save state's thumbnail, for
//! instance. Each one is a texture of the interface renderer under a Dear `ImGui` texture id; the
//! interface says which ones change, and the renderer in use (GL or wgpu) keeps them.

/// The texture id of the font atlas, the renderers' own.
pub const FONT_TEXTURE_ID: usize = 1;

/// The first texture id the interface may give a picture.
pub const FIRST_PICTURE_ID: usize = 16;

/// A picture: `width` x `height` pixels, four bytes each (R, G, B, A), the top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPicture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl UiPicture {
    /// Whether the pixels fill the size exactly.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        self.width > 0
            && self.height > 0
            && usize::try_from(u64::from(self.width) * u64::from(self.height) * 4)
                .is_ok_and(|size| size == self.rgba.len())
    }
}

/// A change of the pictures: the picture under `id` becomes `picture`, or goes with `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureUpdate {
    pub id: usize,
    pub picture: Option<UiPicture>,
}
