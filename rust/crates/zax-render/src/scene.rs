//! Everything the renderer reads for one frame: the user settings of SPEC-06 §4.1, the light,
//! the backdrop, the camera and the per-frame data of the 3D engine.

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::frame::ShapeId;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::ConversionSettings;
use zax_engine3d::space_bend::SpaceBend;
use zax_profiles::values::Vec4F;

use crate::camera::SceneCamera;
use crate::geometry::{self, VisibleRect};
use crate::numerics::Vector3;
use crate::sky::SkyBodySettings;
use crate::style::StyleSettings;

/// Skybox setting, stored as an integer (SPEC-06 REQ-SCN-049).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SkyboxKind {
    /// Flat background, black outside a game (REQ-SCN-046).
    #[default]
    Default = -1,
    /// Flat background, always the custom colour.
    Color = 0,
    /// Gradient sky derived from the frame background colour (REQ-SCN-050).
    Gradient = 1,
    /// Nebula sky (REQ-SCN-051).
    Fantasy = 2,
}

/// The user settings of SPEC-06 §4.1 the renderer reads. Their persistence belongs to SPEC-09;
/// the bounds and defaults here are those of §4.1.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // The independent switches of SPEC-06 §4.1.
pub struct RenderSettings {
    light_intensity: f32,
    shadow_map_size: i32,
    anti_alias_samples: i32,
    anti_alias: i32,
    vsync: i32,
    target_frame_rate: i32,
    /// Vertical field of view in degrees, 45 to 95 by steps of 5 (REQ-SCN-005).
    pub fov: f32,
    /// Whether shadows are enabled at all (REQ-SCN-030).
    pub shadows: bool,
    /// Skybox setting (REQ-SCN-049).
    pub skybox: SkyboxKind,
    /// The custom background colour, black by default (REQ-SCN-046).
    pub custom_background_color: Rgb24,
    /// The sun, the moon and the clouds of the procedural skyboxes (AMD-024 REQ-PUB-003). All
    /// three are off by default, which is what REQ-SCN-050 asks for (Q-064).
    pub sky_bodies: SkyBodySettings,
    /// Render skip (REQ-SCN-060), enabled by default: when no new emulated frame has been
    /// produced since the last render and the game is running, the host may keep the previous
    /// image instead of drawing the scene again.
    pub skip_render: bool,
    /// Whether back faces are culled. Kept as a switch so that a capture can show the raw
    /// meshes; it has no effect on closed geometry (REQ-E3D-505).
    pub cull_back_faces: bool,
    /// Whether the backdrop of REQ-SCN-048 takes the almost-matt reflection model of AMD-049
    /// REQ-SCN-075 instead of the plain diffuse term of REQ-SCN-028 (`render.backdrop_lighting`).
    ///
    /// The spec default on desktop; off on handhelds, where it is a fragment cost on what can be
    /// the largest surface of the scene, and where REQ-SCN-048's own diffuse term — the priority
    /// clause T-1 of AMD-049 keeps it valid — costs nothing.
    pub backdrop_lighting: bool,
    /// The visual style of AMD-055 REQ-SCN-078 and its twelve parameters (`render.visual_style`
    /// and the twelve `render.style.*`), as the host pushes them when one of them changes.
    ///
    /// Classic with every parameter at its REQ-SCN-079 default is the default on every target,
    /// and is the state in which this amendment changes no pixel and costs nothing: the renderer
    /// tests the four predicates of [`StyleSettings`] once a frame and compiles no variant and
    /// allocates no target for an effect that is off.
    pub style: StyleSettings,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            light_intensity: 1.0,
            shadow_map_size: geometry::SHADOW_MAP_SIZE,
            anti_alias_samples: 1,
            anti_alias: 0,
            vsync: 0,
            target_frame_rate: 90,
            fov: geometry::DEFAULT_FOV,
            shadows: true,
            skybox: SkyboxKind::Default,
            custom_background_color: Rgb24::default(),
            sky_bodies: SkyBodySettings::default(),
            skip_render: true,
            cull_back_faces: true,
            backdrop_lighting: true,
            style: StyleSettings::default(),
        }
    }
}

impl RenderSettings {
    /// Light intensity, 0.50 to 1.50 by steps of 0.01 (REQ-SCN-026).
    #[must_use]
    pub const fn light_intensity(&self) -> f32 {
        self.light_intensity
    }

    /// Sets the light intensity, clamped to 0.5..1.5 (REQ-SCN-026).
    pub fn set_light_intensity(&mut self, value: f32) {
        self.light_intensity = value.clamp(0.5, 1.5);
    }

    /// Shadow map side in texels, 4096 by default (AMD-034 §2, replacing the 1024 of
    /// DEC-R2-D4). A size the hardware refuses is lowered by the renderer, not here.
    #[must_use]
    pub const fn shadow_map_size(&self) -> i32 {
        self.shadow_map_size
    }

    /// Sets the shadow map side, clamped to 64..8192 texels.
    pub fn set_shadow_map_size(&mut self, value: i32) {
        self.shadow_map_size = value.clamp(64, 8192);
    }

    /// Samples per pixel the scene is drawn with (AMD-032 §2): 1 for no anti-aliasing, 4 or 8
    /// for the two levels of `video.anti_alias`. The shadow map is never multisampled. A value
    /// the hardware refuses is lowered by the renderer, not here.
    #[must_use]
    pub const fn anti_alias_samples(&self) -> i32 {
        self.anti_alias_samples
    }

    /// Sets the samples per pixel, clamped to 1..8.
    ///
    /// This is the sample count **alone**, which is all the three levels of AMD-032 §2 need. The
    /// fourth level of AMD-060 is not a sample count, so a caller that has the setting should use
    /// [`Self::set_anti_alias`] instead and let it derive this.
    pub fn set_anti_alias_samples(&mut self, value: i32) {
        self.anti_alias_samples = value.clamp(1, geometry::MAX_ANTI_ALIAS_SAMPLES);
    }

    /// The `video.anti_alias` level itself, 0 to 3 (SPEC-09 §4.1.1 as AMD-060 §2 extends it).
    ///
    /// The renderer needs the **level** and not only [`Self::anti_alias_samples`], because level 3
    /// and level 0 both draw with one sample and only one of them runs the filter of REQ-SCN-085.
    #[must_use]
    pub const fn anti_alias(&self) -> i32 {
        self.anti_alias
    }

    /// Sets the `video.anti_alias` level and, with it, the samples per pixel the level asks for
    /// (`geometry::anti_alias_samples`) — so that the two can never disagree.
    pub fn set_anti_alias(&mut self, level: i32) {
        self.anti_alias = level;
        self.set_anti_alias_samples(geometry::anti_alias_samples(level));
    }

    /// `VSync` (REQ-SCN-061, SPEC-09): 0 off, 1 every refresh, 2 every other refresh. Applying it
    /// is the host's business; the renderer accepts any rate.
    #[must_use]
    pub const fn vsync(&self) -> i32 {
        self.vsync
    }

    /// Sets `VSync`, clamped to 0..2.
    pub fn set_vsync(&mut self, value: i32) {
        self.vsync = value.clamp(0, 2);
    }

    /// Target frame rate (REQ-SCN-061): 60, 90 or 120 frames per second, 90 by default.
    #[must_use]
    pub const fn target_frame_rate(&self) -> i32 {
        self.target_frame_rate
    }

    /// Sets the target frame rate; any value other than 60, 90 or 120 comes back as 90.
    pub fn set_target_frame_rate(&mut self, value: i32) {
        self.target_frame_rate = if matches!(value, 60 | 90 | 120) {
            value
        } else {
            90
        };
    }

    /// Whether a skybox is drawn instead of a flat background, which changes the background
    /// colour rules of REQ-SCN-046.
    #[must_use]
    pub const fn skybox_drawn(&self) -> bool {
        matches!(self.skybox, SkyboxKind::Gradient | SkyboxKind::Fantasy)
    }
}

/// The directional light of SPEC-06 §3.9: a profile setting that Lua can change during a game
/// (REQ-SCN-025, REQ-SCN-026).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightState {
    /// Propagation direction of the rays, normalised at use. The default `(1, −1, 1)` is light
    /// coming from above, from the left and from the camera side (REQ-SCN-025).
    pub direction: Vector3,
    /// Light colour, white by default (REQ-SCN-025).
    pub color: Rgb24,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            direction: Vector3::new(1.0, -1.0, 1.0),
            color: Rgb24::new(255, 255, 255),
        }
    }
}

impl LightState {
    /// Puts the profile defaults back (REQ-SCN-025).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The backdrop of SPEC-06 REQ-SCN-048: an optional axis-aligned box driven only by Lua. Every
/// property goes back to its default at the end of each game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropState {
    /// Whether the box is displayed; false by default.
    pub enabled: bool,
    /// Colour of the box, black by default.
    pub color: Rgb24,
    /// Size of the box, `(256, 240, 1)` by default.
    pub size: Vector3,
    /// Position of the box centre relative to `C`, zero by default.
    pub offset: Vector3,
    /// Whether the box receives shadows; true by default.
    pub receive_shadow: bool,
}

impl Default for BackdropState {
    fn default() -> Self {
        Self {
            enabled: false,
            color: Rgb24::default(),
            size: Vector3::new(256.0, 240.0, 1.0),
            offset: Vector3::ZERO,
            receive_shadow: true,
        }
    }
}

impl BackdropState {
    /// Puts every property back to its default (REQ-SCN-048).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The state of the scene the host keeps from frame to frame: the settings, the camera, the
/// clip margins, the light and the backdrop.
///
/// The host owns this value and mutates it between frames; the renderer only reads it, holds no
/// process-wide state and never assumes it owns the window (REQ-SCN-057, REQ-ARCH-003). What
/// belongs to one displayed frame only — the tile list and the pixel picture, which the 3D
/// engine and the core own — is lent to each render through [`SceneFrame`].
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // The independent flags of SPEC-06 §3.13 and §3.16.
pub struct SceneState {
    /// The user settings of §4.1.
    pub settings: RenderSettings,
    /// The directional light.
    pub light: LightState,
    /// The backdrop.
    pub backdrop: BackdropState,
    /// The camera.
    pub camera: SceneCamera,
    /// The curvature of the scene (AMD-043 REQ-SCN-067), built from the four settings of
    /// REQ-SCN-066 when one of them changes. The identity by default, and the renderer tests
    /// `is_identity` once per frame: at the default settings the curvature costs the CPU one
    /// comparison and the GPU nothing at all, the flat shader variants staying in use.
    pub bend: SpaceBend,
    /// Whether the flat rectangle of the 2D mode bends with the scene (AMD-046 REQ-SCN-074).
    /// True by default; in 3D mode it has no effect.
    pub bend_two_dimensional_mode: bool,
    /// The four clip margins `(c0, c1, c2, c3)` in scene units (REQ-SCN-003, DEC-R2-T1). They
    /// come from the profile at game load and Lua can change them during the game.
    pub clipping: Vec4F,
    /// Widescreen margin of the displayed frame, in NES pixels on each side (0 for a standard
    /// frame). The visible rectangle reaches this far past both sides of the 256-pixel picture,
    /// and the left and right clip margins count from those wider edges.
    pub margin: f32,
    /// Vertical widescreen margin of the displayed frame, in NES pixels above and below: the
    /// top and bottom clip margins count from those taller edges.
    pub margin_y: f32,
    /// Whether a game is running. Outside that state the 3D scene is not displayed: only the
    /// background or the skybox stays visible behind the UI (REQ-SCN-047).
    pub game_running: bool,
    /// The profile setting `Setting.BgColor`, forced to true at every profile load
    /// (REQ-SCN-046); Lua can change it afterwards.
    pub use_nes_background_color: bool,
    /// The universal NES background colour of the **displayed** frame, entry 0 of the palette
    /// (REQ-SCN-046, REQ-SCN-052).
    pub nes_background_color: Rgb24,
    /// True when the "2D Mode" profile is loaded, that is when the profile holds no pattern at
    /// all (REQ-SCN-055). The scene then shows the core's pixel frame on the NES plane instead
    /// of the recomposed tiles (AMD-009, Q-022).
    pub two_dimensional_mode: bool,
    /// In the 2D mode, the picture is seen head on and fills the height of the view
    /// ([`crate::camera::SceneCamera::picture_view`]) instead of through the camera (Rust-only,
    /// owner request: the "2D from the front" of the handheld).
    pub picture_head_on: bool,
    /// The tilt of the head-on picture, `(rx, ry)` in degrees: the pull of a camera rotation
    /// before the picture gives way to the camera (Rust-only, owner request).
    pub picture_tilt: [f32; 2],
    /// The sides of the 2D picture the head-on view leaves out besides the profile's clipping,
    /// `(left, bottom, right, top)` in scene units: the overscan in Resize (Rust-only, owner
    /// request), the rest enlarged to fill the view.
    pub picture_crop: Vec4F,
    /// The shape the subjective camera follows, or `None` (REQ-SCN-021). In FPV it decides
    /// which tiles count as occluders and are drawn at half opacity (REQ-SCN-022).
    pub followed_shape: Option<ShapeId>,
    /// Real time elapsed since the renderer started, in seconds. The star twinkle of
    /// REQ-SCN-050 uses it; it never follows the emulated clock (REQ-SCN-059).
    pub real_time: f32,
    /// The frame time in seconds the animated deformation uses: emulated frames divided by 60
    /// (REQ-SCN-043, SPEC-04B REQ-E3D-568). It freezes while the game is paused.
    pub emulated_time: f32,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            settings: RenderSettings::default(),
            light: LightState::default(),
            backdrop: BackdropState::default(),
            camera: SceneCamera::new(),
            bend: SpaceBend::NONE,
            bend_two_dimensional_mode: true,
            clipping: ConversionSettings::DEFAULT_CLIPPING,
            margin: 0.0,
            margin_y: 0.0,
            game_running: false,
            use_nes_background_color: true,
            nes_background_color: Rgb24::default(),
            two_dimensional_mode: false,
            picture_head_on: false,
            picture_tilt: [0.0; 2],
            picture_crop: Vec4F::new(0.0, 0.0, 0.0, 0.0),
            followed_shape: None,
            real_time: 0.0,
            emulated_time: 0.0,
        }
    }
}

impl SceneState {
    /// The background colour of REQ-SCN-046, recomputed every rendered frame from the current
    /// settings and the displayed frame's palette.
    #[must_use]
    pub fn background_color(&self) -> Rgb24 {
        resolve_background(
            self.settings.skybox,
            self.game_running,
            self.use_nes_background_color,
            self.nes_background_color,
            self.settings.custom_background_color,
        )
    }

    /// The curvature this frame actually applies (AMD-043 REQ-SCN-067 as AMD-046 REQ-SCN-074
    /// amends it): [`Self::bend`], or the identity in 2D mode when "Bend 2D Mode" is off.
    ///
    /// The single home of that rule. The renderer reads it to pick the shader variant, and the
    /// host reads it to aim the Zapper (REQ-INP-124, which REQ-SCN-074 switches off for a flat
    /// 2D mode: the aim is then the one of REQ-INP-063, as at curvature 0).
    #[must_use]
    pub fn effective_bend(&self) -> SpaceBend {
        if self.two_dimensional_mode && !self.bend_two_dimensional_mode {
            SpaceBend::NONE
        } else {
            self.bend
        }
    }

    /// The visible rectangle of the current clip margins (REQ-SCN-039).
    #[must_use]
    pub fn visible_rectangle(&self) -> VisibleRect {
        VisibleRect::from_clipping(self.view_clipping())
    }

    /// The clip margins measured from the edges of the displayed picture: the profile's
    /// margins, with the left and right ones moved out by the widescreen margin (a negative
    /// margin reaches past the 256-pixel picture).
    #[must_use]
    pub fn view_clipping(&self) -> Vec4F {
        Vec4F::new(
            self.clipping.x - self.margin,
            self.clipping.y - self.margin_y,
            self.clipping.z - self.margin,
            self.clipping.w - self.margin_y,
        )
    }

    /// The clipping of the 2D picture's head-on view: the profile's, each side at least the
    /// overscan's in Resize ([`Self::picture_crop`]).
    #[must_use]
    pub fn picture_clipping(&self) -> Vec4F {
        Vec4F::new(
            self.clipping.x.max(self.picture_crop.x),
            self.clipping.y.max(self.picture_crop.y),
            self.clipping.z.max(self.picture_crop.z),
            self.clipping.w.max(self.picture_crop.w),
        )
    }
}

/// What one displayed frame lends the renderer: the tile list the 3D engine produced
/// (SPEC-04B REQ-E3D-641) and the pixel frame of the core.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneFrame<'a> {
    /// The tile list of the displayed frame, or `None` when there is nothing to draw.
    pub draw_list: Option<&'a FrameDrawList>,
    /// The 256 × 240 pixel frame the core produced for the **displayed** frame, one NES colour
    /// index per pixel, row 0 first (SPEC-02 REQ-CORE-048). Only read in 2D mode.
    pub nes_picture: &'a [u8],
    /// The rows of `nes_picture`: 240, more with a vertical widescreen margin (0 reads as 240).
    pub nes_picture_height: usize,
}

impl<'a> SceneFrame<'a> {
    /// A frame that only carries a tile list.
    #[must_use]
    pub const fn with_draw_list(draw_list: &'a FrameDrawList) -> Self {
        Self {
            draw_list: Some(draw_list),
            nes_picture: &[],
            nes_picture_height: 0,
        }
    }
}

/// The background colour table of REQ-SCN-046.
#[must_use]
pub fn resolve_background(
    skybox: SkyboxKind,
    game_running: bool,
    use_nes_background: bool,
    nes_background: Rgb24,
    custom: Rgb24,
) -> Rgb24 {
    if skybox == SkyboxKind::Color {
        return custom;
    }

    if !game_running {
        return Rgb24::default();
    }

    if use_nes_background {
        nes_background
    } else {
        custom
    }
}
