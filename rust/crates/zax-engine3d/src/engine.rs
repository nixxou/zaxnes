// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The conversion pipeline of SPEC-04: it turns each batch of the core into a frame of
//! recognised shapes and instances, ready for the geometry of SPEC-04B.

use std::ops::{Deref, DerefMut};

use zax_nes::{FrameBatch, TileRecord};
use zax_profiles::profile::GameProfile;
use zax_profiles::values::Vec4F;

use crate::draw_list::{DrawListBuilder, DrawSources, FrameDrawList};
use crate::error::EngineError;
use crate::frame::{ConversionFrame, FrameInstanceId, ShapeId};
use crate::geometry::builder::{
    GeometryBuilder, NullPatternGeometryBuilder, PatternGeometryBuilder,
};
use crate::grid;
use crate::image::{ImageRegistry, TileImageKey};
use crate::instance::InstanceRef;
use crate::pattern::EnginePattern;
use crate::palette::Rgb24;
use crate::registry::{PatternId, PatternRegistry};
use crate::script::{NullScriptHost, ScriptHost};
use crate::segmenter::{SegmentContext, Segmenter};
use crate::settings::{ConversionSettings, RenderAlgorithm};
use crate::tracker::ShapeTracker;
use crate::vectors::{Cell, Rect, Vec3};

/// Which of the engine's frames a call is about (REQ-E3D-280, REQ-E3D-233).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameRole {
    /// The frame being converted; the one script queries see (SPEC-05 REQ-LUA-084).
    Converting,
    /// The most recently published frame, the one the renderer shows.
    Published,
    /// The pinned frame.
    Pinned,
}

/// Everything the conversion keeps between two calls except the script and geometry sides:
/// the settings, the registries, the frames and the tracker. The script hooks receive it,
/// since it is what the script API reads and changes (SPEC-05 REQ-LUA-084 to 086).
#[derive(Debug)]
pub struct ConversionState {
    settings: ConversionSettings,
    images: ImageRegistry,
    patterns: PatternRegistry,
    frames: Vec<ConversionFrame>,
    converting: usize,
    published: Option<usize>,
    pinned: Option<usize>,
    tracker: ShapeTracker,
    profile_is_version_2: bool,
    profile_image_count: usize,
    /// True in 2D mode, the library entry that plays any ROM without a game profile
    /// (REQ-E3D-380, REQ-E3D-381).
    pub two_dimensional_mode: bool,
}

/// The conversion pipeline of SPEC-04: it turns each batch of the core into a frame of
/// recognised shapes and instances, ready for the geometry of SPEC-04B.
///
/// One instance per emulated game; it holds no global mutable state, so two engines in the
/// same process are independent (REQ-ARCH-011) and a run is reproducible (REQ-ARCH-012).
///
/// The host drives it: load a profile, then call [`ConversionEngine::convert`] once per
/// emulated frame and [`ConversionEngine::publish`] to hand the frame to the renderer (SPEC-07
/// REQ-LOOP-060). The engine dereferences to its [`ConversionState`], which holds the script
/// API.
pub struct ConversionEngine {
    state: ConversionState,
    host: Box<dyn ScriptHost>,
    geometry: Box<dyn PatternGeometryBuilder>,
    segmenter: Segmenter,
    draw_list_builder: DrawListBuilder,
}

impl Default for ConversionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ConversionEngine {
    type Target = ConversionState;

    fn deref(&self) -> &ConversionState {
        &self.state
    }
}

impl DerefMut for ConversionEngine {
    fn deref_mut(&mut self) -> &mut ConversionState {
        &mut self.state
    }
}

/// Runs `BuildPattern` for every instance of the pattern, then builds its geometry
/// (REQ-E3D-322: the hook always runs before the construction).
pub(crate) fn build_geometry(
    patterns: &mut PatternRegistry,
    id: PatternId,
    host: &mut dyn ScriptHost,
    geometry: &mut dyn PatternGeometryBuilder,
    global: RenderAlgorithm,
) {
    let Some(pattern) = patterns.get_mut(id) else {
        return;
    };

    for (index, instance) in pattern.instances_mut().iter_mut().enumerate() {
        // A Lua error must not stop the conversion (REQ-E3D-324); the runtime disables the
        // faulty script (SPEC-05 §3.8), so the error is dropped here as the C# engine does.
        let _ = host.invoke_build_pattern(InstanceRef { pattern: id, index }, instance);
    }

    geometry.build(id, pattern, global);
}

impl ConversionEngine {
    /// Creates an engine with the factory settings of SPEC-04 §4.2.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ConversionState {
                settings: ConversionSettings::default(),
                images: ImageRegistry::new(),
                patterns: PatternRegistry::new(),
                frames: vec![ConversionFrame::new()],
                converting: 0,
                published: None,
                pinned: None,
                tracker: ShapeTracker::new(),
                profile_is_version_2: false,
                profile_image_count: 0,
                two_dimensional_mode: false,
            },
            host: Box::new(NullScriptHost),
            geometry: Box::new(NullPatternGeometryBuilder),
            segmenter: Segmenter::default(),
            draw_list_builder: DrawListBuilder::new(),
        }
    }

    /// The engine state.
    #[must_use]
    pub const fn state(&self) -> &ConversionState {
        &self.state
    }

    /// The engine state, for writing.
    pub fn state_mut(&mut self) -> &mut ConversionState {
        &mut self.state
    }

    /// The Lua runtime the pipeline calls (REQ-E3D-320); a no-op host by default.
    #[must_use]
    pub fn script_host(&self) -> &dyn ScriptHost {
        self.host.as_ref()
    }

    /// The Lua runtime, for writing.
    pub fn script_host_mut(&mut self) -> &mut dyn ScriptHost {
        self.host.as_mut()
    }

    /// The script host as its concrete type, or `None` when it is of another type.
    #[must_use]
    pub fn script_host_as<T: 'static>(&self) -> Option<&T> {
        self.host.as_any().downcast_ref()
    }

    /// The script host as its concrete type, for writing.
    pub fn script_host_as_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.host.as_any_mut().downcast_mut()
    }

    /// Replaces the Lua runtime the pipeline calls and returns the previous one.
    pub fn set_script_host(&mut self, host: Box<dyn ScriptHost>) -> Box<dyn ScriptHost> {
        std::mem::replace(&mut self.host, host)
    }

    /// The geometry side; a builder that builds nothing by default.
    #[must_use]
    pub fn geometry(&self) -> &dyn PatternGeometryBuilder {
        self.geometry.as_ref()
    }

    /// The geometry side, for writing.
    pub fn geometry_mut(&mut self) -> &mut dyn PatternGeometryBuilder {
        self.geometry.as_mut()
    }

    /// The geometry side as its concrete type, or `None` when it is of another type.
    #[must_use]
    pub fn geometry_as<T: 'static>(&self) -> Option<&T> {
        self.geometry.as_any().downcast_ref()
    }

    /// The geometry side as its concrete type, for writing.
    pub fn geometry_as_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.geometry.as_any_mut().downcast_mut()
    }

    /// Replaces the geometry side and returns the previous one.
    pub fn set_geometry(
        &mut self,
        geometry: Box<dyn PatternGeometryBuilder>,
    ) -> Box<dyn PatternGeometryBuilder> {
        std::mem::replace(&mut self.geometry, geometry)
    }

    /// Attaches a real geometry builder (SPEC-04B), so that a frame and a profile yield the
    /// full renderable scene description of REQ-E3D-641. The host reaches it through
    /// [`ConversionEngine::geometry_builder_mut`] to drive its transfers and read its
    /// progress.
    pub fn use_geometry(&mut self) {
        self.geometry = Box::new(GeometryBuilder::new());
    }

    /// The geometry builder attached by [`ConversionEngine::use_geometry`], if any.
    #[must_use]
    pub fn geometry_builder(&self) -> Option<&GeometryBuilder> {
        self.geometry_as()
    }

    /// The geometry builder attached by [`ConversionEngine::use_geometry`], for writing.
    pub fn geometry_builder_mut(&mut self) -> Option<&mut GeometryBuilder> {
        self.geometry_as_mut()
    }

    /// Loading progress of the geometry, 0 to 1 (REQ-E3D-362).
    #[must_use]
    pub fn load_progress(&self) -> f32 {
        self.geometry.progress().value()
    }

    /// Loads a profile: applies the settings it carries, registers its patterns in the order
    /// of REQ-E3D-060, then runs `BuildPattern` and builds the geometry of each (REQ-E3D-322,
    /// SPEC-05 REQ-LUA-015 step 3).
    ///
    /// Loading a profile starts from an empty registry, as a change of game does
    /// (REQ-E3D-008). Relaunching the same game without reloading the profile is
    /// [`ConversionEngine::restart`], which keeps what was learned.
    ///
    /// `apply_settings` says whether to copy the profile's settings into the settings. A host
    /// that owns a settings model (SPEC-09) passes false and applies them through the model
    /// instead, which then mirrors them here: that keeps one source of truth, since the model
    /// also carries the values this slice does not (REQ-PERS-005, REQ-PERS-007).
    ///
    /// # Errors
    ///
    /// [`EngineError::PatternSize`] when a pattern of the profile declares an invalid grid.
    pub fn load_profile(
        &mut self,
        profile: &GameProfile,
        apply_settings: bool,
    ) -> Result<(), EngineError> {
        let state = &mut self.state;
        state.patterns.clear(&mut state.images);
        state.images.clear();
        state.clear_frames();
        state.profile_is_version_2 = profile.is_v2_format();
        if apply_settings {
            state.settings.apply_profile(profile.settings.as_ref());
        }

        let loaded = state.patterns.load_profile(profile, &mut state.images);
        state.profile_image_count = state.images.count();
        self.rebuild_all_geometry();
        loaded
    }

    /// The list of tiles to draw for the published frame, or for the frame being converted
    /// when nothing has been published yet (SPEC-04B REQ-E3D-641).
    pub fn build_draw_list(&mut self) -> &FrameDrawList {
        let role = if self.state.published.is_some() {
            FrameRole::Published
        } else {
            FrameRole::Converting
        };
        self.build_draw_list_of(role)
    }

    /// The list of tiles to draw for one of the engine's frames (SPEC-04B REQ-E3D-641); an
    /// empty list when there is no such frame.
    pub fn build_draw_list_of(&mut self, role: FrameRole) -> &FrameDrawList {
        let state = &self.state;
        let Some(index) = state.frame_index(role) else {
            return self.draw_list_builder.list();
        };

        let mut sources = DrawSources {
            settings: &state.settings,
            geometry: self.geometry.as_mut(),
            patterns: &state.patterns,
            format_version_2: state.profile_is_version_2,
        };
        self.draw_list_builder
            .build(&state.frames[index], &mut sources)
    }

    /// Relaunches the same game without reloading the profile: the frames are emptied and the
    /// pinned frame released (REQ-E3D-281), while the image registry, the learned patterns and
    /// the temporary patterns stay active from the first frame (REQ-E3D-008).
    pub fn restart(&mut self) {
        self.state.clear_frames();
    }

    /// Forgets the registry, the learned patterns and the frames: what a change of game or a
    /// return from 2D mode does (REQ-E3D-008, REQ-E3D-380).
    pub fn unload(&mut self) {
        let state = &mut self.state;
        for id in state.patterns.all() {
            if let Some(pattern) = state.patterns.get(id) {
                self.geometry.release(id, pattern);
            }
        }

        state.patterns.clear(&mut state.images);
        state.images.clear();
        state.profile_image_count = 0;
        state.clear_frames();
    }

    /// Rebuilds the geometry of one pattern, hooks included. Used when a savestate's learned
    /// patterns come back (REQ-E3D-008, REQ-E3D-322).
    pub(crate) fn rebuild_geometry_of(&mut self, id: PatternId) {
        build_geometry(
            &mut self.state.patterns,
            id,
            self.host.as_mut(),
            self.geometry.as_mut(),
            self.state.settings.render_algorithm,
        );
    }

    /// Changes the global mesh algorithm and, when it really changes, rebuilds the geometry of
    /// every pattern of the registry (REQ-E3D-340).
    pub fn set_render_algorithm(&mut self, algorithm: RenderAlgorithm) {
        if self.state.settings.render_algorithm == algorithm {
            return;
        }

        self.state.settings.render_algorithm = algorithm;
        self.rebuild_all_geometry();
    }

    /// Converts one batch of the core into a frame, in the order of REQ-E3D-001: palette, tile
    /// filtering and placement, background then sprite segmentation, Lua hooks, instance
    /// positions. The frame is left as the converting frame; call
    /// [`ConversionEngine::publish`] to hand it over. `frame_counter` is the number of emulated
    /// frames since the game started (REQ-E3D-043).
    pub fn convert(&mut self, batch: &FrameBatch<'_>, frame_counter: i32) -> &ConversionFrame {
        let state = &mut self.state;
        let index = state.acquire_frame();
        state.converting = index;
        let frame = &mut state.frames[index];
        frame.begin_batch(frame_counter, Cell::new(batch.mask_x, batch.mask_y));

        // (a) frame palette.
        frame.palette_mut().load_from(&batch.palette);

        // (b) filtering and placement of the tiles.
        intake(frame, &mut state.images, &state.settings, batch.records);

        // (c) segmentation.
        let mut context = SegmentContext {
            patterns: &mut state.patterns,
            images: &mut state.images,
            settings: &state.settings,
            host: self.host.as_mut(),
            geometry: self.geometry.as_mut(),
        };
        self.segmenter.run(
            &mut state.frames[index],
            &mut context,
            state.two_dimensional_mode,
        );

        // (d) Lua hooks.
        self.run_frame_hooks();

        // (e) instance positions.
        self.state.calculate_positions();
        self.state.converting_frame()
    }

    /// Publishes the converted frame: from now on the renderer shows it (REQ-E3D-280). The
    /// next conversion goes to another buffer, so neither the published frame nor the pinned
    /// one is ever touched.
    pub fn publish(&mut self) -> &ConversionFrame {
        self.state.published = Some(self.state.converting);
        self.state.converting_frame()
    }

    /// The Lua hooks of the pipeline, in the order of REQ-E3D-320. An error in a hook is
    /// counted and the conversion carries on (REQ-E3D-324).
    fn run_frame_hooks(&mut self) {
        let state = &mut self.state;
        let host = self.host.as_mut();
        let (counter, time) = {
            let frame = state.converting_frame();
            (frame.counter(), frame.time())
        };

        let results = [
            host.set_frame_globals(counter, time),
            host.invoke_update(state),
            host.invoke_update_shapes(state),
            host.invoke_late_update(state),
        ];
        for result in results {
            if result.is_err() {
                state.converting_frame_mut().count_script_error();
            }
        }
    }

    fn rebuild_all_geometry(&mut self) {
        let state = &mut self.state;

        // The loading progress of SPEC-04B REQ-E3D-600 counts 3D objects, so the builder is
        // told the total before the first one.
        let objects: usize = state
            .patterns
            .all()
            .filter_map(|id| state.patterns.get(id))
            .map(|pattern| pattern.instances().len())
            .sum();
        self.geometry.begin_mass_build(objects);

        let ids: Vec<PatternId> = state.patterns.all().collect();
        for id in ids {
            build_geometry(
                &mut state.patterns,
                id,
                self.host.as_mut(),
                self.geometry.as_mut(),
                state.settings.render_algorithm,
            );
        }
    }
}

/// Filters and places the tiles of a batch: the clip area of REQ-E3D-002, the sprite rules of
/// REQ-E3D-003, the empty background tiles of REQ-E3D-004 and the grid placement of
/// REQ-E3D-005.
fn intake(
    frame: &mut ConversionFrame,
    images: &mut ImageRegistry,
    settings: &ConversionSettings,
    records: &[TileRecord],
) {
    let clip = settings.clipping;
    for record in records {
        let (x, y) = (i32::from(record.x), i32::from(record.y));
        if !is_inside_clip_area(x, y, clip) {
            continue;
        }

        let image = images.intern(TileImageKey::new(
            record.plane0,
            record.plane1,
            record.mirror_h,
            record.mirror_v,
        ));

        if record.background {
            if image.is_empty() && !settings.empty_tile {
                continue;
            }

            let cell = grid::background_cell(x, y);
            if cell.x <= 0 || cell.y <= 0 || cell.x >= grid::WIDTH - 1 || cell.y >= grid::HEIGHT - 1
            {
                // The grid has one cell of margin on each side precisely so that growth never
                // needs a bounds test (SPEC-04 §2). With the clip margins of §4.2 no tile can
                // land there; one that would is dropped.
                continue;
            }

            frame.place_background(image, x, y, i32::from(record.palette), record.hidden);
            continue;
        }

        if y >= 240 || image.key().is_empty() {
            continue;
        }

        frame.add_sprite(
            image,
            x,
            y,
            i32::from(record.palette),
            record.hidden,
            record.index,
        );
    }
}

/// The clip test of REQ-E3D-002: kept when `x + 8 > g`, `x ≤ 256 − d`, `y + 8 > h` and
/// `y ≤ 248 − b`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // C# int-to-float comparisons of screen coordinates.
pub fn is_inside_clip_area(x: i32, y: i32, clipping: Vec4F) -> bool {
    (x + 8) as f32 > clipping.x
        && x as f32 <= 256.0 - clipping.z
        && (y + 8) as f32 > clipping.w
        && y as f32 <= 248.0 - clipping.y
}

impl ConversionState {
    /// The settings the pipeline consumes (SPEC-04 §4.2).
    #[must_use]
    pub const fn settings(&self) -> &ConversionSettings {
        &self.settings
    }

    /// The settings, for writing.
    pub fn settings_mut(&mut self) -> &mut ConversionSettings {
        &mut self.settings
    }

    /// The session's image registry (REQ-E3D-007).
    #[must_use]
    pub const fn images(&self) -> &ImageRegistry {
        &self.images
    }

    /// The pattern registry (REQ-E3D-060).
    #[must_use]
    pub const fn patterns(&self) -> &PatternRegistry {
        &self.patterns
    }

    /// The pattern and image registries, for writing: registering or removing a pattern
    /// updates the candidate lists of its images.
    pub fn registries_mut(&mut self) -> (&mut PatternRegistry, &mut ImageRegistry) {
        (&mut self.patterns, &mut self.images)
    }

    /// The pattern of a handle, for writing (the script API changes recognition flags and
    /// instances in place), or `None` when it was dropped.
    pub fn pattern_mut(&mut self, id: PatternId) -> Option<&mut EnginePattern> {
        self.patterns.get_mut(id)
    }

    /// Removes every permanent pattern that carries `tag` (SPEC-05 `Pattern.RemoveWithTag`).
    pub fn remove_permanent_patterns_with_tag(&mut self, tag: &str) -> usize {
        self.patterns
            .remove_permanent_with_tag(tag, &mut self.images)
    }

    /// How many images the profile's patterns interned. They keep their identity for the
    /// whole session; the images met while playing come after and belong to the learned state
    /// (REQ-E3D-008, DEC-R2-T5).
    #[must_use]
    pub const fn profile_image_count(&self) -> usize {
        self.profile_image_count
    }

    /// True when the loaded profile is in version 2 format, which moves the deformation pivot
    /// by one unit (SPEC-03, SPEC-04B REQ-E3D-568).
    #[must_use]
    pub const fn profile_is_version_2(&self) -> bool {
        self.profile_is_version_2
    }

    /// The frame being converted; the one script queries see (SPEC-05 REQ-LUA-084).
    #[must_use]
    pub fn converting_frame(&self) -> &ConversionFrame {
        &self.frames[self.converting]
    }

    /// The frame being converted, for writing.
    pub fn converting_frame_mut(&mut self) -> &mut ConversionFrame {
        &mut self.frames[self.converting]
    }

    /// The most recently published frame, the one the renderer shows (REQ-E3D-280), or `None`
    /// before the first publication.
    #[must_use]
    pub fn published_frame(&self) -> Option<&ConversionFrame> {
        self.published.map(|index| &self.frames[index])
    }

    /// The pinned frame, or `None` when none is pinned (REQ-E3D-233).
    #[must_use]
    pub fn pinned_frame(&self) -> Option<&ConversionFrame> {
        self.pinned.map(|index| &self.frames[index])
    }

    /// The frame of a role, or `None` when there is no such frame.
    #[must_use]
    pub fn frame(&self, role: FrameRole) -> Option<&ConversionFrame> {
        self.frame_index(role).map(|index| &self.frames[index])
    }

    /// True when the two roles name the same frame buffer.
    #[must_use]
    pub fn same_frame(&self, a: FrameRole, b: FrameRole) -> bool {
        self.frame_index(a)
            .is_some_and(|index| self.frame_index(b) == Some(index))
    }

    fn frame_index(&self, role: FrameRole) -> Option<usize> {
        match role {
            FrameRole::Converting => Some(self.converting),
            FrameRole::Published => self.published,
            FrameRole::Pinned => self.pinned,
        }
    }

    /// Empties every frame and releases the pinned one: what a game start, a reset or a state
    /// load does (REQ-E3D-281).
    pub fn clear_frames(&mut self) {
        self.pinned = None;
        self.published = None;
        for frame in &mut self.frames {
            frame.clear();
        }

        self.converting = self.acquire_frame();
    }

    /// A frame buffer that is neither displayed nor pinned, so that converting a new frame
    /// never touches either (REQ-E3D-280).
    fn acquire_frame(&mut self) -> usize {
        let free = (0..self.frames.len())
            .find(|&index| Some(index) != self.published && Some(index) != self.pinned);
        free.unwrap_or_else(|| {
            self.frames.push(ConversionFrame::new());
            self.frames.len() - 1
        })
    }

    /// Recomputes `pos` and `center` of every instance of the frame being converted
    /// (REQ-E3D-205).
    pub fn calculate_positions(&mut self) {
        self.frames[self.converting].calculate_positions(&self.settings);
    }

    /// The engine's tracker (REQ-E3D-300), the one the subjective camera uses and the one a
    /// savestate carries (DEC-R2-T5).
    #[must_use]
    pub const fn tracker(&self) -> &ShapeTracker {
        &self.tracker
    }

    /// The engine's tracker, for writing.
    pub fn tracker_mut(&mut self) -> &mut ShapeTracker {
        &mut self.tracker
    }

    /// Starts the engine's tracker on the shape `shape` of the frame of `role`
    /// (REQ-E3D-300); no effect when there is no such frame.
    pub fn start_tracker(&mut self, role: FrameRole, shape: ShapeId) -> &mut ShapeTracker {
        if let Some(index) = self.frame_index(role) {
            self.tracker.start(self.frames[index].shape(shape));
        }

        &mut self.tracker
    }

    /// Runs the engine's tracker on the frame of `role` (REQ-E3D-300).
    pub fn track(&mut self, role: FrameRole) -> Option<ShapeId> {
        let index = self.frame_index(role)?;
        self.tracker.track(&self.frames[index])
    }

    /// The scene background colour of the published frame (REQ-E3D-254).
    #[must_use]
    pub fn background_color(&self, skybox_active: bool) -> Rgb24 {
        self.published_frame()
            .map_or(self.settings.custom_background_color, |frame| {
                self.settings
                    .background_color(frame.palette(), skybox_active)
            })
    }

    // palette access for scripts (REQ-E3D-252)

    /// Reads a frame palette entry: the published frame when `current` is false, the frame
    /// being converted when it is true (REQ-E3D-252). 0 without such a frame or outside
    /// 0..63.
    #[must_use]
    pub fn frame_color_index(&self, entry: usize, current: bool) -> i32 {
        let frame = if current {
            Some(self.converting_frame())
        } else {
            self.published_frame()
        };
        frame
            .and_then(|frame| frame.palette().get(entry))
            .map_or(0, i32::from)
    }

    /// Writes a frame palette entry for this frame only (REQ-E3D-252); false when there is no
    /// such frame or the entry is outside 0..63.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // C# (byte) cast.
    pub fn set_color_index(&mut self, entry: usize, value: i32, current: bool) -> bool {
        let index = if current {
            Some(self.converting)
        } else {
            self.published
        };
        index.is_some_and(|index| self.frames[index].palette_mut().set(entry, value as u8))
    }

    /// The RGB colour of a palette entry of the published frame (REQ-E3D-252); black without
    /// a published frame or outside 0..63.
    #[must_use]
    pub fn frame_color(&self, entry: usize) -> Rgb24 {
        self.published_frame()
            .and_then(|frame| frame.palette().color(entry))
            .unwrap_or_default()
    }

    // shapes injected by scripts (REQ-E3D-230 to 232)

    /// `Frame:CreateNewShape(P, pos, pal)` with a pattern: adds a non-scriptable sprite shape
    /// whose tiles sit at `x = 8i + ⌊pos.x⌉₀` and `y = 8j + (Hs − (⌊pos.y⌉₀ + 8H) + 1)`
    /// (REQ-E3D-230). `None` when the pattern was dropped.
    pub fn create_new_shape(
        &mut self,
        pattern: PatternId,
        position: Vec3,
        palette: i32,
    ) -> Option<ShapeId> {
        self.inject_shape(pattern, position, palette, true, None)
    }

    /// The same with a pattern instance: only instance `P.Index` is enabled and it is the
    /// returned value (REQ-E3D-230, SPEC-05 REQ-LUA-086).
    ///
    /// # Errors
    ///
    /// [`EngineError::NotARegistryInstance`] when the instance belongs to no registry
    /// pattern.
    pub fn create_new_shape_from_instance(
        &mut self,
        instance: InstanceRef,
        position: Vec3,
        palette: i32,
    ) -> Result<FrameInstanceId, EngineError> {
        self.inject_instance(instance, position, palette, true)
    }

    /// `Frame:AddNewShape(P, pos, pal)`: the tiles sit at `x = 8i`, `y = 8j` and every
    /// instance offset receives `(pos.x, pos.y − Hs + 8H, pos.z)` (REQ-E3D-231). `None` when
    /// the pattern was dropped.
    pub fn add_new_shape(
        &mut self,
        pattern: PatternId,
        position: Vec3,
        palette: i32,
    ) -> Option<ShapeId> {
        self.inject_shape(pattern, position, palette, false, None)
    }

    /// The same with a pattern instance (REQ-E3D-231, SPEC-05 REQ-LUA-086).
    ///
    /// # Errors
    ///
    /// [`EngineError::NotARegistryInstance`] when the instance belongs to no registry
    /// pattern.
    pub fn add_new_shape_from_instance(
        &mut self,
        instance: InstanceRef,
        position: Vec3,
        palette: i32,
    ) -> Result<FrameInstanceId, EngineError> {
        self.inject_instance(instance, position, palette, false)
    }

    /// `Frame:InsertShape(P, pos, pal)`, which is `CreateNewShape(P, pos − BottomLeft(P),
    /// pal)` (REQ-E3D-232). `None` when the pattern was dropped.
    pub fn insert_shape(
        &mut self,
        pattern: PatternId,
        position: Vec3,
        palette: i32,
    ) -> Option<ShapeId> {
        let bottom_left = self.patterns.get(pattern)?.bottom_left();
        let shifted = Vec3::new(
            position.x - bottom_left.x,
            position.y - bottom_left.y,
            position.z,
        );
        self.create_new_shape(pattern, shifted, palette)
    }

    fn inject_instance(
        &mut self,
        instance: InstanceRef,
        position: Vec3,
        palette: i32,
        centred: bool,
    ) -> Result<FrameInstanceId, EngineError> {
        let pattern = self
            .find_pattern(instance)
            .ok_or(EngineError::NotARegistryInstance)?;
        let shape = self
            .inject_shape(pattern, position, palette, centred, Some(instance.index))
            .ok_or(EngineError::NotARegistryInstance)?;
        let count = self.converting_frame().shape(shape).instances().len();
        Ok(FrameInstanceId {
            shape,
            index: instance.index.min(count.saturating_sub(1)),
        })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // C# (int) and int-to-float.
    fn inject_shape(
        &mut self,
        id: PatternId,
        position: Vec3,
        palette: i32,
        centred: bool,
        only_instance: Option<usize>,
    ) -> Option<ShapeId> {
        let pattern = self.patterns.get(id)?;
        let frame = &mut self.frames[self.converting];
        let shape = frame.take_shape(false);
        frame.set_scriptable(shape, false);

        let truncated_x = position.x as i32;
        let truncated_y = position.y as i32;
        let flip = grid::SCENE_HEIGHT - (truncated_y + (8 * pattern.height())) + 1;

        for j in 0..pattern.height() {
            for i in 0..pattern.width() {
                let Some(&image) = pattern.image_at(i, j) else {
                    continue;
                };

                let (x, y) = if centred {
                    ((8 * i) + truncated_x, (8 * j) + flip)
                } else {
                    (8 * i, 8 * j)
                };
                let tile = frame.add_injected_tile(image, x, y, palette - 4);
                frame.add_tile(shape, tile);
            }
        }

        frame.bind(shape, id, pattern);

        let extra = if centred {
            Vec3::new(
                position.x - truncated_x as f32,
                -(position.y - truncated_y as f32),
                position.z,
            )
        } else {
            Vec3::new(
                position.x,
                position.y - grid::SCENE_HEIGHT as f32 + (8 * pattern.height()) as f32,
                position.z,
            )
        };

        for instance in frame.shape_mut(shape).instances_mut() {
            instance.offset += extra;
            if only_instance.is_some_and(|only| instance.index() != only) {
                instance.set_enable(false);
            }
        }

        frame.add_shape(shape);
        Some(shape)
    }

    /// The registry pattern `instance` belongs to, or `None` when it belongs to none. Scripts
    /// read it as `refPattern` (SPEC-05 §4.2).
    #[must_use]
    pub fn find_pattern(&self, instance: InstanceRef) -> Option<PatternId> {
        (self.patterns.is_registered(instance.pattern)
            && self.patterns.instance(instance).is_some())
        .then_some(instance.pattern)
    }

    // pinned frame (REQ-E3D-233)

    /// Freezes the last published frame until it is released (REQ-E3D-233).
    pub fn pin_new_frame(&mut self) {
        self.pinned = self.published;
    }

    /// Releases the pinned frame (REQ-E3D-233).
    pub fn release_pinned_frame(&mut self) {
        self.pinned = None;
    }

    /// Adds a rectangle, in scene coordinates, to the list of the frame being converted. No
    /// effect without a pinned frame (REQ-E3D-233).
    pub fn add_rect_from_pinned_frame(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if self.pinned.is_some() {
            self.converting_frame_mut()
                .add_pinned_rect(Rect::new(x, y, width, height));
        }
    }

    /// Adds, as non-scriptable shapes of the frame being converted, every shape of the pinned
    /// frame whose box overlaps one of the frame's rectangles (REQ-E3D-233). Returns how many
    /// were restored.
    pub fn restore_from_pinned_frame(&mut self) -> usize {
        let Some(pinned) = self.pinned else {
            return 0;
        };

        let (source, target) = split_frames(&mut self.frames, pinned, self.converting);
        let mut restored = 0;
        for &shape in source.shapes() {
            let shape_box = shape_box(source, shape);
            if target
                .pinned_rects()
                .iter()
                .any(|rectangle| shape_box.overlaps(rectangle))
                && target.add_foreign_shape(source, shape)
            {
                restored += 1;
            }
        }

        restored
    }

    /// Adds the rectangle, then restores the shapes that overlap it (REQ-E3D-233).
    pub fn restore_from_pinned_frame_rect(&mut self, rectangle: Rect) -> usize {
        if self.pinned.is_none() {
            return 0;
        }

        self.converting_frame_mut().add_pinned_rect(rectangle);
        self.restore_from_pinned_frame()
    }

    /// The shapes of the pinned frame that overlap `rectangle` (SPEC-05
    /// `Frame:GetShapesFromPinnedFrame`).
    pub fn collect_shapes_from_pinned_frame(
        &self,
        rectangle: Rect,
        destination: &mut Vec<ShapeId>,
    ) {
        destination.clear();
        if let Some(pinned) = self.pinned_frame() {
            destination.extend(
                pinned
                    .shapes()
                    .iter()
                    .copied()
                    .filter(|&shape| shape_box(pinned, shape).overlaps(&rectangle)),
            );
        }
    }

    // queries for scripts (SPEC-05 REQ-LUA-084, REQ-LUA-085)

    /// The scriptable instance of index `index` (REQ-E3D-220), or `None` past the end.
    #[must_use]
    pub fn get_shape(&self, index: usize) -> Option<FrameInstanceId> {
        self.converting_frame()
            .scriptable_instances()
            .get(index)
            .copied()
    }

    /// Number of scriptable instances of the frame being converted.
    #[must_use]
    pub fn scriptable_count(&self) -> usize {
        self.converting_frame().scriptable_instances().len()
    }

    /// The scriptable instances carrying `tag`, in the order of REQ-E3D-220 (SPEC-05
    /// REQ-LUA-084).
    pub fn collect_shapes_with_tag(&self, tag: &str, destination: &mut Vec<FrameInstanceId>) {
        let frame = self.converting_frame();
        destination.clear();
        destination.extend(
            frame
                .scriptable_instances()
                .iter()
                .copied()
                .filter(|&id| frame.instance(id).contains_tag(tag)),
        );
    }

    /// The first scriptable instance carrying `tag`, or `None`.
    #[must_use]
    pub fn get_shape_with_tag(&self, tag: &str) -> Option<FrameInstanceId> {
        let frame = self.converting_frame();
        frame
            .scriptable_instances()
            .iter()
            .copied()
            .find(|&id| frame.instance(id).contains_tag(tag))
    }

    /// The first scriptable instance whose box contains the scene point (`x`, `y`) and that
    /// carries `tag` (SPEC-05 REQ-LUA-084 point 5).
    #[must_use]
    pub fn get_shape_with_pos(&self, x: f32, y: f32, tag: &str) -> Option<FrameInstanceId> {
        let frame = self.converting_frame();
        frame.scriptable_instances().iter().copied().find(|&id| {
            let instance = frame.instance(id);
            let bottom_left = instance.bottom_left();
            let top_right = instance.top_right();
            bottom_left.x <= x
                && x <= top_right.x
                && bottom_left.y <= y
                && y <= top_right.y
                && instance.contains_tag(tag)
        })
    }

    /// The first pattern instance carrying `tag`, or `None`.
    #[must_use]
    pub fn get_pattern_with_tag(&self, tag: &str) -> Option<InstanceRef> {
        self.patterns.find_instance_with_tag(tag)
    }

    /// Every pattern instance carrying `tag`, in registry order.
    pub fn get_patterns_with_tag(&self, tag: &str, destination: &mut Vec<InstanceRef>) {
        self.patterns.collect_instances_with_tag(tag, destination);
    }
}

/// The box `(pStart, pEnd − pStart + 1)` a restore test uses. It is in screen pixels while the
/// rectangles are in scene coordinates; the mismatch is the original's and is kept
/// (DEC-04-Q05, Q-05).
#[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
fn shape_box(frame: &ConversionFrame, shape: ShapeId) -> Rect {
    let data = frame.shape(shape);
    let (start, end) = (data.pixel_start(), data.pixel_end());
    Rect::new(
        start.x as f32,
        start.y as f32,
        (end.x - start.x + 1) as f32,
        (end.y - start.y + 1) as f32,
    )
}

/// The frame `read` to read and the frame `write` to write, two different buffers.
fn split_frames(
    frames: &mut [ConversionFrame],
    read: usize,
    write: usize,
) -> (&ConversionFrame, &mut ConversionFrame) {
    if read < write {
        let (low, high) = frames.split_at_mut(write);
        (&low[read], &mut high[0])
    } else {
        let (low, high) = frames.split_at_mut(read);
        (&high[0], &mut low[write])
    }
}
