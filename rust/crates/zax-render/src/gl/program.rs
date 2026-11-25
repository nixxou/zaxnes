//! A compiled and linked GLSL program, with a cache of uniform locations and of the values
//! last uploaded (a uniform keeps its value in the program object, so setting it again to the
//! same value is skipped: most draws of a frame change only a few of their uniforms).
//!
//! OpenGL 3.3 core without any extension (REQ-SCN-056), and the same sources on GLES 3.0
//! (REQ-ARCH-030): no explicit uniform locations, no bindings in the source, no integer texture
//! gathers.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use glow::{Program, UniformLocation};

use super::api::{Gl, GlError};
use super::shaders::{self, ProgramSource};
use crate::numerics::{Matrix4x4, Vector2, Vector3, Vector4};

/// A linked program; deleted when dropped.
#[derive(Debug)]
pub struct ShaderProgram {
    gl: Gl,
    handle: Program,
    uniforms: HashMap<&'static str, Uniform, BuildHasherDefault<NameHasher>>,
}

/// One uniform: its location, `None` when it is not active, and the bits last uploaded.
#[derive(Debug)]
struct Uniform {
    location: Option<UniformLocation>,
    /// The words of the value last uploaded; `len` 0 until the first upload.
    last: [u32; 16],
    len: usize,
}

/// FNV-1a over the uniform name: the names are short literals, looked up a dozen times per
/// draw, where the default `SipHash` costs more than the GL call it guards.
#[derive(Debug)]
struct NameHasher(u64);

impl Default for NameHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for NameHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl ShaderProgram {
    /// Compiles and links the two stages of `source` for the context of `gl`.
    pub fn create(gl: &Gl, source: &ProgramSource) -> Result<Self, GlError> {
        let embedded = gl.embedded();
        let vertex_source = shaders::assemble(&source.vertex, false, embedded);
        let fragment_source = shaders::assemble(&source.fragment, true, embedded);

        if let Some(handle) = gl.cached_program(source.name, &vertex_source, &fragment_source) {
            return Ok(Self {
                gl: gl.clone(),
                handle,
                uniforms: HashMap::default(),
            });
        }
        let vertex = gl.compile(source.name, glow::VERTEX_SHADER, &vertex_source)?;
        let linked = gl
            .compile(source.name, glow::FRAGMENT_SHADER, &fragment_source)
            .and_then(|fragment| {
                let program = gl.link(source.name, vertex, fragment);
                gl.delete_shader(fragment);
                program
            });
        gl.delete_shader(vertex);

        let handle = linked?;
        gl.cache_program(source.name, &vertex_source, &fragment_source, handle);
        Ok(Self {
            gl: gl.clone(),
            handle,
            uniforms: HashMap::default(),
        })
    }

    /// Binds the uniform block `block` to binding point `binding`; `false` when the program has
    /// no such active block.
    pub fn bind_uniform_block(&self, block: &str, binding: u32) -> bool {
        self.gl.uniform_block_binding(self.handle, block, binding)
    }

    /// Makes the program current.
    pub fn use_program(&self) {
        self.gl.use_program(self.handle);
    }

    /// Location of `name` when the value whose bits are `words` has to be uploaded: `None`
    /// when the uniform is not active or already holds exactly these bits.
    fn location(&mut self, name: &'static str, words: &[u32]) -> Option<UniformLocation> {
        let (gl, handle) = (&self.gl, self.handle);
        let uniform = self.uniforms.entry(name).or_insert_with(|| Uniform {
            location: gl.uniform_location(handle, name),
            last: [0; 16],
            len: 0,
        });
        let location = uniform.location?;
        if uniform.len == words.len() && uniform.last[..words.len()] == *words {
            return None;
        }

        uniform.last[..words.len()].copy_from_slice(words);
        uniform.len = words.len();
        Some(location)
    }

    /// Sets an integer uniform.
    pub fn set_i32(&mut self, name: &'static str, value: i32) {
        if let Some(location) = self.location(name, &[u32::from_ne_bytes(value.to_ne_bytes())]) {
            self.gl.uniform_i32(location, value);
        }
    }

    /// Sets a float uniform.
    pub fn set_f32(&mut self, name: &'static str, value: f32) {
        if let Some(location) = self.location(name, &[value.to_bits()]) {
            self.gl.uniform_f32(location, value);
        }
    }

    /// Sets a `vec2` uniform.
    pub fn set_vec2(&mut self, name: &'static str, value: Vector2) {
        if let Some(location) = self.location(name, &[value.x.to_bits(), value.y.to_bits()]) {
            self.gl.uniform_vec2(location, value.x, value.y);
        }
    }

    /// Sets a `vec3` uniform.
    pub fn set_vec3(&mut self, name: &'static str, value: Vector3) {
        let words = [value.x.to_bits(), value.y.to_bits(), value.z.to_bits()];
        if let Some(location) = self.location(name, &words) {
            self.gl.uniform_vec3(location, value.x, value.y, value.z);
        }
    }

    /// Sets a `vec4` uniform.
    pub fn set_vec4(&mut self, name: &'static str, value: Vector4) {
        let words = [
            value.x.to_bits(),
            value.y.to_bits(),
            value.z.to_bits(),
            value.w.to_bits(),
        ];
        if let Some(location) = self.location(name, &words) {
            self.gl
                .uniform_vec4(location, value.x, value.y, value.z, value.w);
        }
    }

    /// Sets a `mat4` uniform from a [`Matrix4x4`]. The matrix is stored row by row and applied
    /// to row vectors, so uploading it untransposed gives GLSL the column-vector matrix the
    /// shaders expect.
    pub fn set_matrix4(&mut self, name: &'static str, value: &Matrix4x4) {
        let cells = value.to_cells();
        if let Some(location) = self.location(name, &cells.map(f32::to_bits)) {
            self.gl.uniform_mat4(location, &cells);
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        self.gl.delete_program(self.handle);
    }
}
