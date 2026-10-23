//! Runs a real ROM headless for a number of emulated frames and captures the last one: the
//! "load a ROM and a profile, replay, export an offscreen capture" of SPEC-01 REQ-ARCH-031 (port
//! of `RenderExport.FromRom` and `ScriptBridge.cs`).
//!
//! Like the C# harness it does not depend on the application: the few views the Lua runtime
//! needs on the scene and on the machine are built here. They are lent to the scripts for the
//! length of one call into the engine, as `zax-scripting` requires.

use std::fs;
use std::path::Path;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::vectors::Vec3;
use zax_nes::{ControllerPortType, FrameBatch, NesCore, TileRecord};
use zax_profiles::profile::GameProfile;
use zax_profiles::setting_application;
use zax_render::camera::{CameraMode, SceneCamera};
use zax_render::scene::{BackdropState, SceneState, SkyboxKind};
use zax_scripting::{
    LoadedScript, LuaColor, ProfileScriptRuntime, ScriptBackdrop, ScriptBridge, ScriptCamera,
    ScriptContext, ScriptEnvironment, ScriptInput, ScriptLight, ScriptLog, ScriptMemory,
    ScriptProfileInfo, ScriptSettings,
};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::frame_json;
use super::input_script::InputScript;
use super::scenes::SyntheticScene;
use super::timed_host::TimedHost;
use crate::HarnessError;

/// What `render --rom` runs.
pub struct RomRequest<'a> {
    /// The `.nes` file.
    pub rom: &'a Path,
    /// Emulated frames to run before the capture (at least one is run).
    pub frames: u32,
    /// The profile to play.
    pub profile: &'a GameProfile,
    /// The mesh algorithm (REQ-E3D-340).
    pub mesh: RenderAlgorithm,
    /// The port 1 buttons to hold.
    pub input: &'a InputScript,
    /// Battery-backed PRG-RAM imported before the reset (REQ-CORE-095).
    pub battery: Option<&'a Path>,
    /// Where to write the captured frame as a `--frames` JSON file.
    pub dump_frame: Option<&'a Path>,
}

/// Runs the ROM and builds the scene of its last frame.
///
/// # Errors
///
/// [`HarnessError::Rom`] when the ROM is refused, [`HarnessError::Script`] when the Lua runtime
/// cannot be installed, [`HarnessError::Engine`] when the engine refuses the profile and
/// [`HarnessError::Io`] when a file cannot be read or written.
pub fn capture(request: &RomRequest<'_>) -> Result<SyntheticScene, HarnessError> {
    let profile = request.profile;
    let file_name = request
        .rom
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());

    let mut core = NesCore::new();
    let load = core.load_rom(&fs::read(request.rom)?, &file_name);
    if !load.success {
        return Err(HarnessError::Rom(format!(
            "Invalid rom file. ({})",
            load.error.unwrap_or_default()
        )));
    }

    core.set_region(load.region);
    if let Some(battery) = request.battery {
        // REQ-CORE-095: the battery RAM is imported before the reset, as the application does,
        // so the run starts from the player's saved game.
        core.import_battery_ram(&fs::read(battery)?);
    }

    let mut scene = SceneState::default();
    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.settings_mut().render_algorithm = request.mesh;
    engine.two_dimensional_mode =
        profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty();

    // The mesh algorithm is a user preference a profile does not override (REQ-PROF-041).
    let mut model = SettingsModel::new();
    model.set(
        SettingId::RenderMeshAlgo,
        SettingValue::Enum(mesh_setting(request.mesh)),
    );

    let mut host = Host {
        core,
        engine,
        bridge: None,
        zapper_offset: Vec3::default(),
    };
    host.install_runtime(profile, model, &mut scene)?;
    host.load_profile(profile, &mut scene)?;
    if let Some(model) = host.model() {
        push_settings(model, &mut scene);
    }
    host.with_runtime(&mut scene, ProfileScriptRuntime::invoke_start);

    host.core.reset();
    let frames = request.frames.max(1);
    let mut records = Vec::new();
    for counter in 0..frames {
        host.run_frame(counter, request.input.at(counter), &mut records, &mut scene)?;
    }

    host.with_runtime(&mut scene, ProfileScriptRuntime::invoke_end);
    report_scripts(&host.engine);

    if let Some(path) = request.dump_frame {
        let batch = host.core.frame_batch();
        fs::write(path, frame_json::write(batch.records, &batch.palette)?)?;
        println!("render: frame written to {}", path.display());
    }

    // What a script wrote to the light during the run (the C# harness bridges it live).
    if let Some(model) = host.model() {
        push_scripted_light(model, &mut scene);
    }

    let picture = host.core.pixel_frame().to_vec();
    let mut engine = host.engine;
    scene.camera.snap_to_setpoints();
    scene.game_running = true;
    scene.clipping = engine.settings().clipping;
    scene.nes_background_color = engine.frame_color(0);
    scene.emulated_time = engine
        .published_frame()
        .map_or(0.0, zax_engine3d::frame::ConversionFrame::time);

    // AMD-009 (Q-022): without a profile the scene shows the core's pixel frame.
    scene.two_dimensional_mode = engine.two_dimensional_mode;
    let draw_list = engine.build_draw_list().clone();

    Ok(SyntheticScene {
        name: Path::new(&file_name)
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
        description: format!(
            "{file_name} after {frames} emulated frames, profile '{}'",
            profile.title.as_deref().unwrap_or("2D Mode")
        ),
        scene,
        engine,
        draw_list,
        nes_picture: picture,
    })
}

/// `render.mesh_algo` as the settings catalogue numbers the algorithms.
const fn mesh_setting(mesh: RenderAlgorithm) -> i32 {
    match mesh {
        RenderAlgorithm::Greedy => 0,
        RenderAlgorithm::Marching => 1,
        RenderAlgorithm::GreedyMulticolor => 2,
    }
}

/// Prints what ran, how many scripts went into error and the hook budget of REQ-LUA-110.
fn report_scripts(engine: &ConversionEngine) {
    let Some(host) = engine.script_host_as::<TimedHost>() else {
        return;
    };
    let scripts = host.runtime.scripts();
    if scripts.is_empty() {
        return;
    }

    let names: Vec<&str> = scripts.iter().map(LoadedScript::name).collect();
    println!(
        "scripts: {} in hook order: {}",
        scripts.len(),
        names.join(", ")
    );
    let failed = scripts.iter().filter(|script| script.failed()).count();
    if failed > 0 {
        println!("scripts: {failed} went into error during the run");
    }

    if host.frames() > 0 {
        println!(
            "scripts: hooks {:.3} ms mean, {:.3} ms p99 over {} frames (REQ-LUA-110: 2 ms / 6 ms)",
            host.mean(),
            host.p99(),
            host.frames()
        );
    }
}

/// The machine and the engine, with the bridge that lends the scene and the machine to the Lua
/// runtime the engine owns.
struct Host {
    core: NesCore,
    engine: ConversionEngine,
    bridge: Option<ScriptBridge>,
    zapper_offset: Vec3,
}

impl Host {
    /// Creates the runtime of `profile`, with its settings, camera, backdrop, inputs and memory;
    /// the chunks run here (REQ-LUA-015 step 1), then the runtime becomes the engine's host.
    fn install_runtime(
        &mut self,
        profile: &GameProfile,
        model: SettingsModel,
        scene: &mut SceneState,
    ) -> Result<(), HarnessError> {
        // The scene starts from the settings as they stand.
        push_settings(&model, scene);
        // The C# harness wires no `ViewReset`: a script's change of field of view or of scene
        // ratio leaves the camera as it is.
        let mut settings = ScriptSettings::new(model, ScriptLight::Model);
        settings.set_resets_view(false);
        let environment = ScriptEnvironment {
            settings,
            log: Box::new(ConsoleScriptLog),
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
            core,
            engine,
            zapper_offset,
            ..
        } = self;
        let runtime = with_context(scene, core, zapper_offset, |context| {
            ProfileScriptRuntime::create_in(profile, environment, engine.state_mut(), Some(context))
        })?;
        self.bridge = Some(runtime.bridge());
        self.engine
            .set_script_host(Box::new(TimedHost::new(runtime)));
        Ok(())
    }

    /// REQ-LUA-015 steps 2 and 3: the profile's settings land in the model, then the patterns
    /// are built with their `BuildPattern` hooks.
    fn load_profile(
        &mut self,
        profile: &GameProfile,
        scene: &mut SceneState,
    ) -> Result<(), HarnessError> {
        // The C# harness never applies the light of a profile: the scene light stays at its
        // default until a script writes it, so the profile's light is left out.
        let values: Vec<_> = profile
            .settings
            .as_ref()
            .map(setting_application::to_setting_values)
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| {
                !matches!(
                    id,
                    SettingId::LightDirection | SettingId::LightColor | SettingId::LightIntensity
                )
            })
            .collect();
        self.engine.with_script_host::<TimedHost, _>(|host, _| {
            host.runtime
                .settings_mut()
                .model_mut()
                .apply_profile(values);
        });
        self.sync_settings(scene);

        // The settings were applied through the model, so the engine must not copy them again.
        let mut loaded = Ok(());
        self.scripted(scene, |engine| {
            loaded = engine.load_profile(profile, false);
        });
        loaded?;
        self.sync_settings(scene);
        Ok(())
    }

    /// Runs one emulated frame: the core, then the conversion of its frame and the publication.
    fn run_frame(
        &mut self,
        counter: u32,
        input: u16,
        records: &mut Vec<TileRecord>,
        scene: &mut SceneState,
    ) -> Result<(), HarnessError> {
        self.core.run_frame(input);
        let batch = self.core.frame_batch();
        records.clear();
        records.extend_from_slice(batch.records);
        let batch = FrameBatch {
            records,
            palette: batch.palette,
            mask_x: batch.mask_x,
            mask_y: batch.mask_y,
        };
        let counter = i32::try_from(counter)
            .map_err(|_| HarnessError::Usage("--run: too many frames".to_owned()))?;
        self.scripted(scene, |engine| {
            engine.convert(&batch, counter);
        });
        self.engine.publish();
        self.sync_settings(scene);
        Ok(())
    }

    /// The settings model the runtime holds.
    fn model(&self) -> Option<&SettingsModel> {
        self.engine
            .script_host_as::<TimedHost>()
            .map(|host| host.runtime.settings().model())
    }

    /// Runs `operation` on the engine with the scene and the machine lent to the script API for
    /// its length. The borrows end with the call: the runtime keeps none.
    fn scripted(&mut self, scene: &mut SceneState, operation: impl FnOnce(&mut ConversionEngine)) {
        let Self {
            core,
            engine,
            bridge,
            zapper_offset,
        } = self;
        with_context(scene, core, zapper_offset, |context| match bridge {
            Some(bridge) => bridge.lend(context, || operation(engine)),
            None => operation(engine),
        });
    }

    /// Runs `operation` on the Lua runtime and the conversion state it works on, with the scene
    /// and the machine lent.
    fn with_runtime(
        &mut self,
        scene: &mut SceneState,
        operation: impl FnOnce(&mut ProfileScriptRuntime, &mut zax_engine3d::engine::ConversionState),
    ) {
        self.scripted(scene, |engine| {
            engine.with_script_host::<TimedHost, _>(|host, state| {
                operation(&mut host.runtime, state);
            });
        });
    }

    /// Copies the settings the scripts changed into the engine's slice.
    fn sync_settings(&mut self, scene: &mut SceneState) {
        self.with_runtime(scene, ProfileScriptRuntime::sync_settings);
    }
}

/// What the scripts read of the machine and write of the Zapper aim, borrowed for one call into
/// the engine. The harness publishes no player action masks, as the C# `CoreScriptInput` of the
/// harness is never given any.
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
            if self.core.set_port_type(port, kind).is_err() {
                // Both ports exist, so this cannot happen; the type is simply not changed.
            }
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

/// Runs `operation` with the scene and the machine lent to the script API.
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

/// The script log that goes to the console: `print` as `Lua: ...`, errors on stderr.
struct ConsoleScriptLog;

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
const fn skybox_of(value: i32) -> SkyboxKind {
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

/// Pushes the scene-owned settings a profile or a script set into the scene (SPEC-09
/// REQ-PERS-005, DEC-06-Q5): the field of view, the camera smoothing, the shadows, the skybox,
/// the background and the light.
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
}

/// The direction and the colour of the light, which is all the C# `HarnessLight` writes to the
/// scene; its intensity is kept apart and never reaches the renderer.
fn push_scripted_light(model: &SettingsModel, scene: &mut SceneState) {
    let direction = model.get(SettingId::LightDirection).as_vector3();
    scene.light.direction =
        zax_render::numerics::Vector3::new(direction.x, direction.y, direction.z);
    scene.light.color = color_of(model, SettingId::LightColor);
}
