//! `profile-dump` and `profile-build` (Rust-only): read a profile's patterns and their 3D looks,
//! and write a profile from a JSON description, for profiles made without the 3dSen editor (the
//! profiles of the homebrew games shipped with the program).
//!
//! The description of `profile-build`:
//!
//! ```json
//! { "title": "...", "author": "...", "publisher": "...", "description": "...",
//!   "template": "empty.3dn",                       // optional: settings taken from a profile
//!   "layers": [{ "index": 4, "offset": 0, "aligned": 1 }],  // optional: layer set overrides
//!   "tiles": { "brick": ["<low hex16>", "<high hex16>"], ... },
//!   "patterns": [
//!     { "grid": [["brick", "brick"], ["brick", null]],   // rows of tile names, null empty
//!       "depth": 8, "geometry": 0, "layer": 2,           // the main variant
//!       "solid": false, "permanent": true, "self_connected": true,
//!       "scale": [1,1,1], "offset": [0,0,0], "tags": "",
//!       "texture": "face.png",                          // optional: explicit texture
//!       "copies": [{ "offset": [0,0,6] }] } ],          // optional: more variants, all drawn
//!   "scripts": [{ "name": "main", "file": "main.lua" }] }  // optional: Lua, files beside it
//! ```
//!
//! A tile's planes are written as in a frame dump (`--dump-frame`): bit `y * 8 + (7 - x)` is
//! pixel (x, y).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use zax_profiles::enums::{
    LayerAlignment, PatternColorFlags, PatternDensity, ProfileLayer, TextureFormat, TileFlip,
    VariantGeometry, VariantRenderStyle,
};
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::{Mat4F, Vec2I, Vec3F};
use zax_profiles::variant::{PatternVariant, TexturePixels, VariantTexture};

use crate::HarnessError;

/// The usage of the two commands.
pub const USAGE: &str = "zax-harness profile-dump <file.3dn> [--tiles]
  Lists the profile's patterns: family, grid, colours and the 3D look of each variant
  (depth, geometry, layer, scale, offset, flags, tags); --tiles adds the tile planes.

zax-harness profile-build <description.json> <out.3dn>
  Writes a profile from a JSON description (see the source of profile_tool.rs).";

fn error(message: impl Into<String>) -> HarnessError {
    HarnessError::Usage(message.into())
}

fn read_profile(path: &Path) -> Result<GameProfile, HarnessError> {
    let bytes = std::fs::read(path).map_err(|e| error(format!("{}: {e}", path.display())))?;
    format::load(&bytes, None, None)
        .map(|document| document.profile)
        .map_err(|e| error(format!("{}: {}", path.display(), e.message)))
}

fn vec3(value: Option<Vec3F>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |v| format!("({:.2},{:.2},{:.2})", v.x, v.y, v.z),
    )
}

/// `profile-dump`: the patterns of a profile, one block each.
///
/// # Errors
/// The file cannot be read or is no profile.
pub fn dump(path: &Path, tiles: bool) -> Result<String, HarnessError> {
    let profile = read_profile(path)?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}: {} permanent, {} temporary patterns",
        profile.title.as_deref().unwrap_or("?"),
        profile.permanent_patterns.len(),
        profile.temporary_patterns.len()
    );
    if let Some(layers) = profile.settings.as_ref().and_then(|s| s.layers.as_ref()) {
        let _ = writeln!(
            out,
            "layers: default {:?}, auto {}",
            layers.default_layer, layers.auto
        );
        for (index, layer) in layers.layers.iter().enumerate() {
            let _ = writeln!(
                out,
                "  layer {index}: offset {:.2} aligned {:?}",
                layer.offset, layer.aligned
            );
        }
    }
    for (index, pattern) in profile.all_patterns().enumerate() {
        let (width, height) = pattern
            .grid
            .as_ref()
            .map_or((0, 0), |grid| (grid.width(), grid.height()));
        let filled = pattern
            .grid
            .as_ref()
            .map_or(0, PatternGrid::filled_cell_count);
        let _ = writeln!(
            out,
            "#{index} {} {width}x{height} ({filled} tiles) csize {}x{} colors {:?} px {} \
             tdens {:?} pdens {:?} self {} skip {}",
            if pattern.permanent { "perm" } else { "temp" },
            pattern.constraint_size.x,
            pattern.constraint_size.y,
            pattern.color_flags,
            pattern.pixel_count,
            pattern.tile_density,
            pattern.pixel_density,
            pattern.self_connected,
            pattern.skip_geometry
        );
        for variant in &pattern.variants {
            let _ = writeln!(
                out,
                "   depth {} geo {:?} layer {:?} solid {} flip {} warp {} anti {} style {:?} \
                 alpha {:.2} mesh {:?} empty {} scale {} rot {} off {} pivot {} tags {:?}",
                variant.depth,
                variant.geometry,
                variant.layer,
                variant.solid,
                variant.flip,
                variant.full_warp,
                variant.anti_shadow,
                variant.render_style,
                variant.alpha,
                variant.mesh_algorithm,
                variant.stored_empty_color,
                vec3(variant.scale),
                vec3(variant.rotation),
                vec3(variant.offset),
                vec3(variant.pivot),
                variant.tag_string.as_deref().unwrap_or("")
            );
            if let Some(amplitude) = variant.deform_amplitude {
                let matrix = |m: Option<Mat4F>| {
                    m.map_or_else(|| "-".to_owned(), |m| format!("{:?}", m.cells()))
                };
                let _ = writeln!(
                    out,
                    "   deform amp {} pivot {} speed {} offset {}",
                    vec3(Some(amplitude)),
                    vec3(variant.deform_pivot),
                    matrix(variant.deform_speed),
                    matrix(variant.deform_offset)
                );
            }
        }
        if tiles {
            if let Some(grid) = &pattern.grid {
                for (cell, tile) in grid.cells().iter().enumerate() {
                    if let Some(tile) = tile {
                        let _ = writeln!(
                            out,
                            "   cell {cell}: {:016x} {:016x} flip {:?}",
                            tile.low_plane, tile.high_plane, tile.flip
                        );
                    }
                }
            }
        }
    }
    Ok(out)
}

fn matrix16(value: Option<&Value>) -> Option<Mat4F> {
    let cells = value?.as_array()?;
    let mut out = [0.0f32; Mat4F::CELL_COUNT];
    if cells.len() != out.len() {
        return None;
    }
    for (cell, value) in out.iter_mut().zip(cells) {
        #[allow(clippy::cast_possible_truncation)] // profile floats are f32
        {
            *cell = value.as_f64()? as f32;
        }
    }
    Some(Mat4F::new(out))
}

fn hex64(value: &Value, what: &str) -> Result<u64, HarnessError> {
    let text = value
        .as_str()
        .ok_or_else(|| error(format!("{what}: a hexadecimal string is expected")))?;
    u64::from_str_radix(text.trim_start_matches("0x"), 16)
        .map_err(|_| error(format!("{what}: {text:?} is not hexadecimal")))
}

fn float3(value: Option<&Value>) -> Option<Vec3F> {
    let list = value?.as_array()?;
    #[allow(clippy::cast_possible_truncation)] // profile floats are f32
    let get = |index: usize| list.get(index).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    Some(Vec3F::new(get(0), get(1), get(2)))
}

fn int(value: Option<&Value>, default: i32) -> i32 {
    value
        .and_then(Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(default)
}

fn flag(value: Option<&Value>, default: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(default)
}

/// The derived fields a pattern stores: pixel count, colours and densities of its tiles.
fn describe(
    cells: &[Option<Arc<ProfileTile>>],
) -> (i32, PatternColorFlags, PatternDensity, PatternDensity) {
    let mut pixels = 0;
    let mut colors = 0;
    for tile in cells.iter().flatten() {
        for y in 0..8 {
            for x in 0..8 {
                match tile.color_at(x, y) {
                    Some(0) | None => {}
                    Some(color) => {
                        pixels += 1;
                        colors |= 1 << color;
                    }
                }
            }
        }
    }
    let total = i32::try_from(cells.len() * 64).unwrap_or(i32::MAX).max(1);
    let filled = i32::try_from(cells.iter().flatten().count()).unwrap_or(0);
    let cell_count = i32::try_from(cells.len()).unwrap_or(1).max(1);
    let density = |part: i32, whole: i32| match part * 100 / whole {
        0 => PatternDensity::Empty,
        1..=25 => PatternDensity::Low,
        26..=50 => PatternDensity::Average,
        51..=75 => PatternDensity::Medium,
        76..=99 => PatternDensity::High,
        _ => PatternDensity::Solid,
    };
    (
        pixels,
        PatternColorFlags::from_bits(colors),
        density(filled, cell_count),
        density(pixels, total),
    )
}

/// `profile-build`: writes the profile `description` describes to `out`.
///
/// # Errors
/// The description cannot be read or is malformed, or the profile cannot be written.
pub fn build(description: &Path, out: &Path) -> Result<String, HarnessError> {
    let text = std::fs::read_to_string(description)
        .map_err(|e| error(format!("{}: {e}", description.display())))?;
    let spec: Value = serde_json::from_str(&text)
        .map_err(|e| error(format!("{}: {e}", description.display())))?;
    let mut profile = match spec.get("template").and_then(Value::as_str) {
        Some(template) => {
            let path = description
                .parent()
                .unwrap_or(Path::new("."))
                .join(template);
            let mut template = read_profile(&path)?;
            template.permanent_patterns.clear();
            template.temporary_patterns.clear();
            template.scripts.clear();
            template
        }
        None => GameProfile::create_empty(),
    };
    let text_field = |name: &str| spec.get(name).and_then(Value::as_str).map(str::to_owned);
    profile.title = text_field("title");
    profile.author = text_field("author");
    profile.publisher = text_field("publisher");
    profile.description = text_field("description");
    profile.id = text_field("id");
    if let Some(overrides) = spec.get("layers").and_then(Value::as_array) {
        apply_layers(&mut profile, overrides)?;
    }
    let tiles = read_tiles(&spec)?;
    let base = description.parent().unwrap_or(Path::new("."));
    let patterns = spec
        .get("patterns")
        .and_then(Value::as_array)
        .ok_or_else(|| error("no \"patterns\" list"))?;
    for (index, entry) in patterns.iter().enumerate() {
        profile
            .permanent_patterns
            .push(Arc::new(build_pattern(index, entry, &tiles, base)?));
    }
    for entry in spec
        .get("scripts")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
    {
        let file = entry
            .get("file")
            .and_then(Value::as_str)
            .ok_or_else(|| error("a script needs a \"file\""))?;
        let path = description.parent().unwrap_or(Path::new(".")).join(file);
        let source = std::fs::read_to_string(&path)
            .map_err(|e| error(format!("{}: {e}", path.display())))?;
        let name = entry.get("name").and_then(Value::as_str).unwrap_or(file);
        profile
            .scripts
            .push(zax_profiles::script::ProfileScript::new(
                Some(name),
                true,
                Some(&source),
                Some("*"),
            ));
    }
    zax_profiles::export::normalize(&mut profile, None).map_err(|e| error(e.to_string()))?;
    let bytes = format::save(&profile, FieldPresence::All);
    std::fs::write(out, &bytes).map_err(|e| error(format!("{}: {e}", out.display())))?;
    Ok(format!(
        "profile-build: {} patterns, {} tiles -> {} ({} bytes)",
        patterns.len(),
        tiles.len(),
        out.display(),
        bytes.len()
    ))
}

/// `"layers": [{"index": 4, "offset": 0, "aligned": 1}]` overrides the stored layer sets
/// (index 0 to 5; aligned 0 front, 1 centre, 2 back).
fn apply_layers(profile: &mut GameProfile, overrides: &[Value]) -> Result<(), HarnessError> {
    let layers = profile
        .settings
        .as_mut()
        .and_then(|settings| settings.layers.as_mut())
        .ok_or_else(|| error("the template has no layer settings"))?;
    for entry in overrides {
        let index = usize::try_from(int(entry.get("index"), -1))
            .ok()
            .filter(|index| *index < layers.layers.len())
            .ok_or_else(|| error(format!("layer {entry}: no such index")))?;
        let layer = &mut layers.layers[index];
        #[allow(clippy::cast_possible_truncation)] // profile floats are f32
        if let Some(offset) = entry.get("offset").and_then(Value::as_f64) {
            layer.offset = offset as f32;
        }
        if let Some(aligned) = entry.get("aligned") {
            layer.aligned = LayerAlignment::from_raw(int(Some(aligned), 0));
        }
    }
    Ok(())
}

/// `"tiles": {"name": [low hex, high hex, optional "h"/"v"/"hv"]}`.
fn read_tiles(spec: &Value) -> Result<HashMap<String, Arc<ProfileTile>>, HarnessError> {
    let mut tiles = HashMap::new();
    let Some(table) = spec.get("tiles").and_then(Value::as_object) else {
        return Ok(tiles);
    };
    for (name, planes) in table {
        let planes = planes
            .as_array()
            .filter(|list| list.len() >= 2)
            .ok_or_else(|| error(format!("tile {name}: [low, high] expected")))?;
        let flip = match planes.get(2).and_then(Value::as_str) {
            Some("h") => TileFlip::HORIZONTAL,
            Some("v") => TileFlip::VERTICAL,
            Some("hv") => TileFlip::from_bits(3),
            _ => TileFlip::NONE,
        };
        tiles.insert(
            name.clone(),
            Arc::new(ProfileTile::new(
                hex64(&planes[0], name)?,
                hex64(&planes[1], name)?,
                flip,
            )),
        );
    }
    Ok(tiles)
}

/// One entry of `"patterns"`: its grid of tile names and its single variant.
fn build_pattern(
    index: usize,
    entry: &Value,
    tiles: &HashMap<String, Arc<ProfileTile>>,
    base: &Path,
) -> Result<ProfilePattern, HarnessError> {
    let rows = entry
        .get("grid")
        .and_then(Value::as_array)
        .ok_or_else(|| error(format!("pattern {index}: no grid")))?;
    let height = rows.len();
    let width = rows
        .iter()
        .filter_map(Value::as_array)
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    let mut cells = Vec::with_capacity(width * height);
    for row in rows {
        let row = row.as_array().cloned().unwrap_or_default();
        for column in 0..width {
            let cell = match row.get(column) {
                Some(Value::String(name)) => {
                    Some(Arc::clone(tiles.get(name).ok_or_else(|| {
                        error(format!("pattern {index}: unknown tile {name:?}"))
                    })?))
                }
                _ => None,
            };
            cells.push(cell);
        }
    }
    let (pixel_count, color_flags, tile_density, pixel_density) = describe(&cells);
    let (width, height) = (
        i32::try_from(width).unwrap_or(0),
        i32::try_from(height).unwrap_or(0),
    );
    let mut variant = PatternVariant {
        depth: int(entry.get("depth"), 8),
        geometry: VariantGeometry::from_raw(int(entry.get("geometry"), 0)),
        solid: flag(entry.get("solid"), false),
        layer: ProfileLayer::from_raw(int(entry.get("layer"), 2)),
        scale: float3(entry.get("scale")).or(Some(Vec3F::new(1.0, 1.0, 1.0))),
        rotation: float3(entry.get("rotation")).or(Some(Vec3F::new(0.0, 0.0, 0.0))),
        offset: float3(entry.get("offset")).or(Some(Vec3F::new(0.0, 0.0, 0.0))),
        pivot: float3(entry.get("pivot")).or(Some(Vec3F::new(0.5, 0.5, 0.5))),
        alpha: 1.0,
        render_style: VariantRenderStyle::Normal,
        tag_string: entry.get("tags").and_then(Value::as_str).map(str::to_owned),
        // "empty": a palette colour (1 to 3) left out of the volume, 0 for none
        stored_empty_color: int(entry.get("empty"), 0),
        ..PatternVariant::default()
    };
    // "texture": "<file.png>" beside the description: the variant's explicit texture
    // (REQ-PROF-035), drawn and given volume in place of the tiles' own pixels; a pixel whose
    // alpha is 0 is empty.
    if let Some(file) = entry.get("texture").and_then(Value::as_str) {
        variant.texture = Some(png_texture(&base.join(file))?);
    }
    // "deform": {"amplitude": [x, y, z], "pivot": [x, y, z], "speed": [16 cells, row by row],
    // "offset": [16 cells]}: the animated deformation (REQ-E3D-568).
    if let Some(deform) = entry.get("deform") {
        variant.deform_amplitude = float3(deform.get("amplitude"));
        variant.deform_pivot = float3(deform.get("pivot"));
        variant.deform_speed = matrix16(deform.get("speed"));
        variant.deform_offset = matrix16(deform.get("offset"));
    }
    // "copies": [{"offset": [x, y, z], "depth": d}, ...]: more variants of the pattern, each
    // drawn as well (one frame instance per variant, REQ-E3D-190): the main variant with that
    // offset and depth, e.g. a motif repeated in depth.
    let mut variants = vec![Arc::new(variant.clone())];
    for copy in entry
        .get("copies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let mut extra = variant.clone();
        if let Some(offset) = float3(copy.get("offset")) {
            extra.offset = Some(offset);
        }
        if copy.get("depth").is_some() {
            extra.depth = int(copy.get("depth"), extra.depth);
        }
        variants.push(Arc::new(extra));
    }
    Ok(ProfilePattern {
        grid: Some(PatternGrid::new(width, height, cells)),
        constraint_size: Vec2I::UNCONSTRAINED,
        tile_size: Some(Vec2I::new(width, height)),
        permanent: flag(entry.get("permanent"), true),
        pixel_count,
        color_flags,
        tile_density,
        pixel_density,
        variants,
        self_connected: flag(entry.get("self_connected"), true),
        skip_geometry: false,
        recorded_fields: None,
    })
}

/// A PNG as a direct variant texture: packed RGB, 0 for a transparent pixel (an opaque pure
/// black is written 1 so that it stays solid).
fn png_texture(path: &Path) -> Result<VariantTexture, HarnessError> {
    let image = zax_render::assets::png_image::load(path)
        .map_err(|e| error(format!("{}: {e}", path.display())))?;
    let data = (0..image.height)
        .flat_map(|y| (0..image.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (r, g, b, a) = image.pixel(x, y);
            if a == 0 {
                0
            } else {
                (i32::from(r) << 16 | i32::from(g) << 8 | i32::from(b)).max(1)
            }
        })
        .collect();
    Ok(VariantTexture::new(
        i32::try_from(image.width).unwrap_or(0),
        i32::try_from(image.height).unwrap_or(0),
        Some(TexturePixels::new(TextureFormat::Direct, data, Vec::new())),
    ))
}
