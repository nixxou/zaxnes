//! Lot F3D: the relief of the Famicom 3D System games in the default 3D.
//!
//! Those games draw one field for the left eye and the next for the right eye, and say which on
//! OUT1 of `$4016` (nesdev, "Famicom 3D System"), for the shutter glasses. The two pictures are
//! the same scene shifted sideways: the shift between the eyes (the disparity) is the depth the
//! game meant. Without a profile, the default 3D puts every shape on the screen plane; this
//! module measures the disparity on the tile records and gives each shape the depth a player
//! with the glasses perceives:
//!
//! * the background, per band of 8 lines (the core records one background row per band), from
//!   the shift of its tiles between a right field and the left fields around it;
//! * the sprites, per OAM slot, the same way.
//!
//! Two left fields are used, the one before the right field and the one after: the scrolling
//! between two fields adds the same shift to both comparisons with opposite signs, so their mean
//! is the disparity alone (*JJ* scrolls its floor 3 to 6 pixels a field, more than its relief).
//!
//! A background shape spans several bands: its depths are fitted with a plane `z = a + c·y`,
//! applied as an offset `a` and a rotation about X with the matching `1 / cos` stretch in Y, which
//! together are the shear `z += c·y` — the shape keeps its place on the screen and lies down.
//!
//! The displayed fields are the left ones only: converting both would make the scene jump sideways
//! by the disparity every frame.

use std::collections::VecDeque;

use zax_core_api::TileRecord;

use crate::frame::ConversionFrame;
use crate::settings::ConversionSettings;

/// How many alternating fields in a row make a game a 3D System one (a tenth of a second).
const ACTIVE_AFTER: u32 = 6;
/// The largest shift looked for between two fields, in pixels.
const MAX_SHIFT: i32 = 24;
/// The fields kept: the newest and the three before it.
const KEPT: usize = 4;

/// Where the picture is watched from, for the perceived depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewing {
    /// Distance from the eyes to the screen, in millimetres.
    pub distance_mm: f32,
    /// Width of the 256 NES pixels on that screen, in millimetres.
    pub picture_width_mm: f32,
    /// Distance between the two eyes, in millimetres.
    pub eyes_mm: f32,
}

impl Default for Viewing {
    /// A small CRT television (27 cm of picture) watched from 1.5 m.
    fn default() -> Self {
        Self {
            distance_mm: 1500.0,
            picture_width_mm: 270.0,
            eyes_mm: 65.0,
        }
    }
}

impl Viewing {
    /// The depth behind the screen (negative: in front), in NES pixels, that a disparity of
    /// `disparity` NES pixels is seen at: `z = D·p / (e − p)`, `p` the disparity on the screen.
    #[must_use]
    pub fn perceived(&self, disparity: f32) -> f32 {
        let mm_per_pixel = self.picture_width_mm / 256.0;
        let p = (disparity * mm_per_pixel).clamp(-self.eyes_mm * 4.0, self.eyes_mm * 0.9);
        self.distance_mm * p / (self.eyes_mm - p) / mm_per_pixel
    }
}

/// One field: which eye it was for and its tile records.
#[derive(Debug, Clone, Default)]
struct Field {
    right: bool,
    records: Vec<TileRecord>,
}

/// The disparity measured on a right field: per background band and per sprite slot, in NES
/// pixels, positive behind the screen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DepthField {
    /// `(top line of the band, disparity)`, by increasing line, measured bands only.
    pub bands: Vec<(i32, f32)>,
    /// `(OAM slot, disparity)`.
    pub sprites: Vec<(i32, f32)>,
}

impl DepthField {
    /// The disparity at screen line `y`: interpolated between the measured bands, the nearest one
    /// beyond them; 0 when no band was measured.
    #[must_use]
    pub fn at_line(&self, y: f32) -> f32 {
        let Some(&(first_y, first)) = self.bands.first() else {
            return 0.0;
        };
        let centre = |top: i32| top as f32 + 4.0;
        if y <= centre(first_y) {
            return first;
        }
        for pair in self.bands.windows(2) {
            let ((y0, d0), (y1, d1)) = (pair[0], pair[1]);
            if y <= centre(y1) {
                let t = (y - centre(y0)) / (centre(y1) - centre(y0)).max(1.0);
                return d0 + (d1 - d0) * t;
            }
        }
        self.bands.last().map_or(0.0, |band| band.1)
    }

    /// The disparity of a sprite slot, when it was measured.
    #[must_use]
    pub fn sprite(&self, slot: i32) -> Option<f32> {
        self.sprites
            .iter()
            .find(|(candidate, _)| *candidate == slot)
            .map(|(_, disparity)| *disparity)
    }
}

/// The fields of the game in play and whether it is a 3D System one.
#[derive(Debug, Clone, Default)]
pub struct StereoTracker {
    fields: VecDeque<Field>,
    alternations: u32,
}

impl StereoTracker {
    /// Forgets everything (a new game, a reset, a state loaded).
    pub fn clear(&mut self) {
        self.fields.clear();
        self.alternations = 0;
    }

    /// Records the field the core just ran: `outputs` the OUT0-OUT2 lines at its end (`None` on an
    /// engine that does not report them), `records` its tiles.
    pub fn push(&mut self, outputs: Option<u8>, records: &[TileRecord]) {
        let Some(outputs) = outputs else {
            self.clear();
            return;
        };
        let right = outputs & 0x02 != 0;
        let alternated = self.fields.back().is_some_and(|last| last.right != right);
        self.alternations = if alternated {
            self.alternations.saturating_add(1)
        } else {
            0
        };
        let mut field = if self.fields.len() >= KEPT {
            self.fields.pop_front().unwrap_or_default()
        } else {
            Field::default()
        };
        field.right = right;
        field.records.clear();
        field.records.extend_from_slice(records);
        self.fields.push_back(field);
    }

    /// Whether the game alternates the eyes field after field.
    #[must_use]
    pub const fn active(&self) -> bool {
        self.alternations >= ACTIVE_AFTER
    }

    /// The tile records of the newest left field, while active.
    #[must_use]
    pub fn left_records(&self) -> Option<&[TileRecord]> {
        if !self.active() {
            return None;
        }
        self.fields
            .iter()
            .rev()
            .find(|field| !field.right)
            .map(|field| field.records.as_slice())
    }

    /// The disparity of the newest right field that has a left field on each side, while active.
    #[must_use]
    pub fn depth_field(&self) -> Option<DepthField> {
        if !self.active() {
            return None;
        }
        let fields: Vec<&Field> = self.fields.iter().collect();
        let at = (1..fields.len().saturating_sub(1))
            .rev()
            .find(|&i| fields[i].right && !fields[i - 1].right && !fields[i + 1].right)?;
        let (before, right, after) = (
            &fields[at - 1].records,
            &fields[at].records,
            &fields[at + 1].records,
        );
        Some(DepthField {
            bands: band_disparities(before, right, after),
            sprites: sprite_disparities(before, right, after),
        })
    }
}

/// Whether every pixel of the tile has the same colour: such a tile matches anywhere.
const fn uniform(record: &TileRecord) -> bool {
    (record.plane0 == 0 || record.plane0 == u64::MAX)
        && (record.plane1 == 0 || record.plane1 == u64::MAX)
}

const fn same_tile(a: &TileRecord, b: &TileRecord) -> bool {
    a.plane0 == b.plane0 && a.plane1 == b.plane1 && a.palette == b.palette
}

/// The shift `x_left − x_right` most of the band's tiles agree on, with how many agree; `None`
/// when fewer than three textured tiles do.
fn band_shift(left: &[&TileRecord], right: &[&TileRecord]) -> Option<i32> {
    let mut votes = [0u32; (2 * MAX_SHIFT + 1) as usize];
    for r in right.iter().filter(|record| !uniform(record)) {
        for l in left.iter().filter(|candidate| same_tile(candidate, r)) {
            let shift = i32::from(l.x) - i32::from(r.x);
            if shift.abs() <= MAX_SHIFT {
                votes[(shift + MAX_SHIFT) as usize] += 1;
            }
        }
    }
    let (best, &count) = votes
        .iter()
        .enumerate()
        .max_by_key(|(index, count)| (**count, -(*index as i32 - MAX_SHIFT).abs()))?;
    (count >= 3).then(|| best as i32 - MAX_SHIFT)
}

/// The background tiles of `records` in the band starting at line `y`.
fn band<'a>(records: &'a [TileRecord], y: i16) -> Vec<&'a TileRecord> {
    records
        .iter()
        .filter(|record| record.background && record.y == y)
        .collect()
}

fn band_disparities(
    before: &[TileRecord],
    right: &[TileRecord],
    after: &[TileRecord],
) -> Vec<(i32, f32)> {
    let mut lines: Vec<i16> = right.iter().filter(|r| r.background).map(|r| r.y).collect();
    lines.sort_unstable();
    lines.dedup();
    let mut bands: Vec<(i32, f32)> = lines
        .into_iter()
        .filter_map(|y| {
            let r = band(right, y);
            let shift_before = band_shift(&band(before, y), &r)?;
            let shift_after = band_shift(&band(after, y), &r)?;
            // x_left − x_right = −d ∓ m: the mean of the two cancels the scrolling m.
            #[allow(clippy::cast_precision_loss)] // Shifts of a few pixels.
            Some((i32::from(y), -(shift_before + shift_after) as f32 / 2.0))
        })
        .collect();
    // A band far from its measured neighbours is a false match (a repeating pattern): drop it.
    let kept: Vec<(i32, f32)> = bands
        .iter()
        .enumerate()
        .filter(|(i, (_, d))| {
            let mut near: Vec<f32> = bands
                .iter()
                .enumerate()
                .filter(|(j, _)| j != i && j.abs_diff(*i) <= 3)
                .map(|(_, band)| band.1)
                .collect();
            if near.len() < 2 {
                return true;
            }
            near.sort_by(f32::total_cmp);
            (d - near[near.len() / 2]).abs() <= 2.5
        })
        .map(|(_, band)| *band)
        .collect();
    bands = kept;
    bands
}

fn sprite_disparities(
    before: &[TileRecord],
    right: &[TileRecord],
    after: &[TileRecord],
) -> Vec<(i32, f32)> {
    let find = |records: &[TileRecord], r: &TileRecord| {
        records
            .iter()
            .find(|l| !l.background && l.index == r.index && same_tile(l, r))
            .map(|l| i32::from(l.x) - i32::from(r.x))
            .filter(|shift| shift.abs() <= MAX_SHIFT)
    };
    right
        .iter()
        .filter(|r| !r.background)
        .filter_map(|r| {
            let (a, b) = (find(before, r)?, find(after, r)?);
            #[allow(clippy::cast_precision_loss)] // Shifts of a few pixels.
            Some((r.index, -(a + b) as f32 / 2.0))
        })
        .collect()
}

/// Gives every shape of `frame` the depth of `field` as seen from `viewing`.
#[allow(clippy::cast_precision_loss)] // Screen coordinates.
pub fn apply(
    frame: &mut ConversionFrame,
    settings: &ConversionSettings,
    field: &DepthField,
    viewing: &Viewing,
) {
    for id in frame.instances().to_vec() {
        let shape_id = frame.instance(id).shape();
        let shape = frame.shape(shape_id);
        let (top, bottom) = (
            shape.pixel_start().y as f32,
            (shape.pixel_end().y + 1) as f32,
        );
        let middle = (top + bottom) / 2.0;
        // One sample per tile: its centre line and the depth there.
        let samples: Vec<(f32, f32)> = frame
            .shape_tiles(shape_id)
            .map(|tile| {
                let y = tile.y() as f32 + 4.0;
                let disparity = if tile.background() {
                    field.at_line(y)
                } else {
                    field.sprite(tile.ord()).unwrap_or_else(|| field.at_line(y))
                };
                (y - middle, viewing.perceived(disparity))
            })
            .collect();
        if samples.is_empty() {
            continue;
        }
        let n = samples.len() as f32;
        let mean_z = samples.iter().map(|s| s.1).sum::<f32>() / n;
        let mean_y = samples.iter().map(|s| s.0).sum::<f32>() / n;
        let var_y: f32 = samples.iter().map(|s| (s.0 - mean_y).powi(2)).sum();
        let slope = if shape.background() && var_y > 1.0 {
            samples
                .iter()
                .map(|s| (s.0 - mean_y) * (s.1 - mean_z))
                .sum::<f32>()
                / var_y
        } else {
            0.0
        };
        // z at the shape's middle line, where the rotation pivots.
        let at_middle = mean_z - slope * mean_y;
        let instance = frame.instance_mut(id);
        instance.offset.z += at_middle;
        if slope != 0.0 && instance.rotation.x == 0.0 {
            // Scene Y grows upwards, screen y downwards: dz/dY = −slope. Rotation about X by θ
            // after a Y stretch of 1/cos θ moves a point at height Y to z = Y·tan θ.
            let angle = (-slope).atan().clamp(-1.4, 1.4);
            instance.rotation.x = angle.to_degrees();
            instance.scale.y /= angle.cos();
        }
    }
    frame.calculate_positions(settings);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: i16, y: i16, pattern: u64, background: bool, index: i32) -> TileRecord {
        TileRecord::new(pattern, 0, x, y, index, false, false, false, 1, background)
    }

    /// A band of three textured tiles at `y`, shifted by `shift`, and one sprite in slot 5.
    fn field(shift: i16, sprite_shift: i16) -> Vec<TileRecord> {
        vec![
            tile(16 + shift, 100, 0x0102_0304_0506_0708, true, 0),
            tile(24 + shift, 100, 0x1112_1314_1516_1718, true, 0),
            tile(32 + shift, 100, 0x2122_2324_2526_2728, true, 0),
            tile(64 + sprite_shift, 60, 0x3132_3334_3536_3738, false, 5),
        ]
    }

    fn tracker(fields: &[(bool, Vec<TileRecord>)]) -> StereoTracker {
        let mut tracker = StereoTracker::default();
        for (right, records) in fields {
            tracker.push(Some(if *right { 2 } else { 0 }), records);
        }
        tracker
    }

    #[test]
    fn inactive_until_the_eyes_alternate() {
        let mut tracker = StereoTracker::default();
        for _ in 0..10 {
            tracker.push(Some(0), &field(0, 0));
        }
        assert!(!tracker.active());
        assert!(tracker.depth_field().is_none());
        tracker.push(None, &field(0, 0));
        assert!(!tracker.active());
    }

    #[test]
    fn the_scrolling_cancels_and_the_disparity_stays() {
        // Disparity 4 behind the screen (right eye 2 px right of the scene, left eye 2 px left),
        // and the scene scrolls 3 px to the right every field.
        let mut fields = Vec::new();
        for t in 0..10i16 {
            let right = t % 2 == 1;
            let eye = if right { 2 } else { -2 };
            fields.push((right, field(3 * t + eye, 3 * t - eye)));
        }
        let tracker = tracker(&fields);
        assert!(tracker.active());
        let depth = tracker
            .depth_field()
            .expect("a right field between two left ones");
        assert_eq!(depth.bands, vec![(100, 4.0)]);
        // The sprite is in front: the left eye sees it 2 px right, the right eye 2 px left.
        assert_eq!(depth.sprite(5), Some(-4.0));
        assert!(tracker.left_records().is_some());
    }

    #[test]
    fn the_perceived_depth_grows_with_the_distance() {
        let viewing = Viewing::default();
        assert!((viewing.perceived(0.0)).abs() < 1e-6);
        let seven = viewing.perceived(7.0);
        assert!((175.0..190.0).contains(&seven), "{seven}");
        assert!(viewing.perceived(-4.0) < 0.0);
        let near = Viewing {
            distance_mm: 600.0,
            ..viewing
        };
        assert!(near.perceived(7.0) < seven);
    }

    #[test]
    fn the_depth_is_interpolated_between_bands() {
        let field = DepthField {
            bands: vec![(100, 2.0), (116, 6.0)],
            sprites: Vec::new(),
        };
        assert!((field.at_line(50.0) - 2.0).abs() < 1e-6);
        assert!((field.at_line(112.0) - 4.0).abs() < 1e-6);
        assert!((field.at_line(200.0) - 6.0).abs() < 1e-6);
    }
}
