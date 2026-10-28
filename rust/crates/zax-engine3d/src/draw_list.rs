//! Turns a converted frame into the ordered list of tiles to draw (SPEC-04B REQ-E3D-641).

use crate::hashing::FastMap;
use std::sync::Arc;

use zax_profiles::enums::TextureFormat;
use zax_profiles::tags::GROUND_AXIS_TAG;

use crate::frame::{ConversionFrame, FrameInstance, FrameInstanceId, FrameShape, FrameTile};
use crate::geometry::builder::PatternGeometryBuilder;
use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::TileMesh;
use crate::geometry::tile_sides;
use crate::grid;
use crate::image::TileImage;
use crate::instance::{PatternInstance, RenderStyle};
use crate::palette::{FramePalette, Rgb24};
use crate::registry::PatternRegistry;
use crate::settings::{ConversionSettings, RenderAlgorithm, ZLayer};
use crate::vectors::{Mat4, Vec2, Vec3};

/// Where an instance is attached in the scene (SPEC-04B REQ-E3D-409, REQ-E3D-641).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneContainer {
    /// The game container.
    Game = 0,
    /// The interface container, for the UI layer.
    Interface = 1,
}

/// Material class of a tile to draw (SPEC-04B REQ-E3D-642).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialClass {
    /// Opaque.
    Opaque = 0,
    /// Alpha blended, casts no shadow.
    Transparent = 1,
    /// UI layer.
    Interface = 2,
    /// Invisible but casts its shadow.
    ShadowOnly = 3,
}

/// The animated deformation parameters of a tile (SPEC-04B REQ-E3D-568).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeformParameters {
    /// `deltaAmp`.
    pub amplitude: Vec3,
    /// `deltaSpeed`.
    pub speed: Mat4,
    /// `deltaOffset`.
    pub phase: Mat4,
    /// `sz = (pixel width, pixel height, D)`.
    pub size: Vec3,
    /// `(tx + 4, Hs − ty − 4, 0) − Pd`.
    pub tile_relative_to_pivot: Vec3,
}

impl DeformParameters {
    /// The displacement `δ` of the local mesh vertex `vertex` at `time` seconds
    /// (REQ-E3D-568). The renderer may evaluate this in a shader instead; the result must be
    /// the same.
    ///
    /// `δ` is added to the vertex in the object's local frame, *before* the scale and the
    /// rotation (REQ-E3D-406, DEC-04B-Q2). Row 3 of the two matrices is free script data and
    /// has no effect here (REQ-E3D-569).
    #[must_use]
    pub fn displacement(&self, vertex: Vec3, time: f32) -> Vec3 {
        let point = self.tile_relative_to_pivot + vertex;
        let ratio = |p: f32, size: f32| if size == 0.0 { 0.0 } else { p / size };
        let normalised = Vec3::new(
            ratio(point.x, self.size.x),
            ratio(point.y, self.size.y),
            ratio(point.z, self.size.z),
        );

        Vec3::new(
            self.component(0, self.amplitude.x, self.size.x, normalised, time),
            self.component(1, self.amplitude.y, self.size.y, normalised, time),
            self.component(2, self.amplitude.z, self.size.z, normalised, time),
        )
    }

    fn component(&self, row: usize, amplitude: f32, size: f32, normalised: Vec3, time: f32) -> f32 {
        let (speed, phase) = (&self.speed, &self.phase);
        amplitude
            * size
            * ((normalised.x * speed[(row, 0)]) + phase[(row, 0)]).sin()
            * ((normalised.y * speed[(row, 1)]) + phase[(row, 1)]).sin()
            * ((normalised.z * speed[(row, 2)]) + phase[(row, 2)]).sin()
            * ((time * speed[(row, 3)]) + phase[(row, 3)]).sin()
    }
}

/// One tile to draw, in the abstract form the renderer consumes (SPEC-04B REQ-E3D-641).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // The four flags of the REQ-E3D-641 record.
pub struct DrawTile {
    /// The frame instance the tile belongs to.
    pub instance: FrameInstanceId,
    /// The tile of the shape.
    pub tile: FrameTile,
    /// The mesh of this tile image for this instance, or `None` when no geometry builder is
    /// plugged in (SPEC-04B REQ-E3D-640).
    pub mesh: Option<Arc<TileMesh>>,
    /// The complete transformation of REQ-E3D-406, scene transformation excluded: it maps a
    /// local mesh vertex to its scene position. The sprite depth nudge `n` and the
    /// deformation are *not* included (DEC-R2-T4).
    pub transform: Mat4,
    /// The anchor point `Pos` of the instance (REQ-E3D-205).
    pub anchor: Vec3,
    /// The tile centre `T` of SPEC-04B REQ-E3D-405.
    pub tile_center: Vec3,
    /// Where the tile is attached in the scene.
    pub container: SceneContainer,
    /// Material class (SPEC-04B REQ-E3D-642).
    pub material: MaterialClass,
    /// Opacity, 0 to 1.
    pub alpha: f32,
    /// The texture of the 3D object, resolved by the geometry builder — the custom texture of
    /// the profile or the pattern image (SPEC-04B REQ-E3D-426, REQ-E3D-641). `None` when no
    /// geometry builder is plugged in.
    pub texture: Option<Arc<ObjectTexture>>,
    /// Texture format: indexed or direct.
    pub texture_format: TextureFormat,
    /// The palette table `P` of SPEC-04B REQ-E3D-560, or −1 for a direct texture, which needs
    /// none.
    pub palette_table: i32,
    /// Texture offset, a fraction of the texture size.
    pub tex_offset: Vec2,
    /// Texture repetition.
    pub tex_tilting: Vec2,
    /// Whether the tile casts a shadow (SPEC-04B REQ-E3D-567).
    pub cast_shadow: bool,
    /// Whether the tile receives shadows (SPEC-04B REQ-E3D-567).
    pub receive_shadow: bool,
    /// True for a tile of a sprite shape. With `behind_background` it lets the renderer pick
    /// the depth nudge `n` (SPEC-04B REQ-E3D-405).
    pub sprite: bool,
    /// True for a sprite tile whose priority bit puts it behind the background.
    pub behind_background: bool,
    /// The deformation parameters, or `None` when no deformation is active.
    pub deform: Option<DeformParameters>,
}

/// Everything the renderer needs for one displayed frame (SPEC-04B REQ-E3D-641): the ordered
/// tiles to draw, the 64 resolved palette colours and the frame time.
#[derive(Debug, Clone)]
pub struct FrameDrawList {
    tiles: Vec<DrawTile>,
    palette: [Rgb24; FramePalette::ENTRY_COUNT],
    frame_counter: i32,
    time: f32,
}

impl Default for FrameDrawList {
    fn default() -> Self {
        Self {
            tiles: Vec::new(),
            palette: [Rgb24::default(); FramePalette::ENTRY_COUNT],
            frame_counter: 0,
            time: 0.0,
        }
    }
}

impl FrameDrawList {
    /// The ordered tiles to draw.
    #[must_use]
    pub fn tiles(&self) -> &[DrawTile] {
        &self.tiles
    }

    /// The 64 resolved RGB colours of the frame palette.
    #[must_use]
    pub const fn palette(&self) -> &[Rgb24; FramePalette::ENTRY_COUNT] {
        &self.palette
    }

    /// The frame counter of the frame (REQ-E3D-043).
    #[must_use]
    pub const fn frame_counter(&self) -> i32 {
        self.frame_counter
    }

    /// The frame time in seconds (REQ-E3D-043).
    #[must_use]
    pub const fn time(&self) -> f32 {
        self.time
    }

    fn begin(&mut self, frame: &ConversionFrame) {
        self.tiles.clear();
        self.frame_counter = frame.counter();
        self.time = frame.time();
        frame.palette().copy_colors_to(&mut self.palette);
    }
}

/// What a draw list build reads besides the frame.
pub struct DrawSources<'a> {
    /// The settings the pipeline consumes.
    pub settings: &'a ConversionSettings,
    /// The geometry side, which owns the meshes.
    pub geometry: &'a mut dyn PatternGeometryBuilder,
    /// The pattern registry, which holds the texture owners.
    pub patterns: &'a PatternRegistry,
    /// True when the loaded profile is in version 2 format, which moves the deformation pivot
    /// by one unit (SPEC-04B REQ-E3D-568).
    pub format_version_2: bool,
}

/// Turns a converted frame into the ordered list of tiles to draw (SPEC-04B REQ-E3D-641).
///
/// This is the hand-over point between SPEC-04 and SPEC-04B: the pipeline supplies the
/// placement, the palette table, the material class, the shadow flags and the deformation
/// parameters; the mesh of each tile comes from the geometry builder.
///
/// The order is the instance order of REQ-E3D-220 and, inside one instance, the tile order of
/// the shape. Instances that are hidden (REQ-E3D-191) or whose geometry is not ready
/// (REQ-E3D-361) contribute nothing; a shape restored from the pinned frame contributes only
/// the tiles that overlap one of the frame's rectangles (REQ-E3D-233). The list and its
/// buffers are reused, so a build allocates nothing once warm.
#[derive(Debug, Default)]
pub struct DrawListBuilder {
    list: FrameDrawList,
    neighbours: FastMap<(i32, i32), Option<TileImage>>,
}

impl DrawListBuilder {
    /// Creates a builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The list of the last build.
    #[must_use]
    pub const fn list(&self) -> &FrameDrawList {
        &self.list
    }

    /// Swaps the list of the last build with `list`: the caller keeps the built list while the
    /// builder fills the other one next time (its storage is reused).
    pub fn exchange(&mut self, list: &mut FrameDrawList) {
        std::mem::swap(&mut self.list, list);
    }

    /// Builds the draw list of `frame`.
    pub fn build(
        &mut self,
        frame: &ConversionFrame,
        sources: &mut DrawSources<'_>,
    ) -> &FrameDrawList {
        self.list.begin(frame);
        for &id in frame.instances() {
            let instance = frame.instance(id);
            if instance.style == RenderStyle::Hidden
                || !sources.geometry.is_ready(instance.reference())
            {
                continue;
            }

            self.add_instance(frame, id, sources);
        }

        &self.list
    }

    fn add_instance(
        &mut self,
        frame: &ConversionFrame,
        id: FrameInstanceId,
        sources: &mut DrawSources<'_>,
    ) {
        let instance = frame.instance(id);
        let shape = frame.shape(id.shape);
        let look = InstanceLook::of(instance, shape, sources);

        // Q-067: the walls that close each tile against a neighbour of the same object show at
        // the joints of a flat surface (REQ-SCN-065). The neighbours are looked up here, once
        // per instance, so that the draw list can ask for the mesh variant without them.
        self.neighbours.clear();
        for tile in frame.shape_tiles(id.shape) {
            if !shape.restored_from_pinned_frame() || frame.tile_overlaps_pinned_rects(tile) {
                // A sprite shape may hold two tiles on one cell (REQ-E3D-043): which of them
                // covers the joint is then undecided, so the cell seals nothing.
                self.neighbours
                    .entry((tile.x(), tile.y()))
                    .and_modify(|neighbour| *neighbour = None)
                    .or_insert(Some(*tile.image()));
            }
        }

        for tile in frame.shape_tiles(id.shape) {
            if shape.restored_from_pinned_frame() && !frame.tile_overlaps_pinned_rects(tile) {
                continue;
            }

            let sealed = self.sealed_sides(instance, tile, &*sources.geometry);
            let mesh = sources
                .geometry
                .mesh_of(instance.reference(), tile.image(), sealed);
            self.list
                .tiles
                .push(look.tile(id, instance, shape, tile, mesh));
        }
    }

    /// The lateral sides of `tile` a neighbouring tile of the same object covers entirely, so
    /// that the wall between the two can be left out of the drawn mesh (Q-067, SPEC-06
    /// REQ-SCN-065).
    ///
    /// Both planes have to be full: the wall is then strictly inside the solid the two tiles
    /// form, and dropping it cannot open a hole. Tiles of two different objects are never
    /// sealed against each other, since nothing says their depths or their z agree.
    fn sealed_sides(
        &self,
        instance: &FrameInstance,
        tile: &FrameTile,
        geometry: &dyn PatternGeometryBuilder,
    ) -> u8 {
        let full = geometry.full_sides_of(instance.reference(), tile.image());
        if full == 0 {
            return 0;
        }

        let seal = |side: u8, x: i32, y: i32| -> u8 {
            if full & side == 0 {
                return 0;
            }

            match self.neighbours.get(&(x, y)) {
                Some(Some(neighbour)) => {
                    let facing = geometry.full_sides_of(instance.reference(), neighbour);
                    if facing & tile_sides::opposite(side) != 0 {
                        side
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        };

        // Scene y runs the other way than the screen row (REQ-E3D-404): the neighbour below
        // the tile in the scene is the one on the row underneath on screen.
        seal(tile_sides::MINUS_X, tile.x() - 8, tile.y())
            | seal(tile_sides::PLUS_X, tile.x() + 8, tile.y())
            | seal(tile_sides::MINUS_Y, tile.x(), tile.y() + 8)
            | seal(tile_sides::PLUS_Y, tile.x(), tile.y() - 8)
    }
}

/// What every tile of one instance shares: material, shadows, texture and transform parts.
struct InstanceLook {
    container: SceneContainer,
    material: MaterialClass,
    cast_shadow: bool,
    receive_shadow: bool,
    texture: Option<Arc<ObjectTexture>>,
    format: TextureFormat,
    /// `Pos . R . S` of REQ-E3D-406, the part of the transform every tile of the instance
    /// shares; each tile only multiplies it by its own translation, in the same order as the
    /// whole product, so the floats are those of the full product.
    placement: Mat4,
    size: Vec3,
    deform_pivot: Vec3,
}

impl InstanceLook {
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    fn of(instance: &FrameInstance, shape: &FrameShape, sources: &DrawSources<'_>) -> Self {
        let settings = sources.settings;
        let layer = instance.effective_layer(settings);
        let layer_setting = settings.layer(layer);
        let ui = layer == ZLayer::Ui;
        let material = material_of(instance, ui);

        // The texture comes from the instance's own pattern instance, unless a script
        // redirected the texture slot with shape.slot = other.slot (SPEC-05 REQ-LUA-097).
        let texture_owner = instance.texture_owner();
        let texture = sources.geometry.texture_of(texture_owner);
        let format = texture.as_ref().map_or_else(
            || {
                sources
                    .patterns
                    .instance(texture_owner)
                    .and_then(|owner| owner.texture.as_ref())
                    .and_then(|texture| texture.pixels.as_ref())
                    .map_or(TextureFormat::Indexed, |pixels| pixels.format)
            },
            |texture| texture.format(),
        );

        let size = Vec3::new(
            (shape.pixel_end().x - shape.pixel_start().x + 1) as f32,
            (shape.pixel_end().y - shape.pixel_start().y + 1) as f32,
            instance.depth as f32,
        );

        Self {
            container: if ui {
                SceneContainer::Interface
            } else {
                SceneContainer::Game
            },
            material,
            cast_shadow: settings.shadow
                && !ui
                && matches!(material, MaterialClass::Opaque | MaterialClass::ShadowOnly)
                && instance.cast_shadow
                && layer_setting.cast_shadow,
            receive_shadow: settings.shadow
                && !ui
                && instance.receive_shadow
                && layer_setting.receive_shadow,
            texture,
            format,
            placement: Mat4::translation(instance.pos)
                * rotation_of(instance)
                * Mat4::scale_matrix(scale_of(instance, shape, settings)),
            size,
            deform_pivot: deform_pivot(instance, shape, size, sources.format_version_2),
        }
    }

    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    fn tile(
        &self,
        id: FrameInstanceId,
        instance: &FrameInstance,
        shape: &FrameShape,
        tile: &FrameTile,
        mesh: Option<Arc<TileMesh>>,
    ) -> DrawTile {
        let anchor = instance.pos;
        let tile_center = Vec3::new(
            (tile.x() + 4) as f32 + instance.offset.x,
            (grid::SCENE_HEIGHT - tile.y() - 4) as f32 + instance.offset.y,
            instance.center.z,
        );

        // P = Pos + R . (S (T - Pos + v)) : REQ-E3D-406.
        let transform = self.placement * Mat4::translation(tile_center - anchor);

        // REQ-E3D-560: the palette table is customPalette when it is set, and otherwise the
        // palette of **each tile of the frame** (plus 4 for a sprite shape) — not the shape's
        // own palette, which REQ-E3D-047 derives from the germ alone. A background shape that
        // spans several attribute quads therefore keeps the colours of each quad, as
        // REQ-SCN-034 restates. A background tile in sub-palette 4-7 (a margin decoder's own
        // palette) uses tables 8-11, where the frame keeps that palette.
        let palette_table = if self.format == TextureFormat::Direct {
            -1
        } else if instance.custom_palette >= 0 {
            instance.custom_palette
        } else {
            tile.palette()
                + if shape.background() && tile.palette() < 4 {
                    0
                } else {
                    4
                }
        };

        let deform = instance.deform_enabled().then(|| DeformParameters {
            amplitude: instance.deform_amplitude,
            speed: instance.deform_speed,
            phase: instance.deform_offset,
            size: self.size,
            tile_relative_to_pivot: Vec3::new(
                (tile.x() + 4) as f32,
                (grid::SCENE_HEIGHT - tile.y() - 4) as f32,
                0.0,
            ) - self.deform_pivot,
        });

        DrawTile {
            instance: id,
            tile: *tile,
            mesh,
            transform,
            anchor,
            tile_center,
            container: self.container,
            material: self.material,
            alpha: instance.alpha,
            texture: self.texture.clone(),
            texture_format: self.format,
            palette_table,
            tex_offset: instance.tex_offset,
            tex_tilting: instance.tex_tilting,
            cast_shadow: self.cast_shadow,
            receive_shadow: self.receive_shadow,
            sprite: !shape.background(),
            behind_background: !shape.background() && tile.hidden(),
            deform,
        }
    }
}

/// The material class of SPEC-04B REQ-E3D-566 and REQ-E3D-642: interface in the UI layer,
/// shadow-only for render style 2, opaque when alpha is 1, transparent otherwise.
fn material_of(instance: &PatternInstance, ui: bool) -> MaterialClass {
    if ui {
        MaterialClass::Interface
    } else if instance.style == RenderStyle::ShadowOnly {
        MaterialClass::ShadowOnly
    } else if instance.alpha >= 1.0 {
        MaterialClass::Opaque
    } else {
        MaterialClass::Transparent
    }
}

/// The rotation of SPEC-04B REQ-E3D-407: `Rx·Ry·Rz`, or `Ry·Rx·Rz` when the tags carry
/// `_gaxis`.
///
/// `_gaxis` has to be one element of the comma-separated list, not a piece of a longer tag;
/// one object of the built-in catalogue stores `mc._gaxis` and so keeps the default order,
/// which is why annex A counts one more than the engine does (Q-018).
fn rotation_of(instance: &PatternInstance) -> Mat4 {
    let x = Mat4::rotation_x(instance.rotation.x);
    let y = Mat4::rotation_y(instance.rotation.y);
    let z = Mat4::rotation_z(instance.rotation.z);
    if instance.has_tag(GROUND_AXIS_TAG) {
        y * x * z
    } else {
        x * y * z
    }
}

/// The scale of an instance, with the MARCHING correction of SPEC-04B REQ-E3D-408.
#[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
fn scale_of(instance: &FrameInstance, shape: &FrameShape, settings: &ConversionSettings) -> Vec3 {
    let scale = instance.scale;
    if instance.effective_algorithm(settings.render_algorithm) != RenderAlgorithm::Marching {
        return scale;
    }

    let sx = (shape.pixel_end().x - shape.pixel_start().x + 1) as f32;
    let sy = (shape.pixel_end().y - shape.pixel_start().y + 1) as f32;
    let sz = instance.depth as f32;
    Vec3::new(
        scale.x * (sx / (sx + 1.0)),
        scale.y * (sy / (sy + 1.0)),
        if sz == 0.0 {
            0.0
        } else {
            scale.z * (sz / (sz + 1.0))
        },
    )
}

/// The deformation pivot `Pd` of SPEC-04B REQ-E3D-568:
/// `deltaPivot ⊙ sz + (pStart.x, Hs − pEnd.y − h₀, −D/2)`, with `h₀ = 0` for a version 2
/// profile and 1 otherwise.
#[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
fn deform_pivot(
    instance: &PatternInstance,
    shape: &FrameShape,
    size: Vec3,
    format_version_2: bool,
) -> Vec3 {
    let h0 = if format_version_2 { 0.0 } else { 1.0 };
    Vec3::scale(instance.deform_pivot, size)
        + Vec3::new(
            shape.pixel_start().x as f32,
            (grid::SCENE_HEIGHT - shape.pixel_end().y) as f32 - h0,
            -instance.depth as f32 / 2.0,
        )
}
