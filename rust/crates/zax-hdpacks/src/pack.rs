//! Reading a Mesen pack's `hires.txt`: its pictures, tile rules, conditions, backgrounds and
//! options (format version 100 to 106 as documented by Mesen).

use std::collections::HashMap;

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

/// Where the HD picture of a tile is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdTile {
    pub image: usize,
    pub x: u32,
    pub y: u32,
    pub brightness: f32,
}

/// How two values compare in a memory condition.
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
    pub const fn holds(self, left: u8, right: u8) -> bool {
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

/// A named condition of `<condition>` lines, or a built-in flag.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// The sprite is mirrored horizontally.
    HMirror,
    /// The sprite is mirrored vertically.
    VMirror,
    /// The sprite is behind the background.
    BgPriority,
    /// The background tile covering screen pixel (`x`, `y`) is `key`.
    TileAtPosition { x: i32, y: i32, key: TileKey },
    /// A sprite covering screen pixel (`x`, `y`) is `key`.
    SpriteAtPosition { x: i32, y: i32, key: TileKey },
    /// The background tile at offset (`x`, `y`) from the tile is `key`.
    TileNearby { x: i32, y: i32, key: TileKey },
    /// A sprite at offset (`x`, `y`) from the tile is `key`.
    SpriteNearby { x: i32, y: i32, key: TileKey },
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
    /// A condition this reader cannot evaluate (PPU memory): never true.
    Unsupported,
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

/// One `<background>` line.
#[derive(Debug, Clone, PartialEq)]
pub struct Background {
    pub image: usize,
    pub brightness: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    /// Sprites behind the background are drawn in front of this picture.
    pub behind_sprites_in_front: bool,
    pub left: i32,
    pub top: i32,
    pub terms: Vec<Term>,
}

/// The `<options>` of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackOptions {
    pub disable_sprite_limit: bool,
    pub disable_original_tiles: bool,
    pub disable_contours: bool,
}

/// A pack ready to draw.
#[derive(Debug, Clone, Default)]
pub struct HdPack {
    pub version: u32,
    /// One NES pixel is `scale` x `scale` pixels.
    pub scale: u32,
    pub images: Vec<Rgba>,
    pub conditions: Vec<Condition>,
    /// The rules of a tile and palette, in file order (the first that holds wins).
    pub rules: HashMap<TileKey, Vec<Rule>>,
    /// The rules of a tile whatever its palette (`default tile` = `Y`).
    pub defaults: HashMap<TileData, Vec<Rule>>,
    pub backgrounds: Vec<Background>,
    pub options: PackOptions,
    /// Lines read but not drawn (sounds, patches, unknown tags).
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
    if text.len() == 32 {
        let bytes = hex_bytes(text)?;
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
/// When `hires.txt` or a picture it names is missing or unreadable, or a tile line is malformed.
pub fn load(files: &dyn PackFiles) -> Result<HdPack> {
    let bytes = files
        .read("hires.txt")
        .ok_or_else(|| HdPackError::Missing("hires.txt".into()))?;
    let text = String::from_utf8_lossy(&bytes);
    parse(&text, |name| {
        let bytes = files
            .read(name)
            .ok_or_else(|| HdPackError::Missing(name.to_owned()))?;
        Rgba::from_png(&bytes)
    })
}

/// Reads `text`, `picture` giving the picture of an `<img>` name.
///
/// # Errors
///
/// As [`load`].
#[allow(clippy::too_many_lines)] // one arm a tag
pub fn parse(text: &str, mut picture: impl FnMut(&str) -> Result<Rgba>) -> Result<HdPack> {
    let mut pack = HdPack {
        scale: 1,
        version: 100,
        ..HdPack::default()
    };
    let mut names: HashMap<String, usize> = HashMap::new();
    for (name, condition) in [
        ("hmirror", Condition::HMirror),
        ("vmirror", Condition::VMirror),
        ("bgpriority", Condition::BgPriority),
    ] {
        names.insert(name.to_owned(), pack.conditions.len());
        pack.conditions.push(condition);
    }
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = || HdPackError::Line {
            number: number + 1,
            text: line.to_owned(),
        };
        // `[a&!b]` before a tag.
        let (terms, line) = match line.strip_prefix('[') {
            Some(rest) => {
                let (inside, after) = rest.split_once(']').ok_or_else(bad)?;
                let terms = inside
                    .split('&')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(|name| {
                        let (negated, name) = name
                            .strip_prefix('!')
                            .map_or((false, name), |rest| (true, rest.trim()));
                        // A condition used before it is defined, or never: never true.
                        let condition = *names.entry(name.to_owned()).or_insert_with(|| {
                            pack.conditions.push(Condition::Unsupported);
                            pack.conditions.len() - 1
                        });
                        Term { condition, negated }
                    })
                    .collect();
                (terms, after.trim())
            }
            None => (Vec::new(), line),
        };
        let Some((tag, value)) = line.strip_prefix('<').and_then(|rest| rest.split_once('>'))
        else {
            pack.skipped += 1;
            continue;
        };
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        match tag.to_ascii_lowercase().as_str() {
            "ver" => pack.version = value.trim().parse().map_err(|_| bad())?,
            "scale" => pack.scale = value.trim().parse::<u32>().map_err(|_| bad())?.max(1),
            "img" => pack.images.push(picture(value.trim())?),
            "tile" => {
                if fields.len() < 5 {
                    return Err(bad());
                }
                let image: usize = fields[0].parse().map_err(|_| bad())?;
                if image >= pack.images.len() {
                    return Err(bad());
                }
                let data = tile_data(fields[1], pack.version >= 103).ok_or_else(bad)?;
                pack.uses_indices |= matches!(data, TileData::Index(_));
                let palette = palette(fields[2]).ok_or_else(bad)?;
                let tile = HdTile {
                    image,
                    x: fields[3].parse().map_err(|_| bad())?,
                    y: fields[4].parse().map_err(|_| bad())?,
                    brightness: fields.get(5).and_then(|b| b.parse().ok()).unwrap_or(1.0),
                };
                let rule = Rule { terms, tile };
                if parse_flag(fields.get(6)) {
                    pack.defaults.entry(data).or_default().push(rule.clone());
                }
                pack.rules
                    .entry(TileKey { data, palette })
                    .or_default()
                    .push(rule);
            }
            "condition" => {
                if fields.len() < 2 {
                    return Err(bad());
                }
                let condition = condition(&fields, pack.version).ok_or_else(bad)?;
                let name = fields[0].to_owned();
                match names.get(&name) {
                    // Named before it was defined: its slot takes the definition.
                    Some(&at) if pack.conditions[at] == Condition::Unsupported => {
                        pack.conditions[at] = condition;
                    }
                    _ => {
                        names.insert(name, pack.conditions.len());
                        pack.conditions.push(condition);
                    }
                }
            }
            "background" => {
                let image = fields
                    .first()
                    .map(|name| picture(name))
                    .transpose()?
                    .ok_or_else(bad)?;
                pack.images.push(image);
                let float = |at: usize, default: f32| {
                    fields
                        .get(at)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(default)
                };
                let int = |at: usize| fields.get(at).and_then(|v| v.parse().ok()).unwrap_or(0);
                pack.backgrounds.push(Background {
                    image: pack.images.len() - 1,
                    brightness: float(1, 1.0),
                    scroll_x: float(2, 0.0),
                    scroll_y: float(3, 0.0),
                    behind_sprites_in_front: parse_flag(fields.get(4)),
                    left: int(5),
                    top: int(6),
                    terms,
                });
            }
            "options" => {
                for option in fields {
                    match option.to_ascii_lowercase().as_str() {
                        "disablespritelimit" => pack.options.disable_sprite_limit = true,
                        "disableoriginaltiles" => pack.options.disable_original_tiles = true,
                        "disablecontours" => pack.options.disable_contours = true,
                        _ => pack.skipped += 1,
                    }
                }
            }
            "overscan" => {}
            _ => pack.skipped += 1,
        }
    }
    Ok(pack)
}

/// The condition of a `<condition>` line's fields (its name first).
fn condition(fields: &[&str], version: u32) -> Option<Condition> {
    let kind = fields.get(1)?.to_ascii_lowercase();
    let int = |at: usize| fields.get(at)?.parse::<i32>().ok();
    let hex_or_dec = |at: usize, hex: bool| -> Option<u32> {
        let text = fields.get(at)?;
        if hex {
            u32::from_str_radix(text, 16).ok()
        } else {
            text.parse().ok()
        }
    };
    let positional = || -> Option<(i32, i32, TileKey)> {
        Some((
            int(2)?,
            int(3)?,
            TileKey {
                data: tile_data(fields.get(4)?, version >= 104)?,
                palette: palette(fields.get(5)?)?,
            },
        ))
    };
    let hex_memory = version >= 102;
    #[allow(clippy::cast_possible_truncation)] // addresses and bytes, masked to their width
    let memory = |at: usize| hex_or_dec(at, hex_memory).map(|v| v as u16);
    #[allow(clippy::cast_possible_truncation)]
    let byte = |at: usize, default: u8| {
        if fields.get(at).is_none() {
            Some(default)
        } else {
            hex_or_dec(at, hex_memory).map(|v| v as u8)
        }
    };
    Some(match kind.as_str() {
        "tileatposition" => {
            let (x, y, key) = positional()?;
            Condition::TileAtPosition { x, y, key }
        }
        "spriteatposition" => {
            let (x, y, key) = positional()?;
            Condition::SpriteAtPosition { x, y, key }
        }
        "tilenearby" => {
            let (x, y, key) = positional()?;
            Condition::TileNearby { x, y, key }
        }
        "spritenearby" => {
            let (x, y, key) = positional()?;
            Condition::SpriteNearby { x, y, key }
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
        "framerange" => Condition::FrameRange {
            divisor: hex_or_dec(2, false)?.max(1),
            compare: hex_or_dec(3, false)?,
        },
        _ => Condition::Unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<HdPack> {
        parse(text, |_| Ok(Rgba::new(64, 64, [255, 0, 0, 255])))
    }

    #[test]
    fn tiles_conditions_and_defaults_are_read() {
        let pack = read(
            "<ver>105\n<scale>4\n<img>a.png\n\
             <condition>low,memoryCheckConstant,00FF,==,1F,3F\n\
             [low&!hmirror]<tile>0,2E,FF16360F,0,0,1,N\n\
             <tile>0,2E,FF16360F,32,0\n\
             <tile>0,0E0E079C1E3EA7076101586121010000,0F100017,8,8,1.5,Y\n\
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
        assert_eq!(1, pack.skipped);
        assert!(pack.uses_indices);
    }

    #[test]
    fn old_versions_read_decimal_indices_and_a_bad_tile_is_an_error() {
        let Ok(pack) = read("<ver>100\n<img>a.png\n<tile>0,46,FF16360F,0,0\n") else {
            panic!("not read");
        };
        assert!(pack.rules.keys().any(|k| k.data == TileData::Index(46)));
        assert!(read("<img>a.png\n<tile>3,46,FF16360F,0,0\n").is_err());
    }
}
