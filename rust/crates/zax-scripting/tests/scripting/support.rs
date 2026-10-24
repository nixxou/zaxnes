//! Helpers of the scripting tests, ported from `TestSupport.cs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use zax_engine3d::engine::{ConversionEngine, ConversionState};
use zax_engine3d::frame::ConversionFrame;
use zax_engine3d::image::TileImageKey;
use zax_nes::{FrameBatch, TileRecord};
use zax_profiles::enums::{ProfileLayer, TileFlip, VariantRenderStyle};
use zax_profiles::format;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::settings::ProfileSettings;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;
use zax_scripting::{
    CollectingScriptLog, LuaScriptError, ProfileScriptRuntime, ScriptEnvironment, ScriptLight,
    ScriptNotification, ScriptProfileInfo, ScriptSettings,
};
use zax_settings::model::SettingsModel;

/// The tile images the synthetic frames use, as in the SPEC-04 test vectors.
pub mod img {
    use super::TileImageKey;

    /// A full image whose identity is `id` (0 to 63).
    pub fn full(id: u32) -> TileImageKey {
        assert!(id < 64);
        TileImageKey::new(u64::MAX, 1 << id, false, false)
    }

    pub fn a() -> TileImageKey {
        full(0)
    }

    pub fn b() -> TileImageKey {
        full(1)
    }

    pub fn c() -> TileImageKey {
        full(2)
    }

    pub fn d() -> TileImageKey {
        full(3)
    }
}

/// Builds a batch of the core and hands it to an engine (SPEC-02 REQ-CORE-030).
#[derive(Debug, Clone, Default)]
pub struct Batch {
    records: Vec<TileRecord>,
    palette: [u8; 32],
    sprite_ord: i32,
}

impl Batch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets one of the 32 PPU palette bytes.
    pub fn palette(mut self, index: usize, value: u8) -> Self {
        self.palette[index] = value;
        self
    }

    /// Adds a background tile at the screen position (x, y).
    pub fn bg(mut self, image: TileImageKey, x: i32, y: i32, palette: u8) -> Self {
        let index = i32::try_from(self.records.len()).unwrap();
        self.records.push(TileRecord::new(
            image.low,
            image.high,
            i16::try_from(x).unwrap(),
            i16::try_from(y).unwrap(),
            index,
            image.flip_x,
            image.flip_y,
            false,
            palette,
            true,
        ));
        self
    }

    /// Adds a background tile in cell `(column + 1, row + 1)`.
    pub fn bg_cell(self, image: TileImageKey, column: i32, row: i32) -> Self {
        self.bg(image, 8 * column, 8 * row, 0)
    }

    /// Adds a background tile in a cell with a palette attribute.
    pub fn bg_cell_palette(self, image: TileImageKey, column: i32, row: i32, palette: u8) -> Self {
        self.bg(image, 8 * column, 8 * row, palette)
    }

    /// Adds a sprite tile at the screen position (x, y), in emission order.
    pub fn sprite(self, image: TileImageKey, x: i32, y: i32) -> Self {
        self.sprite_with(image, x, y, 0)
    }

    /// Adds a sprite tile with a palette attribute.
    pub fn sprite_with(mut self, image: TileImageKey, x: i32, y: i32, palette: u8) -> Self {
        self.records.push(TileRecord::new(
            image.low,
            image.high,
            i16::try_from(x).unwrap(),
            i16::try_from(y).unwrap(),
            self.sprite_ord,
            image.flip_x,
            image.flip_y,
            false,
            palette,
            false,
        ));
        self.sprite_ord += 1;
        self
    }

    /// Converts the batch with `engine`.
    pub fn convert<'a>(
        &self,
        engine: &'a mut ConversionEngine,
        counter: i32,
    ) -> &'a ConversionFrame {
        let batch = FrameBatch {
            records: &self.records,
            palette: self.palette,
            mask_x: 0,
            mask_y: 0,
            margin: 0,
            visible_rows: zax_nes::STANDARD_ROWS,
            margin_y: 0,
        };
        engine.convert(&batch, counter)
    }

    /// Converts and publishes the batch (REQ-E3D-280).
    pub fn convert_and_publish<'a>(
        &self,
        engine: &'a mut ConversionEngine,
        counter: i32,
    ) -> &'a ConversionFrame {
        self.convert(engine, counter);
        engine.publish()
    }
}

/// How a pattern of [`ScriptProfileBuilder`] is made.
#[derive(Debug, Clone)]
pub struct PatternSpec {
    pub tags: Option<String>,
    pub instance_count: usize,
    pub depth: i32,
    pub permanent: bool,
}

impl Default for PatternSpec {
    fn default() -> Self {
        Self {
            tags: None,
            instance_count: 1,
            depth: 3,
            permanent: true,
        }
    }
}

impl PatternSpec {
    pub fn tagged(tags: &str) -> Self {
        Self {
            tags: Some(tags.to_owned()),
            ..Self::default()
        }
    }
}

/// Builds a profile with scripts and patterns and starts a runtime on it in the order of
/// REQ-LUA-015: chunks, then the profile settings and the patterns with their `BuildPattern`
/// hooks, then `Start`.
#[derive(Debug, Clone)]
pub struct ScriptProfileBuilder {
    profile: GameProfile,
}

impl Default for ScriptProfileBuilder {
    fn default() -> Self {
        Self {
            profile: GameProfile::create_empty(),
        }
    }
}

impl ScriptProfileBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The profile settings, so that a test can change one.
    pub fn settings(&mut self) -> &mut ProfileSettings {
        self.profile.settings.as_mut().unwrap()
    }

    /// Adds a script linked to every instance, in file order.
    pub fn script(self, name: &str, source: &str) -> Self {
        self.script_with(name, source, "*", true)
    }

    /// Adds a script with its tags and its `enable` flag, in file order.
    pub fn script_with(mut self, name: &str, source: &str, tags: &str, enabled: bool) -> Self {
        self.profile.scripts.push(ProfileScript::new(
            Some(name),
            enabled,
            Some(source),
            Some(tags),
        ));
        self
    }

    /// Adds a script that runs **first** and defines no hook, so that the others can share a
    /// global it sets up. Its name sorts before every letter, so it is also first in script
    /// order (REQ-LUA-004).
    pub fn prelude(mut self, source: &str) -> Self {
        self.profile.scripts.insert(
            0,
            ProfileScript::new(Some("!prelude"), true, Some(source), Some("")),
        );
        self
    }

    /// Adds a pattern of one row.
    pub fn pattern(self, cells: &[TileImageKey], spec: PatternSpec) -> Self {
        let width = i32::try_from(cells.len()).unwrap();
        self.grid(width, 1, cells, spec)
    }

    /// Adds a pattern over a `width` × `height` grid.
    pub fn grid(
        mut self,
        width: i32,
        height: i32,
        cells: &[TileImageKey],
        spec: PatternSpec,
    ) -> Self {
        let PatternSpec {
            tags,
            instance_count,
            depth,
            permanent,
        } = spec;
        let tiles = cells
            .iter()
            .map(|key| {
                Some(Arc::new(ProfileTile::new(
                    key.low,
                    key.high,
                    TileFlip::NONE,
                )))
            })
            .collect();

        let mut pattern = ProfilePattern {
            grid: Some(PatternGrid::new(width, height, tiles)),
            tile_size: Some(Vec2I::new(width, height)),
            permanent,
            self_connected: true,
            constraint_size: Vec2I::new(99, 99),
            ..ProfilePattern::default()
        };

        for _ in 0..instance_count {
            pattern.variants.push(Arc::new(PatternVariant {
                depth,
                layer: ProfileLayer::L1,
                alpha: 1.0,
                render_style: VariantRenderStyle::Normal,
                tag_string: tags.clone(),
                ..PatternVariant::default()
            }));
        }

        if permanent {
            self.profile.permanent_patterns.push(Arc::new(pattern));
        } else {
            self.profile.temporary_patterns.push(Arc::new(pattern));
        }

        self
    }

    /// The profile built so far.
    pub fn profile(&self) -> &GameProfile {
        &self.profile
    }

    /// Starts a session on this profile without calling `Start`.
    pub fn load(&self) -> ScriptSession {
        ScriptSession::create(&self.profile)
    }

    /// Starts a session and calls `Start` of every script (REQ-LUA-020).
    pub fn start(&self) -> ScriptSession {
        let mut session = self.load();
        session.start();
        session
    }
}

/// One engine, one runtime and the collected log: what a test drives. It follows the load
/// order of REQ-LUA-015, which is what the host does.
pub struct ScriptSession {
    pub engine: ConversionEngine,
    pub log: CollectingScriptLog,
    notifications: Receiver<ScriptNotification>,
}

impl ScriptSession {
    /// Builds engine and runtime for `profile` (REQ-LUA-015).
    pub fn create(profile: &GameProfile) -> Self {
        let mut engine = ConversionEngine::new();
        engine.two_dimensional_mode =
            profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty();

        let log = CollectingScriptLog::new();
        let (sender, notifications) = channel();
        let environment = ScriptEnvironment {
            settings: ScriptSettings::new(SettingsModel::new(), ScriptLight::detached()),
            log: Box::new(log.clone()),
            notify: Some(Box::new(move |notification| {
                sender.send(notification).unwrap();
            })),
            profile: ScriptProfileInfo {
                title: profile.title.clone().unwrap_or_default(),
                maker: profile.author.clone().unwrap_or_default(),
                game_dev: profile.publisher.clone().unwrap_or_default(),
                description: profile.description.clone().unwrap_or_default(),
                id: profile.id.clone().unwrap_or_default(),
                version: profile.version_text.clone().unwrap_or_default(),
            },
            ..ScriptEnvironment::default()
        };

        // 1. the chunks, 2. the profile settings and 3. the patterns with BuildPattern.
        let runtime =
            ProfileScriptRuntime::create(profile, environment, engine.state_mut()).unwrap();
        engine.set_script_host(Box::new(runtime));
        engine.load_profile(profile, true).unwrap();

        Self {
            engine,
            log,
            notifications,
        }
    }

    /// The notifications the scripts raised since the last call.
    pub fn notifications(&self) -> Vec<ScriptNotification> {
        self.notifications.try_iter().collect()
    }

    /// Runs `operation` on the runtime together with the engine state.
    pub fn runtime<R>(
        &mut self,
        operation: impl FnOnce(&mut ProfileScriptRuntime, &mut ConversionState) -> R,
    ) -> R {
        self.engine
            .with_script_host::<ProfileScriptRuntime, R>(operation)
            .unwrap()
    }

    /// Converts one batch, which runs the frame hooks of REQ-LUA-025.
    pub fn frame(&mut self, batch: &Batch, counter: i32) {
        batch.convert_and_publish(&mut self.engine, counter);
    }

    pub fn start(&mut self) {
        self.runtime(ProfileScriptRuntime::invoke_start);
    }

    pub fn end(&mut self) {
        self.runtime(ProfileScriptRuntime::invoke_end);
    }

    pub fn save(&mut self) {
        self.runtime(ProfileScriptRuntime::invoke_save);
    }

    pub fn load(&mut self) {
        self.runtime(ProfileScriptRuntime::invoke_load);
    }

    /// Runs a chunk in the profile's state; the chunk is expected to succeed.
    pub fn evaluate(&mut self, chunk: &str) {
        self.try_evaluate(chunk).unwrap();
    }

    /// Runs a chunk in the profile's state.
    pub fn try_evaluate(&mut self, chunk: &str) -> Result<(), LuaScriptError> {
        self.runtime(|runtime, state| runtime.evaluate(state, chunk))
    }

    /// Runs a chunk that returns one value and gives it back as a number.
    pub fn number(&mut self, chunk: &str) -> f64 {
        self.runtime(|runtime, state| runtime.evaluate_number(state, chunk))
            .unwrap()
    }

    /// Runs a chunk that returns one value and gives it back as text.
    pub fn text(&mut self, chunk: &str) -> String {
        self.runtime(|runtime, state| runtime.evaluate_text(state, chunk))
            .unwrap()
    }

    /// 1 when `expression` is true in the state, 0 when it is not.
    pub fn truth(&mut self, expression: &str) -> f64 {
        self.number(&format!("if {expression} then return 1 else return 0 end"))
    }

    /// Publishes the frame counter and the frame time as the globals `frame` and `time`.
    pub fn set_frame_globals(&mut self, counter: i32, time: f32) {
        self.runtime(|runtime, _| {
            zax_engine3d::script::ScriptHost::set_frame_globals(runtime, counter, time).unwrap();
        });
    }

    /// The entries of the global table `log`, in order.
    pub fn log_entries(&mut self) -> Vec<String> {
        let joined = self.text("return table.concat(log, '\\n')");
        joined.lines().map(str::to_owned).collect()
    }

    /// Whether the script of this name is in error.
    pub fn script_failed(&mut self, name: &str) -> bool {
        self.runtime(|runtime, _| runtime.find_script(name).unwrap().failed())
    }
}

/// The folder that holds the 101 built-in profiles (`ZAXNES_PROFILES`).
pub mod neutral_assets {
    use super::{format, fs, GameProfile, Path, PathBuf};

    /// The folder that holds the `.bytes` profiles.
    pub fn profiles_directory() -> PathBuf {
        let root = std::env::var_os("ZAXNES_PROFILES").unwrap_or_else(|| {
            panic!("ZAXNES_PROFILES is not set: it must name the folder that holds profiles/")
        });
        PathBuf::from(root).join("profiles")
    }

    /// The built-in profile files, ordered by file name.
    pub fn profile_files() -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = fs::read_dir(profiles_directory())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "bytes")
            })
            .collect();
        files.sort();
        files
    }

    /// Loads one built-in profile from its full path.
    pub fn load_path(path: &Path) -> GameProfile {
        let name = path.display().to_string();
        format::load(&fs::read(path).unwrap(), None, Some(&name))
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .profile
    }

    /// Loads the built-in profile whose file name starts with `prefix`.
    pub fn load_named(prefix: &str) -> Option<GameProfile> {
        profile_files().into_iter().find_map(|path| {
            let stem = path.file_stem()?.to_string_lossy().to_lowercase();
            stem.starts_with(&prefix.to_lowercase())
                .then(|| load_path(&path))
        })
    }
}

/// xUnit's `Assert.Equal(expected, actual, precision)`: both rounded to `precision` decimals.
pub fn assert_close(expected: f64, actual: f64, precision: i32) {
    let scale = 10_f64.powi(precision);
    let round = |value: f64| (value * scale).round_ties_even() / scale;
    assert_eq!(
        round(expected),
        round(actual),
        "{expected} is not {actual} to {precision} decimals"
    );
}
