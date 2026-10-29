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
use zax_platform::{AudioOutput, HostWindow, HostWindowSettings};
use zax_profiles::format;
use zax_render::assets::{NeutralSceneAssets, SceneAssets};
use zax_render::controls;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::SceneRenderer;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use crate::app_log;
use crate::app_options::AppOptions;
use crate::audio::audio_assets::AudioAssets;
use crate::audio::synth_audio_library::SynthAudioLibrary;
use crate::audio::{AudioMixer, TrackRandom, UiAudioLibrary};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::frame_pacer::CatchUp;
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::host::{DeferredParts, EmulationHost, EmulationHostOptions, HostEvent};
use crate::emulation::video_pacing::{VideoPacing, VideoPacingRequest};
use crate::frame_profiler::{self, FrameProfiler, Stage, StageClock, Thread};
use crate::frame_rate_counter::FrameRateCounter;
use crate::game_session::GameSession;
use crate::graphics::{Backend, Graphics, SceneView};
use crate::input::cursor_auto_hide::CursorAutoHide;
use crate::input::input_action::to_camera_actions;
use crate::input::{routing, DesktopInputSource, DeviceRefs, InputSnapshot};
use crate::library_game_catalog::LibraryGameCatalog;
use crate::persistence::app_settings::AppSettings;
use crate::persistence::command_line_settings;
use crate::persistence::input_binding_store::FileInputBindingStore;
use crate::persistence::settings_store::SettingsStore;
use crate::pipeline::{PipelineGovernor, PipelineMode};
use crate::render_scaling;
use crate::render_skip;
use crate::scene_script_bridge::{self, ConsoleScriptLog};
use crate::ui::debug_overlay::DebugInfo;
use crate::ui::frontend::{ImguiFrontend, Sizes, UiGraphics};
use crate::ui::handheld;
use crate::ui::strings::UiLanguage;
use crate::ui::zax_ui::{UiEffect, UiFrame};
use crate::user_data_paths::UserDataPaths;
use crate::{neutral_assets, persistence::input_binding_store::InputBindingStore as _};

/// The host the application runs: over the library, with the offscreen picture of the scene as
/// its view of the game.
pub type AppHost = EmulationHost<LibraryGameCatalog, SceneView>;

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
    // Field order is drop order: the renderer and the host (whose view is the scene picture)
    // go before the window, whose GL context or wgpu device they were made on.
    graphics: Graphics,
    input: SdlInput,
    source: DesktopInputSource,
    audio: Option<AudioOutput>,
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
    /// The pipelined frames (`emu.pipeline`).
    pipeline: Pipelined,
    /// The automatic resolution (`video.dynamic_resolution`).
    resolution: AutomaticResolution,
    /// The save state slot `--load-state` asks for, until the game accepts the load.
    pending_state_load: Option<i32>,
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
        let ui = ImguiFrontend::new(ui_graphics, &host, options, height)?;
        Ok(Self {
            options: options.clone(),
            ui,
            graphics,
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
            pipeline: Pipelined::default(),
            resolution: AutomaticResolution::default(),
            pending_state_load: options.load_state,
        })
    }

    /// One displayed frame. Returns false once the application is to close.
    fn frame(&mut self) -> Result<bool, String> {
        self.follow_the_profiler_setting();
        self.load_the_asked_state();
        let frame_begin = self.clock.start();
        let busy_begin = Instant::now();
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
        let pipelined = self.next_frame_pipelined();
        self.host.defer_conversion = pipelined;
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
        let overlap = if pipelined {
            self.draw_pipelined(emulated, width, height, real_f)?
        } else {
            self.draw(emulated, width, height, real_f)?;
            Duration::ZERO
        };
        self.carry_out_ui_effects();
        let mark = self.clock.start();
        pump_audio(&mut self.host, self.audio.as_mut(), real);
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
        );
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
        operation(&mut self.ui, &mut frame, sizes, &debug);
    }

    /// `--load-state`: loads the slot once the game runs and the host accepts the load (the
    /// game loads in the background, and a slot of another game is refused).
    fn load_the_asked_state(&mut self) {
        let Some(slot) = self.pending_state_load else {
            return;
        };
        let args = CommandArgs::of_number(slot);
        if self.host.state() == EmulationState::Running
            && self.host.can_execute(Command::LoadSlot, &args)
        {
            self.host.enqueue(Command::LoadSlot, args);
            self.pending_state_load = None;
            log(&format!("save state slot {slot} loaded (--load-state)."));
        }
    }

    /// Turns the frame profiler on or off with `ui.profiler` (`ZAXNES_PROFILE=1` keeps it on):
    /// off, the loop measures nothing and writes no profile line.
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

    /// The time one displayed frame may take: the swap interval at the display's refresh rate.
    fn frame_budget(&self) -> Duration {
        let refresh = self
            .window
            .window()
            .display_mode()
            .map_or(60, |mode| mode.refresh_rate)
            .max(1);
        let interval = self.host.settings().vsync().max(1);
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
    if options.handheld {
        handheld::defaults::apply(&mut model);
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

struct HostParts<'a> {
    paths: &'a UserDataPaths,
    model: SettingsModel,
    scene: SceneState,
    catalog: LibraryGameCatalog,
    store: SettingsStore,
    asset_root: Option<&'a Path>,
    sample_rate: u32,
    audio_open: bool,
    view: SceneView,
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
    // No rewind on a handheld (owner request): its snapshot every 60 frames costs a spike.
    host.rollback_enabled = !options.handheld;
    host.separate_core = separate_core(options);
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
    if device {
        bindings.use_handheld_defaults();
    } else {
        bindings.use_desktop_console_defaults();
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
        log("no --rom given; the window opens on the ROM list of the console mode (F1: the welcome screen).");
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

fn update_scene(host: &mut AppHost, state: &mut RunState) {
    let two_dimensional = host.showing_two_dimensional();
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
        state.picture_height = session.copy_nes_picture(&mut state.picture);
        // The cut sides blacked out; in Resize the head-on view leaves them out too.
        let in_force = overscan.in_force(
            session
                .engine()
                .published_frame()
                .map_or(0, ConversionFrame::margin),
            state.picture_height == 240 || session.nes_picture_hides_overscan(),
        );
        // Through the camera, which shows the whole plane, they are left out instead.
        if scene.picture_head_on {
            in_force.black_out(&mut state.picture, state.picture_height);
        } else {
            in_force.leave_out(&mut state.picture, state.picture_height);
        }
        scene.picture_crop = zax_profiles::values::Vec4F::new(
            in_force.crop()[0],
            in_force.crop()[1],
            in_force.crop()[2],
            in_force.crop()[3],
        );
    } else {
        scene.picture_crop = zax_profiles::values::Vec4F::new(0.0, 0.0, 0.0, 0.0);
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
    let (session, scene, view) = host.session_scene_and_view();
    let view = view.ok_or("the host has no view of the game")?;
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
