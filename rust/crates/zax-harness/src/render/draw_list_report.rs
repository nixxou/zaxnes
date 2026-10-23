//! A text report of the draw list of one frame, grouped by 3D object: the material path a tile
//! takes (SPEC-06 REQ-SCN-034), the palette table it addresses and the NES colours its texels
//! end up on (port of `DrawListReport.cs`). It is the measurement tool of the colour-fidelity
//! comparison of lot L7b.

use std::collections::HashMap;
use std::fmt::Write as _;

use zax_engine3d::draw_list::{DrawTile, FrameDrawList};
use zax_engine3d::frame::{ConversionFrame, FrameInstance};
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::registry::PatternRegistry;
use zax_profiles::enums::TextureFormat;

/// Tiles of a group whose texels are counted; the later ones add nothing to the colour list.
const SAMPLED_TILES: usize = 4;

/// Largest texture side read for the colour list.
const SAMPLED_SIDE: i32 = 64;

/// Builds the report of `list`; `frame` and `patterns` resolve the instance handles of its tiles.
#[must_use]
pub fn describe(
    list: &FrameDrawList,
    frame: &ConversionFrame,
    patterns: &PatternRegistry,
) -> String {
    let mut groups: Vec<Group> = Vec::new();
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for tile in list.tiles() {
        let instance = frame.instance(tile.instance);
        let pattern = frame
            .shape(tile.instance.shape)
            .reference_pattern()
            .and_then(|id| patterns.get(id))
            .map_or_else(
                || "p-".to_owned(),
                |reference| {
                    format!(
                        "p{}({}x{})",
                        reference.registration_index(),
                        reference.width(),
                        reference.height()
                    )
                },
            );
        let key = format!(
            "{pattern}.{} bg={} pal={} fmt={:?} mat={:?} layer={:?}",
            tile.instance.index,
            if instance.background() {
                "True"
            } else {
                "False"
            },
            tile.palette_table,
            tile.texture_format,
            tile.material,
            instance.layer
        );

        let slot = *index_of.entry(key.clone()).or_insert_with(|| {
            groups.push(Group::new(key, instance));
            groups.len() - 1
        });
        let group = &mut groups[slot];
        group.add(tile, list);
        group.measure(tile);
    }

    // OrderByDescending is stable: equal groups keep the order they were met in.
    groups.sort_by_key(|group| std::cmp::Reverse(group.tiles));

    let mut text = String::new();
    // Writing to a String cannot fail.
    let _ = writeln!(
        text,
        "{} tile(s), {} group(s)",
        list.tiles().len(),
        groups.len()
    );
    for group in &groups {
        let _ = writeln!(
            text,
            "  {:<64} {:>5} tiles  {}",
            group.key,
            group.tiles,
            group.colors_line()
        );
        let _ = writeln!(text, "  {:<64} {}", "", group.depth_line());
    }
    text
}

/// The tiles that share one key, and what is measured over them.
struct Group {
    key: String,
    tiles: usize,
    texture_size: Option<(i32, i32)>,
    /// Packed colour and its count, in the order first met.
    colors: Vec<(i32, i32)>,
    color_slot: HashMap<i32, usize>,
    z_min: f32,
    z_max: f32,
    placement: Placement,
}

/// The placement quantities of the first instance of a group.
struct Placement {
    depth: f32,
    offset: [f32; 3],
    pivot: [f32; 3],
    scale: [f32; 3],
    pos_z: f32,
    center_z: f32,
}

impl Group {
    fn new(key: String, instance: &FrameInstance) -> Self {
        Self {
            key,
            tiles: 0,
            texture_size: None,
            colors: Vec::new(),
            color_slot: HashMap::new(),
            z_min: f32::INFINITY,
            z_max: f32::NEG_INFINITY,
            placement: Placement {
                #[allow(clippy::cast_precision_loss)] // A depth is a small integer.
                depth: instance.depth as f32,
                offset: [instance.offset.x, instance.offset.y, instance.offset.z],
                pivot: [instance.pivot.x, instance.pivot.y, instance.pivot.z],
                scale: [instance.scale.x, instance.scale.y, instance.scale.z],
                pos_z: instance.pos.z,
                center_z: instance.center.z,
            },
        }
    }

    /// Counts the colours of the texels of the first tiles of the group.
    fn add(&mut self, tile: &DrawTile, list: &FrameDrawList) {
        self.tiles += 1;
        let Some(texture) = tile.texture.as_deref() else {
            return;
        };
        if self.tiles == 1 {
            self.texture_size = Some((texture.width(), texture.height()));
        }
        if self.tiles > SAMPLED_TILES {
            return;
        }

        let table = usize::try_from(tile.palette_table.clamp(0, 15)).unwrap_or(0) * 4;
        for y in 0..texture.height().min(SAMPLED_SIDE) {
            for x in 0..texture.width().min(SAMPLED_SIDE) {
                let packed = if texture.format() == TextureFormat::Direct {
                    texture.color(x, y).packed()
                } else {
                    let column = ObjectTexture::palette_column(texture.texel(x, y));
                    list.palette()[table + usize::try_from(column).unwrap_or(0)].packed()
                };
                self.count(packed);
            }
        }
    }

    fn count(&mut self, packed: i32) {
        let slot = *self.color_slot.entry(packed).or_insert_with(|| {
            self.colors.push((packed, 0));
            self.colors.len() - 1
        });
        self.colors[slot].1 += 1;
    }

    /// Accumulates the scene z the group's meshes actually occupy, so that a capture can be
    /// read against the depth interval of REQ-E3D-205 and the placement of REQ-E3D-405/406.
    fn measure(&mut self, tile: &DrawTile) {
        let Some(mesh) = tile.mesh.as_deref().filter(|mesh| !mesh.is_empty()) else {
            return;
        };
        let (min, max) = (mesh.bounds_min(), mesh.bounds_max());
        for corner in 0..8_u8 {
            let local = zax_engine3d::vectors::Vec3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let z = tile.transform.transform_point(local).z;
            self.z_min = self.z_min.min(z);
            self.z_max = self.z_max.max(z);
        }
    }

    /// The texture size and the five most frequent colours.
    fn colors_line(&self) -> String {
        let size = self
            .texture_size
            .map_or_else(|| "no texture".to_owned(), |(w, h)| format!("{w}x{h}"));
        let mut ranked = self.colors.clone();
        ranked.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
        let colors: Vec<String> = ranked
            .iter()
            .take(5)
            .map(|(color, count)| format!("#{color:06X}x{count}"))
            .collect();
        format!("tex {size}  {}", colors.join(" "))
    }

    /// The placement line of the group: what REQ-E3D-205 feeds and what comes out.
    fn depth_line(&self) -> String {
        let p = &self.placement;
        format!(
            "depth {:.1}  offset ({:.1}, {:.1}, {:.1})  pivot ({:.2}, {:.2}, {:.2})  \
             scale ({:.2}, {:.2}, {:.2})  Pos.z {:.1}  Ctr.z {:.1}  drawn z [{:.1}, {:.1}]",
            p.depth,
            p.offset[0],
            p.offset[1],
            p.offset[2],
            p.pivot[0],
            p.pivot[1],
            p.pivot[2],
            p.scale[0],
            p.scale[1],
            p.scale[2],
            p.pos_z,
            p.center_z,
            self.z_min,
            self.z_max
        )
    }
}
