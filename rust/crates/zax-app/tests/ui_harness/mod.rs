//! The harness of the interface tests (`UiHarness.cs`, `UiLanguageScope.cs`): a host over the
//! library catalogue in a temporary user folder, the interface over it, the input layer over fake
//! devices, and a Dear `ImGui` context that draws without a window or a GL context.
#![allow(dead_code)] // each integration test crate uses a part of this module
#![allow(clippy::expect_used, clippy::unwrap_used)] // test support code

pub mod bundle_support;

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, PoisonError};

use time::OffsetDateTime;
use zax_app::application::AppHost;
use zax_app::audio::{AudioMixer, TrackRandom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::frame_pacer::Clock;
use zax_app::emulation::game_catalog::GameCatalog as _;
use zax_app::emulation::host::{EmulationHost, EmulationHostOptions};
use zax_app::input::{DesktopInputSource, DeviceRefs, InputSnapshot};
use zax_app::library_game_catalog::LibraryGameCatalog;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::ui::strings::{self, UiLanguage};
use zax_app::ui::zax_ui::{UiFrame, ZaxUi};
use zax_app::user_data_paths::UserDataPaths;
use zax_profiles::catalogue::ProfileCatalogue;
use zax_profiles::library::GameLibrary;
use zax_render::scene::SceneState;

use crate::common::{
    FakeGamepadHub, FakeKeyboard, SyntheticRom, TemporaryRoot, START_UNIX_SECONDS,
};

/// Puts the interface language back when it goes out of scope, so the order of the tests cannot
/// matter (the language is thread-local, so a test cannot disturb another either).
pub struct UiLanguageScope {
    previous: UiLanguage,
}

impl UiLanguageScope {
    /// Switches to `language` until the scope ends.
    pub fn new(language: UiLanguage) -> Self {
        let previous = strings::language();
        strings::set_language(language);
        Self { previous }
    }

    /// Keeps the language in force, and restores it at the end.
    pub fn keep() -> Self {
        Self::new(strings::language())
    }
}

impl Drop for UiLanguageScope {
    fn drop(&mut self) {
        strings::set_language(self.previous);
    }
}

/// Dear `ImGui` allows one context at a time in the whole process, so the tests that draw take
/// turns (the C# collection `dear-imgui` did the same by disabling parallelisation).
static ONE_CONTEXT: Mutex<()> = Mutex::new(());

/// A Dear `ImGui` context that runs frames without a window or a renderer.
pub struct ImGuiHarness {
    context: imgui::Context,
    _turn: MutexGuard<'static, ()>,
}

impl ImGuiHarness {
    pub fn new() -> Self {
        let mut harness = Self::bare();
        harness.context.fonts().build_rgba32_texture();
        harness
    }

    /// A context whose font atlas is not built yet, for the tests that load their own fonts.
    pub fn bare() -> Self {
        let turn = ONE_CONTEXT.lock().unwrap_or_else(PoisonError::into_inner);
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [1280.0, 720.0];
        context.io_mut().delta_time = 1.0 / 60.0;
        Self {
            context,
            _turn: turn,
        }
    }

    pub fn context_mut(&mut self) -> &mut imgui::Context {
        &mut self.context
    }

    /// Runs one frame of `draw`.
    pub fn frame(&mut self, draw: impl FnOnce(&imgui::Ui)) {
        let ui = self.context.new_frame();
        draw(ui);
        self.context.render();
    }
}

/// The host, the interface and the devices of one test.
pub struct UiHarness {
    pub root: TemporaryRoot,
    pub paths: UserDataPaths,
    pub host: AppHost,
    pub ui: ZaxUi,
    pub input: DesktopInputSource,
    pub keyboard: FakeKeyboard,
    pub pads: FakeGamepadHub,
    imgui: Option<ImGuiHarness>,
    clock: Rc<Cell<f64>>,
}

impl UiHarness {
    /// A library of "2D Mode" and the given titles, which get the identifiers `g0`, `g1`...
    pub fn new(titles: &[&str]) -> Self {
        let root = TemporaryRoot::new("zax-ui-");
        let paths = UserDataPaths::resolve(Some(root.path()), None, None);
        let mut store = SettingsStore::new(&paths);
        let mut settings = zax_settings::model::SettingsModel::new();
        store.load(&mut settings, instant(START_UNIX_SECONDS));
        let catalog = LibraryGameCatalog::from_library(
            GameLibrary::build(&catalogue(titles), &[], None, None),
            paths.clone(),
            Box::new(|_| {}),
        );
        let clock = Rc::new(Cell::new(0.0));
        let mut options =
            EmulationHostOptions::new(paths.clone(), settings, SceneState::default(), catalog);
        options.mixer = Some(AudioMixer::new(48_000, None, TrackRandom::new(7)));
        let clock_for_host = Rc::clone(&clock);
        options.clock = Some(Box::new(move || clock_for_host.get()) as Clock);
        options.now = Some(Box::new(|| instant(START_UNIX_SECONDS)));
        options.settings_store = Some(store);
        options.confirm_with_interface = true;
        let mut host: AppHost = EmulationHost::new(options);
        let mut ui = ZaxUi::new(&host);
        // The language is the setting's, read once when the host is built.
        for event in host.take_events() {
            ui.handle_host_event(&mut host, event);
        }
        Self {
            root,
            paths,
            host,
            ui,
            input: DesktopInputSource::unsaved(),
            keyboard: FakeKeyboard::default(),
            pads: FakeGamepadHub::default(),
            imgui: None,
            clock,
        }
    }

    pub fn two_dimensional_mode_id() -> &'static str {
        "2d"
    }

    /// Writes a synthetic ROM in the user folder and returns its path.
    pub fn write_rom(&self, name: &str) -> String {
        let path: PathBuf = self.root.path().join(name);
        std::fs::write(&path, SyntheticRom::counter()).expect("the ROM can be written");
        path.to_string_lossy().into_owned()
    }

    /// Hands the events the host raised to the interface.
    pub fn pump(&mut self) {
        for event in self.host.take_events() {
            self.ui.handle_host_event(&mut self.host, event);
        }
        self.ui.sync_confirmation(&self.host);
    }

    /// Samples the input layer once, as the loop does.
    pub fn sample(&mut self, seconds: f32) -> InputSnapshot {
        self.input.sample(
            seconds,
            DeviceRefs {
                keyboard: Some(&self.keyboard),
                pointer: None,
                hub: &mut self.pads,
            },
        )
    }

    pub fn tick(&mut self, seconds: f64) -> u32 {
        self.clock.set(self.clock.get() + seconds);
        self.host.set_modal_window_open(self.ui.state.is_modal());
        #[allow(clippy::cast_possible_truncation)] // the C# `(float)seconds`
        let snapshot = self.sample(seconds as f32);
        let emulated = self.host.tick(seconds, &snapshot);
        self.pump();
        emulated
    }

    pub fn settle(&mut self) {
        for _ in 0..128 {
            if !self.host.is_loading() && self.host.pending_commands() == 0 {
                break;
            }
            self.tick(1.0 / 1000.0);
        }
    }

    /// Enqueues `command`, applies it and lets the interface answer.
    pub fn run(&mut self, command: Command) {
        self.run_with(command, CommandArgs::default());
    }

    pub fn run_with(&mut self, command: Command, args: CommandArgs) {
        self.host.set_modal_window_open(self.ui.state.is_modal());
        self.host.enqueue(command, args);
        self.host.apply_commands();
        self.pump();
        self.settle();
    }

    /// Starts the game `game_id` from a synthetic ROM.
    pub fn launch(&mut self, game_id: &str) {
        let rom = self.write_rom("game.nes");
        self.host.games_mut().set_rom_path(game_id, &rom);
        self.run_with(Command::LaunchGame, CommandArgs::of_text(game_id));
    }

    /// Draws one frame of the interface.
    pub fn draw(&mut self) {
        let Self {
            host,
            ui,
            input,
            pads,
            imgui,
            ..
        } = self;
        imgui
            .get_or_insert_with(ImGuiHarness::new)
            .frame(|imgui_ui| {
                let mut frame = UiFrame {
                    host,
                    input,
                    hub: pads,
                    real_seconds: 1.0 / 60.0,
                    frames_per_second: 60.0,
                };
                ui.draw(imgui_ui, &mut frame);
            });
    }

    pub fn draw_frames(&mut self, count: usize) {
        for _ in 0..count {
            self.draw();
        }
    }
}

fn instant(unix_seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(unix_seconds).expect("a valid instant")
}

fn catalogue(titles: &[&str]) -> ProfileCatalogue {
    let mut entries =
        vec![r#"{"id":"2d","title":"2D Mode","file":"profiles/2d.bytes"}"#.to_owned()];
    for (index, title) in titles.iter().enumerate() {
        entries.push(format!(
            r#"{{"id":"g{index}","title":"{title}","file":"profiles/g{index}.bytes"}}"#
        ));
    }
    let json = format!(r#"{{"editions":{{"full":[{}]}}}}"#, entries.join(","));
    ProfileCatalogue::parse(&json, std::path::Path::new("."), "full").expect("a valid catalogue")
}
