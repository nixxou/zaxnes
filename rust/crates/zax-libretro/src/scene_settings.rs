//! How the settings model reaches the scene: a copy of the part of
//! `zax-app/src/scene_script_bridge.rs` that does it (from `skybox_of` to `apply_setting`), the
//! application crate being out of a core's reach. Keep the two the same.

use zax_engine3d::palette::Rgb24;
use zax_engine3d::space_bend::SpaceBend;
use zax_profiles::values::Vec4F;
use zax_render::scene::{SceneState, SkyboxKind};
use zax_render::style::{AmbientOcclusion, StyleSettings};
use zax_scripting::LuaColor;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

/// The skybox of a setting value.
#[must_use]
pub const fn skybox_of(value: i32) -> SkyboxKind {
    match value {
        0 => SkyboxKind::Color,
        1 => SkyboxKind::Gradient,
        2 => SkyboxKind::Fantasy,
        _ => SkyboxKind::Default,
    }
}

fn color_of(model: &SettingsModel, id: SettingId) -> Rgb24 {
    let color = model.get(id).as_color();
    LuaColor::new(color.r, color.g, color.b, color.a).to_rgb24()
}

/// The four clip margins as the scene keeps them: left, bottom, right, top.
#[must_use]
pub fn clipping_of(model: &SettingsModel) -> Vec4F {
    Vec4F::new(
        model.get(SettingId::ClipLeft).as_float(),
        model.get(SettingId::ClipBottom).as_float(),
        model.get(SettingId::ClipRight).as_float(),
        model.get(SettingId::ClipTop).as_float(),
    )
}

fn push_light(model: &SettingsModel, scene: &mut SceneState) {
    let direction = model.get(SettingId::LightDirection).as_vector3();
    scene.light.direction =
        zax_render::numerics::Vector3::new(direction.x, direction.y, direction.z);
    scene.light.color = color_of(model, SettingId::LightColor);
    scene
        .settings
        .set_light_intensity(model.get(SettingId::LightIntensity).as_float());
}

/// The curvature of AMD-043 REQ-SCN-067, built from the four settings of REQ-SCN-066.
///
/// Read **only** when one of the five curvature settings changes, never per frame, per tile or
/// per pixel: `SpaceBend::new` evaluates three sines and three cosines, which belongs nowhere
/// near a frame path. At the default settings every angle is 0 and the result is the identity,
/// which the renderer recognises in one test.
#[must_use]
pub fn bend_of(model: &SettingsModel) -> SpaceBend {
    #[allow(clippy::cast_precision_loss)] // Thirteen rungs from −180 to 180.
    let degrees = |id: SettingId| model.get(id).as_int() as f32;
    SpaceBend::new(
        degrees(SettingId::SceneBendLr),
        degrees(SettingId::SceneBendTb),
        degrees(SettingId::SceneArcH),
        // RO-5 part 2: a fraction of the way between the two heights of REQ-SCN-066, not a flag.
        // Read as a real, 0.5 being one of the five rungs the setting now publishes.
        model.get(SettingId::SceneArcCenter).as_float(),
        // The scene scale of REQ-SCN-002 is 1 outside virtual reality, which Zaxnes has none of.
        1.0,
    )
}

/// The visual style of AMD-055 REQ-SCN-078 and its twelve parameters of REQ-SCN-079.
///
/// Read **only** when the style or one of the twelve changes, never per frame, per tile or per
/// pixel (SPEC-09 §4.1: scope U, effect immediate). Thirteen model lookups is more than a frame
/// path should ever do, and the renderer needs none of them again: it keeps the value and tests
/// four predicates.
///
/// The four curvature parameters are **not** read here. REQ-SCN-078 keeps the curvature outside
/// the style — it acts in Classic as in Enhanced **(D)** — so they stay with `bend_of`, and a
/// change of style cannot lose a curvature setting (AMD-055 §3 point 4).
#[must_use]
pub fn style_of(model: &SettingsModel) -> StyleSettings {
    let level = |id: SettingId| model.get(id).as_int();
    StyleSettings {
        // REQ-SCN-078: 4 and the accepted 1 are Enhanced; 0 and every other value, including one
        // that comes from the settings file, are Classic.
        enhanced: StyleSettings::style_of(level(SettingId::RenderVisualStyle)),
        ao: AmbientOcclusion::of(level(SettingId::RenderStyleAo)),
        ao_strength: level(SettingId::RenderStyleAoStrength),
        ao_radius: level(SettingId::RenderStyleAoRadius),
        ao_thickness: level(SettingId::RenderStyleAoThickness),
        ao_quality: level(SettingId::RenderStyleAoQuality),
        pixel_shadow: level(SettingId::RenderStylePixelShadow) != 0,
        voxel_grid: level(SettingId::RenderStyleVoxelGrid),
        reflections: level(SettingId::RenderStyleReflections) != 0,
        ssr_shine: level(SettingId::RenderStyleSsrShine),
        ssr_clarity: level(SettingId::RenderStyleSsrClarity),
        ssr_length: level(SettingId::RenderStyleSsrLength),
        ssr_quality: level(SettingId::RenderStyleSsrQuality),
    }
}

/// Whether `id` is the style or one of its twelve parameters, so that a write reaches the scene
/// at the next rendered frame (REQ-SCN-078: "sans recharger le jeu").
const fn is_style_setting(id: SettingId) -> bool {
    matches!(
        id,
        SettingId::RenderVisualStyle
            | SettingId::RenderStyleAo
            | SettingId::RenderStyleAoStrength
            | SettingId::RenderStyleAoRadius
            | SettingId::RenderStyleAoThickness
            | SettingId::RenderStyleAoQuality
            | SettingId::RenderStylePixelShadow
            | SettingId::RenderStyleVoxelGrid
            | SettingId::RenderStyleReflections
            | SettingId::RenderStyleSsrShine
            | SettingId::RenderStyleSsrClarity
            | SettingId::RenderStyleSsrLength
            | SettingId::RenderStyleSsrQuality
    )
}

/// Pushes the profile's settings into the scene: the field of view, the shadows, the skybox,
/// the background, the clipping, the curvature and the light (SPEC-09 REQ-PERS-005).
pub fn push_settings(model: &SettingsModel, scene: &mut SceneState) {
    let fov = model.get(SettingId::CameraFov).as_float();
    scene.settings.fov = fov;
    scene.camera.set_fov(fov);
    scene.camera.smooth_time = model.get(SettingId::CameraSmoothTime).as_float();
    scene.camera.local_camera = model.get(SettingId::CameraLocalCam).as_bool();
    scene.settings.shadows = model.get(SettingId::RenderShadow).as_bool();
    scene.settings.skybox = skybox_of(model.get(SettingId::SceneSkybox).as_int());
    scene.settings.custom_background_color = color_of(model, SettingId::SceneCustomBgColor);
    scene.use_nes_background_color = model.get(SettingId::SceneUseNesBgColor).as_bool();
    scene.clipping = clipping_of(model);
    scene.bend = bend_of(model);
    scene.bend_two_dimensional_mode = model.get(SettingId::SceneBend2dMode).as_bool();
    scene.settings.backdrop_lighting = model.get(SettingId::RenderBackdropLighting).as_bool();
    scene.settings.style = style_of(model);
    push_light(model, scene);
}

/// Applies the change of one setting to the scene.
pub fn apply_setting(id: SettingId, model: &SettingsModel, scene: &mut SceneState) {
    match id {
        SettingId::LightDirection | SettingId::LightColor | SettingId::LightIntensity => {
            // The three light values are settings of §4.1.3; the renderer's light is a working
            // copy the model pushes into.
            push_light(model, scene);
        }
        SettingId::CameraFov => {
            let fov = model.get(id).as_float();
            scene.settings.fov = fov;
            scene.camera.set_fov(fov);
        }
        SettingId::CameraSmoothTime => scene.camera.smooth_time = model.get(id).as_float(),
        SettingId::CameraLocalCam => scene.camera.local_camera = model.get(id).as_bool(),
        SettingId::RenderShadow => scene.settings.shadows = model.get(id).as_bool(),
        SettingId::SceneSkybox => scene.settings.skybox = skybox_of(model.get(id).as_int()),
        SettingId::SceneCustomBgColor => {
            scene.settings.custom_background_color = color_of(model, id);
        }
        SettingId::SceneUseNesBgColor => scene.use_nes_background_color = model.get(id).as_bool(),
        SettingId::ClipLeft | SettingId::ClipBottom | SettingId::ClipRight | SettingId::ClipTop => {
            // The clip margins frame the camera and cut the tiles (REQ-E3D-410), so a Lua write
            // has to reach the scene at the next frame (REQ-PERS-007).
            scene.clipping = clipping_of(model);
        }
        // AMD-043 REQ-SCN-066: the four curvature settings apply hot, from the next rendered
        // frame. The three sines and cosines of `SpaceBend::new` are paid here, on a change,
        // and never again.
        SettingId::SceneBendLr
        | SettingId::SceneBendTb
        | SettingId::SceneArcH
        | SettingId::SceneArcCenter => scene.bend = bend_of(model),
        // AMD-046 REQ-SCN-074: likewise, and without effect in 3D mode.
        SettingId::SceneBend2dMode => {
            scene.bend_two_dimensional_mode = model.get(id).as_bool();
        }
        // AMD-049 REQ-SCN-075: likewise, and without effect on a scene with no backdrop. The
        // renderer compiles the backdrop program on the first frame that wants it and keeps it
        // afterwards, so turning this off and on again costs one boolean either way.
        SettingId::RenderBackdropLighting => {
            scene.settings.backdrop_lighting = model.get(id).as_bool();
        }
        // AMD-055 REQ-SCN-078: the style and its twelve parameters apply to the next rendered
        // frame, without reloading the game. The thirteen model lookups are paid here, on a
        // change, and never on a frame path.
        _ if is_style_setting(id) => scene.settings.style = style_of(model),
        // Every other setting is held in the session layer and read from there.
        _ => {}
    }
}
