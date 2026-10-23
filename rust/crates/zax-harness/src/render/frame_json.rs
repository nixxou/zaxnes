//! The JSON frame description `render --frames <file.json>` reads and `--dump-frame` writes
//! (port of `FrameJson.cs` and of `RenderExport.FromJson`).
//!
//! It turns a ROM run into a self-contained scene: the same picture can be rendered again
//! without the ROM, which is what a reproduction case attached to a report has to be.

use std::fmt::Write as _;

use serde_json::Value;
use zax_nes::TileRecord;

use crate::HarnessError;

/// The 32 PPU palette bytes and the tile records of one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDescription {
    /// The tile records, in emission order.
    pub records: Vec<TileRecord>,
    /// The PPU palette.
    pub palette: [u8; 32],
}

/// Serialises `records` and the 32 palette bytes, byte for byte as `FrameJson.Write` does.
///
/// # Errors
///
/// [`HarnessError::Text`] if the text cannot be formatted (it cannot, writing to a `String`).
pub fn write(records: &[TileRecord], palette: &[u8; 32]) -> Result<String, HarnessError> {
    let mut text = String::from("{\n  \"palette\": [");
    for (entry, value) in palette.iter().enumerate() {
        if entry > 0 {
            text.push_str(", ");
        }
        write!(text, "{value}")?;
    }

    text.push_str("],\n  \"tiles\": [\n");
    for (n, record) in records.iter().enumerate() {
        write!(
            text,
            "    {{ \"low\": {}, \"high\": {}, \"x\": {}, \"y\": {}, \"palette\": {}, \
             \"mirrorH\": {}, \"mirrorV\": {}, \"hidden\": {}, \"sprite\": {} }}",
            record.plane0,
            record.plane1,
            record.x,
            record.y,
            record.palette,
            record.mirror_h,
            record.mirror_v,
            record.hidden,
            !record.background
        )?;
        text.push_str(if n + 1 < records.len() { ",\n" } else { "\n" });
    }

    text.push_str("  ]\n}\n");
    Ok(text)
}

/// Reads a frame description: up to 32 palette bytes (`palette`), an optional universal
/// background colour index (`background`) and the `tiles`. A sprite takes the next sprite order
/// and a background tile its position in the list, as the C# harness numbers them.
///
/// # Errors
///
/// [`HarnessError::Format`] when the text is not JSON, a field has the wrong type or is out of
/// range, or a tile misses `low`, `high`, `x` or `y`.
pub fn read(text: &str) -> Result<FrameDescription, HarnessError> {
    let root: Value = serde_json::from_str(text)
        .map_err(|error| HarnessError::Format(format!("frame description: {error}")))?;

    let mut palette = [0; 32];
    if let Some(entries) = root.get("palette") {
        for (slot, entry) in palette.iter_mut().zip(array(entries, "palette")?) {
            *slot = integer(entry, "palette")?;
        }
    }

    if let Some(background) = root.get("background") {
        palette[0] = integer(background, "background")?;
    }

    let mut records = Vec::new();
    let mut sprite_order = 0;
    if let Some(tiles) = root.get("tiles") {
        for tile in array(tiles, "tiles")? {
            let sprite = flag(tile, "sprite")?;
            let index = if sprite {
                sprite_order += 1;
                sprite_order - 1
            } else {
                i32::try_from(records.len())
                    .map_err(|_| HarnessError::Format("too many tile records".to_owned()))?
            };
            records.push(TileRecord::new(
                plane(tile, "low")?,
                plane(tile, "high")?,
                integer(required(tile, "x")?, "x")?,
                integer(required(tile, "y")?, "y")?,
                index,
                flag(tile, "mirrorH")?,
                flag(tile, "mirrorV")?,
                flag(tile, "hidden")?,
                tile.get("palette")
                    .map_or(Ok(0), |value| integer(value, "palette"))?,
                !sprite,
            ));
        }
    }

    Ok(FrameDescription { records, palette })
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], HarnessError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| HarnessError::Format(format!("frame description: '{key}' is not an array")))
}

fn required<'a>(tile: &'a Value, key: &str) -> Result<&'a Value, HarnessError> {
    tile.get(key)
        .ok_or_else(|| HarnessError::Format(format!("frame description: a tile has no '{key}'")))
}

fn plane(tile: &Value, key: &str) -> Result<u64, HarnessError> {
    required(tile, key)?.as_u64().ok_or_else(|| {
        HarnessError::Format(format!(
            "frame description: '{key}' is not an unsigned 64-bit integer"
        ))
    })
}

fn integer<T: TryFrom<i64>>(value: &Value, key: &str) -> Result<T, HarnessError> {
    value
        .as_i64()
        .and_then(|number| T::try_from(number).ok())
        .ok_or_else(|| {
            HarnessError::Format(format!("frame description: '{key}' is not a valid number"))
        })
}

/// A boolean that defaults to false when absent.
fn flag(tile: &Value, key: &str) -> Result<bool, HarnessError> {
    tile.get(key).map_or(Ok(false), |value| {
        value.as_bool().ok_or_else(|| {
            HarnessError::Format(format!("frame description: '{key}' is not a boolean"))
        })
    })
}
