//! The one place the renderer calls OpenGL: a thin wrapper over the host's `glow::Context`
//! that exposes the calls the renderer uses as safe methods.
//!
//! Every method is a single GL call. They are sound under the renderer's contract, stated once
//! here and referred to by each `SAFETY:` note: the context is current on the calling thread for
//! as long as the renderer lives (the host guarantees it, as in the C# reference), every handle
//! passed in was created by this context and not yet deleted, and every slice passed to an
//! upload holds at least the bytes the format and size describe (the callers size them).

use std::path::PathBuf;
use std::rc::Rc;

use glow::{
    Buffer, Framebuffer, HasContext, PixelUnpackData, Program, ProgramBinary, Query, Renderbuffer,
    Shader, Texture, UniformLocation, VertexArray,
};
use thiserror::Error;

use super::program_cache::{supports_program_binary, ProgramCache};

/// A GL object could not be created, compiled or linked.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlError {
    /// The driver refused to create an object.
    #[error("cannot create a GL {kind}: {message}")]
    Create {
        /// What was being created.
        kind: &'static str,
        /// The driver's message.
        message: String,
    },
    /// A shader did not compile.
    #[error("compiling the {stage} shader of '{program}' failed: {log}")]
    Compile {
        /// Name of the program.
        program: String,
        /// `vertex` or `fragment`.
        stage: &'static str,
        /// The compiler log.
        log: String,
    },
    /// A program did not link.
    #[error("linking the program '{program}' failed: {log}")]
    Link {
        /// Name of the program.
        program: String,
        /// The linker log.
        log: String,
    },
    /// A framebuffer the renderer needs is not complete.
    #[error("the {what} framebuffer {width}x{height} is incomplete (status 0x{status:X})")]
    Incomplete {
        /// Which framebuffer.
        what: &'static str,
        /// Width in pixels.
        width: i32,
        /// Height in pixels.
        height: i32,
        /// What `glCheckFramebufferStatus` answered.
        status: u32,
    },
}

/// The host context, shared by the parts of one renderer.
#[derive(Clone)]
pub struct Gl {
    context: Rc<glow::Context>,
    embedded: bool,
    /// The binaries of linked programs, when the driver can give them and the host asked.
    cache: Option<Rc<ProgramCache>>,
}

impl std::fmt::Debug for Gl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gl")
            .field("embedded", &self.embedded)
            .finish_non_exhaustive()
    }
}

/// `GL_GPU_DISJOINT_EXT`.
const GPU_DISJOINT: u32 = 0x8FBB;

fn created<T>(kind: &'static str, result: Result<T, String>) -> Result<T, GlError> {
    result.map_err(|message| GlError::Create { kind, message })
}

impl Gl {
    /// Wraps the host context.
    pub fn new(context: Rc<glow::Context>) -> Self {
        let embedded = context.version().is_embedded;
        Self {
            context,
            embedded,
            cache: None,
        }
    }

    /// Keeps the binaries of the programs in `directory` for the next start, when the driver can
    /// give them; with any other driver the programs are built from source, as before.
    #[must_use]
    pub fn with_program_cache(mut self, directory: PathBuf) -> Self {
        if supports_program_binary(self.context.version(), self.context.supported_extensions()) {
            // SAFETY: renderer contract; two string queries.
            let (renderer, version) = unsafe {
                (
                    self.context.get_parameter_string(glow::RENDERER),
                    self.context.get_parameter_string(glow::VERSION),
                )
            };
            self.cache = Some(Rc::new(ProgramCache::new(directory, &renderer, &version)));
        }
        self
    }

    /// Whether linked programs are saved to and loaded from disk.
    pub const fn caches_programs(&self) -> bool {
        self.cache.is_some()
    }

    /// True on an OpenGL ES context, false on a desktop core one.
    pub const fn embedded(&self) -> bool {
        self.embedded
    }

    // ------------------------------------------------------------ state

    pub fn enable(&self, capability: u32) {
        // SAFETY: renderer contract (module documentation); a plain state call.
        unsafe { self.context.enable(capability) }
    }

    pub fn disable(&self, capability: u32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.disable(capability) }
    }

    pub fn depth_func(&self, function: u32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.depth_func(function) }
    }

    pub fn depth_mask(&self, write: bool) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.depth_mask(write) }
    }

    pub fn color_mask(&self, write: bool) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.color_mask(write, write, write, write) }
    }

    pub fn cull_face(&self, face: u32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.cull_face(face) }
    }

    pub fn front_face(&self, winding: u32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.front_face(winding) }
    }

    pub fn blend_func(&self, source: u32, destination: u32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.blend_func(source, destination) }
    }

    pub fn viewport(&self, width: i32, height: i32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.viewport(0, 0, width, height) }
    }

    pub fn clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.clear_color(r, g, b, a) }
    }

    pub fn clear_depth(&self, depth: f32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.clear_depth_f32(depth) }
    }

    pub fn clear(&self, mask: u32) {
        // SAFETY: renderer contract; clears the bound framebuffer.
        unsafe { self.context.clear(mask) }
    }

    pub fn pixel_store(&self, parameter: u32, value: i32) {
        // SAFETY: renderer contract; a plain state call.
        unsafe { self.context.pixel_store_i32(parameter, value) }
    }

    pub fn get_integer(&self, parameter: u32) -> i32 {
        // SAFETY: renderer contract; `parameter` names a single-integer query.
        unsafe { self.context.get_parameter_i32(parameter) }
    }

    // ------------------------------------------------------------ programs

    pub fn compile(&self, program: &str, stage: u32, source: &str) -> Result<Shader, GlError> {
        // SAFETY: renderer contract; the shader is created, compiled and, on failure, deleted
        // here.
        unsafe {
            let shader = created("shader", self.context.create_shader(stage))?;
            self.context.shader_source(shader, source);
            self.context.compile_shader(shader);
            if self.context.get_shader_compile_status(shader) {
                return Ok(shader);
            }

            let log = self.context.get_shader_info_log(shader);
            self.context.delete_shader(shader);
            Err(GlError::Compile {
                program: program.to_owned(),
                stage: if stage == glow::VERTEX_SHADER {
                    "vertex"
                } else {
                    "fragment"
                },
                log,
            })
        }
    }

    pub fn link(&self, name: &str, vertex: Shader, fragment: Shader) -> Result<Program, GlError> {
        // SAFETY: renderer contract; both shaders are live and compiled, the program is created
        // here and deleted on failure.
        unsafe {
            let program = created("program", self.context.create_program())?;
            if self.caches_programs() {
                // Some drivers only give the binary of a program that was linked with the hint.
                self.context.program_binary_retrievable_hint(program, true);
            }
            self.context.attach_shader(program, vertex);
            self.context.attach_shader(program, fragment);
            self.context.link_program(program);
            if !self.context.get_program_link_status(program) {
                let log = self.context.get_program_info_log(program);
                self.context.delete_program(program);
                return Err(GlError::Link {
                    program: name.to_owned(),
                    log,
                });
            }

            self.context.detach_shader(program, vertex);
            self.context.detach_shader(program, fragment);
            Ok(program)
        }
    }

    /// The program of the binary the cache holds for these sources, when it has one and the driver
    /// accepts it; `None` means the sources must be compiled.
    pub fn cached_program(&self, name: &str, vertex: &str, fragment: &str) -> Option<Program> {
        let cache = self.cache.as_ref()?;
        let key = cache.key(name, vertex, fragment);
        let (format, buffer) = cache.load(&key)?;
        // SAFETY: renderer contract; the program is created here, and deleted when the driver
        // refuses the binary.
        unsafe {
            let program = self.context.create_program().ok()?;
            self.context
                .program_binary(program, &ProgramBinary { buffer, format });
            if self.context.get_program_link_status(program) {
                return Some(program);
            }
            self.context.delete_program(program);
        }
        // A binary of another driver version, or a damaged one: the file is no use again.
        cache.discard(&key);
        None
    }

    /// Files the binary of the freshly linked `program` for the next start. A driver that gives no
    /// binary, or a folder that cannot be written, leaves the cache without it: the next start
    /// compiles that program again.
    pub fn cache_program(&self, name: &str, vertex: &str, fragment: &str, program: Program) {
        let Some(cache) = &self.cache else {
            return;
        };
        // SAFETY: renderer contract; the program is live and linked.
        let binary = unsafe { self.context.get_program_binary(program) };
        if let Some(ProgramBinary { buffer, format }) = binary.filter(|b| !b.buffer.is_empty()) {
            let _ = cache.store(&cache.key(name, vertex, fragment), format, &buffer);
        }
    }

    pub fn delete_shader(&self, shader: Shader) {
        // SAFETY: renderer contract; the shader is live.
        unsafe { self.context.delete_shader(shader) }
    }

    pub fn delete_program(&self, program: Program) {
        // SAFETY: renderer contract; the program is live.
        unsafe { self.context.delete_program(program) }
    }

    pub fn use_program(&self, program: Program) {
        // SAFETY: renderer contract; the program is live and linked.
        unsafe { self.context.use_program(Some(program)) }
    }

    pub fn uniform_location(&self, program: Program, name: &str) -> Option<UniformLocation> {
        // SAFETY: renderer contract; the program is live and linked.
        unsafe { self.context.get_uniform_location(program, name) }
    }

    pub fn uniform_i32(&self, location: UniformLocation, value: i32) {
        // SAFETY: renderer contract; the location belongs to the program in use.
        unsafe { self.context.uniform_1_i32(Some(&location), value) }
    }

    pub fn uniform_f32(&self, location: UniformLocation, value: f32) {
        // SAFETY: renderer contract; the location belongs to the program in use.
        unsafe { self.context.uniform_1_f32(Some(&location), value) }
    }

    pub fn uniform_vec2(&self, location: UniformLocation, x: f32, y: f32) {
        // SAFETY: renderer contract; the location belongs to the program in use.
        unsafe { self.context.uniform_2_f32(Some(&location), x, y) }
    }

    pub fn uniform_vec3(&self, location: UniformLocation, x: f32, y: f32, z: f32) {
        // SAFETY: renderer contract; the location belongs to the program in use.
        unsafe { self.context.uniform_3_f32(Some(&location), x, y, z) }
    }

    pub fn uniform_vec4(&self, location: UniformLocation, x: f32, y: f32, z: f32, w: f32) {
        // SAFETY: renderer contract; the location belongs to the program in use.
        unsafe { self.context.uniform_4_f32(Some(&location), x, y, z, w) }
    }

    pub fn uniform_mat4(&self, location: UniformLocation, cells: &[f32; 16]) {
        // SAFETY: renderer contract; the location belongs to the program in use and the slice
        // holds one matrix.
        unsafe {
            self.context
                .uniform_matrix_4_f32_slice(Some(&location), false, cells);
        }
    }

    // ------------------------------------------------------------ buffers and vertex arrays

    pub fn create_vertex_array(&self) -> Result<VertexArray, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("vertex array", unsafe {
            self.context.create_vertex_array()
        })
    }

    pub fn bind_vertex_array(&self, array: Option<VertexArray>) {
        // SAFETY: renderer contract; the array is live or `None`.
        unsafe { self.context.bind_vertex_array(array) }
    }

    pub fn delete_vertex_array(&self, array: VertexArray) {
        // SAFETY: renderer contract; the array is live.
        unsafe { self.context.delete_vertex_array(array) }
    }

    pub fn create_buffer(&self) -> Result<Buffer, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("buffer", unsafe { self.context.create_buffer() })
    }

    pub fn bind_buffer(&self, target: u32, buffer: Option<Buffer>) {
        // SAFETY: renderer contract; the buffer is live or `None`.
        unsafe { self.context.bind_buffer(target, buffer) }
    }

    pub fn buffer_data(&self, target: u32, data: &[u8]) {
        // SAFETY: renderer contract; a buffer is bound to `target` and GL copies the slice.
        unsafe {
            self.context
                .buffer_data_u8_slice(target, data, glow::STATIC_DRAW);
        }
    }

    pub fn buffer_stream_data(&self, target: u32, data: &[u8]) {
        // SAFETY: renderer contract; a buffer is bound to `target` and GL copies the slice.
        unsafe {
            self.context
                .buffer_data_u8_slice(target, data, glow::STREAM_DRAW);
        }
    }

    pub fn bind_buffer_range(
        &self,
        target: u32,
        index: u32,
        buffer: Buffer,
        offset: i32,
        size: i32,
    ) {
        // SAFETY: renderer contract; `offset` is a multiple of the implementation's alignment
        // and `offset + size` lies inside the buffer's data.
        unsafe {
            self.context
                .bind_buffer_range(target, index, Some(buffer), offset, size);
        }
    }

    /// Binds the uniform block `block` of `program` to binding point `binding`; `false` when
    /// the program has no such active block.
    pub fn uniform_block_binding(&self, program: Program, block: &str, binding: u32) -> bool {
        // SAFETY: renderer contract.
        unsafe {
            match self.context.get_uniform_block_index(program, block) {
                Some(index) => {
                    self.context.uniform_block_binding(program, index, binding);
                    true
                }
                None => false,
            }
        }
    }

    pub fn delete_buffer(&self, buffer: Buffer) {
        // SAFETY: renderer contract; the buffer is live.
        unsafe { self.context.delete_buffer(buffer) }
    }

    pub fn float_attribute(&self, index: u32, size: i32, stride: i32, offset: i32) {
        // SAFETY: renderer contract; a vertex array and an array buffer are bound, and the
        // offset lies inside the vertex layout the caller uploaded.
        unsafe {
            self.context.enable_vertex_attrib_array(index);
            self.context
                .vertex_attrib_pointer_f32(index, size, glow::FLOAT, false, stride, offset);
        }
    }

    pub fn draw_triangles_indexed(&self, count: i32) {
        // SAFETY: renderer contract; the bound vertex array holds an element buffer of at least
        // `count` 32-bit indices, all inside its vertex buffer.
        unsafe {
            self.context
                .draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, 0);
        }
    }

    pub fn draw_triangles_indexed_instanced(&self, count: i32, instances: i32) {
        // SAFETY: renderer contract; the bound vertex array holds an element buffer of at least
        // `count` 32-bit indices, all inside its vertex buffer, and the bound uniform block
        // range holds `instances` placements.
        unsafe {
            self.context.draw_elements_instanced(
                glow::TRIANGLES,
                count,
                glow::UNSIGNED_INT,
                0,
                instances,
            );
        }
    }

    pub fn draw_triangles(&self, count: i32) {
        // SAFETY: renderer contract; the bound vertex array has no attribute enabled, so the
        // vertex shader builds its corners from gl_VertexID.
        unsafe { self.context.draw_arrays(glow::TRIANGLES, 0, count) }
    }

    // ------------------------------------------------------------ textures

    pub fn create_texture(&self) -> Result<Texture, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("texture", unsafe { self.context.create_texture() })
    }

    pub fn bind_texture(&self, target: u32, texture: Option<Texture>) {
        // SAFETY: renderer contract; the texture is live or `None`.
        unsafe { self.context.bind_texture(target, texture) }
    }

    pub fn active_texture(&self, unit: u32) {
        // SAFETY: renderer contract; `unit` is one of the first four units every context has.
        unsafe { self.context.active_texture(glow::TEXTURE0 + unit) }
    }

    pub fn delete_texture(&self, texture: Texture) {
        // SAFETY: renderer contract; the texture is live.
        unsafe { self.context.delete_texture(texture) }
    }

    #[allow(clippy::too_many_arguments)] // The arguments of glTexImage2D.
    pub fn tex_image_2d(
        &self,
        target: u32,
        internal_format: u32,
        width: i32,
        height: i32,
        format: u32,
        ty: u32,
        pixels: Option<&[u8]>,
    ) {
        // SAFETY: renderer contract; a texture is bound to `target` and `pixels`, when given,
        // holds `width × height` texels of `format`/`ty` at the current unpack alignment.
        #[allow(clippy::cast_possible_wrap)] // GL enums are small positive values.
        unsafe {
            self.context.tex_image_2d(
                target,
                0,
                internal_format as i32,
                width,
                height,
                0,
                format,
                ty,
                PixelUnpackData::Slice(pixels),
            );
        }
    }

    pub fn tex_sub_image_2d(&self, target: u32, width: i32, height: i32, format: u32, data: &[u8]) {
        // SAFETY: renderer contract; the bound texture is at least `width × height` and `data`
        // holds that many texels of `format` in unsigned bytes.
        unsafe {
            self.context.tex_sub_image_2d(
                target,
                0,
                0,
                0,
                width,
                height,
                format,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(Some(data)),
            );
        }
    }

    pub fn tex_parameter(&self, target: u32, parameter: u32, value: u32) {
        // SAFETY: renderer contract; a texture is bound to `target`.
        #[allow(clippy::cast_possible_wrap)] // GL enums and levels are small positive values.
        unsafe {
            self.context
                .tex_parameter_i32(target, parameter, value as i32);
        }
    }

    pub fn tex_border_color(&self, target: u32, color: &[f32; 4]) {
        // SAFETY: renderer contract; a texture is bound to `target`.
        unsafe {
            self.context
                .tex_parameter_f32_slice(target, glow::TEXTURE_BORDER_COLOR, color);
        }
    }

    // ------------------------------------------------------------ framebuffers

    /// Whether the context times GPU work: OpenGL 3.3 has timer queries, OpenGL ES needs
    /// `GL_EXT_disjoint_timer_query`.
    pub fn supports_timer_queries(&self) -> bool {
        !self.embedded
            || self
                .context
                .supported_extensions()
                .contains("GL_EXT_disjoint_timer_query")
    }

    pub fn create_query(&self) -> Result<Query, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("query", unsafe { self.context.create_query() })
    }

    pub fn delete_query(&self, query: Query) {
        // SAFETY: renderer contract; the query is live.
        unsafe { self.context.delete_query(query) }
    }

    /// Starts timing the GPU work that follows into `query`.
    pub fn begin_time_elapsed(&self, query: Query) {
        // SAFETY: renderer contract; the query is live and no other time query is open
        // (`GpuTimer` opens one at a time).
        unsafe { self.context.begin_query(glow::TIME_ELAPSED, query) }
    }

    /// Ends the open time query.
    pub fn end_time_elapsed(&self) {
        // SAFETY: renderer contract; `GpuTimer` only ends the query it opened.
        unsafe { self.context.end_query(glow::TIME_ELAPSED) }
    }

    /// The oldest error the context recorded since the last call, cleared (`NO_ERROR` when
    /// none).
    pub fn take_error(&self) -> u32 {
        // SAFETY: renderer contract; a plain state query.
        unsafe { self.context.get_error() }
    }

    /// Whether the result of `query` is there, without waiting for it.
    pub fn query_ready(&self, query: Query) -> bool {
        // SAFETY: renderer contract; the query is live and has ended.
        unsafe {
            self.context
                .get_query_parameter_u32(query, glow::QUERY_RESULT_AVAILABLE)
                != 0
        }
    }

    /// The nanoseconds `query` measured; its result is available.
    pub fn query_nanoseconds(&self, query: Query) -> u32 {
        // SAFETY: renderer contract; the query is live and its result available, so reading it
        // does not wait.
        unsafe {
            self.context
                .get_query_parameter_u32(query, glow::QUERY_RESULT)
        }
    }

    /// Whether the GPU went through a disjoint period (OpenGL ES): the last results are not to
    /// be trusted.
    pub fn gpu_disjoint(&self) -> bool {
        // SAFETY: renderer contract; a plain state query of the extension the timer requires.
        self.embedded && unsafe { self.context.get_parameter_i32(GPU_DISJOINT) } != 0
    }

    pub fn create_framebuffer(&self) -> Result<Framebuffer, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("framebuffer", unsafe { self.context.create_framebuffer() })
    }

    pub fn bind_framebuffer(&self, target: u32, framebuffer: Option<Framebuffer>) {
        // SAFETY: renderer contract; the framebuffer is live, or `None` for the default one.
        unsafe { self.context.bind_framebuffer(target, framebuffer) }
    }

    pub fn delete_framebuffer(&self, framebuffer: Framebuffer) {
        // SAFETY: renderer contract; the framebuffer is live.
        unsafe { self.context.delete_framebuffer(framebuffer) }
    }

    pub fn framebuffer_texture(&self, attachment: u32, texture: Texture) {
        // SAFETY: renderer contract; a framebuffer is bound and the texture is a live 2D one.
        unsafe {
            self.context.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                attachment,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
        }
    }

    pub fn framebuffer_renderbuffer(&self, attachment: u32, renderbuffer: Renderbuffer) {
        // SAFETY: renderer contract; a framebuffer is bound and the renderbuffer is live.
        unsafe {
            self.context.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                attachment,
                glow::RENDERBUFFER,
                Some(renderbuffer),
            );
        }
    }

    pub fn framebuffer_status(&self) -> u32 {
        // SAFETY: renderer contract; queries the bound framebuffer.
        unsafe { self.context.check_framebuffer_status(glow::FRAMEBUFFER) }
    }

    pub fn draw_no_colour(&self) {
        // SAFETY: renderer contract; a framebuffer object is bound. GLES 3.0 has no
        // glDrawBuffer, so the single-entry glDrawBuffers form is used on both.
        unsafe { self.context.draw_buffers(&[glow::NONE]) }
    }

    pub fn blit_colour(&self, width: i32, height: i32) {
        // SAFETY: renderer contract; a read and a draw framebuffer of at least that size are
        // bound.
        unsafe {
            self.context.blit_framebuffer(
                0,
                0,
                width,
                height,
                0,
                0,
                width,
                height,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
        }
    }

    pub fn create_renderbuffer(&self) -> Result<Renderbuffer, GlError> {
        // SAFETY: renderer contract; creates a new object.
        created("renderbuffer", unsafe {
            self.context.create_renderbuffer()
        })
    }

    pub fn renderbuffer_storage(
        &self,
        renderbuffer: Renderbuffer,
        samples: i32,
        format: u32,
        width: i32,
        height: i32,
    ) {
        // SAFETY: renderer contract; the renderbuffer is live; it is bound for the storage call
        // and unbound afterwards.
        unsafe {
            self.context
                .bind_renderbuffer(glow::RENDERBUFFER, Some(renderbuffer));
            if samples > 1 {
                self.context.renderbuffer_storage_multisample(
                    glow::RENDERBUFFER,
                    samples,
                    format,
                    width,
                    height,
                );
            } else {
                self.context
                    .renderbuffer_storage(glow::RENDERBUFFER, format, width, height);
            }
            self.context.bind_renderbuffer(glow::RENDERBUFFER, None);
        }
    }

    pub fn delete_renderbuffer(&self, renderbuffer: Renderbuffer) {
        // SAFETY: renderer contract; the renderbuffer is live.
        unsafe { self.context.delete_renderbuffer(renderbuffer) }
    }
}
