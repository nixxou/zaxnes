//! Writes the instance list of SPEC-04B REQ-E3D-641 and a build summary as JSON, next to the
//! OBJ files (REQ-ARCH-031). Port of `JsonWriter.cs`.

use std::collections::HashMap;
use std::fmt::{self, Display, Write as _};

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::{ConversionEngine, ConversionState};
use zax_engine3d::frame::ConversionFrame;
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::settings::Geo3DType;
use zax_engine3d::vectors::{Mat4, Vec2, Vec3};
use zax_profiles::profile::GameProfile;

use super::{decimals, Options};

/// The six geometry families, in declaration order.
const GEOMETRIES: [Geo3DType; 6] = [
    Geo3DType::Default,
    Geo3DType::HCylinder,
    Geo3DType::VCylinder,
    Geo3DType::Cube,
    Geo3DType::HalfHCylinder,
    Geo3DType::HalfVCylinder,
];

/// A JSON object written field by field, each line ending with a comma.
struct Writer {
    text: String,
}

impl Writer {
    fn raw(&mut self, depth: usize, name: &str, value: impl Display) -> fmt::Result {
        writeln!(self.text, "{}\"{name}\": {value},", "  ".repeat(depth))
    }

    fn string(&mut self, depth: usize, name: &str, value: &str) -> fmt::Result {
        let escaped = crate::json_text::escape(value);
        self.raw(depth, name, format_args!("\"{escaped}\""))
    }

    fn float(&mut self, depth: usize, name: &str, value: f32) -> fmt::Result {
        self.raw(depth, name, decimals(value, 6))
    }

    fn vec3(&mut self, depth: usize, name: &str, value: Vec3) -> fmt::Result {
        let text = format!(
            "[{}, {}, {}]",
            decimals(value.x, 6),
            decimals(value.y, 6),
            decimals(value.z, 6)
        );
        self.raw(depth, name, text)
    }

    fn vec2(&mut self, depth: usize, name: &str, value: Vec2) -> fmt::Result {
        self.raw(
            depth,
            name,
            format!("[{}, {}]", decimals(value.x, 6), decimals(value.y, 6)),
        )
    }

    fn matrix(&mut self, depth: usize, name: &str, value: &Mat4, last: bool) -> fmt::Result {
        let cells: Vec<String> = value
            .to_row_major()
            .iter()
            .map(|&cell| decimals(cell, 6))
            .collect();
        writeln!(
            self.text,
            "{}\"{name}\": [{}]{}",
            "  ".repeat(depth),
            cells.join(", "),
            if last { "" } else { "," }
        )
    }
}

/// The tiles to draw of one frame, with every field of REQ-E3D-641.
///
/// # Errors
/// [`fmt::Error`] when the text cannot be formatted.
pub fn frame(
    list: &FrameDrawList,
    frame: &ConversionFrame,
    number: usize,
) -> Result<String, fmt::Error> {
    let mut out = Writer {
        text: "{\n".to_owned(),
    };
    out.raw(1, "frame", number)?;
    out.raw(1, "frameCounter", list.frame_counter())?;
    out.float(1, "time", list.time())?;
    out.raw(1, "tileCount", list.tiles().len())?;

    let colors: Vec<String> = list
        .palette()
        .iter()
        .map(|color| format!("\"{color}\""))
        .collect();
    writeln!(out.text, "  \"paletteRgb\": [{}],", colors.join(", "))?;
    out.text.push_str("  \"tiles\": [\n");

    for (n, tile) in list.tiles().iter().enumerate() {
        let instance = frame.instance(tile.instance);
        let key = tile.tile.image().key();
        out.text.push_str("    {\n");
        out.raw(3, "index", n)?;
        out.raw(3, "tileX", tile.tile.x())?;
        out.raw(3, "tileY", tile.tile.y())?;
        out.raw(3, "imageId", tile.tile.image().id())?;

        // The tile identity of SPEC-04 §2, so that a synthetic layout can be replayed as a
        // `render --frames <file.json>` frame (the two planes are what that format takes).
        out.raw(3, "low", key.low)?;
        out.raw(3, "high", key.high)?;
        out.raw(3, "mirrorH", key.flip_x)?;
        out.raw(3, "mirrorV", key.flip_y)?;
        out.string(3, "geo", &format!("{:?}", instance.geometry))?;
        out.raw(3, "sizeZ", instance.depth)?;
        out.raw(3, "fullWarp", instance.full_warp)?;
        out.raw(3, "emptyColor", instance.empty_color)?;
        out.raw(
            3,
            "meshTriangles",
            tile.mesh.as_ref().map_or(0, |mesh| mesh.triangle_count()),
        )?;
        out.raw(
            3,
            "meshVertices",
            tile.mesh.as_ref().map_or(0, |mesh| mesh.vertex_count()),
        )?;
        out.string(3, "container", &format!("{:?}", tile.container))?;
        out.string(3, "material", &format!("{:?}", tile.material))?;
        out.float(3, "alpha", tile.alpha)?;
        out.string(3, "textureFormat", &format!("{:?}", tile.texture_format))?;
        out.raw(
            3,
            "textureWidth",
            tile.texture.as_ref().map_or(0, |texture| texture.width()),
        )?;
        out.raw(
            3,
            "textureHeight",
            tile.texture.as_ref().map_or(0, |texture| texture.height()),
        )?;
        out.raw(3, "paletteTable", tile.palette_table)?;
        out.vec2(3, "texOffset", tile.tex_offset)?;
        out.vec2(3, "texTilting", tile.tex_tilting)?;
        out.raw(3, "castShadow", tile.cast_shadow)?;
        out.raw(3, "receiveShadow", tile.receive_shadow)?;
        out.raw(3, "sprite", tile.sprite)?;
        out.raw(3, "behindBackground", tile.behind_background)?;
        out.vec3(3, "anchor", tile.anchor)?;
        out.vec3(3, "tileCenter", tile.tile_center)?;

        let cells: Vec<String> = tile
            .transform
            .to_row_major()
            .iter()
            .map(|&cell| decimals(cell, 6))
            .collect();
        write!(out.text, "      \"transform\": [{}]", cells.join(", "))?;

        if let Some(deform) = &tile.deform {
            out.text.push_str(",\n      \"deform\": {\n");
            out.vec3(4, "amplitude", deform.amplitude)?;
            out.vec3(4, "size", deform.size)?;
            out.vec3(4, "tileRelativeToPivot", deform.tile_relative_to_pivot)?;
            out.matrix(4, "speed", &deform.speed, false)?;
            out.matrix(4, "phase", &deform.phase, true)?;
            out.text.push_str("      }");
        }

        out.text.push('\n');
        out.text.push_str(if n + 1 < list.tiles().len() {
            "    },\n"
        } else {
            "    }\n"
        });
    }

    out.text.push_str("  ]\n}\n");
    Ok(out.text)
}

/// What a summary reports.
pub struct SummaryInput<'a> {
    /// The profile's file name.
    pub profile_name: &'a str,
    /// The loaded profile.
    pub profile: &'a GameProfile,
    /// The engine, with its geometry builder.
    pub engine: &'a ConversionEngine,
    /// The command line.
    pub options: &'a Options,
    /// How long the profile load and the geometry build took.
    pub load_milliseconds: f64,
    /// Frames exported.
    pub frames: usize,
    /// Tiles drawn over every frame.
    pub tiles: usize,
}

/// What was loaded and built, with the per-geometry counts of annex A.
///
/// # Errors
/// [`fmt::Error`] when the text cannot be formatted.
pub fn summary(input: &SummaryInput<'_>) -> Result<String, fmt::Error> {
    let engine = input.engine;
    let state: &ConversionState = engine.state();
    let mut objects_by_geometry: HashMap<Geo3DType, usize> = HashMap::new();
    let (mut objects, mut meshes, mut triangles, mut vertices) = (0, 0, 0, 0);

    if let Some(builder) = engine.geometry_builder() {
        for id in state.patterns().all() {
            let Some(pattern) = state.patterns().get(id) else {
                continue;
            };

            for (index, instance) in pattern.instances().iter().enumerate() {
                let Some(built) = builder.of(InstanceRef { pattern: id, index }) else {
                    continue;
                };

                objects += 1;
                *objects_by_geometry.entry(instance.geometry).or_default() += 1;
                for mesh in built.meshes() {
                    meshes += 1;
                    triangles += mesh.triangle_count();
                    vertices += mesh.vertex_count();
                }
            }
        }
    }

    let mut out = Writer {
        text: "{\n".to_owned(),
    };
    out.string(1, "profile", input.profile_name)?;
    out.string(
        1,
        "formatVersion",
        input.profile.version_text.as_deref().unwrap_or_default(),
    )?;
    out.raw(1, "isV2Format", input.profile.is_v2_format())?;
    out.string(1, "algorithm", &format!("{:?}", input.options.algorithm))?;
    out.string(
        1,
        "handedness",
        if input.options.right_handed {
            "right"
        } else {
            "left"
        },
    )?;
    out.raw(
        1,
        "patterns",
        state.patterns().permanent().len() + state.patterns().temporary().len(),
    )?;
    out.raw(1, "objects", objects)?;
    out.raw(1, "tileMeshes", meshes)?;
    out.raw(1, "triangles", triangles)?;
    out.raw(1, "vertices", vertices)?;
    #[allow(clippy::cast_possible_truncation)] // C# (float) cast of the milliseconds.
    let milliseconds = input.load_milliseconds as f32;
    out.float(1, "buildMilliseconds", milliseconds)?;
    out.float(1, "loadProgress", engine.load_progress())?;
    out.raw(1, "framesExported", input.frames)?;
    out.raw(1, "tilesDrawn", input.tiles)?;

    out.text.push_str("  \"objectsByGeometry\": {\n");
    for (n, kind) in GEOMETRIES.iter().enumerate() {
        let count = objects_by_geometry.get(kind).copied().unwrap_or(0);
        let separator = if n + 1 < GEOMETRIES.len() { "," } else { "" };
        writeln!(out.text, "    \"{kind:?}\": {count}{separator}")?;
    }

    out.text.push_str("  }\n}\n");
    Ok(out.text)
}
