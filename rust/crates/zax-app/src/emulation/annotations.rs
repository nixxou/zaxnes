//! Drawings over the view (EDITOR-1, owner request): with the pens of the editor's tool box the
//! user circles a zone, writes a note, draws what is to be modelled and where; the MCP server's
//! `screenshot` shows them over the picture, so that Claude sees what the user means.
//!
//! The strokes are kept in coordinates of the view, 0 to 1 from its top left corner, so that
//! they stay in place when the window is resized and lay over a capture of any size. They are
//! only drawings: nothing of the game, the scene or the profile depends on them.

// Pixel coordinates of pictures a few thousand pixels wide, as f32 and back.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use serde_json::{json, Value};

/// A pen of the tool box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pen {
    /// Red.
    Red,
    /// Blue.
    Blue,
    /// Black.
    Black,
    /// White.
    White,
}

impl Pen {
    /// The pens in the order of the tool box.
    pub const ALL: [Self; 4] = [Self::Red, Self::Blue, Self::Black, Self::White];

    /// The colour of the pen.
    #[must_use]
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Red => [230, 30, 30],
            Self::Blue => [40, 90, 240],
            Self::Black => [0, 0, 0],
            Self::White => [255, 255, 255],
        }
    }

    /// The name the MCP server gives the pen.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Black => "black",
            Self::White => "white",
        }
    }
}

/// One stroke: a pen and the points it went through, in view coordinates (0 to 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// The pen.
    pub pen: Pen,
    /// The points, in order.
    pub points: Vec<[f32; 2]>,
}

/// The strokes drawn over the view, and the pen in hand.
#[derive(Debug, Clone, Default)]
pub struct Annotations {
    /// Whether the tool box is offered (the editor runs: `--mcp`).
    pub enabled: bool,
    /// The pen in hand; none: the pointer drives the camera as usual.
    pub pen: Option<Pen>,
    strokes: Vec<Stroke>,
    /// The stroke being drawn, while the button is held.
    current: Option<Stroke>,
}

/// Points closer than this to the previous one (in view coordinates) are not kept.
const MIN_STEP: f32 = 0.002;

impl Annotations {
    /// The finished strokes, then the one being drawn.
    pub fn strokes(&self) -> impl Iterator<Item = &Stroke> {
        self.strokes.iter().chain(self.current.as_ref())
    }

    /// Whether nothing is drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty() && self.current.is_none()
    }

    /// Starts a stroke with the pen in hand at `point`; nothing without a pen.
    pub fn begin(&mut self, point: [f32; 2]) {
        if let Some(pen) = self.pen {
            self.current = Some(Stroke {
                pen,
                points: vec![point],
            });
        }
    }

    /// Carries the stroke being drawn on to `point`.
    pub fn extend(&mut self, point: [f32; 2]) {
        let Some(stroke) = &mut self.current else {
            return;
        };
        let far_enough = stroke
            .points
            .last()
            .is_none_or(|last| (last[0] - point[0]).abs() + (last[1] - point[1]).abs() >= MIN_STEP);
        if far_enough {
            stroke.points.push(point);
        }
    }

    /// Ends the stroke being drawn.
    pub fn end(&mut self) {
        if let Some(stroke) = self.current.take() {
            self.strokes.push(stroke);
        }
    }

    /// Takes back the last stroke.
    pub fn undo(&mut self) {
        self.current = None;
        self.strokes.pop();
    }

    /// Wipes every stroke.
    pub fn clear(&mut self) {
        self.current = None;
        self.strokes.clear();
    }

    /// The strokes as the MCP server lists them: pen and points, in view coordinates and in
    /// pixels of a `width` x `height` view.
    #[must_use]
    pub fn to_json(&self, width: u32, height: u32) -> Value {
        let (w, h) = (f64::from(width), f64::from(height));
        let strokes: Vec<Value> = self
            .strokes()
            .map(|stroke| {
                let points: Vec<Value> = stroke
                    .points
                    .iter()
                    .map(|&[x, y]| {
                        let (x, y) = (f64::from(x), f64::from(y));
                        json!([round3(x), round3(y), (x * w).round(), (y * h).round()])
                    })
                    .collect();
                json!({ "pen": stroke.pen.name(), "points": points })
            })
            .collect();
        json!({
            "view": [width, height],
            "points": "[x, y, px, py]: x and y from 0 to 1 from the top left of the view, \
                       px and py in pixels of the view",
            "strokes": strokes,
        })
    }

    /// Draws the strokes into an RGB picture of `width` x `height` (rows from the top).
    pub fn draw_into(&self, rgb: &mut [u8], width: u32, height: u32) {
        let radius = (width.max(height) as f32 / 400.0).max(1.5);
        let (w, h) = (width as f32, height as f32);
        for stroke in self.strokes() {
            let color = stroke.pen.rgb();
            let to_pixels = |[x, y]: [f32; 2]| [x * w, y * h];
            let mut points = stroke.points.iter().map(|&p| to_pixels(p));
            let Some(mut previous) = points.next() else {
                continue;
            };
            disc(rgb, width, height, previous, radius, color);
            for point in points {
                segment(rgb, width, height, previous, point, radius, color);
                previous = point;
            }
        }
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// A thick segment: discs along it, a pixel apart.
fn segment(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    from: [f32; 2],
    to: [f32; 2],
    radius: f32,
    color: [u8; 3],
) {
    let length = (to[0] - from[0]).hypot(to[1] - from[1]);
    let steps = length.ceil().max(1.0) as u32;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let point = [
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
        ];
        disc(rgb, width, height, point, radius, color);
    }
}

/// A filled disc.
fn disc(rgb: &mut [u8], width: u32, height: u32, centre: [f32; 2], radius: f32, color: [u8; 3]) {
    let left = (centre[0] - radius).floor().max(0.0) as u32;
    let top = (centre[1] - radius).floor().max(0.0) as u32;
    let right = ((centre[0] + radius).ceil().max(0.0) as u32).min(width);
    let bottom = ((centre[1] + radius).ceil().max(0.0) as u32).min(height);
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - centre[0];
            let dy = y as f32 + 0.5 - centre[1];
            if dx * dx + dy * dy <= radius * radius {
                let at = ((y * width + x) * 3) as usize;
                if let Some(pixel) = rgb.get_mut(at..at + 3) {
                    pixel.copy_from_slice(&color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn() -> Annotations {
        let mut annotations = Annotations {
            pen: Some(Pen::Red),
            ..Annotations::default()
        };
        annotations.begin([0.1, 0.5]);
        annotations.extend([0.9, 0.5]);
        annotations.end();
        annotations
    }

    #[test]
    fn a_stroke_needs_a_pen_and_ends_in_the_list() {
        let mut annotations = Annotations::default();
        annotations.begin([0.5, 0.5]);
        annotations.extend([0.6, 0.5]);
        annotations.end();
        assert!(annotations.is_empty());

        let annotations = drawn();
        let strokes: Vec<_> = annotations.strokes().collect();
        assert_eq!(strokes.len(), 1);
        assert_eq!(strokes[0].pen, Pen::Red);
        assert_eq!(strokes[0].points, vec![[0.1, 0.5], [0.9, 0.5]]);
    }

    #[test]
    fn undo_and_clear_take_strokes_back() {
        let mut annotations = drawn();
        annotations.pen = Some(Pen::Blue);
        annotations.begin([0.2, 0.2]);
        annotations.end();
        annotations.undo();
        assert_eq!(annotations.strokes().count(), 1);
        annotations.clear();
        assert!(annotations.is_empty());
    }

    #[test]
    fn the_strokes_are_drawn_into_the_picture_where_they_lie() {
        let (width, height) = (100, 50);
        let mut rgb = vec![128; (width * height * 3) as usize];
        drawn().draw_into(&mut rgb, width, height);
        let pixel = |x: u32, y: u32| {
            let at = ((y * width + x) * 3) as usize;
            [rgb[at], rgb[at + 1], rgb[at + 2]]
        };
        assert_eq!(pixel(50, 25), Pen::Red.rgb());
        assert_eq!(pixel(12, 25), Pen::Red.rgb());
        assert_eq!(pixel(50, 5), [128, 128, 128]);
        assert_eq!(pixel(2, 25), [128, 128, 128]);
    }

    #[test]
    fn the_list_gives_view_and_pixel_coordinates() {
        let listed = drawn().to_json(200, 100);
        assert_eq!(listed["view"], json!([200, 100]));
        assert_eq!(listed["strokes"][0]["pen"], "red");
        assert_eq!(
            listed["strokes"][0]["points"][1],
            json!([0.9, 0.5, 180.0, 50.0])
        );
    }
}
