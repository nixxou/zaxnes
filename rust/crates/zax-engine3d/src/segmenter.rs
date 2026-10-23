// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The segmentation of a frame: the three background passes (REQ-E3D-092), the two sprite
//! passes (REQ-E3D-131) and the constrained growth they share (REQ-E3D-090).
//!
//! Every buffer is reused between frames; the segmentation of a full frame allocates nothing
//! once warm (REQ-E3D-360).
//!
//! The greedy shrinking of the candidate set depends on the order the tiles are explored. That
//! is deliberate and required for the built-in profiles to be recognised as they are
//! (REQ-E3D-090 notes).

use std::collections::VecDeque;

use crate::engine::build_geometry;
use crate::frame::{ConversionFrame, FrameTile, ShapeId, TileId};
use crate::geometry::builder::PatternGeometryBuilder;
use crate::grid;
use crate::image::{ImageRegistry, ImageShape2D};
use crate::instance::PatternInstance;
use crate::pattern::{EnginePattern, GrowDirection, ImageSetKey};
use crate::registry::{PatternId, PatternRegistry};
use crate::script::ScriptHost;
use crate::settings::ConversionSettings;

/// What the segmentation reads and changes besides the frame: the registries, the settings,
/// and the hook and geometry sides a learned pattern goes through (REQ-E3D-161).
pub(crate) struct SegmentContext<'a> {
    pub patterns: &'a mut PatternRegistry,
    pub images: &'a mut ImageRegistry,
    pub settings: &'a ConversionSettings,
    pub host: &'a mut dyn ScriptHost,
    pub geometry: &'a mut dyn PatternGeometryBuilder,
}

impl SegmentContext<'_> {
    /// Learns a pattern from a shape (REQ-E3D-161): a grid the size of its tile box, the
    /// recognition defaults, one instance with the 3D defaults of SPEC-04 §4.3, registration,
    /// the `BuildPattern` hook and then its geometry.
    fn learn(&mut self, frame: &ConversionFrame, shape: ShapeId, permanent: bool) -> PatternId {
        let data = frame.shape(shape);
        let size = data.tile_size();
        let start = data.tile_start();
        let width = size.x.max(1);
        let height = size.y.max(1);
        let cells = (0..height).flat_map(|j| {
            (0..width).map(move |i| {
                frame
                    .shape_tile_at(shape, grid::index(start.x + i, start.y + j))
                    .map(|tile| *tile.image())
            })
        });

        // A tile box is at least 1 x 1 and at most the grid, 35 x 33 cells.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut pattern = EnginePattern::new(width as u16, height as u16, cells, permanent);
        pattern.add_instance(PatternInstance::new());
        let id = self.patterns.add(pattern, self.images);
        build_geometry(
            self.patterns,
            id,
            self.host,
            self.geometry,
            self.settings.render_algorithm,
        );
        id
    }
}

/// The segmenter and its reused buffers.
#[derive(Debug, Default)]
pub(crate) struct Segmenter {
    germs: Vec<TileId>,
    queue: VecDeque<TileId>,
    rejected: Vec<ShapeId>,
    candidates: Vec<PatternId>,
    next_candidates: Vec<PatternId>,
    key: Vec<u32>,
}

impl Segmenter {
    /// Segments the background then the sprites of `frame` (REQ-E3D-001 c).
    pub fn run(
        &mut self,
        frame: &mut ConversionFrame,
        context: &mut SegmentContext<'_>,
        two_dimensional_mode: bool,
    ) {
        self.segment_background(frame, context, two_dimensional_mode);
        frame.set_background_shape_count();
        self.segment_sprites(frame, context, two_dimensional_mode);
    }

    fn segment_background(
        &mut self,
        frame: &mut ConversionFrame,
        context: &mut SegmentContext<'_>,
        two_dimensional_mode: bool,
    ) {
        self.build_germination_order(frame, context.settings);
        let germs = std::mem::take(&mut self.germs);

        // Pass 1: large known regions. A shape is kept when it is full with at least 8 tiles
        // and 2 distinct images, or when it is big; otherwise it is rejected and its tiles
        // stay unavailable until the end of the pass (REQ-E3D-092).
        self.rejected.clear();
        for &germ in &germs {
            if !self.is_known_free_germ(frame, context, germ) {
                continue;
            }

            let shape = self.grow_background(frame, context, germ);
            let data = frame.shape(shape);
            let keep = (frame.is_full(shape) && data.tile_count() >= 8 && data.images().len() >= 2)
                || data.is_big();
            if keep {
                self.bind_to_chosen_candidate(frame, context, shape);
                frame.add_shape(shape);
            } else {
                self.rejected.push(shape);
            }
        }

        for shape in self.rejected.drain(..) {
            frame.return_shape(shape);
        }

        // Pass 2: tiles whose image is unknown at the moment they are visited (REQ-E3D-092).
        for &germ in &germs {
            let tile = frame.tile(germ);
            if !tile.is_free() || context.images.is_known(tile.image()) {
                continue;
            }

            let mut shape = frame.take_shape(true);
            frame.set_scriptable(shape, true);
            frame.add_tile(shape, germ);

            if two_dimensional_mode {
                self.grow_unknown_background_in_2d_mode(frame, context, shape, germ);
                shape = self.extract_band(frame, shape);
                if frame.shape(shape).tile_count() == 0 {
                    frame.return_shape(shape);
                    continue;
                }
            }

            self.bind_unknown_shape(frame, context, shape);
            frame.add_shape(shape);
        }

        // Pass 3: the rest of the known tiles; always kept (REQ-E3D-092, REQ-E3D-095).
        for &germ in &germs {
            if !self.is_known_free_germ(frame, context, germ) {
                continue;
            }

            let shape = self.grow_background(frame, context, germ);
            self.bind_to_chosen_candidate(frame, context, shape);
            frame.add_shape(shape);
        }

        self.germs = germs;
    }

    /// True when the germ is free, its image known and it has at least one candidate, which
    /// [`Segmenter::initialise_candidates`] has then set up (REQ-E3D-091, REQ-E3D-093).
    fn is_known_free_germ(
        &mut self,
        frame: &ConversionFrame,
        context: &SegmentContext<'_>,
        germ: TileId,
    ) -> bool {
        let tile = frame.tile(germ);
        tile.is_free()
            && context.images.is_known(tile.image())
            && self.initialise_candidates(tile, context)
    }

    fn segment_sprites(
        &mut self,
        frame: &mut ConversionFrame,
        context: &mut SegmentContext<'_>,
        two_dimensional_mode: bool,
    ) {
        let sprite_count = frame.sprites().len();

        // Pass S1: groups of sprites whose image is unknown when they are visited; free growth
        // with an exact adjacency and the same palette (REQ-E3D-131).
        for germ in 0..sprite_count {
            let tile = &frame.sprites()[germ];
            if !tile.is_free() || context.images.is_known(tile.image()) {
                continue;
            }

            let shape = frame.take_shape(false);
            frame.set_scriptable(shape, true);
            frame.add_tile(shape, TileId::Sprite(germ));

            self.queue.clear();
            self.queue.push_back(TileId::Sprite(germ));
            while let Some(current) = self.queue.pop_front() {
                let tile = *frame.tile(current);
                for neighbour in 0..sprite_count {
                    let candidate = &frame.sprites()[neighbour];
                    if !candidate.is_free()
                        || candidate.palette() != tile.palette()
                        || adjacency(&tile, candidate, 0).is_none()
                        || frame.sprite_occupant(candidate.grid_index()).is_some()
                    {
                        continue;
                    }

                    // 2D mode keeps one tile per image in the group (REQ-E3D-381).
                    if two_dimensional_mode
                        && (context.images.is_known(candidate.image())
                            || frame.shape(shape).images().contains(candidate.image()))
                    {
                        continue;
                    }

                    frame.add_tile(shape, TileId::Sprite(neighbour));
                    self.queue.push_back(TileId::Sprite(neighbour));
                }
            }

            self.bind_unknown_shape(frame, context, shape);
            frame.add_shape(shape);
        }

        // Pass S2: every sprite still free, constrained growth over all free sprites
        // (REQ-E3D-131).
        for germ in 0..sprite_count {
            let tile = &frame.sprites()[germ];
            if !tile.is_free() || !self.initialise_candidates(tile, context) {
                continue;
            }

            let shape = frame.take_shape(false);
            frame.set_scriptable(shape, true);
            frame.add_tile(shape, TileId::Sprite(germ));

            self.queue.clear();
            self.queue.push_back(TileId::Sprite(germ));
            while let Some(current) = self.queue.pop_front() {
                let tile = *frame.tile(current);
                for neighbour in 0..sprite_count {
                    let candidate = &frame.sprites()[neighbour];
                    if !candidate.is_free() {
                        continue;
                    }

                    let Some(direction) =
                        adjacency(&tile, candidate, context.settings.sprite_tolerance)
                    else {
                        continue;
                    };

                    if context.settings.same_sprite_palette && candidate.palette() != tile.palette()
                    {
                        continue;
                    }

                    let candidate = *candidate;
                    if !self.shrink_candidates(frame, context, shape, &tile, &candidate, direction)
                    {
                        continue;
                    }

                    frame.add_tile(shape, TileId::Sprite(neighbour));
                    self.queue.push_back(TileId::Sprite(neighbour));
                }
            }

            self.bind_to_chosen_candidate(frame, context, shape);
            frame.add_shape(shape);
        }
    }

    /// The germination order of the background tiles (REQ-E3D-006): by y then x when
    /// `scan_direction` is 0, by x then y when it is 1.
    fn build_germination_order(&mut self, frame: &ConversionFrame, settings: &ConversionSettings) {
        self.germs.clear();
        self.germs.extend(
            (0..grid::CELL_COUNT)
                .map(TileId::Background)
                .filter(|&id| frame.tile(id).present()),
        );

        // Two tiles of one batch that land in the same cell share one slot (Q-012), so the
        // sort keys are unique and the order is total.
        let by_column = settings.scan_direction == 1;
        self.germs.sort_unstable_by_key(|&id| {
            let tile = frame.tile(id);
            if by_column {
                (tile.x(), tile.y())
            } else {
                (tile.y(), tile.x())
            }
        });
    }

    /// Constrained growth over the background grid: the 4-neighbourhood in the order right,
    /// down, up, left, a FIFO queue, and the greedy shrinking of the candidate set
    /// (REQ-E3D-090).
    fn grow_background(
        &mut self,
        frame: &mut ConversionFrame,
        context: &SegmentContext<'_>,
        germ: TileId,
    ) -> ShapeId {
        let shape = frame.take_shape(true);
        frame.set_scriptable(shape, true);
        frame.add_tile(shape, germ);

        self.queue.clear();
        self.queue.push_back(germ);
        while let Some(current) = self.queue.pop_front() {
            let tile = *frame.tile(current);
            for (offset, direction) in grid::NEIGHBOUR_OFFSETS.iter().zip(GrowDirection::ORDER) {
                let Some(neighbour_id) = frame.background_id(tile.grid_index() + offset) else {
                    continue;
                };

                let neighbour = *frame.tile(neighbour_id);
                if !neighbour.is_free() {
                    continue;
                }

                if context.settings.same_background_palette && neighbour.palette() != tile.palette()
                {
                    continue;
                }

                if !self.shrink_candidates(frame, context, shape, &tile, &neighbour, direction) {
                    continue;
                }

                frame.add_tile(shape, neighbour_id);
                self.queue.push_back(neighbour_id);
            }
        }

        shape
    }

    /// Growth of an unknown background tile in 2D mode (REQ-E3D-381): 4-neighbourhood, same
    /// palette, free, unknown, not CHAR, and an image the shape does not already carry.
    fn grow_unknown_background_in_2d_mode(
        &mut self,
        frame: &mut ConversionFrame,
        context: &SegmentContext<'_>,
        shape: ShapeId,
        germ: TileId,
    ) {
        if frame.tile(germ).image().shape_2d() == ImageShape2D::Char {
            return;
        }

        self.queue.clear();
        self.queue.push_back(germ);
        while let Some(current) = self.queue.pop_front() {
            let tile = *frame.tile(current);
            for offset in grid::NEIGHBOUR_OFFSETS {
                let Some(neighbour_id) = frame.background_id(tile.grid_index() + offset) else {
                    continue;
                };

                let neighbour = frame.tile(neighbour_id);
                if !neighbour.is_free()
                    || context.images.is_known(neighbour.image())
                    || neighbour.image().shape_2d() == ImageShape2D::Char
                    || neighbour.palette() != tile.palette()
                    || frame.shape(shape).images().contains(neighbour.image())
                {
                    continue;
                }

                frame.add_tile(shape, neighbour_id);
                self.queue.push_back(neighbour_id);
            }
        }
    }

    /// The band extraction of 2D mode (REQ-E3D-381): when the shape is big and does not fill
    /// its tile box, keep the first run of entirely full rows (or columns), release the other
    /// tiles and extend the band to free tiles whose image it already carries. Returns the
    /// shape to bind, which may be a new one.
    fn extract_band(&mut self, frame: &mut ConversionFrame, shape: ShapeId) -> ShapeId {
        let data = frame.shape(shape);
        let size = data.tile_size();
        let (start, end) = (data.tile_start(), data.tile_end());
        let fills_box =
            usize::try_from(size.x * size.y).is_ok_and(|area| area == data.tile_count());
        if !data.is_big() || fills_box {
            return shape;
        }

        let band = frame.take_shape(true);
        frame.set_scriptable(band, true);

        // Rows when the box is at least as wide as high, columns otherwise.
        let by_rows = size.x >= size.y;
        let (lines, cells) = if by_rows {
            (start.y..=end.y, start.x..=end.x)
        } else {
            (start.x..=end.x, start.y..=end.y)
        };
        let index = |line: i32, cell: i32| {
            if by_rows {
                grid::index(cell, line)
            } else {
                grid::index(line, cell)
            }
        };

        let mut begun = false;
        for line in lines {
            let full = cells
                .clone()
                .all(|cell| frame.shape_tile_id_at(shape, index(line, cell)).is_some());
            if full {
                for cell in cells.clone() {
                    if let Some(id) = frame.shape_tile_id_at(shape, index(line, cell)) {
                        frame.add_tile(band, id);
                    }
                }

                begun = true;
            } else if begun {
                break;
            }
        }

        if frame.shape(band).tile_count() == 0 {
            frame.return_shape(band);
            return shape;
        }

        frame.return_shape(shape);
        self.expand_band(frame, band);
        band
    }

    fn expand_band(&mut self, frame: &mut ConversionFrame, band: ShapeId) {
        self.queue.clear();
        self.queue.extend(frame.shape(band).tiles());
        while let Some(current) = self.queue.pop_front() {
            let tile = *frame.tile(current);
            for offset in grid::NEIGHBOUR_OFFSETS {
                let Some(neighbour_id) = frame.background_id(tile.grid_index() + offset) else {
                    continue;
                };

                let neighbour = frame.tile(neighbour_id);
                if !neighbour.is_free() || !frame.shape(band).images().contains(neighbour.image()) {
                    continue;
                }

                frame.add_tile(band, neighbour_id);
                self.queue.push_back(neighbour_id);
            }
        }
    }

    /// The initial candidate list of a germ: the non-disabled permanent patterns that contain
    /// its image, in registration order (REQ-E3D-091). False when there is none, in which
    /// case the germ stays free and is not displayed (REQ-E3D-093).
    fn initialise_candidates(&mut self, germ: &FrameTile, context: &SegmentContext<'_>) -> bool {
        self.candidates.clear();
        self.candidates.extend(
            context
                .images
                .permanent_patterns(germ.image())
                .iter()
                .copied()
                .filter(|&id| {
                    context
                        .patterns
                        .get(id)
                        .is_some_and(EnginePattern::is_candidate)
                }),
        );
        !self.candidates.is_empty()
    }

    /// Keeps the candidates that allow adding `neighbour` from `tile` (REQ-E3D-062). The set
    /// is only replaced when it stays non-empty (REQ-E3D-090 point 4).
    fn shrink_candidates(
        &mut self,
        frame: &ConversionFrame,
        context: &SegmentContext<'_>,
        shape: ShapeId,
        tile: &FrameTile,
        neighbour: &FrameTile,
        direction: GrowDirection,
    ) -> bool {
        let size = frame.shape(shape).size_with(neighbour.position());
        self.next_candidates.clear();
        self.next_candidates
            .extend(self.candidates.iter().copied().filter(|&id| {
                context.patterns.get(id).is_some_and(|pattern| {
                    pattern.allows_direction(tile.image(), neighbour.image(), direction)
                        && pattern.fits_constraint(size.x, size.y)
                })
            }));

        if self.next_candidates.is_empty() {
            return false;
        }

        std::mem::swap(&mut self.candidates, &mut self.next_candidates);
        true
    }

    /// Binds the shape to the candidate with the fewest distinct images, the first in
    /// candidate order on a tie (REQ-E3D-090, REQ-E3D-095).
    fn bind_to_chosen_candidate(
        &self,
        frame: &mut ConversionFrame,
        context: &SegmentContext<'_>,
        shape: ShapeId,
    ) {
        let mut chosen: Option<(PatternId, &EnginePattern)> = None;
        for &id in &self.candidates {
            let Some(pattern) = context.patterns.get(id) else {
                continue;
            };

            if chosen.is_none_or(|(_, best)| pattern.images().len() < best.images().len()) {
                chosen = Some((id, pattern));
            }
        }

        if let Some((id, pattern)) = chosen {
            frame.bind(shape, id, pattern);
        }
    }

    /// Binds a shape built from unknown tiles (background pass 2 or sprite pass S1), learning
    /// a permanent or a temporary pattern when the registry has none (REQ-E3D-160).
    fn bind_unknown_shape(
        &mut self,
        frame: &mut ConversionFrame,
        context: &mut SegmentContext<'_>,
        shape: ShapeId,
    ) {
        let data = frame.shape(shape);
        ImageSetKey::collect_ids(data.images(), &mut self.key);
        let id = if data.is_finished() {
            match context.patterns.find_permanent(&self.key) {
                Some(id) => id,
                None => context.learn(frame, shape, true),
            }
        } else {
            match context
                .patterns
                .find_temporary(&self.key)
                .or_else(|| context.patterns.find_temporary_containing(&self.key))
            {
                Some(id) => id,
                None => context.learn(frame, shape, false),
            }
        };

        if let Some(pattern) = context.patterns.get(id) {
            frame.bind(shape, id, pattern);
        }
    }
}

/// The direction of `neighbour` relative to `tile` for two sprites with tolerance
/// `tolerance`, or `None` when they are not adjacent (REQ-E3D-130). The four tests are made
/// in the order right, left, down, up and the first that fits wins.
fn adjacency(tile: &FrameTile, neighbour: &FrameTile, tolerance: i32) -> Option<GrowDirection> {
    let dx = (neighbour.x() - tile.x()).abs();
    let dy = (neighbour.y() - tile.y()).abs();

    if dy <= tolerance && (tile.x() + 8 - neighbour.x()).abs() <= tolerance {
        return Some(GrowDirection::Right);
    }

    if dy <= tolerance && (neighbour.x() + 8 - tile.x()).abs() <= tolerance {
        return Some(GrowDirection::Left);
    }

    if dx <= tolerance && (tile.y() + 8 - neighbour.y()).abs() <= tolerance {
        return Some(GrowDirection::Down);
    }

    if dx <= tolerance && (neighbour.y() + 8 - tile.y()).abs() <= tolerance {
        return Some(GrowDirection::Up);
    }

    None
}
