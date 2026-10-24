//! Test support: a GL context with an offscreen target, captures and their pixel statistics,
//! and the synthetic frames the render tests assert on (ports of `GlFixture.cs` and
//! `SceneFixtures.cs`).
//!
//! Every GPU test creates its own context on its own thread, as the offscreen context allows;
//! a machine with no EGL driver fails them with the reason instead of passing them silently.

#![allow(dead_code)] // Each test binary uses its own part of the support module.
#![allow(clippy::expect_used, clippy::panic)] // A broken fixture fails the test loudly.

use std::collections::HashSet;
use std::sync::Arc;

use zax_engine3d::draw_list::{DrawTile, FrameDrawList};
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::frame::{FrameInstance, FrameInstanceId};
use zax_engine3d::palette::nes_color;
use zax_engine3d::settings::ConversionSettings;
use zax_nes::{FrameBatch, TileRecord};
use zax_profiles::enums::{ProfileLayer, TileFlip, VariantRenderStyle};
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;
use zax_render::assets::SceneAssets;
use zax_render::offscreen::{ContextApi, OffscreenContext};
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::SceneRenderer;

/// Asserts that `actual` is within `tolerance` of `expected`.
#[track_caller]
pub fn assert_close(expected: f64, actual: f64, tolerance: f64) {
    assert!(
        (expected - actual).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

/// A GL context with an offscreen colour-and-depth target, the same shape of target the
/// harness hands the renderer (SPEC-06 REQ-SCN-057, SPEC-01 REQ-ARCH-031).
pub struct GlFixture {
    context: OffscreenContext,
}

impl GlFixture {
    /// A desktop GL 3.3 core context of 640 × 480.
    pub fn new() -> Self {
        Self::create(640, 480)
    }

    /// A desktop GL 3.3 core context of the given size.
    pub fn create(width: i32, height: i32) -> Self {
        Self::with_api(width, height, ContextApi::DesktopCore)
    }

    /// A context of the given API and size.
    pub fn with_api(width: i32, height: i32, api: ContextApi) -> Self {
        let context = OffscreenContext::new(width, height, api)
            .unwrap_or_else(|error| panic!("no OpenGL context on this machine: {error}"));
        Self { context }
    }

    pub fn width(&self) -> i32 {
        self.context.width()
    }

    pub fn height(&self) -> i32 {
        self.context.height()
    }

    /// A renderer with the procedural sky.
    pub fn renderer(&self) -> SceneRenderer {
        SceneRenderer::new(self.context.gl(), None).expect("renderer")
    }

    /// A renderer that keeps the binaries of its programs in `directory`.
    pub fn renderer_cached(&self, directory: &std::path::Path) -> SceneRenderer {
        SceneRenderer::with_program_cache(self.context.gl(), None, Some(directory.to_path_buf()))
            .expect("renderer")
    }

    /// A renderer whose skybox cubemaps come from `assets`.
    pub fn renderer_with(&self, assets: Box<dyn SceneAssets>) -> SceneRenderer {
        SceneRenderer::new(self.context.gl(), Some(assets)).expect("renderer")
    }

    /// Renders and reads the capture back, top row first.
    pub fn render(
        &self,
        renderer: &mut SceneRenderer,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
    ) -> Capture {
        renderer
            .render(&self.context.target(), scene, frame)
            .expect("render");
        Capture {
            pixels: self.context.read_pixels(),
            width: self.width(),
            height: self.height(),
        }
    }

    /// Renders a scene with no frame data.
    pub fn render_scene(&self, renderer: &mut SceneRenderer, scene: &SceneState) -> Capture {
        self.render(renderer, scene, &SceneFrame::default())
    }

    /// Renders a built synthetic frame.
    pub fn render_built(&self, renderer: &mut SceneRenderer, built: &Built) -> Capture {
        self.render(renderer, &built.scene, &built.frame())
    }
}

/// One capture, as rows of RGBA bytes with the top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

impl Capture {
    /// The RGB of the pixel at (`x`, `y`).
    pub fn at(&self, x: i32, y: i32) -> (i32, i32, i32) {
        let at = usize::try_from(((y * self.width) + x) * 4).expect("pixel inside");
        (
            i32::from(self.pixels[at]),
            i32::from(self.pixels[at + 1]),
            i32::from(self.pixels[at + 2]),
        )
    }

    /// The packed RGB of the pixel.
    pub fn packed(&self, x: i32, y: i32) -> i32 {
        let (red, green, blue) = self.at(x, y);
        (red << 16) | (green << 8) | blue
    }

    /// Relative luma of the pixel, 0 to 1.
    pub fn luma(&self, x: i32, y: i32) -> f64 {
        let (red, green, blue) = self.at(x, y);
        ((0.299 * f64::from(red)) + (0.587 * f64::from(green)) + (0.114 * f64::from(blue))) / 255.0
    }

    fn all(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| (x, y)))
    }

    /// How many pixels have the packed colour `color`.
    pub fn count(&self, color: i32) -> usize {
        self.all()
            .filter(|&(x, y)| self.packed(x, y) == color)
            .count()
    }

    /// How many pixels differ from `color`.
    pub fn count_other(&self, color: i32) -> usize {
        self.all()
            .filter(|&(x, y)| self.packed(x, y) != color)
            .count()
    }

    /// The distinct colours of the capture.
    pub fn distinct_colors(&self) -> HashSet<i32> {
        self.all().map(|(x, y)| self.packed(x, y)).collect()
    }

    /// The rows that hold at least one pixel other than `background`, as (first, last);
    /// (−1, −1) when there is none.
    pub fn row_span(&self, background: i32) -> (i32, i32) {
        let rows: Vec<i32> = (0..self.height)
            .filter(|&y| (0..self.width).any(|x| self.packed(x, y) != background))
            .collect();
        (
            rows.first().copied().unwrap_or(-1),
            rows.last().copied().unwrap_or(-1),
        )
    }

    /// The same for columns.
    pub fn column_span(&self, background: i32) -> (i32, i32) {
        let columns: Vec<i32> = (0..self.width)
            .filter(|&x| (0..self.height).any(|y| self.packed(x, y) != background))
            .collect();
        (
            columns.first().copied().unwrap_or(-1),
            columns.last().copied().unwrap_or(-1),
        )
    }
}

/// A tile image as the two bit planes of a profile tile.
pub type Tile = (u64, u64);

/// A full tile of the colour `color` with a unique identity.
pub fn solid(color: i32, id: i32) -> Tile {
    let mut low = if color & 1 != 0 { u64::MAX } else { 0 };
    let mut high = if color & 2 != 0 { u64::MAX } else { 0 };
    if color & 2 == 0 {
        high = 1 << (id & 63);
    } else if color & 1 == 0 {
        low = 1 << (id & 63);
    }

    (low, high)
}

/// The packed RGB of a NES colour index.
pub fn nes_packed(index: usize) -> i32 {
    nes_color(index).packed()
}

/// The options of one pattern of the fixture.
#[derive(Debug, Clone, Copy)]
pub struct PatternOptions {
    pub alpha: f32,
    pub style: VariantRenderStyle,
    pub full_warp: bool,
}

impl Default for PatternOptions {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            style: VariantRenderStyle::Normal,
            full_warp: false,
        }
    }
}

/// Builds the synthetic frames the render tests assert on: a profile, a batch of tile records
/// and the resulting scene state, all without a ROM.
#[derive(Default)]
pub struct SceneFixture {
    profile: Option<GameProfile>,
    records: Vec<TileRecord>,
    palette: [u8; 32],
    sprite_order: i32,
    /// Whether the frame is converted in "2D Mode": the empty profile, where every tile image
    /// is learned as it appears (SPEC-06 REQ-SCN-055, SPEC-04 REQ-E3D-381).
    pub two_dimensional_mode: bool,
}

impl SceneFixture {
    pub fn new() -> Self {
        Self {
            profile: Some(GameProfile::create_empty()),
            ..Self::default()
        }
    }

    fn profile(&mut self) -> &mut GameProfile {
        self.profile.get_or_insert_with(GameProfile::create_empty)
    }

    /// Sets the universal background colour, entry 0 of the palette.
    pub fn background(mut self, nes_color: u8) -> Self {
        self.palette[0] = nes_color;
        self
    }

    /// Sets one of the 32 PPU palette bytes.
    pub fn palette(mut self, entry: usize, nes_color: u8) -> Self {
        if (1..32).contains(&entry) {
            self.palette[entry] = nes_color;
        }
        self
    }

    /// Adds a one-cell pattern carrying `tile`.
    pub fn pattern(self, tile: Tile, depth: i32, layer: ProfileLayer) -> Self {
        self.pattern_with(tile, depth, layer, PatternOptions::default())
    }

    /// Adds a one-cell pattern carrying `tile`, with options.
    pub fn pattern_with(
        self,
        tile: Tile,
        depth: i32,
        layer: ProfileLayer,
        options: PatternOptions,
    ) -> Self {
        self.add_pattern(1, 1, &[tile], depth, layer, options)
    }

    /// Adds a one-row pattern carrying `cells` left to right.
    pub fn pattern_row(self, depth: i32, layer: ProfileLayer, alpha: f32, cells: &[Tile]) -> Self {
        let width = i32::try_from(cells.len()).expect("small row");
        let options = PatternOptions {
            alpha,
            ..PatternOptions::default()
        };
        self.add_pattern(width, 1, cells, depth, layer, options)
    }

    /// Adds a `columns × rows` pattern whose cells all carry `tile`: one shape instance of
    /// several tiles, the "object" of AMD-012.
    pub fn pattern_block(
        self,
        tile: Tile,
        columns: i32,
        rows: i32,
        depth: i32,
        layer: ProfileLayer,
        alpha: f32,
    ) -> Self {
        let cells = vec![tile; usize::try_from(columns * rows).expect("small block")];
        let options = PatternOptions {
            alpha,
            ..PatternOptions::default()
        };
        self.add_pattern(columns, rows, &cells, depth, layer, options)
    }

    fn add_pattern(
        mut self,
        columns: i32,
        rows: i32,
        cells: &[Tile],
        depth: i32,
        layer: ProfileLayer,
        options: PatternOptions,
    ) -> Self {
        let tiles = cells
            .iter()
            .map(|&(low, high)| Some(Arc::new(ProfileTile::new(low, high, TileFlip::NONE))))
            .collect();
        let mut pattern = ProfilePattern {
            grid: Some(PatternGrid::new(columns, rows, tiles)),
            tile_size: Some(Vec2I::new(columns, rows)),
            permanent: true,
            self_connected: true,
            constraint_size: Vec2I::UNCONSTRAINED,
            ..ProfilePattern::default()
        };
        pattern.variants.push(Arc::new(PatternVariant {
            depth,
            layer,
            alpha: options.alpha,
            render_style: options.style,
            full_warp: options.full_warp,
            ..PatternVariant::default()
        }));
        self.profile().permanent_patterns.push(Arc::new(pattern));
        self
    }

    /// Adds a background tile record at the screen position (`x`, `y`).
    pub fn tile(mut self, tile: Tile, x: i32, y: i32, palette: u8) -> Self {
        self.push_tile(tile, x, y, palette);
        self
    }

    fn push_tile(&mut self, tile: Tile, x: i32, y: i32, palette: u8) {
        let index = i32::try_from(self.records.len()).expect("few records");
        self.records.push(TileRecord::new(
            tile.0,
            tile.1,
            i16::try_from(x).expect("screen x"),
            i16::try_from(y).expect("screen y"),
            index,
            false,
            false,
            false,
            palette,
            true,
        ));
    }

    /// Adds a rectangle of background tile records, in tile cells.
    pub fn rect(mut self, tile: Tile, column: i32, row: i32, columns: i32, rows: i32) -> Self {
        self.push_rect(tile, column, row, columns, rows, 0);
        self
    }

    /// Adds a rectangle of background tile records with a palette.
    pub fn rect_palette(
        mut self,
        tile: Tile,
        column: i32,
        row: i32,
        columns: i32,
        rows: i32,
        palette: u8,
    ) -> Self {
        self.push_rect(tile, column, row, columns, rows, palette);
        self
    }

    fn push_rect(
        &mut self,
        tile: Tile,
        column: i32,
        row: i32,
        columns: i32,
        rows: i32,
        palette: u8,
    ) {
        for y in 0..rows {
            for x in 0..columns {
                self.push_tile(tile, (column + x) * 8, (row + y) * 8, palette);
            }
        }
    }

    /// Adds a sprite tile record.
    pub fn sprite(mut self, tile: Tile, x: i32, y: i32, palette: u8, behind: bool) -> Self {
        self.push_sprite(tile, x, y, palette, behind);
        self
    }

    pub fn push_sprite(&mut self, tile: Tile, x: i32, y: i32, palette: u8, behind: bool) {
        self.records.push(TileRecord::new(
            tile.0,
            tile.1,
            i16::try_from(x).expect("screen x"),
            i16::try_from(y).expect("screen y"),
            self.sprite_order,
            false,
            false,
            behind,
            palette,
            false,
        ));
        self.sprite_order += 1;
    }

    /// Converts the frame and returns a scene state ready to render.
    pub fn build(self) -> Built {
        self.build_with(|_| {})
    }

    /// Converts the frame, configuring the settings after the profile load (loading a profile
    /// resets the layer shadow permissions, REQ-SCN-033).
    pub fn build_with(mut self, configure: impl FnOnce(&mut ConversionSettings)) -> Built {
        let profile = self.profile().clone();
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine.state_mut().two_dimensional_mode = self.two_dimensional_mode;
        engine.load_profile(&profile, true).expect("profile load");
        configure(engine.settings_mut());

        let batch = FrameBatch {
            records: &self.records,
            palette: self.palette,
            mask_x: 0,
            mask_y: 0,
            margin: 0,
            visible_rows: zax_nes::STANDARD_ROWS,
        };
        engine.convert(&batch, 0);
        let time = engine.publish().time();
        let background = engine.frame_color(0);

        let mut scene = SceneState::default();
        scene.camera.snap_to_setpoints();
        scene.game_running = true;
        scene.clipping = engine.settings().clipping;
        scene.nes_background_color = background;
        scene.emulated_time = time;
        let draw_list = engine.build_draw_list().clone();
        Built {
            engine,
            scene,
            draw_list,
        }
    }
}

/// A converted synthetic frame: the engine that produced it, the scene state and the draw
/// list of the displayed frame.
pub struct Built {
    pub engine: ConversionEngine,
    pub scene: SceneState,
    pub draw_list: FrameDrawList,
}

impl Built {
    /// The frame data to render.
    pub fn frame(&self) -> SceneFrame<'_> {
        SceneFrame::with_draw_list(&self.draw_list)
    }

    /// Builds the draw list again after a change to the engine.
    pub fn rebuild_draw_list(&mut self) {
        self.draw_list = self.engine.build_draw_list().clone();
    }

    /// The distinct shape instances of the draw list whose tiles pass `filter`, in list order.
    pub fn instances(&self, filter: impl Fn(&DrawTile) -> bool) -> Vec<FrameInstanceId> {
        let mut instances = Vec::new();
        for tile in self.draw_list.tiles().iter().filter(|tile| filter(tile)) {
            if !instances.contains(&tile.instance) {
                instances.push(tile.instance);
            }
        }
        instances
    }

    /// An instance of the displayed frame.
    pub fn instance(&self, id: FrameInstanceId) -> &FrameInstance {
        self.engine.state().converting_frame().instance(id)
    }

    /// An instance of the displayed frame, for writing. The frame is still the converting one:
    /// nothing was converted after the publication.
    pub fn instance_mut(&mut self, id: FrameInstanceId) -> &mut FrameInstance {
        self.engine
            .state_mut()
            .converting_frame_mut()
            .instance_mut(id)
    }

    /// An instance of the displayed frame and the settings it is positioned with.
    pub fn instance_and_settings_mut(
        &mut self,
        id: FrameInstanceId,
    ) -> (&mut FrameInstance, &ConversionSettings) {
        let (frame, settings) = self.engine.state_mut().converting_frame_and_settings_mut();
        (frame.instance_mut(id), settings)
    }
}
