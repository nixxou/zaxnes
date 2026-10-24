//! The application window and its GL context, or, for the wgpu renderer (feature `wgpu`), its
//! wgpu display ([`WindowGraphics::Wgpu`]).
//!
//! Desktop GL 3.3 core (forward compatible), or GLES 3.0 when `ZAXNES_GLES=1` or on aarch64
//! Linux (see [`GlFlavor`]). SDL is linked dynamically against the system SDL2 by default.
//!
//! Windows packaging (T14): either ship `SDL2.dll` next to the executable, or build with the
//! cargo feature `bundled`, which compiles SDL2 from source and links it statically. The
//! `bundled` feature is also the one for aarch64 packages.

use std::rc::Rc;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::video::{FullscreenType, GLContext, GLProfile, Window};
use sdl2::{EventPump, Sdl, VideoSubsystem};

use crate::icon::{WindowIconImage, WindowIconTarget};
use crate::pacing::{self, DisplayPacingState, DisplaySurface};
use crate::{gl_loader, PlatformError};

/// Everything the platform layer needs in order to open the application window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostWindowSettings {
    /// Window title.
    pub title: String,
    /// Client width in pixels.
    pub width: u32,
    /// Client height in pixels.
    pub height: u32,
    /// Buffer-swap synchronisation (SPEC-06 REQ-SCN-061, SPEC-09 `video.vsync`): 0 off, 1 every
    /// refresh, 2 every other refresh. [`HostWindow::apply_pacing`] puts the exact interval in
    /// force once the GL context exists.
    pub vsync: i32,
    /// Display rate the loop aims at, in frames per second, or 0 for "as fast as the machine
    /// allows" (SPEC-06 REQ-SCN-061). Ignored when `vsync` is on.
    pub target_frame_rate: i32,
    /// Whether the window opens full screen, which is what the persisted `ui.fullscreen` of
    /// SPEC-09 REQ-PERS-067 restores. The size is then the windowed size the window goes back to.
    pub fullscreen: bool,
    /// What draws into the window.
    pub graphics: WindowGraphics,
}

/// What draws into the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowGraphics {
    /// A GL context SDL creates with the window, current on this thread.
    #[default]
    Gl,
    /// A wgpu display on the window ([`HostWindow::wgpu`]), which [`HostWindow::swap`] presents.
    #[cfg(feature = "wgpu")]
    Wgpu(crate::wgpu_display::GpuApi),
}

impl Default for HostWindowSettings {
    /// The P0 defaults: a 1280x720 window titled "Zaxnes", with vsync on.
    fn default() -> Self {
        Self {
            title: "Zaxnes".to_owned(),
            width: 1280,
            height: 720,
            vsync: 1,
            target_frame_rate: 0,
            fullscreen: false,
            graphics: WindowGraphics::Gl,
        }
    }
}

impl HostWindowSettings {
    /// Whether the swaps are synchronised at all.
    #[must_use]
    pub fn is_vsync_enabled(&self) -> bool {
        self.vsync > 0
    }
}

/// Which GL the context is, so that the renderer picks the GLSL version header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlFlavor {
    /// Desktop OpenGL 3.3 core profile (`#version 330 core`).
    Desktop33Core,
    /// OpenGL ES 3.0 (`#version 300 es`).
    Gles30,
}

impl GlFlavor {
    /// The flavour for this machine: GLES 3.0 when `ZAXNES_GLES=1` or on aarch64 Linux,
    /// desktop GL otherwise.
    #[must_use]
    pub fn select() -> Self {
        Self::choose(
            std::env::var("ZAXNES_GLES").ok().as_deref(),
            cfg!(all(target_arch = "aarch64", target_os = "linux")),
        )
    }

    /// The rule of [`select`](Self::select), without the environment.
    #[must_use]
    pub fn choose(zaxnes_gles: Option<&str>, aarch64_linux: bool) -> Self {
        if zaxnes_gles == Some("1") || aarch64_linux {
            Self::Gles30
        } else {
            Self::Desktop33Core
        }
    }

    /// Whether this is an OpenGL ES context.
    #[must_use]
    pub fn is_gles(self) -> bool {
        self == Self::Gles30
    }
}

/// Owns the application window and its GL context, or its wgpu display.
pub struct HostWindow {
    // Field order is drop order: the display and the context go before the window and SDL.
    #[cfg(feature = "wgpu")]
    display: Option<crate::wgpu_display::WgpuDisplay>,
    gl_context: Option<GLContext>,
    window: Window,
    event_pump: EventPump,
    video: VideoSubsystem,
    sdl: Sdl,
    flavor: GlFlavor,
    frame_cap: Option<Duration>,
    next_frame: Instant,
    /// The swap interval the pacing put in force: 0 or 1, or 2 with a GL context.
    swap_interval: i32,
}

impl HostWindow {
    /// Creates a window with a GL context of the flavour [`GlFlavor::select`] picks, current on
    /// this thread.
    ///
    /// # Errors
    /// [`PlatformError`] if the size is zero or SDL cannot create the window or the context.
    pub fn create(settings: &HostWindowSettings) -> Result<Self, PlatformError> {
        if settings.width == 0 || settings.height == 0 {
            return Err(PlatformError::new("Window size must be positive."));
        }

        // The pads keep reporting when the window is not focused, as GLFW's polling did.
        sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");

        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let flavor = GlFlavor::select();
        {
            let attributes = video.gl_attr();
            match flavor {
                GlFlavor::Desktop33Core => {
                    attributes.set_context_profile(GLProfile::Core);
                    attributes.set_context_version(3, 3);
                    attributes.set_context_flags().forward_compatible().set();
                }
                GlFlavor::Gles30 => {
                    attributes.set_context_profile(GLProfile::GLES);
                    attributes.set_context_version(3, 0);
                }
            }
            attributes.set_double_buffer(true);
        }

        let gl = settings.graphics == WindowGraphics::Gl;
        let mut builder = video.window(&settings.title, settings.width, settings.height);
        if gl {
            builder.opengl();
        }
        builder.resizable().position_centered();
        if settings.fullscreen {
            // SPEC-09 REQ-PERS-067: the persisted full-screen state is restored at launch, on
            // the window itself (AMD-032 §6).
            builder.fullscreen_desktop();
        }
        let window = builder.build()?;
        let gl_context = if gl {
            Some(window.gl_create_context()?)
        } else {
            None
        };
        let event_pump = sdl.event_pump()?;

        #[cfg(feature = "wgpu")]
        let display = match settings.graphics {
            // SAFETY: the display is the first field of the window it is stored in, so it is
            // dropped before the SDL window it draws on.
            WindowGraphics::Wgpu(api) => Some(unsafe {
                crate::wgpu_display::WgpuDisplay::new(&window, api, settings.vsync > 0)?
            }),
            WindowGraphics::Gl => None,
        };
        let mut host = Self {
            #[cfg(feature = "wgpu")]
            display,
            gl_context,
            window,
            event_pump,
            video,
            sdl,
            flavor,
            frame_cap: None,
            next_frame: Instant::now(),
            swap_interval: 0,
        };
        host.apply_pacing(settings.vsync, settings.target_frame_rate);
        Ok(host)
    }

    /// The SDL handle, for the input and audio subsystems.
    #[must_use]
    pub fn sdl(&self) -> &Sdl {
        &self.sdl
    }

    /// Which GL the context is.
    #[must_use]
    pub fn gl_flavor(&self) -> GlFlavor {
        self.flavor
    }

    /// The `glow` context the renderer draws with, over this window's GL context (a
    /// [`WindowGraphics::Gl`] window).
    #[must_use]
    pub fn glow_context(&self) -> Rc<glow::Context> {
        // The context created with the window is current on this thread.
        debug_assert!(self.gl_context.as_ref().is_some_and(GLContext::is_current));
        gl_loader::glow_context(&self.video)
    }

    /// The underlying SDL window.
    #[must_use]
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// One line with the SDL video driver, the display mode the window is on and the drawable
    /// size, for the start-up log.
    #[must_use]
    pub fn display_report(&self) -> String {
        let (drawable_width, drawable_height) = self.drawable_size();
        let mode = self.window.display_mode().map_or_else(
            |error| format!("unknown ({error})"),
            |mode| format!("{}x{}@{}Hz", mode.w, mode.h, mode.refresh_rate),
        );
        format!(
            "SDL video driver '{}', display mode {mode}, drawable {drawable_width}x{drawable_height}",
            self.video.current_video_driver()
        )
    }

    /// One line with the GL context SDL created: its profile and version, the colour, depth and
    /// stencil bits, the multisampling and the double buffering it actually got.
    #[must_use]
    pub fn gl_config_report(&self) -> String {
        use sdl2::sys::SDL_GLattr as Attribute;
        if self.gl_context.is_none() {
            return "no GL context: the window is drawn through a surface".to_owned();
        }
        // The `sdl2` getters panic when the driver refuses a query (Mesa refuses the stencil
        // size of the default framebuffer, for one), so SDL is asked directly and a refused
        // attribute reads "?".
        let get = |attribute: Attribute| {
            let mut value = 0;
            // SAFETY: SDL_GL_GetAttribute only writes the int behind the pointer, which lives
            // for the call; the window's GL context is current on this thread.
            let status = unsafe { sdl2::sys::SDL_GL_GetAttribute(attribute, &raw mut value) };
            if status == 0 {
                value.to_string()
            } else {
                "?".to_owned()
            }
        };
        let profile = match get(Attribute::SDL_GL_CONTEXT_PROFILE_MASK).as_str() {
            "1" => "core".to_owned(),
            "2" => "compatibility".to_owned(),
            "4" => "GLES".to_owned(),
            other => format!("profile {other}"),
        };
        format!(
            "GL context {profile} {}.{} (requested {:?}), RGBA {}/{}/{}/{} depth {} stencil {}, \
             multisample {}x{}, double buffer {}, accelerated {}, swap interval {}",
            get(Attribute::SDL_GL_CONTEXT_MAJOR_VERSION),
            get(Attribute::SDL_GL_CONTEXT_MINOR_VERSION),
            self.flavor,
            get(Attribute::SDL_GL_RED_SIZE),
            get(Attribute::SDL_GL_GREEN_SIZE),
            get(Attribute::SDL_GL_BLUE_SIZE),
            get(Attribute::SDL_GL_ALPHA_SIZE),
            get(Attribute::SDL_GL_DEPTH_SIZE),
            get(Attribute::SDL_GL_STENCIL_SIZE),
            get(Attribute::SDL_GL_MULTISAMPLEBUFFERS),
            get(Attribute::SDL_GL_MULTISAMPLESAMPLES),
            get(Attribute::SDL_GL_DOUBLEBUFFER),
            get(Attribute::SDL_GL_ACCELERATED_VISUAL),
            // SAFETY: as above (the wrapper's getter asserts the interval is -1, 0 or 1).
            unsafe { sdl2::sys::SDL_GL_GetSwapInterval() }
        )
    }

    /// Size of the GL drawable in pixels.
    #[must_use]
    pub fn drawable_size(&self) -> (u32, u32) {
        self.window.drawable_size()
    }

    /// Switches full screen on or off (`ui.fullscreen`, REQ-UI-209). The caller re-applies the
    /// pacing afterwards.
    ///
    /// # Errors
    /// [`PlatformError`] if SDL refuses the change.
    pub fn set_fullscreen(&mut self, fullscreen: bool) -> Result<(), PlatformError> {
        let mode = if fullscreen {
            FullscreenType::Desktop
        } else {
            FullscreenType::Off
        };
        self.window.set_fullscreen(mode)?;
        Ok(())
    }

    /// Drains the pending SDL events, handing each to `handler` (feed them to
    /// [`SdlInput::handle_event`](crate::input::sdl::SdlInput::handle_event) there).
    pub fn pump_events(&mut self, mut handler: impl FnMut(&Event)) {
        for event in self.event_pump.poll_iter() {
            handler(&event);
        }
    }

    /// Puts `vsync` and `target_frame_rate` in force on this window (`video.vsync`,
    /// `video.frame_rate`).
    pub fn apply_pacing(&mut self, vsync: i32, target_frame_rate: i32) -> DisplayPacingState {
        pacing::apply(self, vsync, target_frame_rate)
    }

    /// Presents the frame, then waits out the rest of the frame period when a cap is in force.
    pub fn swap_and_pace(&mut self) {
        self.swap();
        self.pace();
    }

    /// Presents the frame (with vsync on, this is where the wait for the refresh happens). A
    /// [`WindowGraphics::Surface`] window has nothing to swap: its owner presents.
    pub fn swap(&mut self) {
        if self.gl_context.is_some() {
            self.window.gl_swap_window();
        }
        #[cfg(feature = "wgpu")]
        if let Some(display) = &mut self.display {
            display.present();
        }
    }

    /// The wgpu display of a [`WindowGraphics::Wgpu`] window.
    #[cfg(feature = "wgpu")]
    #[must_use]
    pub const fn wgpu(&self) -> Option<&crate::wgpu_display::WgpuDisplay> {
        self.display.as_ref()
    }

    /// The wgpu display of a [`WindowGraphics::Wgpu`] window, to draw a frame.
    #[cfg(feature = "wgpu")]
    pub const fn wgpu_mut(&mut self) -> Option<&mut crate::wgpu_display::WgpuDisplay> {
        self.display.as_mut()
    }

    /// The swap interval the pacing put in force: 0 (no vsync), 1, or 2 with a GL context.
    #[must_use]
    pub const fn vsync_interval(&self) -> i32 {
        self.swap_interval
    }

    /// The period of the frame cap in force, if any.
    #[must_use]
    pub const fn frame_cap(&self) -> Option<Duration> {
        self.frame_cap
    }

    /// Sleeps until the frame cap's next frame is due; returns at once without a cap.
    pub fn pace(&mut self) {
        let Some(period) = self.frame_cap else {
            return;
        };
        let now = Instant::now();
        if now < self.next_frame {
            std::thread::sleep(self.next_frame - now);
            self.next_frame += period;
        } else {
            // Behind schedule (a pause, a load): restart the cadence instead of catching up.
            self.next_frame = now + period;
        }
    }
}

impl DisplaySurface for HostWindow {
    fn set_vsync(&mut self, enabled: bool) {
        // Silk.NET's `IWindow.VSync` ignores a refusal too: the driver keeps its own interval
        // and the frame cap of the pacing rule paces the loop.
        let _ = self.try_set_swap_interval(i32::from(enabled));
    }

    fn set_frame_cap(&mut self, frames_per_second: u32) {
        self.frame_cap = (frames_per_second > 0)
            .then(|| Duration::from_secs_f64(1.0 / f64::from(frames_per_second)));
        self.next_frame = Instant::now();
    }

    fn try_set_swap_interval(&mut self, interval: i32) -> bool {
        let applied = if self.gl_context.is_some() {
            // SDL is called directly: the `sdl2` wrapper's `SwapInterval::from` panics on 2,
            // the every-other-refresh interval of `video.vsync` 2.
            // SAFETY: a plain SDL call on the window's GL context, current on this thread.
            unsafe { sdl2::sys::SDL_GL_SetSwapInterval(interval) == 0 }
        } else {
            // A surface presents with vsync or without (Q-066 falls back for 2).
            #[cfg(feature = "wgpu")]
            if interval <= 1 {
                if let Some(display) = &mut self.display {
                    display.set_vsync(interval > 0);
                }
            }
            interval <= 1
        };
        if applied {
            self.swap_interval = interval;
        }
        applied
    }
}

impl WindowIconTarget for HostWindow {
    /// SDL2 takes one icon, and the window manager scales it: the largest size is handed over.
    fn set_icon(&mut self, images: &[WindowIconImage]) -> Result<(), String> {
        let image = images.last().ok_or("no icon image")?;
        let mut pixels = image.pixels.clone();
        let surface = Surface::from_data(
            &mut pixels,
            image.width,
            image.height,
            image.width * 4,
            PixelFormatEnum::ABGR8888,
        )?;
        self.window.set_icon(surface);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gles_is_chosen_by_the_environment_or_on_aarch64_linux() {
        assert_eq!(GlFlavor::choose(None, false), GlFlavor::Desktop33Core);
        assert_eq!(GlFlavor::choose(Some("0"), false), GlFlavor::Desktop33Core);
        assert_eq!(GlFlavor::choose(Some("1"), false), GlFlavor::Gles30);
        assert_eq!(GlFlavor::choose(None, true), GlFlavor::Gles30);
        assert!(GlFlavor::Gles30.is_gles());
    }

    #[test]
    fn the_default_window_is_1280_by_720_with_vsync() {
        let settings = HostWindowSettings::default();
        assert_eq!(
            (settings.width, settings.height, settings.vsync),
            (1280, 720, 1)
        );
        assert_eq!(settings.title, "Zaxnes");
    }
}
