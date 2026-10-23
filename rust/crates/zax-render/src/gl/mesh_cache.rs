//! Uploads the abstract tile meshes of SPEC-04B REQ-E3D-640 to the GPU and keeps them, keyed by
//! mesh identity.
//!
//! A mesh is immutable once built and a rebuild produces a new [`TileMesh`] (REQ-E3D-542,
//! REQ-E3D-602), so identity is the right key: nothing is re-uploaded while the engine does not
//! rebuild, whatever a script changes on an instance. The cache holds a reference to each mesh
//! it keys, so an address is never reused while its entry lives. Entries not used for
//! [`RETENTION_FRAMES`] rendered frames are released, which covers temporary patterns
//! (REQ-E3D-603).
//!
//! Vertices are interleaved: position (3 floats), normal (3 floats), texel coordinates
//! (2 floats). Indices are 32 bits, as REQ-E3D-524 allows meshes over 65 535 vertices.

use std::collections::HashMap;
use std::sync::Arc;

use glow::{Buffer, VertexArray};
use zax_engine3d::geometry::tile_mesh::TileMesh;

use super::api::{Gl, GlError};

/// Rendered frames an unused mesh is kept for.
pub const RETENTION_FRAMES: u64 = 600;

const FLOATS_PER_VERTEX: usize = 8;

/// Bytes per interleaved vertex.
const STRIDE: i32 = 32;

/// A mesh ready to draw: its vertex array and its index count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMesh {
    /// The vertex array object.
    pub vertex_array: VertexArray,
    /// Number of indices, three per triangle.
    pub index_count: i32,
}

#[derive(Debug)]
struct Entry {
    /// Held only so that the keyed address stays unique while the entry lives.
    _mesh: Arc<TileMesh>,
    vertex_array: VertexArray,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: i32,
    last_used: u64,
}

/// The resident meshes; everything is released when dropped.
#[derive(Debug)]
pub struct MeshCache {
    gl: Gl,
    entries: HashMap<*const TileMesh, Entry>,
    frame: u64,
    uploads: u32,
}

impl MeshCache {
    /// Creates the cache for the current context.
    pub fn new(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            entries: HashMap::new(),
            frame: 0,
            uploads: 0,
        }
    }

    /// Number of meshes currently resident on the GPU.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Number of uploads performed since the cache was created.
    pub const fn uploads(&self) -> u32 {
        self.uploads
    }

    /// Starts a rendered frame, for the retention bookkeeping.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The vertex array of `mesh`, uploading it on first use, or `None` when the mesh draws
    /// nothing.
    pub fn get(&mut self, mesh: &Arc<TileMesh>) -> Result<Option<GpuMesh>, GlError> {
        if mesh.is_empty() {
            return Ok(None);
        }

        let key = Arc::as_ptr(mesh);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.frame;
            return Ok(Some(entry.gpu()));
        }

        let entry = self.upload(mesh)?;
        let gpu = entry.gpu();
        self.entries.insert(key, entry);
        self.uploads += 1;
        Ok(Some(gpu))
    }

    /// Releases every mesh whose last use is older than the retention window.
    pub fn collect(&mut self) {
        let (gl, frame) = (&self.gl, self.frame);
        self.entries.retain(|_, entry| {
            let keep = frame - entry.last_used <= RETENTION_FRAMES;
            if !keep {
                entry.release(gl);
            }
            keep
        });
    }

    /// Releases everything, for instance when the global mesh algorithm changes.
    pub fn clear(&mut self) {
        for (_, entry) in self.entries.drain() {
            entry.release(&self.gl);
        }
    }

    fn upload(&self, mesh: &Arc<TileMesh>) -> Result<Entry, GlError> {
        let mut vertices = Vec::with_capacity(mesh.vertex_count() * FLOATS_PER_VERTEX * 4);
        for vertex in mesh.vertices() {
            for value in [
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
                vertex.normal.x,
                vertex.normal.y,
                vertex.normal.z,
                vertex.uv.x,
                vertex.uv.y,
            ] {
                vertices.extend_from_slice(&value.to_ne_bytes());
            }
        }

        let indices: Vec<u8> = mesh
            .triangles()
            .iter()
            .flat_map(|index| index.to_ne_bytes())
            .collect();

        let gl = &self.gl;
        let vertex_array = gl.create_vertex_array()?;
        gl.bind_vertex_array(Some(vertex_array));
        let vertex_buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        gl.buffer_data(glow::ARRAY_BUFFER, &vertices);
        let index_buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data(glow::ELEMENT_ARRAY_BUFFER, &indices);

        gl.float_attribute(0, 3, STRIDE, 0);
        gl.float_attribute(1, 3, STRIDE, 3 * 4);
        gl.float_attribute(2, 2, STRIDE, 6 * 4);

        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

        Ok(Entry {
            _mesh: Arc::clone(mesh),
            vertex_array,
            vertex_buffer,
            index_buffer,
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            // A tile mesh holds far fewer than 2^31 indices.
            index_count: mesh.triangles().len() as i32,
            last_used: self.frame,
        })
    }
}

impl Entry {
    const fn gpu(&self) -> GpuMesh {
        GpuMesh {
            vertex_array: self.vertex_array,
            index_count: self.index_count,
        }
    }

    fn release(&self, gl: &Gl) {
        gl.delete_vertex_array(self.vertex_array);
        gl.delete_buffer(self.vertex_buffer);
        gl.delete_buffer(self.index_buffer);
    }
}

impl Drop for MeshCache {
    fn drop(&mut self) {
        self.clear();
    }
}
