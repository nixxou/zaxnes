//! Lot F3D: the relief of the Famicom 3D System games in the default 3D.
//!
//! Those games draw one field for the left eye and the next for the right eye, and say which on
//! OUT1 of `$4016` (nesdev, "Famicom 3D System"), for the shutter glasses. The two pictures are
//! the same scene shifted sideways: the shift between the eyes (the disparity) is the depth the
//! game meant. Without a profile, the default 3D puts every shape on the screen plane; this
//! module measures the disparity on the tile records and gives each shape the depth a player
//! with the glasses perceives:
//!
//! * the background, per line of the picture, from the shift that best lines up the line of a
//!   right field with the same line of the left fields around it (the games scroll the floor
//!   line by line; the tile records keep one position per band of 8 lines, too coarse);
//! * the sprites, per OAM slot, from the tile records.
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
use crate::grid;
use crate::settings::ConversionSettings;

/// How many alternating fields in a row make a game a 3D System one (a tenth of a second).
const ACTIVE_AFTER: u32 = 6;
/// The largest shift looked for between two fields, in pixels.
const MAX_SHIFT: i32 = 24;
/// The fields kept: the newest and the three before it.
const KEPT: usize = 4;
/// The default camera's target on the screen plane, in scene coordinates (Y up): `T'` of
/// `render --explain`.
const AXIS_X: f32 = 124.0;
const AXIS_Y: f32 = 124.0;

/// Where the picture is watched from, for the perceived depth, and where the scene is looked at
/// from, to keep it in place on the screen. Lengths in NES pixels of the picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewing {
    /// Distance from the player's eyes to the screen.
    pub viewer: f32,
    /// Distance between the two eyes.
    pub eyes: f32,
    /// Distance from the default camera to the screen plane: every shape is enlarged by
    /// `(camera + z) / camera` about the camera's axis, so that seen from there it covers the
    /// screen exactly as the flat picture does, whatever its depth.
    pub camera: f32,
}

/// NES pixels per millimetre of a 27 cm wide picture (a small CRT television).
const PIXELS_PER_MM: f32 = 256.0 / 270.0;
/// Distance from the default camera to the screen plane (`render --explain`: `d 189.65`).
pub const DEFAULT_CAMERA: f32 = 189.65;

impl Default for Viewing {
    /// The player sits where the default camera is: the relief a player with the glasses sees
    /// from there, and the default view shows the picture as it is.
    fn default() -> Self {
        Self {
            viewer: DEFAULT_CAMERA,
            eyes: 65.0 * PIXELS_PER_MM,
            camera: DEFAULT_CAMERA,
        }
    }
}

impl Viewing {
    /// A small television watched from 1.5 m: the full depth of the glasses, deeper than the
    /// default camera's place gives.
    #[must_use]
    pub fn television() -> Self {
        Self {
            viewer: 1500.0 * PIXELS_PER_MM,
            ..Self::default()
        }
    }

    /// The depth behind the screen (negative: in front), in NES pixels, that a disparity of
    /// `disparity` NES pixels is seen at: `z = D·p / (e − p)`.
    #[must_use]
    pub fn perceived(&self, disparity: f32) -> f32 {
        let p = disparity.clamp(-self.eyes * 4.0, self.eyes * 0.9);
        self.viewer * p / (self.eyes - p)
    }
}

/// One field: which eye it was for, its tile records and its picture (NES colour indices, 256 x
/// 240, kept only while the eyes alternate).
#[derive(Debug, Clone, Default)]
struct Field {
    /// Counts the fields pushed, to know when the measure has to be taken again.
    serial: u64,
    right: bool,
    records: Vec<TileRecord>,
    picture: Vec<u8>,
}

/// Picture size.
const WIDTH: usize = 256;
const HEIGHT: usize = 240;
/// The same, signed, for the sprite boxes that reach past it.
const WIDTH_I: i32 = 256;
const HEIGHT_I: i32 = 240;
/// The columns compared, clear of the sides a shift brings in.
const COMPARED: std::ops::Range<usize> = 24..232;
/// A line counts as measured when the best shift beats the worst by this many pixels and by four
/// times over: enough texture to be scenery, not a lone sprite on a plain sky.
const TEXTURE: u32 = 16;
/// The two comparisons of a line differ by twice the scrolling; more than this, one of them
/// matched something else (a shot crossing the line).
const MAX_SCROLL_GAP: i32 = 12;

/// The disparity measured on a right field: per line and per sprite slot, in NES pixels,
/// positive behind the screen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DepthField {
    /// `(line, disparity)` of the measured lines, by increasing line.
    pub bands: Vec<(i32, f32)>,
    /// `(OAM slot, disparity)`.
    pub sprites: Vec<(i32, f32)>,
}

impl DepthField {
    /// The disparity at screen line `y`: interpolated between the measured bands, the nearest one
    /// beyond them; 0 when no band was measured.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Screen lines, below 256.
    pub fn at_line(&self, y: f32) -> f32 {
        let Some(&(first_y, first)) = self.bands.first() else {
            return 0.0;
        };
        let centre = |line: i32| line as f32 + 0.5;
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
    pushed: u64,
    /// The last measure and the serial of the right field it was taken on: a right field is
    /// measured once, when the left field after it arrives.
    measured: Option<(u64, DepthField)>,
}

impl StereoTracker {
    /// Forgets everything (a new game, a reset, a state loaded).
    pub fn clear(&mut self) {
        self.fields.clear();
        self.alternations = 0;
        self.measured = None;
    }

    /// Records the field the core just ran: `outputs` the OUT0-OUT2 lines at its end (`None` on an
    /// engine that does not report them), `records` its tiles; `picture` copies its picture into
    /// the buffer it is given, and is called only while the eyes alternate.
    pub fn push(
        &mut self,
        outputs: Option<u8>,
        records: &[TileRecord],
        picture: impl FnOnce(&mut Vec<u8>),
    ) {
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
        self.pushed += 1;
        field.serial = self.pushed;
        field.right = right;
        field.records.clear();
        field.records.extend_from_slice(records);
        if self.alternations > 0 {
            picture(&mut field.picture);
            mask_sprites(&mut field.picture, records);
        } else {
            field.picture.clear();
        }
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
    pub fn depth_field(&mut self) -> Option<DepthField> {
        if !self.active() {
            return None;
        }
        let fields: Vec<&Field> = self.fields.iter().collect();
        let at = (1..fields.len().saturating_sub(1))
            .rev()
            .find(|&i| fields[i].right && !fields[i - 1].right && !fields[i + 1].right)?;
        let (before, right, after) = (fields[at - 1], fields[at], fields[at + 1]);
        if let Some((serial, depth)) = &self.measured {
            if *serial == right.serial {
                return Some(depth.clone());
            }
        }
        let full = |field: &Field| field.picture.len() == WIDTH * HEIGHT;
        if !(full(before) && full(right) && full(after)) {
            return None;
        }
        let depth = DepthField {
            bands: line_disparities(&before.picture, &right.picture, &after.picture),
            sprites: sprite_disparities(&before.records, &right.records, &after.records),
        };
        self.measured = Some((right.serial, depth.clone()));
        Some(depth)
    }
}

const fn same_tile(a: &TileRecord, b: &TileRecord) -> bool {
    a.plane0 == b.plane0 && a.plane1 == b.plane1 && a.palette == b.palette
}

/// Pixels under a sprite in the copied picture: they match anything, so that the lines measure
/// the scenery alone (the sprites have their own measure).
const MASKED: u8 = 0xFF;

/// Marks the 8 x 8 box of every sprite record of `records` as [`MASKED`] in `picture`.
fn mask_sprites(picture: &mut [u8], records: &[TileRecord]) {
    if picture.len() != WIDTH * HEIGHT {
        return;
    }
    for record in records.iter().filter(|record| !record.background) {
        let (x0, y0) = (i32::from(record.x), i32::from(record.y));
        for y in y0.max(0)..(y0 + 8).min(HEIGHT_I) {
            for x in x0.max(0)..(x0 + 8).min(WIDTH_I) {
                #[allow(clippy::cast_sign_loss)] // Clamped to the picture above.
                let index = y as usize * WIDTH + x as usize;
                picture[index] = MASKED;
            }
        }
    }
}

/// The shift `s` that best lines up line `y` of `right` with `left` (`left[x + s] = right[x]`), or
/// `None` for a line without texture.
fn line_shift(left: &[u8], right: &[u8], line: usize) -> Option<i32> {
    let row = line * WIDTH..(line + 1) * WIDTH;
    let (left, right) = (&left[row.clone()], &right[row]);
    let (mut best, mut best_shift, mut worst) = (u32::MAX, 0i32, 0u32);
    for shift in -MAX_SHIFT..=MAX_SHIFT {
        let mismatches = COMPARED
            .map(|x| {
                let (seen_left, seen_right) =
                    (left[x.wrapping_add_signed(shift as isize)], right[x]);
                u32::from(seen_left != seen_right && seen_left != MASKED && seen_right != MASKED)
            })
            .sum::<u32>();
        if mismatches < best || (mismatches == best && shift.abs() < best_shift.abs()) {
            best = mismatches;
            best_shift = shift;
        }
        worst = worst.max(mismatches);
    }
    (worst - best >= TEXTURE && best.saturating_mul(4) <= worst && best_shift.abs() < MAX_SHIFT)
        .then_some(best_shift)
}

fn line_disparities(before: &[u8], right: &[u8], after: &[u8]) -> Vec<(i32, f32)> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 240 lines.
    let lines: Vec<(i32, f32)> = (0..HEIGHT)
        .filter_map(|y| {
            let shift_before = line_shift(before, right, y)?;
            let shift_after = line_shift(after, right, y)?;
            if (shift_before - shift_after).abs() > MAX_SCROLL_GAP {
                return None;
            }
            // x_left − x_right = −d ∓ m: the mean of the two cancels the scrolling m.
            #[allow(clippy::cast_precision_loss)] // Shifts of a few pixels.
            Some((y as i32, -(shift_before + shift_after) as f32 / 2.0))
        })
        .collect();
    // A line far from its measured neighbours is a false match (a repeating pattern): drop it.
    lines
        .iter()
        .enumerate()
        .filter(|(i, (line, d))| {
            let mut near: Vec<f32> = lines
                .iter()
                .enumerate()
                .filter(|(j, (other, _))| j != i && (other - line).abs() <= 6)
                .map(|(_, other)| other.1)
                .collect();
            if near.len() < 3 {
                return true;
            }
            near.sort_by(f32::total_cmp);
            (d - near[near.len() / 2]).abs() <= 2.5
        })
        .map(|(_, line)| *line)
        .collect()
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
        let middle = f32::midpoint(top, bottom);
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
        // z at the shape's middle line, where the rotation pivots, and the enlargement that keeps
        // the shape where the flat picture has it, seen from the camera.
        let at_middle = mean_z - slope * mean_y;
        let enlargement = ((viewing.camera + at_middle) / viewing.camera).max(0.05);
        let centre_x = (shape.pixel_start().x + shape.pixel_end().x + 1) as f32 / 2.0;
        let centre_y = grid::SCENE_HEIGHT as f32 - middle;
        let instance = frame.instance_mut(id);
        instance.offset.x += (centre_x - AXIS_X) * (enlargement - 1.0);
        instance.offset.y += (centre_y - AXIS_Y) * (enlargement - 1.0);
        instance.offset.z += at_middle;
        instance.scale.x *= enlargement;
        if slope != 0.0 && instance.rotation.x == 0.0 {
            // Scene Y grows upwards, screen y downwards: dz/dY = −slope. A point at height Y must
            // go to (k·Y, −slope·Y): a stretch s then a rotation θ about X give (s·cos θ·Y,
            // s·sin θ·Y), so tan θ = −slope / k and s = √(k² + slope²).
            let angle = (-slope / enlargement).atan().clamp(-1.4, 1.4);
            instance.rotation.x = angle.to_degrees();
            instance.scale.y *= enlargement.hypot(slope);
        } else {
            instance.scale.y *= enlargement;
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

    /// One sprite in slot 5 at `64 + x`.
    fn records(x: i16) -> Vec<TileRecord> {
        vec![tile(64 + x, 60, 0x3132_3334_3536_3738, false, 5)]
    }

    /// A picture whose line 100 carries a pseudo-random pattern moved right by `shift`, the rest
    /// plain.
    fn picture(shift: i32) -> Vec<u8> {
        let mut picture = vec![0x21u8; WIDTH * HEIGHT];
        for x in 0..WIDTH {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let source = (x as i32 - shift).rem_euclid(WIDTH as i32) as u32;
            #[allow(clippy::cast_possible_truncation)]
            let colour = ((source.wrapping_mul(2_654_435_761) >> 13) & 7) as u8;
            picture[100 * WIDTH + x] = colour;
        }
        picture
    }

    #[test]
    fn a_right_field_is_measured_once() {
        let fields: Vec<(bool, i32, i16)> =
            (0..9i16).map(|t| (t % 2 == 1, i32::from(t), t)).collect();
        let mut tracker = tracker(&fields);
        let first = tracker.depth_field();
        assert!(first.is_some());
        assert_eq!(tracker.measured.as_ref().map(|m| m.0), Some(8));
        assert_eq!(tracker.depth_field(), first);
    }

    fn tracker(fields: &[(bool, i32, i16)]) -> StereoTracker {
        let mut tracker = StereoTracker::default();
        for &(right, shift, sprite) in fields {
            tracker.push(Some(if right { 2 } else { 0 }), &records(sprite), |out| {
                *out = picture(shift);
            });
        }
        tracker
    }

    #[test]
    fn inactive_until_the_eyes_alternate() {
        let mut tracker = StereoTracker::default();
        for _ in 0..10 {
            tracker.push(Some(0), &records(0), |_| {
                panic!("no picture is copied while the eyes do not alternate");
            });
        }
        assert!(!tracker.active());
        assert!(tracker.depth_field().is_none());
        tracker.push(None, &records(0), |_| {});
        assert!(!tracker.active());
    }

    #[test]
    fn the_scrolling_cancels_and_the_disparity_stays() {
        // Line 100 is 4 behind the screen (right eye 2 px right, left eye 2 px left), the sprite
        // 4 in front, and everything scrolls 3 px to the right every field.
        let fields: Vec<(bool, i32, i16)> = (0..10i16)
            .map(|t| {
                let right = t % 2 == 1;
                let eye = if right { 2 } else { -2 };
                (right, i32::from(3 * t + eye), 3 * t - eye)
            })
            .collect();
        let mut tracker = tracker(&fields);
        assert!(tracker.active());
        let depth = tracker
            .depth_field()
            .expect("a right field between two left ones");
        assert_eq!(depth.bands, vec![(100, 4.0)]);
        assert_eq!(depth.sprite(5), Some(-4.0));
        assert!(tracker.left_records().is_some());
    }

    #[test]
    fn the_perceived_depth_grows_with_the_distance() {
        let camera = Viewing::default();
        assert!((camera.perceived(0.0)).abs() < 1e-6);
        let television = Viewing::television();
        let seven = television.perceived(7.0);
        assert!((175.0..190.0).contains(&seven), "{seven}");
        assert!(television.perceived(-4.0) < 0.0);
        assert!(camera.perceived(7.0) < seven / 5.0);
    }

    #[test]
    fn the_depth_is_interpolated_between_lines() {
        let field = DepthField {
            bands: vec![(100, 2.0), (120, 6.0)],
            sprites: Vec::new(),
        };
        assert!((field.at_line(50.0) - 2.0).abs() < 1e-6);
        assert!((field.at_line(110.5) - 4.0).abs() < 1e-6);
        assert!((field.at_line(200.0) - 6.0).abs() < 1e-6);
    }
}
