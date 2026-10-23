//! What the Lua environment sees of the scene (SPEC-05 REQ-LUA-088, REQ-LUA-090) and how the
//! settings model reaches the scene (SPEC-09 REQ-PERS-007).
//!
//! The scene is shared between the loop, which owns the renderer's inputs, and the camera and
//! backdrop the scripts drive: the script environment keeps `'static` trait objects, so those
//! cannot borrow it. No borrow is ever held across a call into the session, so the runtime
//! borrow checks of the [`RefCell`] cannot fail.

use std::cell::RefCell;
use std::rc::Rc;

use zax_engine3d::palette::Rgb24;
use zax_engine3d::vectors::Vec3;
use zax_profiles::values::Vec4F;
use zax_render::camera::CameraMode;
use zax_render::scene::{SceneState, SkyboxKind};
use zax_scripting::{LuaColor, ScriptBackdrop, ScriptCamera, ScriptLog};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

/// The scene, shared by the loop and the script bridges.
pub type SharedScene = Rc<RefCell<SceneState>>;

/// The camera of the scene as scripts see it.
pub struct SceneCameraBridge(pub SharedScene);

impl ScriptCamera for SceneCameraBridge {
    fn angles(&self) -> Vec3 {
        self.0.borrow().camera.current().angles.into()
    }

    fn set_angles(&mut self, angles: Vec3) {
        self.0.borrow_mut().camera.current_mut().angles = angles.into();
    }

    fn distance(&self) -> f32 {
        // The setpoint bounds the distance to [0, 512] itself (REQ-SCN-011).
        self.0.borrow().camera.current().distance()
    }

    fn set_distance(&mut self, distance: f32) {
        self.0.borrow_mut().camera.current_mut().set_distance(distance);
    }

    fn mode(&self) -> i32 {
        self.0.borrow().camera.mode() as i32
    }

    fn set_mode(&mut self, mode: i32) {
        let mode = if mode == 1 { CameraMode::Fpv } else { CameraMode::Tpv };
        self.0.borrow_mut().camera.set_mode(mode);
    }

    fn look_at(&self) -> Vec3 {
        self.0.borrow().camera.current().target.into()
    }

    fn set_look_at(&mut self, target: Vec3) {
        self.0.borrow_mut().camera.current_mut().target = target.into();
    }

    fn reset_view(&mut self) {
        self.0.borrow_mut().camera.reset_view();
    }
}

/// The backdrop panel of the scene as scripts see it.
pub struct SceneBackdropBridge(pub SharedScene);

impl ScriptBackdrop for SceneBackdropBridge {
    fn enabled(&self) -> bool {
        self.0.borrow().backdrop.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.0.borrow_mut().backdrop.enabled = enabled;
    }

    fn color(&self) -> LuaColor {
        let color = self.0.borrow().backdrop.color;
        LuaColor::from_bytes(color.r, color.g, color.b)
    }

    fn set_color(&mut self, color: LuaColor) {
        self.0.borrow_mut().backdrop.color = color.to_rgb24();
    }

    fn receive_shadow(&self) -> bool {
        self.0.borrow().backdrop.receive_shadow
    }

    fn set_receive_shadow(&mut self, receive: bool) {
        self.0.borrow_mut().backdrop.receive_shadow = receive;
    }

    fn size(&self) -> Vec3 {
        self.0.borrow().backdrop.size.into()
    }

    fn set_size(&mut self, size: Vec3) {
        self.0.borrow_mut().backdrop.size = size.into();
    }

    fn offset(&self) -> Vec3 {
        self.0.borrow().backdrop.offset.into()
    }

    fn set_offset(&mut self, offset: Vec3) {
        self.0.borrow_mut().backdrop.offset = offset.into();
    }

    fn reset(&mut self) {
        self.0.borrow_mut().backdrop.reset();
    }
}

/// The script log that goes to the console.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsoleScriptLog;

impl ScriptLog for ConsoleScriptLog {
    fn print(&mut self, line: &str) {
        println!("Lua: {line}");
    }

    fn error(&mut self, line: &str) {
        eprintln!("{line}");
    }

    fn warning(&mut self, line: &str) {
        println!("{line}");
    }
}

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
    scene.light.direction = zax_render::numerics::Vector3::new(direction.x, direction.y, direction.z);
    scene.light.color = color_of(model, SettingId::LightColor);
    scene
        .settings
        .set_light_intensity(model.get(SettingId::LightIntensity).as_float());
}

/// Pushes the profile's settings into the scene: the field of view, the shadows, the skybox,
/// the background, the clipping and the light (SPEC-09 REQ-PERS-005).
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
        // Every other setting is held in the session layer and read from there.
        _ => {}
    }
}
