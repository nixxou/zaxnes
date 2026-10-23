// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! Follows one shape from frame to frame (REQ-E3D-300). The subjective camera that follows a
//! character uses it (SPEC-06).

use std::collections::HashSet;

use crate::frame::{ConversionFrame, FrameShape, ShapeId};
use crate::registry::PatternId;
use crate::vectors::Vec3;

/// Follows one shape from frame to frame (REQ-E3D-300).
///
/// The state is the last known position, the followed layer, the set of *included* patterns
/// and the set of *excluded* ones. Both sets are only emptied when the tracker is restarted.
/// The walk stops at the first shape whose pattern is included; everything it met before,
/// included shapes aside, is excluded for good. For a sprite follow, "everything before"
/// covers all the background shapes of the frame, because the frame orders its shapes
/// background first (REQ-E3D-220); that is the original's rule and it is kept.
///
/// The tracker is part of the learned state a savestate carries (DEC-R2-T5).
#[derive(Debug, Clone, Default)]
pub struct ShapeTracker {
    // Membership is a set, order is a list: a run must be reproducible byte for byte
    // (REQ-ARCH-012), and a hash order is not (DEC-04-Q02).
    included: HashSet<PatternId>,
    excluded: HashSet<PatternId>,
    included_order: Vec<PatternId>,
    excluded_order: Vec<PatternId>,
    running: bool,
    sprites: bool,
    position: Vec3,
}

impl ShapeTracker {
    /// The proximity threshold δ, in pixels (REQ-E3D-300).
    pub const THRESHOLD: f32 = 9.0;

    /// Creates a stopped tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the tracker is following something.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// True when the followed layer is the sprite layer.
    #[must_use]
    pub const fn sprites(&self) -> bool {
        self.sprites
    }

    /// The last known position: the centre of the last shape found (REQ-E3D-044).
    #[must_use]
    pub const fn position(&self) -> Vec3 {
        self.position
    }

    /// The included patterns, in the order they were added; the tracker stops at the first
    /// shape carrying one.
    #[must_use]
    pub fn included(&self) -> &[PatternId] {
        &self.included_order
    }

    /// The excluded patterns, in the order they were added; the walk skips them.
    #[must_use]
    pub fn excluded(&self) -> &[PatternId] {
        &self.excluded_order
    }

    /// Starts the tracker on `shape`: the position is its centre, the layer is its layer and
    /// the included set holds its reference pattern (REQ-E3D-300).
    pub fn start(&mut self, shape: &FrameShape) {
        self.clear_sets();
        self.position = shape.center();
        self.sprites = !shape.background();
        if let Some(pattern) = shape.reference_pattern() {
            self.include(pattern);
        }

        self.running = true;
    }

    /// Stops the tracker.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Restores a tracker's state, as a savestate load does (DEC-R2-T5).
    pub fn restore(
        &mut self,
        running: bool,
        sprites: bool,
        position: Vec3,
        included: impl IntoIterator<Item = PatternId>,
        excluded: impl IntoIterator<Item = PatternId>,
    ) {
        self.running = running;
        self.sprites = sprites;
        self.position = position;
        self.clear_sets();
        for pattern in included {
            self.include(pattern);
        }

        for pattern in excluded {
            self.exclude(pattern);
        }
    }

    fn clear_sets(&mut self) {
        self.included.clear();
        self.excluded.clear();
        self.included_order.clear();
        self.excluded_order.clear();
    }

    fn include(&mut self, pattern: PatternId) {
        if self.included.insert(pattern) {
            self.included_order.push(pattern);
        }
    }

    fn exclude(&mut self, pattern: PatternId) {
        if self.excluded.insert(pattern) {
            self.excluded_order.push(pattern);
        }
    }

    /// The shape of `frame` that continues the followed one, or `None` (REQ-E3D-300).
    pub fn track(&mut self, frame: &ConversionFrame) -> Option<ShapeId> {
        if !self.running {
            return None;
        }

        let shapes = frame.shapes();
        let (start, end) = if self.sprites {
            (frame.background_shape_count(), shapes.len())
        } else {
            (0, frame.background_shape_count())
        };

        let mut candidate: Option<ShapeId> = None;
        let mut found: Option<ShapeId> = None;
        let mut best = Self::THRESHOLD;

        for i in start..end {
            let shape = frame.shape(shapes[i]);
            let Some(pattern) = shape.reference_pattern() else {
                continue;
            };

            if self.included.contains(&pattern) {
                // Case 1: everything up to and including this shape that is not included is
                // excluded for good, and so is the current candidate.
                for &earlier in shapes[..=i].iter().rev() {
                    if let Some(earlier) = frame.shape(earlier).reference_pattern() {
                        if !self.included.contains(&earlier) {
                            self.exclude(earlier);
                        }
                    }
                }

                if let Some(previous) = candidate.and_then(|id| frame.shape(id).reference_pattern())
                {
                    self.exclude(previous);
                }

                found = Some(shapes[i]);
                break;
            }

            if self.excluded.contains(&pattern) {
                // Case 2.
                continue;
            }

            let distance = Vec3::distance(self.position, shape.center());
            if distance < best {
                // Case 3: a closer shape; the previous candidate is excluded.
                if let Some(previous) = candidate.and_then(|id| frame.shape(id).reference_pattern())
                {
                    self.exclude(previous);
                }

                candidate = Some(shapes[i]);
                best = distance;
            } else {
                // Case 4.
                self.exclude(pattern);
            }
        }

        if let Some(found) = found {
            self.position = frame.shape(found).center();
            return Some(found);
        }

        let candidate = candidate?;
        let shape = frame.shape(candidate);
        self.position = shape.center();
        if let Some(tracked) = shape.reference_pattern() {
            self.include(tracked);
        }

        Some(candidate)
    }
}
