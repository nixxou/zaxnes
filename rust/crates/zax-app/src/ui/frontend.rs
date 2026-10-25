//! The interface as the application loop holds it: a Dear `ImGui` context with its renderer and
//! platform input, over the screens of [`ZaxUi`] and the developer panel.

use std::path::PathBuf;
use std::rc::Rc;

use imgui::DrawData;
use sdl2::event::Event;
use zax_platform::imgui_renderer::ImguiRenderer;
use zax_platform::input::{Key, KeyboardDevice};
use zax_platform::ui_image::PictureUpdate;

use super::commands::UiCommand;
use super::debug_overlay::{self, DebugInfo};
use super::fonts::{FontSizes, UiFonts};
use super::handheld::keyboard::KeyboardPad;
use super::handheld::layout::Metrics;
use super::handheld::states::StatesMode;
use super::handheld::{HandheldUi, Screen as HandheldScreen};
use super::menu_shortcuts::ShortcutModifiers;
use super::platform::SdlPlatform;
use super::shots::{ShotStep, UiShots};
use super::zax_ui::{UiEffect, UiFrame, ZaxUi};
use crate::app_options::AppOptions;
use crate::application::AppHost;
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

/// What the interface is drawn with: the GL renderer of this crate, or the window's wgpu
/// display, which draws the data [`ImguiFrontend::draw_data`] gives it.
pub enum UiGraphics<'a> {
    Gl(&'a Rc<glow::Context>),
    #[cfg(feature = "wgpu")]
    Wgpu(&'a mut zax_platform::wgpu_display::WgpuDisplay),
}

/// The console mode of the desktop: on, and whether it came on by itself (the gamepad's menu
/// button, a save state shortcut), in which case it goes when the game is back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ConsoleMode {
    active: bool,
    automatic: bool,
}

/// The body height the console mode of the desktop is sized for, whatever the window: between
/// the smallest handheld and a full HD screen.
fn console_height(drawable_height: f32) -> f32 {
    drawable_height.clamp(480.0, 1080.0)
}

/// What the screenshot plan asks after a frame is drawn.
#[derive(Debug, Default)]
pub struct ShotRequest {
    /// The window is to be captured into this file.
    pub capture: Option<PathBuf>,
    /// The plan is over: the program quits.
    pub finished: bool,
}

/// `ImGui` over the window.
pub struct ImguiFrontend {
    context: imgui::Context,
    /// The GL renderer; none when the window's wgpu display draws the interface.
    renderer: Option<ImguiRenderer>,
    platform: SdlPlatform,
    ui: ZaxUi,
    /// The gamepad-only interface, which takes the place of `ui` in the handheld mode; `ui` stays
    /// for the texts and the commands the two share. On the desktop it is the console mode (F1),
    /// and the gamepad's gestures in game (Rust-only, owner request).
    handheld: Option<HandheldUi>,
    /// Whether this is a handheld, where the gamepad interface is the only one.
    device: bool,
    /// The console mode of the desktop.
    console: ConsoleMode,
    shots: Option<UiShots>,
    /// The keyboard as a pad of the handheld interface.
    keyboard_pad: KeyboardPad,
    /// The changes of the interface's pictures not handed to the renderer yet.
    pictures: Vec<PictureUpdate>,
}

impl ImguiFrontend {
    /// Builds the context, loads the fonts and uploads the atlas. `drawable_height` sizes the
    /// fonts and the rows of the handheld interface.
    ///
    /// # Errors
    /// The renderer could not be built, or the folder of the screenshots could not be created.
    // The wgpu variant hands over a unique borrow; the GL one, the only one without the
    // feature, a shared one.
    #[cfg_attr(not(feature = "wgpu"), allow(clippy::needless_pass_by_value))]
    pub fn new(
        graphics: UiGraphics<'_>,
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
        // The console mode of the desktop has faces of its own, baked for the window it opens in
        // (a screenshot run photographs the desktop as it is, without it).
        let console = (!options.handheld && options.ui_shot_directory.is_none()).then(|| {
            let metrics = Metrics::for_height(console_height(height));
            let faces = UiFonts::load_console(
                context.fonts(),
                options.font_file.as_deref(),
                options.built_in_font,
                metrics.font_sizes(),
            );
            (metrics, faces)
        });
        let renderer = match graphics {
            UiGraphics::Gl(gl) => {
                Some(ImguiRenderer::new(gl, context.fonts()).map_err(|error| error.to_string())?)
            }
            #[cfg(feature = "wgpu")]
            UiGraphics::Wgpu(display) => {
                display.upload_fonts(context.fonts());
                None
            }
        };
        // REQ-UI-207: the renderer has just built the font atlas, so the glyph tables exist; this
        // is the earliest moment a missing code point can be pointed at a glyph that is in it.
        fonts.remap_missing_glyphs(context.fonts());
        let mut ui = ZaxUi::new(host);
        ui.set_heading_font(fonts.heading);
        let handheld = if options.handheld {
            let directory = roms_directory(host, options);
            if let Err(error) = std::fs::create_dir_all(&directory) {
                ui.log(format!(
                    "the ROM folder '{}' could not be created: {error}",
                    directory.display()
                ));
            }
            let mut handheld = HandheldUi::new(host, directory, height);
            handheld.set_heading_font(fonts.heading);
            Some(handheld)
        } else {
            console.map(|(metrics, (body, heading))| {
                let directory = roms_directory(host, options);
                let mut handheld = HandheldUi::new(host, directory, console_height(height));
                handheld.set_heading_font(heading);
                handheld.use_on_desktop(metrics, Some(body));
                handheld
            })
        };
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
            device: options.handheld,
            console: ConsoleMode {
                active: options.console,
                automatic: false,
            },
            shots,
            keyboard_pad: KeyboardPad::new(),
            pictures: Vec::new(),
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

    /// Whether the gamepad interface is the one shown: always on a handheld, in the console
    /// mode on the desktop.
    #[must_use]
    pub const fn console_shown(&self) -> bool {
        self.handheld.is_some() && (self.device || self.console.active)
    }

    /// Whether the interface holds a modal window (REQ-UI-004).
    #[must_use]
    pub fn is_modal(&self) -> bool {
        match &self.handheld {
            Some(handheld) if self.console_shown() => handheld.is_modal(),
            _ => self.ui.state.is_modal(),
        }
    }

    /// Height of the menu bar, which the game view sits under (REQ-UI-200).
    #[must_use]
    pub const fn chrome_height(&self) -> f32 {
        if self.console_shown() {
            0.0
        } else {
            self.ui.chrome_height
        }
    }

    /// Switches the desktop to its console mode, or back to the menu bar. On the way back, a
    /// menu left open over the game is closed and the game resumes.
    fn switch_console(&mut self, host: &mut AppHost, active: bool, automatic: bool) {
        let Some(handheld) = &mut self.handheld else {
            return;
        };
        if self.device {
            return;
        }
        if !active && self.console.active {
            handheld.leave_for_desktop(host);
            self.keyboard_pad.release(self.platform.pad_mut());
        }
        self.console = ConsoleMode { active, automatic };
    }

    /// Carries out what a shortcut, a binding or a menu entry asked of the console mode.
    fn follow_console_request(&mut self, host: &mut AppHost) {
        let Some(request) = self.ui.take_console_request() else {
            return;
        };
        match request {
            UiCommand::ToggleConsole => {
                let active = !self.console.active;
                self.switch_console(host, active, false);
            }
            UiCommand::SaveStates | UiCommand::LoadStates => {
                let mode = if request == UiCommand::SaveStates {
                    StatesMode::Save
                } else {
                    StatesMode::Load
                };
                let opened = self
                    .handheld
                    .as_mut()
                    .is_some_and(|handheld| handheld.open_states(mode, host));
                if opened && !self.console.active {
                    self.switch_console(host, true, true);
                }
            }
            UiCommand::ToggleDebugOverlay | UiCommand::CloseWindows => {}
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
        // In the console mode of the desktop, F1 and Home still switch back; the other keys
        // drive the pad, as on a handheld.
        if self.console_shown() && !self.device {
            let down = |key: Key| keyboard.is_down(key as i32);
            let pressed = self.ui.rising_keys(down);
            let plain = !(down(Key::ControlLeft)
                || down(Key::ControlRight)
                || down(Key::ShiftLeft)
                || down(Key::ShiftRight)
                || down(Key::AltLeft)
                || down(Key::AltRight));
            if !binding_capture_active
                && plain
                && pressed.iter().any(|key| matches!(key, Key::F1 | Key::Home))
            {
                self.switch_console(host, false, false);
                return;
            }
        }
        // A handheld has no menu bar: the keys drive its pad instead of the shortcuts.
        if self.console_shown() {
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
        self.follow_console_request(host);
    }

    /// The keys that go down this frame as an application shortcut, which the game must not see.
    #[must_use]
    pub fn consumed_keys(&self, keyboard: &dyn KeyboardDevice) -> Vec<i32> {
        if self.console_shown() || self.wants_keyboard() {
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

    /// The step of the screenshot plan after a drawn frame, if there is a plan: the file to
    /// capture the window into, and whether the plan is over (the caller then quits).
    pub fn advance_shots(&mut self) -> ShotRequest {
        let Some(shots) = &mut self.shots else {
            return ShotRequest::default();
        };
        let capture = match shots.advance(&mut self.ui, self.handheld.as_mut()) {
            ShotStep::Capture(path) => Some(path),
            ShotStep::Wait => None,
        };
        ShotRequest {
            capture,
            finished: shots.finished(),
        }
    }

    /// Whether a screenshot plan drives the interface.
    #[must_use]
    pub const fn has_shots(&self) -> bool {
        self.shots.is_some()
    }

    /// Builds the interface of the frame, which [`ImguiFrontend::render`] then draws: a
    /// pipelined frame builds it before the host is lent to the conversion thread.
    pub fn build(&mut self, frame: &mut UiFrame<'_>, sizes: Sizes, debug: &DebugInfo<'_>) {
        self.platform.prepare_frame(
            self.context.io_mut(),
            sizes.window,
            sizes.drawable,
            frame.real_seconds,
        );
        // A menu entry clicked last frame.
        self.follow_console_request(frame.host);
        let shown = self.console_shown();
        let device = self.device;
        let desktop_modal = self.ui.state.is_modal();
        let ui = self.context.new_frame();
        let mut open_console = false;
        if let Some(handheld) = &mut self.handheld {
            // A screenshot run has no gamepad, and its screens are staged by the plan.
            if self.shots.is_none() {
                self.platform.pad_mut().advance(frame.real_seconds);
            }
            let playing = frame.host.current_game_id().is_some() || frame.host.is_loading();
            if shown {
                if self.shots.is_none() {
                    handheld.update(self.platform.pad(), frame.real_seconds, frame);
                }
                handheld.draw(ui, frame);
                self.pictures.extend(handheld.take_picture_updates());
                self.ui.draw_fps_counter(ui, frame);
            } else if !device && playing && !desktop_modal && self.shots.is_none() {
                // The gamepad plays as in the console mode (owner request): the menu button
                // opens its menu, X, Y and L2 act in game.
                handheld.update(self.platform.pad(), frame.real_seconds, frame);
                open_console = handheld.screen() != HandheldScreen::InGame;
            }
        }
        if !shown {
            self.ui.draw(ui, frame);
            if self.ui.state.debug_overlay_visible() {
                debug_overlay::draw(ui, frame.host, self.ui.chrome_height, debug);
            }
        }
        if open_console {
            self.console = ConsoleMode {
                active: true,
                automatic: true,
            };
        }
        // A console mode that came by itself goes when the game is back.
        let back_in_game = self
            .handheld
            .as_ref()
            .is_some_and(|handheld| handheld.screen() == HandheldScreen::InGame);
        if shown && !device && self.console.automatic && back_in_game {
            self.console = ConsoleMode::default();
        }
        self.platform.pad_mut().end_frame();
    }

    /// Draws the interface [`ImguiFrontend::build`] built, with the GL renderer (a wgpu window
    /// draws [`ImguiFrontend::draw_data`] itself).
    pub fn render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.update_pictures(&std::mem::take(&mut self.pictures));
            renderer.render(self.context.render());
        }
    }

    /// The changes of the interface's pictures since the last call, for a renderer that is not
    /// the front end's own (wgpu).
    pub fn take_picture_updates(&mut self) -> Vec<PictureUpdate> {
        std::mem::take(&mut self.pictures)
    }

    /// The draw data of the interface [`ImguiFrontend::build`] built, for a wgpu window.
    pub fn draw_data(&mut self) -> &DrawData {
        self.context.render()
    }

    /// Takes one event of the host.
    pub fn handle_host_event(&mut self, host: &mut AppHost, event: HostEvent) {
        let shown = self.console_shown();
        match (&mut self.handheld, event) {
            (Some(_), HostEvent::LanguageChanged(code)) if shown => self.ui.language_changed(&code),
            (Some(handheld), event) if shown => handheld.handle_host_event(host, event),
            (_, event) => self.ui.handle_host_event(host, event),
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
