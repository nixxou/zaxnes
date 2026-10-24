//! What the loop draws with: the GL renderer over the window's GL context (every build), or,
//! with the `wgpu` feature, the wgpu renderer over the window's wgpu display (`video.backend`,
//! `--backend`). The picture of the scene is the host's view of the game either way.

use std::path::Path;
use std::rc::Rc;

use zax_platform::framebuffer::{self, OffscreenFramebuffer};
use zax_platform::HostWindow;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::statistics::RenderStatistics;
use zax_render::target::RenderTarget;
use zax_render::SceneRenderer;

use crate::emulation::host::GameView;
use crate::ui::frontend::ImguiFrontend;

/// The size of a save-state and cover thumbnail.
pub const THUMBNAIL_SIZE: (usize, usize) = (256, 192);

/// The graphics API a run asks for: `--backend`, else `video.backend` (0 OpenGL, 1 Vulkan,
/// 2 DirectX 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    OpenGl,
    Vulkan,
    Dx12,
}

impl Backend {
    /// The backend of a `video.backend` value; anything unknown is OpenGL.
    #[must_use]
    pub const fn from_setting(value: i32) -> Self {
        match value {
            1 => Self::Vulkan,
            2 => Self::Dx12,
            _ => Self::OpenGl,
        }
    }

    /// The backend this build can open for the one asked: OpenGL without the `wgpu` feature,
    /// Vulkan for DirectX 12 off Windows. The second value says why it differs, if it does.
    #[must_use]
    pub fn available(self) -> (Self, Option<&'static str>) {
        match self {
            Self::OpenGl => (self, None),
            _ if !cfg!(feature = "wgpu") => (
                Self::OpenGl,
                Some("this build has no Vulkan or DirectX 12 renderer"),
            ),
            Self::Dx12 if !cfg!(windows) => (Self::Vulkan, Some("DirectX 12 is a Windows API")),
            _ => (self, None),
        }
    }

    /// What the window is made with for this backend.
    #[must_use]
    pub const fn window_graphics(self) -> zax_platform::WindowGraphics {
        match self {
            Self::OpenGl => zax_platform::WindowGraphics::Gl,
            #[cfg(feature = "wgpu")]
            Self::Vulkan => {
                zax_platform::WindowGraphics::Wgpu(zax_platform::wgpu_display::GpuApi::Vulkan)
            }
            #[cfg(feature = "wgpu")]
            Self::Dx12 => {
                zax_platform::WindowGraphics::Wgpu(zax_platform::wgpu_display::GpuApi::Dx12)
            }
            #[cfg(not(feature = "wgpu"))]
            Self::Vulkan | Self::Dx12 => zax_platform::WindowGraphics::Gl,
        }
    }
}

/// The picture of the scene: the host's view of the game.
pub enum SceneView {
    Gl(OffscreenFramebuffer),
    #[cfg(feature = "wgpu")]
    Wgpu(WgpuView),
}

impl SceneView {
    /// Gives the picture `width` x `height` pixels (at least 1 x 1).
    ///
    /// # Errors
    ///
    /// The GL framebuffer could not be made again.
    pub fn resize(&mut self, width: i32, height: i32) -> Result<(), String> {
        match self {
            Self::Gl(view) => view
                .resize(width, height)
                .map_err(|error| error.to_string()),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(view) => {
                view.resize(width, height);
                Ok(())
            }
        }
    }
}

impl GameView for SceneView {
    fn capture_thumbnail(&mut self) -> Option<Vec<u8>> {
        let (width, height) = THUMBNAIL_SIZE;
        match self {
            Self::Gl(view) => view.read_scaled_rgb(
                i32::try_from(width).unwrap_or(0),
                i32::try_from(height).unwrap_or(0),
            ),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(view) => Some(view.read_scaled_rgb(width, height)),
        }
    }
}

/// The scene picture of the wgpu renderer: an RGBA8 texture, sampled by the window and read
/// back for the thumbnails.
#[cfg(feature = "wgpu")]
pub struct WgpuView {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

#[cfg(feature = "wgpu")]
impl WgpuView {
    /// The format the scene renderer resolves into.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    /// A picture of `width` x `height` on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: i32, height: i32) -> Self {
        let (width, height) = (size(width), size(height));
        let (texture, view) = Self::texture(device, width, height);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            texture,
            view,
            width,
            height,
        }
    }

    fn texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zax scene picture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn resize(&mut self, width: i32, height: i32) {
        let (width, height) = (size(width), size(height));
        if (width, height) == (self.width, self.height) {
            return;
        }
        (self.texture, self.view) = Self::texture(&self.device, width, height);
        (self.width, self.height) = (width, height);
    }

    /// The texture view the window samples.
    #[must_use]
    pub const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// The picture, nearest-sampled to `width` x `height` RGB, rows top to bottom.
    fn read_scaled_rgb(&self, width: usize, height: usize) -> Vec<u8> {
        let rgba = zax_platform::wgpu_display::read_rgba(
            &self.device,
            &self.queue,
            &self.texture,
            self.width,
            self.height,
        );
        let (source_width, source_height) = (self.width as usize, self.height as usize);
        let mut target = vec![0_u8; width * height * 3];
        for y in 0..height {
            let source_y = (y * source_height / height).min(source_height - 1);
            for x in 0..width {
                let source_x = (x * source_width / width).min(source_width - 1);
                let from = (source_y * source_width + source_x) * 4;
                let to = (y * width + x) * 3;
                target[to..to + 3].copy_from_slice(&rgba[from..from + 3]);
            }
        }
        target
    }
}

#[cfg(feature = "wgpu")]
fn size(value: i32) -> u32 {
    u32::try_from(value).unwrap_or(1).max(1)
}

/// The scene renderer of the run.
#[allow(clippy::large_enum_variant)] // One per run, never moved around.
pub enum Graphics {
    Gl {
        gl: Rc<glow::Context>,
        renderer: SceneRenderer,
    },
    #[cfg(feature = "wgpu")]
    Wgpu(Box<zax_render::wgpu_backend::WgpuSceneRenderer>),
}

impl Graphics {
    /// What the last rendered frame cost.
    #[must_use]
    pub fn statistics(&self) -> &RenderStatistics {
        match self {
            Self::Gl { renderer, .. } => renderer.statistics(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(renderer) => renderer.statistics(),
        }
    }

    /// Turns the GPU timing on or off; false when the renderer cannot time its work (the wgpu
    /// renderer does not yet).
    pub fn set_gpu_timing(&mut self, enabled: bool) -> bool {
        match self {
            Self::Gl { renderer, .. } => renderer.set_gpu_timing(enabled),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(_) => false,
        }
    }

    /// Draws `frame` with `scene` into `view`.
    ///
    /// # Errors
    ///
    /// The renderer failed, or the view is not the renderer's kind.
    pub fn render(
        &mut self,
        view: &SceneView,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
    ) -> Result<(), String> {
        match (self, view) {
            (Self::Gl { renderer, .. }, SceneView::Gl(view)) => {
                let target = RenderTarget {
                    framebuffer: view.handle(),
                    width: view.width(),
                    height: view.height(),
                };
                renderer
                    .render(&target, scene, frame)
                    .map_err(|error| error.to_string())
            }
            #[cfg(feature = "wgpu")]
            (Self::Wgpu(renderer), SceneView::Wgpu(view)) => renderer
                .render(
                    zax_render::wgpu_backend::WgpuTarget {
                        view: &view.view,
                        width: view.width,
                        height: view.height,
                    },
                    scene,
                    frame,
                )
                .map_err(|error| error.to_string()),
            #[cfg(feature = "wgpu")]
            _ => Err("the scene view does not belong to the renderer".to_owned()),
        }
    }

    /// Puts the frame on the window: the scene picture `view` (when there is one) under the
    /// menu bar, `view_height` pixels high, then the interface [`ImguiFrontend::build`] built.
    pub fn draw_window(
        &self,
        window: &mut HostWindow,
        view: Option<&SceneView>,
        ui: &mut ImguiFrontend,
        (width, height): (i32, i32),
        view_height: i32,
    ) {
        match self {
            Self::Gl { gl, .. } => {
                framebuffer::begin_window_pass(gl, width, height);
                if let Some(SceneView::Gl(view)) = view {
                    view.blit_to_default_framebuffer(width, view_height);
                }
                ui.render();
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(_) => {
                let layer = match view {
                    Some(SceneView::Wgpu(view)) => Some(zax_platform::wgpu_display::SceneLayer {
                        view: &view.view,
                        height: size(view_height),
                    }),
                    _ => None,
                };
                if let Some(display) = window.wgpu_mut() {
                    display.draw((size(width), size(height)), layer, Some(ui.draw_data()));
                }
            }
        }
        #[cfg(not(feature = "wgpu"))]
        let _ = window;
    }

    /// Writes the window's picture to `path` as a PNG (the command-line captures).
    pub fn capture_window(&self, window: &HostWindow, width: i32, height: i32, path: &Path) {
        let rgb = match self {
            Self::Gl { gl, .. } => Some(framebuffer::read_window_rgb(gl, width, height)),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(_) => window
                .wgpu()
                .and_then(zax_platform::wgpu_display::WgpuDisplay::read_rgb),
        };
        #[cfg(not(feature = "wgpu"))]
        let _ = window;
        let Some(rgb) = rgb else {
            log("the window cannot be read back: no capture");
            return;
        };
        let encoded = zax_render::assets::png_image::encode_rgb(
            u32::try_from(width).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
            &rgb,
        );
        match encoded {
            Ok(png) => match std::fs::write(path, png) {
                Ok(()) => log(&format!("captured {}", path.display())),
                Err(error) => log(&format!("the capture could not be written: {error}")),
            },
            Err(error) => log(&format!("the capture could not be encoded: {error}")),
        }
    }
}

fn log(message: &str) {
    crate::app_log::line(&format!("Zaxnes: {message}"));
}
