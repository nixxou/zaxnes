//! The rows of the catalogue, in the order of SPEC-09 §4.1. Each row carries the key, shape,
//! default, scope, effect and theme; the domain, Lua name, profile field and libretro values
//! follow when the row has them.

use crate::catalogue::{
    alignment_coefficient, default_cast_shadow, default_recv_shadow, Layer, Port, SettingId,
};
use crate::domain::{Domain, SettingOption};
use crate::meta::{SettingDescriptor, SettingEffect, SettingKind, SettingScope, SettingTheme};
use crate::num;
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
    vr_editor_movie_derived(&mut all);
    reimplementation_extras(&mut all);
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
    let anti_alias_labels = vec![opt(0, "None"), opt(1, "FFAA"), opt(2, "SMAA")];
    out.push(
        SettingDescriptor::new(
            SettingId::VideoAntiAlias,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Video,
        )
        .with_domain(Domain::choice(anti_alias_labels.clone()))
        .with_lua("AntiAlias")
        .with_field("32")
        .with_label("Anti Alias")
        .with_libretro(anti_alias_labels),
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
}

fn clipping(out: &mut Vec<SettingDescriptor>) {
    // Index 0 left, 1 bottom, 2 right, 3 top (DEC-R2-T1, DEC-09-Q2). The non-integer defaults are
    // wanted: they avoid equalities at the edges.
    let margins: [(SettingId, f32, f64, &str, &str); 4] = [
        (SettingId::ClipLeft, 0.1, 256.0, "Clipping[0]", "4.1"),
        (SettingId::ClipBottom, 8.1, 248.0, "Clipping[1]", "4.2"),
        (SettingId::ClipRight, 8.1, 256.0, "Clipping[2]", "4.3"),
        (SettingId::ClipTop, 0.1, 248.0, "Clipping[3]", "4.4"),
    ];
    for (id, value, maximum, lua, field) in margins {
        out.push(
            SettingDescriptor::new(
                id,
                SettingKind::Float,
                SettingValue::Float(value),
                SettingScope::Profile,
                SettingEffect::Immediate,
                SettingTheme::Clipping,
            )
            .with_domain(Domain::range(0.0, maximum))
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
    emulation_rust_only(out);
    video_rust_only(out);
    video_sprite_limit(out);
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
    // game is late, 0 for none; REQ-LOOP-012's five by default. Not exposed to Lua.
    out.push(
        SettingDescriptor::new(
            SettingId::EmuCatchUp,
            SettingKind::Enum,
            SettingValue::Enum(5),
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
    out.push(
        SettingDescriptor::new(
            SettingId::UiSortOption,
            SettingKind::Enum,
            SettingValue::Enum(0),
            SettingScope::User,
            SettingEffect::Immediate,
            SettingTheme::Interface,
        )
        .with_domain(Domain::choice(vec![opt(0, "A-Z"), opt(1, "Recent")]))
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
