//! Helpers of the engine tests, ported from `TestSupport.cs` and `GeometrySupport.cs`.

pub mod geo;

use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use zax_engine3d::engine::{ConversionEngine, ConversionState};
use zax_engine3d::frame::{ConversionFrame, FrameInstance, FrameInstanceId};
use zax_engine3d::geometry::builder::{GeometryProgress, PatternGeometryBuilder};
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::{MeshVertex, TileMesh};
use zax_engine3d::image::{TileImage, TileImageKey};
use zax_engine3d::instance::{InstanceRef, PatternInstance};
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::registry::PatternId;
use zax_engine3d::script::{ScriptError, ScriptHost};
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::vectors::{Vec2, Vec3};
use zax_nes::{FrameBatch, TileRecord};
use zax_profiles::enums::{ProfileLayer, TileFlip, VariantRenderStyle};
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::settings::ProfileSettings;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;

/// The tile images the test vectors of SPEC-04 §5 use.
///
/// The conventions of §5 call A, B, C… "distinct full images (64 pixels, colour 1)". A full
/// tile of nothing but colour 1 is unique, so `full` keeps all 64 pixels opaque and makes the
/// identity unique with one marker pixel in colour 3.
pub mod img {
    use super::TileImageKey;

    /// A full image whose identity is `id` (0 to 63).
    pub fn full(id: u32) -> TileImageKey {
        assert!(id < 64);
        TileImageKey::new(u64::MAX, 1 << id, false, false)
    }

    /// The empty image: both planes zero.
    pub const EMPTY: TileImageKey = TileImageKey::new(0, 0, false, false);

    /// A single-pixel image: colour 1 at the source pixel (`x`, `y`), used by TV-E3D-10.
    pub fn pixel(x: u32, y: u32, flip_x: bool, flip_y: bool) -> TileImageKey {
        TileImageKey::new(1 << ((y << 3) | (7 - x)), 0, flip_x, flip_y)
    }

    /// The same image shown flipped, which is another image (SPEC-04 §2).
    pub fn flipped(image: TileImageKey, flip_x: bool, flip_y: bool) -> TileImageKey {
        TileImageKey { flip_x, flip_y, ..image }
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

    pub fn u() -> TileImageKey {
        full(4)
    }
}

/// Builds a batch of the core (SPEC-02 REQ-CORE-030) and hands it to an engine.
#[derive(Debug, Clone, Default)]
pub struct Batch {
    records: Vec<TileRecord>,
    palette: [u8; 32],
    sprite_ord: i32,
    mask_x: i32,
    mask_y: i32,
}

#[allow(dead_code)] // Each test module uses its own part of the helpers.
impl Batch {
    pub fn new() -> Self {
        Self::default()
    }

    /// The records, in emission order.
    pub fn records(&self) -> &[TileRecord] {
        &self.records
    }

    /// Sets the mask the core reports (REQ-CORE-046).
    pub fn mask(&mut self, x: i32, y: i32) -> &mut Self {
        self.mask_x = x;
        self.mask_y = y;
        self
    }

    /// Sets one of the 32 PPU palette bytes.
    pub fn palette(&mut self, index: usize, value: u8) -> &mut Self {
        self.palette[index] = value;
        self
    }

    /// Adds a background tile at the screen position (x, y).
    pub fn bg(&mut self, image: TileImageKey, x: i32, y: i32, palette: u8) -> &mut Self {
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

    /// Adds a background tile in cell `(column + 1, row + 1)`, that is at the screen position
    /// `(8·column, 8·row)` — the convention of SPEC-04 §5.
    pub fn bg_cell(&mut self, image: TileImageKey, column: i32, row: i32) -> &mut Self {
        self.bg(image, 8 * column, 8 * row, 0)
    }

    /// The same with a palette attribute.
    pub fn bg_cell_pal(&mut self, image: TileImageKey, column: i32, row: i32, palette: u8) -> &mut Self {
        self.bg(image, 8 * column, 8 * row, palette)
    }

    /// Adds a whole row of background cells, left to right from `column`.
    pub fn bg_row(&mut self, row: i32, column: i32, images: &[TileImageKey]) -> &mut Self {
        for (i, image) in images.iter().enumerate() {
            self.bg_cell(*image, column + i32::try_from(i).unwrap(), row);
        }
        self
    }

    /// Adds a sprite tile at the screen position (x, y), in emission order.
    pub fn sprite(&mut self, image: TileImageKey, x: i32, y: i32) -> &mut Self {
        self.sprite_full(image, x, y, 0, false)
    }

    /// Adds a sprite tile with its palette and priority bit.
    pub fn sprite_full(&mut self, image: TileImageKey, x: i32, y: i32, palette: u8, hidden: bool) -> &mut Self {
        self.records.push(TileRecord::new(
            image.low,
            image.high,
            i16::try_from(x).unwrap(),
            i16::try_from(y).unwrap(),
            self.sprite_ord,
            image.flip_x,
            image.flip_y,
            hidden,
            palette,
            false,
        ));
        self.sprite_ord += 1;
        self
    }

    /// Converts the batch with `engine` and publishes nothing.
    pub fn convert<'a>(&self, engine: &'a mut ConversionEngine, counter: i32) -> &'a ConversionFrame {
        let batch = FrameBatch {
            records: &self.records,
            palette: self.palette,
            mask_x: self.mask_x,
            mask_y: self.mask_y,
        };
        engine.convert(&batch, counter)
    }

    /// Converts the batch and publishes the frame (REQ-E3D-280).
    pub fn convert_and_publish<'a>(&self, engine: &'a mut ConversionEngine, counter: i32) -> &'a ConversionFrame {
        self.convert(engine, counter);
        engine.publish()
    }

    /// A full frame: 33 × 31 background tiles and 64 sprites (REQ-E3D-360).
    pub fn full_frame(distinct_images: u32) -> Self {
        let mut batch = Self::new();
        let mut n = 0;
        for row in 0..31 {
            for column in 0..33 {
                let palette = u8::try_from((row + column) & 3).unwrap();
                batch.bg_cell_pal(img::full(n % distinct_images), column, row, palette);
                n += 1;
            }
        }

        for i in 0..64 {
            let palette = u8::try_from(i & 3).unwrap();
            let id = u32::try_from(i).unwrap() % distinct_images;
            batch.sprite_full(img::full(id), (i * 3) % 248, 8 + ((i * 5) % 224), palette, false);
        }

        batch
    }
}

/// The optional parts of a pattern of [`ProfileBuilder`].
#[derive(Debug, Clone)]
pub struct PatternOptions {
    pub self_connected: bool,
    pub skip_geometry: bool,
    pub constraint_width: i32,
    pub constraint_height: i32,
    pub permanent: bool,
    pub tags: Option<String>,
    pub depth: i32,
    pub layer: ProfileLayer,
    pub instance_count: usize,
}

impl Default for PatternOptions {
    fn default() -> Self {
        Self {
            self_connected: true,
            skip_geometry: false,
            constraint_width: 99,
            constraint_height: 99,
            permanent: true,
            tags: None,
            depth: 3,
            layer: ProfileLayer::L1,
            instance_count: 1,
        }
    }
}

/// Builds a profile whose patterns the registry can load (REQ-E3D-060).
#[derive(Debug, Clone)]
pub struct ProfileBuilder {
    profile: GameProfile,
}

impl Default for ProfileBuilder {
    fn default() -> Self {
        Self {
            profile: GameProfile::create_empty(),
        }
    }
}

#[allow(dead_code)] // Each test module uses its own part of the helpers.
impl ProfileBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The profile settings, so that a test can change one.
    pub fn settings(&mut self) -> &mut ProfileSettings {
        self.profile.settings.as_mut().unwrap()
    }

    /// Adds a pattern of one row: the `[A B]` of the test vectors.
    pub fn row(&mut self, cells: &[Option<TileImageKey>]) -> &mut Self {
        self.row_with(cells, PatternOptions::default())
    }

    /// Adds a pattern of one row with options.
    pub fn row_with(&mut self, cells: &[Option<TileImageKey>], options: PatternOptions) -> &mut Self {
        let width = i32::try_from(cells.len()).unwrap();
        self.pattern_with(width, 1, cells, options)
    }

    /// Adds a pattern.
    pub fn pattern(&mut self, width: i32, height: i32, cells: &[Option<TileImageKey>]) -> &mut Self {
        self.pattern_with(width, height, cells, PatternOptions::default())
    }

    /// Adds a pattern with options.
    pub fn pattern_with(
        &mut self,
        width: i32,
        height: i32,
        cells: &[Option<TileImageKey>],
        options: PatternOptions,
    ) -> &mut Self {
        let tiles = cells
            .iter()
            .map(|cell| {
                cell.map(|key| {
                    let flip = if key.flip_x { TileFlip::HORIZONTAL } else { TileFlip::NONE }
                        | if key.flip_y { TileFlip::VERTICAL } else { TileFlip::NONE };
                    Arc::new(ProfileTile::new(key.low, key.high, flip))
                })
            })
            .collect();

        let mut pattern = ProfilePattern {
            grid: Some(PatternGrid::new(width, height, tiles)),
            tile_size: Some(Vec2I::new(width, height)),
            permanent: options.permanent,
            self_connected: options.self_connected,
            skip_geometry: options.skip_geometry,
            constraint_size: Vec2I::new(options.constraint_width, options.constraint_height),
            ..ProfilePattern::default()
        };

        for _ in 0..options.instance_count {
            pattern.variants.push(Arc::new(PatternVariant {
                depth: options.depth,
                layer: options.layer,
                alpha: 1.0,
                render_style: VariantRenderStyle::Normal,
                tag_string: options.tags.clone(),
                ..PatternVariant::default()
            }));
        }

        if options.permanent {
            self.profile.permanent_patterns.push(Arc::new(pattern));
        } else {
            self.profile.temporary_patterns.push(Arc::new(pattern));
        }

        self
    }

    /// The pattern `index` of the permanent list, for changing it before a load.
    pub fn permanent_pattern(&mut self, index: usize) -> &mut ProfilePattern {
        Arc::make_mut(&mut self.profile.permanent_patterns[index])
    }

    /// The variant `variant` of the permanent pattern `pattern`, for changing it before a load.
    pub fn variant(&mut self, pattern: usize, variant: usize) -> &mut PatternVariant {
        Arc::make_mut(&mut self.permanent_pattern(pattern).variants[variant])
    }

    /// The profile.
    pub fn build(&self) -> GameProfile {
        self.profile.clone()
    }

    /// An engine with this profile loaded.
    pub fn load(&self) -> ConversionEngine {
        let mut engine = ConversionEngine::new();
        engine.load_profile(&self.profile, true).unwrap();
        engine
    }

    /// An engine with no pattern at all, the "empty profile" of the vectors.
    pub fn empty_engine() -> ConversionEngine {
        let mut engine = ConversionEngine::new();
        engine.load_profile(&GameProfile::create_empty(), true).unwrap();
        engine
    }
}

/// Shorthand for a list of present cells.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn cells(keys: &[TileImageKey]) -> Vec<Option<TileImageKey>> {
    keys.iter().copied().map(Some).collect()
}

/// The handle of the permanent pattern `index` of an engine.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn permanent(engine: &ConversionEngine, index: usize) -> PatternId {
    engine.patterns().permanent()[index]
}

/// The pattern of a handle.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn pattern(engine: &ConversionEngine, id: PatternId) -> &EnginePattern {
    engine.patterns().get(id).unwrap()
}

/// The instances of a frame, in order.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn instances(frame: &ConversionFrame) -> Vec<&FrameInstance> {
    frame.instances().iter().map(|&id| frame.instance(id)).collect()
}

type StateHook = Box<dyn FnMut(&mut ConversionState)>;
type ShapeHook = Box<dyn FnMut(&mut FrameInstance) -> Result<(), ScriptError>>;
type PatternHook = Box<dyn FnMut(&mut PatternInstance)>;

/// A script host that records what the pipeline calls and lets a test run code at each call
/// point (REQ-E3D-320).
#[derive(Default)]
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub struct RecordingScriptHost {
    /// The call log, in order.
    pub calls: Vec<String>,
    /// The frame counter of the last `set_frame_globals`.
    pub last_frame: Option<i32>,
    /// The frame time of the last `set_frame_globals`.
    pub last_time: Option<f32>,
    /// The instances the last `UpdateS` round received, in order.
    pub update_shapes_argument: Vec<FrameInstanceId>,
    /// The pattern instances `BuildPattern` was called with, in order.
    pub built_patterns: Vec<InstanceRef>,
    /// Runs during `Update`.
    pub on_update: Option<StateHook>,
    /// Runs once per scriptable instance, in the order of REQ-E3D-220.
    pub on_update_shape: Option<ShapeHook>,
    /// Runs during `LateUpdate`.
    pub on_late_update: Option<StateHook>,
    /// Runs during `BuildPattern`.
    pub on_build_pattern: Option<PatternHook>,
}

impl ScriptHost for RecordingScriptHost {
    fn has_frame_hooks(&self) -> bool {
        true
    }

    fn set_frame_globals(&mut self, frame_counter: i32, time: f32) -> Result<(), ScriptError> {
        self.last_frame = Some(frame_counter);
        self.last_time = Some(time);
        self.calls.push(format!("globals({frame_counter})"));
        Ok(())
    }

    fn invoke_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        self.calls.push("Update".to_owned());
        if let Some(hook) = &mut self.on_update {
            hook(state);
        }
        Ok(())
    }

    fn invoke_update_shapes(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        let frame = state.converting_frame_mut();
        let scriptable = frame.scriptable_instances().to_vec();
        self.calls.push(format!("UpdateS[{}]", scriptable.len()));
        self.update_shapes_argument.clone_from(&scriptable);
        if let Some(hook) = &mut self.on_update_shape {
            for id in scriptable {
                hook(frame.instance_mut(id))?;
            }
        }
        Ok(())
    }

    fn invoke_late_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        self.calls.push("LateUpdate".to_owned());
        if let Some(hook) = &mut self.on_late_update {
            hook(state);
        }
        Ok(())
    }

    fn invoke_build_pattern(&mut self, reference: InstanceRef, instance: &mut PatternInstance) -> Result<(), ScriptError> {
        self.calls.push("BuildPattern".to_owned());
        self.built_patterns.push(reference);
        if let Some(hook) = &mut self.on_build_pattern {
            hook(instance);
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// The recording script host of an engine.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn host(engine: &ConversionEngine) -> &RecordingScriptHost {
    engine.script_host_as::<RecordingScriptHost>().unwrap()
}

/// The recording script host of an engine, for writing.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn host_mut(engine: &mut ConversionEngine) -> &mut RecordingScriptHost {
    engine.script_host_as_mut::<RecordingScriptHost>().unwrap()
}

/// A geometry builder that records what the pipeline asks of it and lets a test drive the
/// readiness and the progress (REQ-E3D-340, REQ-E3D-361, REQ-E3D-362).
#[derive(Debug)]
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub struct RecordingGeometryBuilder {
    /// The patterns built, in order; a pattern rebuilt appears twice.
    pub built: Vec<PatternId>,
    /// The patterns released, in order.
    pub released: Vec<PatternId>,
    /// Instances declared not ready (REQ-E3D-361).
    pub not_ready: HashSet<InstanceRef>,
    /// What `progress` returns.
    pub reported_progress: GeometryProgress,
    /// The object counts `begin_mass_build` was told, in order.
    pub mass_builds: Vec<usize>,
    /// The mesh it hands out for every tile.
    pub stub: Arc<TileMesh>,
}

impl Default for RecordingGeometryBuilder {
    fn default() -> Self {
        let normal = Vec3::new(0.0, 0.0, -1.0);
        // A mesh that stands in for a real one: one triangle, so it is not empty.
        let stub = TileMesh::new(
            vec![
                MeshVertex::new(Vec3::ZERO, normal, Vec2::ZERO),
                MeshVertex::new(Vec3::new(1.0, 0.0, 0.0), normal, Vec2::ZERO),
                MeshVertex::new(Vec3::new(0.0, 1.0, 0.0), normal, Vec2::ZERO),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        Self {
            built: Vec::new(),
            released: Vec::new(),
            not_ready: HashSet::new(),
            reported_progress: GeometryProgress::default(),
            mass_builds: Vec::new(),
            stub: Arc::new(stub),
        }
    }
}

impl PatternGeometryBuilder for RecordingGeometryBuilder {
    fn build(&mut self, id: PatternId, _pattern: &mut EnginePattern, _global: RenderAlgorithm) {
        self.built.push(id);
    }

    fn release(&mut self, id: PatternId, _pattern: &EnginePattern) {
        self.released.push(id);
    }

    fn is_ready(&self, instance: InstanceRef) -> bool {
        !self.not_ready.contains(&instance)
    }

    fn mesh_of(&mut self, _instance: InstanceRef, _image: &TileImage, _sealed_sides: u8) -> Option<Arc<TileMesh>> {
        Some(Arc::clone(&self.stub))
    }

    fn full_sides_of(&self, _instance: InstanceRef, _image: &TileImage) -> u8 {
        0
    }

    fn texture_of(&self, _instance: InstanceRef) -> Option<Arc<ObjectTexture>> {
        None
    }

    fn progress(&self) -> GeometryProgress {
        self.reported_progress
    }

    fn begin_mass_build(&mut self, object_count: usize) {
        self.mass_builds.push(object_count);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// The recording geometry builder of an engine.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn recorder(engine: &ConversionEngine) -> &RecordingGeometryBuilder {
    engine.geometry_as::<RecordingGeometryBuilder>().unwrap()
}

/// The recording geometry builder of an engine, for writing.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn recorder_mut(engine: &mut ConversionEngine) -> &mut RecordingGeometryBuilder {
    engine.geometry_as_mut::<RecordingGeometryBuilder>().unwrap()
}

/// Asserts that two values agree to `digits` decimal places, as xUnit's
/// `Assert.Equal(expected, actual, precision)` does.
#[allow(dead_code)] // Each test module uses its own part of the helpers.
pub fn assert_close(expected: f32, actual: f32, digits: i32) {
    let tolerance = 0.5 * 10f32.powi(-digits);
    assert!(
        (expected - actual).abs() <= tolerance,
        "expected {expected}, got {actual} (to {digits} digits)"
    );
}
