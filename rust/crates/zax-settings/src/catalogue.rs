//! The settings catalogue of SPEC-09 §4.1.1 to §4.1.10 (REQ-PERS-001): one identifier and one
//! descriptor per setting, enumerable by program (REQ-ARCH-021). The table itself is in `table`.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::meta::{SettingDescriptor, SettingScope, SettingTheme};
use crate::table;

/// One of the six depth layers of §4.1.6 (`layer[i]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer(u8);

impl Layer {
    /// The layers, in index order.
    pub const ALL: [Layer; 6] = [Layer(0), Layer(1), Layer(2), Layer(3), Layer(4), Layer(5)];

    /// The layer of `index`, or none when the index is outside 0 to 5.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// The index, 0 to 5.
    #[must_use]
    pub fn index(self) -> u8 {
        self.0
    }
}

/// One of the two input ports of SPEC-08 REQ-INP-060 (`input.port_type[i]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port(u8);

impl Port {
    /// The ports, in index order.
    pub const ALL: [Port; 2] = [Port(0), Port(1)];

    /// The port of `index`, or none when the index is neither 0 nor 1.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// The index, 0 or 1.
    #[must_use]
    pub fn index(self) -> u8 {
        self.0
    }
}

/// Declares [`SettingId`]: one variant per fixed key, plus the families of indexed keys. Each
/// fixed variant names its stable key once, here.
macro_rules! setting_ids {
    ($($variant:ident => $key:literal,)*) => {
        /// Identifier of a setting. Its [`key`](Self::key) is the stable key of REQ-PERS-080.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum SettingId {
            $($variant,)*
            LayerOffset(Layer),
            LayerAligned(Layer),
            LayerCoef(Layer),
            LayerCastShadow(Layer),
            LayerRecvShadow(Layer),
            InputPortType(Port),
        }

        impl SettingId {
            /// The stable key, for instance `camera.fov` (REQ-PERS-080).
            #[must_use]
            pub fn key(self) -> Cow<'static, str> {
                match self {
                    $(Self::$variant => Cow::Borrowed($key),)*
                    Self::LayerOffset(layer) => Cow::Owned(format!("layer[{}].offset", layer.index())),
                    Self::LayerAligned(layer) => Cow::Owned(format!("layer[{}].aligned", layer.index())),
                    Self::LayerCoef(layer) => Cow::Owned(format!("layer[{}].coef", layer.index())),
                    Self::LayerCastShadow(layer) => Cow::Owned(format!("layer[{}].cast_shadow", layer.index())),
                    Self::LayerRecvShadow(layer) => Cow::Owned(format!("layer[{}].recv_shadow", layer.index())),
                    Self::InputPortType(port) => Cow::Owned(format!("input.port_type[{}]", port.index())),
                }
            }
        }
    };
}

setting_ids! {
    VideoGraphicQuality => "video.graphic_quality",
    VideoVSync => "video.vsync",
    VideoFrameRate => "video.frame_rate",
    VideoSkipFrame => "video.skip_frame",
    VideoRenderMethod => "video.render_method",
    VideoResScale => "video.res_scale",
    VideoAntiAlias => "video.anti_alias",
    // AMD-052 REQ-LOOP-077; its Lua path is `MaxQueuedFrames` (AMD-059 REQ-LUA-116).
    VideoMaxQueuedFrames => "video.max_queued_frames",
    // AMD-059 REQ-LUA-116: kept and handed back, the anti-aliasing staying `video.anti_alias`.
    VideoMsaa20 => "video.msaa_2_0",
    RenderMeshAlgo => "render.mesh_algo",
    RenderShadow => "render.shadow",
    // AMD-040 §3, amended by AMD-055 REQ-SCN-078: 0 Classic, 4 Enhanced (1 written reads 4).
    RenderVisualStyle => "render.visual_style",
    UiFullscreen => "ui.fullscreen",
    CameraFov => "camera.fov",
    CameraLocalCam => "camera.local_cam",
    CameraSmoothTime => "camera.smooth_time",
    LightDirection => "light.direction",
    LightColor => "light.color",
    LightIntensity => "light.intensity",
    SceneSkybox => "scene.skybox",
    SceneCustomBgColor => "scene.custom_bg_color",
    SceneUseNesBgColor => "scene.use_nes_bg_color",
    SceneMapEnable => "scene.map_enable",
    // The four curvature settings of AMD-043 REQ-SCN-066, which the style parameters
    // `spl.bend_zx`, `spl.bend_zy`, `spl.arc_yx` and `spl.arc_center` stand for (REQ-LUA-114).
    SceneBendLr => "scene.bend_lr",
    SceneBendTb => "scene.bend_tb",
    SceneArcH => "scene.arc_h",
    SceneArcCenter => "scene.arc_center",
    // AMD-046 REQ-SCN-074, a setting of Zaxnes's own (divergence D): no Lua access, the
    // original program has none.
    SceneBend2dMode => "scene.bend_2d_mode",
    // AMD-049 REQ-SCN-075, also a setting of Zaxnes's own (divergence D): the original always
    // lights the backdrop with the almost-matt model, so it has no such setting and no Lua
    // access to it. Zaxnes keeps the diffuse term of REQ-SCN-048 reachable because the model is
    // a fragment cost on what can be the largest surface of the scene, and the priority clause
    // T-1 of AMD-049 keeps that term valid.
    RenderBackdropLighting => "render.backdrop_lighting",
    ClipLeft => "clip.left",
    ClipBottom => "clip.bottom",
    ClipRight => "clip.right",
    ClipTop => "clip.top",
    LayerDefaultLayer => "layer.default_layer",
    LayerAuto => "layer.auto",
    EmuGameSpeed => "emu.game_speed",
    EmuTvMode => "emu.tv_mode",
    EmuAutoTvMode => "emu.auto_tv_mode",
    EmuCore => "emu.core",
    // Lot FCEUX-23 step 5 (design note §4): which emulation engine plays a game, as a per-game
    // option. The identifier is in the catalogue in **every** build so that a `games.json`
    // carrying it loads and saves without error; a build without the `fceux-core` feature has
    // only one engine, does not show the row and ignores the value.
    EmuEngine => "emu.engine",
    // Lot CARTDB (Rust-only): the cartridge database that corrects iNES 1.0 headers at load, and
    // what the work RAM holds at power-on.
    EmuCartridgeDb => "emu.cartridge_db",
    EmuPowerOnRam => "emu.power_on_ram",
    EmuOamCorruption => "emu.oam_corruption",
    EmuOamaddrBug => "emu.oamaddr_bug",
    EmuOamDecay => "emu.oam_decay",
    EmuPowerOnTop => "emu.power_on_top",
    EmuPpu2006Delay => "emu.ppu_2006_delay",
    EmuPpu2006Glitch => "emu.ppu_2006_glitch",
    EmuPpu2000Glitch => "emu.ppu_2000_glitch",
    EmuPaletteCorruption => "emu.palette_corruption",
    EmuCpuPpuAlignment => "emu.cpu_ppu_alignment",
    EmuPpuBusConflicts => "emu.ppu_bus_conflicts",
    EmuCatchUp => "emu.catch_up",
    // AMD-041 REQ-LOOP-064, exposed as `RunAheadFrames` (AMD-040 REQ-LUA-114).
    EmuRunAhead => "emu.run_ahead",
    EmuLuaSmoothGc => "emu.lua_smooth_gc",
    EmuLuaOptimized => "emu.lua_optimized",
    EmuLuaExperimental => "emu.lua_experimental",
    EmuPipeline => "emu.pipeline",
    EmuPause3dIn2d => "emu.pause_3d_in_2d",
    EmuCatchUpLast3d => "emu.catch_up_last_3d",
    VideoHalfRate3d => "video.half_rate_3d",
    VideoDynamicResolution => "video.dynamic_resolution",
    VideoShaderResolution => "video.shader_resolution",
    VideoBackend => "video.backend",
    // Rust-only (owner request): the RetroArch slang preset of each of the four display
    // modes, with its fallback, the lines of the picture it reads and its parameters.
    Shader2dMode => "shader.2d.mode",
    Shader2dPreset => "shader.2d.preset",
    Shader2dLines => "shader.2d.lines",
    Shader2dParams => "shader.2d.params",
    Shader2dArea => "shader.2d.area",
    Shader2dFrame => "shader.2d.frame",
    Shader2dWsMode => "shader.2d_ws.mode",
    Shader2dWsPreset => "shader.2d_ws.preset",
    Shader2dWsLines => "shader.2d_ws.lines",
    Shader2dWsParams => "shader.2d_ws.params",
    Shader2dWsArea => "shader.2d_ws.area",
    Shader2dWsFrame => "shader.2d_ws.frame",
    Shader3dMode => "shader.3d.mode",
    Shader3dPreset => "shader.3d.preset",
    Shader3dLines => "shader.3d.lines",
    Shader3dParams => "shader.3d.params",
    Shader3dArea => "shader.3d.area",
    Shader3dFrame => "shader.3d.frame",
    Shader3dWsMode => "shader.3d_ws.mode",
    Shader3dWsPreset => "shader.3d_ws.preset",
    Shader3dWsLines => "shader.3d_ws.lines",
    Shader3dWsParams => "shader.3d_ws.params",
    Shader3dWsArea => "shader.3d_ws.area",
    Shader3dWsFrame => "shader.3d_ws.frame",
    VideoWidescreen => "video.widescreen",
    VideoWidescreenExtra => "video.widescreen_extra",
    VideoWidescreenStrictEdges => "video.widescreen_strict_edges",
    VideoWidescreenAdvanced => "video.widescreen_advanced",
    EmuSmoothScrollAhead => "emu.smooth_scroll_ahead",
    EmuSmoothScrollView => "emu.smooth_scroll_view",
    VideoRemoveSpriteLimit => "video.remove_sprite_limit",
    VideoOverscanNtscTop => "video.overscan_ntsc_top",
    VideoOverscanNtscBottom => "video.overscan_ntsc_bottom",
    VideoOverscanNtscLeft => "video.overscan_ntsc_left",
    VideoOverscanNtscRight => "video.overscan_ntsc_right",
    VideoOverscanPalTop => "video.overscan_pal_top",
    VideoOverscanPalBottom => "video.overscan_pal_bottom",
    VideoOverscanPalLeft => "video.overscan_pal_left",
    VideoOverscanPalRight => "video.overscan_pal_right",
    VideoOverscanTop => "video.overscan_top",
    VideoOverscanBottom => "video.overscan_bottom",
    VideoOverscanLeft => "video.overscan_left",
    VideoOverscanRight => "video.overscan_right",
    VideoOverscanMode => "video.overscan_mode",
    VideoNesPalette => "video.nes_palette",
    EmuBuiltinPatches => "emu.builtin_patches",
    EmuForcedPatch => "emu.forced_patch",
    EmuHackOptions => "emu.hack_options",
    ConvSameBgPalette => "conv.same_bg_palette",
    ConvSameSpPalette => "conv.same_sp_palette",
    ConvSpriteTolerance => "conv.sprite_tolerance",
    ConvScanDirection => "conv.scan_direction",
    ConvEmptyTile => "conv.empty_tile",
    AudioVolume => "audio.volume",
    AudioBgMusic => "audio.bg_music",
    AudioLatencyMs => "audio.latency_ms",
    AudioClassicOutput => "audio.classic_output",
    AudioConsoleProfile => "audio.console_profile",
    InputZapperEnable => "input.zapper_enable",
    InputEmulatedMenuButton => "input.emulated_menu_button",
    InputTouchPad => "input.touch_pad",
    InputTouchScale => "input.touch_scale",
    InputTouchHapticDpad => "input.touch_haptic_dpad",
    InputTouchHapticButtons => "input.touch_haptic_buttons",
    InputTouchExtra => "input.touch_extra",
    UiOrientation => "ui.orientation",
    UiSortOption => "ui.sort_option",
    UiLastBrowseDir => "ui.last_browse_dir",
    UiWindowRect => "ui.window_rect",
    // AMD-059 REQ-LUA-116: kept exactly as written; Zaxnes has one library layout.
    UiLibraryView => "ui.library_view",
    VrRuntime => "vr.runtime",
    // AMD-059 REQ-LUA-116: the refresh rate of a headset, without effect here.
    VrRefreshRate => "vr.refresh_rate",
    VrSceneRatio => "vr.scene_ratio",
    VrMirroring => "vr.mirroring",
    VrControlMethod => "vr.control_method",
    EditorSelectionMode => "editor.selection_mode",
    EditorSavedPatterns => "editor.saved_patterns",
    EditorSavedAnimPatterns => "editor.saved_anim_patterns",
    MovieMode => "movie.mode",
    SceneRootScale => "scene.root_scale",
    SceneSceneScale => "scene.scene_scale",
    SceneSkyboxEnable => "scene.skybox_enable",
    VideoResScaleFactor => "video.res_scale_factor",
    VideoTargetFramerate => "video.target_framerate",
    VideoTargetTimePerFrame => "video.target_time_per_frame",
    EmuTvData => "emu.tv_data",
    AppEngineMode => "app.engine_mode",
    AudioMute => "audio.mute",
    UiShowFps => "ui.show_fps",
    UiProfiler => "ui.profiler",
    UiEditorMode => "ui.editor_mode",
    UiLanguage => "ui.language",
    UiRomsFolder => "ui.roms_folder",
    // The twelve parameters of the Enhanced style (AMD-055 REQ-SCN-079, keys of its §4). They
    // are persisted and reachable from Lua here; the rendering is lot RU-5.
    RenderStyleAo => "render.style.ao",
    RenderStyleAoStrength => "render.style.ao_strength",
    RenderStyleAoRadius => "render.style.ao_radius",
    RenderStyleAoThickness => "render.style.ao_thickness",
    RenderStyleAoQuality => "render.style.ao_quality",
    RenderStylePixelShadow => "render.style.pixel_shadow",
    RenderStyleVoxelGrid => "render.style.voxel_grid",
    RenderStyleReflections => "render.style.reflections",
    RenderStyleSsrShine => "render.style.ssr_shine",
    RenderStyleSsrClarity => "render.style.ssr_clarity",
    RenderStyleSsrLength => "render.style.ssr_length",
    RenderStyleSsrQuality => "render.style.ssr_quality",
    AchievementsEnabled => "achievements.enabled",
    AchievementsEncore => "achievements.encore",
    AchievementsUnofficial => "achievements.unofficial",
    AchievementsSound => "achievements.sound",
    AchievementsChallenges => "achievements.challenges",
    AchievementsUsername => "achievements.username",
    AchievementsToken => "achievements.token",
}

/// Whether `key` is a stable key of the grammar of REQ-PERS-080: dot-separated segments of `a-z`,
/// `0-9`, `_`, `[` and `]`. The brackets are accepted in every segment, as the keys of §4.1.6 need
/// (Q-034).
#[must_use]
pub fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '[' | ']'))
        })
}

/// Keys a former version persisted and this one no longer knows (AMD-031 §3). They are read
/// without error and dropped on the next write, rather than kept as unknown keys.
const RETIRED_KEYS: [&str; 2] = ["scene.tv_box", "video.hide_overscan"];

/// The catalogue, built once: the table in its order, with the lookups by key and Lua path.
struct Catalogue {
    order: Vec<SettingId>,
    by_id: HashMap<SettingId, SettingDescriptor>,
    by_key: HashMap<String, SettingId>,
    by_lua_path: HashMap<String, SettingId>,
}

static CATALOGUE: LazyLock<Catalogue> = LazyLock::new(|| {
    let entries = table::build();
    let order = entries.iter().map(SettingDescriptor::id).collect();
    let by_key = entries
        .iter()
        .map(|descriptor| (descriptor.key().into_owned(), descriptor.id()))
        .collect();
    let by_lua_path = entries
        .iter()
        .flat_map(|descriptor| {
            descriptor
                .lua_path()
                .into_iter()
                .chain(descriptor.lua_aliases().iter().copied())
                .map(move |path| (path.to_owned(), descriptor.id()))
        })
        .collect();
    let by_id = entries
        .into_iter()
        .map(|descriptor| (descriptor.id(), descriptor))
        .collect();
    Catalogue {
        order,
        by_id,
        by_key,
        by_lua_path,
    }
});

impl SettingId {
    /// The setting of `key`, or none when no setting carries that key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        CATALOGUE.by_key.get(key).copied()
    }

    /// The setting the Lua path `path` designates, aliases included (REQ-PERS-008). The path is
    /// written as the column "Lua" writes it, with `Setting.` left off.
    #[must_use]
    pub fn from_lua_path(path: &str) -> Option<Self> {
        CATALOGUE.by_lua_path.get(path).copied()
    }
}

/// The descriptor of `id`.
#[must_use]
pub fn descriptor(id: SettingId) -> &'static SettingDescriptor {
    // Every identifier has a descriptor by construction of the table, which the catalogue tests
    // check against the specification.
    &CATALOGUE.by_id[&id]
}

/// Every setting of §4.1, in the order of the specification's tables.
pub fn descriptors() -> impl Iterator<Item = &'static SettingDescriptor> {
    CATALOGUE.order.iter().map(|id| descriptor(*id))
}

/// The settings of `scope`.
pub fn of_scope(scope: SettingScope) -> impl Iterator<Item = &'static SettingDescriptor> {
    descriptors().filter(move |descriptor| descriptor.scope() == scope)
}

/// The settings of `theme`, for the options window.
pub fn of_theme(theme: SettingTheme) -> impl Iterator<Item = &'static SettingDescriptor> {
    descriptors().filter(move |descriptor| descriptor.theme() == theme)
}

/// Whether `key` is one of the retired keys of AMD-031 §3.
#[must_use]
pub fn is_retired_key(key: &str) -> bool {
    RETIRED_KEYS.contains(&key)
}

/// Whether layer `layer` casts shadows after a profile load: layer 2 alone (REQ-PERS-005 point 4,
/// REQ-PROF-042). The first-launch state of the original is not reproduced (DEC-04B-Q5).
#[must_use]
pub fn default_cast_shadow(layer: Layer) -> bool {
    layer.index() == 2
}

/// Whether layer `layer` receives shadows after a profile load: layers 2 to 5.
#[must_use]
pub fn default_recv_shadow(layer: Layer) -> bool {
    layer.index() >= 2
}

/// The derived alignment coefficient: 1 for FRONT, 0 for CENTER, -1 for BACK (REQ-PERS-013
/// point 8).
#[must_use]
pub fn alignment_coefficient(alignment: i32) -> f32 {
    match alignment {
        0 => 1.0,
        2 => -1.0,
        _ => 0.0,
    }
}
