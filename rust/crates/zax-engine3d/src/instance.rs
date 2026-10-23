//! The 3D parameters of an instance (SPEC-04 §4.3, SPEC-05 §4.2 type `Instance`).

use std::sync::Arc;

use zax_profiles::enums::VariantRenderStyle;
use zax_profiles::tags;
use zax_profiles::values::{Mat4F, Vec3F};
use zax_profiles::variant::{PatternVariant, VariantTexture};

use crate::registry::PatternId;
use crate::settings::{Geo3DType, RenderAlgorithm, ZLayer};
use crate::vectors::{Mat4, Vec2, Vec3};

/// How an instance is drawn (SPEC-03 `renderStyle`, REQ-E3D-191).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderStyle {
    /// Kept in the frame but not drawn.
    Hidden = 0,
    /// Drawn normally.
    #[default]
    Normal = 1,
    /// Invisible but casts its shadow.
    ShadowOnly = 2,
}

impl RenderStyle {
    /// The style of a stored integer. Any other value is drawn as Normal, which is what every
    /// test of the pipeline does with it (REQ-E3D-191, SPEC-04B REQ-E3D-566).
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Hidden,
            2 => Self::ShadowOnly,
            _ => Self::Normal,
        }
    }

    /// The style of a profile variant.
    #[must_use]
    pub const fn from_profile(style: VariantRenderStyle) -> Self {
        Self::from_raw(style.raw())
    }
}

/// The handle the script layer caches on a long-lived engine object so that the same object is
/// always the same Lua value (SPEC-05 REQ-LUA-043) without a per-frame lookup table
/// (REQ-LUA-111).
///
/// `owner` identifies the runtime that assigned `handle`; a runtime that does not recognise
/// the owner allocates its own handle elsewhere, so two engines in one process stay
/// independent (REQ-ARCH-011). It is never copied: cloning an object gives it an unassigned
/// slot. It is never serialised and has no meaning outside the Lua layer.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScriptHandleSlot {
    /// Identifier of the runtime that owns `handle`; 0 when unassigned.
    pub owner: i32,
    /// The handle, meaningful only to the owning runtime.
    pub handle: i32,
}

impl Clone for ScriptHandleSlot {
    /// An unassigned slot: a handle belongs to one object only.
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// The address of a pattern instance: its pattern in the registry and its index in the
/// pattern's instance list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceRef {
    /// The pattern.
    pub pattern: PatternId,
    /// The index of the instance in the pattern's list (0-based).
    pub index: usize,
}

/// The tag string of an instance and its parsed tags, shared between a pattern instance and
/// its frame instances (REQ-E3D-190).
#[derive(Debug, Default, PartialEq, Eq)]
struct InstanceTags {
    string: String,
    tags: Vec<String>,
}

/// The 3D parameters of one instance: a pattern instance of the registry (the "3D variant" of
/// SPEC-03) or the parameter part of a frame instance, the per-frame copy of REQ-E3D-190.
///
/// The vector and matrix parameters are public mutable fields so that a script can write one
/// component in place (`shape.Offset.x = …`, REQ-LUA-044).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // The flags of SPEC-04 §4.3.
pub struct PatternInstance {
    /// Depth in voxels (`sizeZ`, 0 to 96).
    pub depth: i32,
    /// Geometry family (`geo`).
    pub geometry: Geo3DType,
    /// Excludes region 2 texels of the cylinders (`antiShadow`).
    pub anti_shadow: bool,
    /// Side of the half cylinders (`flip`).
    pub flip: bool,
    /// Solid cylinder section (`solid`).
    pub solid: bool,
    /// Wrapped extrusion (`fullWarp`).
    pub full_warp: bool,
    /// Depth layer (`layer`).
    pub layer: ZLayer,
    /// Scale factors (`scale`).
    pub scale: Vec3,
    /// Rotation in degrees (`rot`).
    pub rotation: Vec3,
    /// Offset of the anchor point (`offset`).
    pub offset: Vec3,
    /// Pivot, a fraction of the placement box per axis (`pivot`).
    pub pivot: Vec3,
    /// Opacity 0 to 1 (`alpha`).
    pub alpha: f32,
    /// Per-instance mesh algorithm (`rm`): 0 = use the global setting, 1 = GREEDY,
    /// 2 = MARCHING, 3 = `GREEDY_MULTICOLOR` (REQ-E3D-340).
    pub mesh_algorithm: i32,
    /// Deformation amplitude (`deltaAmp`).
    pub deform_amplitude: Vec3,
    /// Deformation pivot (`deltaPivot`).
    pub deform_pivot: Vec3,
    /// Deformation speeds (`deltaSpeed`); row 3 holds `CData[0..3]`.
    pub deform_speed: Mat4,
    /// Deformation phases (`deltaOffset`); row 3 holds `CData[4..7]`.
    pub deform_offset: Mat4,
    /// Render style (`renderStyle`).
    pub style: RenderStyle,
    /// Transparent colour after normalisation (`emptyColor`, REQ-E3D-424).
    pub empty_color: i32,
    /// Custom palette table, or −1 for "none" (`customPalette`, REQ-E3D-192).
    pub custom_palette: i32,
    /// Texture offset, a fraction of the texture size (`texOffset`).
    pub tex_offset: Vec2,
    /// Texture repetition (`texTilting`).
    pub tex_tilting: Vec2,
    /// Fill rate of the cylindrical geometries (`fillrate`); runtime only.
    pub fill_rate: f32,
    /// Whether the instance casts a shadow; false when the tags carry `!cast`.
    pub cast_shadow: bool,
    /// Whether the instance receives shadows; false when the tags carry `!receiv` or `!recv`.
    pub receive_shadow: bool,
    /// The custom texture of the profile, or `None` when the instance uses the pattern image
    /// (SPEC-04B REQ-E3D-426). Shared with the pattern instance.
    pub texture: Option<Arc<VariantTexture>>,
    /// The pattern instance whose texture this one is drawn with, or `None` to use its own. A
    /// script sets it through `shape.slot = other.slot` (SPEC-05 REQ-LUA-097).
    pub texture_source: Option<InstanceRef>,
    /// The script layer's handle cache; not part of the instance parameters.
    pub script_handle: ScriptHandleSlot,
    index: usize,
    tags: Arc<InstanceTags>,
}

impl Default for PatternInstance {
    /// An instance with the defaults of SPEC-04 §4.3 and REQ-E3D-453.
    fn default() -> Self {
        let mut deform_offset = Mat4::default();
        set_rows_012(&mut deform_offset, std::f32::consts::PI / 2.0);
        Self {
            depth: 3,
            geometry: Geo3DType::Default,
            anti_shadow: false,
            flip: false,
            solid: false,
            full_warp: false,
            layer: ZLayer::L1,
            scale: Vec3::ONE,
            rotation: Vec3::ZERO,
            offset: Vec3::ZERO,
            pivot: Vec3::HALF,
            alpha: 1.0,
            mesh_algorithm: 0,
            deform_amplitude: Vec3::ZERO,
            deform_pivot: Vec3::HALF,
            deform_speed: Mat4::default(),
            deform_offset,
            style: RenderStyle::Normal,
            empty_color: 0,
            custom_palette: -1,
            tex_offset: Vec2::ZERO,
            tex_tilting: Vec2::ONE,
            fill_rate: 1.0,
            cast_shadow: true,
            receive_shadow: true,
            texture: None,
            texture_source: None,
            script_handle: ScriptHandleSlot::default(),
            index: 0,
            tags: Arc::default(),
        }
    }
}

/// Deformation default of REQ-E3D-453: `value` on rows 0 to 2, 0 on row 3.
fn set_rows_012(matrix: &mut Mat4, value: f32) {
    for row in 0..4 {
        for column in 0..4 {
            matrix[(row, column)] = if row < 3 { value } else { 0.0 };
        }
    }
}

impl PatternInstance {
    /// Creates an instance with the defaults of SPEC-04 §4.3 and REQ-E3D-453.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Index of the instance in the instance list of its owner (0-based).
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    pub(crate) fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// The raw tag string.
    #[must_use]
    pub fn tag_string(&self) -> &str {
        &self.tags.string
    }

    /// Writes the tag string, which recomputes the tags and the two shadow flags (SPEC-04B
    /// REQ-E3D-567).
    pub fn set_tag_string(&mut self, value: &str) {
        self.tags = Arc::new(InstanceTags {
            string: value.to_owned(),
            tags: tags::parse(Some(value))
                .into_iter()
                .map(str::to_owned)
                .collect(),
        });
        self.cast_shadow = !self.has_tag(tags::CAST_SHADOW_TAG);
        self.receive_shadow = !self.has_tag(tags::RECEIVE_SHADOW_TAG)
            && !self.has_tag(tags::RECEIVE_SHADOW_SHORT_TAG);
    }

    /// Tags of the instance: non-empty comma-separated elements, trimmed.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags.tags
    }

    /// True when the instance carries `tag` (exact, case-sensitive).
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.tags.iter().any(|own| own == tag)
    }

    /// True when `tag` is `"*"` or one of the instance's tags (SPEC-05 REQ-LUA-083 point 5).
    #[must_use]
    pub fn contains_tag(&self, tag: &str) -> bool {
        tag == tags::WILDCARD || self.has_tag(tag)
    }

    /// True when the instance is drawn at all (`Enable`, REQ-E3D-191).
    #[must_use]
    pub fn enable(&self) -> bool {
        self.style != RenderStyle::Hidden
    }

    /// Sets `Enable`: Normal when true, Hidden when false (REQ-E3D-191).
    pub fn set_enable(&mut self, value: bool) {
        self.style = if value {
            RenderStyle::Normal
        } else {
            RenderStyle::Hidden
        };
    }

    /// True when a deformation is active (SPEC-04B REQ-E3D-568).
    #[must_use]
    pub fn deform_enabled(&self) -> bool {
        self.deform_amplitude.x != 0.0
            || self.deform_amplitude.y != 0.0
            || self.deform_amplitude.z != 0.0
    }

    /// One of the eight free script values (`CData`, SPEC-05 REQ-LUA-082): indices 0 to 3 are
    /// row 3 of `deform_speed`, 4 and up row 3 of `deform_offset` (column `index & 3`).
    #[must_use]
    pub fn custom_data(&self, index: usize) -> f32 {
        if index < 4 {
            self.deform_speed[(3, index)]
        } else {
            self.deform_offset[(3, index & 3)]
        }
    }

    /// Writes one of the eight free script values.
    pub fn set_custom_data(&mut self, index: usize, value: f32) {
        if index < 4 {
            self.deform_speed[(3, index)] = value;
        } else {
            self.deform_offset[(3, index & 3)] = value;
        }
    }

    /// The effective mesh algorithm (SPEC-04B REQ-E3D-500): `rm` 1, 2 or 3 wins, anything else
    /// (0 included) takes the global setting.
    #[must_use]
    pub const fn effective_algorithm(&self, global: RenderAlgorithm) -> RenderAlgorithm {
        match self.mesh_algorithm {
            1 => RenderAlgorithm::Greedy,
            2 => RenderAlgorithm::Marching,
            3 => RenderAlgorithm::GreedyMulticolor,
            _ => global,
        }
    }

    /// Copies every 3D parameter from `source`, resets the custom palette to −1 and shares
    /// the tags and the texture with it (REQ-E3D-190). The script handle stays this object's.
    pub fn copy_from(&mut self, source: &Self) {
        self.depth = source.depth;
        self.geometry = source.geometry;
        self.anti_shadow = source.anti_shadow;
        self.flip = source.flip;
        self.solid = source.solid;
        self.full_warp = source.full_warp;
        self.layer = source.layer;
        self.scale = source.scale;
        self.rotation = source.rotation;
        self.offset = source.offset;
        self.pivot = source.pivot;
        self.alpha = source.alpha;
        self.mesh_algorithm = source.mesh_algorithm;
        self.deform_amplitude = source.deform_amplitude;
        self.deform_pivot = source.deform_pivot;
        self.deform_speed = source.deform_speed;
        self.deform_offset = source.deform_offset;
        self.style = source.style;
        self.empty_color = source.empty_color;
        self.tex_offset = source.tex_offset;
        self.tex_tilting = source.tex_tilting;
        self.fill_rate = source.fill_rate;
        self.cast_shadow = source.cast_shadow;
        self.receive_shadow = source.receive_shadow;
        self.texture.clone_from(&source.texture);
        self.texture_source = source.texture_source;
        self.custom_palette = -1;

        // Shared, not copied: the tag list and, through it, the script links.
        self.tags = Arc::clone(&source.tags);
        self.index = source.index;
    }

    /// Reads the 3D parameters of a profile's 3D variant (SPEC-03 §4.4), applying the
    /// defaults of REQ-PROF-034 for the sub-messages a file leaves out.
    #[must_use]
    pub fn from_variant(variant: &PatternVariant) -> Self {
        let mut instance = Self {
            depth: variant.depth,
            geometry: Geo3DType::from_profile(variant.geometry),
            anti_shadow: variant.anti_shadow,
            flip: variant.flip,
            solid: variant.solid,
            full_warp: variant.full_warp,
            layer: ZLayer::from_profile(variant.layer),
            scale: to_vec3(variant.scale, Vec3::ONE),
            rotation: to_vec3(variant.rotation, Vec3::ZERO),
            offset: to_vec3(variant.offset, Vec3::ZERO),
            pivot: to_vec3(variant.pivot, Vec3::HALF),
            alpha: variant.alpha,
            mesh_algorithm: variant.mesh_algorithm.raw(),
            deform_amplitude: to_vec3(variant.deform_amplitude, Vec3::ZERO),
            deform_pivot: to_vec3(variant.deform_pivot, Vec3::HALF),
            style: RenderStyle::from_profile(variant.render_style),
            empty_color: variant.empty_color(),
            texture: variant
                .texture
                .as_ref()
                .filter(|texture| !texture.is_null_marker())
                .map(|texture| Arc::new(texture.clone())),
            ..Self::default()
        };

        copy_matrix(
            variant.deform_speed.as_ref(),
            &mut instance.deform_speed,
            0.0,
        );
        copy_matrix(
            variant.deform_offset.as_ref(),
            &mut instance.deform_offset,
            std::f32::consts::PI / 2.0,
        );
        instance.set_tag_string(variant.tag_string.as_deref().unwrap_or_default());
        instance
    }
}

fn to_vec3(value: Option<Vec3F>, fallback: Vec3) -> Vec3 {
    value.map_or(fallback, |v| Vec3::new(v.x, v.y, v.z))
}

fn copy_matrix(source: Option<&Mat4F>, destination: &mut Mat4, absent_rows_012: f32) {
    match source {
        // An absent matrix keeps the default of REQ-E3D-453: zero speeds, pi/2 phases on rows
        // 0 to 2 and 0 on row 3.
        None => set_rows_012(destination, absent_rows_012),
        Some(matrix) => {
            for (n, &cell) in matrix.cells().iter().enumerate() {
                destination[(n / 4, n % 4)] = cell;
            }
        }
    }
}
