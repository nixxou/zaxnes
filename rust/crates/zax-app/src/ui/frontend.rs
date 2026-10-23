//! The interface as the application loop holds it: a Dear `ImGui` context with its renderer and
//! platform input, over the screens of [`ZaxUi`] and the developer panel.

use std::rc::Rc;

use sdl2::event::Event;
use zax_platform::imgui_renderer::ImguiRenderer;
use zax_platform::input::{Key, KeyboardDevice};

use super::debug_overlay::{self, DebugInfo};
use super::fonts::UiFonts;
use super::menu_shortcuts::ShortcutModifiers;
use super::platform::SdlPlatform;
use super::shots::{ShotStep, UiShots};
use super::zax_ui::{UiEffect, UiFrame, ZaxUi};
use crate::app_options::AppOptions;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::host::HostEvent;
use crate::input::DesktopInputSource;

/// The window sizes of one frame: the window's own and the drawable's.
#[derive(Debug, Clone, Copy)]
pub struct Sizes {
    /// The window, in screen coordinates.
    pub window: (u32, u32),
    /// The drawable, in pixels.
    pub drawable: (u32, u32),
}

/// `ImGui` over the window.
pub struct ImguiFrontend {
    context: imgui::Context,
    renderer: ImguiRenderer,
    platform: SdlPlatform,
    ui: ZaxUi,
    shots: Option<UiShots>,
}

impl ImguiFrontend {
    /// Builds the context, loads the fonts and uploads the atlas.
    ///
    /// # Errors
    /// The renderer could not be built, or the folder of the screenshots could not be created.
    pub fn new(
        gl: &Rc<glow::Context>,
        host: &AppHost,
        options: &AppOptions,
    ) -> Result<Self, String> {
        let mut context = imgui::Context::create();
        // Dear ImGui would otherwise drop an imgui.ini next to the working directory.
        context.set_ini_filename(None);
        context.set_log_filename(None);
        let fonts = UiFonts::load(
            context.fonts(),
            options.font_file.as_deref(),
            options.built_in_font,
        );
        let renderer =
            ImguiRenderer::new(gl, context.fonts()).map_err(|error| error.to_string())?;
        // REQ-UI-207: the renderer has just built the font atlas, so the glyph tables exist; this
        // is the earliest moment a missing code point can be pointed at a glyph that is in it.
        fonts.remap_missing_glyphs(context.fonts());
        let mut ui = ZaxUi::new(host);
        ui.set_heading_font(fonts.heading);
        #[cfg(feature = "native-dialog")]
        if options.ui_shot_directory.is_none() {
            ui.set_file_picker(Box::new(super::native_picker::NativePicker::detect()));
        }
        ui.log(match &fonts.body_file {
            Some(file) => format!("interface font: {}", file.display()),
            None => "no system font was loaded; the built-in ImGui font has no accents.".to_owned(),
        });
        let shots = options
            .ui_shot_directory
            .as_deref()
            .map(UiShots::new)
            .transpose()
            .map_err(|error| format!("the screenshot folder could not be created: {error}"))?;
        Ok(Self {
            context,
            renderer,
            platform: if shots.is_some() {
                SdlPlatform::for_capture()
            } else {
                SdlPlatform::new()
            },
            ui,
            shots,
        })
    }

    /// An SDL event, before the input layer sees it.
    pub fn handle_event(&mut self, event: &Event) {
        self.platform.handle_event(self.context.io_mut(), event);
    }

    /// Whether a text field of the interface owns the keyboard.
    #[must_use]
    pub fn wants_keyboard(&self) -> bool {
        self.context.io().want_capture_keyboard
    }

    /// Whether the pointer is over a piece of the interface.
    #[must_use]
    pub fn wants_pointer(&self) -> bool {
        self.context.io().want_capture_mouse
    }

    /// Whether the interface holds a modal window (REQ-UI-004).
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.ui.state.is_modal()
    }

    /// Height of the menu bar, which the game view sits under (REQ-UI-200).
    #[must_use]
    pub const fn chrome_height(&self) -> f32 {
        self.ui.chrome_height
    }

    /// The menu shortcuts of REQ-UI-204: the rising edge of every watched key.
    pub fn handle_shortcuts(
        &mut self,
        host: &mut AppHost,
        keyboard: &dyn KeyboardDevice,
        binding_capture_active: bool,
    ) {
        // While a binding capture is listening, the keyboard belongs to the capture: the fixed
        // shortcuts must not fire either, for the same reason REQ-INP-047 and DEC-08-2 suspend
        // the actions. A text field of the interface (the two search boxes) owns the keyboard
        // while it has the focus, so a shortcut must not fire from what is being typed.
        if binding_capture_active || self.wants_keyboard() {
            self.ui.release_held_keys();
            return;
        }
        let down = |key: Key| keyboard.is_down(key as i32);
        let pressed = self.ui.rising_keys(down);
        let modifiers = ShortcutModifiers {
            control: down(Key::ControlLeft) || down(Key::ControlRight),
            shift: down(Key::ShiftLeft) || down(Key::ShiftRight),
            alt: down(Key::AltLeft) || down(Key::AltRight),
        };
        self.ui.handle_shortcuts(host, &pressed, modifiers);
    }

    /// The commands a binding triggered that belong to the interface.
    pub fn run_bound_ui_commands(&mut self, input: &DesktopInputSource) {
        for name in input.triggered_commands() {
            self.ui.run_named_ui_command(name);
        }
    }

    /// Draws the interface over the game view, then the screenshot plan if there is one.
    pub fn draw(
        &mut self,
        frame: &mut UiFrame<'_>,
        sizes: Sizes,
        debug: &DebugInfo<'_>,
        capture: impl FnOnce(&std::path::Path),
    ) {
        self.platform.prepare_frame(
            self.context.io_mut(),
            sizes.window,
            sizes.drawable,
            frame.real_seconds,
        );
        let ui = self.context.new_frame();
        self.ui.draw(ui, frame);
        if self.ui.state.debug_overlay_visible() {
            debug_overlay::draw(ui, frame.host, self.ui.chrome_height, debug);
        }
        self.renderer.render(self.context.render());
        if let Some(shots) = &mut self.shots {
            if let ShotStep::Capture(path) = shots.advance(&mut self.ui) {
                capture(&path);
            }
            if shots.finished() {
                frame.host.enqueue(Command::Quit, CommandArgs::default());
            }
        }
    }

    /// Takes one event of the host.
    pub fn handle_host_event(&mut self, host: &mut AppHost, event: HostEvent) {
        self.ui.handle_host_event(host, event);
    }

    /// Stops a binding capture in progress (REQ-UI-067).
    pub fn close_capture(input: &mut DesktopInputSource) {
        ZaxUi::close_capture(input);
    }

    /// What the interface asked of the application since the last call.
    pub fn take_effects(&mut self) -> Vec<UiEffect> {
        self.ui.take_effects()
    }
}
