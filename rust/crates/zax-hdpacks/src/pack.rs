//! Reading a Mesen pack's `hires.txt` (format versions 100 to 109, as Mesen2's loader reads
//! them): its pictures, tile rules, conditions, backgrounds, additions and options.
//!
//! Pictures are kept as their PNG bytes and decoded the first time they are drawn: a pack can
//! name hundreds of large pictures, most of them for one stage of the game.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::files::PackFiles;
use crate::image::Rgba;
use crate::{HdPackError, Result};

/// What identifies an original tile in a `<tile>` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileData {
    /// A CHR-RAM tile: its 16 bytes (8 of the low plane, then 8 of the high plane).
    Bytes([u8; 16]),
    /// A CHR-ROM tile: its index (address / 16) in the CHR ROM.
    Index(u32),
}

/// A tile and the 4 colours it is drawn with (the first `FF` for a sprite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub data: TileData,
    pub palette: [u8; 4],
}

/// A picture of the pack, decoded when first drawn.
#[derive(Debug, Default)]
pub struct Picture {
    bytes: Vec<u8>,
    decoded: OnceLock<Rgba>,
}

impl Picture {
    /// A picture of PNG `bytes`.
    #[must_use]
    pub fn from_png(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            decoded: OnceLock::new(),
        }
    }

    /// A picture already decoded.
    #[must_use]
    pub fn ready(picture: Rgba) -> Self {
        let decoded = OnceLock::new();
        let _ = decoded.set(picture);
        Self {
            bytes: Vec::new(),
            decoded,
        }
    }

    /// The picture; an empty one when its PNG cannot be decoded.
    pub fn get(&self) -> &Rgba {
        self.decoded
            .get_or_init(|| Rgba::from_png(&self.bytes).unwrap_or_default())
    }

    /// Whether it was decoded already.
    #[must_use]
    pub fn is_decoded(&self) -> bool {
        self.decoded.get().is_some()
    }
}

/// Where the HD picture of a tile is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdTile {
    /// The picture, in [`HdPack::pictures`].
    pub image: usize,
    pub x: u32,
    pub y: u32,
    pub brightness: f32,
}

/// How two values compare in a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
}

impl Operator {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "==" => Self::Equal,
            "!=" => Self::NotEqual,
            ">" => Self::Greater,
            "<" => Self::Less,
            ">=" => Self::GreaterOrEqual,
            "<=" => Self::LessOrEqual,
            _ => return None,
        })
    }

    /// `left` compared with `right`.
    #[must_use]
    pub const fn holds(self, left: i32, right: i32) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Greater => left > right,
            Self::Less => left < right,
            Self::GreaterOrEqual => left >= right,
            Self::LessOrEqual => left <= right,
        }
    }
}

/// A tile a positional condition looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wanted {
    pub key: TileKey,
    /// Any palette (format 108).
    pub ignore_palette: bool,
}

/// A named condition of `<condition>` lines, or a built-in flag.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// The sprite is mirrored horizontally.
    HMirror,
    /// The sprite is mirrored vertically.
    VMirror,
    /// The sprite is behind the background.
    BgPriority,
    /// The sprite uses sprite palette `0` to 3 (`sppalette0` to `sppalette3`, format 107).
    SpritePalette(u8),
    /// The background tile covering screen pixel (`x`, `y`) is the one wanted.
    TileAtPosition { x: i32, y: i32, wanted: Wanted },
    /// A sprite covering screen pixel (`x`, `y`) is the one wanted.
    SpriteAtPosition { x: i32, y: i32, wanted: Wanted },
    /// The background tile at offset (`x`, `y`) from the pixel is the one wanted.
    TileNearby { x: i32, y: i32, wanted: Wanted },
    /// A sprite at offset (`x`, `y`) from the pixel (mirrored with the sprite) is the one wanted.
    SpriteNearby { x: i32, y: i32, wanted: Wanted },
    /// CPU memory: `[left] & mask` against `[right] & mask`.
    MemoryCheck {
        left: u16,
        operator: Operator,
        right: u16,
        mask: u8,
    },
    /// CPU memory: `[address] & mask` against `value`.
    MemoryConstant {
        address: u16,
        operator: Operator,
        value: u8,
        mask: u8,
    },
    /// `frame % divisor >= compare`.
    FrameRange { divisor: u32, compare: u32 },
    /// The pixel's screen x (`vertical`: y) against `value` (format 108).
    Position {
        vertical: bool,
        operator: Operator,
        value: i32,
    },
    /// The column (`vertical`: row) of the pixel inside its tile, mirrored with it, against
    /// `value` (format 108).
    OriginPosition {
        vertical: bool,
        operator: Operator,
        value: i32,
    },
    /// A condition this reader cannot evaluate (PPU memory): never true.
    Unsupported,
}

impl Condition {
    /// Whether the condition holds for the whole frame (Mesen caches these per frame), so a
    /// background may use it.
    #[must_use]
    pub const fn is_global(&self) -> bool {
        matches!(
            self,
            Self::TileAtPosition { .. }
                | Self::SpriteAtPosition { .. }
                | Self::MemoryCheck { .. }
                | Self::MemoryConstant { .. }
                | Self::FrameRange { .. }
                | Self::Unsupported
        )
    }
}

/// A condition as a rule uses it, possibly negated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Term {
    pub condition: usize,
    pub negated: bool,
}

/// One `<tile>` line: the HD tile, used when all its terms hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub terms: Vec<Term>,
    pub tile: HdTile,
}

/// How a background is laid over what is under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blend {
    #[default]
    Alpha,
    Add,
    Subtract,
}

/// One `<background>` line.
#[derive(Debug, Clone, PartialEq)]
pub struct Background {
    /// The picture, in [`HdPack::pictures`].
    pub image: usize,
    pub brightness: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    /// 0 to 39: 0-9 behind the background-priority sprites, 10-19 behind the background tiles,
    /// 20-29 behind the other sprites, 30-39 in front of everything.
    pub priority: u8,
    /// The NES pixel of the picture shown at the screen's left column and top row.
    pub left: i32,
    pub top: i32,
    pub blend: Blend,
    pub terms: Vec<Term>,
}

/// An `<addition>` (format 107): wherever `original` shows, `added` is drawn too, `x`, `y` NES
/// pixels away (mirrored with it), as one more sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addition {
    pub original: TileKey,
    pub x: i32,
    pub y: i32,
    pub added: TileKey,
    pub ignore_palette: bool,
}

/// The `<options>` of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackOptions {
    pub disable_sprite_limit: bool,
    pub disable_original_tiles: bool,
}

/// A pack ready to draw.
#[derive(Debug, Default)]
pub struct HdPack {
    pub version: u32,
    /// One NES pixel is `scale` x `scale` pixels.
    pub scale: u32,
    /// Every picture, each read once whatever the number of lines naming it.
    pub pictures: Vec<Picture>,
    pub conditions: Vec<Condition>,
    /// The rules of a tile and palette, in file order (the first that holds wins).
    pub rules: HashMap<TileKey, Vec<Rule>>,
    /// The rules of a tile whatever its palette (`default tile` = `Y`).
    pub defaults: HashMap<TileData, Vec<Rule>>,
    pub backgrounds: Vec<Background>,
    pub additions: Vec<Addition>,
    pub options: PackOptions,
    /// Lines read but not drawn (sounds, patches, unknown tags), and terms dropped.
    pub skipped: usize,
    /// True when a `<tile>` names CHR-ROM indices.
    pub uses_indices: bool,
}

fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

/// The tile data of a field: 32 hex digits are CHR-RAM bytes, else a CHR-ROM index (hex from
/// `hex_index`, decimal before).
fn tile_data(text: &str, hex_index: bool) -> Option<TileData> {
    if text.len() >= 32 {
        let bytes = hex_bytes(text.get(..32)?)?;
        let mut chr = [0u8; 16];
        chr.copy_from_slice(&bytes);
        Some(TileData::Bytes(chr))
    } else if hex_index {
        u32::from_str_radix(text, 16).ok().map(TileData::Index)
    } else {
        text.parse().ok().map(TileData::Index)
    }
}

fn palette(text: &str) -> Option<[u8; 4]> {
    hex_bytes(text)?.try_into().ok()
}

fn parse_flag(text: Option<&&str>) -> bool {
    text.is_some_and(|value| value.eq_ignore_ascii_case("Y") || value.eq_ignore_ascii_case("true"))
}

/// Reads a pack's text and pictures from `files`.
///
/// # Errors
///
/// When `hires.txt` or a picture it names is missing, or a line is malformed.
pub fn load(files: &dyn PackFiles) -> Result<HdPack> {
    let bytes = files
        .read("hires.txt")
        .ok_or_else(|| HdPackError::Missing("hires.txt".into()))?;
    let text = String::from_utf8_lossy(&bytes);
    parse(&text, |name| {
        files
            .read(name)
            .map(Picture::from_png)
            .ok_or_else(|| HdPackError::Missing(name.to_owned()))
    })
}

/// A term as written, its condition found once the file is read.
struct Named {
    name: String,
    negated: bool,
}

/// What [`parse`] keeps while it reads.
struct Reader<F> {
    pack: HdPack,
    picture: F,
    /// The pictures by name.
    by_name: HashMap<String, usize>,
    /// The `<img>` pictures, in order: a `<tile>` numbers them.
    tile_images: Vec<usize>,
    conditions: HashMap<String, usize>,
    /// The terms of each tile rule and background, by where they are, to resolve at the end.
    pending: Vec<(Owner, Vec<Named>)>,
}

/// What a pending list of terms belongs to.
#[derive(Clone, Copy)]
enum Owner {
    Rule {
        key: TileKey,
        at: usize,
        default: Option<usize>,
    },
    Background(usize),
}

impl<F: FnMut(&str) -> Result<Picture>> Reader<F> {
    fn picture(&mut self, name: &str) -> Result<usize> {
        let key = name.trim().replace('\\', "/").to_lowercase();
        if let Some(&at) = self.by_name.get(&key) {
            return Ok(at);
        }
        let picture = (self.picture)(name.trim())?;
        self.pack.pictures.push(picture);
        let at = self.pack.pictures.len() - 1;
        self.by_name.insert(key, at);
        Ok(at)
    }

    /// The terms of `owner`, each name found (a name never defined is dropped, as Mesen does).
    fn resolve(&mut self) {
        for (owner, names) in std::mem::take(&mut self.pending) {
            let mut terms = Vec::with_capacity(names.len());
            for named in names {
                let Some(&condition) = self.conditions.get(&named.name) else {
                    self.pack.skipped += 1;
                    continue;
                };
                // Mesen2 refuses on a background what is not true of the whole frame.
                if matches!(owner, Owner::Background(_))
                    && !self.pack.conditions[condition].is_global()
                {
                    self.pack.skipped += 1;
                    continue;
                }
                terms.push(Term {
                    condition,
                    negated: named.negated,
                });
            }
            match owner {
                Owner::Rule { key, at, default } => {
                    if let Some(rule) = self.pack.rules.get_mut(&key).and_then(|r| r.get_mut(at)) {
                        rule.terms.clone_from(&terms);
                    }
                    if let Some(at) = default {
                        if let Some(rule) = self
                            .pack
                            .defaults
                            .get_mut(&key.data)
                            .and_then(|r| r.get_mut(at))
                        {
                            rule.terms = terms;
                        }
                    }
                }
                Owner::Background(at) => self.pack.backgrounds[at].terms = terms,
            }
        }
    }
}

/// Reads `text`, `picture` giving the picture of a file name.
///
/// # Errors
///
/// As [`load`].
#[allow(clippy::too_many_lines)] // one arm a tag
pub fn parse(text: &str, picture: impl FnMut(&str) -> Result<Picture>) -> Result<HdPack> {
    let mut reader = Reader {
        pack: HdPack {
            scale: 1,
            version: 100,
            ..HdPack::default()
        },
        picture,
        by_name: HashMap::new(),
        tile_images: Vec::new(),
        conditions: HashMap::new(),
        pending: Vec::new(),
    };
    let mut builtins = vec![
        ("hmirror".to_owned(), Condition::HMirror),
        ("vmirror".to_owned(), Condition::VMirror),
        ("bgpriority".to_owned(), Condition::BgPriority),
    ];
    for palette in 0..4u8 {
        builtins.push((
            format!("sppalette{palette}"),
            Condition::SpritePalette(palette),
        ));
    }
    for (name, condition) in builtins {
        reader.conditions.insert(name, reader.pack.conditions.len());
        reader.pack.conditions.push(condition);
    }
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = || HdPackError::Line {
            number: number + 1,
            text: line.chars().take(120).collect(),
        };
        // `[a&!b]` before a tag.
        let (names, line) = match line.strip_prefix('[') {
            Some(rest) => {
                let (inside, after) = rest.split_once(']').ok_or_else(bad)?;
                let names: Vec<Named> = inside
                    .split('&')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(|name| match name.strip_prefix('!') {
                        Some(rest) => Named {
                            name: rest.trim().to_owned(),
                            negated: true,
                        },
                        None => Named {
                            name: name.to_owned(),
                            negated: false,
                        },
                    })
                    .collect();
                (names, after.trim())
            }
            None => (Vec::new(), line),
        };
        let Some((tag, value)) = line.strip_prefix('<').and_then(|rest| rest.split_once('>'))
        else {
            reader.pack.skipped += 1;
            continue;
        };
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        let version = reader.pack.version;
        match tag.to_ascii_lowercase().as_str() {
            "ver" => reader.pack.version = value.trim().parse().map_err(|_| bad())?,
            "scale" => {
                reader.pack.scale = value.trim().parse::<u32>().map_err(|_| bad())?.clamp(1, 10);
            }
            "img" => {
                let at = reader.picture(value)?;
                reader.tile_images.push(at);
            }
            "tile" => {
                if fields.len() < 5 {
                    return Err(bad());
                }
                let index: usize = fields[0].parse().map_err(|_| bad())?;
                let image = *reader.tile_images.get(index).ok_or_else(bad)?;
                let data = tile_data(fields[1], version >= 103).ok_or_else(bad)?;
                reader.pack.uses_indices |= matches!(data, TileData::Index(_));
                let palette = palette(fields[2]).ok_or_else(bad)?;
                let tile = HdTile {
                    image,
                    x: fields[3].parse().map_err(|_| bad())?,
                    y: fields[4].parse().map_err(|_| bad())?,
                    brightness: fields.get(5).and_then(|b| b.parse().ok()).unwrap_or(1.0),
                };
                let rule = Rule {
                    terms: Vec::new(),
                    tile,
                };
                let key = TileKey { data, palette };
                let default = parse_flag(fields.get(6)).then(|| {
                    let list = reader.pack.defaults.entry(data).or_default();
                    list.push(rule.clone());
                    list.len() - 1
                });
                let list = reader.pack.rules.entry(key).or_default();
                list.push(rule);
                let at = list.len() - 1;
                if !names.is_empty() {
                    reader
                        .pending
                        .push((Owner::Rule { key, at, default }, names));
                }
            }
            "condition" => {
                if fields.len() < 2 {
                    return Err(bad());
                }
                let condition = condition(&fields, version).ok_or_else(bad)?;
                reader
                    .conditions
                    .insert(fields[0].to_owned(), reader.pack.conditions.len());
                reader.pack.conditions.push(condition);
            }
            "background" => {
                let name = fields.first().ok_or_else(bad)?;
                let image = reader.picture(name)?;
                let float = |at: usize, default: f32| {
                    fields
                        .get(at)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(default)
                };
                let int = |at: usize| {
                    fields
                        .get(at)
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(0)
                        .max(0)
                };
                let priority = match fields.get(4) {
                    None => 10,
                    // An integer from 106; a Y/N before, and read so whatever the version.
                    Some(value) => match value.parse::<u8>() {
                        Ok(level) if version >= 106 => level.min(39),
                        _ if parse_flag(fields.get(4)) => 0,
                        _ => 10,
                    },
                };
                let blend = match fields.get(7).map(|b| b.to_ascii_lowercase()).as_deref() {
                    Some("add") => Blend::Add,
                    Some("subtract") => Blend::Subtract,
                    _ => Blend::Alpha,
                };
                reader.pack.backgrounds.push(Background {
                    image,
                    brightness: float(1, 1.0),
                    scroll_x: float(2, 0.0),
                    scroll_y: float(3, 0.0),
                    priority,
                    left: int(5),
                    top: int(6),
                    blend,
                    terms: Vec::new(),
                });
                if !names.is_empty() {
                    let at = reader.pack.backgrounds.len() - 1;
                    reader.pending.push((Owner::Background(at), names));
                }
            }
            "addition" => {
                if fields.len() < 6 {
                    return Err(bad());
                }
                let key = |data: &str, colours: &str| -> Option<TileKey> {
                    Some(TileKey {
                        data: tile_data(data, true)?,
                        palette: palette(colours)?,
                    })
                };
                reader.pack.additions.push(Addition {
                    original: key(fields[0], fields[1]).ok_or_else(bad)?,
                    x: fields[2].parse().map_err(|_| bad())?,
                    y: fields[3].parse().map_err(|_| bad())?,
                    added: key(fields[4], fields[5]).ok_or_else(bad)?,
                    ignore_palette: parse_flag(fields.get(6)),
                });
            }
            "options" => {
                for option in fields {
                    match option.to_ascii_lowercase().as_str() {
                        "disablespritelimit" => reader.pack.options.disable_sprite_limit = true,
                        "disableoriginaltiles" => {
                            reader.pack.options.disable_original_tiles = true;
                        }
                        "disablecontours" | "disablecache" => {}
                        _ => reader.pack.skipped += 1,
                    }
                }
            }
            "overscan" => {}
            _ => reader.pack.skipped += 1,
        }
    }
    reader.resolve();
    Ok(reader.pack)
}

/// The condition of a `<condition>` line's fields (its name first).
fn condition(fields: &[&str], version: u32) -> Option<Condition> {
    let kind = fields.get(1)?.to_ascii_lowercase();
    let int = |at: usize| fields.get(at)?.parse::<i32>().ok();
    let hex = |at: usize| u32::from_str_radix(fields.get(at)?, 16).ok();
    let positional = || -> Option<(i32, i32, Wanted)> {
        Some((
            int(2)?,
            int(3)?,
            Wanted {
                key: TileKey {
                    data: tile_data(fields.get(4)?, version >= 104)?,
                    palette: palette(fields.get(5)?)?,
                },
                ignore_palette: parse_flag(fields.get(6)),
            },
        ))
    };
    #[allow(clippy::cast_possible_truncation)] // addresses and bytes, masked to their width
    let memory = |at: usize| {
        if version >= 102 {
            hex(at)
        } else {
            fields.get(at)?.parse().ok()
        }
        .map(|v| v as u16)
    };
    #[allow(clippy::cast_possible_truncation)]
    let byte = |at: usize, default: u8| {
        if fields.get(at).is_none() {
            Some(default)
        } else {
            hex(at).map(|v| v as u8)
        }
    };
    let compare =
        || -> Option<(Operator, i32)> { Some((Operator::parse(fields.get(2)?)?, int(3)?)) };
    Some(match kind.as_str() {
        "tileatposition" => {
            let (x, y, wanted) = positional()?;
            Condition::TileAtPosition { x, y, wanted }
        }
        "spriteatposition" => {
            let (x, y, wanted) = positional()?;
            Condition::SpriteAtPosition { x, y, wanted }
        }
        "tilenearby" => {
            let (x, y, wanted) = positional()?;
            Condition::TileNearby { x, y, wanted }
        }
        "spritenearby" => {
            let (x, y, wanted) = positional()?;
            Condition::SpriteNearby { x, y, wanted }
        }
        "memorycheck" => Condition::MemoryCheck {
            left: memory(2)?,
            operator: Operator::parse(fields.get(3)?)?,
            right: memory(4)?,
            mask: byte(5, 0xFF)?,
        },
        "memorycheckconstant" => Condition::MemoryConstant {
            address: memory(2)?,
            operator: Operator::parse(fields.get(3)?)?,
            value: byte(4, 0)?,
            mask: byte(5, 0xFF)?,
        },
        "framerange" => {
            let number = |at: usize| {
                if version <= 101 {
                    hex(at)
                } else {
                    fields.get(at)?.parse().ok()
                }
            };
            Condition::FrameRange {
                divisor: number(2)?.max(1),
                compare: number(3)?,
            }
        }
        "positioncheckx" | "positionchecky" => {
            let (operator, value) = compare()?;
            Condition::Position {
                vertical: kind.ends_with('y'),
                operator,
                value,
            }
        }
        "originpositioncheckx" | "originpositionchecky" => {
            let (operator, value) = compare()?;
            Condition::OriginPosition {
                vertical: kind.ends_with('y'),
                operator,
                value,
            }
        }
        _ => Condition::Unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<HdPack> {
        parse(text, |_| {
            Ok(Picture::ready(Rgba::new(64, 64, [255, 0, 0, 255])))
        })
    }

    #[test]
    fn tiles_conditions_and_defaults_are_read() {
        let pack = read(
            "<ver>105\n<scale>4\n<img>a.png\n\
             [low&!hmirror&nowhere]<tile>0,2E,FF16360F,0,0,1,N\n\
             <condition>low,memoryCheckConstant,00FF,==,1F,3F\n\
             <tile>0,2E,FF16360F,32,0\n\
             <tile>0,0e0e079c1e3ea7076101586121010000,0F100017,8,8,1.5,Y\n\
             <bgm>0,0,a.ogg\n",
        );
        assert!(pack.is_ok(), "{pack:?}");
        let pack = pack.unwrap_or_default();
        assert_eq!(4, pack.scale);
        let key = TileKey {
            data: TileData::Index(0x2E),
            palette: [0xFF, 0x16, 0x36, 0x0F],
        };
        let rules = &pack.rules[&key];
        assert_eq!(2, rules.len());
        // `nowhere` is never defined: dropped, as Mesen does.
        assert_eq!(2, rules[0].terms.len());
        assert!(rules[0].terms[1].negated);
        assert_eq!(
            Condition::MemoryConstant {
                address: 0xFF,
                operator: Operator::Equal,
                value: 0x1F,
                mask: 0x3F
            },
            pack.conditions[rules[0].terms[0].condition]
        );
        assert_eq!(1, pack.defaults.len());
        assert_eq!(2, pack.skipped);
        assert!(pack.uses_indices);
    }

    #[test]
    fn backgrounds_share_their_picture_and_keep_only_whole_frame_conditions() {
        let mut opened = 0;
        let pack = parse(
            "<ver>107\n<condition>far,tileNearby,8,0,2E,0F100017\n\
             <condition>on,frameRange,2,1\n\
             [on&far]<background>b.png,1,0,0,35,16,8\n\
             <background>B.png,1,0,0,N\n\
             <addition>2E,FF170700,-8,0,00000000000000000000000000000001,FF400000\n",
            |_| {
                opened += 1;
                Ok(Picture::ready(Rgba::new(8, 8, [0; 4])))
            },
        )
        .unwrap_or_default();
        assert_eq!(1, opened);
        assert_eq!(2, pack.backgrounds.len());
        assert_eq!(35, pack.backgrounds[0].priority);
        assert_eq!((16, 8), (pack.backgrounds[0].left, pack.backgrounds[0].top));
        assert_eq!(1, pack.backgrounds[0].terms.len());
        assert_eq!(1, pack.additions.len());
        assert_eq!(-8, pack.additions[0].x);
    }

    #[test]
    fn old_versions_read_decimal_indices_and_a_bad_tile_is_an_error() {
        let pack = read("<ver>100\n<img>a.png\n<tile>0,46,FF16360F,0,0\n").unwrap_or_default();
        assert!(pack.rules.keys().any(|k| k.data == TileData::Index(46)));
        assert!(read("<img>a.png\n<tile>3,46,FF16360F,0,0\n").is_err());
    }
}
