//! The visual style of AMD-055 REQ-SCN-078 and its twelve parameters of REQ-SCN-079, with the
//! normative quantities the amendment derives from them.
//!
//! Platform-free on purpose: every number below is a fact of the specification, so it is written
//! once, unit-tested without a GL context, and read by the renderer where it is needed — never
//! recomputed per tile or per fragment. The renderer holds one [`StyleSettings`] value, which the
//! host replaces when a setting changes (SPEC-09 §4.1: scope U, effect immediate).
//!
//! **What is not here is as much the design as what is.** There is no "style" uniform: Classic and
//! Enhanced-with-every-effect-off have to be the same picture *and the same cost* (REQ-SCN-080),
//! so what the effects need is compiled into shader variants and the screen passes simply do not
//! run. The predicates below ([`StyleSettings::voxel_grid_active`] and its three siblings) are
//! what the renderer tests, once a frame, to decide which variant draws and which pass runs.

/// The style identifier of Classic (AMD-040 §3, AMD-055 REQ-SCN-078).
pub const CLASSIC: i32 = 0;

/// The style identifier of Enhanced, the one `Setting.VisualStyle` reads back (REQ-SCN-078).
pub const ENHANCED: i32 = 4;

/// The other identifier REQ-SCN-078 accepts for Enhanced in a write. AMD-040 wrote the style 1;
/// 3dSen 2.0 numbers it 4.
pub const ENHANCED_ALIAS: i32 = 1;

/// The ambient-occlusion mode of `spl.ao` (REQ-SCN-079).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AmbientOcclusion {
    /// 0 — off, the default.
    #[default]
    Off = 0,
    /// 1 — the multi-scale occlusion of REQ-SCN-083's *Mode MSVO*. Rendered exactly as
    /// [`Self::Depth`] until that port exists: the declared fallback **(D)** of REQ-SCN-083, the
    /// label staying exposed and read back.
    Msvo = 1,
    /// 2 — the hemisphere occlusion of REQ-SCN-083 reconstructed from depth.
    Depth = 2,
}

impl AmbientOcclusion {
    /// The mode `value` names; anything outside 0..2 is off (the domain of REQ-PERS-003 already
    /// bounds the setting, so this is the belt).
    #[must_use]
    pub const fn of(value: i32) -> Self {
        match value {
            1 => Self::Msvo,
            2 => Self::Depth,
            _ => Self::Off,
        }
    }
}

/// The style and its twelve parameters, as one value the host pushes when any of them changes.
///
/// The four curvature parameters of AMD-043 REQ-SCN-066 are **not** here: REQ-SCN-078 keeps the
/// curvature outside the style — it acts in Classic as in Enhanced **(D)** — so it stays in
/// [`crate::scene::SceneState::bend`] where AMD-043 put it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleSettings {
    /// Whether the Enhanced style of §2.2 to §2.5 is selected (`render.visual_style` = 4, or the
    /// accepted 1). Classic is the rendering of SPEC-06, unchanged.
    pub enhanced: bool,
    /// `spl.ao`.
    pub ao: AmbientOcclusion,
    /// `ao.strength`, 1 to 5 (default 3).
    pub ao_strength: i32,
    /// `ao.radius`, 1 to 5 (default 3).
    pub ao_radius: i32,
    /// `ao.thickness`, 1 to 5 (default 3). Without effect outside an effective MSVO mode
    /// (REQ-SCN-079), which under the fallback **(D)** of REQ-SCN-083 is always.
    pub ao_thickness: i32,
    /// `ao.quality`, 0 Low, 1 Medium (default), 2 High.
    pub ao_quality: i32,
    /// `spl.pixel_shadow` (default off).
    pub pixel_shadow: bool,
    /// `spl.voxel_grid`, 0 off (default), 1, 2 or 3 — a cell side of `2^value` voxels.
    pub voxel_grid: i32,
    /// `spl.reflections` (default off).
    pub reflections: bool,
    /// `ssr.shine`, 1 to 5 (default 3).
    pub ssr_shine: i32,
    /// `ssr.clarity`, 1 to 5 (default 3).
    pub ssr_clarity: i32,
    /// `ssr.length`, 1 to 5 (default 3).
    pub ssr_length: i32,
    /// `ssr.quality`, 0 Low, 1 Medium, 2 High (default), 3 Ultra.
    pub ssr_quality: i32,
}

impl Default for StyleSettings {
    /// The defaults of REQ-SCN-079, with Classic as the style: the state in which this amendment
    /// changes no pixel at all.
    fn default() -> Self {
        Self {
            enhanced: false,
            ao: AmbientOcclusion::Off,
            ao_strength: 3,
            ao_radius: 3,
            ao_thickness: 3,
            ao_quality: 1,
            pixel_shadow: false,
            voxel_grid: 0,
            reflections: false,
            ssr_shine: 3,
            ssr_clarity: 3,
            ssr_length: 3,
            ssr_quality: 2,
        }
    }
}

impl StyleSettings {
    /// The style identifier a read of `Setting.VisualStyle` gives back (REQ-SCN-078): 4 when
    /// Enhanced is active, 0 otherwise.
    #[must_use]
    pub const fn identifier(&self) -> i32 {
        if self.enhanced {
            ENHANCED
        } else {
            CLASSIC
        }
    }

    /// The style `value` selects: 4 and the accepted 1 are Enhanced, 0 and **every other value**
    /// are Classic (REQ-SCN-078, including a value that comes from the settings file).
    #[must_use]
    pub const fn style_of(value: i32) -> bool {
        value == ENHANCED || value == ENHANCED_ALIAS
    }

    /// Whether the voxel grid of REQ-SCN-081 modulates the albedo this frame.
    #[must_use]
    pub const fn voxel_grid_active(&self) -> bool {
        self.enhanced && self.voxel_grid != 0
    }

    /// Whether the shadow of an opaque tile is evaluated at the voxel centre (REQ-SCN-082).
    #[must_use]
    pub const fn pixel_shadow_active(&self) -> bool {
        self.enhanced && self.pixel_shadow
    }

    /// Whether the ambient-occlusion pass of REQ-SCN-083 runs this frame.
    #[must_use]
    pub const fn ao_active(&self) -> bool {
        self.enhanced && !matches!(self.ao, AmbientOcclusion::Off)
    }

    /// Whether the floor-reflection pass of REQ-SCN-084 runs this frame.
    #[must_use]
    pub const fn reflections_active(&self) -> bool {
        self.enhanced && self.reflections
    }

    /// Whether either screen-space pass runs, which is what decides whether the renderer owns a
    /// depth texture and the targets of the two passes at all.
    ///
    /// **This is the predicate the brief's point 2 rests on**: Enhanced with `spl.ao` = 0 and
    /// `spl.reflections` = 0 — the defaults of REQ-SCN-079 — answers false, so no target is
    /// allocated and no pass is submitted, and Enhanced then costs what Classic costs.
    #[must_use]
    pub const fn screen_effects_active(&self) -> bool {
        self.ao_active() || self.reflections_active()
    }

    /// The side of a grid cell in voxels, `N = 2^spl.voxel_grid` (REQ-SCN-081): 2, 4 or 8. 1 when
    /// the grid is off, a value no caller uses.
    #[must_use]
    pub fn voxel_cell(&self) -> i32 {
        1 << self.voxel_grid.clamp(0, 3)
    }

    /// The radius of influence of the occlusion in scene units (REQ-SCN-083):
    /// `(8 + 6 × (ao.radius − 1)) × scene scale`, so 8, 14, 20, 26 and 32 at a scale of 1.
    #[must_use]
    pub fn ao_radius_units(&self, scene_scale: f32) -> f32 {
        #[allow(clippy::cast_precision_loss)] // Five rungs.
        let steps = (self.ao_radius.clamp(1, 5) - 1) as f32;
        (8.0 + 6.0 * steps) * scene_scale
    }

    /// The darkening ceiling of REQ-SCN-083: `O ≤ min(0,95 ; 0,3 × ao.strength)`, so 0.3, 0.6,
    /// 0.9, 0.95 and 0.95 (v1.4, Q-097).
    #[must_use]
    pub fn ao_ceiling(&self) -> f32 {
        #[allow(clippy::cast_precision_loss)] // Five rungs.
        let strength = self.ao_strength.clamp(1, 5) as f32;
        (0.3 * strength).min(0.95)
    }

    /// Samples per pixel the occlusion takes, in the order of magnitude REQ-SCN-083 names: 6, 10
    /// and 16 for Low, Medium and High. Not observable — the aspect is what the requirement
    /// judges — so this is a SHOULD the implementation is free to read as a target.
    #[must_use]
    pub const fn ao_samples(&self) -> i32 {
        match self.ao_quality {
            0 => 6,
            2 => 16,
            _ => 10,
        }
    }

    /// Whether the occlusion is computed at half of each dimension of the render target
    /// (REQ-SCN-083: "sauf en High").
    #[must_use]
    pub const fn ao_half_resolution(&self) -> bool {
        self.ao_quality != 2
    }

    /// The fraction of the reflected colour mixed into the surface (REQ-SCN-084):
    /// `0,045 × n(ssr.shine)`, so 0.045, 0.135, 0.225, 0.315 and 0.405 (v1.5, Q-099).
    #[must_use]
    pub fn ssr_intensity(&self) -> f32 {
        0.045 * level_odd(self.ssr_shine)
    }

    /// The maximum length of a reflected ray in scene units (REQ-SCN-084):
    /// `40 × n(ssr.length) × scene scale`, so 40, 120, 200, 280 and 360 at a scale of 1.
    #[must_use]
    pub fn ssr_max_length(&self, scene_scale: f32) -> f32 {
        40.0 * level_odd(self.ssr_length) * scene_scale
    }

    /// The blur radius at the maximum ray length, as a fraction of the **screen height**
    /// (REQ-SCN-084): `0,003 × (10 − n(ssr.clarity))`, so 0.027, 0.021, 0.015, 0.009 and 0.003.
    /// The radius grows linearly with the distance travelled, from 0 at the ray's origin.
    #[must_use]
    pub fn ssr_blur_fraction(&self) -> f32 {
        0.003 * (10.0 - level_odd(self.ssr_clarity))
    }

    /// March steps the trace takes, in the order of magnitude REQ-SCN-084 names: 12, 24, 48 and
    /// 48 for Low, Medium, High and Ultra. Not observable.
    #[must_use]
    pub const fn ssr_steps(&self) -> i32 {
        match self.ssr_quality {
            0 => 12,
            1 => 24,
            _ => 48,
        }
    }

    /// Bisection refinements around the first depth crossing: 2, 3, 5 and 5. Not observable.
    #[must_use]
    pub const fn ssr_refinements(&self) -> i32 {
        match self.ssr_quality {
            0 => 2,
            1 => 3,
            _ => 5,
        }
    }

    /// Whether the trace runs at half of each dimension of the render target (REQ-SCN-084:
    /// "sauf en Ultra").
    #[must_use]
    pub const fn ssr_half_resolution(&self) -> bool {
        self.ssr_quality != 3
    }
}

/// `n(niveau) = 2 · niveau − 1` of REQ-SCN-084: levels 1 to 5 give 1, 3, 5, 7 and 9.
#[must_use]
fn level_odd(level: i32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // Five rungs.
    let level = level.clamp(1, 5) as f32;
    2.0 * level - 1.0
}

/// Half-width of a groove, as a fraction of a cell (REQ-SCN-081).
pub const GROOVE_HALF_WIDTH: f32 = 0.035;

/// Where the bright edge of REQ-SCN-081 reaches 1 again, as a fraction of a cell.
pub const BRIGHT_EDGE_END: f32 = 0.16;

/// The albedo factor the voxel grid of REQ-SCN-081 applies at the point `p` of a face of unit
/// normal `n`, both in the frame the grid is evaluated in (the mesh's local frame for a tile,
/// scene units for the backdrop), for a cell side of `cell`.
///
/// The CPU twin of `shaders/voxel_grid.glsl`, written first and checked against the amendment's
/// four-point table before any GPU code existed — the shape `color::backdrop_linear` took for
/// AMD-049. **The one clause it cannot carry** is the smoothing of the transition at
/// `d = 0.035`, which REQ-SCN-081 states in *screen* pixels: it is a screen-space quantity, so
/// the twin gives the unsmoothed value and the measured points of the criteria are all well
/// clear of that band.
#[must_use]
// The named quantities of REQ-SCN-081, spelled as the specification spells them (`p`, `n`,
// `m`, `t`, `k`, `e`, `w`, `d`): the idiom `geometry.rs` already uses for a formula of the spec.
#[allow(clippy::many_single_char_names)]
pub fn voxel_grid_factor(p: [f32; 3], n: [f32; 3], cell: f32) -> f32 {
    let an = [n[0].abs(), n[1].abs(), n[2].abs()];
    let m = an[0].max(an[1]).max(an[2]);
    let t = ((m - 0.8) / 0.1).clamp(0.0, 1.0);
    let k = 0.38 + 0.24 * t;
    let e = 0.20 - 0.10 * t;

    let mut factor = 1.0;
    for axis in 0..3 {
        // Axis weight: 0 perpendicular to the face, 1 in its plane.
        let w = (4.0 * (1.0 - an[axis])).clamp(0.0, 1.0);
        // Distance to the nearest cell edge, as a fraction of a cell.
        let cells = p[axis] / cell;
        let d = (cells - cells.round()).abs();
        factor *= if d <= GROOVE_HALF_WIDTH {
            1.0 - w * (1.0 - k)
        } else if d <= BRIGHT_EDGE_END {
            1.0 + w * e * (BRIGHT_EDGE_END - d) / (BRIGHT_EDGE_END - GROOVE_HALF_WIDTH)
        } else {
            1.0
        };
    }
    factor
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ-SCN-078: 4 and 1 are Enhanced, 0 and anything else Classic, and a read gives 4.
    #[test]
    fn req_scn_078_four_and_one_are_the_enhanced_style() {
        assert!(StyleSettings::style_of(4));
        assert!(StyleSettings::style_of(1));
        assert!(!StyleSettings::style_of(0));
        for other in [-1, 2, 3, 5, 7, 100] {
            assert!(!StyleSettings::style_of(other), "{other} is not Enhanced");
        }

        assert_eq!(CLASSIC, StyleSettings::default().identifier());
        let enhanced = StyleSettings {
            enhanced: true,
            ..StyleSettings::default()
        };
        assert_eq!(ENHANCED, enhanced.identifier());
    }

    /// REQ-SCN-079: the twelve defaults, which are also the state in which AMD-055 changes no
    /// pixel — eight of them are **not** zero, which is the note AMD-055 §3 point 2 adds.
    #[test]
    fn req_scn_079_the_twelve_defaults() {
        let style = StyleSettings::default();
        assert!(!style.enhanced);
        assert_eq!(AmbientOcclusion::Off, style.ao);
        assert_eq!(3, style.ao_strength);
        assert_eq!(3, style.ao_radius);
        assert_eq!(3, style.ao_thickness);
        assert_eq!(1, style.ao_quality);
        assert!(!style.pixel_shadow);
        assert_eq!(0, style.voxel_grid);
        assert!(!style.reflections);
        assert_eq!(3, style.ssr_shine);
        assert_eq!(3, style.ssr_clarity);
        assert_eq!(3, style.ssr_length);
        assert_eq!(2, style.ssr_quality);
    }

    /// The brief's point 2, as a predicate: Enhanced with the four switches at their defaults
    /// asks for no variant and no pass at all.
    #[test]
    fn req_scn_080_enhanced_with_every_effect_off_asks_for_nothing() {
        let style = StyleSettings {
            enhanced: true,
            ..StyleSettings::default()
        };
        assert!(!style.voxel_grid_active());
        assert!(!style.pixel_shadow_active());
        assert!(!style.ao_active());
        assert!(!style.reflections_active());
        assert!(!style.screen_effects_active());
    }

    /// And in Classic no switch can turn an effect on, whatever the parameters hold: the style is
    /// the gate, so a player who goes back to Classic keeps his parameters and pays nothing.
    #[test]
    fn req_scn_078_classic_turns_every_effect_off() {
        let style = StyleSettings {
            enhanced: false,
            ao: AmbientOcclusion::Depth,
            pixel_shadow: true,
            voxel_grid: 3,
            reflections: true,
            ..StyleSettings::default()
        };
        assert!(!style.voxel_grid_active());
        assert!(!style.pixel_shadow_active());
        assert!(!style.ao_active());
        assert!(!style.reflections_active());
        assert!(!style.screen_effects_active());
    }

    /// REQ-SCN-081: the cell side is `N = 2^spl.voxel_grid`, so 2, 4 and 8.
    #[test]
    fn req_scn_081_the_cell_side_is_two_to_the_setting() {
        for (setting, cell) in [(1, 2), (2, 4), (3, 8)] {
            let style = StyleSettings {
                voxel_grid: setting,
                ..StyleSettings::default()
            };
            assert_eq!(cell, style.voxel_cell(), "spl.voxel_grid = {setting}");
        }
    }

    /// REQ-SCN-083: the radius is 8, 14, 20, 26 and 32 scene units, and the scene scale
    /// multiplies it.
    #[test]
    fn req_scn_083_the_radius_of_the_five_rungs() {
        for (rung, radius) in [(1, 8.0), (2, 14.0), (3, 20.0), (4, 26.0), (5, 32.0)] {
            let style = StyleSettings {
                ao_radius: rung,
                ..StyleSettings::default()
            };
            assert!((style.ao_radius_units(1.0) - radius).abs() < 1e-5, "{rung}");
            assert!(
                (style.ao_radius_units(2.0) - 2.0 * radius).abs() < 1e-5,
                "{rung} at scale 2"
            );
        }
        assert!((StyleSettings::default().ao_radius_units(1.0) - 20.0).abs() < 1e-5);
    }

    /// REQ-SCN-083 as v1.4 raised it: the ceiling is `min(0,95 ; 0,3 × ao.strength)`, so 0.3,
    /// 0.6, 0.9, 0.95 and 0.95 — the two top rungs share it.
    #[test]
    fn req_scn_083_the_darkening_ceiling_of_the_five_rungs() {
        for (rung, ceiling) in [(1, 0.3), (2, 0.6), (3, 0.9), (4, 0.95), (5, 0.95)] {
            let style = StyleSettings {
                ao_strength: rung,
                ..StyleSettings::default()
            };
            assert!((style.ao_ceiling() - ceiling).abs() < 1e-6, "{rung}");
        }
        // The invariant of §2.4 reads the ceiling as a floor on the ratio: 1 − O.
        let one = StyleSettings {
            ao_strength: 1,
            ..StyleSettings::default()
        };
        assert!((1.0 - one.ao_ceiling() - 0.70).abs() < 1e-6);
        let five = StyleSettings {
            ao_strength: 5,
            ..StyleSettings::default()
        };
        assert!((1.0 - five.ao_ceiling() - 0.05).abs() < 1e-6);
    }

    /// REQ-SCN-084: the three parameterised quantities, each at its five rungs, and the defaults
    /// the amendment prints beside them (0.225, 200 units, 0.015 of the screen height).
    #[test]
    fn req_scn_084_the_three_parameterised_quantities() {
        let intensities = [0.045, 0.135, 0.225, 0.315, 0.405];
        let lengths = [40.0, 120.0, 200.0, 280.0, 360.0];
        let blurs = [0.027, 0.021, 0.015, 0.009, 0.003];
        for rung in 1..=5 {
            let index = usize::try_from(rung - 1).unwrap();
            let style = StyleSettings {
                ssr_shine: rung,
                ssr_length: rung,
                ssr_clarity: rung,
                ..StyleSettings::default()
            };
            assert!(
                (style.ssr_intensity() - intensities[index]).abs() < 1e-6,
                "shine {rung}"
            );
            assert!(
                (style.ssr_max_length(1.0) - lengths[index]).abs() < 1e-4,
                "length {rung}"
            );
            assert!(
                (style.ssr_max_length(3.0) - 3.0 * lengths[index]).abs() < 1e-3,
                "length {rung} at scale 3"
            );
            assert!(
                (style.ssr_blur_fraction() - blurs[index]).abs() < 1e-6,
                "clarity {rung}"
            );
        }

        let default = StyleSettings::default();
        assert!((default.ssr_intensity() - 0.225).abs() < 1e-6);
        assert!((default.ssr_max_length(1.0) - 200.0).abs() < 1e-4);
        assert!((default.ssr_blur_fraction() - 0.015).abs() < 1e-6);
        // "0,015, soit 16 pixels à 1080p": 0.015 × 1080 = 16.2.
        assert!((default.ssr_blur_fraction() * 1080.0 - 16.2).abs() < 0.01);
    }

    /// The quality rungs of the two effects: the orders of magnitude the amendment names, and the
    /// one clause of each that **is** observable — which pass runs at half resolution.
    #[test]
    fn req_scn_083_and_084_the_quality_rungs() {
        for (quality, samples, half) in [(0, 6, true), (1, 10, true), (2, 16, false)] {
            let style = StyleSettings {
                ao_quality: quality,
                ..StyleSettings::default()
            };
            assert_eq!(samples, style.ao_samples(), "ao.quality {quality}");
            assert_eq!(half, style.ao_half_resolution(), "ao.quality {quality}");
        }
        for (quality, steps, refinements, half) in [
            (0, 12, 2, true),
            (1, 24, 3, true),
            (2, 48, 5, true),
            (3, 48, 5, false),
        ] {
            let style = StyleSettings {
                ssr_quality: quality,
                ..StyleSettings::default()
            };
            assert_eq!(steps, style.ssr_steps(), "ssr.quality {quality}");
            assert_eq!(
                refinements,
                style.ssr_refinements(),
                "ssr.quality {quality}"
            );
            assert_eq!(half, style.ssr_half_resolution(), "ssr.quality {quality}");
        }
    }

    /// `spl.ao` reads its three values and nothing else.
    #[test]
    fn req_scn_079_the_three_occlusion_modes() {
        assert_eq!(AmbientOcclusion::Off, AmbientOcclusion::of(0));
        assert_eq!(AmbientOcclusion::Msvo, AmbientOcclusion::of(1));
        assert_eq!(AmbientOcclusion::Depth, AmbientOcclusion::of(2));
        for other in [-1, 3, 9] {
            assert_eq!(AmbientOcclusion::Off, AmbientOcclusion::of(other));
        }
    }
}
