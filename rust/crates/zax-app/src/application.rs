//! The application: the window, the loop that samples the input, runs the host, draws the
//! scene and pumps the audio (SPEC-07, SPEC-06 §3.13). The interface plugs in through
//! [`ApplicationUi`].

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use sdl2::event::{Event, WindowEvent};
use time::OffsetDateTime;
use zax_platform::audio::PREFERRED_SAMPLE_RATE;
use zax_platform::framebuffer::{self, OffscreenFramebuffer};
use zax_platform::icon;
use zax_platform::input::sdl::SdlInput;
use zax_platform::pacing::DisplayPacingState;
use zax_platform::{AudioOutput, HostWindow, HostWindowSettings};
use zax_profiles::format;
use zax_render::assets::{NeutralSceneAssets, SceneAssets};
use zax_render::controls;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::target::RenderTarget;
use zax_render::SceneRenderer;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

use crate::app_log;
use crate::app_options::AppOptions;
use crate::audio::audio_assets::AudioAssets;
use crate::audio::synth_audio_library::SynthAudioLibrary;
use crate::audio::{AudioMixer, TrackRandom, UiAudioLibrary};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::frame_pacer::{CatchUp, MAX_BACKLOG};
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::host::{
    EmulationHost, EmulationHostOptions, GameView, HostEvent, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH,
};
use crate::emulation::video_pacing::{VideoPacing, VideoPacingRequest};
use crate::frame_profiler::{self, FrameProfiler, Stage, StageClock};
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
use crate::ui::debug_overlay::DebugInfo;
use crate::ui::frontend::{ImguiFrontend, Sizes};
use crate::ui::handheld;
use crate::ui::strings::UiLanguage;
use crate::ui::zax_ui::{UiEffect, UiFrame};
use crate::user_data_paths::UserDataPaths;
use crate::{neutral_assets, persistence::input_binding_store::InputBindingStore as _};

/// The host the application runs: over the library, with the offscreen picture of the scene as
/// its view of the game.
pub type AppHost = EmulationHost<LibraryGameCatalog, OffscreenFramebuffer>;

impl GameView for OffscreenFramebuffer {
    fn capture_thumbnail(&mut self) -> Option<Vec<u8>> {
        self.read_scaled_rgb(
            to_i32_usize(THUMBNAIL_WIDTH),
            to_i32_usize(THUMBNAIL_HEIGHT),
        )
    }
}

/// The folder of the user data where the linked shader programs are kept.
const SHADER_CACHE_FOLDER: &str = "shader-cache";

/// Whether the window opens in full screen: the persisted choice, or always on a handheld,
/// except for smoke runs and screenshot runs.
#[must_use]
pub const fn should_open_fullscreen(options: &AppOptions, persisted: bool) -> bool {
    (persisted || options.handheld) && !options.smoke_test && options.ui_shot_directory.is_none()
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
    app_log::line(&format!("Zaxnes: {message}"));
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
    pub fn run(self) -> i32 {
        match self.run_inner() {
            Ok(code) => code,
            Err(message) => {
                app_log::error(&message);
                1
            }
        }
    }

    fn run_inner(&self) -> Result<i32, String> {
        let mut running = Running::build(&self.options)?;
        while running.frame()? {}
        Ok(running.finish())
    }
}

/// Everything a run owns once the window is open.
struct Running {
    options: AppOptions,
    ui: ImguiFrontend,
    window: HostWindow,
    gl: Rc<glow::Context>,
    renderer: SceneRenderer,
    input: SdlInput,
    source: DesktopInputSource,
    audio: Option<AudioOutput>,
    host: AppHost,
    pacing: VideoPacing,
    counter: FrameRateCounter,
    cursor: CursorAutoHide,
    state: RunState,
    last: Instant,
    /// The frame profiler (`ZAXNES_PROFILE=1`) and the clock of the loop's stages.
    profiler: Option<FrameProfiler>,
    clock: StageClock,
}

impl Running {
    fn build(options: &AppOptions) -> Result<Self, String> {
        let asset_root = neutral_assets::locate(options.asset_root.as_deref().map(Path::new));
        let paths = resolve_user_data(options);
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
        log_display(&window, &gl);
        let (renderer_name, version) = framebuffer::gl_strings(&gl);
        let request_options = options.clone();
        let mut pacing = VideoPacing::new(
            Some(Box::new(move |model| {
                pacing_request(&request_options, model)
            })),
            Some(Box::new(log)),
        );
        let pacing_state = pacing.apply(&mut window, &model);
        let profiling = frame_profiler::requested_by_environment();
        if profiling {
            log_profiler_startup(
                &model,
                pacing_state,
                catch_up(options),
                group_draws(options),
            );
        }
        let (width, height) = window.drawable_size();
        // The host owns the picture of the scene: it reads it back for the thumbnails whenever
        // the game pauses, and the loop draws into it through the host.
        let view = OffscreenFramebuffer::new(Rc::clone(&gl), to_i32(width), to_i32(height))
            .map_err(|error| error.to_string())?;
        // The linked programs are kept in the user folder, where the next start finds them.
        let mut renderer = open_renderer(&gl, asset_root.as_deref(), &paths)?;
        renderer.set_group_opaque_draws(group_draws(options));
        let input = SdlInput::new(window.sdl()).map_err(|error| error.to_string())?;
        let mut bindings_store = FileInputBindingStore::new(paths.clone(), OffsetDateTime::now_utc);
        let mut bindings = crate::input::InputBindings::new();
        bindings_store.load(&mut bindings);
        let source = DesktopInputSource::new(bindings, Box::new(bindings_store));

        let (audio, sample_rate) = open_audio(options, window.sdl());
        let parts = HostParts {
            paths: &paths,
            model,
            scene,
            catalog,
            store,
            asset_root: asset_root.as_deref(),
            sample_rate,
            audio_open: audio.is_some(),
            view,
        };
        let mut host = build_host(parts);
        host.set_profiling(profiling);
        host.catch_up = catch_up(options);
        report_startup_problems(&mut host, &paths);
        start_game_from_command_line(&mut host, options, startup_game.as_deref());
        let ui = ImguiFrontend::new(&gl, &host, options, height)?;
        Ok(Self {
            options: options.clone(),
            ui,
            window,
            gl,
            renderer,
            input,
            source,
            audio,
            host,
            pacing,
            counter: FrameRateCounter::new(),
            cursor: CursorAutoHide::new(),
            state: RunState::new(renderer_name, version),
            last: Instant::now(),
            profiler: profiling.then(|| FrameProfiler::new(Instant::now())),
            clock: StageClock::new(profiling),
        })
    }

    /// One displayed frame. Returns false once the application is to close.
    fn frame(&mut self) -> Result<bool, String> {
        let frame_begin = self.clock.start();
        let mark = self.clock.start();
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
        self.host.set_modal_window_open(self.ui.is_modal());
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
        self.clock.stop(Stage::Input, mark);
        let mark = self.clock.start();
        let emulated = self.host.tick(real, &snapshot);
        self.source.end_sample();
        if let Some(mark) = mark {
            // The emulated frames' own stages are counted apart from the rest of the tick.
            let inner = self.host.take_stage_times();
            self.clock.add(
                Stage::Emulation,
                mark.elapsed().saturating_sub(inner.total()),
            );
            self.clock.merge(&inner);
        }
        let mark = self.clock.start();
        update_scene(&mut self.host, &mut self.state);
        {
            let scene = self.host.scene_mut();
            scene.camera.advance(real_f);
            scene.real_time += real_f;
        }
        self.clock.stop(Stage::Scene, mark);
        forward_events(
            &mut self.host,
            &mut self.ui,
            &mut self.window,
            &mut self.pacing,
        );
        self.draw(emulated, width, height, real_f)?;
        self.carry_out_ui_effects();
        let mark = self.clock.start();
        pump_audio(&mut self.host, self.audio.as_mut(), real);
        self.clock.stop(Stage::Audio, mark);
        if self.host.quit_requested()
            || (self.options.smoke_test
                && smoke_run_is_over(&self.options, &self.host, &self.state))
        {
            if let Some(shot) = self.options.smoke_shot_path.as_deref() {
                capture_window(&self.gl, width, height, Path::new(shot));
            }
            return Ok(false);
        }
        let mark = self.clock.start();
        self.window.swap();
        self.clock.stop(Stage::Swap, mark);
        let mark = self.clock.start();
        self.window.pace();
        self.clock.stop(Stage::Pacer, mark);
        if let (Some(profiler), Some(begin)) = (self.profiler.as_mut(), frame_begin) {
            let times = self.clock.take();
            if let Some(report) =
                profiler.end_frame(&times, begin.elapsed(), emulated, Instant::now())
            {
                log(&report);
            }
        }
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
        let consumed = self.ui.consumed_keys(&self.input.keyboard);
        self.source.consume_keys(consumed);
        let snapshot = self.source.sample(
            real,
            DeviceRefs {
                keyboard: Some(&self.input.keyboard),
                pointer: Some(&mut self.input.pointer),
                hub: &mut self.input.gamepads,
            },
        );
        let capturing = self.source.rebinding().suppresses_actions();
        self.ui.handle_shortcuts(
            &mut self.host,
            &self.input.keyboard,
            capturing,
            self.source.bindings(),
        );
        for command in self.source.triggered_commands() {
            if let Some(command) = Command::from_name(command) {
                self.host.enqueue(command, CommandArgs::default());
            }
        }
        // The commands the interface owns (F12 and its like) are not in the host's model.
        self.ui.run_bound_ui_commands(&self.source);
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
        self.host
            .view_mut()
            .ok_or("the host has no view of the game")?
            .resize(scene_width, scene_height)
            .map_err(|error| error.to_string())?;
        let skip = render_skip::should_skip(
            self.host.scene().settings.skip_render,
            self.host.state() == EmulationState::Running,
            emulated,
            self.state.last_render_size == (scene_width, scene_height),
        );
        let mark = self.clock.start();
        if skip {
            self.state.skipped_frames += 1;
        } else {
            render_scene(&mut self.renderer, &mut self.host, &mut self.state)?;
            self.state.last_render_size = (scene_width, scene_height);
            self.state.scene_frames += 1;
        }
        self.clock.stop(Stage::Scene, mark);
        let mark = self.clock.start();
        framebuffer::begin_window_pass(&self.gl, width, height);
        if let Some(view) = self.host.view() {
            view.blit_to_default_framebuffer(width, view_height);
        }
        let debug = DebugInfo {
            frames_per_second: self.counter.frames_per_second(),
            frame_milliseconds: self.counter.frame_milliseconds(),
            rendered_frames: self.state.rendered_frames,
            scene_frames: self.state.scene_frames,
            skipped_frames: self.state.skipped_frames,
            statistics: self.renderer.statistics(),
            gl_renderer: &self.state.renderer_name,
        };
        let sizes = Sizes {
            window: self.window.window().size(),
            drawable: self.window.drawable_size(),
        };
        let mut frame = UiFrame {
            host: &mut self.host,
            input: &mut self.source,
            hub: &self.input.gamepads,
            real_seconds: real,
            #[allow(clippy::cast_possible_truncation)] // the C# `(float)FramesPerSecond`
            frames_per_second: debug.frames_per_second as f32,
            profile_line: self.profiler.as_ref().map_or("", FrameProfiler::summary),
        };
        let gl = &self.gl;
        self.ui.draw(&mut frame, sizes, &debug, |path| {
            capture_window(gl, width, height, path);
        });
        self.clock.stop(Stage::Ui, mark);
        Ok(())
    }

    /// Does what the interface asked: the lines of the log, the data folder in the file manager.
    fn carry_out_ui_effects(&mut self) {
        for effect in self.ui.take_effects() {
            match effect {
                UiEffect::Log(message) => log(&message),
                UiEffect::RevealFolder(folder) => reveal_folder(&folder),
                UiEffect::LeaveFullscreen => self.set_window_fullscreen(false),
                UiEffect::RestoreFullscreen => {
                    self.set_window_fullscreen(self.host.settings().fullscreen());
                }
            }
        }
    }

    /// Puts the window in or out of full screen without touching `ui.fullscreen` (a native file
    /// dialog needs the window out of the way while it is open).
    fn set_window_fullscreen(&mut self, fullscreen: bool) {
        if let Err(error) = self.window.set_fullscreen(fullscreen) {
            log(&format!("the window state could not be changed: {error}"));
        }
        // As for any window-state change (FIX-1): the vsync interval is applied again.
        self.pacing.apply(&mut self.window, self.host.settings());
    }

    /// REQ-LOOP-052: the current game is stopped (End hook), the settings and the slot metadata
    /// are written, and nothing is auto-saved.
    fn finish(mut self) -> i32 {
        ImguiFrontend::close_capture(&mut self.source);
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
    if options.handheld {
        handheld::defaults::apply(&mut model);
    }
    if let Some(locale) = options.system_locale.as_deref() {
        // Only the default: a saved `ui.language` (read next) always wins.
        model.set_environment_default(
            SettingId::UiLanguage,
            UiLanguage::from_locale(locale).code(),
        );
    }
    store.load(&mut model, OffsetDateTime::now_utc());
    // The command line wins over the file, but only for the options it really carries (Q-066);
    // then the scene gets the values that came out of the two.
    command_line_settings::apply(options, &mut model);
    if !paths.is_writable {
        app_log::error(&format!(
            "Zaxnes: {}",
            paths.unwritable_message.clone().unwrap_or_default()
        ));
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
    view: OffscreenFramebuffer,
}

/// How far the frame clock catches up: the command line or `ZAXNES_CATCH_UP`, else none on a
/// handheld (a late frame slows the game a little there instead of collapsing the display into
/// five emulated frames per shown one) and REQ-LOOP-012 elsewhere.
fn catch_up(options: &AppOptions) -> CatchUp {
    options.catch_up.unwrap_or(if options.handheld {
        CatchUp::Off
    } else {
        CatchUp::Limit(MAX_BACKLOG)
    })
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
    options.view = Some(parts.view);
    EmulationHost::new(options)
}

/// The user folder, and the file copy of the log opened in it. On a handheld, a root the
/// command line names that refuses writes is replaced by the per-user folder when that one
/// accepts them (Rust-only: the settings and the saves would otherwise be lost silently).
fn resolve_user_data(options: &AppOptions) -> UserDataPaths {
    let explicit = options.user_data_root.as_deref().map(Path::new);
    let mut paths = UserDataPaths::resolve(explicit, None, None);
    if !paths.is_writable && explicit.is_some() && options.handheld {
        let fallback = UserDataPaths::resolve(None, None, None);
        if fallback.is_writable {
            app_log::error(&format!(
                "Zaxnes: '{}' refuses writes; the user data goes to '{}' instead.",
                paths.root.display(),
                fallback.root.display()
            ));
            paths = fallback;
        }
    }
    // AMD-029 §2: the first run after the rename seeds the new root from the old one.
    if let Some(legacy) = &paths.migrated_from {
        log(&format!(
            "user data copied from '{}' to '{}'; the old folder was left untouched (AMD-029).",
            legacy.display(),
            paths.root.display()
        ));
    }
    open_log_file(&paths);
    paths
}

/// The start-up report of the display: SDL's driver and mode, the GL context it created, the GL
/// strings, and a warning when the renderer is a software one.
fn log_display(window: &HostWindow, gl: &glow::Context) {
    log(&window.display_report());
    log(&window.gl_config_report());
    log(&framebuffer::gl_report(gl));
    if let Some(warning) = framebuffer::software_renderer_warning(gl) {
        log(&warning);
    }
}

/// Opens the file copy of the log (Rust-only): in the user folder's `logs`, else next to the
/// executable, else in the temporary folder. The start-up report and the profiler reach it even
/// when the launcher loses the console. Says where the user data and the log are.
fn open_log_file(paths: &UserDataPaths) {
    let candidates = [
        paths.logs_directory.clone(),
        paths.executable_directory.join("logs"),
        std::env::temp_dir(),
    ];
    let (file, failures) = app_log::open_first(&candidates);
    match &file {
        Some(file) => log(&format!(
            "zaxnes {} ({}), log file '{}'",
            env!("CARGO_PKG_VERSION"),
            frame_profiler::build_profile(),
            file.display()
        )),
        None => app_log::error("Zaxnes: no log file could be created; the console has the log."),
    }
    for failure in failures {
        app_log::error(&format!(
            "Zaxnes: the log file could not be created at {failure}"
        ));
    }
    log(&format!(
        "user data folder '{}' ({})",
        paths.root.display(),
        if paths.is_writable {
            "writable"
        } else {
            "NOT writable: settings and saves will not be kept"
        }
    ));
}

/// Whether the renderer reorders the opaque tiles by draw state, which batches far more of them
/// but may change a few edge pixels where two surfaces sit at the same depth:
/// `ZAXNES_GROUP_DRAWS=1` or `0`, else on a handheld only.
fn group_draws(options: &AppOptions) -> bool {
    match std::env::var(GROUP_DRAWS_VARIABLE)
        .as_deref()
        .map(str::trim)
    {
        Ok("1") => true,
        Ok("0") => false,
        _ => options.handheld,
    }
}

/// The variable that forces [`group_draws`].
const GROUP_DRAWS_VARIABLE: &str = "ZAXNES_GROUP_DRAWS";

/// The scene renderer, with its linked programs kept in the user folder.
fn open_renderer(
    gl: &Rc<glow::Context>,
    asset_root: Option<&Path>,
    paths: &UserDataPaths,
) -> Result<SceneRenderer, String> {
    SceneRenderer::with_program_cache(
        Rc::clone(gl),
        NeutralSceneAssets::try_open(asset_root)
            .map(|assets| Box::new(assets) as Box<dyn SceneAssets>),
        paths
            .is_writable
            .then(|| paths.root.join(SHADER_CACHE_FOLDER)),
    )
    .map_err(|error| error.to_string())
}

/// The start-up line of the frame profiler: the machine, the build, the render settings and
/// the pacing in force.
fn log_profiler_startup(
    model: &SettingsModel,
    pacing: DisplayPacingState,
    catch_up: CatchUp,
    group_draws: bool,
) {
    log(&frame_profiler::startup_report(
        &format!(
            "render scale {}, render method {}, shadows {}, MSAA setting {}, vsync {}, \
             frame rate setting {}",
            model.res_scale_factor(),
            model.render_method(),
            model.shadow(),
            model.anti_alias(),
            model.vsync(),
            model.frame_rate()
        ),
        &format!("{pacing}, catch-up {catch_up:?}, grouped draws {group_draws}"),
    ));
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
        log("no --rom given; the window opens on the welcome screen (AMD-010 REQ-UI-207).");
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
    host: &mut AppHost,
    state: &mut RunState,
) -> Result<(), String> {
    let view = host.view().ok_or("the host has no view of the game")?;
    let target = RenderTarget {
        framebuffer: view.handle(),
        width: view.width(),
        height: view.height(),
    };
    let (session, scene) = host.session_and_scene();
    let picture: &[u8] = if scene.two_dimensional_mode {
        &state.picture
    } else {
        &[]
    };
    // In 2D mode the renderer shows the core's picture alone (AMD-009) and reads nothing of the
    // draw list, so it is not built (Rust-only saving: the C# loop builds it every frame).
    let list = if picture.is_empty() {
        session.map(GameSession::build_draw_list)
    } else {
        None
    };
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
    ui: &mut ImguiFrontend,
    window: &mut HostWindow,
    pacing: &mut VideoPacing,
) {
    for event in host.take_events() {
        match &event {
            HostEvent::SettingChanged(id) => {
                pacing.on_setting_changed(window, host.settings(), *id);
            }
            // REQ-UI-209: the window state follows ui.fullscreen, whoever flipped it.
            HostEvent::UiRequested {
                command: Command::ToggleFullscreen,
                ..
            } => {
                if let Err(error) = window.set_fullscreen(host.settings().fullscreen()) {
                    log(&format!("the window state could not be changed: {error}"));
                }
                // FIX-1: a window-state change makes the windowing layer re-apply its own
                // boolean vsync, which would lose the interval 2 and, on some drivers, the
                // interval altogether.
                pacing.apply(window, host.settings());
            }
            HostEvent::UiRequested { .. } | HostEvent::LanguageChanged(_) => {}
        }
        ui.handle_host_event(host, event);
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

pub(crate) fn capture_window(gl: &glow::Context, width: i32, height: i32, path: &Path) {
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

/// Opens `folder` in the desktop's file manager (REQ-PERS-072). A failure is logged and nothing
/// else: the path is also printed.
fn reveal_folder(folder: &Path) {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let result = std::fs::create_dir_all(folder).and_then(|()| {
        std::process::Command::new(opener)
            .arg(folder)
            .spawn()
            .map(drop)
    });
    if let Err(error) = result {
        log(&format!("the data folder could not be opened: {error}"));
    }
}
