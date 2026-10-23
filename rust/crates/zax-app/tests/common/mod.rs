//! Shared by the integration tests: synthetic ROMs, a fake catalogue, the loop harness and the
//! fake input devices (`LoopHarness.cs`, `FakeInputDevices.cs`).
#![allow(dead_code)] // each integration test crate uses a part of this module
#![allow(clippy::expect_used, clippy::unwrap_used)] // test support code

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use time::OffsetDateTime;
use zax_app::audio::{AudioMixer, TrackRandom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::frame_pacer::Clock;
use zax_app::emulation::game_catalog::{GameCatalog, ResolvedGame};
use zax_app::emulation::host::{EmulationHost, EmulationHostOptions};
use zax_app::input::{InputAction, InputSnapshot};
use zax_app::user_data_paths::UserDataPaths;
use zax_platform::input::{PhysicalControl, PhysicalControlKind};
use zax_profiles::profile::GameProfile;
use zax_render::scene::SceneState;

/// The instant the harness starts at, as the C# harness: 2025-10-20 08:46:40 UTC.
pub const START_UNIX_SECONDS: i64 = 1_760_950_000;

/// A folder that is removed when it goes out of scope.
pub struct TemporaryRoot {
    path: PathBuf,
}

impl TemporaryRoot {
    pub fn new(prefix: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{prefix}{}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("the temporary folder can be created");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        // A locked temporary folder must not fail a test run.
        if std::fs::remove_dir_all(&self.path).is_err() {
            eprintln!("could not remove {}", self.path.display());
        }
    }
}

/// Builds iNES images in memory.
pub struct SyntheticRom;

impl SyntheticRom {
    pub fn build(program: Option<&[u8]>, battery: bool) -> Vec<u8> {
        let mut image = vec![0_u8; 16 + 32 * 1024 + 8 * 1024];
        image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
        image[4] = 2; // 2 x 16 KiB PRG.
        image[5] = 1; // 1 x 8 KiB CHR.
        image[6] = 0x01 | if battery { 0x02 } else { 0x00 };
        let code = program
            .filter(|code| !code.is_empty())
            .unwrap_or(&[0x4C, 0x00, 0x80]);
        image[16..16 + code.len()].copy_from_slice(code);
        for vector in 0..3 {
            image[16 + 32 * 1024 - 6 + vector * 2] = 0x00;
            image[16 + 32 * 1024 - 5 + vector * 2] = 0x80;
        }
        image
    }

    pub fn counter() -> Vec<u8> {
        Self::build(Some(&[0xE6, 0x10, 0x4C, 0x00, 0x80]), false)
    }

    pub fn battery_writer(value: u8) -> Vec<u8> {
        Self::build(
            Some(&[0xA9, value, 0x8D, 0x00, 0x60, 0x4C, 0x05, 0x80]),
            true,
        )
    }
}

#[derive(Clone)]
struct Entry {
    title: String,
    profile: Arc<GameProfile>,
    is_2d_mode: bool,
    rom_path: Option<String>,
    cover: Option<String>,
}

/// A library of a few entries held in memory.
#[derive(Default)]
pub struct FakeCatalog {
    entries: HashMap<String, Entry>,
    pub played: Vec<(String, i64)>,
    pub deleted: Vec<String>,
    two_dimensional_mode_id: Option<String>,
    pub default_three_dimensional_id: Option<String>,
}

impl FakeCatalog {
    pub fn add(&mut self, id: &str, rom_path: Option<&str>) -> &mut Self {
        self.add_with(id, rom_path, None, false, None)
    }

    pub fn add_with(
        &mut self,
        id: &str,
        rom_path: Option<&str>,
        profile: Option<GameProfile>,
        is_2d_mode: bool,
        title: Option<&str>,
    ) -> &mut Self {
        self.entries.insert(
            id.to_owned(),
            Entry {
                title: title.unwrap_or(id).to_owned(),
                profile: Arc::new(profile.unwrap_or_else(GameProfile::create_empty)),
                is_2d_mode,
                rom_path: rom_path.map(str::to_owned),
                cover: None,
            },
        );
        if is_2d_mode {
            self.two_dimensional_mode_id = Some(id.to_owned());
        }
        self
    }

    pub fn remove(&mut self, id: &str) {
        self.entries.remove(id);
    }
}

impl GameCatalog for FakeCatalog {
    fn resolve(&mut self, game_id: &str) -> Option<ResolvedGame> {
        self.entries.get(game_id).map(|entry| ResolvedGame {
            id: game_id.to_owned(),
            title: entry.title.clone(),
            profile: Arc::clone(&entry.profile),
            is_2d_mode: entry.is_2d_mode,
            is_default_three_dimensional: false,
        })
    }

    fn exists(&self, game_id: &str) -> bool {
        self.entries.contains_key(game_id)
    }

    fn rom_path(&self, game_id: &str) -> Option<String> {
        self.entries.get(game_id)?.rom_path.clone()
    }

    fn game_for_rom(&self, rom_path: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, entry)| {
                entry
                    .rom_path
                    .as_deref()
                    .is_some_and(|path| path.eq_ignore_ascii_case(rom_path))
            })
            .map(|(id, _)| id.clone())
    }

    fn two_dimensional_mode_id(&self) -> Option<String> {
        self.two_dimensional_mode_id.clone()
    }

    fn default_three_dimensional_id(&self) -> Option<String> {
        self.default_three_dimensional_id.clone()
    }

    fn set_rom_path(&mut self, game_id: &str, rom_path: &str) {
        if let Some(entry) = self.entries.get_mut(game_id) {
            entry.rom_path = Some(rom_path.to_owned());
        }
    }

    fn mark_played(&mut self, game_id: &str, unix_seconds: i64) {
        self.played.push((game_id.to_owned(), unix_seconds));
    }

    fn delete_game(&mut self, game_id: &str) -> bool {
        if !self.can_delete(game_id) {
            return false;
        }
        self.deleted.push(game_id.to_owned());
        self.entries.remove(game_id);
        true
    }

    fn title(&self, game_id: &str) -> Option<String> {
        self.entries.get(game_id).map(|entry| entry.title.clone())
    }

    fn can_delete(&self, game_id: &str) -> bool {
        self.entries
            .get(game_id)
            .is_some_and(|entry| !entry.is_2d_mode)
    }

    fn set_cover(&mut self, game_id: &str, file_name: Option<&str>) -> bool {
        match self.entries.get_mut(game_id) {
            Some(entry) => {
                entry.cover = file_name.map(str::to_owned);
                true
            }
            None => false,
        }
    }

    fn cover(&self, game_id: &str) -> Option<String> {
        self.entries.get(game_id)?.cover.clone()
    }
}

/// A host over a temporary user folder, a fake catalogue and a clock the test moves by hand.
pub struct LoopHarness {
    pub root: TemporaryRoot,
    pub host: EmulationHost<FakeCatalog>,
    pub paths: UserDataPaths,
    pub log: Rc<RefCell<Vec<String>>>,
    pub thumbnail: Rc<RefCell<Option<Vec<u8>>>>,
    clock: Rc<Cell<f64>>,
    now: Rc<Cell<i64>>,
}

impl LoopHarness {
    pub fn create() -> Self {
        Self::create_with(|_| {})
    }

    /// A harness whose host options are adjusted by `adjust` before the host is built.
    pub fn create_with(adjust: impl FnOnce(&mut EmulationHostOptions<FakeCatalog>)) -> Self {
        let root = TemporaryRoot::new("zax-loop-");
        let paths = UserDataPaths::resolve(Some(root.path()), None, None);
        let clock = Rc::new(Cell::new(0.0));
        let now = Rc::new(Cell::new(START_UNIX_SECONDS));
        let log = Rc::new(RefCell::new(Vec::new()));
        let thumbnail = Rc::new(RefCell::new(None));
        let mut options = EmulationHostOptions::new(
            paths.clone(),
            zax_settings::model::SettingsModel::new(),
            SceneState::default(),
            FakeCatalog::default(),
        );
        options.mixer = Some(AudioMixer::new(48_000, None, TrackRandom::new(1234)));
        let clock_for_host = Rc::clone(&clock);
        options.clock = Some(Box::new(move || clock_for_host.get()) as Clock);
        let now_for_host = Rc::clone(&now);
        options.now = Some(Box::new(move || {
            OffsetDateTime::from_unix_timestamp(now_for_host.get()).expect("a valid instant")
        }));
        let capture = Rc::clone(&thumbnail);
        options.capture_thumbnail = Some(Box::new(move || capture.borrow().clone()));
        let lines = Rc::clone(&log);
        options.log = Some(Box::new(move |message| {
            lines.borrow_mut().push(message.to_owned());
        }));
        adjust(&mut options);
        Self {
            host: EmulationHost::new(options),
            root,
            paths,
            log,
            thumbnail,
            clock,
            now,
        }
    }

    pub fn now(&self) -> i64 {
        self.now.get()
    }

    pub fn set_now(&self, unix_seconds: i64) {
        self.now.set(unix_seconds);
    }

    pub fn clock(&self) -> f64 {
        self.clock.get()
    }

    pub fn set_thumbnail(&self, picture: Option<Vec<u8>>) {
        *self.thumbnail.borrow_mut() = picture;
    }

    pub fn catalog(&mut self) -> &mut FakeCatalog {
        self.host.games_mut()
    }

    pub fn write_rom(&self, image: &[u8], name: &str) -> String {
        let path = self.root.path().join(name);
        std::fs::write(&path, image).expect("the ROM can be written");
        path.to_string_lossy().into_owned()
    }

    pub fn tick(&mut self, seconds: f64) -> u32 {
        self.tick_actions(seconds, InputAction::NONE, InputAction::NONE)
    }

    pub fn tick_actions(
        &mut self,
        seconds: f64,
        actions: InputAction,
        player_two: InputAction,
    ) -> u32 {
        self.clock.set(self.clock.get() + seconds);
        self.host.tick_actions(seconds, actions, player_two)
    }

    pub fn tick_snapshot(&mut self, seconds: f64, snapshot: &InputSnapshot) -> u32 {
        self.clock.set(self.clock.get() + seconds);
        self.host.tick(seconds, snapshot)
    }

    pub fn tick_frame(&mut self) -> u32 {
        self.tick(1.0 / 60.0)
    }

    pub fn settle(&mut self) {
        for _ in 0..64 {
            if !self.host.is_loading() {
                break;
            }
            self.tick(1.0 / 1000.0);
        }
    }

    pub fn run(&mut self, command: Command) {
        self.run_with(command, CommandArgs::default());
    }

    pub fn run_with(&mut self, command: Command, args: CommandArgs) {
        self.host.enqueue(command, args);
        self.host.apply_commands();
        self.settle();
    }

    pub fn launch(&mut self, game_id: &str) {
        let rom = self.write_rom(&SyntheticRom::counter(), &format!("{game_id}.nes"));
        self.catalog().add(game_id, Some(&rom));
        self.run_with(Command::LaunchGame, CommandArgs::of_text(game_id));
    }

    /// Runs frames straight on the session, bypassing the pacer.
    pub fn run_exact_frames(&mut self, frames: usize, sequence: &[InputAction]) {
        let sequence = if sequence.is_empty() {
            &[InputAction::NONE][..]
        } else {
            sequence
        };
        for frame in 0..frames {
            self.host
                .run_session_frame(sequence[frame % sequence.len()], InputAction::NONE);
        }
    }

    pub fn run_frames(&mut self, frames: u32) -> u32 {
        self.run_frames_with(frames, InputAction::NONE)
    }

    pub fn run_frames_with(&mut self, frames: u32, actions: InputAction) -> u32 {
        let mut ran = 0;
        let period = 1.0 / self.host.pacer.effective_rate().max(1.0);
        for _ in 0..frames * 4 {
            if ran >= frames {
                break;
            }
            ran += self.tick_actions(period, actions, InputAction::NONE);
        }
        ran
    }
}

/// A keyboard driven by the test: the set of physical key positions currently held
/// (SPEC-08 DEC-08-5).
#[derive(Default)]
pub struct FakeKeyboard {
    down: std::collections::BTreeSet<i32>,
}

impl FakeKeyboard {
    /// Holds the key at position `code`.
    pub fn press(&mut self, code: i32) {
        self.down.insert(code);
    }

    /// Releases the key at position `code`.
    pub fn release(&mut self, code: i32) {
        self.down.remove(&code);
    }

    /// Releases every key.
    pub fn release_all(&mut self) {
        self.down.clear();
    }
}

impl zax_platform::input::KeyboardDevice for FakeKeyboard {
    fn is_down(&self, key_code: i32) -> bool {
        self.down.contains(&key_code)
    }

    fn collect_held_keys(&self, keys: &mut Vec<i32>) {
        keys.extend(self.down.iter().copied());
    }
}

/// A mouse driven by the test (REQ-INP-120).
pub struct FakePointer {
    pub position: [f32; 2],
    pub left: bool,
    pub right: bool,
    pub visible: bool,
    wheel: f32,
}

impl Default for FakePointer {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            left: false,
            right: false,
            visible: true,
            wheel: 0.0,
        }
    }
}

impl FakePointer {
    /// Adds `notches` of wheel travel, positive forward.
    pub fn scroll(&mut self, notches: f32) {
        self.wheel += notches;
    }
}

impl zax_platform::input::PointerDevice for FakePointer {
    fn position(&self) -> [f32; 2] {
        self.position
    }

    fn left_button(&self) -> bool {
        self.left
    }

    fn right_button(&self) -> bool {
        self.right
    }

    fn take_wheel_notches(&mut self) -> f32 {
        std::mem::take(&mut self.wheel)
    }

    fn cursor_visible(&self) -> bool {
        self.visible
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

#[derive(Default)]
struct PadState {
    buttons: std::collections::BTreeSet<i32>,
    axes: [f32; 4],
    triggers: [f32; 2],
}

/// A simulated gamepad: raw button states, raw stick axes and raw triggers, so the dead zones
/// and the 0.34 threshold of SPEC-08 §3.4 are exercised exactly as on a real pad. Clones share
/// the state, so a test keeps a handle after the pad went into a hub.
#[derive(Clone)]
pub struct FakeGamepad {
    identity: String,
    standard: bool,
    state: Rc<RefCell<PadState>>,
}

impl FakeGamepad {
    /// Creates a standard pad with the given identity (REQ-INP-025).
    pub fn new(identity: &str) -> Self {
        Self::with_layout(identity, true)
    }

    /// Creates a pad that answers to the standard layout or not (REQ-INP-020).
    pub fn with_layout(identity: &str, standard: bool) -> Self {
        Self {
            identity: identity.to_owned(),
            standard,
            state: Rc::default(),
        }
    }

    /// Holds button `code`.
    pub fn press(&self, code: i32) -> &Self {
        self.state.borrow_mut().buttons.insert(code);
        self
    }

    /// Releases button `code`.
    pub fn release(&self, code: i32) -> &Self {
        self.state.borrow_mut().buttons.remove(&code);
        self
    }

    /// Sets the raw value of stick axis `index`, -1 to 1.
    pub fn axis(&self, index: i32, value: f32) -> &Self {
        self.state.borrow_mut().axes[usize::try_from(index).expect("axis index")] = value;
        self
    }

    /// Sets the raw travel of trigger `index`, 0 to 1.
    pub fn trigger(&self, index: i32, value: f32) -> &Self {
        self.state.borrow_mut().triggers[usize::try_from(index).expect("trigger index")] = value;
        self
    }

    /// Releases everything.
    pub fn release_all(&self) -> &Self {
        *self.state.borrow_mut() = PadState::default();
        self
    }

    fn paired(state: &PadState, axis: i32) -> f32 {
        use zax_platform::input::standard_gamepad as pad;
        let other = match axis {
            pad::LEFT_STICK_X => pad::LEFT_STICK_Y,
            pad::LEFT_STICK_Y => pad::LEFT_STICK_X,
            pad::RIGHT_STICK_X => pad::RIGHT_STICK_Y,
            pad::RIGHT_STICK_Y => pad::RIGHT_STICK_X,
            _ => return 0.0,
        };
        state.axes[usize::try_from(other).expect("axis index")]
    }
}

impl zax_platform::input::GamepadDevice for FakeGamepad {
    fn identity(&self) -> &str {
        &self.identity
    }

    fn display_name(&self) -> &str {
        &self.identity
    }

    fn is_standard(&self) -> bool {
        self.standard
    }

    fn value(&self, control: PhysicalControl) -> f32 {
        use zax_platform::input::calibration;
        let state = self.state.borrow();
        let index = usize::try_from(control.code).ok();
        match control.kind {
            PhysicalControlKind::Button => {
                f32::from(u8::from(state.buttons.contains(&control.code)))
            }
            PhysicalControlKind::Axis => index.and_then(|i| state.axes.get(i)).map_or(0.0, |raw| {
                calibration::half_axis(*raw, Self::paired(&state, control.code), control.direction)
            }),
            PhysicalControlKind::Trigger => index
                .and_then(|i| state.triggers.get(i))
                .map_or(0.0, |raw| calibration::trigger(*raw)),
            _ => 0.0,
        }
    }

    fn collect_active_controls(&self, controls: &mut Vec<PhysicalControl>) {
        use zax_platform::input::{calibration, standard_gamepad as pad};
        for button in 0..pad::BUTTON_COUNT {
            if self.state.borrow().buttons.contains(&button) {
                controls.push(PhysicalControl::button(button));
            }
        }
        for axis in 0..pad::AXIS_COUNT {
            for direction in [1, -1] {
                let half = PhysicalControl::axis(axis, direction);
                if calibration::is_pressed(self.value(half)) {
                    controls.push(half);
                }
            }
        }
        for trigger in 0..pad::TRIGGER_COUNT {
            let control = PhysicalControl::trigger(trigger);
            if calibration::is_pressed(self.value(control)) {
                controls.push(control);
            }
        }
    }
}

/// A hub whose device list the test plugs and unplugs (REQ-INP-024).
#[derive(Default)]
pub struct FakeGamepadHub {
    devices: Vec<Box<dyn zax_platform::input::GamepadDevice>>,
    /// How many times `poll` ran.
    pub polls: u32,
}

impl FakeGamepadHub {
    /// Plugs `device` in, unless its identity is a VR one.
    pub fn plug(&mut self, device: &FakeGamepad) -> &mut Self {
        // REQ-INP-022: the hub is what filters VR devices out, so they are never offered.
        if !zax_platform::input::devices::is_virtual_reality(&device.identity) {
            self.devices.push(Box::new(device.clone()));
        }
        self
    }

    /// Unplugs the device of that identity (REQ-INP-024).
    pub fn unplug(&mut self, identity: &str) -> &mut Self {
        self.devices
            .retain(|device| !device.identity().eq_ignore_ascii_case(identity));
        self
    }
}

impl zax_platform::input::GamepadHub for FakeGamepadHub {
    fn devices(&self) -> &[Box<dyn zax_platform::input::GamepadDevice>] {
        &self.devices
    }

    fn poll(&mut self) {
        self.polls += 1;
    }
}
