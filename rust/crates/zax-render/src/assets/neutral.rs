//! Reads the skybox assets straight out of `assets-neutral/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use zax_engine3d::geometry::tile_mesh::TileMesh;

use super::{obj, png_image, CubeFace, ImageData, SceneAssets};

/// The scene assets of an `assets-neutral` folder.
///
/// Meshes and the small textures are cached once read; cubemap faces are **not** cached,
/// because the three 2048 × 2048 skybox cubemaps would hold about 300 MB of RGBA in memory. The
/// GPU side uploads each face as it is read and lets it go. A file that is missing or cannot be
/// read is reported as missing, which makes the renderer fall back to the plain background.
#[derive(Debug)]
pub struct NeutralSceneAssets {
    root: PathBuf,
    textures: HashMap<String, Option<Arc<ImageData>>>,
    meshes: HashMap<String, Option<Arc<TileMesh>>>,
}

impl NeutralSceneAssets {
    /// Creates the reader for the folder holding `cubemaps/`, `meshes/` and `textures/`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            textures: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    /// Opens `root` when it holds the cubemap folder, else `None`.
    #[must_use]
    pub fn try_open(root: Option<&Path>) -> Option<Self> {
        root.filter(|root| root.join("cubemaps").is_dir())
            .map(Self::new)
    }

    /// The asset folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A PNG of the folder; an unreadable file counts as missing, as in the C# reader.
    fn read(path: &Path) -> Option<ImageData> {
        png_image::load(path).ok()
    }
}

impl SceneAssets for NeutralSceneAssets {
    fn cubemap_face(&mut self, cubemap: &str, face: CubeFace) -> Option<ImageData> {
        let file = format!("{}.png", face.file_name());
        Self::read(&self.root.join("cubemaps").join(cubemap).join(file))
    }

    fn texture(&mut self, name: &str) -> Option<Arc<ImageData>> {
        let root = &self.root;
        self.textures
            .entry(name.to_owned())
            .or_insert_with(|| {
                Self::read(&root.join("textures").join(format!("{name}.png"))).map(Arc::new)
            })
            .clone()
    }

    fn mesh(&mut self, name: &str) -> Option<Arc<TileMesh>> {
        let root = &self.root;
        self.meshes
            .entry(name.to_owned())
            .or_insert_with(|| {
                // An unreadable or malformed file counts as missing, as in the C# reader.
                let text = std::fs::read_to_string(root.join("meshes").join(format!("{name}.obj")));
                text.ok()
                    .and_then(|text| obj::parse(&text).ok())
                    .map(Arc::new)
            })
            .clone()
    }
}
