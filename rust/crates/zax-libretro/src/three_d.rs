//! The 3D of a game with a profile (step 2): the conversion engine of `zax-engine3d`, the
//! profile's Lua scripts of `zax-scripting` and the scene `zax-render` draws, put together as the
//! harness's headless runner does (`zax-harness/src/render/rom.rs`), which depends on the
//! application no more than this core may. The drawing itself is [`crate::ffi`]'s, into the
//! frontend's framebuffer.

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::frame::ConversionFrame;
use zax_engine3d::vectors::Vec3;
use zax_nes::{ControllerPortType, FrameBatch, NesCore, TileRecord};
use zax_profiles::hash::tiles::TileResolver;
use zax_profiles::profile::GameProfile;
use zax_render::camera::{CameraMode, SceneCamera};
use zax_render::scene::{BackdropState, SceneFrame, SceneState, SkyboxKind};
use zax_scripting::{
    lend_context, LuaColor, ProfileScriptRuntime, ScriptBackdrop, ScriptCamera, ScriptContext,
    ScriptEnvironment, ScriptInput, ScriptLight, ScriptLog, ScriptMemory, ScriptProfileInfo,
    ScriptSettings,
};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

/// How fast the right stick turns the camera, in degrees a second at full tilt.
const TURN_SPEED: f32 = 90.0;
/// How fast the shoulder triggers bring the camera nearer or farther, a share of the distance
/// a second.
const ZOOM_SPEED: f32 = 1.0;

/// The 3D of the game in play.
pub struct ThreeD {
    engine: ConversionEngine,
    scene: SceneState,
    zapper_offset: Vec3,
    records: Vec<TileRecord>,
    picture: Vec<u8>,
    picture_height: usize,
    frame: i32,
    /// The 2D view of a profile's 3D game, the player's choice (L3).
    pub flat: bool,
}

impl ThreeD {
    /// Loads `profile` (its tiles resolved by `tiles` for a "3dh" one) onto the game `nes` has
    /// loaded, runs the `Start` hooks and resets the machine (REQ-LUA-015 steps 1 to 3).
    /// `log` receives what the scripts print and their errors.
    ///
    /// # Errors
    ///
    /// Why the scripts or the profile were refused.
    pub fn start(
        nes: &mut NesCore,
        profile: &GameProfile,
        tiles: Option<TileResolver>,
        log: fn(&str),
    ) -> Result<Self, String> {
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine.two_dimensional_mode =
            profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty();
        let mut three = Self {
            engine,
            scene: SceneState::default(),
            zapper_offset: Vec3::default(),
            records: Vec::new(),
            picture: Vec::new(),
            picture_height: 240,
            frame: 0,
            flat: false,
        };
        let model = SettingsModel::new();
        push_settings(&model, &mut three.scene);
        let environment = ScriptEnvironment {
            settings: ScriptSettings::new(model, ScriptLight::Model),
            log: Box::new(Log(log)),
            profile: ScriptProfileInfo {
                title: profile.title.clone().unwrap_or_default(),
                maker: profile.author.clone().unwrap_or_default(),
                game_dev: profile.publisher.clone().unwrap_or_default(),
                description: profile.description.clone().unwrap_or_default(),
                id: profile.id.clone().unwrap_or_default(),
                version: profile.version_text.clone().unwrap_or_default(),
            },
            ..ScriptEnvironment::default()
        };
        let Self {
            engine,
            scene,
            zapper_offset,
            ..
        } = &mut three;
        let runtime = with_context(scene, nes, zapper_offset, |context| {
            ProfileScriptRuntime::create_in(profile, environment, engine.state_mut(), Some(context))
        })
        .map_err(|error| error.to_string())?;
        three.engine.set_script_host(Box::new(runtime));
        three.sync_settings(nes);
        let mut loaded = Ok(());
        let mut tiles = tiles;
        three.scripted(nes, |engine| {
            loaded = engine.load_profile_with_tiles(profile, true, tiles.take());
        });
        loaded.map_err(|error| error.to_string())?;
        three.sync_settings(nes);
        if let Some(model) = three.model() {
            let model = model.clone();
            push_settings(&model, &mut three.scene);
        }
        three.with_runtime(nes, ProfileScriptRuntime::invoke_start);
        nes.reset();
        three.scene.camera.snap_to_setpoints();
        three.scene.game_running = true;
        Ok(three)
    }

    /// Converts the frame `nes` just ran (its scripts' hooks run here) and makes the scene
    /// follow it.
    pub fn convert(&mut self, nes: &mut NesCore) {
        let batch = nes.frame_batch();
        self.records.clear();
        self.records.extend_from_slice(batch.records);
        let batch = FrameBatch {
            records: &self.records,
            palette: batch.palette,
            margin_palette: batch.margin_palette,
            mask_x: batch.mask_x,
            mask_y: batch.mask_y,
            margin: batch.margin,
            visible_rows: batch.visible_rows,
            margin_y: batch.margin_y,
        };
        let counter = self.frame;
        self.frame = self.frame.wrapping_add(1);
        let chr_ram = nes.chr_ram();
        self.engine.offer_chr_ram(chr_ram);
        let Self {
            engine,
            scene,
            zapper_offset,
            ..
        } = self;
        with_context(scene, nes, zapper_offset, |context| {
            lend_context(
                engine,
                |engine| engine,
                context,
                |engine| {
                    engine.convert(&batch, counter);
                },
            );
        });
        self.engine.publish();
        self.sync_settings(nes);
        self.picture_height = nes.wide_picture_into(&mut self.picture);
        self.follow_frame();
    }

    /// The scene's fields the frame decides, as the application's `update_scene` sets them.
    fn follow_frame(&mut self) {
        let scene = &mut self.scene;
        let published = self.engine.published_frame();
        scene.nes_background_color = self.engine.frame_color(0);
        scene.emulated_time = published.map_or(0.0, ConversionFrame::time);
        scene.clipping = self.engine.settings().clipping;
        let two_dimensional = self.engine.two_dimensional_mode || self.flat;
        scene.two_dimensional_mode = two_dimensional;
        #[allow(clippy::cast_precision_loss)] // Small margins.
        if two_dimensional {
            let width = self.picture.len() / self.picture_height.max(1);
            scene.margin = (width.saturating_sub(256) / 2) as f32;
            scene.margin_y = (self.picture_height.saturating_sub(240) / 2) as f32;
        } else {
            scene.margin = published.map_or(0, ConversionFrame::margin) as f32;
            scene.margin_y = published.map_or(0, ConversionFrame::margin_y) as f32;
        }
        scene.followed_shape = self
            .engine
            .script_host_as::<ProfileScriptRuntime>()
            .and_then(ProfileScriptRuntime::fpv_shape);
    }

    /// Moves the camera along `real_seconds` of real time, turned by the right stick (`turn`,
    /// each axis -1 to 1) and brought nearer or farther by `zoom` (-1 to 1).
    pub fn advance(&mut self, real_seconds: f32, turn: (f32, f32), zoom: f32) {
        let camera = &mut self.scene.camera;
        if turn != (0.0, 0.0) || zoom != 0.0 {
            let setpoint = camera.current_mut();
            setpoint.angles.y += turn.0 * TURN_SPEED * real_seconds;
            setpoint.angles.x =
                (setpoint.angles.x + turn.1 * TURN_SPEED * real_seconds).clamp(-89.0, 89.0);
            let distance = setpoint.distance();
            setpoint.set_distance(distance * (1.0 + zoom * ZOOM_SPEED * real_seconds));
        }
        camera.advance(real_seconds);
        self.scene.real_time += real_seconds;
    }

    /// The scripts' 64 `PData` values, the block a state carries (REQ-LUA-071).
    #[must_use]
    pub fn persistent_data(&self) -> Option<[u8; zax_scripting::persistent::BLOCK_SIZE]> {
        self.engine
            .script_host_as::<ProfileScriptRuntime>()
            .and_then(|runtime| runtime.capture_persistent_data().ok())
    }

    /// Puts a state's `PData` values back (REQ-LUA-072).
    pub fn restore_persistent_data(&self, block: &[u8]) {
        if let Some(runtime) = self.engine.script_host_as::<ProfileScriptRuntime>() {
            let _ = runtime.restore_persistent_data(block);
        }
    }

    /// The camera back where the profile puts it.
    pub fn reset_view(&mut self) {
        self.scene.camera.reset_view();
    }

    /// The scene to draw.
    #[must_use]
    pub const fn scene(&self) -> &SceneState {
        &self.scene
    }

    /// The scene and the frame to draw in it: the draw list in 3D, the game's picture in 2D.
    pub fn scene_and_frame(&mut self) -> (&SceneState, SceneFrame<'_>) {
        if self.scene.two_dimensional_mode {
            return (
                &self.scene,
                SceneFrame {
                    draw_list: None,
                    nes_picture: &self.picture,
                    nes_picture_height: self.picture_height,
                },
            );
        }
        (
            &self.scene,
            SceneFrame::with_draw_list(self.engine.build_draw_list()),
        )
    }

    fn model(&self) -> Option<&SettingsModel> {
        self.engine
            .script_host_as::<ProfileScriptRuntime>()
            .map(|runtime| runtime.settings().model())
    }

    fn scripted(&mut self, nes: &mut NesCore, operation: impl FnOnce(&mut ConversionEngine)) {
        let Self {
            engine,
            scene,
            zapper_offset,
            ..
        } = self;
        with_context(scene, nes, zapper_offset, |context| {
            lend_context(engine, |engine| engine, context, operation);
        });
    }

    fn with_runtime(
        &mut self,
        nes: &mut NesCore,
        operation: impl FnOnce(&mut ProfileScriptRuntime, &mut zax_engine3d::engine::ConversionState),
    ) {
        self.scripted(nes, |engine| {
            engine.with_script_host::<ProfileScriptRuntime, _>(operation);
        });
    }

    fn sync_settings(&mut self, nes: &mut NesCore) {
        self.with_runtime(nes, ProfileScriptRuntime::sync_settings);
    }
}

/// What the scripts print and their errors, to the frontend's log.
struct Log(fn(&str));

impl ScriptLog for Log {
    fn print(&mut self, line: &str) {
        (self.0)(&format!("Lua: {line}"));
    }

    fn error(&mut self, line: &str) {
        (self.0)(line);
    }

    fn warning(&mut self, line: &str) {
        (self.0)(line);
    }
}

/// What the scripts read of the machine and write of the Zapper aim, for one call.
struct CoreMachine<'a> {
    core: &'a mut NesCore,
    zapper_offset: &'a mut Vec3,
}

impl ScriptInput for CoreMachine<'_> {
    fn player_state(&self, _index: usize) -> i32 {
        0
    }

    fn global_state(&self) -> i32 {
        0
    }

    fn port_byte(&self, port: usize) -> i32 {
        if port < 2 {
            i32::from(self.core.bus().input().port_byte(port))
        } else {
            0
        }
    }

    fn port_type(&self, port: usize) -> i32 {
        self.core.port_type(port).map_or(0, |kind| kind as i32)
    }

    fn set_port_types(&mut self, first: i32, second: i32) {
        for (port, kind) in [first, second].into_iter().enumerate() {
            let kind = if kind.clamp(0, 1) == 1 {
                ControllerPortType::Zapper
            } else {
                ControllerPortType::Controller
            };
            // Both ports exist: this cannot fail.
            let _ = self.core.set_port_type(port, kind);
        }
    }

    fn zapper_offset(&self) -> Vec3 {
        *self.zapper_offset
    }

    fn set_zapper_offset(&mut self, offset: Vec3) {
        *self.zapper_offset = offset;
    }
}

impl ScriptMemory for CoreMachine<'_> {
    fn read(&self, address: u16) -> u8 {
        self.core.read_memory(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.core.write_memory(address, value);
    }
}

fn with_context<R>(
    scene: &mut SceneState,
    core: &mut NesCore,
    zapper_offset: &mut Vec3,
    operation: impl FnOnce(&mut ScriptContext<'_>) -> R,
) -> R {
    let mut camera = SceneCameraBridge(&mut scene.camera);
    let mut backdrop = SceneBackdropBridge(&mut scene.backdrop);
    let mut machine = CoreMachine {
        core,
        zapper_offset,
    };
    operation(&mut ScriptContext {
        camera: &mut camera,
        backdrop: &mut backdrop,
        machine: &mut machine,
    })
}

/// The camera of the scene as scripts see it.
struct SceneCameraBridge<'a>(&'a mut SceneCamera);

impl ScriptCamera for SceneCameraBridge<'_> {
    fn angles(&self) -> Vec3 {
        self.0.current().angles.into()
    }

    fn set_angles(&mut self, angles: Vec3) {
        self.0.current_mut().angles = angles.into();
    }

    fn rot_override(&self) -> Option<Vec3> {
        self.0.rot_override().map(Into::into)
    }

    fn set_rot_override(&mut self, angles: Option<Vec3>) {
        self.0.set_rot_override(angles.map(Into::into));
    }

    fn distance(&self) -> f32 {
        self.0.current().distance()
    }

    fn set_distance(&mut self, distance: f32) {
        self.0.current_mut().set_distance(distance);
    }

    fn mode(&self) -> i32 {
        self.0.mode() as i32
    }

    fn set_mode(&mut self, mode: i32) {
        self.0.set_mode(if mode == 1 {
            CameraMode::Fpv
        } else {
            CameraMode::Tpv
        });
    }

    fn look_at(&self) -> Vec3 {
        self.0.current().target.into()
    }

    fn set_look_at(&mut self, target: Vec3) {
        self.0.current_mut().target = target.into();
    }

    fn reset_view(&mut self) {
        self.0.reset_view();
    }

    fn fov(&self) -> f32 {
        self.0.fov()
    }

    fn set_fov(&mut self, fov: f32) {
        self.0.set_fov(fov);
    }

    fn reset_distance(&self) -> f32 {
        self.0.reset_distance()
    }
}

/// The backdrop panel of the scene as scripts see it.
struct SceneBackdropBridge<'a>(&'a mut BackdropState);

impl ScriptBackdrop for SceneBackdropBridge<'_> {
    fn enabled(&self) -> bool {
        self.0.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.0.enabled = enabled;
    }

    fn color(&self) -> LuaColor {
        let color = self.0.color;
        LuaColor::from_bytes(color.r, color.g, color.b)
    }

    fn set_color(&mut self, color: LuaColor) {
        self.0.color = color.to_rgb24();
    }

    fn receive_shadow(&self) -> bool {
        self.0.receive_shadow
    }

    fn set_receive_shadow(&mut self, receive: bool) {
        self.0.receive_shadow = receive;
    }

    fn size(&self) -> Vec3 {
        self.0.size.into()
    }

    fn set_size(&mut self, size: Vec3) {
        self.0.size = size.into();
    }

    fn offset(&self) -> Vec3 {
        self.0.offset.into()
    }

    fn set_offset(&mut self, offset: Vec3) {
        self.0.offset = offset.into();
    }

    fn reset(&mut self) {
        self.0.reset();
    }
}

const fn skybox_of(value: i32) -> SkyboxKind {
    match value {
        0 => SkyboxKind::Color,
        1 => SkyboxKind::Gradient,
        2 => SkyboxKind::Fantasy,
        _ => SkyboxKind::Default,
    }
}

fn color_of(model: &SettingsModel, id: SettingId) -> zax_engine3d::palette::Rgb24 {
    let color = model.get(id).as_color();
    LuaColor::new(color.r, color.g, color.b, color.a).to_rgb24()
}

/// The scene-owned settings a profile or a script set (as the harness pushes them): the field
/// of view, the camera smoothing, the shadows, the sky, the background and the light.
fn push_settings(model: &SettingsModel, scene: &mut SceneState) {
    let fov = model.get(SettingId::CameraFov).as_float();
    scene.settings.fov = fov;
    scene.camera.set_fov(fov);
    scene.camera.smooth_time = model.get(SettingId::CameraSmoothTime).as_float();
    scene.camera.local_camera = model.get(SettingId::CameraLocalCam).as_bool();
    scene.settings.shadows = model.get(SettingId::RenderShadow).as_bool();
    scene.settings.skybox = skybox_of(model.get(SettingId::SceneSkybox).as_int());
    scene.settings.custom_background_color = color_of(model, SettingId::SceneCustomBgColor);
    scene.use_nes_background_color = model.get(SettingId::SceneUseNesBgColor).as_bool();
    let direction = model.get(SettingId::LightDirection).as_vector3();
    scene.light.direction =
        zax_render::numerics::Vector3::new(direction.x, direction.y, direction.z);
    scene.light.color = color_of(model, SettingId::LightColor);
}
