//! `classify` (Rust-only, the editor mode): sorts the tiles of frame dumps into classes by rules
//! and writes them as patterns, the step every profile starts with.
//!
//! The rules (`rules.json`, paths relative to it):
//!
//! ```json
//! { "dumps": ["dumps"],                        // dump files, or folders of them (*.json)
//!   "classes": [
//!     { "name": "hud",   "where": { "y": [0, 31] },
//!       "pattern": { "layer": 1, "depth": 2 } },
//!     { "name": "floor", "where": { "palette": [0], "sprite": false, "y": [176, 239] },
//!       "pattern": { "depth": 48, "offset": [0, 0, 0], "tags": "solid" } },
//!     { "name": "back",  "where": { "colours": [17, 33] }, "pattern": { "depth": 2, "offset": [0, 0, 46] } } ] }
//! ```
//!
//! `where` holds conditions on one occurrence of a tile in a dump, all to hold: `palette` (the
//! sub-palettes, 0 to 3), `sprite`, `x` and `y` (inclusive ranges of the tile's top-left
//! pixel), `frames` (dump file names), `colours` (NES colour indices, 0 to 63: the occurrence
//! shows at least one of them), `tiles` / `not_tiles` (picture keys of the report). An empty
//! `where` takes everything left.
//!
//! A pattern knows only a tile's picture, so a picture goes to the **first** class (in the
//! order of the list) that one of its occurrences matches; its other occurrences are listed in
//! the report as `shared` (the same picture in two classes: a depth to give in Lua, by palette
//! or place). Pictures no class takes are `unclaimed`, with where they show. A sprite drawn
//! mirrored is a picture of its own (the profile matches mirrored sprites only as they are
//! drawn).
//!
//! Outputs: the description to put in `profile.json`'s `includes` (tiles `<class>_<n>`, one
//! one-tile pattern each, tagged with the class name and the class's `tags`), the report, and
//! a picture sheet: one band of tiles per class in their colours, then the unclaimed ones.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use zax_engine3d::palette::nes_color;
use zax_profiles::enums::TileFlip;
use zax_profiles::tile::ProfileTile;

use crate::HarnessError;

/// The usage of the command.
pub const USAGE: &str = "zax-harness classify <rules.json> --out <classified.json>
       [--report <report.json>] [--sheet <sheet.png>] [<dump.json | folder> ...]
  Sorts the tiles of frame dumps (dump_frame, render --dump-frame) into the classes of the
  rules and writes them as patterns, for the \"includes\" of a profile.json (see the source of
  classify.rs, or the editor folder's docs/CLASSIFY.md).";

/// The options of the command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The rules file.
    pub rules: PathBuf,
    /// Where the description goes.
    pub out: PathBuf,
    /// Where the report goes; beside the description when absent.
    pub report: Option<PathBuf>,
    /// Where the picture sheet goes, if wanted.
    pub sheet: Option<PathBuf>,
    /// Dumps given on the command line, added to the rules' own.
    pub dumps: Vec<PathBuf>,
}

impl Options {
    /// Reads the command line after the command name.
    ///
    /// # Errors
    /// [`HarnessError::Usage`] on a missing or unknown argument.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let mut options = Self::default();
        let mut rules = None;
        let mut out = None;
        let mut remaining = arguments.iter();
        while let Some(argument) = remaining.next() {
            let mut value = |name: &str| {
                remaining
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| HarnessError::Usage(format!("{name} needs a value\n{USAGE}")))
            };
            match argument.as_str() {
                "--out" => out = Some(value("--out")?),
                "--report" => options.report = Some(value("--report")?),
                "--sheet" => options.sheet = Some(value("--sheet")?),
                flag if flag.starts_with("--") => {
                    return Err(HarnessError::Usage(format!(
                        "unknown option {flag}\n{USAGE}"
                    )));
                }
                path if rules.is_none() => rules = Some(PathBuf::from(path)),
                path => options.dumps.push(PathBuf::from(path)),
            }
        }
        options.rules = rules.ok_or_else(|| HarnessError::Usage(USAGE.to_owned()))?;
        options.out = out.ok_or_else(|| HarnessError::Usage(USAGE.to_owned()))?;
        Ok(options)
    }
}

/// A picture: its two planes and how a sprite is mirrored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Picture {
    low: u64,
    high: u64,
    /// `""`, `"h"`, `"v"` or `"hv"`.
    mirror: &'static str,
}

impl Picture {
    fn key(self) -> String {
        let suffix = if self.mirror.is_empty() {
            String::new()
        } else {
            format!("_{}", self.mirror)
        };
        format!("t{:016x}{:016x}{suffix}", self.low, self.high)
    }

    fn tile(self) -> ProfileTile {
        let flip = match self.mirror {
            "h" => TileFlip::HORIZONTAL,
            "v" => TileFlip::VERTICAL,
            "hv" => TileFlip::from_bits(3),
            _ => TileFlip::NONE,
        };
        ProfileTile::new(self.low, self.high, flip)
    }

    fn planes(self) -> Value {
        let mut planes = vec![
            json!(format!("{:016x}", self.low)),
            json!(format!("{:016x}", self.high)),
        ];
        if !self.mirror.is_empty() {
            planes.push(json!(self.mirror));
        }
        Value::Array(planes)
    }
}

/// One occurrence of a picture in a dump.
#[derive(Debug, Clone)]
struct Occurrence {
    picture: Picture,
    frame: String,
    x: i64,
    y: i64,
    palette: u8,
    sprite: bool,
    /// The four NES colour indices it is drawn with (0: the backdrop).
    colours: [u8; 4],
}

/// The conditions of a class.
#[derive(Debug, Clone, Default)]
struct Filter {
    palette: Option<Vec<u8>>,
    sprite: Option<bool>,
    x: Option<(i64, i64)>,
    y: Option<(i64, i64)>,
    frames: Option<Vec<String>>,
    colours: Option<Vec<u8>>,
    tiles: Option<Vec<String>>,
    not_tiles: Vec<String>,
}

impl Filter {
    fn read(value: Option<&Value>, class: &str) -> Result<Self, HarnessError> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        let bad = |field: &str| HarnessError::Format(format!("class {class}: bad \"{field}\""));
        let numbers = |field: &str| -> Result<Option<Vec<i64>>, HarnessError> {
            value
                .get(field)
                .map(|list| {
                    list.as_array()
                        .ok_or_else(|| bad(field))?
                        .iter()
                        .map(|n| n.as_i64().ok_or_else(|| bad(field)))
                        .collect()
                })
                .transpose()
        };
        let range = |field: &str| -> Result<Option<(i64, i64)>, HarnessError> {
            match numbers(field)? {
                None => Ok(None),
                Some(list) if list.len() == 2 => Ok(Some((list[0], list[1]))),
                Some(_) => Err(bad(field)),
            }
        };
        let texts = |field: &str| -> Result<Option<Vec<String>>, HarnessError> {
            value
                .get(field)
                .map(|list| {
                    list.as_array()
                        .ok_or_else(|| bad(field))?
                        .iter()
                        .map(|t| t.as_str().map(str::to_owned).ok_or_else(|| bad(field)))
                        .collect()
                })
                .transpose()
        };
        let bytes = |field: &str| -> Result<Option<Vec<u8>>, HarnessError> {
            numbers(field)?
                .map(|list| {
                    list.into_iter()
                        .map(|n| u8::try_from(n).map_err(|_| bad(field)))
                        .collect()
                })
                .transpose()
        };
        Ok(Self {
            palette: bytes("palette")?,
            sprite: value.get("sprite").and_then(Value::as_bool),
            x: range("x")?,
            y: range("y")?,
            frames: texts("frames")?,
            colours: bytes("colours")?,
            tiles: texts("tiles")?,
            not_tiles: texts("not_tiles")?.unwrap_or_default(),
        })
    }

    fn matches(&self, occurrence: &Occurrence) -> bool {
        let within = |range: Option<(i64, i64)>, value: i64| {
            range.is_none_or(|(low, high)| low <= value && value <= high)
        };
        let key = occurrence.picture.key();
        self.palette
            .as_ref()
            .is_none_or(|list| list.contains(&occurrence.palette))
            && self.sprite.is_none_or(|sprite| sprite == occurrence.sprite)
            && within(self.x, occurrence.x)
            && within(self.y, occurrence.y)
            && self
                .frames
                .as_ref()
                .is_none_or(|list| list.contains(&occurrence.frame))
            && self.colours.as_ref().is_none_or(|list| {
                let tile = occurrence.picture.tile();
                (0..8).any(|y| {
                    (0..8).any(|x| {
                        tile.color_at(x, y).is_some_and(|c| {
                            c != 0 && list.contains(&occurrence.colours[c as usize])
                        })
                    })
                })
            })
            && self.tiles.as_ref().is_none_or(|list| list.contains(&key))
            && !self.not_tiles.contains(&key)
    }
}

/// A class of the rules.
#[derive(Debug, Clone)]
struct Class {
    name: String,
    filter: Filter,
    /// The pattern fields given to each of its tiles (`grid` excepted).
    pattern: Map<String, Value>,
}

fn read_json(path: &Path) -> Result<Value, HarnessError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| HarnessError::Format(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| HarnessError::Format(format!("{}: {e}", path.display())))
}

/// The dump files a path names: itself, or the `.json` files of a folder, sorted.
fn dump_files(path: &Path) -> Result<Vec<PathBuf>, HarnessError> {
    if path.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(path)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|file| file.extension().is_some_and(|ext| ext == "json"))
            .collect();
        files.sort();
        Ok(files)
    } else {
        Ok(vec![path.to_path_buf()])
    }
}

/// The occurrences of one dump; a JSON file that is no dump (no `tiles` list) gives none.
fn read_dump(path: &Path) -> Result<Vec<Occurrence>, HarnessError> {
    let dump = read_json(path)?;
    let Some(tiles) = dump.get("tiles").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut palette = [0_u8; 32];
    for (slot, value) in palette.iter_mut().zip(
        dump.get("palette")
            .and_then(Value::as_array)
            .into_iter()
            .flatten(),
    ) {
        *slot = value
            .as_u64()
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(0)
            & 63;
    }
    let frame = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut occurrences = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let number = |field: &str| tile.get(field).and_then(Value::as_i64).unwrap_or(0);
        let flag = |field: &str| tile.get(field).and_then(Value::as_bool).unwrap_or(false);
        let (Some(low), Some(high)) = (
            tile.get("low").and_then(Value::as_u64),
            tile.get("high").and_then(Value::as_u64),
        ) else {
            continue;
        };
        let sprite = flag("sprite");
        let mirror = match (sprite && flag("mirrorH"), sprite && flag("mirrorV")) {
            (true, true) => "hv",
            (true, false) => "h",
            (false, true) => "v",
            (false, false) => "",
        };
        let sub = u8::try_from(number("palette")).unwrap_or(0) & 3;
        let base = usize::from(sub) * 4 + if sprite { 16 } else { 0 };
        occurrences.push(Occurrence {
            picture: Picture { low, high, mirror },
            frame: frame.clone(),
            x: number("x"),
            y: number("y"),
            palette: sub,
            sprite,
            colours: [
                palette[0],
                palette[base + 1],
                palette[base + 2],
                palette[base + 3],
            ],
        });
    }
    Ok(occurrences)
}

/// What the rules made of the dumps.
struct Outcome {
    /// For each class, its pictures in the order they were first seen.
    classes: Vec<Vec<Picture>>,
    /// The pictures no class took.
    unclaimed: Vec<Picture>,
    /// Each picture's occurrences, and the class each one matched (None: none).
    occurrences: HashMap<Picture, Vec<(Occurrence, Option<usize>)>>,
}

fn classify(classes: &[Class], occurrences: Vec<Occurrence>) -> Outcome {
    let mut order: Vec<Picture> = Vec::new();
    let mut seen: HashMap<Picture, Vec<(Occurrence, Option<usize>)>> = HashMap::new();
    for occurrence in occurrences {
        let class = classes.iter().position(|c| c.filter.matches(&occurrence));
        let list = seen.entry(occurrence.picture).or_insert_with(|| {
            order.push(occurrence.picture);
            Vec::new()
        });
        list.push((occurrence, class));
    }
    let mut by_class = vec![Vec::new(); classes.len()];
    let mut unclaimed = Vec::new();
    for picture in order {
        let first = seen[&picture].iter().filter_map(|(_, class)| *class).min();
        match first {
            Some(class) => by_class[class].push(picture),
            None => unclaimed.push(picture),
        }
    }
    Outcome {
        classes: by_class,
        unclaimed,
        occurrences: seen,
    }
}

/// A picture sheet: bands of tiles, `SCALE` times, `PER_ROW` to a row.
const SCALE: usize = 3;
const PER_ROW: usize = 32;
const CELL: usize = 8 * SCALE + 2;

fn sheet(outcome: &Outcome) -> (usize, usize, Vec<u8>, Vec<(usize, usize)>) {
    let bands: Vec<&Vec<Picture>> = outcome
        .classes
        .iter()
        .chain(std::iter::once(&outcome.unclaimed))
        .collect();
    let rows_of = |count: usize| count.div_ceil(PER_ROW).max(1);
    let total_rows: usize = bands.iter().map(|band| rows_of(band.len())).sum();
    let width = PER_ROW * CELL;
    // a 4-pixel rule between the bands
    let height = total_rows * CELL + bands.len() * 4;
    let mut pixels = vec![40_u8; width * height * 3];
    let mut row = 0;
    let mut top = 0;
    let mut placed = Vec::new();
    for band in bands {
        let rows = rows_of(band.len());
        placed.push((row, row + rows - 1));
        for (index, picture) in band.iter().enumerate() {
            let colours = outcome.occurrences[picture][0].0.colours;
            let tile = picture.tile();
            let left = (index % PER_ROW) * CELL + 1;
            let cell_top = top + (index / PER_ROW) * CELL + 1;
            for y in 0..8_u32 {
                for x in 0..8_u32 {
                    let colour = tile.color_at(x, y).unwrap_or(0) as usize;
                    let rgb = nes_color(usize::from(colours[colour]));
                    for dy in 0..SCALE {
                        for dx in 0..SCALE {
                            let px = left + x as usize * SCALE + dx;
                            let py = cell_top + y as usize * SCALE + dy;
                            let at = (py * width + px) * 3;
                            pixels[at..at + 3].copy_from_slice(&[rgb.r, rgb.g, rgb.b]);
                        }
                    }
                }
            }
        }
        row += rows;
        top += rows * CELL;
        for y in top..top + 4 {
            for x in 0..width {
                let at = (y * width + x) * 3;
                pixels[at..at + 3].copy_from_slice(&[200, 0, 200]);
            }
        }
        top += 4;
    }
    (width, height, pixels, placed)
}

/// Runs the command; returns its summary.
///
/// # Errors
/// A file cannot be read or written, or the rules are malformed.
#[allow(clippy::too_many_lines)] // the reading, the outputs and the summary, in their order
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let rules = read_json(&options.rules)?;
    let base = options.rules.parent().unwrap_or(Path::new("."));
    let mut classes = Vec::new();
    for entry in rules
        .get("classes")
        .and_then(Value::as_array)
        .ok_or_else(|| HarnessError::Format("the rules have no \"classes\" list".to_owned()))?
    {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty() && !name.contains(char::is_whitespace))
            .ok_or_else(|| {
                HarnessError::Format(format!("a class needs a one-word \"name\": {entry}"))
            })?
            .to_owned();
        let filter = Filter::read(entry.get("where"), &name)?;
        let pattern = entry
            .get("pattern")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        classes.push(Class {
            name,
            filter,
            pattern,
        });
    }
    let mut dumps = Vec::new();
    for path in rules
        .get("dumps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        dumps.extend(dump_files(&base.join(path))?);
    }
    for path in &options.dumps {
        dumps.extend(dump_files(path)?);
    }
    if dumps.is_empty() {
        return Err(HarnessError::Usage(format!("no dump given\n{USAGE}")));
    }
    let mut occurrences = Vec::new();
    for dump in &dumps {
        occurrences.extend(read_dump(dump)?);
    }
    let outcome = classify(&classes, occurrences);

    let mut tiles = Map::new();
    let mut patterns = Vec::new();
    let mut class_report = Vec::new();
    for (class, pictures) in classes.iter().zip(&outcome.classes) {
        let extra = class
            .pattern
            .get("tags")
            .and_then(Value::as_str)
            .unwrap_or("");
        let tags = if extra.is_empty() {
            class.name.clone()
        } else {
            format!("{} {extra}", class.name)
        };
        let mut names = Vec::new();
        for (index, picture) in pictures.iter().enumerate() {
            let name = format!("{}_{index}", class.name);
            tiles.insert(name.clone(), picture.planes());
            let mut pattern = class.pattern.clone();
            pattern.insert("grid".to_owned(), json!([[name]]));
            pattern.insert("tags".to_owned(), json!(tags));
            patterns.push(Value::Object(pattern));
            names.push(json!({ "tile": name, "key": picture.key() }));
        }
        class_report.push(json!({ "class": class.name, "count": pictures.len(), "tiles": names }));
    }

    // the same picture matched by other classes, or by none, somewhere
    let mut shared = Vec::new();
    for (index, pictures) in outcome.classes.iter().enumerate() {
        for picture in pictures {
            let mut others: BTreeMap<String, usize> = BTreeMap::new();
            for (_, class) in &outcome.occurrences[picture] {
                if *class != Some(index) {
                    let name =
                        class.map_or_else(|| "(none)".to_owned(), |c| classes[c].name.clone());
                    *others.entry(name).or_default() += 1;
                }
            }
            if !others.is_empty() {
                shared.push(json!({ "key": picture.key(), "class": classes[index].name,
                    "also": others }));
            }
        }
    }
    let unclaimed: Vec<Value> = outcome
        .unclaimed
        .iter()
        .map(|picture| {
            let list = &outcome.occurrences[picture];
            let first = &list[0].0;
            json!({ "key": picture.key(), "planes": picture.planes(), "count": list.len(),
                "palette": first.palette, "sprite": first.sprite,
                "example": { "frame": first.frame, "x": first.x, "y": first.y } })
        })
        .collect();

    let mut summary = String::new();
    let _ = writeln!(
        summary,
        "classify: {} dumps, {} pictures",
        dumps.len(),
        outcome.occurrences.len()
    );
    let mut sheet_rows = Vec::new();
    if let Some(path) = &options.sheet {
        let (width, height, pixels, placed) = sheet(&outcome);
        let png = zax_render::assets::png_image::encode_rgb(
            u32::try_from(width).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
            &pixels,
        )?;
        std::fs::write(path, png)?;
        sheet_rows = placed;
    }
    for (index, (class, pictures)) in classes.iter().zip(&outcome.classes).enumerate() {
        let _ = write!(
            summary,
            "  {:<16} {:>5} pictures",
            class.name,
            pictures.len()
        );
        if let Some((first, last)) = sheet_rows.get(index) {
            let _ = write!(summary, "   sheet rows {first}-{last}");
        }
        summary.push('\n');
    }
    let _ = write!(
        summary,
        "  {:<16} {:>5} pictures",
        "(unclaimed)",
        unclaimed.len()
    );
    if let Some((first, last)) = sheet_rows.last() {
        let _ = write!(summary, "   sheet rows {first}-{last}");
    }
    let _ = writeln!(
        summary,
        "\n  shared: {} pictures also seen outside their class",
        shared.len()
    );

    let description = json!({ "tiles": tiles, "patterns": patterns });
    std::fs::write(
        &options.out,
        serde_json::to_string_pretty(&description).unwrap_or_default(),
    )?;
    let report_path = options
        .report
        .clone()
        .unwrap_or_else(|| options.out.with_extension("report.json"));
    let report = json!({ "dumps": dumps.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
        "classes": class_report, "shared": shared, "unclaimed": unclaimed,
        "sheet": { "scale": SCALE, "per_row": PER_ROW, "cell": CELL,
                   "rows": sheet_rows.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>() } });
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    )?;
    let _ = write!(
        summary,
        "  -> {} and {}",
        options.out.display(),
        report_path.display()
    );
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn occurrence(low: u64, palette: u8, y: i64) -> Occurrence {
        Occurrence {
            picture: Picture {
                low,
                high: 0,
                mirror: "",
            },
            frame: "d1.json".to_owned(),
            x: 0,
            y,
            palette,
            sprite: false,
            colours: [15, 1, 2, 3],
        }
    }

    fn class(name: &str, filter: Filter) -> Class {
        Class {
            name: name.to_owned(),
            filter,
            pattern: Map::new(),
        }
    }

    #[test]
    fn a_picture_goes_to_its_first_class_and_its_other_places_are_shared() {
        let classes = [
            class(
                "hud",
                Filter {
                    y: Some((0, 15)),
                    ..Filter::default()
                },
            ),
            class(
                "floor",
                Filter {
                    palette: Some(vec![0]),
                    ..Filter::default()
                },
            ),
        ];
        let outcome = classify(
            &classes,
            vec![
                occurrence(1, 0, 200), // floor
                occurrence(1, 0, 8),   // the same picture in the hud: hud wins
                occurrence(2, 1, 200), // nobody
            ],
        );
        assert_eq!(outcome.classes[0].len(), 1);
        assert_eq!(outcome.classes[1].len(), 0);
        assert_eq!(outcome.unclaimed.len(), 1);
        let picture = outcome.classes[0][0];
        assert_eq!(
            outcome.occurrences[&picture]
                .iter()
                .filter(|(_, c)| *c == Some(1))
                .count(),
            1
        );
    }

    #[test]
    fn colours_are_those_the_tile_shows() {
        // low plane 0xFF: the first row in colour 1, NES colour 1 for that occurrence
        let filter = Filter {
            colours: Some(vec![1]),
            ..Filter::default()
        };
        assert!(filter.matches(&occurrence(0xFF, 0, 0)));
        let other = Filter {
            colours: Some(vec![2]),
            ..Filter::default()
        };
        assert!(!other.matches(&occurrence(0xFF, 0, 0)));
    }
}
