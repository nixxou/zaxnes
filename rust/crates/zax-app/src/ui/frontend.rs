//! The interface as the application loop holds it: a Dear `ImGui` context with its renderer and
//! platform input, over the screens of [`ZaxUi`] and the developer panel.

use std::path::PathBuf;
use std::rc::Rc;

use sdl2::event::Event;
use zax_platform::imgui_renderer::ImguiRenderer;
use zax_platform::input::{Key, KeyboardDevice};

use super::debug_overlay::{self, DebugInfo};
use super::fonts::{FontSizes, UiFonts};
use super::handheld::keyboard::KeyboardPad;
use super::handheld::layout::Metrics;
use super::handheld::HandheldUi;
use super::menu_shortcuts::ShortcutModifiers;
use super::platform::SdlPlatform;
use super::shots::{ShotStep, UiShots};
use super::zax_ui::{UiEffect, UiFrame, ZaxUi};
use crate::app_options::AppOptions;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::host::HostEvent;
use crate::input::{DesktopInputSource, InputBindings};

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
    /// The gamepad-only interface, which takes the place of `ui` in the handheld mode; `ui` stays
    /// for the texts and the commands the two share.
    handheld: Option<HandheldUi>,
    shots: Option<UiShots>,
    /// The keyboard as a pad of the handheld interface.
    keyboard_pad: KeyboardPad,
}

impl ImguiFrontend {
    /// Builds the context, loads the fonts and uploads the atlas. `drawable_height` sizes the
    /// fonts and the rows of the handheld interface.
    ///
    /// # Errors
    /// The renderer could not be built, or the folder of the screenshots could not be created.
    pub fn new(
        gl: &Rc<glow::Context>,
        host: &AppHost,
        options: &AppOptions,
        drawable_height: u32,
    ) -> Result<Self, String> {
        #[allow(clippy::cast_precision_loss)] // display heights are far below 2^24
        let height = drawable_height as f32;
        let metrics = Metrics::for_height(height);
        let mut context = imgui::Context::create();
        // Dear ImGui would otherwise drop an imgui.ini next to the working directory.
        context.set_ini_filename(None);
        context.set_log_filename(None);
        let fonts = UiFonts::load_sized(
            context.fonts(),
            options.font_file.as_deref(),
            options.built_in_font,
            if options.handheld {
                metrics.font_sizes()
            } else {
                FontSizes::DESKTOP
            },
        );
        let renderer =
            ImguiRenderer::new(gl, context.fonts()).map_err(|error| error.to_string())?;
        // REQ-UI-207: the renderer has just built the font atlas, so the glyph tables exist; this
        // is the earliest moment a missing code point can be pointed at a glyph that is in it.
        fonts.remap_missing_glyphs(context.fonts());
        let mut ui = ZaxUi::new(host);
        ui.set_heading_font(fonts.heading);
        let handheld = options.handheld.then(|| {
            let directory = roms_directory(host, options);
            if let Err(error) = std::fs::create_dir_all(&directory) {
                ui.log(format!(
                    "the ROM folder '{}' could not be created: {error}",
                    directory.display()
                ));
            }
            let mut handheld = HandheldUi::new(host, directory, height);
            handheld.set_heading_font(fonts.heading);
            handheld
        });
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
            .map(|directory| UiShots::new(directory, options.handheld))
            .transpose()
            .map_err(|error| format!("the screenshot folder could not be created: {error}"))?;
        let platform = if options.handheld {
            SdlPlatform::for_handheld()
        } else if shots.is_some() {
            SdlPlatform::for_capture()
        } else {
            SdlPlatform::new()
        };
        platform.configure(context.io_mut());
        Ok(Self {
            context,
            renderer,
            platform,
            ui,
            handheld,
            shots,
            keyboard_pad: KeyboardPad::new(),
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
    pub fn is_modal(&self) -> bool {
        self.handheld
            .as_ref()
            .map_or_else(|| self.ui.state.is_modal(), HandheldUi::is_modal)
    }

    /// Height of the menu bar, which the game view sits under (REQ-UI-200).
    #[must_use]
    pub const fn chrome_height(&self) -> f32 {
        if self.handheld.is_some() {
            0.0
        } else {
            self.ui.chrome_height
        }
    }

    /// The menu shortcuts of REQ-UI-204: the rising edge of every watched key.
    pub fn handle_shortcuts(
        &mut self,
        host: &mut AppHost,
        keyboard: &dyn KeyboardDevice,
        binding_capture_active: bool,
        bindings: &InputBindings,
    ) {
        // A handheld has no menu bar: the keys drive its pad instead of the shortcuts.
        if self.handheld.is_some() {
            if binding_capture_active {
                self.keyboard_pad.release(self.platform.pad_mut());
            } else {
                self.keyboard_pad
                    .update(self.platform.pad_mut(), bindings, keyboard);
            }
            return;
        }
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

    /// The keys that go down this frame as an application shortcut, which the game must not see.
    #[must_use]
    pub fn consumed_keys(&self, keyboard: &dyn KeyboardDevice) -> Vec<i32> {
        if self.handheld.is_some() || self.wants_keyboard() {
            return Vec::new();
        }
        self.ui
            .consumed_keys(|key: Key| keyboard.is_down(key as i32))
            .into_iter()
            .map(|key| key as i32)
            .collect()
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
        if let Some(handheld) = &mut self.handheld {
            // A screenshot run has no gamepad, and its screens are staged by the plan.
            if self.shots.is_none() {
                self.platform.pad_mut().advance(frame.real_seconds);
                handheld.update(self.platform.pad(), frame.real_seconds, frame);
            }
            handheld.draw(ui, frame);
            self.ui.draw_fps_counter(ui, frame);
        } else {
            self.ui.draw(ui, frame);
            if self.ui.state.debug_overlay_visible() {
                debug_overlay::draw(ui, frame.host, self.ui.chrome_height, debug);
            }
        }
        self.platform.pad_mut().end_frame();
        self.renderer.render(self.context.render());
        if let Some(shots) = &mut self.shots {
            if let ShotStep::Capture(path) = shots.advance(&mut self.ui, self.handheld.as_mut()) {
                capture(&path);
            }
            if shots.finished() {
                frame.host.enqueue(Command::Quit, CommandArgs::default());
            }
        }
    }

    /// Takes one event of the host.
    pub fn handle_host_event(&mut self, host: &mut AppHost, event: HostEvent) {
        match (&mut self.handheld, event) {
            (Some(_), HostEvent::LanguageChanged(code)) => self.ui.language_changed(&code),
            (Some(handheld), event) => handheld.handle_host_event(host, event),
            (None, event) => self.ui.handle_host_event(host, event),
        }
    }

    /// Stops a binding capture in progress (REQ-UI-067).
    pub fn close_capture(input: &mut DesktopInputSource) {
        ZaxUi::close_capture(input);
    }

    /// What the interface asked of the application since the last call.
    pub fn take_effects(&mut self) -> Vec<UiEffect> {
        let mut effects = self.ui.take_effects();
        if let Some(handheld) = &mut self.handheld {
            effects.extend(handheld.take_effects());
        }
        effects
    }
}

/// The folder the handheld ROM list reads: `--roms`, or the `roms` folder of the user data.
fn roms_directory(host: &AppHost, options: &AppOptions) -> PathBuf {
    let directory = options
        .roms_directory
        .as_deref()
        .map_or_else(|| host.paths().root.join("roms"), PathBuf::from);
    std::path::absolute(&directory).unwrap_or(directory)
}
