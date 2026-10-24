//! Draws the Dear `ImGui` draw data with `glow` (the C# `ImGuiController` of `Silk.NET`). The
//! upstream `imgui-glow-renderer` still builds on `glow` 0.14 and the application uses 0.16, so
//! the few calls it needs live here, where the GL `unsafe` is allowed. Desktop GL 3.3 core and
//! GLES 3.0 share the program; the textures are the font atlas and the interface's pictures
//! (`ui_image`).

use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU32;
use std::rc::Rc;

use glow::HasContext;
use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawVert, FontAtlas, TextureId};

use crate::ui_image::{PictureUpdate, FONT_TEXTURE_ID};
use crate::PlatformError;

const VERTEX_SHADER: &str = "\
uniform mat4 ProjMtx;
in vec2 Position;
in vec2 UV;
in vec4 Color;
out vec2 Frag_UV;
out vec4 Frag_Color;
void main() {
    Frag_UV = UV;
    Frag_Color = Color;
    gl_Position = ProjMtx * vec4(Position.xy, 0.0, 1.0);
}
";

const FRAGMENT_SHADER: &str = "\
uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;
out vec4 Out_Color;
void main() {
    Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
}
";

/// The GL objects the interface is drawn with.
pub struct ImguiRenderer {
    gl: Rc<glow::Context>,
    program: glow::Program,
    projection: Option<glow::UniformLocation>,
    vertex_array: glow::VertexArray,
    vertex_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    font_texture: glow::Texture,
    /// The interface's pictures, by texture id.
    pictures: HashMap<usize, glow::Texture>,
}

/// The GL state the interface changes, put back after it is drawn so that the next scene pass
/// finds what it left.
struct SavedState {
    program: i32,
    texture: i32,
    active_texture: i32,
    vertex_array: i32,
    array_buffer: i32,
    viewport: [i32; 4],
    scissor_box: [i32; 4],
    capabilities: [(u32, bool); 5],
}

/// The capabilities an interface pass switches.
const CAPABILITIES: [u32; 5] = [
    glow::BLEND,
    glow::CULL_FACE,
    glow::DEPTH_TEST,
    glow::STENCIL_TEST,
    glow::SCISSOR_TEST,
];

impl ImguiRenderer {
    /// Compiles the program, uploads the font atlas (building it) and gives it the texture id.
    ///
    /// # Errors
    /// A shader does not compile or link, or an object cannot be created.
    pub fn new(gl: &Rc<glow::Context>, fonts: &mut FontAtlas) -> Result<Self, PlatformError> {
        let header = if gl.version().is_embedded {
            "#version 300 es\nprecision mediump float;\n"
        } else {
            "#version 330 core\n"
        };
        // SAFETY: plain GL calls on the context the caller made current; every object created is
        // owned by the renderer and deleted in `Drop`.
        let renderer = unsafe {
            let program = link(gl, header)?;
            let position = attribute(gl, program, "Position")?;
            let uv = attribute(gl, program, "UV")?;
            let color = attribute(gl, program, "Color")?;
            let projection = gl.get_uniform_location(program, "ProjMtx");
            let vertex_array = gl.create_vertex_array().map_err(PlatformError::from)?;
            let vertex_buffer = gl.create_buffer().map_err(PlatformError::from)?;
            let index_buffer = gl.create_buffer().map_err(PlatformError::from)?;
            gl.bind_vertex_array(Some(vertex_array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            let stride = i32::try_from(size_of::<DrawVert>()).unwrap_or(20);
            gl.enable_vertex_attrib_array(position);
            gl.vertex_attrib_pointer_f32(position, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(uv);
            gl.vertex_attrib_pointer_f32(uv, 2, glow::FLOAT, false, stride, 8);
            gl.enable_vertex_attrib_array(color);
            gl.vertex_attrib_pointer_f32(color, 4, glow::UNSIGNED_BYTE, true, stride, 16);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let atlas = fonts.build_rgba32_texture();
            let font_texture = upload_rgba(gl, atlas.width, atlas.height, atlas.data)?;
            Self {
                gl: Rc::clone(gl),
                program,
                projection,
                vertex_array,
                vertex_buffer,
                index_buffer,
                font_texture,
                pictures: HashMap::new(),
            }
        };
        fonts.tex_id = TextureId::new(FONT_TEXTURE_ID);
        Ok(renderer)
    }

    /// Applies the changes of the interface's pictures; a picture whose pixels do not fill its
    /// size is left out.
    ///
    /// The context must be current on the calling thread.
    pub fn update_pictures(&mut self, updates: &[PictureUpdate]) {
        for update in updates {
            if let Some(texture) = self.pictures.remove(&update.id) {
                // SAFETY: deletes a texture this renderer made, on its context.
                unsafe { self.gl.delete_texture(texture) };
            }
            let Some(picture) = update.picture.as_ref().filter(|picture| picture.is_whole()) else {
                continue;
            };
            if let Ok(texture) = upload_rgba(&self.gl, picture.width, picture.height, &picture.rgba)
            {
                self.pictures.insert(update.id, texture);
            }
        }
    }

    /// The texture of the texture id `id`: the font atlas, or one of the pictures.
    fn texture(&self, id: usize) -> Option<glow::Texture> {
        if id == FONT_TEXTURE_ID {
            Some(self.font_texture)
        } else {
            self.pictures.get(&id).copied()
        }
    }

    /// Draws `draw_data` over the window's framebuffer.
    pub fn render(&self, draw_data: &DrawData) {
        let [display_width, display_height] = draw_data.display_size;
        let [scale_x, scale_y] = draw_data.framebuffer_scale;
        #[allow(clippy::cast_possible_truncation)] // pixel sizes of a window
        let (width, height) = (
            (display_width * scale_x) as i32,
            (display_height * scale_y) as i32,
        );
        if width <= 0 || height <= 0 {
            return;
        }
        let gl = &*self.gl;
        // SAFETY: plain GL calls on the current context; the buffers are the renderer's own and
        // the slices handed to GL are the draw lists', alive for the call.
        unsafe {
            let saved = SavedState::read(gl);
            self.set_up(draw_data, width, height);
            let clip_offset = draw_data.display_pos;
            let mut bound = FONT_TEXTURE_ID;
            for list in draw_data.draw_lists() {
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    as_bytes(list.vtx_buffer()),
                    glow::STREAM_DRAW,
                );
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    as_bytes(list.idx_buffer()),
                    glow::STREAM_DRAW,
                );
                for command in list.commands() {
                    if let DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                clip_rect,
                                texture_id,
                                ..
                            },
                    } = command
                    {
                        let id = texture_id.id();
                        let Some(texture) = self.texture(id) else {
                            continue;
                        };
                        if id != bound {
                            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                            bound = id;
                        }
                        let clip = [
                            (clip_rect[0] - clip_offset[0]) * scale_x,
                            (clip_rect[1] - clip_offset[1]) * scale_y,
                            (clip_rect[2] - clip_offset[0]) * scale_x,
                            (clip_rect[3] - clip_offset[1]) * scale_y,
                        ];
                        if clip[2] <= clip[0] || clip[3] <= clip[1] {
                            continue;
                        }
                        #[allow(clippy::cast_possible_truncation)] // pixel rectangle
                        gl.scissor(
                            clip[0] as i32,
                            height - clip[3] as i32,
                            (clip[2] - clip[0]) as i32,
                            (clip[3] - clip[1]) as i32,
                        );
                        if let (Ok(count), Ok(offset)) = (
                            i32::try_from(count),
                            i32::try_from(command_offset(&command) * size_of::<DrawIdx>()),
                        ) {
                            gl.draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_SHORT, offset);
                        }
                    }
                }
            }
            saved.restore(gl);
        }
    }

    /// Binds the program, the buffers and the fixed state of an interface pass.
    ///
    /// The context must be current on the calling thread.
    fn set_up(&self, draw_data: &DrawData, width: i32, height: i32) {
        // SAFETY: plain GL calls on the context the caller made current.
        unsafe {
            let gl = &*self.gl;
            let [left, top] = draw_data.display_pos;
            let [right, bottom] = [
                left + draw_data.display_size[0],
                top + draw_data.display_size[1],
            ];
            let projection: [f32; 16] = [
                2.0 / (right - left),
                0.0,
                0.0,
                0.0,
                0.0,
                2.0 / (top - bottom),
                0.0,
                0.0,
                0.0,
                0.0,
                -1.0,
                0.0,
                (right + left) / (left - right),
                (top + bottom) / (bottom - top),
                0.0,
                1.0,
            ];
            gl.viewport(0, 0, width, height);
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.enable(glow::SCISSOR_TEST);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.projection.as_ref(), false, &projection);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.font_texture));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        }
    }
}

impl Drop for ImguiRenderer {
    fn drop(&mut self) {
        // SAFETY: deletes the objects `new` created, on the context they belong to.
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_buffer(self.index_buffer);
            self.gl.delete_texture(self.font_texture);
            for (_, texture) in self.pictures.drain() {
                self.gl.delete_texture(texture);
            }
        }
    }
}

/// The index of the first index a command draws.
fn command_offset(command: &DrawCmd) -> usize {
    match command {
        DrawCmd::Elements { cmd_params, .. } => cmd_params.idx_offset,
        _ => 0,
    }
}

/// The bytes of a draw list's vertex or index buffer.
fn as_bytes<T>(slice: &[T]) -> &[u8] {
    // SAFETY: the vertex and index types are plain data without padding that matters to GL, and
    // the byte view has the same lifetime as the slice.
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

/// Compiles and links the program with the version `header`.
///
/// The context must be current on the calling thread.
fn link(gl: &glow::Context, header: &str) -> Result<glow::Program, PlatformError> {
    // SAFETY: plain GL calls on the context the caller made current.
    unsafe {
        let program = gl.create_program().map_err(PlatformError::from)?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, VERTEX_SHADER),
            (glow::FRAGMENT_SHADER, FRAGMENT_SHADER),
        ] {
            let shader = gl.create_shader(kind).map_err(PlatformError::from)?;
            gl.shader_source(shader, &format!("{header}{source}"));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                gl.delete_program(program);
                return Err(PlatformError::from(format!(
                    "the interface shader does not compile: {log}"
                )));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        if !linked {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(PlatformError::from(format!(
                "the interface program does not link: {log}"
            )));
        }
        Ok(program)
    }
}

/// The location of the vertex attribute `name`.
///
/// The context must be current on the calling thread.
fn attribute(gl: &glow::Context, program: glow::Program, name: &str) -> Result<u32, PlatformError> {
    // SAFETY: plain GL calls on the context the caller made current.
    unsafe {
        gl.get_attrib_location(program, name)
            .ok_or_else(|| PlatformError::from(format!("the interface program has no '{name}'")))
    }
}

/// Uploads `width` x `height` RGBA pixels as a texture filtered bilinearly.
///
/// The context must be current on the calling thread.
fn upload_rgba(
    gl: &glow::Context,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<glow::Texture, PlatformError> {
    // SAFETY: plain GL calls on the context the caller made current; `pixels` holds the
    // `width * height * 4` bytes the upload reads.
    unsafe {
        let texture = gl.create_texture().map_err(PlatformError::from)?;
        let previous = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                parameter,
                i32::try_from(value).unwrap_or(0),
            );
        }
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            i32::try_from(glow::RGBA8).unwrap_or(0),
            i32::try_from(width).unwrap_or(0),
            i32::try_from(height).unwrap_or(0),
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        gl.bind_texture(glow::TEXTURE_2D, texture_of(previous));
        Ok(texture)
    }
}

fn texture_of(name: i32) -> Option<glow::Texture> {
    u32::try_from(name)
        .ok()
        .and_then(NonZeroU32::new)
        .map(glow::NativeTexture)
}

impl SavedState {
    /// Reads the state the interface pass is about to change.
    ///
    /// The context must be current on the calling thread.
    fn read(gl: &glow::Context) -> Self {
        // SAFETY: plain GL calls on the context the caller made current.
        unsafe {
            let mut viewport = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let mut scissor_box = [0; 4];
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor_box);
            Self {
                program: gl.get_parameter_i32(glow::CURRENT_PROGRAM),
                texture: gl.get_parameter_i32(glow::TEXTURE_BINDING_2D),
                active_texture: gl.get_parameter_i32(glow::ACTIVE_TEXTURE),
                vertex_array: gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING),
                array_buffer: gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING),
                viewport,
                scissor_box,
                capabilities: CAPABILITIES
                    .map(|capability| (capability, gl.is_enabled(capability))),
            }
        }
    }

    /// Puts the state back.
    ///
    /// The context must be current on the calling thread.
    fn restore(&self, gl: &glow::Context) {
        // SAFETY: plain GL calls on the context the caller made current.
        unsafe {
            let name = |value: i32| u32::try_from(value).ok().and_then(NonZeroU32::new);
            gl.use_program(name(self.program).map(glow::NativeProgram));
            gl.active_texture(u32::try_from(self.active_texture).unwrap_or(glow::TEXTURE0));
            gl.bind_texture(glow::TEXTURE_2D, texture_of(self.texture));
            gl.bind_vertex_array(name(self.vertex_array).map(glow::NativeVertexArray));
            gl.bind_buffer(
                glow::ARRAY_BUFFER,
                name(self.array_buffer).map(glow::NativeBuffer),
            );
            gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            gl.scissor(
                self.scissor_box[0],
                self.scissor_box[1],
                self.scissor_box[2],
                self.scissor_box[3],
            );
            for (capability, enabled) in self.capabilities {
                if enabled {
                    gl.enable(capability);
                } else {
                    gl.disable(capability);
                }
            }
        }
    }
}
