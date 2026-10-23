//! The settings the conversion pipeline consumes (SPEC-04 §4.2) and the enumerations of the
//! 3D parameters (SPEC-04 §4.3).

use zax_profiles::enums::{LayerAlignment, ProfileLayer, VariantGeometry};
use zax_profiles::settings::{LayerSettingsSet, ProfileSettings};
use zax_profiles::values::Vec4F;

use crate::palette::{FramePalette, Rgb24};

/// Depth layer of an instance (SPEC-04 §4.3, SPEC-04B REQ-E3D-402).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZLayer {
    /// Not identified; resolved per shape by REQ-E3D-205.
    Unidentified = 0,
    /// Game interface.
    Ui = 1,
    /// First depth layer.
    #[default]
    L1 = 2,
    /// Second depth layer.
    L2 = 3,
    /// Third depth layer.
    L3 = 4,
    /// Fourth depth layer.
    L4 = 5,
}

impl ZLayer {
    /// The layer of a stored integer. A value outside 0 to 5, which the C# engine could not
    /// index its layer table with, is taken as Unidentified.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Ui,
            2 => Self::L1,
            3 => Self::L2,
            4 => Self::L3,
            5 => Self::L4,
            _ => Self::Unidentified,
        }
    }

    /// The layer of a profile variant (SPEC-03 §4.4).
    #[must_use]
    pub const fn from_profile(layer: ProfileLayer) -> Self {
        Self::from_raw(layer.raw())
    }

    /// The index of the layer in the layer table, 0 to 5.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Depth alignment of a layer (REQ-E3D-205, SPEC-04B REQ-E3D-403).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZAlignment {
    /// Front faces aligned; coefficient 1.
    #[default]
    Front = 0,
    /// Centred; coefficient 0.
    Center = 1,
    /// Back faces aligned; coefficient −1.
    Back = 2,
    /// A stored value outside the three named ones; coefficient 1 like C#'s default arm.
    Other = 3,
}

impl ZAlignment {
    /// The alignment of a profile layer setting (SPEC-03 §4.6).
    #[must_use]
    pub const fn from_profile(alignment: LayerAlignment) -> Self {
        match alignment {
            LayerAlignment::Front => Self::Front,
            LayerAlignment::Center => Self::Center,
            LayerAlignment::Back => Self::Back,
            LayerAlignment::Other(_) => Self::Other,
        }
    }
}

/// Geometry family of an instance (SPEC-04 §4.3, semantics in SPEC-04B §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Geo3DType {
    /// Extrusion of the pattern image.
    #[default]
    Default = 0,
    /// Cylinder with a horizontal axis.
    HCylinder = 1,
    /// Cylinder with a vertical axis.
    VCylinder = 2,
    /// Textured box.
    Cube = 3,
    /// Half cylinder with a horizontal axis.
    HalfHCylinder = 4,
    /// Half cylinder with a vertical axis.
    HalfVCylinder = 5,
}

impl Geo3DType {
    /// The geometry of a stored integer; any other value builds as Default, which is what the
    /// default arm of the construction does with it (SPEC-04B §3.3).
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::HCylinder,
            2 => Self::VCylinder,
            3 => Self::Cube,
            4 => Self::HalfHCylinder,
            5 => Self::HalfVCylinder,
            _ => Self::Default,
        }
    }

    /// The geometry of a profile variant (SPEC-03 §4.4).
    #[must_use]
    pub const fn from_profile(geometry: VariantGeometry) -> Self {
        Self::from_raw(geometry.raw())
    }

    /// True for the four cylindrical geometry types (REQ-E3D-444).
    #[must_use]
    pub const fn is_cylinder(self) -> bool {
        matches!(
            self,
            Self::VCylinder | Self::HCylinder | Self::HalfVCylinder | Self::HalfHCylinder
        )
    }
}

/// Mesh algorithm (REQ-E3D-340; geometry in SPEC-04B §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderAlgorithm {
    /// Rectangular faces merged without looking at colour.
    #[default]
    Greedy = 0,
    /// Smoothed surface, marching cubes.
    Marching = 1,
    /// Greedy meshing limited to equal colours.
    GreedyMulticolor = 2,
}

/// Parameters of one depth layer (REQ-E3D-205, SPEC-04B REQ-E3D-403).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayerSetting {
    /// Depth offset in scene units.
    pub offset: f32,
    /// Depth alignment.
    pub alignment: ZAlignment,
    /// Whether the layer lets its objects cast shadows (SPEC-04B REQ-E3D-567).
    pub cast_shadow: bool,
    /// Whether the layer lets its objects receive shadows.
    pub receive_shadow: bool,
}

impl LayerSetting {
    /// The alignment coefficient `a`: 1 front, 0 centre, −1 back.
    #[must_use]
    pub const fn coefficient(&self) -> f32 {
        match self.alignment {
            ZAlignment::Center => 0.0,
            ZAlignment::Back => -1.0,
            ZAlignment::Front | ZAlignment::Other => 1.0,
        }
    }
}

/// Number of layers: one per [`ZLayer`] value.
pub const LAYER_COUNT: usize = 6;

/// The settings the conversion pipeline consumes (SPEC-04 §4.2), with the factory defaults of
/// that table.
///
/// The settings model itself belongs to SPEC-09; this is the slice the pipeline reads, so that
/// the engine stays independent of the persistence layer (REQ-ARCH-002, REQ-ARCH-021). A
/// profile supplies some of these values (SPEC-03); [`ConversionSettings::apply_profile`]
/// does that.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // The flags of the SPEC-04 §4.2 table.
pub struct ConversionSettings {
    layers: [LayerSetting; LAYER_COUNT],
    /// Clip margins `(left, bottom, right, top)` in pixels, indices 0 to 3 of the profile's
    /// vector 4 and of the Lua object (DEC-R2-T1). Default `(0.1, 8.1, 8.1, 0.1)`.
    pub clipping: Vec4F,
    /// Whether an empty background tile is kept as an ordinary tile (REQ-E3D-004).
    pub empty_tile: bool,
    /// Germination order of background tiles: 0 = by y then x, 1 = by x then y (REQ-E3D-006).
    pub scan_direction: i32,
    /// Whether background growth requires an equal palette attribute (REQ-E3D-090).
    pub same_background_palette: bool,
    /// Whether sprite pass S2 requires an equal palette attribute (REQ-E3D-131).
    pub same_sprite_palette: bool,
    /// Adjacency tolerance in pixels of sprite pass S2 (REQ-E3D-130, REQ-E3D-131).
    pub sprite_tolerance: i32,
    /// Global mesh algorithm (REQ-E3D-340). A user setting: it is never taken from a profile.
    pub render_algorithm: RenderAlgorithm,
    /// Layer given to a background shape whose instance layer is Unidentified.
    pub default_layer: ZLayer,
    /// Kept for data compatibility; without effect (REQ-E3D-342).
    pub auto_layer: bool,
    /// Whether shadows are enabled at all (SPEC-04B REQ-E3D-567).
    pub shadow: bool,
    /// Whether the scene background uses the NES background colour (REQ-E3D-254).
    pub use_nes_background_color: bool,
    /// The custom scene background colour used when `use_nes_background_color` is false.
    pub custom_background_color: Rgb24,
}

impl Default for ConversionSettings {
    fn default() -> Self {
        Self {
            layers: default_layers(),
            clipping: Self::DEFAULT_CLIPPING,
            empty_tile: false,
            scan_direction: 0,
            same_background_palette: false,
            same_sprite_palette: false,
            sprite_tolerance: 0,
            render_algorithm: RenderAlgorithm::Greedy,
            default_layer: ZLayer::L1,
            auto_layer: false,
            shadow: true,
            use_nes_background_color: true,
            custom_background_color: Rgb24::default(),
        }
    }
}

impl ConversionSettings {
    /// The factory clip margins of SPEC-04 §4.2.
    pub const DEFAULT_CLIPPING: Vec4F = Vec4F::new(0.1, 8.1, 8.1, 0.1);

    /// Parameters of layer `layer`.
    #[must_use]
    pub const fn layer(&self, layer: ZLayer) -> &LayerSetting {
        &self.layers[layer.index()]
    }

    /// Parameters of layer `layer`, for writing.
    pub fn layer_mut(&mut self, layer: ZLayer) -> &mut LayerSetting {
        &mut self.layers[layer.index()]
    }

    /// Applies the settings a profile carries (SPEC-03 §4.6) to the pipeline slice.
    ///
    /// Only the keys REQ-PROF-041 lists as applied are taken: the clip margins, the five
    /// conversion settings, the layers — with the shadow permissions reset per REQ-PROF-042 —
    /// and the NES background flag, forced to true. The global mesh algorithm (REQ-E3D-340),
    /// the shadow switch and the custom background colour are user settings and a profile
    /// load leaves them alone.
    pub fn apply_profile(&mut self, settings: Option<&ProfileSettings>) {
        let Some(settings) = settings else {
            return;
        };

        if let Some(clipping) = settings.clipping {
            self.clipping = clipping;
        }

        self.empty_tile = settings.empty_tile;
        self.scan_direction = settings.scan_direction;
        self.same_background_palette = settings.same_background_palette;
        self.same_sprite_palette = settings.same_sprite_palette;
        self.sprite_tolerance = settings.sprite_tolerance;
        self.use_nes_background_color = true;

        if let Some(layers) = &settings.layers {
            self.default_layer = ZLayer::from_profile(layers.default_layer);
            self.auto_layer = layers.auto;
            for (i, (target, source)) in self.layers.iter_mut().zip(&layers.layers).enumerate() {
                target.offset = source.offset;
                target.alignment = ZAlignment::from_profile(source.aligned);

                // REQ-PROF-042: whatever the file stores, after a load layer 2 casts and
                // layers 2 and up receive.
                target.cast_shadow = LayerSettingsSet::effective_cast_shadow(i);
                target.receive_shadow = LayerSettingsSet::effective_recv_shadow(i);
            }
        }
    }

    /// The scene background colour of the frame whose palette is given (REQ-E3D-254).
    #[must_use]
    pub fn background_color(&self, palette: &FramePalette, skybox_active: bool) -> Rgb24 {
        if self.use_nes_background_color && !skybox_active {
            palette.universal_color()
        } else {
            self.custom_background_color
        }
    }
}

fn default_layers() -> [LayerSetting; LAYER_COUNT] {
    // REQ-E3D-205 notes: UI offset 0 CENTER; L1 0, L2 12, L3 24, L4 36, all FRONT. Shadow
    // permissions: L1 casts and receives, L2..L4 receive only, UI neither (SPEC-04B
    // REQ-E3D-567).
    const OFFSETS: [f32; LAYER_COUNT] = [0.0, 0.0, 0.0, 12.0, 24.0, 36.0];
    std::array::from_fn(|i| LayerSetting {
        offset: OFFSETS[i],
        alignment: if i == ZLayer::Ui.index() {
            ZAlignment::Center
        } else {
            ZAlignment::Front
        },
        cast_shadow: i == ZLayer::L1.index(),
        receive_shadow: i >= ZLayer::L1.index(),
    })
}
