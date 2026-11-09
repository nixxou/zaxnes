//! The rows of the catalogue, in the order of SPEC-09 §4.1. Each row carries the key, shape,
//! default, scope, effect and theme; the domain, Lua name, profile field and libretro values
//! follow when the row has them.

use crate::catalogue::{
    alignment_coefficient, default_cast_shadow, default_recv_shadow, Layer, Port, SettingId,
};
use crate::domain::{Domain, SettingOption};
use crate::meta::{SettingDescriptor, SettingEffect, SettingKind, SettingScope, SettingTheme};
use crate::num;
use crate::styles;
use crate::values::{Color, Rect, SettingValue, Vector3};

/// Default depth offset of each layer (§4.1.6).
const LAYER_OFFSET_DEFAULTS: [f32; 6] = [0.0, 0.0, 0.0, 12.0, 24.0, 36.0];

/// Default alignment of each layer: FRONT, CENTER, then FRONT (§4.1.6).
const LAYER_ALIGNMENT_DEFAULTS: [i32; 6] = [0, 1, 0, 0, 0, 0];

/// The background presets of DEC-09-Q5, in the order the decision lists them, as packed RGB. The
/// default, black, is one of them (REQ-PERS-081).
const BACKGROUND_PRESETS: [(i32, &str); 8] = [
    (0x0000_0000, "Black"),
    (0x00FF_FFFF, "White"),
    (0x0080_8080, "Gray"),
    // NES colour $21 of the palette of SPEC-04 (DEC-04B-Q1).
    (0x0000_78FC, "NES Sky Blue"),
    (0x0000_0080, "Dark Blue"),
    (0x0000_8000, "Green"),
    (0x00FF_0000, "Red"),
    (0x0080_0080, "Purple"),
];

/// The two texts of `ui.language` (AMD-035).
const LANGUAGE_ENGLISH: &str = "en";
const LANGUAGE_FRENCH: &str = "fr";

/// The angles the three curvature settings of AMD-043 REQ-SCN-066 take, in degrees (RO-1 point 7,
/// owner decision): **5° between −30 and 30**, where a player wants to tune a slight bend, and the
/// 30° of the amendment beyond, where a bend is a deliberate deformation. 23 rungs.
///
/// A divergence from REQ-SCN-066, which gives 13 rungs of 30°. Every one of those 13 is still a
/// rung here, so a value saved before this lot loads unchanged — and since RO-2 part 3 (owner
/// decision) that holds for a **Lua** write too: it snaps on this very grid, to the nearest rung,
/// ties away from zero (`styles::snap_write`), so a profile written for 3dSen out of multiples of
/// 30 behaves exactly as it did, while 45 gives 60, 75 gives 90 and 12.5 gives 15.
// Q-107
pub(crate) const CURVATURE_DEGREES: [f64; 23] = [
    -180.0, -150.0, -120.0, -90.0, -60.0, -30.0, -25.0, -20.0, -15.0, -10.0, -5.0, 0.0, 5.0, 10.0,
    15.0, 20.0, 25.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0,
];

/// [`CURVATURE_DEGREES`] as the libretro option list of AMD-043 §6, labelled as whole degrees.
fn curvature_options() -> Vec<SettingOption> {
    CURVATURE_DEGREES
        .iter()
        .map(|value| SettingOption::new(*value, &format!("{value:.0}")))
        .collect()
}

/// The rungs of `scene.arc_center`: the two heights AMD-043 §6 names, 0 for the screen centre and
/// 1 for the bottom edge of the reference frame, and the three quarters between them that RO-5
/// part 2 adds (owner decision, Q-107 §8).
///
/// The fraction multiplies `space_bend::EDGE_PIVOT_OFFSET`, so a rung *is* the height of the
/// pivot: nothing has to be mapped anywhere.
pub(crate) const ARC_CENTER_RUNGS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The label of each rung of [`ARC_CENTER_RUNGS`]. The two ends keep the names AMD-043 §6 gives
/// them, which is what the libretro option stores and what `option.<label>` translates; the three
/// new ones are read as percentages, there being no name for a point part of the way down.
pub(crate) const ARC_CENTER_LABELS: [&str; 5] =
    ["Screen Center", "25%", "50%", "75%", "Screen Edge"];

/// [`ARC_CENTER_RUNGS`] as a libretro option list, each rung under its label.
fn arc_center_options() -> Vec<SettingOption> {
    ARC_CENTER_RUNGS
        .iter()
        .zip(ARC_CENTER_LABELS)
        .map(|(value, label)| SettingOption::new(*value, label))
        .collect()
}

/// Every setting of §4.1, in the order of the specification's tables.
pub(crate) fn build() -> Vec<SettingDescriptor> {
    let mut all = Vec::new();
    video(&mut all);
    camera(&mut all);
    light(&mut all);
    scene(&mut all);
    clipping(&mut all);
    layers(&mut all);
    emulation(&mut all);
    conversion(&mut all);
    audio_input_interface(&mut all);
    touch(&mut all);
    vr_editor_movie_derived(&mut all);
    reimplementation_extras(&mut all);
    achievements(&mut all);
    all
}

fn opt(value: i32, label: &str) -> SettingOption {
    SettingOption::new(f64::from(value), label)
}

/// The values `min` to `max` in steps of `step`, labelled with `decimals` decimals, as the
/// libretro discretisation of REQ-PERS-081 gives them.
fn steps(min: f64, max: f64, step: f64, decimals: usize) -> Vec<SettingOption> {
    let count = num::round_to_i32((max - min) / step) + 1;
    (0..count)
        .map(|i| {
            let value = (min + f64::from(i) * step) * 1e6;
            let value = value.round_ties_even() / 1e6;
            SettingOption::new(value, &format!("{value:.decimals$}"))
        })
        .collect()
}

/// The volume discretisation of §4.6: 0 to 100 % in steps of 5.
fn percent_steps() -> Vec<SettingOption> {
    (0..=100)
        .step_by(5)
        .map(|percent: i32| SettingOption::new(f64::from(percent) / 100.0, &format!("{percent}%")))
        .collect()
}

fn background_presets() -> Vec<SettingOption> {
    BACKGROUND_PRESETS
        .iter()
        .map(|(packed, label)| SettingOption::new(f64::from(*packed), label))
        .collect()
}

fn video(out: &mut Vec<SettingDescriptor>) {
    video_output(out);
    video_scaling(out);
    render_style(out);
}

/// The visual style of AMD-040 §3, amended by AMD-055 REQ-SCN-078, and the twelve parameters of
/// the Enhanced style (AMD-055 REQ-SCN-079, keys of its §4). All of scope U, immediate effect,
/// never taken from a profile and never exposed in libretro.
///
/// They are persisted and reachable from Lua (`Setting.VisualStyle`,
/// `Setting:Get/SetVisualStyleParameter`) since RU-2, they act on the rendering since RU-5, and
/// they are rows of the options window since the step of RU-5 that gave them the "Style" group of
/// SPEC-10 REQ-UI-061 as amended (`zax_app::ui::options::STYLE`). They left
/// `zax_app::ui::options::EXCLUDED` with it: an option that moves and changes nothing would have
/// been worse than no option, which is why they waited for their effect.
fn render_style(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::RenderVisualStyle,
            SettingKind::Int,
            SettingValue::Int(styles::CLASSIC_STYLE),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Render,
        )
        // 1 is written for Enhanced and read back as 4; any other value takes the default, 0,
        // wherever it comes from — a script or the settings file (REQ-SCN-078).
        .with_domain(Domain::aliased_choice(
            vec![(
                f64::from(styles::ENHANCED_STYLE_ALIAS),
                f64::from(styles::ENHANCED_STYLE),
            )],
            vec![
                opt(styles::CLASSIC_STYLE, "Classic"),
                opt(styles::ENHANCED_STYLE, "Enhanced"),
            ],
        ))
        .with_lua("VisualStyle")
        .with_label("Style"),
    );
    style_parameters(out);
}

/// One row of [`STYLE_PARAMETERS`]: the key, its default, its lower and upper bound, its label,
/// and the names of its rungs (empty for a setting whose rungs are numbers).
type StyleParameter = (
    SettingId,
    i32,
    f64,
    f64,
    &'static str,
    &'static [&'static str],
);

/// The twelve parameters of REQ-SCN-079 — eight ladders of 1 to 5, which have no named rungs,
/// and four lists, which do.
///
/// The step is always 1 and carries the rounding of AMD-040 §3 (`Domain::snap`).
///
/// The four lists publish the named values of AMD-055 §4 (REQ-PERS-081), and they do it the way
/// `scene.arc_center` does: a **stepped** domain whose rungs are named, never `Domain::choice`.
/// The bounds, the step and the `Snap` rule stay the domain's, which is what keeps a write of
/// `ao.strength` = 9 at 5 and of `spl.voxel_grid` = 4 at 3 (REQ-SCN-079's first criterion) where
/// a finite choice would send both to the default. The two switches keep their numeric rungs:
/// §4 shows them as a check box, and the interface draws one (`ui::options::SWITCHES`).
const STYLE_PARAMETERS: [StyleParameter; 12] = [
    (
        SettingId::RenderStyleAo,
        0,
        0.0,
        2.0,
        "Ambient Occlusion",
        &["Off", "MSVO", "Depth"],
    ),
    (
        SettingId::RenderStyleAoStrength,
        3,
        1.0,
        5.0,
        "Strength",
        &[],
    ),
    (SettingId::RenderStyleAoRadius, 3, 1.0, 5.0, "Radius", &[]),
    (
        SettingId::RenderStyleAoThickness,
        3,
        1.0,
        5.0,
        "Thickness",
        &[],
    ),
    (
        SettingId::RenderStyleAoQuality,
        1,
        0.0,
        2.0,
        "AO Quality",
        &["Low", "Medium", "High"],
    ),
    (
        SettingId::RenderStylePixelShadow,
        0,
        0.0,
        1.0,
        "Pixel Shadows",
        &[],
    ),
    (
        SettingId::RenderStyleVoxelGrid,
        0,
        0.0,
        3.0,
        "Voxel Grid",
        &["Off", "Every 2 voxels", "Every 4 voxels", "Every 8 voxels"],
    ),
    (
        SettingId::RenderStyleReflections,
        0,
        0.0,
        1.0,
        "Reflections",
        &[],
    ),
    (
        SettingId::RenderStyleSsrShine,
        3,
        1.0,
        5.0,
        "Floor Shine",
        &[],
    ),
    (
        SettingId::RenderStyleSsrClarity,
        3,
        1.0,
        5.0,
        "Clarity",
        &[],
    ),
    (SettingId::RenderStyleSsrLength, 3, 1.0, 5.0, "Length", &[]),
    (
        SettingId::RenderStyleSsrQuality,
        2,
        0.0,
        3.0,
        "Reflection Quality",
        &["Low", "Medium", "High", "Ultra"],
    ),
];

/// The twelve parameters of the Enhanced style, from [`STYLE_PARAMETERS`].
fn style_parameters(out: &mut Vec<SettingDescriptor>) {
    for (id, default, minimum, maximum, label, values) in STYLE_PARAMETERS {
        let domain = Domain::stepped(minimum, maximum, 1.0, 0);
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Int,
                SettingValue::Int(default),
                SettingScope::User,
                SettingEffect::Immediate,
                SettingTheme::Render,
            )
            .with_domain(if values.is_empty() {
                domain
            } else {
                domain.with_labels(values)
            })
            .with_label(label),
        );
    }
}

fn video_output(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::VideoGraphicQuality,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        // Profile-scoped, faithfully (DEC-09-Q1); hidden on desktop (SPEC-10 REQ-UI-062).
        .with_domain(Domain::choice(vec![
            opt(0, "Low"),
            opt(1, "Medium"),
            opt(2, "High"),
        ]))
        .with_lua("GraphicQuality")
        .with_field("7"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoVSync,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Off"),
            opt(1, "Every refresh"),
            opt(2, "Every other refresh"),
        ]))
        .with_lua("VSync")
        .with_field("9"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoFrameRate,
            SettingKind::Int,
            SettingValue::Int(90),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        // Rule 1 of REQ-PERS-003: anything other than 60, 90 or 120 becomes 90.
        .with_domain(Domain::choice(vec![
            SettingOption::new(60.0, "60fps"),
            SettingOption::new(90.0, "90fps"),
            SettingOption::new(120.0, "120fps"),
        ]))
        .with_lua("FrameRate")
        .with_field("10")
        .with_label("Render Rate"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoSkipFrame,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::switch())
        .with_lua("SkipFrame")
        .with_field("30")
        .with_label("Fast Render"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoRenderMethod,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![opt(0, "Direct"), opt(1, "Render Tex")]))
        .with_lua("RenderMethod")
        .with_field("28")
        .with_label("Direct / Render Tex"),
    );
    video_queue_and_msaa(out);
}

/// The two rows of §4.1.1 the 2.0 update adds: the GPU frame queue of AMD-052 and the
/// multisampling count of AMD-059.
fn video_queue_and_msaa(out: &mut Vec<SettingDescriptor>) {
    // The GPU frame queue of AMD-052 REQ-LOOP-077, which AMD-059 REQ-LUA-116 opens to the
    // scripts as `Setting.MaxQueuedFrames`: the same value and the same effect, not a second
    // key. Not exposed in libretro (AMD-052 §3 point 8). The bound the loop reads is RU-3's.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoMaxQueuedFrames,
            SettingKind::Int,
            SettingValue::Int(2),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::stepped(1.0, 2.0, 1.0, 0))
        .with_lua("MaxQueuedFrames")
        .with_label("Max Queued Frames"),
    );
    // AMD-059 REQ-LUA-116: kept and handed back, with no effect of any kind — the
    // anti-aliasing of Zaxnes stays `video.anti_alias` (AMD-032) **(D)**. No interface, not
    // exposed in libretro.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoMsaa20,
            SettingKind::Int,
            SettingValue::Int(2),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Video,
        )
        // 0, 2 or 4; the Lua write has its own rounding (v ≤ 1 → 0, 2 or 3 → 2, ≥ 4 → 4), so
        // the only values that ever reach the model are these three.
        .with_domain(Domain::choice(vec![opt(0, "0"), opt(2, "2"), opt(4, "4")]))
        .with_lua("Msaa"),
    );
}

fn video_scaling(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::VideoResScale,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        // §4.1.1 shows a discretisation, §4.6 lists the key among the ones libretro does not get
        // because the host owns the resolution. §4.6 wins; see Q-035.
        .with_domain(Domain::choice(vec![
            opt(0, "Native"),
            opt(1, "2/3"),
            opt(2, "1/2"),
            opt(3, "1/3"),
            opt(4, "1/4"),
        ]))
        .with_lua("ResScaleIndex")
        .with_field("29"),
    );
    // The three levels of SPEC-09 §4.1.1, which AMD-032 §2 renders as multisampling, and the
    // fourth AMD-060 adds. The libretro list keeps the three: a core option's value strings are
    // persisted by the host, AMD-032 §7 lists the libretro table among what does not change, and
    // AMD-060 §2 extends the **domain** and names only the options window. So the fourth level is
    // reachable from the settings file, the options window, the console pages and Lua, and a
    // libretro front end keeps the enumeration it already stored.
    let anti_alias_libretro = vec![opt(0, "None"), opt(1, "FFAA"), opt(2, "SMAA")];
    let mut anti_alias_labels = anti_alias_libretro.clone();
    // AMD-060 REQ-SCN-085: SMAA 1x with edge detection on the scene's **depth**, the one
    // full-screen filter AMD-032 §2's objection does not reach — it finds geometric contours only
    // and never a contour inside a nearest-neighbour texture. A domain extension **(D)**: the
    // value does not exist in the original, a settings file or a profile field 32 that carries it
    // is read as it stands, and a value outside 0..3 is brought back the way one outside 0..2 was.
    anti_alias_labels.push(opt(3, "SMAA_DEPTH"));
    // AMD-052 REQ-PERS-089: shipped **on**, which is MSAA 4x by AMD-032 §2 — a divergence
    // **(D)** from the original, whose own default is 0 and which has no effect at all
    // (REQ-PERS-017). The handheld keeps 0 (`ui/handheld/defaults.rs`). AMD-060 §2 leaves the
    // default where it is.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoAntiAlias,
            SettingKind::Enum,
            SettingValue::Enum(1),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(anti_alias_labels))
        .with_lua("AntiAlias")
        .with_field("32")
        .with_label("Anti Alias")
        .with_libretro(anti_alias_libretro),
    );
    let mesh_labels = vec![
        opt(0, "GREEDY"),
        opt(1, "MARCHING"),
        opt(2, "GREEDY_MULTICOLOR"),
    ];
    out.push(
        SettingDescriptor::new(
            SettingId::RenderMeshAlgo,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Render,
        )
        .with_domain(Domain::choice(mesh_labels.clone()))
        .with_lua("RenderMode")
        .with_field("13")
        .with_label("Render")
        .with_libretro(mesh_labels),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::RenderShadow,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Render,
        )
        .with_domain(Domain::switch())
        .with_lua("Shadow")
        .with_field("17")
        .with_label("Shadow")
        .with_libretro(enabled_disabled()),
    );
    // AMD-049 REQ-SCN-075: the backdrop of REQ-SCN-048 lit with the almost-matt reflection model
    // of the original instead of the plain diffuse term. On by default, which is what the
    // original does and what the amendment asks for; Zaxnes keeps the switch because the model is
    // a fragment cost on a surface that can fill the picture, and the priority clause T-1 of
    // AMD-049 keeps the diffuse term valid. A setting of Zaxnes's own (divergence **(D)**): no
    // Lua access and no `Setting` field, the original program has neither.
    out.push(
        SettingDescriptor::new(
            SettingId::RenderBackdropLighting,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Render,
        )
        .with_domain(Domain::switch())
        .with_label("Backdrop Lighting")
        // The libretro option `zaxnes_render_backdrop_lighting`, values `enabled, disabled`,
        // default `enabled` — the shape AMD-043 §6 and AMD-046 §3 set for a boolean.
        .with_libretro(enabled_disabled()),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::UiFullscreen,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_domain(Domain::switch())
        .with_label("Full Screen"),
    );
}

/// The libretro labels of a switch: `enabled` for 1, `disabled` for 0.
fn enabled_disabled() -> Vec<SettingOption> {
    vec![
        SettingOption::new(1.0, "enabled"),
        SettingOption::new(0.0, "disabled"),
    ]
}

fn camera(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::CameraFov,
            SettingKind::Float,
            SettingValue::Float(70.0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Camera,
        )
        // The UI slider covers 45 to 95 in steps of 5; a script is bounded to [10, 150], which the
        // original did not bound at all.
        .with_domain(Domain::range(10.0, 150.0))
        .with_lua("Fov")
        .with_field("38")
        .with_label("FOV")
        .with_libretro(steps(45.0, 95.0, 5.0, 0)),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::CameraLocalCam,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Camera,
        )
        .with_domain(Domain::switch())
        .with_lua("LocalCam")
        .with_alias("TopDownCam")
        .with_field("25"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::CameraSmoothTime,
            SettingKind::Float,
            SettingValue::Float(0.4),
            SettingScope::Session,
            SettingEffect::Immediate,
            SettingTheme::Camera,
        )
        // Domain "> 0" with no repair rule, so a value outside it takes the default (REQ-PERS-003).
        .with_domain(Domain::greater_than(0.0))
        .with_lua("SmoothTime"),
    );
}

fn light(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::LightDirection,
            SettingKind::Vector3,
            SettingValue::from(Vector3::new(1.0, -1.0, 1.0)),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Light,
        )
        .with_domain(Domain::non_zero_vector())
        .with_lua("Light.Direction")
        .with_field("1.1"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LightColor,
            SettingKind::Color,
            SettingValue::from(Color::WHITE),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Light,
        )
        .with_lua("Light.Color")
        .with_field("1.2"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LightIntensity,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Light,
        )
        // Unbounded on purpose: the slider covers 0.5 to 1.5, but SPEC-03 REQ-PROF-041 keeps a user
        // intensity of 2 across a profile load.
        .with_lua("Light.Intensity")
        .with_field("1.3")
        .with_label("Light")
        .with_libretro(steps(0.5, 1.5, 0.05, 2)),
    );
}

fn scene(out: &mut Vec<SettingDescriptor>) {
    let skybox_labels = vec![
        opt(-1, "Default"),
        opt(0, "Color"),
        opt(1, "Gradient"),
        opt(2, "Fantasy"),
    ];
    out.push(
        SettingDescriptor::new(
            SettingId::SceneSkybox,
            SettingKind::Enum,
            SettingValue::Enum(-1),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Scene,
        )
        .with_domain(Domain::choice(skybox_labels.clone()))
        .with_lua("Skybox")
        .with_field("21")
        .with_label("Skybox")
        .with_libretro(skybox_labels),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::SceneCustomBgColor,
            SettingKind::Color,
            SettingValue::from(Color::BLACK),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Scene,
        )
        .with_lua("CustomBgColor")
        .with_field("3")
        .with_label("Background colour")
        .with_libretro(background_presets()),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::SceneUseNesBgColor,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Scene,
        )
        // Forced true by every profile load (REQ-PERS-005 point 3).
        .with_domain(Domain::switch())
        .with_lua("BgColor")
        .with_field("2"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::SceneMapEnable,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::NoEffect,
            SettingTheme::Scene,
        )
        .with_domain(Domain::switch())
        .with_lua("MapEnable")
        .with_field("5"),
    );
    curvature(out);
}

/// The five curvature settings of the "Geometry" group: the four angles of AMD-043 REQ-SCN-066
/// and the "Bend 2D Mode" switch of AMD-046 REQ-SCN-074.
fn curvature(out: &mut Vec<SettingDescriptor>) {
    // The three angles of AMD-043 REQ-SCN-066, on the finer grid of RO-1 point 7
    // ([`CURVATURE_DEGREES`]). They are the style parameters `spl.bend_zx`, `spl.bend_zy` and
    // `spl.arc_yx` of REQ-LUA-114; since RO-2 part 3 one grid serves them all — the interface,
    // the settings file and a Lua write go through the domain alone (`styles::snap_write`).
    for (id, label) in [
        (SettingId::SceneBendLr, "Bend Left-Right"),
        (SettingId::SceneBendTb, "Bend Top-Bottom"),
        (SettingId::SceneArcH, "Arc Horizontal"),
    ] {
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Int,
                SettingValue::Int(0),
                SettingScope::User,
                SettingEffect::Immediate,
                SettingTheme::Scene,
            )
            .with_domain(Domain::grid(&CURVATURE_DEGREES, 0))
            .with_label(label)
            // AMD-043 §6 writes "option, 13 valeurs"; RO-1 point 7 makes it 23 — the owner's
            // divergence, recorded in `docs/OPTIONS-LAYOUT.md` and raised as Q-107. The libretro
            // list is also what the console's advanced page steps by, so the two have to agree.
            .with_libretro(curvature_options()),
        );
    }
    // The arc centre names the height of the arc's pivot: AMD-043 §6 gives it two values, the
    // screen centre and the bottom edge of the reference frame, published as the two named rungs
    // "Screen Center" / "Screen Edge" — the idiom of `emu.run_ahead` (AMD-041 §3), the interface
    // translating each label through `option.<label>`.
    //
    // RO-5 part 2 (owner decision, a divergence recorded in `docs/OPTIONS-LAYOUT.md`): the two
    // become **five**, the three new ones placing the pivot at the matching fraction of the way
    // between the two heights (`zax_engine3d::space_bend::SpaceBend::new`). Three consequences,
    // and the reason for each:
    //
    //   * the kind is `Float`, because `Domain::coerce` rebuilds an `Int` value with
    //     `truncate_to_i32` and so cannot hold 0.25 at all. The settings file is **unchanged** for
    //     the two values that existed: `json::float_json` writes the shortest text that reads back
    //     as the same float, which is `0` and `1`, exactly what an `Int` wrote, and the `Float`
    //     branch of `from_json` reads any JSON number, so an older file loads as it always did.
    //     SPEC-09 §4.1 gives the key the type "entier"; that is the divergence.
    //   * the domain is a `Domain::grid`, not a `Domain::stepped`: the owner asks for the nearest
    //     rung with **ties away from zero**, which is what `grid` does and what RO-2 part 3 gave
    //     the three angles. `stepped` rounds ties to the even rung, so 0.125 would give 0 and
    //     0.375 would give 0.5. `grid` answers no single `step()`, which costs nothing here:
    //     `ui::options::bounds` takes the step from the libretro list, and that list has one.
    //   * the libretro list grows from two values to five. No persisted choice is lost —
    //     `core_options::try_apply` matches a value by **label**, and the two old labels are
    //     still there — and it has to grow, being what the console page steps by.
    // Q-107 §8
    out.push(
        SettingDescriptor::new(
            SettingId::SceneArcCenter,
            SettingKind::Float,
            SettingValue::Float(0.0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Scene,
        )
        .with_domain(Domain::grid(&ARC_CENTER_RUNGS, 2).with_labels(&ARC_CENTER_LABELS))
        .with_label("Arc Center")
        .with_libretro(arc_center_options()),
    );
    // AMD-046 REQ-SCN-074: the rectangle of the 2D mode bends like the game scene, which is what
    // the original program does, and this setting is how Zaxnes lets a player keep it flat — a
    // flat screen seen in 3D being a reasonable use of the 2D mode (divergence **(D)**, the
    // original has no such setting and so no Lua access to it either).
    out.push(
        SettingDescriptor::new(
            SettingId::SceneBend2dMode,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Scene,
        )
        .with_domain(Domain::switch())
        .with_label("Bend 2D Mode")
        // AMD-046 §3: the libretro option `zaxnes_scene_bend_2d_mode`, values `enabled,
        // disabled`, default `enabled`.
        .with_libretro(enabled_disabled()),
    );
}

fn clipping(out: &mut Vec<SettingDescriptor>) {
    // Index 0 left, 1 bottom, 2 right, 3 top (DEC-R2-T1, DEC-09-Q2). The non-integer defaults are
    // wanted: they avoid equalities at the edges.
    let margins: [(SettingId, f32, &str, &str); 4] = [
        (SettingId::ClipLeft, 0.1, "Clipping[0]", "4.1"),
        (SettingId::ClipBottom, 8.1, "Clipping[1]", "4.2"),
        (SettingId::ClipRight, 8.1, "Clipping[2]", "4.3"),
        (SettingId::ClipTop, 0.1, "Clipping[3]", "4.4"),
    ];
    for (id, value, lua, field) in margins {
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Float,
                SettingValue::Float(value),
                SettingScope::Profile,
                SettingEffect::Immediate,
                SettingTheme::Clipping,
            )
            // AMD-047 REQ-PERS-087: any finite real, negative included — profiles such as the
            // top-down ring of "Punch-Out!!" write −128 and −80. Only NaN and ±∞ fall back on
            // the default.
            .with_domain(Domain::finite_real())
            .with_lua(lua)
            .with_field(field),
        );
    }
}

fn layers(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::LayerDefaultLayer,
            SettingKind::Enum,
            SettingValue::Enum(2),
            SettingScope::Profile,
            // The column reads "jeu / imm."; the weaker of the two is recorded, so that a frontend
            // never promises an effect sooner than the specification does.
            SettingEffect::NextGameLoad,
            SettingTheme::Layers,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "UNIDENTIFIED"),
            opt(1, "UI"),
            opt(2, "L1"),
            opt(3, "L2"),
            opt(4, "L3"),
            opt(5, "L4"),
        ]))
        .with_lua("Layer.DefLayer")
        .with_field("6.2"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LayerAuto,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::NoEffect,
            SettingTheme::Layers,
        )
        .with_domain(Domain::switch())
        .with_lua("Layer.Auto")
        .with_field("6.3"),
    );
    for layer in Layer::ALL {
        layer_fields(out, layer);
    }
}

/// The per-layer settings of §4.1.6, in the order of the table.
fn layer_fields(out: &mut Vec<SettingDescriptor>, layer: Layer) {
    let alignment_domain =
        || Domain::choice(vec![opt(0, "FRONT"), opt(1, "CENTER"), opt(2, "BACK")]);
    let index = usize::from(layer.index());
    let n = layer.index();
    out.push(
        SettingDescriptor::new(
            SettingId::LayerOffset(layer),
            SettingKind::Float,
            SettingValue::Float(LAYER_OFFSET_DEFAULTS[index]),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Layers,
        )
        .with_domain(Domain::unbounded())
        .with_lua(format!("Layer[{n}].Offset"))
        .with_field("6.1.1"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LayerAligned(layer),
            SettingKind::Enum,
            SettingValue::Enum(LAYER_ALIGNMENT_DEFAULTS[index]),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Layers,
        )
        .with_domain(alignment_domain())
        .with_lua(format!("Layer[{n}].Aligned"))
        .with_field("6.1.2"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LayerCoef(layer),
            SettingKind::Float,
            SettingValue::Float(alignment_coefficient(LAYER_ALIGNMENT_DEFAULTS[index])),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Layers,
        )
        .with_lua(format!("Layer[{n}].Coef"))
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LayerCastShadow(layer),
            SettingKind::Bool,
            SettingValue::Bool(default_cast_shadow(layer)),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Layers,
        )
        .with_domain(Domain::switch())
        .with_lua(format!("Layer[{n}].CastShadow"))
        .with_field("6.1.3"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::LayerRecvShadow(layer),
            SettingKind::Bool,
            SettingValue::Bool(default_recv_shadow(layer)),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Layers,
        )
        .with_domain(Domain::switch())
        .with_lua(format!("Layer[{n}].RecvShadow"))
        .with_field("6.1.4"),
    );
}

fn emulation(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::EmuGameSpeed,
            SettingKind::Float,
            SettingValue::Float(0.0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        // Rule 3 of REQ-PERS-003. Back to 0 at the end of a game (REQ-PERS-012).
        .with_domain(Domain::range(-8.0, 8.0))
        .with_lua("GameSpeed")
        .with_field("11")
        .with_label("Speed")
        .with_libretro(steps(-8.0, 8.0, 1.0, 0)),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::EmuTvMode,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        // Profile-scoped, faithfully (DEC-09-Q1). MULTI is not offered by the UI.
        .with_domain(Domain::choice(vec![
            opt(0, "NTSC"),
            opt(1, "PAL"),
            opt(2, "MULTI"),
            opt(3, "DENDY"),
        ]))
        .with_lua("TVMode")
        .with_field("34")
        .with_label("TV Region"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::EmuAutoTvMode,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::NextRomLoad,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_lua("AutoTVMode")
        .with_field("33")
        .with_libretro(enabled_disabled()),
    );
    // The run-ahead of AMD-041 REQ-LOOP-064, which AMD-040 REQ-LUA-114 exposes as
    // `RunAheadFrames`: a write outside the domain brought back to the nearest bound. Not exposed
    // in libretro (AMD-041 §4). The frames it asks the loop to run ahead are RU-3's; until then
    // it is left out of the options window.
    // The domain is **0 to 10**, not the spec's 0 to 3 (owner request, `NesCore::MAX_RUN_AHEAD`):
    // see `Q-181`, which asks for the two requirements to be amended on that number.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuRunAhead,
            SettingKind::Int,
            SettingValue::Int(1),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        // The bounds, the step and the `Snap` rule stay those of a stepped domain — a write of 11
        // reads 10 and one of -1 reads 0 (REQ-LOOP-064's rule at the bound Q-181 records) — and
        // only the published labels change, which is what turns the row into the list of AMD-041
        // §3. The interface translates them through `option.<label>`; the value written to the
        // settings file is the number.
        // Rust-only (owner request): up to 10 frames, to try a game's own lag to the absurd.
        .with_domain(Domain::stepped(0.0, 10.0, 1.0, 0).with_labels(&[
            "Run-ahead off",
            "1 frame (Recommended)",
            "2 frames",
            "3 frames",
            "4 frames",
            "5 frames",
            "6 frames",
            "7 frames",
            "8 frames",
            "9 frames",
            "10 frames",
        ]))
        .with_lua("RunAheadFrames")
        .with_label("Run-ahead"),
    );
    emulation_rust_only(out);
    emulation_pause_3d(out);
    video_rust_only(out);
    video_smooth_scroll(out);
    video_sprite_limit(out);
    video_overscan(out);
    video_nes_palette(out);
    out.push(
        SettingDescriptor::new(
            SettingId::EmuCore,
            SettingKind::Int,
            SettingValue::Int(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        // Rule 2 of REQ-PERS-003: the remainder of the division by the number of cores, which is
        // exactly 1 in v1, so the effective value is always 0.
        .with_domain(Domain::core_index())
        .with_lua("Core")
        .with_field("22"),
    );
}

/// The emulation settings of the Rust port alone, in their place in the catalogue.
fn emulation_rust_only(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): how many emulated frames a displayed frame may run when the
    // game is late, 0 for none. The default is the 2 of AMD-052 REQ-LOOP-074, which replaces
    // the 5 REQ-LOOP-012 carried; the domain keeps its five values, the handheld default being
    // 5 (owner decision, `ui/handheld/defaults.rs`). Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuCatchUp,
            SettingKind::Enum,
            SettingValue::Enum(2),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "None"),
            opt(1, "1"),
            opt(2, "2"),
            opt(3, "3"),
            opt(4, "4"),
            opt(5, "5"),
        ]))
        .with_label("Catch-up (frames)"),
    );
    // Rust-only (owner request): the scripts' Lua collector in its smooth tuning, which removes
    // the long collection stalls. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuLuaSmoothGc,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_label("Smooth Lua collector"),
    );
    // Rust-only (owner request): the optimised Lua paths, which use `unsafe` code: the host values
    // reached through C metamethods instead of `mlua`'s userdata (same results, less time per
    // access), and the pipelined frames of `emu.pipeline`, which only run with it. Off: only the
    // safe `mlua` paths run. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuLuaOptimized,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::NextGameLoad,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_label("Optimized Lua (unsafe)"),
    );
    // Rust-only (owner request): the experimental Lua paths, on top of the optimised Lua (which
    // they need): the UpdateS hooks that only use fields of `shape`, as read from the source
    // when the profile loads, receive a light `shape` and no `shape2D`. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuLuaExperimental,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::NextGameLoad,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_label("Experimental Lua optimization"),
    );
    // Rust-only (owner request): the scripts and the conversion of a frame run on their own
    // thread while the frame before it is drawn, one displayed frame later; "Auto" does so only
    // while the frames are late. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuPipeline,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Off"),
            opt(1, "Auto"),
            opt(2, "Always"),
        ]))
        .with_label("Pipelined frames"),
    );
    // Rust-only (owner request): the graphics API the window draws with, read at start-up.
    // Vulkan and DirectX 12 go through wgpu, in the desktop builds that carry it; the others, and
    // DirectX 12 off Windows, fall back. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoBackend,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Restart,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "OpenGL"),
            opt(1, "Vulkan"),
            opt(2, "DirectX 12"),
        ]))
        .with_label("Graphics API"),
    );
    // Rust-only (owner request): the scene's render scale follows the GPU time, between native
    // and 1/2, instead of `video.res_scale` (render texture method only). Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoDynamicResolution,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::switch())
        .with_label("Automatic resolution"),
    );
}

fn emulation_pause_3d(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): while the 2D picture is shown, nothing of the 3D scene is, so
    // the conversion and the frame hooks of profiles whose scripts leave the core alone are
    // not run; the frame shown when the 3D comes back is converted first. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuPause3dIn2d,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_label("Pause the 3D in 2D"),
    );
    // Rust-only (owner request): when a displayed frame runs several emulated ones to catch up,
    // only the last is drawn, so the others of a profile whose scripts leave the core alone are
    // neither converted nor run through the frame hooks. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuCatchUpLast3d,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_domain(Domain::switch())
        .with_label("Catch-up converts the shown frame only"),
    );
    // Rust-only (owner request, handheld options only): while the 3D is shown, the screen is
    // refreshed every other vsync and the emulation catches up at full speed, with only the
    // shown frame converted; the 2D picture keeps every refresh. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoHalfRate3d,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::switch())
        .with_label("3D at half the refresh rate"),
    );
}

fn conversion(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::ConvSameBgPalette,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Conversion,
        )
        .with_domain(Domain::switch())
        .with_lua("SameBgPalette")
        .with_alias("SamePalette")
        .with_field("18"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::ConvSameSpPalette,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Conversion,
        )
        .with_domain(Domain::switch())
        .with_lua("SameSpPalette")
        .with_field("37"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::ConvSpriteTolerance,
            SettingKind::Int,
            SettingValue::Int(0),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Conversion,
        )
        // Rule 4 of REQ-PERS-003: a negative value becomes its absolute value.
        .with_domain(Domain::non_negative_by_absolute_value())
        .with_lua("SpriteTolerance")
        .with_field("19"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::ConvScanDirection,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Conversion,
        )
        .with_domain(Domain::choice(vec![opt(0, "Rows"), opt(1, "Columns")]))
        .with_lua("ScanDirection")
        .with_field("20"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::ConvEmptyTile,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Profile,
            SettingEffect::Immediate,
            SettingTheme::Conversion,
        )
        .with_domain(Domain::switch())
        .with_lua("EmptyTile")
        .with_field("24"),
    );
}

fn touch(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request, Android only): the transparent pad of a touch screen; in "Auto"
    // it hides while a real gamepad is connected. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::InputTouchPad,
            SettingKind::Enum,
            SettingValue::Enum(2),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Input,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Off"),
            opt(1, "On"),
            opt(2, "Auto"),
        ]))
        .with_label("Touch pad"),
    );
}

fn audio_input_interface(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::AudioVolume,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Audio,
        )
        .with_domain(Domain::range(0.0, 1.0))
        .with_lua("Volume")
        .with_field("12")
        .with_label("Volume")
        // The UI slider is a percentage in steps of 1; libretro gets steps of 5.
        .with_libretro(percent_steps()),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::AudioBgMusic,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Audio,
        )
        .with_domain(Domain::switch())
        .with_lua("BGMusic")
        .with_field("31")
        .with_label("BG Music"),
    );
    for port in Port::ALL {
        out.push(
            SettingDescriptor::new(
                SettingId::InputPortType(port),
                SettingKind::Enum,
                SettingValue::Enum(0),
                SettingScope::Session,
                SettingEffect::Immediate,
                SettingTheme::Input,
            )
            // Lua reaches these through Setting:GetInputType and Setting:SetInputType, not through a
            // property, so there is no Lua path (SPEC-08 REQ-INP-060).
            .with_domain(Domain::choice(vec![opt(0, "Gamepad"), opt(1, "Zapper")])),
        );
    }
    out.push(
        SettingDescriptor::new(
            SettingId::InputZapperEnable,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Input,
        )
        .with_lua("ZapperEnable")
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::InputEmulatedMenuButton,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            // Stored and exposed with no effect at all (DEC-08-6, DEC-R2-D7).
            SettingEffect::NoEffect,
            SettingTheme::Input,
        )
        .with_domain(Domain::switch())
        .with_lua("EmulatedMenuButton")
        .with_field("27")
        .with_label("Menu = Select + Start"),
    );
    interface(out);
}

/// The interface settings of §4.1.9.
fn interface(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::UiSortOption,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        // AMD-053 §3 point 10: a third order, « Favorites first ». The value 2 is never written
        // into profile field 36 — 0 goes there instead — and a 2 read from a profile is ignored
        // like every value of user scope (REQ-PERS-005 point 2).
        .with_domain(Domain::choice(vec![
            opt(0, "A-Z"),
            opt(1, "Recent"),
            opt(2, "Favorites first"),
        ]))
        .with_lua("SortOption")
        .with_field("36")
        .with_label("Sort by"),
    );
    // The column gives the personal folder as the default. This crate knows nothing of the
    // platform (REQ-PERS-082), so the default is empty and the host substitutes its own documents
    // folder when it reads an empty value (REQ-PERS-066).
    out.push(SettingDescriptor::new(
        SettingId::UiLastBrowseDir,
        SettingKind::Text,
        SettingValue::from(""),
        SettingScope::User,
        SettingEffect::Immediate,
        SettingTheme::Interface,
    ));
    // AMD-059 REQ-LUA-116: `Setting.LibraryView` is kept exactly as written and read back as it
    // was — no domain at all. Zaxnes has one library layout, so nothing reads it; no interface,
    // not exposed in libretro.
    out.push(
        SettingDescriptor::new(
            SettingId::UiLibraryView,
            SettingKind::Int,
            SettingValue::Int(0),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Interface,
        )
        .with_lua("LibraryView"),
    );
    // The default size is the one of SPEC-10; -1 for the position means "let the platform place the
    // window" (REQ-PERS-067).
    out.push(SettingDescriptor::new(
        SettingId::UiWindowRect,
        SettingKind::Rect,
        SettingValue::from(Rect::new(-1, -1, 1280, 720)),
        SettingScope::User,
        SettingEffect::Restart,
        SettingTheme::Interface,
    ));
}

fn vr_editor_movie_derived(out: &mut Vec<SettingDescriptor>) {
    // Always 0 on reading, every write ignored without error (REQ-PERS-011).
    out.push(
        SettingDescriptor::new(
            SettingId::VrRuntime,
            SettingKind::Int,
            SettingValue::Int(0),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Vr,
        )
        .with_lua("VrRuntime")
        .with_field("8"),
    );
    vr_scene_and_editor(out);
    derived_and_internal(out);
}

fn vr_scene_and_editor(out: &mut Vec<SettingDescriptor>) {
    // The written value is kept; reading gives 1 while the runtime is 0 (REQ-PERS-011).
    out.push(
        SettingDescriptor::new(
            SettingId::VrSceneRatio,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Vr,
        )
        .with_lua("SceneRatio")
        .with_field("26"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VrMirroring,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Vr,
        )
        .with_domain(Domain::switch())
        .with_lua("Mirroring")
        .with_field("23"),
    );
    // AMD-059 REQ-LUA-116: the refresh rate of a virtual-reality headset — {72, 90, 120}, the
    // Lua write taking the nearest rung and the smaller one at equal distance. Without virtual
    // reality it has no effect here **(D)**; the `Vr` theme leaves it out of every tab on its
    // own, so it needs no exclusion line.
    out.push(
        SettingDescriptor::new(
            SettingId::VrRefreshRate,
            SettingKind::Int,
            SettingValue::Int(90),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Vr,
        )
        .with_domain(Domain::choice(vec![
            SettingOption::new(72.0, "72"),
            SettingOption::new(90.0, "90"),
            SettingOption::new(120.0, "120"),
        ]))
        .with_lua("RefreshRate"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VrControlMethod,
            SettingKind::Int,
            SettingValue::Int(0),
            SettingScope::User,
            SettingEffect::NoEffect,
            SettingTheme::Vr,
        )
        .with_lua("ControlMethod")
        .with_field("16"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::EditorSelectionMode,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::Session,
            SettingEffect::NoEffect,
            SettingTheme::Editor,
        )
        .with_domain(Domain::choice(vec![opt(0, "Shape3D"), opt(1, "Tile")]))
        .with_lua("SelectionMode"),
    );
    // Five records named Config1 to Config5 by default (SPEC-03 REQ-PROF-044); the records belong to
    // the profile format, so this model only carries them.
    out.push(
        SettingDescriptor::new(
            SettingId::EditorSavedPatterns,
            SettingKind::Opaque,
            SettingValue::Opaque(None),
            SettingScope::Profile,
            SettingEffect::NoEffect,
            SettingTheme::Editor,
        )
        .with_lua("SavedPattern3D")
        .with_field("14"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::EditorSavedAnimPatterns,
            SettingKind::Opaque,
            SettingValue::Opaque(None),
            SettingScope::Profile,
            SettingEffect::NoEffect,
            SettingTheme::Editor,
        )
        .with_lua("SaveAnimPattern3D")
        .with_field("15"),
    );
    movie_and_scene_scale(out);
}

fn movie_and_scene_scale(out: &mut Vec<SettingDescriptor>) {
    // Frozen at NONE; a write of anything else is ignored (REQ-PERS-016).
    out.push(
        SettingDescriptor::new(
            SettingId::MovieMode,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::Session,
            SettingEffect::NoEffect,
            SettingTheme::Movie,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "NONE"),
            opt(1, "PLAYBACK"),
            opt(2, "RECORDING"),
        ]))
        .with_lua("MovieMode"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::SceneRootScale,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::Session,
            SettingEffect::Immediate,
            SettingTheme::Internal,
        )
        .with_lua("RootScale"),
    );
    // Writing SceneScale writes SceneRatio (REQ-PERS-013 point 3), so this derived member is the one
    // exception that Lua may assign.
    out.push(
        SettingDescriptor::new(
            SettingId::SceneSceneScale,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::Derived,
            SettingEffect::Immediate,
            SettingTheme::Internal,
        )
        .with_lua("SceneScale"),
    );
}

fn derived_and_internal(out: &mut Vec<SettingDescriptor>) {
    out.push(
        SettingDescriptor::new(
            SettingId::SceneSkyboxEnable,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_lua("SkyboxEnable")
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoResScaleFactor,
            SettingKind::Float,
            SettingValue::Float(1.0),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_lua("ResScale")
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoTargetFramerate,
            SettingKind::Float,
            SettingValue::Float(90.0),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_lua("TargetFramerate")
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::VideoTargetTimePerFrame,
            SettingKind::Float,
            SettingValue::Float(1.0 / 90.0),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_lua("TargetTimePerFrame")
        .read_only_in_lua(),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::EmuTvData,
            SettingKind::Opaque,
            SettingValue::Opaque(None),
            SettingScope::Derived,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_lua("TVData")
        .read_only_in_lua(),
    );
    // Always PROFILE: the ROM mode is the editor's (§4.1.10).
    out.push(
        SettingDescriptor::new(
            SettingId::AppEngineMode,
            SettingKind::Enum,
            SettingValue::Enum(1),
            SettingScope::Session,
            SettingEffect::NoEffect,
            SettingTheme::Internal,
        )
        .with_domain(Domain::choice(vec![opt(0, "ROM"), opt(1, "PROFILE")]))
        .with_lua("EngineMode")
        .read_only_in_lua(),
    );
}

fn reimplementation_extras(out: &mut Vec<SettingDescriptor>) {
    // The mute switch of the Audio menu (AMD-010 REQ-UI-200). SPEC-11 has no separate mute setting,
    // so this one forces the effective gain to zero and leaves `audio.volume` alone.
    out.push(
        SettingDescriptor::new(
            SettingId::AudioMute,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::Session,
            SettingEffect::Immediate,
            SettingTheme::Audio,
        )
        .with_domain(Domain::switch())
        .with_label("Mute"),
    );
    // Rust-only (AUDIO-1, Q-153 §2): how much audio the core's ring is held at before the
    // device plays it. SPEC-11 fixes no setting — REQ-AUD-003's notes only aim at a total
    // latency below 60 ms — but the cause of the pops is a main thread that occasionally takes
    // longer than the buffer, and that depends on the machine and on the 3D scene. Not exposed
    // to Lua: it is a property of the output, not of the game.
    out.push(
        SettingDescriptor::new(
            SettingId::AudioLatencyMs,
            SettingKind::Int,
            SettingValue::Int(60),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Audio,
        )
        .with_domain(Domain::range(20.0, 200.0))
        // No libretro option: REQ-AUD-007 leaves the latency to the host in that frontend, and
        // the normative list of REQ-PERS-081 is not to grow. The options window draws a whole
        // slider over the domain on its own.
        .with_label("Output latency (ms)"),
    );
    // Rust-only (AUDIO-2): the fallback to the output `v0.4.2` had — an SDL queue of four
    // 512-frame blocks topped up once per rendered frame, with its own rate control on the
    // core's ring. Off by default: the AUDIO-1 output is the one the program opens, and this is
    // for a device the callback path misbehaves on. Read **only** when the device opens, so that
    // the choice costs nothing per frame and nothing per sample. Not exposed to Lua, and no
    // libretro option: the output is a property of this frontend's device.
    out.push(
        SettingDescriptor::new(
            SettingId::AudioClassicOutput,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Audio,
        )
        .with_domain(Domain::switch())
        .with_label("Classic audio output (v0.4.2)"),
    );
    out.push(
        SettingDescriptor::new(
            SettingId::UiShowFps,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_domain(Domain::switch())
        .with_label("Show FPS"),
    );
    // Rust-only (owner request): the frame profiler, which writes the cost of each stage of the
    // frame to the log every two seconds and next to the FPS counter. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::UiProfiler,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_domain(Domain::switch())
        .with_label("Performance measures"),
    );
    // Rust-only (owner request): the editor mode, for writing a Lua profile with an AI agent:
    // its MCP server, the pens, and the editor folder of the user data (docs/EDITOR.md). Not
    // exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::UiEditorMode,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_domain(Domain::switch())
        .with_label("Editor mode"),
    );
    // AMD-035 §1 (amended): the interface language, "en" in the catalogue. While nothing is saved
    // the application suggests the operating-system language through the model's environment
    // default. The two texts are the whole domain; anything else reads as "en".
    out.push(
        SettingDescriptor::new(
            SettingId::UiLanguage,
            SettingKind::Text,
            SettingValue::from(LANGUAGE_ENGLISH),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_text_options(&[LANGUAGE_ENGLISH, LANGUAGE_FRENCH])
        .with_label("Language"),
    );
    // Rust-only (owner request): the folder the ROM list of the console mode shows. Empty means
    // the `roms` folder of the user data; `--roms` wins over it.
    out.push(
        SettingDescriptor::new(
            SettingId::UiRomsFolder,
            SettingKind::Text,
            SettingValue::from(""),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_label("ROM folder"),
    );
}

fn video_rust_only(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): the built-in patches of zax-nes::builtin_patches (the Zelda II
    // widescreen hack). Read when a ROM is loaded; states made the other way do not fit. Not
    // exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuBuiltinPatches,
            SettingKind::Bool,
            SettingValue::Bool(true),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_label("Built-in ROM patches"),
    );
    // Rust-only (owner request): the advanced mode of a game's widescreen (a built-in patch whose
    // `advanced` is set: Super Mario Bros.' enemies in the margins); off, its margin decoder alone
    // widens it. An option of each game only, on by default (owner request: the widescreen is the
    // whole hack, its testing aids aside).
    out.push(SettingDescriptor::new(
        SettingId::VideoWidescreenAdvanced,
        SettingKind::Bool,
        SettingValue::Bool(true),
        SettingScope::User,
        SettingEffect::NextRomLoad,
        SettingTheme::Video,
    ));
    // Rust-only (owner request): the built-in patch applied to a ROM none is made for (another
    // version of its game, a ROM hack), by name; an option of each game only.
    out.push(SettingDescriptor::new(
        SettingId::EmuForcedPatch,
        SettingKind::Text,
        SettingValue::from(""),
        SettingScope::User,
        SettingEffect::NextRomLoad,
        SettingTheme::Emulation,
    ));
    // Rust-only (owner request): the options of the built-in patches the player changed, as
    // "<patch>/<option>=0|1" entries separated by ";" (zax-nes::builtin_patches::HackOption).
    // Not exposed to Lua, not shown in the options window (a menu edits it).
    out.push(
        SettingDescriptor::new(
            SettingId::EmuHackOptions,
            SettingKind::Text,
            SettingValue::from(""),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_label("Hack options"),
    );
    // Rust-only (owner request): how much wider than 256 pixels the picture of a ROM made for
    // the widescreen extension may be (zax-nes::wide). The other ROMs never see it. Not exposed
    // to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoWidescreen,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Off"),
            // The ratio of the view: the window, the screen in full screen or on a handheld.
            opt(1, "Auto"),
            opt(2, "3:2"),
            opt(3, "16:10"),
            opt(4, "16:9"),
        ]))
        .with_label("Widescreen"),
    );
    // Rust-only (owner request): a percentage added to the widescreen picture's width, so that the
    // objects at the edges of the 3D view are already whole when they come into sight. Not
    // exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoWidescreenExtra,
            SettingKind::Enum,
            // +20 % (owner request).
            SettingValue::Enum(4),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "+0 %"),
            opt(1, "+5 %"),
            opt(2, "+10 %"),
            opt(3, "+15 %"),
            opt(4, "+20 %"),
            opt(5, "+25 %"),
            opt(6, "+30 %"),
            opt(7, "+40 %"),
            opt(8, "+50 %"),
            opt(9, "+75 %"),
            opt(10, "+100 %"),
            opt(11, "+150 %"),
            opt(12, "+200 %"),
            opt(13, "+300 %"),
        ]))
        .with_label("Widescreen extra width"),
    );
    // Rust-only (owner request): a shape cut where the game stops drawing a band of the
    // widescreen picture is left unfinished (temporary pattern) until it is whole. Off by
    // default: a one-tile object standing there would stay temporary. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoWidescreenStrictEdges,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_label("Widescreen: wait for whole objects at the drawn edges"),
    );
}

/// `emu.smooth_scroll_ahead` (Rust-only).
fn video_smooth_scroll(out: &mut Vec<SettingDescriptor>) {
    // Rust-only, experimental (owner request): the playfield drawn between two frames on a
    // display faster than the game, from the scroll the PPU drew in the run-ahead's real frame
    // and the frame shown; in the emulation options (and each game's, with its run-ahead),
    // without effect unless run-ahead is on and the
    // display shows more than 60 frames a second. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuSmoothScrollAhead,
            SettingKind::Bool,
            // **Off** by default, and the one place this lot overrode the branch's own choice.
            // The branch shipped it on (owner request), with the handheld's defaults turning it
            // back off. On, with the default run-ahead of 1 and any display above 61 Hz, it
            // shifts the 2D playfield of **every game, with no pack at all**, and turns on the
            // PPU's per-line scroll record — a picture and a cost for a game that asked for
            // neither. The owner's rule for this lot is that nothing may move for a game
            // without a pack unless it is behind an option **off by default**, so this is that
            // option. Nothing else about the feature changed: on, it behaves exactly as the
            // branch built it (`docs/progress/KIRBY-MERGE.md` §2.4, (S4)).
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_label("Smooth scrolling (interpolated from run-ahead)"),
    );
    // Rust-only (owner request): the band the smooth scrolling slides tinted red, the sprites it
    // keeps still green, to see what it does. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuSmoothScrollView,
            SettingKind::Bool,
            SettingValue::Bool(false),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Emulation,
        )
        .with_label("Show the smooth scrolling's band"),
    );
}

/// `video.remove_sprite_limit` (Rust-only).
fn video_sprite_limit(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): the 2D picture shows more than eight sprites on a line, which
    // ends the flicker of the games that cycle their sprites. "Smart" keeps the limit on a line
    // where a game stacks eight sprites to hide the ones after them, as Mesen does (the 3D view
    // has every sprite whatever the choice). Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoRemoveSpriteLimit,
            SettingKind::Enum,
            // Smart (owner request).
            SettingValue::Enum(2),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Disabled"),
            opt(1, "Always"),
            opt(2, "Smart"),
        ]))
        .with_label("Remove sprite limit"),
    );
}

/// The overscan values: 0 to 32 pixels, and with `global` the value that follows the
/// preferences (a game's own).
fn overscan_pixels(global: bool) -> Vec<SettingOption> {
    let mut options = Vec::new();
    if global {
        options.push(opt(-1, "Global"));
    }
    options.extend((0..=32).map(|pixels| opt(pixels, &format!("{pixels} px"))));
    options
}

/// `video.overscan_*` (Rust-only).
fn video_overscan(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): the picture's edges a television did not show (games leave
    // scrolling garbage there), as Mesen sets them: per side, for the NTSC games and for the PAL
    // ones, 0 by default (the whole picture). A game has the four sides of its own
    // (`video.overscan_top` and the others, "Global" by default: its region's), and the way they
    // show: left out (the scene's background shows there), the picture keeping its size ("Fit"), or the rest enlarged to fill the
    // view in 2D ("Resize"). With a widescreen margin the left and right sides are not cut (the
    // margins go on past them), with a vertical one the top and bottom are not either but where
    // the game's margin decoder asks. Not exposed to Lua.
    let sides = [
        (SettingId::VideoOverscanNtscTop, "Overscan NTSC: top", false),
        (
            SettingId::VideoOverscanNtscBottom,
            "Overscan NTSC: bottom",
            false,
        ),
        (
            SettingId::VideoOverscanNtscLeft,
            "Overscan NTSC: left",
            false,
        ),
        (
            SettingId::VideoOverscanNtscRight,
            "Overscan NTSC: right",
            false,
        ),
        (SettingId::VideoOverscanPalTop, "Overscan PAL: top", false),
        (
            SettingId::VideoOverscanPalBottom,
            "Overscan PAL: bottom",
            false,
        ),
        (SettingId::VideoOverscanPalLeft, "Overscan PAL: left", false),
        (
            SettingId::VideoOverscanPalRight,
            "Overscan PAL: right",
            false,
        ),
        (SettingId::VideoOverscanTop, "Overscan: top", true),
        (SettingId::VideoOverscanBottom, "Overscan: bottom", true),
        (SettingId::VideoOverscanLeft, "Overscan: left", true),
        (SettingId::VideoOverscanRight, "Overscan: right", true),
    ];
    for (id, label, global) in sides {
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Enum,
                SettingValue::Enum(if global { -1 } else { 0 }),
                SettingScope::User,
                SettingEffect::Immediate,
                SettingTheme::Video,
            )
            .with_domain(Domain::choice(overscan_pixels(global)))
            .with_label(label),
        );
    }
    out.push(
        SettingDescriptor::new(
            SettingId::VideoOverscanMode,
            SettingKind::Enum,
            // Resize (owner request).
            SettingValue::Enum(1),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![opt(0, "Fit"), opt(1, "Resize")]))
        .with_label("Overscan: fit or resize"),
    );
}

/// `video.nes_palette` (Rust-only).
fn video_nes_palette(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): the RGB table the 64 NES colours are shown in, the 2D picture
    // and the 3D view alike. The console outputs a composite signal, which each emulator decodes
    // into a table of its own: Zaxnes' (3DSen's) or one of the main emulators' (Mesen's the default)
    // (`zax_engine3d::palette::NesPalette`, in this order). Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::VideoNesPalette,
            SettingKind::Enum,
            // Mesen's (owner request).
            SettingValue::Enum(1),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(vec![
            opt(0, "Zaxnes"),
            opt(1, "Mesen"),
            opt(2, "FCEUX"),
            opt(3, "Nestopia (YUV)"),
            opt(4, "Nestopia (RGB)"),
            opt(5, "Smooth (FirebrandX)"),
            opt(6, "NES Classic (FirebrandX)"),
            opt(7, "Composite Direct (FirebrandX)"),
            opt(8, "PVM Style (FirebrandX)"),
            opt(9, "Sony CXA2025AS"),
            opt(10, "Wavebeam"),
        ]))
        .with_label("NES colour palette"),
    );
}

fn achievements(out: &mut Vec<SettingDescriptor>) {
    // Rust-only (owner request): RetroAchievements, softcore only. The switches are read when a
    // game starts; the account is the name and the token of the last sign-in (never the
    // password), edited by the sign-in screen only. Not exposed to Lua.
    for (id, default, label) in [
        (
            SettingId::AchievementsEnabled,
            false,
            "Enable RetroAchievements",
        ),
        (SettingId::AchievementsEncore, false, "Encore mode"),
        (
            SettingId::AchievementsUnofficial,
            false,
            "Test unofficial achievements",
        ),
        (SettingId::AchievementsSound, true, "Unlock sound"),
        // The icons of the challenges under way, in a corner of the game (owner request: off).
        (
            SettingId::AchievementsChallenges,
            false,
            "Show the challenges under way",
        ),
    ] {
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Bool,
                SettingValue::Bool(default),
                SettingScope::User,
                SettingEffect::Immediate,
                SettingTheme::Achievements,
            )
            .with_domain(Domain::switch())
            .with_label(label),
        );
    }
    for id in [
        SettingId::AchievementsUsername,
        SettingId::AchievementsToken,
    ] {
        out.push(SettingDescriptor::new(
            id,
            SettingKind::Text,
            SettingValue::from(""),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Achievements,
        ));
    }
}
