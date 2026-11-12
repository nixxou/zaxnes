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
use std::sync::{Arc, Mutex};

use zax_render::scene::{BackdropState, SceneFrame, SceneState};
use zax_scripting::{
    lend_context, LuaColor, ProfileScriptRuntime, ScriptBackdrop, ScriptCamera, ScriptContext,
    ScriptEnvironment, ScriptInput, ScriptLight, ScriptLog, ScriptMemory, ScriptProfileInfo,
    ScriptNotification, ScriptSettings,
};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use crate::head_on_pull::{HeadOnPull, Pull};
use crate::scene_settings;

/// How fast the right stick turns the camera, in degrees a second at full tilt.
const TURN_SPEED: f32 = 90.0;
/// How fast the shoulder triggers bring the camera nearer or farther, a share of the distance
/// a second.
const ZOOM_SPEED: f32 = 1.0;
/// How long Y is held to see the game in 2D head on, and L2 to keep the camera (the
/// application's `CAMERA_KEEP_SECONDS`).
const HOLD_SECONDS: f32 = 0.6;

/// What a button of the camera did, for the frontend to tell the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraNews {
    /// L2 held: the camera's angles and distance are kept.
    Kept,
    /// L2 pressed: the kept angles and distance are back.
    Restored,
}

/// The two held buttons of the camera, as the application's handheld interface times them.
#[derive(Debug, Default)]
struct Buttons {
    /// How long Y has been held, and whether this hold already acted.
    flat: Option<(f32, bool)>,
    /// How long L2 has been held, while it is.
    memory: Option<f32>,
    /// Whether this hold of L2 already kept the camera.
    kept: bool,
    /// The kept angles and distance.
    saved: Option<(zax_render::numerics::Vector3, f32)>,
}

/// The render settings the core options choose, written in the user layer of the settings as
/// the application's options window writes them: a profile or a script may still change them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    /// `video.anti_alias`: 0 none, 1 MSAA 4x, 2 MSAA 8x, 3 SMAA.
    pub anti_alias: i32,
    /// `render.shadow`.
    pub shadows: bool,
    /// `render.visual_style` Enhanced (else Classic).
    pub enhanced: bool,
    /// `render.style.reflections` (the Enhanced style's).
    pub reflections: bool,
    /// `scene.skybox`: -1 the default, 0 colour, 1 gradient, 2 fantasy.
    pub skybox: i32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            anti_alias: 1,
            shadows: true,
            enhanced: false,
            reflections: false,
            skybox: -1,
        }
    }
}

impl RenderOptions {
    fn write(self, model: &mut SettingsModel) {
        model.set(SettingId::VideoAntiAlias, SettingValue::Enum(self.anti_alias));
        model.set(SettingId::RenderShadow, self.shadows);
        model.set(
            SettingId::RenderVisualStyle,
            SettingValue::Int(if self.enhanced { 4 } else { 0 }),
        );
        model.set(
            SettingId::RenderStyleReflections,
            SettingValue::Int(i32::from(self.reflections)),
        );
        model.set(SettingId::SceneSkybox, SettingValue::Enum(self.skybox));
    }
}

/// The 3D of the game in play.
pub struct ThreeD {
    engine: ConversionEngine,
    scene: SceneState,
    zapper_offset: Vec3,
    records: Vec<TileRecord>,
    picture: Vec<u8>,
    picture_height: usize,
    frame: i32,
    /// The settings changed since the scene last followed them (the scripts', the options').
    changes: Arc<Mutex<Vec<SettingId>>>,
    /// What the scripts raised through `NotiCenter` and the frontend has not shown yet: the
    /// text and how many seconds it stays.
    notices: Arc<Mutex<Vec<(String, f32)>>>,
    /// The pull on the head-on 2D picture.
    pull: HeadOnPull,
    /// Whether the scene was in 2D at the last frame.
    was_two_dimensional: bool,
    buttons: Buttons,
    /// The sides a television did not show, left out of the scene and of the 2D picture.
    pub overscan: crate::overscan::Overscan,
    /// `video.widescreen_strict_edges`.
    pub strict_edges: bool,
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
        render: RenderOptions,
        optimized_lua: bool,
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
            changes: Arc::default(),
            notices: Arc::default(),
            pull: HeadOnPull::new(),
            was_two_dimensional: false,
            buttons: Buttons::default(),
            overscan: crate::overscan::Overscan::default(),
            strict_edges: false,
            flat: false,
        };
        let mut model = SettingsModel::new();
        render.write(&mut model);
        let mut settings = ScriptSettings::new(model, ScriptLight::Model);
        let changes = Arc::clone(&three.changes);
        settings.set_on_changed(move |id| {
            if let Ok(mut changes) = changes.lock() {
                changes.push(id);
            }
        });
        let notices = Arc::clone(&three.notices);
        let environment = ScriptEnvironment {
            settings,
            log: Box::new(Log(log)),
            notify: Some(Box::new(move |notice: ScriptNotification| {
                if let Ok(mut notices) = notices.lock() {
                    notices.push((notice.message, notice.seconds));
                }
            })),
            optimized: optimized_lua,
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
        three.take_changes();
        if let Some(runtime) = three.engine.script_host_as::<ProfileScriptRuntime>() {
            let model = runtime.settings().model();
            scene_settings::push_settings(model, &mut three.scene);
            three
                .scene
                .settings
                .set_anti_alias(model.get(SettingId::VideoAntiAlias).as_int());
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
        let cut = self.overscan.in_force(batch.margin);
        if cut.is_none() {
            self.records.extend_from_slice(batch.records);
        } else {
            self.records.extend(
                batch
                    .records
                    .iter()
                    .filter(|record| cut.keeps(record.x, record.y)),
            );
        }
        // As the application: unless the edges are strict, every band counts as drawn across
        // the whole margin.
        let visible_rows = if self.strict_edges || batch.margin == 0 {
            batch.visible_rows
        } else {
            #[allow(clippy::cast_possible_truncation)] // At most 512.
            let margin = batch.margin as i16;
            [(-margin, 256 + margin); 30]
        };
        let batch = FrameBatch {
            records: &self.records,
            palette: batch.palette,
            margin_palette: batch.margin_palette,
            mask_x: batch.mask_x,
            mask_y: batch.mask_y,
            margin: batch.margin,
            visible_rows,
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
        self.follow_settings();
        self.picture_height = nes.wide_picture_into(&mut self.picture);
        let width = self.picture.len() / self.picture_height.max(1);
        self.overscan
            .in_force(i32::try_from(width.saturating_sub(256) / 2).unwrap_or(0))
            .leave_out(&mut self.picture, self.picture_height);
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
        // As in the application: going into 2D puts the picture head on.
        if two_dimensional && !self.was_two_dimensional {
            scene.picture_head_on = true;
        }
        self.was_two_dimensional = two_dimensional;
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
        if self.scene.two_dimensional_mode && self.scene.picture_head_on {
            self.pull_head_on(real_seconds, turn);
        } else {
            self.pull.clear();
            self.scene.picture_tilt = [0.0; 2];
            self.steer(real_seconds, turn, zoom);
        }
        self.scene.camera.advance(real_seconds);
        self.scene.real_time += real_seconds;
    }

    /// The 2D picture seen head on, as the application has it: the camera does not turn, the
    /// stick pulls the picture, which gives way to the camera past the gauge's limit.
    fn pull_head_on(&mut self, real_seconds: f32, turn: (f32, f32)) {
        let asked = [
            turn.1 * TURN_SPEED * real_seconds,
            turn.0 * TURN_SPEED * real_seconds,
        ];
        match self.pull.update(asked, real_seconds) {
            Pull::Held(tilt) => self.scene.picture_tilt = tilt,
            Pull::Released(tilt, whole) => {
                use zax_render::numerics::Vector3;
                let clipping = self.scene.picture_clipping();
                self.scene.camera.leave_picture_view(
                    clipping,
                    Vector3::new(tilt[0], tilt[1], 0.0),
                    Vector3::new(whole[0], whole[1], 0.0),
                );
                self.scene.picture_head_on = false;
                self.scene.picture_tilt = [0.0; 2];
            }
        }
    }

    fn steer(&mut self, real_seconds: f32, turn: (f32, f32), zoom: f32) {
        let camera = &mut self.scene.camera;
        if turn != (0.0, 0.0) || zoom != 0.0 {
            let setpoint = camera.current_mut();
            setpoint.angles.y += turn.0 * TURN_SPEED * real_seconds;
            setpoint.angles.x =
                (setpoint.angles.x + turn.1 * TURN_SPEED * real_seconds).clamp(-89.0, 89.0);
            let distance = setpoint.distance();
            setpoint.set_distance(distance * (1.0 + zoom * ZOOM_SPEED * real_seconds));
        }
    }

    /// Y and L2 for a frame of `real_seconds`, as the application's handheld interface has them.
    /// Y: a short press switches between 2D and 3D, a hold shows the game in 2D head on. L2: a
    /// hold keeps the camera's angles and distance, a short press puts them back.
    pub fn buttons(&mut self, real_seconds: f32, y: bool, l2: bool) -> Option<CameraNews> {
        let buttons = &mut self.buttons;
        if y {
            let (held, acted) = buttons.flat.get_or_insert((0.0, false));
            *held += real_seconds;
            if *held >= HOLD_SECONDS && !*acted {
                *acted = true;
                if !self.scene.two_dimensional_mode {
                    self.flat = true;
                }
                self.scene.picture_head_on = true;
                self.pull.clear();
            }
        } else if let Some((_, acted)) = buttons.flat.take() {
            if !acted {
                self.flat = !self.flat;
            }
        }
        if l2 {
            let held = buttons.memory.get_or_insert(0.0);
            *held += real_seconds;
            if *held >= HOLD_SECONDS && !buttons.kept {
                buttons.kept = true;
                let current = self.scene.camera.current();
                buttons.saved = Some((current.angles, current.distance()));
                return Some(CameraNews::Kept);
            }
        } else if buttons.memory.take().is_some() && !std::mem::take(&mut buttons.kept) {
            if let Some((angles, distance)) = buttons.saved {
                let setpoint = self.scene.camera.current_mut();
                setpoint.angles = angles;
                setpoint.set_distance(distance);
                return Some(CameraNews::Restored);
            }
        }
        None
    }

    /// Puts the render settings `render` chooses in force at once.
    pub fn set_render(&mut self, nes: &mut NesCore, render: RenderOptions) {
        let Some(runtime) = self.engine.script_host_as_mut::<ProfileScriptRuntime>() else {
            return;
        };
        render.write(runtime.settings_mut().model_mut());
        self.sync_settings(nes);
        self.follow_settings();
    }

    /// What the scripts raised through `NotiCenter` since the last call, for the frontend to
    /// show: the text and how many seconds it stays.
    #[must_use]
    pub fn take_notices(&self) -> Vec<(String, f32)> {
        self.notices
            .lock()
            .map(|mut notices| std::mem::take(&mut *notices))
            .unwrap_or_default()
    }

    fn take_changes(&self) -> Vec<SettingId> {
        self.changes
            .lock()
            .map(|mut changes| std::mem::take(&mut *changes))
            .unwrap_or_default()
    }

    /// Brings the scene to the settings that changed (the light, the field of view... that a
    /// script wrote), as the application does after each frame.
    fn follow_settings(&mut self) {
        let changes = self.take_changes();
        if changes.is_empty() {
            return;
        }
        let Some(runtime) = self.engine.script_host_as::<ProfileScriptRuntime>() else {
            return;
        };
        let model = runtime.settings().model();
        for id in changes {
            if id == SettingId::VideoAntiAlias {
                self.scene.settings.set_anti_alias(model.get(id).as_int());
            } else {
                scene_settings::apply_setting(id, model, &mut self.scene);
            }
        }
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
        // The head-on picture comes back with the default view.
        self.scene.picture_head_on = true;
        self.pull.clear();
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
