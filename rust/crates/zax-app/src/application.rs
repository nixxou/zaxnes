//! The application: the window, the loop that samples the input, runs the host, draws the
//! scene and pumps the audio (SPEC-07, SPEC-06 §3.13). The interface plugs in through
//! [`ApplicationUi`].

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sdl2::event::{Event, WindowEvent};
use time::OffsetDateTime;
use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::frame::ConversionFrame;
use zax_nes::ppu::SpriteLimit;
use zax_platform::audio::PREFERRED_SAMPLE_RATE;
use zax_platform::framebuffer::{self, OffscreenFramebuffer};
use zax_platform::icon;
use zax_platform::input::sdl::SdlInput;
use zax_platform::pacing::DisplayPacingState;
use zax_platform::WindowGraphics;
use zax_platform::{
    AudioOutput, ClassicAudioOutput, HostWindow, HostWindowSettings, PlatformError,
};
use zax_profiles::catalogue::ProfileImportOutcome;
use zax_profiles::format;
use zax_render::assets::{NeutralSceneAssets, SceneAssets};
use zax_render::controls;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::sync::GpuFrameQueue;
use zax_render::SceneRenderer;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use crate::app_log;
use crate::app_options::AppOptions;
use crate::audio::audio_assets::AudioAssets;
use crate::audio::stream::{self as audio_stream, AudioLink, AudioStream, ProgramWriter};
use crate::audio::synth_audio_library::SynthAudioLibrary;
use crate::audio::{AudioMixer, TrackRandom, UiAudioLibrary};
use crate::demo_game;
use crate::editor_mode;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::frame_pacer::CatchUp;
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::host::{
    DeferredParts, EditorRequest, EmulationHost, EmulationHostOptions, HostEvent,
};
use crate::emulation::video_pacing::{VideoPacing, VideoPacingRequest};
use crate::frame_profiler::{self, FrameProfiler, Stage, StageClock, Thread};
use crate::frame_rate_counter::FrameRateCounter;
use crate::game_session::GameSession;
use crate::graphics::{Backend, Graphics, SceneView};
use crate::input::cursor_auto_hide::CursorAutoHide;
use crate::input::input_action::to_camera_actions;
use crate::input::{routing, DesktopInputSource, DeviceRefs, InputSnapshot};
use crate::library_game_catalog::LibraryGameCatalog;
use crate::persistence::app_settings::{AppSettings, AppSettingsMut as _};
use crate::persistence::command_line_settings;
use crate::persistence::input_binding_store::FileInputBindingStore;
use crate::persistence::save_slots;
use crate::persistence::settings_store::SettingsStore;
use crate::pipeline::{PipelineGovernor, PipelineMode};
use crate::render_scaling;
use crate::render_skip;
use crate::scene_script_bridge::{self, ConsoleScriptLog};
use crate::ui::debug_overlay::DebugInfo;
use crate::ui::frontend::{ImguiFrontend, Sizes, UiGraphics};
use crate::ui::handheld;
use crate::ui::strings::{self, UiLanguage};
use crate::ui::zax_ui::{UiEffect, UiFrame};
use crate::user_data_paths::UserDataPaths;
use crate::{neutral_assets, persistence::input_binding_store::InputBindingStore as _};

/// The host the application runs: over the library, with the offscreen picture of the scene as
/// its view of the game.
pub type AppHost = EmulationHost<LibraryGameCatalog, SceneView>;

/// The folder of the user data where the linked shader programs are kept.
const SHADER_CACHE_FOLDER: &str = "shader-cache";

/// The window of AMD-051 REQ-E3D-646, as the diagnostic line states it: a late object has not
/// been drawable for this many emulated frames.
#[allow(clippy::cast_sign_loss)] // A positive constant of the specification.
const LATE_OBJECT_FRAMES_REPORTED: u32 = zax_engine3d::draw_list::LATE_OBJECT_FRAMES as u32;

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

/// Where the editor mode stands (`editor_mode.rs`).
#[derive(Debug, Clone, Copy)]
struct EditorState {
    /// Whether it is on.
    on: bool,
    /// Whether the setting was read once (the start does not make a profile by itself).
    checked: bool,
    /// Whether a game on a generic entry is to get a blank Lua profile once it runs.
    wants_project: bool,
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
    // Field order is drop order: the renderer and the host (whose view is the scene picture)
    // go before the window, whose GL context or wgpu device they were made on.
    graphics: Graphics,
    /// Bounds the rendered frames the GPU keeps waiting for display (AMD-052 REQ-LOOP-077),
    /// `None` outside the OpenGL renderer and on a handheld at the default depth (see
    /// [`Running::bound_gpu_queue`]).
    gpu_queue: Option<GpuFrameQueue>,
    input: SdlInput,
    source: DesktopInputSource,
    /// The output that is open and what the main thread needs of it (AUDIO-2). It stands before
    /// `host` in the field order on purpose: dropping it closes the device, which must happen
    /// before the mixer and the core it reads go away.
    audio: AudioPath,
    host: AppHost,
    window: HostWindow,
    pacing: VideoPacing,
    counter: FrameRateCounter,
    cursor: CursorAutoHide,
    state: RunState,
    last: Instant,
    /// The frame profiler (`ui.profiler`, or `ZAXNES_PROFILE=1`) and the clock of the loop's
    /// stages.
    profiler: Option<FrameProfiler>,
    clock: StageClock,
    /// The displayed and emulated frames counted since `rate_since`, logged every 5 s.
    rate_count: (u32, u32),
    rate_since: Instant,
    /// The pipelined frames (`emu.pipeline`).
    pipeline: Pipelined,
    /// The automatic resolution (`video.dynamic_resolution`).
    resolution: AutomaticResolution,
    /// The save state slot `-slot=<n>` (AMD-053 REQ-UI-213) or `--load-state` asks for, until the
    /// game accepts the load: `"1"` to `"36"` or `"quick"`.
    pending_state_load: Option<String>,
    /// When the audio counters were last written to the log.
    audio_logged_at: Instant,
    /// The MCP server of the editor (`--mcp`, the editor mode), off by default.
    mcp: Option<crate::mcp::McpBridge>,
    /// The editor mode (`editor_mode.rs`): whether it is on, and whether it is to make a blank
    /// Lua profile for the game once one runs.
    editor: EditorState,
}

/// The state of the automatic resolution between two displayed frames.
#[derive(Default)]
struct AutomaticResolution {
    governor: render_scaling::AutomaticScale,
    /// Whether it is in force: the setting is on, the render method scales the view, the
    /// renderer is the OpenGL one and the frames have a pace to keep.
    active: bool,
    /// When the frame before began and how long its work before the swap took: its outcome is
    /// known once the next one begins.
    previous: Option<(Instant, Duration)>,
    /// The profiler's window: the lowest and highest scale, and the frames late because of the
    /// drawing and because of the work.
    window: Option<(f32, f32, u32, u32)>,
}

/// The state of the pipelined frames between two displayed frames.
#[derive(Default)]
struct Pipelined {
    /// Whether the next frame is pipelined.
    governor: PipelineGovernor,
    /// The list a pipelined frame draws while the next one is built.
    displayed: FrameDrawList,
    /// Whether the engine holds a list newer than `displayed` (built by the last conversion or
    /// drawn by the direct path), which the next pipelined frame takes.
    list_waiting: bool,
}

impl Running {
    #[allow(clippy::too_many_lines)] // The start-up sequence, in its order.
    fn build(options: &AppOptions) -> Result<Self, String> {
        let asset_root = neutral_assets::locate(options.asset_root.as_deref().map(Path::new));
        let paths = resolve_user_data(options);
        let (store, mut model) = load_settings(options, &paths);
        let demo_rom = install_demo_game(&paths, &model);
        #[cfg(feature = "shaders")]
        install_shader_pack(&paths);
        let mut catalog = LibraryGameCatalog::open(asset_root.as_deref(), &paths, Box::new(log));
        if let Some(rom) = demo_rom {
            catalog.set_rom_path(demo_game::PROFILE_ID, &rom.to_string_lossy());
        }
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
        let window_settings = HostWindowSettings {
            title: "Zaxnes".to_owned(),
            width: options.width,
            height: options.height,
            vsync: request.vsync,
            target_frame_rate: request.target_frame_rate,
            fullscreen: should_open_fullscreen(options, model.fullscreen()),
            graphics: WindowGraphics::Gl,
        };
        let mut window = open_window(options, &model, &window_settings)?;
        // The display's refresh rate, which `video.frame_rate` Auto and the vsync's derived rates
        // read (60 until it is known).
        if let Some(rate) = window.refresh_rate() {
            model.set_display_refresh_rate(rate);
        }
        if options.smoke_test {
            println!("{}", smoke_banner(options));
        }
        log_window_icon(&mut window);
        let (renderer_name, version) = log_display(&window);
        let request_options = options.clone();
        let mut pacing = VideoPacing::new(
            Some(Box::new(move |model| {
                pacing_request(&request_options, model)
            })),
            Some(Box::new(log)),
        );
        let pacing_state = pacing.apply(&mut window, &model);
        let profiling = frame_profiler::requested_by_environment()
            || model.get(SettingId::UiProfiler).as_bool();
        if profiling {
            log_profiler_startup(
                &model,
                pacing_state,
                options.catch_up.unwrap_or_else(|| {
                    CatchUp::from_setting(model.get(SettingId::EmuCatchUp).as_int())
                }),
                group_draws(options),
                separate_core(options),
                model.get(SettingId::EmuLuaSmoothGc).as_bool(),
            );
        }
        let (width, height) = window.drawable_size();
        // The host owns the picture of the scene: it reads it back for the thumbnails whenever
        // the game pauses, and the loop draws into it through the host.
        let (graphics, view) = open_graphics(
            &window,
            asset_root.as_deref(),
            &paths,
            group_draws(options),
            (to_i32(width), to_i32(height)),
        )?;
        let input = SdlInput::new(window.sdl()).map_err(|error| error.to_string())?;
        let mut bindings_store = FileInputBindingStore::new(paths.clone(), OffsetDateTime::now_utc);
        let bindings = load_bindings(&mut bindings_store, options.handheld);
        let source = DesktopInputSource::new(bindings, Box::new(bindings_store));

        // AUDIO-2: which output opens is read here, once. `attach_audio` is what reads the
        // setting again, and only when the options window changes it.
        let classic_audio = model.classic_audio_output();
        let latency_ms = model.audio_latency_ms();
        let OpenedAudio {
            path: audio,
            sample_rate,
            link,
        } = open_audio(options, window.sdl(), latency_ms, classic_audio);
        let classic_audio = audio.is_classic();
        let parts = HostParts {
            paths: &paths,
            model,
            scene,
            catalog,
            store,
            asset_root: asset_root.as_deref(),
            sample_rate,
            audio: link,
            classic_audio,
            view,
            suspend_resume: options.no_resume,
            emu_stats: options.emu_stats,
            tile_cache: !options.no_tile_cache,
        };
        let mut host = build_host(parts);
        // REQ-UI-213: a value the command line could not use is logged, never refused.
        for notice in &options.notices {
            log(notice);
        }
        host.set_profiling(profiling);
        configure_pacing(&mut host, options);
        // The game in 2D (the "2D Mode" profile, F11) is seen head on and fills the height of the
        // view, not through the 3D camera (owner request); the console mode does the same.
        host.scene_mut().picture_head_on = true;
        report_startup_problems(&mut host, &paths);
        start_game_from_command_line(&mut host, options, startup_game.as_deref());
        let ui_graphics = match &graphics {
            Graphics::Gl { gl, .. } => UiGraphics::Gl(gl),
            #[cfg(feature = "wgpu")]
            Graphics::Wgpu(_) => UiGraphics::Wgpu(
                window
                    .wgpu_mut()
                    .ok_or("the wgpu renderer has no wgpu window")?,
            ),
        };
        // The editor's pens come with its MCP server (EDITOR-1).
        host.annotations_mut().enabled = options.mcp_port.is_some();
        if options.editor {
            host.update_settings(|model| model.set_editor_mode(true));
        }
        let mut ui = ImguiFrontend::new(ui_graphics, &host, options, height)?;
        ui.set_clipboard(window.clipboard());
        // REQ-LOOP-077 is presentation, so it only exists for the renderer that presents here.
        let gpu_queue = match &graphics {
            Graphics::Gl { gl, .. } => Some(GpuFrameQueue::new(Rc::clone(gl))),
            #[cfg(feature = "wgpu")]
            Graphics::Wgpu(_) => None,
        };
        Ok(Self {
            options: options.clone(),
            ui,
            graphics,
            gpu_queue,
            input,
            source,
            audio,
            host,
            window,
            pacing,
            counter: FrameRateCounter::new(),
            cursor: CursorAutoHide::new(),
            state: RunState::new(renderer_name, version),
            last: Instant::now(),
            profiler: profiling.then(|| FrameProfiler::new(Instant::now())),
            clock: StageClock::new(profiling),
            rate_count: (0, 0),
            rate_since: Instant::now(),
            pipeline: Pipelined::default(),
            resolution: AutomaticResolution::default(),
            pending_state_load: options.slot.clone(),
            audio_logged_at: Instant::now(),
            mcp: options
                .mcp_port
                .and_then(|port| match crate::mcp::McpBridge::start(port) {
                    Ok(bridge) => {
                        log(&format!(
                            "MCP server: http://127.0.0.1:{}/mcp",
                            bridge.port()
                        ));
                        Some(bridge)
                    }
                    Err(error) => {
                        log(&format!("MCP server not started on port {port}: {error}"));
                        None
                    }
                }),
            editor: EditorState {
                on: false,
                checked: false,
                wants_project: false,
            },
        })
    }

    /// One displayed frame. Returns false once the application is to close.
    /// Logs the displayed and emulated frame rates every 5 seconds (the display's own rate
    /// seen, for the smooth scrolling between frames).
    fn log_rates(&mut self, emulated: u32) {
        self.rate_count.0 += 1;
        self.rate_count.1 += emulated;
        let seconds = self.rate_since.elapsed().as_secs_f64();
        if seconds >= 5.0 {
            let (shown, run) = self.rate_count;
            log(&format!(
                "frame rate: {:.1} displayed, {:.1} emulated per second",
                f64::from(shown) / seconds,
                f64::from(run) / seconds
            ));
            self.rate_count = (0, 0);
            self.rate_since = Instant::now();
        }
    }

    #[allow(clippy::too_many_lines)] // One host frame, its steps in their order.
    fn frame(&mut self) -> Result<bool, String> {
        self.follow_the_profiler_setting();
        self.reopen_the_audio_output();
        self.follow_the_editor_setting();
        self.load_the_asked_state();
        let frame_begin = self.clock.start();
        let busy_begin = Instant::now();
        let mark = self.clock.start();
        if self.pump_events() {
            return Ok(false);
        }
        self.state.frame_time = self.host.pacer.elapsed_seconds();
        self.host.display_rate = self.display_rate();
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
        let pipelined = self.next_frame_pipelined();
        self.host.defer_conversion = pipelined;
        let mark = self.clock.start();
        let emulated = self.host.tick(real, &snapshot);
        self.log_rates(emulated);
        self.state.emulated_now = emulated;
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
        let bounded = self.wait_for_the_gpu_queue();
        let overlap = if pipelined {
            self.draw_pipelined(emulated, width, height, real_f)?
        } else {
            self.draw(emulated, width, height, real_f)?;
            Duration::ZERO
        };
        self.close_the_gpu_queue(bounded);
        self.serve_the_editor();
        self.carry_out_ui_effects();
        let mark = self.clock.start();
        pump_audio(&mut self.host, &mut self.audio, real);
        self.clock.stop(Stage::Audio, mark);
        let budget = self.frame_budget();
        self.pipeline
            .governor
            .record(pipelined, busy_begin.elapsed(), overlap, budget);
        self.clock.add_thread(Thread::Main, busy_begin.elapsed());
        if pipelined {
            self.clock.count_pipelined();
        }
        self.follow_the_frames(busy_begin, busy_begin.elapsed());
        if self.host.quit_requested()
            || (self.options.smoke_test
                && smoke_run_is_over(&self.options, &self.host, &self.state))
        {
            if let Some(shot) = self.options.smoke_shot_path.as_deref() {
                self.graphics
                    .capture_window(&self.window, width, height, Path::new(shot));
            }
            return Ok(false);
        }
        let mark = self.clock.start();
        self.window.swap();
        self.clock.stop(Stage::Swap, mark);
        let mark = self.clock.start();
        self.window.pace();
        self.clock.stop(Stage::Pacer, mark);
        if let Some(begin) = frame_begin {
            self.end_profiled_frame(begin, emulated);
        }
        Ok(true)
    }

    /// Hands the frame that began at `begin` to the profiler and logs its report when a window
    /// is over: the stages, the threads, then the Lua hooks.
    fn end_profiled_frame(&mut self, begin: Instant, emulated: u32) {
        if self.profiler.is_none() {
            return;
        }
        self.log_audio_counters();
        let Some(profiler) = self.profiler.as_mut() else {
            return;
        };
        let times = self.clock.take();
        if let Some(report) = profiler.end_frame(&times, begin.elapsed(), emulated, Instant::now())
        {
            log(&report);
            log(profiler.thread_report());
            let hooks = self.host.take_lua_hook_times(true);
            log(&frame_profiler::lua_report(
                &hooks,
                profiler.reported_frames(),
            ));
            if let Some(line) = self.resolution_report() {
                log(&line);
            }
        }
    }

    /// Writes the audio counters to the log once a second while the profiler is on (Q-153 §6):
    /// the latency measured against the one asked for, the extremes of the ring's fill over the
    /// last second, the correction in force and the count of droughts.
    fn log_audio_counters(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.audio_logged_at) < Duration::from_secs(1) {
            return;
        }
        self.audio_logged_at = now;
        if let Some(line) = self.host.audio_report() {
            log(&format!("audio  {line}"));
        }
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

    /// Asks the wgpu window for the slang preset of the display mode: the one `ZAXNES_SHADER`
    /// names, otherwise the slot of the mode (2D or 3D, widescreen or not) after its fallbacks.
    /// The share of the window the shaders draw at (`video.shader_resolution`, a percentage).
    #[cfg(feature = "shaders")]
    #[allow(clippy::cast_precision_loss)] // 50 to 100.
    fn shader_resolution(&self) -> f32 {
        self.host
            .settings()
            .get(SettingId::VideoShaderResolution)
            .as_int() as f32
            / 100.0
    }

    #[cfg(feature = "shaders")]
    fn request_shader(&mut self) {
        use crate::ui::shader_panel::{ShaderReport, ShaderState};
        use zax_platform::slang_chain::{preset_from_environment, ShaderRequest, ShaderStatus};
        let scene = self.host.scene();
        let three_d = !scene.two_dimensional_mode;
        let wide = scene.margin > 0.0 || scene.margin_y > 0.0;
        // No tab is the one on screen without a game.
        let mode_slot = self
            .host
            .session()
            .map(|_| zax_settings::shader_slots::ShaderSlot::of(three_d, wide));
        let resolution = self.shader_resolution();
        let Some(display) = self.window.wgpu_mut() else {
            self.host.shader_report = ShaderReport {
                state: ShaderState::NoWgpu,
                mode_slot,
                ..ShaderReport::default()
            };
            return;
        };
        let request = match preset_from_environment() {
            Some(preset) => {
                let fills = zax_settings::shader_slots::fills_window(
                    zax_settings::shader_slots::AREA_AUTO,
                    &preset.to_string_lossy(),
                );
                Some(ShaderRequest {
                    preset,
                    lines: zax_settings::shader_slots::CONSOLE_LINES.unsigned_abs(),
                    parameters: Vec::new(),
                    picture_aspect: (!three_d && !fills).then(|| scene.picture_aspect()),
                    scene_aspect: None,
                    resolution,
                })
            }
            None => zax_settings::shader_slots::resolve(self.host.settings(), three_d, wide).map(
                |choice| {
                    // The 3D scene framed on the game's picture: drawn at its aspect, and placed
                    // in its rectangle by a preset that does not draw over the whole window.
                    let framed = three_d && choice.frames_picture;
                    let mut parameters = choice.parameters;
                    // The Mega Bezel guesses the screen's shape from the lines it reads: the
                    // console's 240 give a 4:3 TV, the window's lines of the framed 3D scene a
                    // square-pixel one. Framed in 4:3, the 3D TV takes the 2D picture's shape
                    // (4:3) unless the player picked a shape; the widescreen tabs start on PAR.
                    if framed
                        && !wide
                        && choice.preset.to_ascii_lowercase().contains("mega_bezel")
                        && !parameters
                            .iter()
                            .any(|(name, _)| name == "HSM_ASPECT_RATIO_MODE")
                    {
                        parameters.push(("HSM_ASPECT_RATIO_MODE".to_owned(), 2.0));
                    }
                    ShaderRequest {
                        preset: crate::shader_pack::resolve(
                            &self.host.paths().root,
                            &choice.preset,
                        ),
                        lines: choice.lines,
                        parameters,
                        picture_aspect: ((!three_d || framed) && !choice.fills_window)
                            .then(|| scene.picture_aspect()),
                        scene_aspect: framed.then(|| scene.picture_aspect()),
                        resolution,
                    }
                },
            ),
        };
        let shaders = display.shaders_mut();
        shaders.request(request.as_ref());
        if let Some(line) =
            shaders.update(usize::try_from(self.host.emulated_frames()).unwrap_or(0))
        {
            crate::app_log::line(&line);
        }
        let (state, preset, problem) = match shaders.status() {
            ShaderStatus::None => (ShaderState::None, String::new(), String::new()),
            ShaderStatus::Compiling(preset) => (
                ShaderState::Compiling,
                preset.display().to_string(),
                String::new(),
            ),
            ShaderStatus::Active(preset) => (
                ShaderState::Active,
                preset.display().to_string(),
                String::new(),
            ),
            ShaderStatus::Refused(preset, problem) => (
                ShaderState::Refused,
                preset.display().to_string(),
                problem.to_owned(),
            ),
        };
        let report = &mut self.host.shader_report;
        if report.state != state || report.preset != preset || report.mode_slot != mode_slot {
            *report = ShaderReport {
                state,
                preset,
                problem,
                mode_slot,
            };
        }
    }

    /// Sizes the view for the window and decides whether the render-skip rule keeps the
    /// previous image.
    fn plan_view(&mut self, emulated: u32, width: i32, height: i32) -> Result<ViewPlan, String> {
        let view_height = (height - to_i32_f(self.ui.chrome_height())).max(1);
        self.state.view_size = (width.max(1), view_height);
        let automatic = self.resolution.active;
        let factor = if automatic {
            self.resolution.governor.factor()
        } else {
            self.host.settings().res_scale_factor()
        };
        let (scene_width, scene_height) = render_scaling::scene_size(
            width,
            view_height,
            self.host.settings().render_method(),
            factor,
        );
        // A slang preset is written for a resolution of its own (the console's 240 lines in
        // 2D): the scene is drawn at the window's aspect but at that line count, and the preset
        // scales it up to the window. 0 lines keeps the window's own.
        #[cfg(feature = "shaders")]
        let scene_aspect = self
            .window
            .wgpu()
            .and_then(zax_platform::wgpu_display::WgpuDisplay::shader_scene_aspect);
        #[cfg(feature = "shaders")]
        {
            let scene = self.host.scene_mut();
            scene.picture_framed =
                scene_aspect.is_some() && !(scene.two_dimensional_mode && scene.picture_head_on);
        }
        #[cfg(feature = "shaders")]
        let (scene_width, scene_height) = match self
            .window
            .wgpu()
            .and_then(zax_platform::wgpu_display::WgpuDisplay::shader_lines)
        {
            // In 2D the preset gets the game picture alone, as in RetroArch: the scene is drawn
            // at the console's own pixels (240 lines) or a multiple, and the preset places it in
            // the window (a bezel around it, or the picture centred).
            Some(lines)
                if self.host.scene().two_dimensional_mode && self.host.scene().picture_head_on =>
            {
                render_scaling::picture_lines(self.host.scene().picture_size(), lines, view_height)
            }
            // The 3D scene framed on the game's picture: drawn at the picture's aspect, the
            // window's height or the lines asked for.
            Some(lines) if scene_aspect.is_some() => render_scaling::aspect_lines(
                scene_aspect.unwrap_or(1.0),
                lines,
                (width.max(1), view_height),
            ),
            Some(lines) if lines > 0 => {
                render_scaling::shader_lines(width.max(1), view_height, lines)
            }
            _ => (scene_width, scene_height),
        };
        let view = self
            .host
            .view_mut()
            .ok_or("the host has no view of the game")?;
        if automatic {
            // The scale changes every few frames: the buffers keep the room of the whole view,
            // and the scene is drawn into part of them.
            view.resize_within(scene_width, scene_height, (width.max(1), view_height))?;
        } else {
            view.resize(scene_width, scene_height)?;
        }
        let skip = render_skip::should_skip(
            self.host.scene().settings.skip_render,
            self.host.state() == EmulationState::Running,
            emulated,
            self.state.last_render_size == (scene_width, scene_height),
        ) && !self.state.picture_between;
        Ok(ViewPlan {
            view_height,
            size: (scene_width, scene_height),
            skip,
        })
    }

    /// Runs `operation` on the interface with the frame it is drawn for.
    fn with_ui_frame(
        &mut self,
        real: f32,
        operation: impl FnOnce(&mut ImguiFrontend, &mut UiFrame<'_>, Sizes, &DebugInfo<'_>),
    ) {
        let debug = DebugInfo {
            frames_per_second: self.counter.frames_per_second(),
            frame_milliseconds: self.counter.frame_milliseconds(),
            rendered_frames: self.state.rendered_frames,
            scene_frames: self.state.scene_frames,
            skipped_frames: self.state.skipped_frames,
            statistics: self.graphics.statistics(),
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
        if zax_platform::input::touch::offered() {
            let mode = zax_platform::input::touch::TouchPadMode::from_setting(
                frame
                    .host
                    .settings()
                    .get(zax_settings::catalogue::SettingId::InputTouchPad)
                    .as_int(),
            );
            #[allow(clippy::cast_precision_loss)] // window sizes are far below 2^24
            let size = |(width, height): (u32, u32)| [width as f32, height as f32];
            let playing = self.ui.game_on_screen();
            self.input
                .touch
                .update(mode, playing, size(sizes.window), size(sizes.drawable));
            let (taps, swipe) = (self.input.touch.take_taps(), self.input.touch.take_scroll());
            self.ui.set_touch_pad(self.input.touch.view(), taps, swipe);
        }
        operation(&mut self.ui, &mut frame, sizes, &debug);
    }

    /// `-slot=<n>` (AMD-053 REQ-UI-213) and `--load-state`: loads the slot once the game runs and
    /// the host accepts the load — after step 7 of REQ-LOOP-021, the state having gone to Running
    /// (the game loads in the background, and a slot of another game is refused).
    ///
    /// The slot is dropped at the first displayed frame where the game runs, whether the load was
    /// accepted or not: an empty slot is ignored, the game of `--rom` stays launched, and the fact
    /// is logged.
    ///
    /// `-slot=<n>` names a slot **of the game this run starts**. The clause of REQ-UI-213 about a
    /// slot belonging to another game has no referent here: there is no `-id=` option and no
    /// common slot index to look a game up in, a game's states being the files of its ROM.
    /// Q-102.
    // Q-102
    fn load_the_asked_state(&mut self) {
        let Some(slot) = self.pending_state_load.clone() else {
            return;
        };
        if self.host.state() != EmulationState::Running {
            return;
        }
        self.pending_state_load = None;
        let (command, args) = if slot == save_slots::QUICK_SLOT_ID {
            (Command::QuickLoad, CommandArgs::default())
        } else {
            let number = slot.parse::<i32>().unwrap_or(0);
            (Command::LoadSlot, CommandArgs::of_number(number))
        };
        if self.host.can_execute(command, &args) {
            self.host.enqueue(command, args);
            log(&format!("save state slot {slot} loaded (-slot)."));
        } else {
            // « Si le slot est vide, il est ignoré, le jeu de -id reste lancé et le fait est
            // journalisé. »
            log(&format!(
                "save state slot {slot} is empty; the option is ignored and the start-up carries \
                 on (REQ-UI-213)."
            ));
        }
    }

    /// Turns the frame profiler on or off with `ui.profiler` (`ZAXNES_PROFILE=1` keeps it on):
    /// off, the loop measures nothing and writes no profile line.
    /// AUDIO-2: `audio.classic_output` changed, so the device is closed and reopened under the
    /// other output — the choice takes effect when the device is (re)opened and at no other
    /// moment. One `bool` read and one branch per rendered frame; the rest only runs on the
    /// frame a user ticks the box.
    fn reopen_the_audio_output(&mut self) {
        if !self.host.take_audio_reopen() {
            return;
        }
        // The old device first, so that its callbacks have stopped before the host is told
        // anything: dropping it closes it and, on the default output, drops the stream with it.
        self.audio = AudioPath::Silent;
        self.host.attach_audio(None, false);
        let classic = self.host.settings().classic_audio_output();
        let latency_ms = self.host.settings().audio_latency_ms();
        let opened = open_audio(&self.options, self.window.sdl(), latency_ms, classic);
        let is_classic = opened.path.is_classic();
        self.audio = opened.path;
        self.host.attach_audio(opened.link, is_classic);
        log(if is_classic {
            "audio output: the classic one of v0.4.2."
        } else {
            "audio output: the default one."
        });
    }

    fn follow_the_profiler_setting(&mut self) {
        let wanted = frame_profiler::requested_by_environment()
            || self.host.settings().get(SettingId::UiProfiler).as_bool();
        if wanted == self.profiler.is_some() {
            return;
        }
        self.profiler = wanted.then(|| FrameProfiler::new(Instant::now()));
        self.clock = StageClock::new(wanted);
        self.host.set_profiling(wanted);
        self.host.take_lua_hook_times(wanted);
        log(if wanted {
            "frame profiler on"
        } else {
            "frame profiler off"
        });
    }

    /// Turns the editor mode on or off with `ui.editor_mode` (`--mcp` keeps its server on): on,
    /// the editor folder is laid out, the MCP server listens and the pens show; turned on while
    /// running (or by `--editor`), a game on a generic entry gets a blank Lua profile.
    fn follow_the_editor_setting(&mut self) {
        let wanted = self.host.settings().editor_mode() || self.options.mcp_port.is_some();
        let first = !self.editor.checked;
        self.editor.checked = true;
        if wanted == self.editor.on {
            return;
        }
        self.editor.on = wanted;
        self.host.annotations_mut().enabled = wanted;
        if !wanted {
            self.editor.wants_project = false;
            if self.options.mcp_port.is_none() && self.mcp.take().is_some() {
                log("editor mode off: MCP server closed");
            }
            return;
        }
        let folder = editor_mode::folder(&self.host.paths().root);
        match editor_mode::deploy(&folder) {
            Ok(true) => log(&format!("editor folder laid out: {}", folder.display())),
            Ok(false) => {}
            Err(error) => log(&format!("editor folder {}: {error}", folder.display())),
        }
        if self.mcp.is_none() {
            let port = self.options.mcp_port.unwrap_or(crate::mcp::DEFAULT_PORT);
            match crate::mcp::McpBridge::start(port) {
                Ok(bridge) => {
                    log(&format!(
                        "MCP server: http://127.0.0.1:{}/mcp",
                        bridge.port()
                    ));
                    self.mcp = Some(bridge);
                }
                Err(error) => log(&format!("MCP server not started on port {port}: {error}")),
            }
        }
        self.editor.wants_project = !first || self.options.editor;
    }

    /// The editor mode's work of a frame, after it is drawn: the MCP calls, a blank profile for a game on a generic entry once it
    /// runs, and the requests of the menu.
    fn serve_the_editor(&mut self) {
        // Its MCP server, after the frame is drawn so a screenshot is of it.
        if let Some(mcp) = &mut self.mcp {
            let port = mcp.port();
            let info =
                move |host: &AppHost| editor_mode::agent_prompt(&prompt_context(host, Some(port)));
            mcp.serve(&mut self.host, &info);
        }
        if !self.editor.on {
            return;
        }
        if self.editor.wants_project
            && !self.host.is_loading()
            && self.host.current_game_id().is_some()
        {
            self.editor.wants_project = false;
            if self.host.current_game_is_generic() {
                self.new_lua_profile();
            }
        }
        match self.host.take_editor_request() {
            Some(EditorRequest::NewLuaProfile) => self.new_lua_profile(),
            Some(EditorRequest::CopyAgentPrompt) => {
                let prompt = editor_mode::agent_prompt(&self.prompt_context());
                self.copy_prompt(&prompt);
            }
            None => {}
        }
    }

    /// What the agent's prompt says of this session.
    fn prompt_context(&self) -> editor_mode::PromptContext {
        prompt_context(
            &self.host,
            self.mcp.as_ref().map(crate::mcp::McpBridge::port),
        )
    }
}

/// What the agent's prompt says of this session: the game on `host`, the MCP server on
/// `mcp_port` when it runs (also the `editor_info` tool's answer).
fn prompt_context(host: &AppHost, mcp_port: Option<u16>) -> editor_mode::PromptContext {
    {
        let folder = editor_mode::folder(&host.paths().root);
        let game_id = host.current_game_id();
        let rom = game_id.and_then(|id| host.games().rom_path(id));
        let game = game_id.zip(rom).map(|(id, rom)| {
            let title = host.games().title(id).unwrap_or_else(|| id.to_owned());
            (title, rom)
        });
        let project = game_id.and_then(|id| editor_mode::project_of(&folder, id));
        let rom_identity = game.as_ref().and_then(|(_, rom)| {
            let image = std::fs::read(rom).ok()?;
            let mapper = image
                .get(6..8)
                .filter(|_| image.starts_with(b"NES\x1a"))
                .map(|flags| (flags[0] >> 4) | (flags[1] & 0xF0));
            let region = host.session().map(|session| session.core().region());
            Some(format!(
                "data CRC {:08x} (No-Intro), mapper {}, {}",
                zax_nes::builtin_patches::data_crc(&image),
                mapper.map_or_else(|| "?".to_owned(), |m| m.to_string()),
                region.map_or_else(|| "?".to_owned(), |r| format!("{r:?}")),
            ))
        });
        let movie = host
            .movie_file()
            .zip(host.movie_position())
            .map(|(file, (_, length))| (file.to_path_buf(), length));
        let harness = ["zax-harness.exe", "zax-harness"]
            .iter()
            .map(|name| host.paths().executable_directory.join(name))
            .find(|path| path.is_file());
        editor_mode::PromptContext {
            folder,
            game,
            project,
            rom_identity,
            movie,
            mcp_url: mcp_port.map(|port| format!("http://127.0.0.1:{port}/mcp")),
            harness,
            french: strings::language() == UiLanguage::French,
        }
    }
}

impl Running {
    /// Puts `prompt` into the clipboard, and says so.
    fn copy_prompt(&mut self, prompt: &str) {
        let french = strings::language() == UiLanguage::French;
        match self.window.clipboard().set_clipboard_text(prompt) {
            Ok(()) => self.host.show_notification(
                if french {
                    "Prompt pour l'agent IA copié dans le presse-papiers"
                } else {
                    "Prompt for the AI agent copied to the clipboard"
                },
                false,
            ),
            Err(error) => {
                log(&format!("clipboard: {error}"));
                self.host.show_notification(&error, true);
            }
        }
    }

    /// A blank Lua profile for the game on screen: a project in the editor folder, its profile
    /// built and imported, the game started again on it, and the agent's prompt written into the
    /// project and copied.
    fn new_lua_profile(&mut self) {
        let french = strings::language() == UiLanguage::French;
        let Some(game_id) = self.host.current_game_id().map(str::to_owned) else {
            return;
        };
        let Some(rom) = self.host.games().rom_path(&game_id) else {
            return;
        };
        let movie = self.host.movie_file().map(Path::to_path_buf);
        let name = Path::new(&rom).file_stem().map_or_else(
            || "game".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        let folder = editor_mode::folder(&self.host.paths().root);
        // A game already on a project's profile has nothing to make: its prompt is copied.
        if editor_mode::project_of(&folder, &game_id).is_some() {
            let prompt = editor_mode::agent_prompt(&self.prompt_context());
            self.copy_prompt(&prompt);
            return;
        }
        // A game with a profile of its own gets a project built on it, else a blank one.
        let source = self.host.games().profile_file(&game_id);
        let made = editor_mode::deploy(&folder)
            .map_err(|error| format!("{}: {error}", folder.display()))
            .and_then(|_| match &source {
                Some(source) => editor_mode::export_project(&folder, &name, source),
                None => editor_mode::create_project(&folder, &name),
            });
        let project = match made {
            Ok(project) => project,
            Err(message) => {
                log(&format!("new Lua profile: {message}"));
                self.host.show_notification(&message, true);
                return;
            }
        };
        let result = self.host.games_mut().import_profile(&project.profile_file);
        let Some(profile) = result.profile.filter(|_| {
            matches!(
                result.outcome,
                ProfileImportOutcome::Added | ProfileImportOutcome::Replaced
            )
        }) else {
            log(&format!("new Lua profile: {}", result.message));
            self.host.show_notification(&result.message, true);
            return;
        };
        log(&format!(
            "new Lua profile '{}' for {rom}: project {}",
            project.title,
            project.folder.display()
        ));
        if let Some(movie) = movie {
            self.host.play_movie_after_launch(&movie.to_string_lossy());
        }
        self.host.launch_rom_with_profile(&rom, &profile.id);
        let mut context = self.prompt_context();
        context.game = Some((project.title.clone(), rom));
        context.project = Some(project.folder.clone());
        let prompt = editor_mode::agent_prompt(&context);
        if let Err(error) = editor_mode::write_prompt(&project.folder, &prompt) {
            log(&format!("PROMPT.txt: {error}"));
        }
        self.copy_prompt(&prompt);
        self.host.show_notification(
            &if french {
                format!(
                    "Profil Lua créé : {}. Le prompt de l'agent est copié.",
                    project.folder.display()
                )
            } else {
                format!(
                    "Lua profile made: {}. The agent's prompt is copied.",
                    project.folder.display()
                )
            },
            false,
        );
    }

    /// AMD-052 REQ-LOOP-077: waits until the GPU holds fewer rendered frames than the depth in
    /// force. It runs after the emulated frames of the tick and before the first drawing
    /// command, so that it cannot shift the cadence of the emulated frames (REQ-LOOP-010).
    /// Returns whether this frame is bounded at all, for [`Self::close_the_gpu_queue`].
    fn wait_for_the_gpu_queue(&mut self) -> bool {
        let Some(depth) = self.bound_gpu_queue() else {
            return false;
        };
        if let Some(queue) = self.gpu_queue.as_mut() {
            let mark = self.clock.start();
            queue.begin_frame(depth);
            self.clock.stop(Stage::Wait, mark);
            return true;
        }
        false
    }

    /// Places the fence the next [`Self::wait_for_the_gpu_queue`] waits on, after the frame's
    /// last drawing command (AMD-052 REQ-LOOP-077).
    fn close_the_gpu_queue(&mut self, bounded: bool) {
        if bounded {
            if let Some(queue) = self.gpu_queue.as_mut() {
                queue.end_frame();
            }
        }
    }

    /// The depth the GPU frame queue of AMD-052 REQ-LOOP-077 is to be bounded at this frame, or
    /// `None` when no fence is to be issued at all.
    ///
    /// **Handheld (D).** On `PortMaster` and Android the default depth of 2 issues **no fence
    /// call at all** and keeps the behaviour the device has today (owner decision): that depth
    /// is the driver's own queue, and the Mali keeps three frames of its own, so bounding it
    /// could cost frames rather than latency. The depth of 1 — the one a player picks to lower
    /// the latency — works on every build, there as on the desktop.
    fn bound_gpu_queue(&self) -> Option<i32> {
        let depth = self
            .host
            .settings()
            .max_queued_frames()
            .clamp(zax_render::sync::MIN_DEPTH, zax_render::sync::MAX_DEPTH);
        if self.options.handheld && depth >= zax_render::sync::MAX_DEPTH {
            return None;
        }
        Some(depth)
    }

    /// Whether the next frame is pipelined: `emu.pipeline`, for a game running in 3D whose
    /// scripts leave the core alone, with no modal window and no screenshot plan.
    fn next_frame_pipelined(&mut self) -> bool {
        let allows = self.host.can_defer_conversion()
            && !self.host.showing_two_dimensional()
            && !self.ui.is_modal()
            && !self.ui.has_shots();
        self.pipeline
            .governor
            .next_frame(PipelineMode::of(self.host.settings()), allows)
    }

    /// The automatic resolution: the outcome of the frame before (on time, late because of the
    /// drawing, late because of the work) goes to the governor, once this frame, which began at
    /// `began` and worked `work` before its swap, tells how long it lasted.
    fn follow_the_frames(&mut self, began: Instant, work: Duration) {
        let settings = self.host.settings();
        let budget = self.paced_budget();
        let wanted = settings.get(SettingId::VideoDynamicResolution).as_bool()
            && render_scaling::scale_applies(settings.render_method())
            && self.graphics.is_gl()
            && budget.is_some();
        let resolution = &mut self.resolution;
        if wanted != resolution.active {
            resolution.active = wanted;
            resolution.previous = None;
            if wanted {
                log(&format!(
                    "automatic resolution: on, from scale {:.2}",
                    resolution.governor.factor()
                ));
            }
        }
        let Some(budget) = budget.filter(|_| resolution.active) else {
            return;
        };
        let Some((start, previous_work)) = resolution.previous.replace((began, work)) else {
            return;
        };
        let outcome =
            render_scaling::frame_outcome(began.duration_since(start), previous_work, budget);
        resolution.governor.record(outcome);
        let scale = resolution.governor.factor();
        let window = resolution.window.get_or_insert((scale, scale, 0, 0));
        window.0 = window.0.min(scale);
        window.1 = window.1.max(scale);
        match outcome {
            render_scaling::FrameOutcome::LateDrawing => window.2 += 1,
            render_scaling::FrameOutcome::LateWork => window.3 += 1,
            render_scaling::FrameOutcome::OnTime => {}
        }
    }

    /// The resolution line of a profiler report while the automatic resolution is in force:
    /// the scales of the window and the late frames. Starts the next window.
    fn resolution_report(&mut self) -> Option<String> {
        let resolution = &mut self.resolution;
        let window = resolution.window.take();
        if !resolution.active {
            return None;
        }
        let (lowest, highest, drawing, work) = window.unwrap_or_default();
        Some(format!(
            "profile resolution: scale {:.2} now, {lowest:.2} to {highest:.2} in the window | \
             late frames: {drawing} because of the drawing, {work} because of the work",
            resolution.governor.factor()
        ))
    }

    /// The time a displayed frame has, as the pacing in force sets it: the swap interval at the
    /// display's refresh rate, or the frame cap's period; none when nothing paces the frames.
    fn paced_budget(&self) -> Option<Duration> {
        let interval = self.window.vsync_interval();
        if interval > 0 {
            let refresh = self
                .window
                .window()
                .display_mode()
                .map_or(60, |mode| mode.refresh_rate)
                .max(1);
            return Some(Duration::from_secs_f64(
                f64::from(interval) / f64::from(refresh),
            ));
        }
        self.window.frame_cap()
    }

    /// The frames the display shows a second: the display's refresh over the swap interval, or
    /// the frame cap's rate (a high one when nothing caps the frames).
    fn display_rate(&self) -> f64 {
        let interval = self.window.vsync_interval();
        if interval > 0 {
            let refresh = self
                .window
                .window()
                .display_mode()
                .map_or(60, |mode| mode.refresh_rate)
                .max(1);
            return f64::from(refresh) / f64::from(interval);
        }
        self.window
            .frame_cap()
            .map_or(1000.0, |period| 1.0 / period.as_secs_f64().max(0.001))
    }

    /// The time one displayed frame may take: the swap interval at the display's refresh rate.
    fn frame_budget(&self) -> Duration {
        let refresh = self
            .window
            .window()
            .display_mode()
            .map_or(60, |mode| mode.refresh_rate)
            .max(1);
        // The interval in force, which `video.half_rate_3d` may double.
        let interval = self.window.vsync_interval().max(1);
        Duration::from_secs_f64(f64::from(interval) / f64::from(refresh))
    }

    /// The pipelined frame: the interface is built first, then the conversion the tick left
    /// over runs on its own thread while the list of the frame before is drawn with the scene
    /// as it stood. Returns how long the conversion ran beside the drawing.
    fn draw_pipelined(
        &mut self,
        emulated: u32,
        width: i32,
        height: i32,
        real: f32,
    ) -> Result<Duration, String> {
        let ViewPlan {
            view_height,
            size: (scene_width, scene_height),
            skip,
        } = self.plan_view(emulated, width, height)?;
        let mark = self.clock.start();
        self.with_ui_frame(real, |ui, frame, sizes, debug| {
            ui.build(frame, sizes, debug);
        });
        self.clock.stop(Stage::Ui, mark);
        // The scene as the frame drawn now left it; the conversion thread changes the live one.
        let scene = self.host.scene().clone();
        let Some(DeferredParts {
            session,
            settings,
            scene: live_scene,
            view,
        }) = self.host.deferred_parts()
        else {
            self.graphics.draw_window(
                &mut self.window,
                None,
                &mut self.ui,
                (width, height),
                view_height,
            );
            return Ok(Duration::ZERO);
        };
        let view = view.ok_or("the host has no view of the game")?;
        if std::mem::take(&mut self.pipeline.list_waiting) {
            session.exchange_draw_list(&mut self.pipeline.displayed);
        }
        // Without a new frame the conversion thread has nothing to do.
        let convert = session.has_captured_frame();
        self.pipeline.list_waiting = convert;
        let mut converting = StageClock::new(self.clock.enabled());
        let converting_clock = &mut converting;
        let (graphics, clock, ui, window, displayed) = (
            &mut self.graphics,
            &mut self.clock,
            &mut self.ui,
            &mut self.window,
            &self.pipeline.displayed,
        );
        let shadow_reported = &mut self.state.shadow_map_reported;
        let (converted, wait, drawn) = std::thread::scope(|threads| {
            let conversion = convert.then(|| {
                threads.spawn(move || {
                    let started = Instant::now();
                    session.convert_deferred(settings, live_scene, converting_clock);
                    started.elapsed()
                })
            });
            let drawn = if skip {
                Ok(())
            } else {
                let frame = SceneFrame {
                    draw_list: Some(displayed),
                    nes_picture: &[],
                    nes_picture_height: 0,
                };
                draw_scene(graphics, view, &scene, &frame, shadow_reported, clock)
            };
            let mark = clock.start();
            graphics.draw_window(window, Some(view), ui, (width, height), view_height);
            clock.stop(Stage::Ui, mark);
            let waiting = Instant::now();
            let converted = conversion.map_or(Duration::ZERO, |conversion| {
                conversion
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            });
            (converted, waiting.elapsed(), drawn)
        });
        drawn?;
        self.clock.merge(&converting.take());
        self.clock.add(Stage::Wait, wait);
        self.clock.add_thread(Thread::Conversion, converted);
        if skip {
            self.state.skipped_frames += 1;
        } else {
            self.state.last_render_size = (scene_width, scene_height);
            self.state.scene_frames += 1;
        }
        Ok(converted.saturating_sub(wait))
    }

    /// Draws the scene into the offscreen buffer (unless the render-skip rule keeps the
    /// previous image), blits it under the interface and draws the interface.
    fn draw(&mut self, emulated: u32, width: i32, height: i32, real: f32) -> Result<(), String> {
        #[cfg(feature = "shaders")]
        self.request_shader();
        let ViewPlan {
            view_height,
            size: (scene_width, scene_height),
            skip,
        } = self.plan_view(emulated, width, height)?;
        if skip {
            self.state.skipped_frames += 1;
        } else {
            render_scene(
                &mut self.graphics,
                &mut self.host,
                &mut self.state,
                &mut self.clock,
            )?;
            self.state.last_render_size = (scene_width, scene_height);
            self.state.scene_frames += 1;
            // The engine's list is the one drawn now: a pipelined frame that follows draws it
            // again while the next is built.
            self.pipeline.list_waiting = !self.host.showing_two_dimensional();
        }
        let mark = self.clock.start();
        self.with_ui_frame(real, |ui, frame, sizes, debug| {
            ui.build(frame, sizes, debug);
        });
        self.graphics.draw_window(
            &mut self.window,
            self.host.view(),
            &mut self.ui,
            (width, height),
            view_height,
        );
        let shot = self.ui.advance_shots();
        if let Some(path) = shot.capture {
            self.graphics
                .capture_window(&self.window, width, height, &path);
        }
        if shot.finished {
            self.host.enqueue(Command::Quit, CommandArgs::default());
        }
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
    // The economical defaults of a PortMaster handheld; the Android package keeps the desktop's
    // (owner request: some devices are powerful, each player tunes for his own).
    if options.handheld && !cfg!(target_os = "android") {
        handheld::defaults::apply(&mut model);
    }
    if cfg!(target_os = "android") {
        handheld::defaults::apply_android(&mut model);
    }
    // Rust-only (owner request): the region comes from the ROM header unless the user turned the
    // automatic mode off, so a European dump runs at 50 Hz (`emu.auto_tv_mode` is off in C#).
    model.set_environment_default(SettingId::EmuAutoTvMode, SettingValue::Bool(true));
    if let Some(locale) = options.system_locale.as_deref() {
        // Only the default: a saved `ui.language` (read next) always wins.
        model.set_environment_default(
            SettingId::UiLanguage,
            UiLanguage::from_locale(locale).code(),
        );
    }
    store.load(&mut model, OffsetDateTime::now_utc());
    // Rust-only (owner request): the widescreen, its margin and the built-in patches are options
    // of each game now, with no global choice left; a value an earlier version saved goes.
    for id in [
        SettingId::VideoWidescreen,
        SettingId::VideoWidescreenExtra,
        SettingId::VideoWidescreenAdvanced,
        SettingId::EmuBuiltinPatches,
        SettingId::EmuForcedPatch,
    ] {
        model.reset(id);
    }
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

#[allow(clippy::struct_excessive_bools)] // The independent flags a launch carries.
struct HostParts<'a> {
    paths: &'a UserDataPaths,
    model: SettingsModel,
    scene: SceneState,
    catalog: LibraryGameCatalog,
    store: SettingsStore,
    asset_root: Option<&'a Path>,
    sample_rate: u32,
    audio: Option<AudioLink>,
    /// Whether the device that opened is the classic output of `v0.4.2` (AUDIO-2).
    classic_audio: bool,
    view: SceneView,
    /// `-noresume` (AMD-053 REQ-UI-213).
    suspend_resume: bool,
    /// `-emustats` (AMD-053 REQ-UI-213).
    emu_stats: bool,
    /// Whether the resolved tiles of a "3dh" profile are kept between runs (AMD-062
    /// REQ-PROF-082); `--no-tile-cache` turns it off.
    tile_cache: bool,
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
    options.audio = parts.audio;
    options.classic_audio = parts.classic_audio;
    options.log = Some(Box::new(log));
    options.settings_store = Some(parts.store);
    options.confirm_with_interface = true;
    options.view = Some(parts.view);
    options.suspend_resume = parts.suspend_resume;
    // AMD-053 REQ-UI-213: the per-frame lines go to the log file only, which a normal launch
    // does not share with a parent console. Absent, the frame loop does nothing extra.
    options.emu_stats = parts
        .emu_stats
        .then(|| Box::new(app_log::file_line) as crate::emulation::host::LogFn);
    // AMD-062 REQ-PROF-082: `--no-tile-cache` starts from an empty resolved set.
    options.tile_cache = parts.tile_cache;
    EmulationHost::new(options)
}

/// The user folder, and the file copy of the log opened in it. On a handheld, a root the
/// command line names that refuses writes is replaced by the per-user folder when that one
/// accepts them (Rust-only: the settings and the saves would otherwise be lost silently).
/// `FlappyZaxNes` into the ROM folder of the options (or the user data's `roms`) and the user's
/// profiles, when this version is not there yet; the path of its ROM when it was written now.
fn install_demo_game(paths: &UserDataPaths, model: &SettingsModel) -> Option<PathBuf> {
    let folder = model
        .get(SettingId::UiRomsFolder)
        .as_text()
        .trim()
        .to_owned();
    let roms = if folder.is_empty() {
        paths.root.join("roms")
    } else {
        PathBuf::from(folder)
    };
    match demo_game::install(paths, &roms) {
        Ok(rom) => {
            if let Some(rom) = &rom {
                log(&format!("FlappyZaxNes installed: '{}'.", rom.display()));
            }
            rom
        }
        Err(error) => {
            log(&format!("FlappyZaxNes could not be installed: {error}"));
            None
        }
    }
}

/// The slang shader pack into the user folder, when this version is not there yet.
#[cfg(feature = "shaders")]
fn install_shader_pack(paths: &UserDataPaths) {
    if !paths.is_writable {
        return;
    }
    let started = std::time::Instant::now();
    match crate::shader_pack::install(&paths.root) {
        Ok(true) => log(&format!(
            "Shader pack {} written to '{}' in {} ms.",
            crate::shader_pack::version(),
            crate::shader_pack::folder(&paths.root).display(),
            started.elapsed().as_millis()
        )),
        Ok(false) => {}
        Err(error) => log(&format!("The shader pack could not be written: {error}")),
    }
}

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
    if explicit.is_none()
        && paths.kind == crate::user_data_paths::UserDataRootKind::LocalAppData
        && paths
            .executable_directory
            .join(crate::user_data_paths::PORTABLE_MARKER_NAME)
            .is_file()
    {
        log(&format!(
            "the portable folder next to the program refuses writes; the user data is in '{}'.",
            paths.root.display()
        ));
    }
    // AMD-029 §2: the first run after the rename seeds the new root from the old one; the first
    // portable start, from the per-user root.
    if let Some(legacy) = &paths.migrated_from {
        log(&format!(
            "user data copied from '{}' to '{}'; the old folder was left untouched.",
            legacy.display(),
            paths.root.display()
        ));
    }
    open_log_file(&paths);
    paths
}

/// The start-up report of the display: SDL's driver and mode, the GL context it created and the
/// GL strings, with a warning when the renderer is a software one, or the wgpu adapter. Returns
/// the renderer's name and version for the statistics.
fn log_display(window: &HostWindow) -> (String, String) {
    log(&window.display_report());
    #[cfg(feature = "wgpu")]
    if let Some(display) = window.wgpu() {
        log(&display.report());
        return (display.adapter_name(), "wgpu".to_owned());
    }
    let gl = window.glow_context();
    log(&window.gl_config_report());
    log(&framebuffer::gl_report(&gl));
    if let Some(warning) = framebuffer::software_renderer_warning(&gl) {
        log(&warning);
    }
    framebuffer::gl_strings(&gl)
}

/// The window for the graphics API `video.backend` (or `--backend`) asks: a GL one, or a wgpu
/// one in the desktop builds. A wgpu window that cannot be made (no Vulkan driver, say) falls
/// back to GL, and the log says why.
fn open_window(
    options: &AppOptions,
    model: &SettingsModel,
    settings: &HostWindowSettings,
) -> Result<HostWindow, String> {
    let asked = Backend::from_setting(
        options
            .backend
            .unwrap_or_else(|| model.get(SettingId::VideoBackend).as_int()),
    );
    let (backend, why) = asked.available();
    if let Some(why) = why {
        log(&format!("{asked:?} asked: {why}; using {backend:?}."));
    }
    let graphics = backend.window_graphics();
    if graphics == WindowGraphics::Gl {
        return HostWindow::create(settings).map_err(|error| error.to_string());
    }
    match HostWindow::create(&HostWindowSettings {
        graphics,
        ..settings.clone()
    }) {
        Ok(window) => Ok(window),
        Err(error) => {
            log(&format!(
                "{backend:?} could not be opened ({error}); using OpenGL."
            ));
            HostWindow::create(settings).map_err(|error| error.to_string())
        }
    }
}

/// The scene renderer of the window's graphics and the picture it draws into.
fn open_graphics(
    window: &HostWindow,
    asset_root: Option<&Path>,
    paths: &UserDataPaths,
    group: bool,
    (width, height): (i32, i32),
) -> Result<(Graphics, SceneView), String> {
    #[cfg(feature = "wgpu")]
    if let Some(display) = window.wgpu() {
        if asset_root.is_some() {
            log("the skybox cubemaps of an asset folder are a GL-only option: the wgpu renderer draws the procedural skies.");
        }
        let mut renderer = zax_render::wgpu_backend::WgpuSceneRenderer::new(
            display.adapter(),
            display.device().clone(),
            display.queue().clone(),
            crate::graphics::WgpuView::FORMAT,
        )
        .map_err(|error| error.to_string())?;
        renderer.set_group_opaque_draws(group);
        let view = crate::graphics::WgpuView::new(display.device(), display.queue(), width, height);
        return Ok((Graphics::Wgpu(Box::new(renderer)), SceneView::Wgpu(view)));
    }
    let gl = window.glow_context();
    let view = OffscreenFramebuffer::new(Rc::clone(&gl), width, height)
        .map_err(|error| error.to_string())?;
    // The linked programs are kept in the user folder, where the next start finds them.
    let mut renderer = open_renderer(&gl, asset_root, paths)?;
    renderer.set_group_opaque_draws(group);
    Ok((Graphics::Gl { gl, renderer }, SceneView::Gl(view)))
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

/// The Rust-only choices of the host's loop: forced catch-up, rewind, emulation thread and
/// emulation ahead.
fn configure_pacing(host: &mut AppHost, options: &AppOptions) {
    host.catch_up = options.catch_up;
    // No rewind on a PortMaster handheld (owner request): its snapshot every 60 frames costs a
    // spike. The Android package has it, as the desktop (owner request).
    host.rollback_enabled = !options.handheld || cfg!(target_os = "android");
    host.separate_core = separate_core(options);
    // RO-2 part 4: a handheld build, `--handheld`, or a desktop opened in its console mode. It
    // is the launch-time choice of interface; F1 switching the presentation later does not make
    // a desktop a handheld.
    host.console_interface = options.handheld || options.console;
    // `ZAXNES_CORE_AHEAD=0` keeps the next frame waiting for the scripts of the last one.
    host.core_ahead = std::env::var("ZAXNES_CORE_AHEAD").map_or(true, |value| value.trim() != "0");
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

/// Whether each emulated frame runs on its own thread while the previous one is drawn:
/// `ZAXNES_SEPARATE_CORE=1` or `0`, else on a handheld only (the game reads its inputs one
/// displayed frame later).
fn separate_core(options: &AppOptions) -> bool {
    match std::env::var(SEPARATE_CORE_VARIABLE)
        .as_deref()
        .map(str::trim)
    {
        Ok("1") => true,
        Ok("0") => false,
        _ => options.handheld,
    }
}

/// The variable that forces [`separate_core`].
const SEPARATE_CORE_VARIABLE: &str = "ZAXNES_SEPARATE_CORE";

/// The variable that forces [`group_draws`].
const GROUP_DRAWS_VARIABLE: &str = "ZAXNES_GROUP_DRAWS";

/// The saved bindings, a gamepad left at its defaults taking those of the handheld: the desktop's
/// pads play as in its console mode (owner request), the menu button, X, Y and L2 being the
/// console interface's, its face buttons where a handheld has them.
fn load_bindings(store: &mut FileInputBindingStore, device: bool) -> crate::input::InputBindings {
    let mut bindings = crate::input::InputBindings::new();
    store.load(&mut bindings);
    let loaded = bindings.revision();
    if cfg!(target_os = "android") {
        bindings.use_android_defaults();
    } else if device {
        bindings.use_handheld_defaults();
    } else {
        bindings.use_desktop_console_defaults();
    }
    // A pad moved to the new defaults is written, so that the file says what it plays with.
    if bindings.revision() != loaded {
        store.save(&bindings);
    }
    bindings
}

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
    separate_core: bool,
    smooth_gc: bool,
) {
    log(&frame_profiler::startup_report(
        &format!(
            "render scale {}, automatic resolution {}, render method {}, shadows {}, \
             MSAA setting {}, vsync {}, frame rate setting {}",
            model.res_scale_factor(),
            model.get(SettingId::VideoDynamicResolution).as_bool(),
            model.render_method(),
            model.shadow(),
            model.anti_alias(),
            model.vsync(),
            model.frame_rate()
        ),
        &format!("{pacing}, catch-up {catch_up:?}, grouped draws {group_draws}, separate core {separate_core}, Lua GC {}", if smooth_gc { "smooth" } else { "original" }),
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

/// The view of one displayed frame: its height under the menu bar, the scene size and whether
/// the render-skip rule keeps the previous image.
struct ViewPlan {
    view_height: i32,
    size: (i32, i32),
    skip: bool,
}

struct RunState {
    rendered_frames: u64,
    scene_frames: u64,
    skipped_frames: u64,
    last_render_size: (i32, i32),
    /// The size of the view the scene fills, before any resolution scale: its ratio is the
    /// picture's, which the widescreen margins follow (the scene's own size moves with the
    /// automatic resolution, and its rounding with it).
    view_size: (i32, i32),
    renderer_name: String,
    version: String,
    shadow_map_reported: bool,
    drag_origin: Option<[f32; 2]>,
    /// The pull on the head-on 2D picture (`crate::head_on_pull`).
    head_on_pull: crate::head_on_pull::HeadOnPull,
    /// Whether the game was in 2D last frame: going into it puts the picture head on.
    was_two_dimensional: bool,
    picture: Vec<u8>,
    /// Its rows.
    picture_height: usize,
    /// Whether its playfield is drawn between two frames (the experimental 120 Hz scrolling):
    /// every display frame is then drawn, a new emulated frame or not.
    picture_between: bool,
    /// The emulated frames of this display frame, and the scrolling debug log
    /// (`ZAXNES_SCROLL_LOG`, a file path): one line per display frame.
    emulated_now: u32,
    /// When this display frame began, on the frame pacer's clock.
    frame_time: f64,
    scroll_log: Option<std::fs::File>,
}

impl RunState {
    fn new(renderer_name: String, version: String) -> Self {
        Self {
            rendered_frames: 0,
            scene_frames: 0,
            skipped_frames: 0,
            last_render_size: (0, 0),
            view_size: (0, 0),
            renderer_name,
            version,
            shadow_map_reported: false,
            drag_origin: None,
            head_on_pull: crate::head_on_pull::HeadOnPull::new(),
            was_two_dimensional: false,
            picture: Vec::new(),
            picture_height: 0,
            picture_between: false,
            emulated_now: 0,
            frame_time: 0.0,
            scroll_log: std::env::var_os("ZAXNES_SCROLL_LOG")
                .and_then(|path| std::fs::File::create(path).ok()),
        }
    }
}

/// The view of the smooth scrolling (`emu.smooth_scroll_view`): the band it slides of `picture`
/// (NES colour indexes, `height` rows, its vertical margin centred) in reds, its brightness kept,
/// the sprites it keeps still (`overlay`) in greens; the blacks stay black.
fn tint_smooth_band(
    picture: &mut [u8],
    height: usize,
    (start, end): (i32, i32),
    overlay: &mut [u8],
) {
    const HIDDEN: u8 = 0xFF;
    let recolour = |pixel: &mut u8, hue: u8| {
        if *pixel != HIDDEN && *pixel & 0x0F < 0x0D {
            *pixel = (*pixel & !0x0F) | hue;
        }
    };
    let width = picture.len() / height.max(1);
    let margin_y = height.saturating_sub(240) / 2;
    let line = |screen: i32| usize::try_from(screen).map_or(0, |screen| margin_y + screen);
    let from = if start <= 0 { 0 } else { line(start) };
    let to = if end >= 240 { height } else { line(end) };
    for row in picture.chunks_mut(width.max(1)).take(to).skip(from) {
        for pixel in row.iter_mut() {
            recolour(pixel, 0x06);
        }
    }
    for pixel in overlay.iter_mut() {
        recolour(pixel, 0x0A);
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
    scene.settings.set_anti_alias(model.anti_alias());
    scene.camera.snap_to_setpoints();
}

/// Which output is open, and everything the main thread needs of it (AUDIO-2).
///
/// The choice is made **once**, when the device opens, from `audio.classic_output`; nothing reads
/// that setting again. What it costs the loop afterwards is the one `match` of [`pump_audio`] per
/// rendered frame — no second device, no dispatch through a trait object, nothing per sample.
enum AudioPath {
    /// No device: either `--no-audio` or a machine SDL found no sound card on.
    Silent,
    /// The default output (AUDIO-1, Q-153): SDL's own thread pulls the stream, which reads the
    /// core's ring itself, and the main thread only keeps the programme ring a few tens of
    /// milliseconds ahead.
    Pull {
        /// The open device. Nothing reads it: it is held so that it outlives the stream it pulls
        /// from, since dropping it closes the device and stops the callbacks.
        #[allow(dead_code)]
        device: AudioOutput,
        /// The main thread's end of the stream's programme ring (menu music, interface sounds).
        program: ProgramWriter,
        /// The block the mixer produces the programme into, allocated once.
        block: Vec<f32>,
    },
    /// The classic output of `v0.4.2` (`audio.classic_output`): the main thread tops an SDL queue
    /// of four 512-frame blocks up once per rendered frame, mixing the console into each.
    Classic { device: ClassicAudioOutput },
}

impl AudioPath {
    /// Whether this is the classic output, which is what the host branches on.
    const fn is_classic(&self) -> bool {
        matches!(self, Self::Classic { .. })
    }
}

/// What opening the audio device produced: the path the loop keeps, the rate it opened at, and
/// the handles the host needs on the stream when the default output is the one that opened.
struct OpenedAudio {
    path: AudioPath,
    sample_rate: u32,
    link: Option<AudioLink>,
}

/// Opens the output `classic` asks for: `false` — the default — the pull output of AUDIO-1,
/// `true` the queue of `v0.4.2` (AUDIO-2). `latency_ms` belongs to the pull output alone; the
/// classic one has the fixed four blocks it always had.
fn open_audio(
    options: &AppOptions,
    sdl: &sdl2::Sdl,
    latency_ms: f32,
    classic: bool,
) -> OpenedAudio {
    let default_rate = u32::try_from(PREFERRED_SAMPLE_RATE).unwrap_or(48_000);
    let silent = OpenedAudio {
        path: AudioPath::Silent,
        sample_rate: default_rate,
        link: None,
    };
    if !options.audio {
        return silent;
    }
    if classic {
        return open_classic_audio(sdl, default_rate).unwrap_or_else(|error| {
            log(&format!("no audio output ({error}); the host runs silent."));
            silent
        });
    }
    // The stream is built inside, once SDL has said what rate and block size it gave us: both
    // size its buffers, and the rate turns the latency in milliseconds into frames.
    let opened = AudioOutput::open(sdl, |freq, block_frames| {
        let rate = u32::try_from(freq).unwrap_or(default_rate);
        // What the core's ring can hold bounds the latency, since the core is not to change
        // (Q-153 §3).
        let usable = audio_stream::usable_latency_ms(
            rate,
            audio_stream::core_ring_samples(rate),
            latency_ms,
        );
        if usable < latency_ms - 0.5 {
            log(&format!(
                "audio latency {latency_ms:.0} ms is more than the core's ring can hold; using {usable:.0} ms."
            ));
        }
        let target = audio_stream::frames_of_ms(rate, usable);
        let (stream, control, console, program) = AudioStream::open(rate, block_frames, target);
        let link = AudioLink {
            control,
            console,
            sample_rate: rate,
        };
        let render: Box<dyn zax_platform::AudioRender> = Box::new(stream);
        (render, (link, program))
    });
    match opened {
        Ok((device, (link, program))) => {
            let rate = u32::try_from(device.sample_rate()).unwrap_or(default_rate);
            log(&format!(
                "audio: {rate} Hz, blocks of {} frames, target latency {:.0} ms.",
                device.block_frames(),
                link.target_ms()
            ));
            OpenedAudio {
                path: AudioPath::Pull {
                    device,
                    program,
                    // One block of programme, the size the mixer works in.
                    block: vec![0.0; crate::audio::audio_mixer::BLOCK * 2],
                },
                sample_rate: rate,
                link: Some(link),
            }
        }
        Err(error) => {
            log(&format!("no audio output ({error}); the host runs silent."));
            silent
        }
    }
}

/// The classic output of `v0.4.2` (AUDIO-2): no stream, no link, no latency to negotiate — the
/// device is an SDL queue and the mixer fills it from the main thread.
fn open_classic_audio(sdl: &sdl2::Sdl, default_rate: u32) -> Result<OpenedAudio, PlatformError> {
    let device = ClassicAudioOutput::open(sdl)?;
    let rate = u32::try_from(device.sample_rate()).unwrap_or(default_rate);
    log(&format!(
        "audio: {rate} Hz, the classic output of v0.4.2 ({} blocks of {} frames); \
         audio.latency_ms and the output counters do not apply to it.",
        zax_platform::audio::BLOCK_COUNT,
        zax_platform::audio::BLOCK_FRAMES
    ));
    Ok(OpenedAudio {
        path: AudioPath::Classic { device },
        sample_rate: rate,
        link: None,
    })
}

fn report_startup_problems(host: &mut AppHost, paths: &UserDataPaths) {
    use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
    if host.legacy_slots.was_recovered {
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
        log("no --rom given; the window opens on the ROM list of the console mode (F1: the welcome screen).");
        return;
    };
    let full: PathBuf = std::path::absolute(rom).unwrap_or_else(|_| PathBuf::from(rom));
    let full = full.to_string_lossy().into_owned();
    if let Some(movie) = options.movie_path.as_deref() {
        let movie = std::path::absolute(movie).unwrap_or_else(|_| PathBuf::from(movie));
        host.play_movie_after_launch(&movie.to_string_lossy());
    }
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
    // REQ-SCN-018 / REQ-INP-083: the camera controls only act while the game is running —
    // and, for the editor (EDITOR-1), while a movie is paused, to look around a frame.
    let paused_movie = host.state() == EmulationState::Paused && host.movie_position().is_some();
    if host.state() != EmulationState::Running && !paused_movie {
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
    // Rust-only (owner request): going into 2D puts the picture head on.
    if scene.two_dimensional_mode && !state.was_two_dimensional {
        scene.picture_head_on = true;
    }
    state.was_two_dimensional = scene.two_dimensional_mode;
    if scene.two_dimensional_mode && scene.picture_head_on {
        pull_head_on(scene, snapshot, camera_actions, real, state);
        return;
    }
    state.head_on_pull.clear();
    scene.picture_tilt = [0.0; 2];
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

/// The 2D picture seen head on (Rust-only, owner request): the camera does not turn, the
/// rotation asked for (camera actions, a drag; not the zoom) pulls the picture instead, which
/// gives way to the camera past the gauge's limit (`crate::head_on_pull`).
fn pull_head_on(
    scene: &mut SceneState,
    snapshot: &InputSnapshot,
    camera_actions: controls::CameraAction,
    real: f32,
    state: &mut RunState,
) {
    use crate::head_on_pull::Pull;
    use zax_render::numerics::Vector3;
    // The rotation asked for, on a copy of the camera (its zoom is not kept).
    let mut asked = scene.camera.clone();
    controls::apply(&mut asked, camera_actions, real);
    if snapshot.left_button && !snapshot.pointer_over_ui {
        if let Some(origin) = state.drag_origin {
            controls::apply_drag(
                &mut asked,
                snapshot.pointer[0] - origin[0],
                origin[1] - snapshot.pointer[1],
            );
        }
        state.drag_origin = Some(snapshot.pointer);
    } else {
        state.drag_origin = None;
    }
    let before = scene.camera.current().angles;
    let after = asked.current().angles;
    match state
        .head_on_pull
        .update([after.x - before.x, after.y - before.y], real)
    {
        Pull::Held(tilt) => scene.picture_tilt = tilt,
        Pull::Released(tilt, whole) => {
            let clipping = scene.picture_clipping();
            scene.camera.leave_picture_view(
                clipping,
                Vector3::new(tilt[0], tilt[1], 0.0),
                Vector3::new(whole[0], whole[1], 0.0),
            );
            scene.picture_head_on = false;
            scene.picture_tilt = [0.0; 2];
        }
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

/// The percentages of the `video.widescreen_extra` choices, by index.
const WIDESCREEN_EXTRA_PERCENT: [u16; 14] =
    [0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300];

/// The widescreen margin, in NES pixels on each side, that the `video.widescreen` choice offers
/// a ROM made for the extension: none, enough to fill the view's `size` (width, height, before
/// any resolution scale) with a 240-line picture, or enough for a fixed ratio; then `extra`
/// (`video.widescreen_extra`, an index into [`WIDESCREEN_EXTRA_PERCENT`]) wider, so that the 3D
/// objects at the edges of the view are already whole when they come into sight. Always a
/// whole number of tiles, at most [`zax_nes::wide::MAX_MARGIN`].
///
/// Returns the margins on each side and above and below: the picture keeps the chosen ratio,
/// so `extra` makes it as much taller as wider (only a margin decoder fills the height).
fn widescreen_margin(choice: i32, extra: i32, size: (i32, i32)) -> (i32, i32) {
    #[allow(clippy::cast_precision_loss)] // Window sizes.
    let ratio = match choice {
        1 if size.1 > 0 => size.0 as f32 / size.1 as f32,
        2 => 3.0 / 2.0,
        3 => 16.0 / 10.0,
        4 => 16.0 / 9.0,
        _ => return (0, 0),
    };
    let percent = usize::try_from(extra)
        .ok()
        .and_then(|index| WIDESCREEN_EXTRA_PERCENT.get(index))
        .copied()
        .unwrap_or(0);
    let bonus = 1.0 + (f32::from(percent) / 100.0);
    let whole_tiles = |pixels: f32, max: i32| {
        #[allow(clippy::cast_possible_truncation)] // Clamped right after.
        let tiles = (pixels.max(0.0) / 8.0).ceil() as i32;
        (tiles * 8).clamp(0, max)
    };
    (
        whole_tiles(
            (240.0 * ratio * bonus - 256.0) / 2.0,
            zax_nes::wide::MAX_MARGIN,
        ),
        whole_tiles(
            240.0 * (bonus - 1.0) / 2.0,
            zax_nes::wide::MAX_VERTICAL_MARGIN,
        ),
    )
}

#[allow(clippy::too_many_lines)] // The scene of a frame, each source of motion in its order.
fn update_scene(host: &mut AppHost, state: &mut RunState) {
    let two_dimensional = host.showing_two_dimensional();
    // How far into the emulated frame the display is, for the experimental 120 Hz scrolling.
    let phase = host.pacer.phase_at(state.frame_time);
    let show_band = host
        .settings()
        .get(SettingId::EmuSmoothScrollView)
        .as_bool();
    state.picture_between = false;
    let (widescreen, widescreen_y) = widescreen_margin(
        host.settings().get(SettingId::VideoWidescreen).as_int(),
        host.settings()
            .get(SettingId::VideoWidescreenExtra)
            .as_int(),
        state.view_size,
    );
    let strict_edges = host
        .settings()
        .get(SettingId::VideoWidescreenStrictEdges)
        .as_bool();
    // The colours the NES ones are shown in: the whole process's.
    zax_engine3d::palette::select_nes_palette(
        u8::try_from(host.settings().get(SettingId::VideoNesPalette).as_int()).unwrap_or(0),
    );
    let hack_options = host.hack_option_bits();
    // The smooth scrolling from the run-ahead: nothing to draw between two frames at 60 frames a
    // second or less, nor a real frame to slide from without run-ahead; then nothing is done.
    let smooth_ahead = host
        .settings()
        .get(SettingId::EmuSmoothScrollAhead)
        .as_bool()
        && host.settings().get(SettingId::EmuRunAhead).as_int() >= 1
        && host.display_rate > 61.0;
    let sprite_limit = match host
        .settings()
        .get(SettingId::VideoRemoveSpriteLimit)
        .as_int()
    {
        0 => SpriteLimit::Kept,
        1 => SpriteLimit::Removed,
        _ => SpriteLimit::Smart,
    };
    let mut overscan = crate::overscan::Overscan::default();
    let ntsc = host
        .session()
        .is_some_and(|session| session.nominal_frame_rate() > 55);
    if host.session().is_some() {
        overscan = crate::overscan::Overscan::from_settings(host.settings(), ntsc);
    }
    if let Some(session) = host.session_mut() {
        session.set_overscan(overscan);
        session.core_mut().set_hack_options(hack_options);
        session.core_mut().set_smooth_scroll_ahead(smooth_ahead);
        session.core_mut().set_sprite_limit(sprite_limit);
        session.core_mut().set_widescreen_margin(widescreen);
        session
            .core_mut()
            .set_widescreen_vertical_margin(widescreen_y);
        session.set_strict_wide_edges(strict_edges);
    }
    let (session, scene) = host.session_and_scene_mut();
    let Some(session) = session else {
        scene.two_dimensional_mode = false;
        scene.followed_shape = None;
        return;
    };
    scene.nes_background_color = session.nes_background_color();
    scene.emulated_time = session.emulated_time();
    scene.clipping = session.engine().settings().clipping;
    #[allow(clippy::cast_precision_loss)] // 0..=128.
    let margin = session
        .engine()
        .published_frame()
        .map_or(0, ConversionFrame::margin) as f32;
    scene.margin = margin;
    #[allow(clippy::cast_precision_loss)] // 0..=240.
    let margin_y = session
        .engine()
        .published_frame()
        .map_or(0, ConversionFrame::margin_y) as f32;
    scene.margin_y = margin_y;
    // AMD-009 (Q-022): the 2D mode shows the core's pixel frame on the NES plane. AMD-017
    // (Q-037) adds the hot F11 toggle to the "2D Mode" profile of REQ-SCN-055: the host combines
    // the two, and nothing of the session is reloaded to switch.
    scene.two_dimensional_mode = two_dimensional;
    if two_dimensional {
        state.picture_height =
            session.copy_nes_picture_layers(&mut state.picture, &mut scene.picture_overlay);
        scene.picture_shift = session.nes_picture_shift();
        // Experimental (owner request): on a display faster than the game, the playfield drawn
        // between two frames, along the camera's motion to the next one.
        if let (Some((shift, start, end)), Some(motion)) =
            (scene.picture_shift, session.nes_picture_motion())
        {
            #[allow(clippy::cast_possible_truncation)] // 0 to 1.
            let along = motion * phase as f32;
            scene.picture_shift = Some((shift + along, start, end));
            state.picture_between = true;
            if let Some(file) = state.scroll_log.as_mut() {
                use std::io::Write as _;
                let camera = session.nes_picture_camera().unwrap_or(0.0);
                let _ = writeln!(
                    file,
                    "{:.5} {} {phase:.4} {camera:.4} {shift:.4} {motion:.4} {:.4}",
                    state.frame_time,
                    state.emulated_now,
                    camera + f64::from(shift) + f64::from(along),
                );
            }
        }
        if show_band {
            if let Some((_, start, end)) = scene.picture_shift {
                tint_smooth_band(
                    &mut state.picture,
                    state.picture_height,
                    (start, end),
                    &mut scene.picture_overlay,
                );
            }
        }
        // The cut sides left out, the scene's background showing there; in Resize the head-on
        // view frames the rest. The picture's own size tells its margins: one 256 wide has none,
        // whatever margin the 3D conversion was asked for.
        let width = state.picture.len() / state.picture_height.max(1);
        // The picture's margins, not the conversion's: with `emu.pause_3d_in_2d` no frame is
        // converted in 2D, and the margin of the last one (none, in a game started in 2D) cut
        // the sides off, grey bands where the widescreen picture is.
        #[allow(clippy::cast_precision_loss)] // 0..=512.
        {
            scene.margin = (width.saturating_sub(256) / 2) as f32;
            scene.margin_y = (state.picture_height.saturating_sub(240) / 2) as f32;
        }
        let in_force = overscan.in_force(i32::try_from(width).map_or(0, |width| (width - 256) / 2));
        in_force.leave_out(&mut state.picture, state.picture_height);
        scene.picture_crop = zax_profiles::values::Vec4F::new(
            in_force.crop()[0],
            in_force.crop()[1],
            in_force.crop()[2],
            in_force.crop()[3],
        );
    } else {
        // The 3D scene framed on the game's picture for a slang preset leaves the same sides
        // out: the overscan's in Resize, none in Fit (the cut sides show the scene there).
        #[allow(clippy::cast_possible_truncation)] // A margin of 0..=128.
        let crop = overscan.in_force(scene.margin.max(0.0) as i32).crop();
        scene.picture_crop = zax_profiles::values::Vec4F::new(crop[0], crop[1], crop[2], crop[3]);
        scene.picture_shift = None;
        scene.picture_overlay.clear();
    }
    // REQ-SCN-022: the renderer needs the followed shape to find the occluders.
    scene.followed_shape = session.followed_shape();
}

fn render_scene(
    graphics: &mut Graphics,
    host: &mut AppHost,
    state: &mut RunState,
    clock: &mut StageClock,
) -> Result<(), String> {
    let (mut session, scene, view) = host.session_scene_and_view();
    let view = view.ok_or("the host has no view of the game")?;
    // AMD-051 REQ-E3D-646: the objects the last build found waiting, handed to the shared line
    // budget before the new list is built — the list is borrowed for the whole of this frame's
    // drawing, so a diagnostic found during a build is logged on the frame after it. An ordinary
    // frame takes an empty vector and allocates nothing.
    if let Some(session) = session.as_mut() {
        for object in session.take_late_objects() {
            graphics.report_late_object(object, LATE_OBJECT_FRAMES_REPORTED);
        }
    }
    let picture: &[u8] = if scene.two_dimensional_mode {
        &state.picture
    } else {
        &[]
    };
    // In 2D mode the renderer shows the core's picture alone (AMD-009) and reads nothing of the
    // draw list, so it is not built (Rust-only saving: the C# loop builds it every frame).
    let started = clock.start();
    let list = if picture.is_empty() {
        session.map(GameSession::build_draw_list)
    } else {
        None
    };
    clock.stop(Stage::DrawList, started);
    let frame = SceneFrame {
        draw_list: list,
        nes_picture: picture,
        nes_picture_height: state.picture_height,
    };
    draw_scene(
        graphics,
        view,
        scene,
        &frame,
        &mut state.shadow_map_reported,
        clock,
    )
}

/// Renders `frame` with `scene` into `view`, timing the collection and the passes.
fn draw_scene(
    graphics: &mut Graphics,
    view: &SceneView,
    scene: &SceneState,
    frame: &SceneFrame<'_>,
    shadow_map_reported: &mut bool,
    clock: &mut StageClock,
) -> Result<(), String> {
    let started = clock.start();
    graphics.render(view, scene, frame)?;
    let statistics = graphics.statistics();
    if let Some(started) = started {
        clock.add(Stage::Collect, statistics.collect_time);
        clock.add(
            Stage::Scene,
            started.elapsed().saturating_sub(statistics.collect_time),
        );
    }
    if !*shadow_map_reported && statistics.shadow_map_size != statistics.requested_shadow_map_size {
        *shadow_map_reported = true;
        log(&format!(
            "the hardware refused a {}-texel shadow map; it is {} texels (AMD-034).",
            statistics.requested_shadow_map_size, statistics.shadow_map_size
        ));
    }
    // AMD-051 REQ-E3D-644 and REQ-E3D-646: the renderer degrades instead of failing, and says so
    // here, at most 100 lines a game and then one summary a minute. An ordinary frame hands back
    // an empty vector, which allocates nothing.
    for line in graphics.take_render_diagnostics() {
        log(&line);
    }
    Ok(())
}

fn forward_events(
    host: &mut AppHost,
    ui: &mut ImguiFrontend,
    window: &mut HostWindow,
    pacing: &mut VideoPacing,
) {
    // `video.half_rate_3d`: the swap interval follows the 2D / 3D view.
    let half_rate = host.half_rate_3d();
    if half_rate != pacing.half_rate_3d {
        pacing.half_rate_3d = half_rate;
        pacing.apply(window, host.settings());
    }
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

/// Feeds the output that is open, once per rendered frame — the one branch the choice of output
/// of AUDIO-2 costs the loop.
///
/// On the **default** output (Q-153) this keeps the menu music and the interface sounds a few
/// tens of milliseconds ahead of the audio thread, and nothing else: the console's samples do not
/// pass through here, the audio thread reading the core's ring itself, so a long frame on this
/// thread cannot starve the device. The mixer's clock advances with the programme it produced,
/// and the ring is drained in real time, so the fades of REQ-AUD-020 and REQ-AUD-022 keep their
/// real-time slope.
///
/// On the **classic** output this is `v0.4.2`'s `pump_audio`, unchanged: the device pulls as many
/// whole blocks as it has free buffers and the mixer fills each with the console, the music and
/// the sounds, so its clock advances with the audio it produced, which is real time.
fn pump_audio(host: &mut AppHost, audio: &mut AudioPath, real: f64) {
    let Some(mixer) = host.mixer_mut() else {
        return;
    };
    match audio {
        AudioPath::Pull { program, block, .. } => {
            let mut missing = program.missing_frames();
            while missing > 0 {
                let frames = missing.min(block.len() / 2);
                let slice = &mut block[..frames * 2];
                mixer.fill(slice);
                program.push(slice);
                missing -= frames;
            }
        }
        AudioPath::Classic { device } => {
            match device.pump(|buffer| mixer.fill_with_console(buffer)) {
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
        AudioPath::Silent => mixer.advance(real),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nes_palette_choices_are_the_engines_palettes() {
        let labels: Vec<&str> = zax_settings::catalogue::descriptor(SettingId::VideoNesPalette)
            .domain()
            .options()
            .unwrap_or_default()
            .iter()
            .map(|option| option.label.as_str())
            .collect();
        assert_eq!(labels, zax_engine3d::palette::NesPalette::LABELS);
    }
}
