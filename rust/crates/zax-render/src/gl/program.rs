//! A compiled and linked GLSL program, with a cache of uniform locations.
//!
//! OpenGL 3.3 core without any extension (REQ-SCN-056), and the same sources on GLES 3.0
//! (REQ-ARCH-030): no explicit uniform locations, no bindings in the source, no integer texture
//! gathers.

use std::collections::HashMap;

use glow::{Program, UniformLocation};

use super::api::{Gl, GlError};
use super::shaders::{self, ProgramSource};
use crate::numerics::{Matrix4x4, Vector2, Vector3, Vector4};

/// A linked program; deleted when dropped.
#[derive(Debug)]
pub struct ShaderProgram {
    gl: Gl,
    handle: Program,
    uniforms: HashMap<&'static str, Option<UniformLocation>>,
}

impl ShaderProgram {
    /// Compiles and links the two stages of `source` for the context of `gl`.
    pub fn create(gl: &Gl, source: &ProgramSource) -> Result<Self, GlError> {
        let embedded = gl.embedded();
        let vertex_source = shaders::assemble(&source.vertex, false, embedded);
        let fragment_source = shaders::assemble(&source.fragment, true, embedded);

        let vertex = gl.compile(source.name, glow::VERTEX_SHADER, &vertex_source)?;
        let linked = gl
            .compile(source.name, glow::FRAGMENT_SHADER, &fragment_source)
            .and_then(|fragment| {
                let program = gl.link(source.name, vertex, fragment);
                gl.delete_shader(fragment);
                program
            });
        gl.delete_shader(vertex);

        Ok(Self {
            gl: gl.clone(),
            handle: linked?,
            uniforms: HashMap::new(),
        })
    }

    /// Makes the program current.
    pub fn use_program(&self) {
        self.gl.use_program(self.handle);
    }

    /// Location of `name`, or `None` when the uniform is not active.
    fn location(&mut self, name: &'static str) -> Option<UniformLocation> {
        let (gl, handle) = (&self.gl, self.handle);
        *self
            .uniforms
            .entry(name)
            .or_insert_with(|| gl.uniform_location(handle, name))
    }

    /// Sets an integer uniform.
    pub fn set_i32(&mut self, name: &'static str, value: i32) {
        if let Some(location) = self.location(name) {
            self.gl.uniform_i32(location, value);
        }
    }

    /// Sets a float uniform.
    pub fn set_f32(&mut self, name: &'static str, value: f32) {
        if let Some(location) = self.location(name) {
            self.gl.uniform_f32(location, value);
        }
    }

    /// Sets a `vec2` uniform.
    pub fn set_vec2(&mut self, name: &'static str, value: Vector2) {
        if let Some(location) = self.location(name) {
            self.gl.uniform_vec2(location, value.x, value.y);
        }
    }

    /// Sets a `vec3` uniform.
    pub fn set_vec3(&mut self, name: &'static str, value: Vector3) {
        if let Some(location) = self.location(name) {
            self.gl.uniform_vec3(location, value.x, value.y, value.z);
        }
    }

    /// Sets a `vec4` uniform.
    pub fn set_vec4(&mut self, name: &'static str, value: Vector4) {
        if let Some(location) = self.location(name) {
            self.gl
                .uniform_vec4(location, value.x, value.y, value.z, value.w);
        }
    }

    /// Sets a `mat4` uniform from a [`Matrix4x4`]. The matrix is stored row by row and applied
    /// to row vectors, so uploading it untransposed gives GLSL the column-vector matrix the
    /// shaders expect.
    pub fn set_matrix4(&mut self, name: &'static str, value: &Matrix4x4) {
        if let Some(location) = self.location(name) {
            self.gl.uniform_mat4(location, &value.to_cells());
        }
    }

    /// Sets a `mat3` uniform from the nine cells, row by row.
    pub fn set_matrix3(&mut self, name: &'static str, row_major: &[f32; 9]) {
        if let Some(location) = self.location(name) {
            // GLSL reads a mat3 column by column, so the row-major cells are uploaded
            // transposed.
            let m = row_major;
            let cells = [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]];
            self.gl.uniform_mat3(location, &cells);
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        self.gl.delete_program(self.handle);
    }
}
