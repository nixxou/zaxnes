//! The application: the window, the loop that samples the input, runs the host, draws the
//! scene and pumps the audio (SPEC-07, SPEC-06 §3.13). The interface plugs in through
//! [`ApplicationUi`].

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use sdl2::event::{Event, WindowEvent};
use time::OffsetDateTime;
use zax_platform::audio::PREFERRED_SAMPLE_RATE;
use zax_platform::framebuffer::{self, OffscreenFramebuffer};
use zax_platform::icon;
use zax_platform::input::sdl::SdlInput;
use zax_platform::{AudioOutput, HostWindow, HostWindowSettings};
use zax_profiles::format;
use zax_render::assets::{NeutralSceneAssets, SceneAssets};
use zax_render::controls;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::target::RenderTarget;
use zax_render::SceneRenderer;
use zax_settings::model::SettingsModel;

use crate::app_options::AppOptions;
use crate::audio::audio_assets::AudioAssets;
use crate::audio::synth_audio_library::SynthAudioLibrary;
use crate::audio::{AudioMixer, TrackRandom, UiAudioLibrary};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::host::{
    EmulationHost, EmulationHostOptions, HostEvent, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH,
};
use crate::emulation::video_pacing::{VideoPacing, VideoPacingRequest};
use crate::frame_rate_counter::FrameRateCounter;
use crate::game_session::GameSession;
use crate::input::cursor_auto_hide::CursorAutoHide;
use crate::input::input_action::to_camera_actions;
use crate::input::{routing, DesktopInputSource, DeviceRefs, InputSnapshot};
use crate::library_game_catalog::LibraryGameCatalog;
use crate::persistence::app_settings::AppSettings;
use crate::persistence::command_line_settings;
use crate::persistence::input_binding_store::FileInputBindingStore;
use crate::persistence::settings_store::SettingsStore;
use crate::render_scaling;
use crate::render_skip;
use crate::scene_script_bridge::{self, ConsoleScriptLog};
use crate::user_data_paths::UserDataPaths;
use crate::{neutral_assets, persistence::input_binding_store::InputBindingStore as _};

/// The host the application runs.
pub type AppHost = EmulationHost<LibraryGameCatalog>;

/// What the interface layer provides to the application. Every member has a do-nothing default,
/// which is the headless application.
pub trait ApplicationUi {
    /// An SDL event, before the input layer sees it.
    fn handle_event(&mut self, _event: &Event) {}

    /// Whether a text field of the interface owns the keyboard.
    fn wants_keyboard(&self) -> bool {
        false
    }

    /// Whether the pointer is over a piece of the interface.
    fn wants_pointer(&self) -> bool {
        false
    }

    /// Whether the interface holds a modal window (REQ-UI-004).
    fn is_modal(&self) -> bool {
        false
    }

    /// Height of the menu bar, which the game view sits under (REQ-UI-200).
    fn chrome_height(&self) -> f32 {
        0.0
    }

    /// Draws the interface over the game view.
    fn draw(&mut self, _host: &mut AppHost, _real_seconds: f32) {}

    /// A command asked the interface for a window or a dialogue.
    fn handle_ui_request(&mut self, _host: &mut AppHost, _command: Command, _args: &CommandArgs) {}

    /// The language of the interface changed.
    fn language_changed(&mut self, _code: &str) {}

    /// The window is closing: a binding capture in progress stops.
    fn close_capture(&mut self) {}
}

/// An interface that draws nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoUi;

impl ApplicationUi for NoUi {}

/// Whether the window opens in full screen: the persisted choice, except for smoke runs and
/// screenshot runs.
#[must_use]
pub const fn should_open_fullscreen(options: &AppOptions, persisted: bool) -> bool {
    persisted && !options.smoke_test && options.ui_shot_directory.is_none()
}

/// What the window's pacing should be: the settings, except that a smoke run is not paced by
/// the display's refresh rate.
#[must_use]
pub fn pacing_request(options: &AppOptions, model: &SettingsModel) -> VideoPacingRequest {
    if options.smoke_test {
        VideoPacingRequest {
            vsync: options.vsync.unwrap_or(0),
            target_frame_rate: 0,
        }
    } else {
        VideoPacingRequest {
            vsync: model.vsync(),
            target_frame_rate: model.frame_rate(),
        }
    }
}

/// The library entry the command line asks to start: a profile file that is in the library, an
/// identifier or title, a unique title prefix; "2D Mode" otherwise.
pub fn resolve_startup_game(
    options: &AppOptions,
    catalog: &LibraryGameCatalog,
    paths: &UserDataPaths,
    log: &dyn Fn(&str),
) -> Option<String> {
    let library = catalog.library();
    let mut request = options
        .profile
        .as_deref()
        .filter(|request| !request.trim().is_empty());
    if let Some(file) = request.filter(|request| Path::new(request).is_file()) {
        let id = std::fs::read(file)
            .ok()
            .and_then(|bytes| format::load(&bytes, None, Some(file)).ok())
            .and_then(|document| document.profile.id);
        if let Some(matched) = id.as_deref().and_then(|id| library.find(id)) {
            return Some(matched.id().to_owned());
        }
        log(&format!(
            "the profile '{file}' is not in the library; copy it into '{}' to play it. \
             Falling back to 2D Mode.",
            paths.profiles_directory.display()
        ));
        request = None;
    }
    if let Some(request) = request {
        let wanted = request.to_lowercase();
        if let Some(entry) = library.entries().iter().find(|entry| {
            entry.id().to_lowercase() == wanted || entry.display_title().to_lowercase() == wanted
        }) {
            return Some(entry.id().to_owned());
        }
        let mut matches = library
            .entries()
            .iter()
            .filter(|entry| entry.display_title().to_lowercase().starts_with(&wanted));
        match (matches.next(), matches.next()) {
            (Some(_), Some(_)) => {
                log(&format!(
                    "'{request}' matches several games; use the exact title."
                ));
                return catalog.two_dimensional_mode_id();
            }
            (Some(unique), None) => return Some(unique.id().to_owned()),
            _ => log(&format!(
                "no game matches '{request}'. Use --list-profiles to see the titles."
            )),
        }
    }
    catalog.two_dimensional_mode_id()
}

fn log(message: &str) {
    println!("Zaxnes: {message}");
}

/// The windowed application.
pub struct ZaxApplication {
    options: AppOptions,
}

impl ZaxApplication {
    /// Creates the application for one run.
    #[must_use]
    pub const fn new(options: AppOptions) -> Self {
        Self { options }
    }

    /// Runs the application to the end and returns the exit code (0 for success).
    #[must_use]
    pub fn run(self, ui: Box<dyn ApplicationUi>) -> i32 {
        match self.run_inner(ui) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("{message}");
                1
            }
        }
    }

    fn run_inner(&self, ui: Box<dyn ApplicationUi>) -> Result<i32, String> {
        let mut running = Running::build(&self.options, ui)?;
        while running.frame()? {}
        Ok(running.finish())
    }
}

/// Everything a run owns once the window is open.
struct Running {
    options: AppOptions,
    ui: Box<dyn ApplicationUi>,
    window: HostWindow,
    gl: Rc<glow::Context>,
    offscreen: Rc<RefCell<OffscreenFramebuffer>>,
    renderer: SceneRenderer,
    input: SdlInput,
    source: DesktopInputSource,
    audio: Option<AudioOutput>,
    host: AppHost,
    pacing: VideoPacing,
    modal_flag: Rc<Cell<bool>>,
    counter: FrameRateCounter,
    cursor: CursorAutoHide,
    state: RunState,
    last: Instant,
}

impl Running {
    fn build(options: &AppOptions, ui: Box<dyn ApplicationUi>) -> Result<Self, String> {
        let asset_root = neutral_assets::locate(options.asset_root.as_deref().map(Path::new));
        let paths =
            UserDataPaths::resolve(options.user_data_root.as_deref().map(Path::new), None, None);
        // AMD-029 §2: the first run after the rename seeds the new root from the old one.
        if let Some(legacy) = &paths.migrated_from {
            log(&format!(
                "user data copied from '{}' to '{}'; the old folder was left untouched (AMD-029).",
                legacy.display(),
                paths.root.display()
            ));
        }
        let (store, model) = load_settings(options, &paths);
        let catalog = LibraryGameCatalog::open(asset_root.as_deref(), &paths, Box::new(log));
        if let Some(rom) = options
            .rom_path
            .as_deref()
            .filter(|rom| !Path::new(rom).is_file())
        {
            return Err(format!("Zaxnes: the ROM '{rom}' does not exist."));
        }
        let startup_game = resolve_startup_game(options, &catalog, &paths, &log);
        let mut scene = SceneState::default();
        publish_settings_to_scene(&model, &mut scene);

        let request = pacing_request(options, &model);
        let mut window = HostWindow::create(&HostWindowSettings {
            title: "Zaxnes".to_owned(),
            width: options.width,
            height: options.height,
            vsync: request.vsync,
            target_frame_rate: request.target_frame_rate,
            fullscreen: should_open_fullscreen(options, model.fullscreen()),
        })
        .map_err(|error| error.to_string())?;
        if options.smoke_test {
            println!("{}", smoke_banner(options));
        }
        log_window_icon(&mut window);
        let gl = window.glow_context();
        let (renderer_name, version) = framebuffer::gl_strings(&gl);
        let request_options = options.clone();
        let mut pacing = VideoPacing::new(
            Some(Box::new(move |model| {
                pacing_request(&request_options, model)
            })),
            Some(Box::new(log)),
        );
        pacing.apply(&mut window, &model);
        let (width, height) = window.drawable_size();
        // The host reads the buffer back for the thumbnails whenever the game pauses, so the
        // loop and the host's capture closure share it; the borrows are momentary.
        let offscreen = Rc::new(RefCell::new(
            OffscreenFramebuffer::new(Rc::clone(&gl), to_i32(width), to_i32(height))
                .map_err(|error| error.to_string())?,
        ));
        let renderer = SceneRenderer::new(
            Rc::clone(&gl),
            NeutralSceneAssets::try_open(asset_root.as_deref())
                .map(|assets| Box::new(assets) as Box<dyn SceneAssets>),
        )
        .map_err(|error| error.to_string())?;
        let input = SdlInput::new(window.sdl()).map_err(|error| error.to_string())?;
        let mut bindings_store = FileInputBindingStore::new(paths.clone(), OffsetDateTime::now_utc);
        let mut bindings = crate::input::InputBindings::new();
        bindings_store.load(&mut bindings);
        let source = DesktopInputSource::new(bindings, Box::new(bindings_store));

        let (audio, sample_rate) = open_audio(options, window.sdl());
        let modal_flag = Rc::new(Cell::new(false));
        let parts = HostParts {
            paths: &paths,
            model,
            scene,
            catalog,
            store,
            asset_root: asset_root.as_deref(),
            sample_rate,
            audio_open: audio.is_some(),
            offscreen: &offscreen,
            modal_flag: &modal_flag,
        };
        let mut host = build_host(parts);
        report_startup_problems(&mut host, &paths);
        start_game_from_command_line(&mut host, options, startup_game.as_deref());
        Ok(Self {
            options: options.clone(),
            ui,
            window,
            gl,
            offscreen,
            renderer,
            input,
            source,
            audio,
            host,
            pacing,
            modal_flag,
            counter: FrameRateCounter::new(),
            cursor: CursorAutoHide::new(),
            state: RunState::new(renderer_name, version),
            last: Instant::now(),
        })
    }

    /// One displayed frame. Returns false once the application is to close.
    fn frame(&mut self) -> Result<bool, String> {
        if self.pump_events() {
            return Ok(false);
        }
        let now = Instant::now();
        let delta = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.counter.add(delta);
        self.state.rendered_frames += 1;
        let real = delta.clamp(0.0, 0.25);
        #[allow(clippy::cast_possible_truncation)] // the C# `(float)` of a clamped delta
        let real_f = real as f32;
        self.modal_flag.set(self.ui.is_modal());
        let (width, height) = self.window.drawable_size();
        let (width, height) = (to_i32(width), to_i32(height));
        let snapshot = self.sample_input(real_f, width, height);
        handle_camera_input(&mut self.host, &snapshot, real_f, &mut self.state);
        update_cursor(
            &mut self.cursor,
            &self.host,
            &mut self.input,
            &snapshot,
            real_f,
        );
        let emulated = self.host.tick(real, &snapshot);
        self.source.end_sample();
        update_scene(&mut self.host, &mut self.state);
        {
            let scene = self.host.scene_mut();
            scene.camera.advance(real_f);
            scene.real_time += real_f;
        }
        forward_events(
            &mut self.host,
            &mut self.ui,
            &mut self.window,
            &mut self.pacing,
        );
        self.draw(emulated, width, height, real_f)?;
        pump_audio(&mut self.host, self.audio.as_mut(), real);
        if self.host.quit_requested()
            || (self.options.smoke_test
                && smoke_run_is_over(&self.options, &self.host, &self.state))
        {
            if let Some(shot) = self.options.smoke_shot_path.as_deref() {
                capture_window(&self.gl, width, height, Path::new(shot));
            }
            return Ok(false);
        }
        self.window.swap_and_pace();
        Ok(true)
    }

    /// Feeds the window's events to the interface and the input layer. Returns whether the
    /// window is closing.
    fn pump_events(&mut self) -> bool {
        let mut closing = false;
        let (ui, input, host) = (&mut self.ui, &mut self.input, &mut self.host);
        self.window.pump_events(|event| {
            ui.handle_event(event);
            input.handle_event(event);
            match event {
                Event::Quit { .. } => closing = true,
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => host.suspend(),
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => host.resume_from_suspend(),
                _ => {}
            }
        });
        closing
    }

    fn sample_input(&mut self, real: f32, width: i32, height: i32) -> InputSnapshot {
        self.source.keyboard_captured_by_ui = self.ui.wants_keyboard();
        self.source.pointer_over_ui = self.ui.wants_pointer();
        #[allow(clippy::cast_precision_loss)] // window sizes are far below 2^24
        {
            self.source.render_target_size = [width as f32, height as f32];
        }
        let snapshot = self.source.sample(
            real,
            DeviceRefs {
                keyboard: Some(&self.input.keyboard),
                pointer: Some(&mut self.input.pointer),
                hub: &mut self.input.gamepads,
            },
        );
        for command in self.source.triggered_commands() {
            if let Some(command) = Command::from_name(command) {
                self.host.enqueue(command, CommandArgs::default());
            }
        }
        snapshot
    }

    /// Draws the scene into the offscreen buffer (unless the render-skip rule keeps the
    /// previous image), blits it under the interface and draws the interface.
    fn draw(&mut self, emulated: u32, width: i32, height: i32, real: f32) -> Result<(), String> {
        let view_height = (height - to_i32_f(self.ui.chrome_height())).max(1);
        let (scene_width, scene_height) = render_scaling::scene_size(
            width,
            view_height,
            self.host.settings().render_method(),
            self.host.settings().res_scale_factor(),
        );
        self.offscreen
            .borrow_mut()
            .resize(scene_width, scene_height)
            .map_err(|error| error.to_string())?;
        let skip = render_skip::should_skip(
            self.host.scene().settings.skip_render,
            self.host.state() == EmulationState::Running,
            emulated,
            self.state.last_render_size == (scene_width, scene_height),
        );
        if skip {
            self.state.skipped_frames += 1;
        } else {
            render_scene(
                &mut self.renderer,
                &self.offscreen.borrow(),
                &mut self.host,
                &mut self.state,
            )?;
            self.state.last_render_size = (scene_width, scene_height);
            self.state.scene_frames += 1;
        }
        framebuffer::begin_window_pass(&self.gl, width, height);
        self.offscreen
            .borrow()
            .blit_to_default_framebuffer(width, view_height);
        self.ui.draw(&mut self.host, real);
        Ok(())
    }

    /// REQ-LOOP-052: the current game is stopped (End hook), the settings and the slot metadata
    /// are written, and nothing is auto-saved.
    fn finish(mut self) -> i32 {
        self.ui.close_capture();
        let emulated_frames = self.host.emulated_frames();
        self.host.shutdown();
        // REQ-INP-050 / REQ-PERS-065: the bindings are written at exit as well as after each
        // change.
        self.source.flush_changes();
        self.host.games().save();
        finish(&self.options, &self.state, emulated_frames)
    }
}

fn load_settings(options: &AppOptions, paths: &UserDataPaths) -> (SettingsStore, SettingsModel) {
    let mut store = SettingsStore::new(paths);
    let mut model = SettingsModel::new();
    store.load(&mut model, OffsetDateTime::now_utc());
    // The command line wins over the file, but only for the options it really carries (Q-066);
    // then the scene gets the values that came out of the two.
    command_line_settings::apply(options, &mut model);
    if !paths.is_writable {
        eprintln!(
            "Zaxnes: {}",
            paths.unwritable_message.clone().unwrap_or_default()
        );
    }
    (store, model)
}

struct HostParts<'a> {
    paths: &'a UserDataPaths,
    model: SettingsModel,
    scene: SceneState,
    catalog: LibraryGameCatalog,
    store: SettingsStore,
    asset_root: Option<&'a Path>,
    sample_rate: u32,
    audio_open: bool,
    offscreen: &'a Rc<RefCell<OffscreenFramebuffer>>,
    modal_flag: &'a Rc<Cell<bool>>,
}

fn build_host(parts: HostParts<'_>) -> AppHost {
    let library: Box<dyn UiAudioLibrary> = match AudioAssets::try_open(parts.asset_root, log) {
        Some(files) => Box::new(files),
        None => Box::new(SynthAudioLibrary::new(parts.sample_rate)),
    };
    let mut mixer = AudioMixer::new(parts.sample_rate, Some(library), TrackRandom::new(seed()));
    mixer.set_volume(parts.model.effective_volume());
    mixer.set_menu_music_enabled(parts.model.menu_music());
    let mut options =
        EmulationHostOptions::new(parts.paths.clone(), parts.model, parts.scene, parts.catalog);
    options.mixer = Some(mixer);
    options.script_log = Some(Box::new(|| Box::new(ConsoleScriptLog)));
    options.audio_device_open = parts.audio_open;
    options.log = Some(Box::new(log));
    options.settings_store = Some(parts.store);
    options.confirm_with_interface = true;
    let probe = Rc::clone(parts.modal_flag);
    options.modal_window_open = Some(Box::new(move || probe.get()));
    let capture = Rc::clone(parts.offscreen);
    options.capture_thumbnail = Some(Box::new(move || {
        capture.borrow().read_scaled_rgb(
            to_i32_usize(THUMBNAIL_WIDTH),
            to_i32_usize(THUMBNAIL_HEIGHT),
        )
    }));
    EmulationHost::new(options)
}

fn log_window_icon(window: &mut HostWindow) {
    let (images, problems) = icon::load_embedded();
    let state = icon::apply(Some(window), &images);
    let detail = if problems.is_empty() {
        String::new()
    } else {
        format!(" [{}]", problems.join("; "))
    };
    log(&format!("{state}{detail}"));
}

struct RunState {
    rendered_frames: u64,
    scene_frames: u64,
    skipped_frames: u64,
    last_render_size: (i32, i32),
    renderer_name: String,
    version: String,
    shadow_map_reported: bool,
    drag_origin: Option<[f32; 2]>,
    picture: Vec<u8>,
}

impl RunState {
    fn new(renderer_name: String, version: String) -> Self {
        Self {
            rendered_frames: 0,
            scene_frames: 0,
            skipped_frames: 0,
            last_render_size: (0, 0),
            renderer_name,
            version,
            shadow_map_reported: false,
            drag_origin: None,
            picture: Vec::new(),
        }
    }
}

fn to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[allow(clippy::cast_possible_truncation)] // the C# `(int)MathF.Round(ChromeHeight)`
fn to_i32_f(value: f32) -> i32 {
    value.round() as i32
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| u64::from(elapsed.subsec_nanos()))
}

fn smoke_banner(options: &AppOptions) -> String {
    if options.smoke_emulated_frames > 0 {
        format!(
            "Zaxnes smoke test: playing {} emulated frames.",
            options.smoke_emulated_frames
        )
    } else {
        format!(
            "Zaxnes smoke test: rendering {} frames.",
            options.smoke_test_frames
        )
    }
}

fn smoke_run_is_over(options: &AppOptions, host: &AppHost, state: &RunState) -> bool {
    if options.smoke_emulated_frames > 0 {
        u64::try_from(host.emulated_frames()).unwrap_or(0) >= options.smoke_emulated_frames
    } else {
        state.rendered_frames >= u64::from(options.smoke_test_frames)
    }
}

fn finish(options: &AppOptions, state: &RunState, emulated: i64) -> i32 {
    if !options.smoke_test {
        return 0;
    }
    let emulated = u64::try_from(emulated).unwrap_or(0);
    println!(
        "Zaxnes smoke test: rendered {} frames and emulated {emulated} on {} ({}).",
        state.rendered_frames, state.renderer_name, state.version
    );
    if options.smoke_emulated_frames > 0 {
        // AMD-024 REQ-PUB-001: what the run had to reach is a number of emulated frames, that is
        // a duration of play, not a number of redraws.
        if emulated >= options.smoke_emulated_frames {
            return 0;
        }
        eprintln!(
            "Zaxnes smoke test: expected {} emulated frames, only {emulated} were produced.",
            options.smoke_emulated_frames
        );
        return 1;
    }
    if state.rendered_frames >= u64::from(options.smoke_test_frames) {
        return 0;
    }
    eprintln!(
        "Zaxnes smoke test: expected {} frames, only {} rendered.",
        options.smoke_test_frames, state.rendered_frames
    );
    1
}

fn publish_settings_to_scene(model: &SettingsModel, scene: &mut SceneState) {
    scene_script_bridge::push_settings(model, scene);
    scene.settings.skip_render = model.skip_frame();
    scene.settings.set_target_frame_rate(model.frame_rate());
    scene.settings.set_vsync(model.vsync());
    scene
        .settings
        .set_anti_alias_samples(zax_render::geometry::anti_alias_samples(model.anti_alias()));
    scene.camera.snap_to_setpoints();
}

fn open_audio(options: &AppOptions, sdl: &sdl2::Sdl) -> (Option<AudioOutput>, u32) {
    let default_rate = u32::try_from(PREFERRED_SAMPLE_RATE).unwrap_or(48_000);
    if !options.audio {
        return (None, default_rate);
    }
    match AudioOutput::open(sdl) {
        Ok(output) => {
            let rate = u32::try_from(output.sample_rate()).unwrap_or(default_rate);
            (Some(output), rate)
        }
        Err(error) => {
            log(&format!("no audio output ({error}); the host runs silent."));
            (None, default_rate)
        }
    }
}

fn report_startup_problems(host: &mut AppHost, paths: &UserDataPaths) {
    use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
    if host.slots.was_recovered {
        host.notifications.show(
            "Save slots were rebuilt from the state files.",
            DEFAULT_SECONDS,
            NotificationSeverity::Warning,
        );
    }
    let messages: Vec<String> = host
        .settings_store()
        .map(|store| store.messages().to_vec())
        .unwrap_or_default();
    for message in messages {
        host.notifications
            .show(&message, DEFAULT_SECONDS, NotificationSeverity::Warning);
    }
    if let Some(message) = &paths.unwritable_message {
        host.notifications.show_error(message);
    }
}

fn start_game_from_command_line(
    host: &mut AppHost,
    options: &AppOptions,
    startup_game: Option<&str>,
) {
    let Some(rom) = options.rom_path.as_deref() else {
        println!(
            "Zaxnes: no --rom given; the window opens on the welcome screen (AMD-010 REQ-UI-207)."
        );
        return;
    };
    let full: PathBuf = std::path::absolute(rom).unwrap_or_else(|_| PathBuf::from(rom));
    let full = full.to_string_lossy().into_owned();
    // A smoke test must never stop on a dialog, so it takes the resolved entry, "2D Mode" when
    // no profile was named, rather than the interactive flow.
    if options.profile.is_some() || options.smoke_test {
        if let Some(game_id) = startup_game {
            host.launch_rom_with_profile(&full, game_id);
            return;
        }
    }
    host.enqueue(Command::OpenRom, CommandArgs::of_text(&full));
}

fn handle_camera_input(
    host: &mut AppHost,
    snapshot: &InputSnapshot,
    real: f32,
    state: &mut RunState,
) {
    // REQ-SCN-018 / REQ-INP-083: the camera controls only act while the game is running.
    if host.state() != EmulationState::Running {
        state.drag_origin = None;
        return;
    }
    // REQ-INP-080: a player whose port is in Zapper does not drive the camera.
    let camera_actions = to_camera_actions(routing::camera_sources(
        snapshot.player_one,
        snapshot.player_two,
        host.port_type(0),
        host.port_type(1),
    ));
    let scene = host.scene_mut();
    controls::apply(&mut scene.camera, camera_actions, real);
    // REQ-INP-090: the wheel is ignored over a piece of interface and while a zoom action is
    // held; 5 scene units per notch (DEC-08-3).
    if snapshot.wheel != 0.0 && !snapshot.pointer_over_ui {
        controls::apply_wheel(&mut scene.camera, snapshot.wheel, camera_actions);
    }
    // REQ-INP-091: left-button drag orbits, 0.1 degrees per pixel.
    if snapshot.left_button && !snapshot.pointer_over_ui {
        if let Some(origin) = state.drag_origin {
            // The screen y axis points down, so the vertical delta is negated (REQ-SCN-017).
            controls::apply_drag(
                &mut scene.camera,
                snapshot.pointer[0] - origin[0],
                origin[1] - snapshot.pointer[1],
            );
        }
        state.drag_origin = Some(snapshot.pointer);
    } else {
        state.drag_origin = None;
    }
}

fn update_cursor(
    cursor: &mut CursorAutoHide,
    host: &AppHost,
    input: &mut SdlInput,
    snapshot: &InputSnapshot,
    real: f32,
) {
    use zax_platform::input::PointerDevice as _;
    let visible = cursor.update(
        host.state() == EmulationState::Running,
        snapshot.pointer_moved,
        real,
    );
    if input.pointer.cursor_visible() != visible {
        input.pointer.set_cursor_visible(visible);
    }
}

fn update_scene(host: &mut AppHost, state: &mut RunState) {
    let two_dimensional = host.showing_two_dimensional();
    let (session, scene) = host.session_and_scene_mut();
    let Some(session) = session else {
        scene.two_dimensional_mode = false;
        scene.followed_shape = None;
        return;
    };
    scene.nes_background_color = session.nes_background_color();
    scene.emulated_time = session.emulated_time();
    scene.clipping = session.engine().settings().clipping;
    // AMD-009 (Q-022): the 2D mode shows the core's pixel frame on the NES plane. AMD-017
    // (Q-037) adds the hot F11 toggle to the "2D Mode" profile of REQ-SCN-055: the host combines
    // the two, and nothing of the session is reloaded to switch.
    scene.two_dimensional_mode = two_dimensional;
    if two_dimensional {
        session.copy_nes_picture(&mut state.picture);
    }
    // REQ-SCN-022: the renderer needs the followed shape to find the occluders.
    scene.followed_shape = session.followed_shape();
}

fn render_scene(
    renderer: &mut SceneRenderer,
    offscreen: &OffscreenFramebuffer,
    host: &mut AppHost,
    state: &mut RunState,
) -> Result<(), String> {
    let target = RenderTarget {
        framebuffer: offscreen.handle(),
        width: offscreen.width(),
        height: offscreen.height(),
    };
    let (session, scene) = host.session_and_scene();
    let picture: &[u8] = if scene.two_dimensional_mode {
        &state.picture
    } else {
        &[]
    };
    let list = session.map(GameSession::build_draw_list);
    let frame = SceneFrame {
        draw_list: list,
        nes_picture: picture,
    };
    renderer
        .render(&target, scene, &frame)
        .map_err(|error| error.to_string())?;
    let statistics = renderer.statistics();
    if !state.shadow_map_reported
        && statistics.shadow_map_size != statistics.requested_shadow_map_size
    {
        state.shadow_map_reported = true;
        log(&format!(
            "the hardware refused a {}-texel shadow map; it is {} texels (AMD-034).",
            statistics.requested_shadow_map_size, statistics.shadow_map_size
        ));
    }
    Ok(())
}

fn forward_events(
    host: &mut AppHost,
    ui: &mut Box<dyn ApplicationUi>,
    window: &mut HostWindow,
    pacing: &mut VideoPacing,
) {
    for event in host.take_events() {
        match event {
            HostEvent::SettingChanged(id) => pacing.on_setting_changed(window, host.settings(), id),
            HostEvent::LanguageChanged(code) => ui.language_changed(&code),
            HostEvent::UiRequested { command, args } => {
                // REQ-UI-209: the window state follows ui.fullscreen, whoever flipped it.
                if command == Command::ToggleFullscreen {
                    if let Err(error) = window.set_fullscreen(host.settings().fullscreen()) {
                        log(&format!("the window state could not be changed: {error}"));
                    }
                    // FIX-1: a window-state change makes the windowing layer re-apply its own
                    // boolean vsync, which would lose the interval 2 and, on some drivers, the
                    // interval altogether.
                    pacing.apply(window, host.settings());
                }
                ui.handle_ui_request(host, command, &args);
            }
        }
    }
}

fn pump_audio(host: &mut AppHost, audio: Option<&mut AudioOutput>, real: f64) {
    let Some(mixer) = host.mixer_mut() else {
        return;
    };
    match audio {
        Some(output) => {
            // The device pulls as many whole blocks as it has free buffers; the mixer's own
            // clock therefore advances with the audio it produced, which is real time.
            match output.pump(|buffer| mixer.fill(buffer)) {
                Ok(0) => mixer.advance(real),
                Ok(_) => {}
                Err(error) => {
                    log(&format!("audio output: {error}"));
                    mixer.advance(real);
                }
            }
        }
        // No device: the fades and the music selection still advance, so the state machine
        // behaves identically in silence.
        None => mixer.advance(real),
    }
}

fn capture_window(gl: &glow::Context, width: i32, height: i32, path: &Path) {
    let rgb = framebuffer::read_window_rgb(gl, width, height);
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

fn to_i32_usize(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
