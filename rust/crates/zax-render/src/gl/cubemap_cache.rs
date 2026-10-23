//! Uploads the skybox cubemaps of SPEC-06 REQ-SCN-050 and REQ-SCN-051 to the GPU, one face at a
//! time, and keeps them resident for as long as the renderer lives.
//!
//! The faces are uploaded in the OpenGL cube-map order, which matches the left-handed, Y-up
//! convention `assets-neutral/cubemaps/cubemaps.json` documents: a direction given in scene
//! coordinates can therefore be used as the sampling vector unchanged. Rows are stored top-down
//! in the files, which is also what the cube-map targets expect. The notes of REQ-SCN-050 say
//! the cubemaps hold **linear** data, so the texels are uploaded as plain `RGBA8` and never
//! passed through an sRGB transfer.

use std::collections::HashMap;

use glow::Texture;

use super::api::{Gl, GlError};
use crate::assets::{CubeFace, ImageData, SceneAssets};

/// The cubemaps of one asset source; released when dropped.
pub struct CubemapCache {
    gl: Gl,
    assets: Box<dyn SceneAssets>,
    handles: HashMap<&'static str, Option<Texture>>,
    fallback: Option<Texture>,
}

impl std::fmt::Debug for CubemapCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CubemapCache")
            .field("handles", &self.handles)
            .finish_non_exhaustive()
    }
}

impl CubemapCache {
    /// Creates the cache over `assets`.
    pub fn new(gl: &Gl, assets: Box<dyn SceneAssets>) -> Self {
        Self {
            gl: gl.clone(),
            assets,
            handles: HashMap::new(),
            fallback: None,
        }
    }

    /// The handle of `name`, uploading it on first use. When the asset source has no such
    /// cubemap, a black 1 × 1 cubemap is returned, which makes every formula of REQ-SCN-050 and
    /// REQ-SCN-051 degrade to its "no cubemap" case.
    pub fn get(&mut self, name: &'static str) -> Result<Texture, GlError> {
        let handle = if let Some(&handle) = self.handles.get(name) {
            handle
        } else {
            let handle = self.upload(name)?;
            self.handles.insert(name, handle);
            handle
        };

        match handle {
            Some(handle) => Ok(handle),
            None => self.fallback(),
        }
    }

    fn upload(&mut self, name: &str) -> Result<Option<Texture>, GlError> {
        let mut faces = Vec::with_capacity(6);
        for face in CubeFace::ALL {
            match self.assets.cubemap_face(name, face) {
                Some(image) => faces.push(image),
                None => return Ok(None),
            }
        }

        upload_faces(&self.gl, faces.into_iter()).map(Some)
    }

    fn fallback(&mut self) -> Result<Texture, GlError> {
        if let Some(fallback) = self.fallback {
            return Ok(fallback);
        }

        let black = (0..6).map(|_| ImageData::single(0, 0, 0, 255));
        let fallback = upload_faces(&self.gl, black)?;
        self.fallback = Some(fallback);
        Ok(fallback)
    }
}

impl Drop for CubemapCache {
    fn drop(&mut self) {
        for handle in self.handles.values().flatten().chain(&self.fallback) {
            self.gl.delete_texture(*handle);
        }
    }
}

/// Uploads six faces in cube-map order. Each face is dropped once uploaded: the three nebula
/// cubemaps are 2048 × 2048 each and would otherwise hold a hundred megabytes at once.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // Face sides fit GL sizes.
fn upload_faces(gl: &Gl, faces: impl Iterator<Item = ImageData>) -> Result<Texture, GlError> {
    let handle = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(handle));
    gl.pixel_store(glow::UNPACK_ALIGNMENT, 1);
    for (target, face) in (glow::TEXTURE_CUBE_MAP_POSITIVE_X..).zip(faces) {
        gl.tex_image_2d(
            target,
            glow::RGBA8,
            face.width as i32,
            face.height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&face.pixels),
        );
    }

    gl.pixel_store(glow::UNPACK_ALIGNMENT, 4);
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
        (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_R, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_BASE_LEVEL, 0),
        (glow::TEXTURE_MAX_LEVEL, 0),
    ] {
        gl.tex_parameter(glow::TEXTURE_CUBE_MAP, parameter, value);
    }
    gl.bind_texture(glow::TEXTURE_CUBE_MAP, None);
    Ok(handle)
}
