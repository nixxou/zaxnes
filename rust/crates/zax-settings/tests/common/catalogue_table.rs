//! An independent transcription of the tables of SPEC-09 §4.1.1 to §4.1.10, used to check the
//! catalogue the implementation builds (REQ-PERS-001). It is written out again from the
//! specification rather than derived from the catalogue, so that a mistake in the catalogue shows
//! up as a disagreement between the two.

use zax_settings::meta::{SettingEffect, SettingKind, SettingScope};
use zax_settings::values::SettingValue;

/// One row of §4.1: the columns "Clé", "Lua", "Type", "Défaut", "Portée" and "Effet".
pub struct CatalogueRow {
    pub key: String,
    pub lua: Option<String>,
    pub kind: SettingKind,
    /// The default, as text: the number, `true`/`false`, the `#RRGGBBAA` colour or the
    /// comma-separated vector.
    pub default: &'static str,
    pub scope: SettingScope,
    pub effect: SettingEffect,
}

use SettingEffect::{
    Immediate as Imm, NextGameLoad as Game, NextRomLoad as Rom, NoEffect as No, Restart,
};
use SettingKind::{
    Bool as Flag, Color as Colour, Enum as Choice, Float as Real, Int as Whole, Opaque,
    Rect as Rectangle, Text, Vector3 as Vector,
};
use SettingScope::{Derived as D, Profile as P, Session as S, User as U};

fn row(
    key: &str,
    lua: Option<&str>,
    kind: SettingKind,
    default: &'static str,
    scope: SettingScope,
    effect: SettingEffect,
) -> CatalogueRow {
    CatalogueRow {
        key: key.to_owned(),
        lua: lua.map(str::to_owned),
        kind,
        default,
        scope,
        effect,
    }
}

/// Every row of §4.1, in the order of the specification.
#[must_use]
pub fn rows() -> Vec<CatalogueRow> {
    let mut rows = first_sections();
    rows.extend(layer_rows());
    rows.extend(last_sections());
    rows.extend(tail_sections());
    rows
}

/// §4.1.1 to §4.1.6, up to the layer table.
fn first_sections() -> Vec<CatalogueRow> {
    vec![
        // §4.1.1 Video and rendering
        row(
            "video.graphic_quality",
            Some("GraphicQuality"),
            Choice,
            "0",
            P,
            Imm,
        ),
        row("video.vsync", Some("VSync"), Choice, "0", U, Imm),
        row("video.frame_rate", Some("FrameRate"), Whole, "90", U, Imm),
        row("video.skip_frame", Some("SkipFrame"), Flag, "true", U, Imm),
        row(
            "video.render_method",
            Some("RenderMethod"),
            Choice,
            "0",
            U,
            Imm,
        ),
        row(
            "video.res_scale",
            Some("ResScaleIndex"),
            Choice,
            "0",
            U,
            Imm,
        ),
        row("video.anti_alias", Some("AntiAlias"), Choice, "0", U, Imm),
        row("render.mesh_algo", Some("RenderMode"), Choice, "0", U, Imm),
        row("render.shadow", Some("Shadow"), Flag, "true", U, Imm),
        row("ui.fullscreen", None, Flag, "false", U, Imm),
        // §4.1.2 Camera
        row("camera.fov", Some("Fov"), Real, "70", U, Imm),
        row("camera.local_cam", Some("LocalCam"), Flag, "false", P, Imm),
        row(
            "camera.smooth_time",
            Some("SmoothTime"),
            Real,
            "0.4",
            S,
            Imm,
        ),
        // §4.1.3 Light
        row(
            "light.direction",
            Some("Light.Direction"),
            Vector,
            "1,-1,1",
            P,
            Imm,
        ),
        row(
            "light.color",
            Some("Light.Color"),
            Colour,
            "#FFFFFFFF",
            P,
            Imm,
        ),
        row(
            "light.intensity",
            Some("Light.Intensity"),
            Real,
            "1",
            U,
            Imm,
        ),
        // §4.1.4 Background and skybox. AMD-031 retired scene.tv_box.
        row("scene.skybox", Some("Skybox"), Choice, "-1", U, Imm),
        row(
            "scene.custom_bg_color",
            Some("CustomBgColor"),
            Colour,
            "#000000FF",
            U,
            Imm,
        ),
        row(
            "scene.use_nes_bg_color",
            Some("BgColor"),
            Flag,
            "true",
            P,
            Imm,
        ),
        row("scene.map_enable", Some("MapEnable"), Flag, "false", P, No),
        // §4.1.5 Clipping: index 0 left, 1 bottom, 2 right, 3 top (DEC-R2-T1)
        row("clip.left", Some("Clipping[0]"), Real, "0.1", P, Imm),
        row("clip.bottom", Some("Clipping[1]"), Real, "8.1", P, Imm),
        row("clip.right", Some("Clipping[2]"), Real, "8.1", P, Imm),
        row("clip.top", Some("Clipping[3]"), Real, "0.1", P, Imm),
        // §4.1.6 Layers
        row(
            "layer.default_layer",
            Some("Layer.DefLayer"),
            Choice,
            "2",
            P,
            Game,
        ),
        row("layer.auto", Some("Layer.Auto"), Flag, "false", P, No),
    ]
}

/// §4.1.6: the six layers, with offsets 0, 0, 0, 12, 24, 36; alignment FRONT, CENTER then FRONT; the
/// derived coefficient; cast only for layer 2; receive for layers 2 to 5.
fn layer_rows() -> Vec<CatalogueRow> {
    let mut rows = Vec::new();
    let offsets = ["0", "0", "0", "12", "24", "36"];
    let alignments = ["0", "1", "0", "0", "0", "0"];
    let coefficients = ["1", "0", "1", "1", "1", "1"];
    let cast = ["false", "false", "true", "false", "false", "false"];
    let recv = ["false", "false", "true", "true", "true", "true"];
    for i in 0..6 {
        let key = |field: &str| format!("layer[{i}].{field}");
        let lua = |field: &str| format!("Layer[{i}].{field}");
        rows.push(row(
            &key("offset"),
            Some(&lua("Offset")),
            Real,
            offsets[i],
            P,
            Imm,
        ));
        rows.push(row(
            &key("aligned"),
            Some(&lua("Aligned")),
            Choice,
            alignments[i],
            P,
            Imm,
        ));
        rows.push(row(
            &key("coef"),
            Some(&lua("Coef")),
            Real,
            coefficients[i],
            D,
            No,
        ));
        rows.push(row(
            &key("cast_shadow"),
            Some(&lua("CastShadow")),
            Flag,
            cast[i],
            P,
            Imm,
        ));
        rows.push(row(
            &key("recv_shadow"),
            Some(&lua("RecvShadow")),
            Flag,
            recv[i],
            P,
            Imm,
        ));
    }
    rows
}

/// §4.1.7 and §4.1.8: emulation and conversion.
fn last_sections() -> Vec<CatalogueRow> {
    vec![
        // §4.1.7 Emulation
        row("emu.game_speed", Some("GameSpeed"), Real, "0", U, Imm),
        row("emu.tv_mode", Some("TVMode"), Choice, "0", P, Imm),
        row(
            "emu.auto_tv_mode",
            Some("AutoTVMode"),
            Flag,
            "false",
            U,
            Rom,
        ),
        row("emu.core", Some("Core"), Whole, "0", U, Imm),
        // §4.1.8 Conversion, all profile-scoped
        row(
            "conv.same_bg_palette",
            Some("SameBgPalette"),
            Flag,
            "false",
            P,
            Imm,
        ),
        row(
            "conv.same_sp_palette",
            Some("SameSpPalette"),
            Flag,
            "false",
            P,
            Imm,
        ),
        row(
            "conv.sprite_tolerance",
            Some("SpriteTolerance"),
            Whole,
            "0",
            P,
            Imm,
        ),
        row(
            "conv.scan_direction",
            Some("ScanDirection"),
            Choice,
            "0",
            P,
            Imm,
        ),
        row("conv.empty_tile", Some("EmptyTile"), Flag, "false", P, Imm),
    ]
}

/// §4.1.9, §4.1.10 and the settings this reimplementation adds.
fn tail_sections() -> Vec<CatalogueRow> {
    vec![
        // §4.1.9 Audio, inputs, interface
        row("audio.volume", Some("Volume"), Real, "1", U, Imm),
        row("audio.bg_music", Some("BGMusic"), Flag, "true", U, Imm),
        row("input.port_type[0]", None, Choice, "0", S, Imm),
        row("input.port_type[1]", None, Choice, "0", S, Imm),
        row(
            "input.zapper_enable",
            Some("ZapperEnable"),
            Flag,
            "false",
            D,
            No,
        ),
        row(
            "input.emulated_menu_button",
            Some("EmulatedMenuButton"),
            Flag,
            "false",
            U,
            No,
        ),
        row("ui.sort_option", Some("SortOption"), Choice, "0", U, Imm),
        row("ui.last_browse_dir", None, Text, "", U, Imm),
        row(
            "ui.window_rect",
            None,
            Rectangle,
            "-1,-1,1280,720",
            U,
            Restart,
        ),
        // §4.1.10 VR, editor, movie, internal and derived
        row("vr.runtime", Some("VrRuntime"), Whole, "0", U, No),
        row("vr.scene_ratio", Some("SceneRatio"), Real, "1", U, No),
        row("vr.mirroring", Some("Mirroring"), Flag, "true", U, No),
        row(
            "vr.control_method",
            Some("ControlMethod"),
            Whole,
            "0",
            U,
            No,
        ),
        row(
            "editor.selection_mode",
            Some("SelectionMode"),
            Choice,
            "0",
            S,
            No,
        ),
        row(
            "editor.saved_patterns",
            Some("SavedPattern3D"),
            Opaque,
            "",
            P,
            No,
        ),
        row(
            "editor.saved_anim_patterns",
            Some("SaveAnimPattern3D"),
            Opaque,
            "",
            P,
            No,
        ),
        row("movie.mode", Some("MovieMode"), Choice, "0", S, No),
        row("scene.root_scale", Some("RootScale"), Real, "1", S, Imm),
        row("scene.scene_scale", Some("SceneScale"), Real, "1", D, Imm),
        row(
            "scene.skybox_enable",
            Some("SkyboxEnable"),
            Flag,
            "false",
            D,
            No,
        ),
        row("video.res_scale_factor", Some("ResScale"), Real, "1", D, No),
        row(
            "video.target_framerate",
            Some("TargetFramerate"),
            Real,
            "90",
            D,
            No,
        ),
        row(
            "video.target_time_per_frame",
            Some("TargetTimePerFrame"),
            Real,
            "0.011111",
            D,
            No,
        ),
        row("emu.tv_data", Some("TVData"), Opaque, "", D, No),
        row("app.engine_mode", Some("EngineMode"), Choice, "1", S, No),
    ]
}

/// The text of `value` as the transcription writes a default: numbers with at most six decimals,
/// `true`/`false`, `#RRGGBBAA`, comma-separated vectors and rectangles, and nothing for opaque values.
#[must_use]
pub fn describe(value: &SettingValue) -> String {
    match value {
        SettingValue::Bool(flag) => flag.to_string(),
        SettingValue::Int(whole) | SettingValue::Enum(whole) => whole.to_string(),
        SettingValue::Float(real) => six_decimals(*real),
        SettingValue::Color(colour) => colour.to_string(),
        SettingValue::Vector3(vector) => format!("{},{},{}", vector.x, vector.y, vector.z),
        SettingValue::Text(text) => text.clone(),
        SettingValue::Rect(rect) => format!("{},{},{},{}", rect.x, rect.y, rect.width, rect.height),
        SettingValue::Opaque(_) => String::new(),
    }
}

/// `0.######`: at most six decimals, trailing zeros removed.
fn six_decimals(value: f32) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}
