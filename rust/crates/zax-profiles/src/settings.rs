//! The settings message embedded in a profile (SPEC-03 §4.6, REQ-PROF-040 to REQ-PROF-044).

use crate::enums::{GlobalMeshAlgorithm, LayerAlignment, ProfileLayer, TvRegionMode};
use crate::layout::FieldMask;
use crate::pattern::RecordedPatternSlot;
use crate::values::{ArgbColor, Vec3F, Vec4F};

/// The light of the scene.
#[derive(Debug, Clone, Default)]
pub struct LightSettings {
    /// Direction, when stored.
    pub direction: Option<Vec3F>,
    /// Colour, when stored.
    pub color: Option<ArgbColor>,
    /// Intensity.
    pub intensity: f32,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

/// The parameters of one depth layer.
#[derive(Debug, Clone, Default)]
pub struct LayerSettings {
    /// Depth offset.
    pub offset: f32,
    /// Depth alignment.
    pub aligned: LayerAlignment,
    /// The cast-shadow flag as stored; the model resets it (REQ-PROF-042).
    pub stored_cast_shadow: bool,
    /// The receive-shadow flag as stored; the model resets it (REQ-PROF-042).
    pub stored_recv_shadow: bool,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl LayerSettings {
    /// The depth coefficient of the alignment (REQ-PERS-013): 1 front, 0 centre, -1 back.
    #[must_use]
    pub fn coefficient(&self) -> f32 {
        match self.aligned {
            LayerAlignment::Center => 0.0,
            LayerAlignment::Back => -1.0,
            LayerAlignment::Front | LayerAlignment::Other(_) => 1.0,
        }
    }
}

/// The six layer parameter sets, the default layer and the auto flag.
#[derive(Debug, Clone, Default)]
pub struct LayerSettingsSet {
    /// The layer parameter sets; a valid profile holds [`Self::LAYER_COUNT`].
    pub layers: Vec<LayerSettings>,
    /// Layer given to a variant that has none.
    pub default_layer: ProfileLayer,
    /// `Auto`.
    pub auto: bool,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl LayerSettingsSet {
    /// Number of layers a profile holds (REQ-PROF-043).
    pub const LAYER_COUNT: usize = 6;

    /// The layer `index`, or none outside 0 to 5.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&LayerSettings> {
        self.layers.get(index)
    }

    /// The cast-shadow flag the model applies to layer `index` (REQ-PROF-042).
    #[must_use]
    pub const fn effective_cast_shadow(index: usize) -> bool {
        index == 2
    }

    /// The receive-shadow flag the model applies to layer `index` (REQ-PROF-042).
    #[must_use]
    pub const fn effective_recv_shadow(index: usize) -> bool {
        index >= 2
    }

    /// The default layer set (REQ-PERS-002).
    #[must_use]
    pub fn create_default() -> Self {
        const OFFSETS: [f32; LayerSettingsSet::LAYER_COUNT] = [0.0, 0.0, 0.0, 12.0, 24.0, 36.0];
        let layers = OFFSETS
            .iter()
            .enumerate()
            .map(|(index, offset)| LayerSettings {
                offset: *offset,
                aligned: if index == 1 {
                    LayerAlignment::Center
                } else {
                    LayerAlignment::Front
                },
                stored_cast_shadow: Self::effective_cast_shadow(index),
                stored_recv_shadow: Self::effective_recv_shadow(index),
                recorded_fields: None,
            })
            .collect();
        Self {
            layers,
            default_layer: ProfileLayer::L1,
            auto: false,
            recorded_fields: None,
        }
    }
}

/// The settings message embedded in a profile (SPEC-03 §4.6). Defaults are those of an absent
/// field (REQ-PROF-040).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // one flag per stored field of the message
pub struct ProfileSettings {
    /// Light.
    pub light: Option<LightSettings>,
    /// `bgColor`: use the NES background colour.
    pub use_nes_background_color: bool,
    /// Custom background colour, when stored.
    pub custom_background_color: Option<ArgbColor>,
    /// Clipping box, when stored.
    pub clipping: Option<Vec4F>,
    /// `mapEnable`.
    pub map_enable: bool,
    /// Layers.
    pub layers: Option<LayerSettingsSet>,
    /// Graphic quality.
    pub graphic_quality: i32,
    /// VR runtime.
    pub vr_runtime: i32,
    /// V-sync.
    pub vsync: i32,
    /// Frame rate.
    pub frame_rate: i32,
    /// Game speed.
    pub game_speed: f32,
    /// Volume.
    pub volume: f32,
    /// Global mesh algorithm.
    pub mesh_algorithm: GlobalMeshAlgorithm,
    /// Memorised shape slots.
    pub saved_patterns: Vec<RecordedPatternSlot>,
    /// Memorised deformation slots.
    pub saved_anim_patterns: Vec<RecordedPatternSlot>,
    /// Control method.
    pub control_method: i32,
    /// Shadow.
    pub shadow: bool,
    /// Same palette for backgrounds.
    pub same_background_palette: bool,
    /// Sprite tolerance.
    pub sprite_tolerance: i32,
    /// Scan direction.
    pub scan_direction: i32,
    /// Skybox, -1 for none.
    pub skybox: i32,
    /// Core.
    pub core: i32,
    /// Mirroring.
    pub mirroring: bool,
    /// Empty tile.
    pub empty_tile: bool,
    /// Local camera.
    pub local_camera: bool,
    /// Scene ratio.
    pub scene_ratio: f32,
    /// Emulated menu button.
    pub emulated_menu_button: bool,
    /// Render method.
    pub render_method: i32,
    /// Resolution scale index.
    pub resolution_scale_index: i32,
    /// Skip frame.
    pub skip_frame: bool,
    /// Background music.
    pub background_music: bool,
    /// Anti-aliasing.
    pub anti_alias: i32,
    /// Automatic TV mode.
    pub auto_tv_mode: bool,
    /// Video region.
    pub tv_mode: TvRegionMode,
    /// TV box (retired by AMD-031, still stored).
    pub tv_box: i32,
    /// Sort option.
    pub sort_option: i32,
    /// Same palette for sprites.
    pub same_sprite_palette: bool,
    /// Field of view.
    pub fov: f32,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            light: None,
            use_nes_background_color: false,
            custom_background_color: None,
            clipping: None,
            map_enable: false,
            layers: None,
            graphic_quality: 0,
            vr_runtime: 0,
            vsync: 0,
            frame_rate: 0,
            game_speed: 0.0,
            volume: 0.0,
            mesh_algorithm: GlobalMeshAlgorithm::default(),
            saved_patterns: Vec::new(),
            saved_anim_patterns: Vec::new(),
            control_method: 0,
            shadow: false,
            same_background_palette: false,
            sprite_tolerance: 0,
            scan_direction: 0,
            skybox: -1,
            core: 0,
            mirroring: true,
            empty_tile: false,
            local_camera: false,
            scene_ratio: 1.0,
            emulated_menu_button: false,
            render_method: 0,
            resolution_scale_index: 0,
            skip_frame: true,
            background_music: true,
            anti_alias: 0,
            auto_tv_mode: false,
            tv_mode: TvRegionMode::default(),
            tv_box: 0,
            sort_option: 0,
            same_sprite_palette: false,
            fov: 70.0,
            recorded_fields: None,
        }
    }
}
