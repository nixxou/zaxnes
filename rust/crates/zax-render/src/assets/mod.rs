//! Where the renderer gets the extracted assets of SPEC-06: the skybox cubemaps of REQ-SCN-050
//! and REQ-SCN-051, plus the meshes and textures of an asset folder.

mod neutral;
pub mod obj;
pub mod png_image;

use std::sync::Arc;

use zax_engine3d::geometry::tile_mesh::TileMesh;

pub use neutral::NeutralSceneAssets;

/// A decoded 8-bit RGBA image, rows stored top-down, as the PNG files of
/// `assets-neutral/textures` and `assets-neutral/cubemaps` hold them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// `width × height × 4` bytes, four per pixel in R, G, B, A order, the first row first.
    pub pixels: Vec<u8>,
}

impl ImageData {
    /// The four channels of pixel (`x`, `y`).
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> (u8, u8, u8, u8) {
        let at = ((y * self.width) + x) * 4;
        (
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        )
    }

    /// A one-pixel image of the given colour, used when an asset is missing.
    #[must_use]
    pub fn single(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![r, g, b, a],
        }
    }
}

/// The six faces of a cubemap, in the order `assets-neutral/cubemaps/cubemaps.json` names them:
/// a left-handed frame with Y up, rows stored top-down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeFace {
    /// `px`, the +X face.
    PositiveX = 0,
    /// `nx`, the −X face.
    NegativeX = 1,
    /// `py`, the +Y face.
    PositiveY = 2,
    /// `ny`, the −Y face.
    NegativeY = 3,
    /// `pz`, the +Z face.
    PositiveZ = 4,
    /// `nz`, the −Z face.
    NegativeZ = 5,
}

impl CubeFace {
    /// The six faces in OpenGL cube-map order.
    pub const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    /// The file name of the face, as `cubemaps.json` spells it.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::PositiveX => "px",
            Self::NegativeX => "nx",
            Self::PositiveY => "py",
            Self::NegativeY => "ny",
            Self::PositiveZ => "pz",
            Self::NegativeZ => "nz",
        }
    }
}

/// Where the renderer gets the extracted assets of SPEC-06.
///
/// The renderer never touches the file system itself: the host supplies this (REQ-SCN-057,
/// REQ-ARCH-003). A test can therefore inject the uniform cubemaps the acceptance criterion of
/// REQ-SCN-051 asks for. Every member may return `None`; the renderer then falls back to the
/// plain background.
pub trait SceneAssets {
    /// One face of the cubemap `cubemap` (`Stars_LP_01`, `Starfield`, `NebulaDensity`,
    /// `NebulaDiffusion`, `Cubemap`), or `None` when it is missing. All six faces of one cubemap
    /// have the same size.
    fn cubemap_face(&mut self, cubemap: &str, face: CubeFace) -> Option<ImageData>;

    /// The texture `name` of `assets-neutral/textures`, or `None`.
    fn texture(&mut self, name: &str) -> Option<Arc<ImageData>>;

    /// The mesh `name` of `assets-neutral/meshes`, or `None`.
    fn mesh(&mut self, name: &str) -> Option<Arc<TileMesh>>;
}
