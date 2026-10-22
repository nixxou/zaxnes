namespace Zax.Settings;

/// <summary>
/// The stable keys of SPEC-09 §4.1 (REQ-PERS-080). A published key is never renamed and never
/// reused for another meaning; a withdrawn key stays reserved.
/// </summary>
public static class SettingKeys
{
    /// <summary>Number of depth layers (SPEC-09 §4.1.6).</summary>
    public const int LayerCount = 6;

    /// <summary>Number of clipping margins (SPEC-09 §4.1.5).</summary>
    public const int ClippingCount = 4;

    /// <summary>Number of input ports (SPEC-08 REQ-INP-060).</summary>
    public const int PortCount = 2;

    // §4.1.1 Video and rendering

    /// <summary>Graphic quality; stored, without visible effect.</summary>
    public const string VideoGraphicQuality = "video.graphic_quality";

    /// <summary>Vertical synchronisation.</summary>
    public const string VideoVSync = "video.vsync";

    /// <summary>Render rate cap when vsync is off.</summary>
    public const string VideoFrameRate = "video.frame_rate";

    /// <summary>Skip the render when no new frame was emulated.</summary>
    public const string VideoSkipFrame = "video.skip_frame";

    /// <summary>Direct or offscreen rendering.</summary>
    public const string VideoRenderMethod = "video.render_method";

    /// <summary>Resolution scale index of the offscreen render.</summary>
    public const string VideoResScale = "video.res_scale";

    /// <summary>Full-screen anti-aliasing.</summary>
    public const string VideoAntiAlias = "video.anti_alias";

    /// <summary>Default mesh algorithm.</summary>
    public const string RenderMeshAlgo = "render.mesh_algo";

    /// <summary>Global shadow switch.</summary>
    public const string RenderShadow = "render.shadow";

    /// <summary>Full-screen state.</summary>
    public const string UiFullscreen = "ui.fullscreen";

    // §4.1.2 Camera

    /// <summary>Vertical field of view in degrees.</summary>
    public const string CameraFov = "camera.fov";

    /// <summary>Top-down camera mode.</summary>
    public const string CameraLocalCam = "camera.local_cam";

    /// <summary>Camera smoothing time in seconds.</summary>
    public const string CameraSmoothTime = "camera.smooth_time";

    // §4.1.3 Light

    /// <summary>Direction of the directional light.</summary>
    public const string LightDirection = "light.direction";

    /// <summary>Colour of the directional light.</summary>
    public const string LightColor = "light.color";

    /// <summary>Intensity of the directional light; a user preference.</summary>
    public const string LightIntensity = "light.intensity";

    // §4.1.4 Background, skybox, television

    /// <summary>Skybox choice.</summary>
    public const string SceneSkybox = "scene.skybox";

    /// <summary>Custom background colour.</summary>
    public const string SceneCustomBgColor = "scene.custom_bg_color";

    /// <summary>Use the NES universal background colour.</summary>
    public const string SceneUseNesBgColor = "scene.use_nes_bg_color";

    /// <summary>Decorative television model.</summary>
    public const string SceneTvBox = "scene.tv_box";

    /// <summary>Kept for data compatibility; no observable effect.</summary>
    public const string SceneMapEnable = "scene.map_enable";

    // §4.1.5 Clipping

    /// <summary>Left margin, Lua index 0.</summary>
    public const string ClipLeft = "clip.left";

    /// <summary>Bottom margin, Lua index 1 (DEC-R2-T1).</summary>
    public const string ClipBottom = "clip.bottom";

    /// <summary>Right margin, Lua index 2.</summary>
    public const string ClipRight = "clip.right";

    /// <summary>Top margin, Lua index 3 (DEC-R2-T1).</summary>
    public const string ClipTop = "clip.top";

    /// <summary>The four margins in Lua index order: left, bottom, right, top.</summary>
    public static IReadOnlyList<string> Clipping { get; } = [ClipLeft, ClipBottom, ClipRight, ClipTop];

    // §4.1.6 Layers

    /// <summary>Layer given to patterns that name none.</summary>
    public const string LayerDefaultLayer = "layer.default_layer";

    /// <summary>Kept for data compatibility; no effect.</summary>
    public const string LayerAuto = "layer.auto";

    /// <summary>Depth offset of layer <paramref name="index"/>.</summary>
    /// <param name="index">Layer index, 0 to 5.</param>
    public static string LayerOffset(int index) => $"layer[{Check(index)}].offset";

    /// <summary>Depth alignment of layer <paramref name="index"/>.</summary>
    /// <param name="index">Layer index, 0 to 5.</param>
    public static string LayerAligned(int index) => $"layer[{Check(index)}].aligned";

    /// <summary>Derived alignment coefficient of layer <paramref name="index"/>.</summary>
    /// <param name="index">Layer index, 0 to 5.</param>
    public static string LayerCoef(int index) => $"layer[{Check(index)}].coef";

    /// <summary>Whether layer <paramref name="index"/> casts shadows.</summary>
    /// <param name="index">Layer index, 0 to 5.</param>
    public static string LayerCastShadow(int index) => $"layer[{Check(index)}].cast_shadow";

    /// <summary>Whether layer <paramref name="index"/> receives shadows.</summary>
    /// <param name="index">Layer index, 0 to 5.</param>
    public static string LayerRecvShadow(int index) => $"layer[{Check(index)}].recv_shadow";

    // §4.1.7 Emulation

    /// <summary>Emulation speed index.</summary>
    public const string EmuGameSpeed = "emu.game_speed";

    /// <summary>Region of the core.</summary>
    public const string EmuTvMode = "emu.tv_mode";

    /// <summary>Take the region from the ROM header.</summary>
    public const string EmuAutoTvMode = "emu.auto_tv_mode";

    /// <summary>Core index.</summary>
    public const string EmuCore = "emu.core";

    /// <summary>Derived region timing block.</summary>
    public const string EmuTvData = "emu.tv_data";

    // §4.1.8 Conversion

    /// <summary>Background recognition ignores the palette.</summary>
    public const string ConvSameBgPalette = "conv.same_bg_palette";

    /// <summary>Sprite recognition ignores the palette.</summary>
    public const string ConvSameSpPalette = "conv.same_sp_palette";

    /// <summary>Sprite grouping tolerance.</summary>
    public const string ConvSpriteTolerance = "conv.sprite_tolerance";

    /// <summary>Background scan order.</summary>
    public const string ConvScanDirection = "conv.scan_direction";

    /// <summary>Include empty background tiles.</summary>
    public const string ConvEmptyTile = "conv.empty_tile";

    // §4.1.9 Audio, inputs, interface

    /// <summary>Master volume.</summary>
    public const string AudioVolume = "audio.volume";

    /// <summary>Menu music.</summary>
    public const string AudioBgMusic = "audio.bg_music";

    /// <summary>Type of input port <paramref name="port"/>.</summary>
    /// <param name="port">Port index, 0 or 1.</param>
    public static string InputPortType(int port) => port is 0 or 1
        ? $"input.port_type[{port}]"
        : throw new ArgumentOutOfRangeException(nameof(port));

    /// <summary>Derived: at least one port holds a Zapper.</summary>
    public const string InputZapperEnable = "input.zapper_enable";

    /// <summary>Kept and exposed; no effect (DEC-R2-D7).</summary>
    public const string InputEmulatedMenuButton = "input.emulated_menu_button";

    /// <summary>Library and slot sort order.</summary>
    public const string UiSortOption = "ui.sort_option";

    /// <summary>Last folder the file browser opened.</summary>
    public const string UiLastBrowseDir = "ui.last_browse_dir";

    /// <summary>Window position and size in windowed mode.</summary>
    public const string UiWindowRect = "ui.window_rect";

    // §4.1.10 VR, editor, movie, internal and derived

    /// <summary>VR runtime; always 0 on desktop.</summary>
    public const string VrRuntime = "vr.runtime";

    /// <summary>VR scene ratio; reads 1 on desktop.</summary>
    public const string VrSceneRatio = "vr.scene_ratio";

    /// <summary>Stored, without effect.</summary>
    public const string VrMirroring = "vr.mirroring";

    /// <summary>Stored, without effect.</summary>
    public const string VrControlMethod = "vr.control_method";

    /// <summary>Editor selection mode; no effect.</summary>
    public const string EditorSelectionMode = "editor.selection_mode";

    /// <summary>Editor pattern presets; kept.</summary>
    public const string EditorSavedPatterns = "editor.saved_patterns";

    /// <summary>Editor animation presets; kept.</summary>
    public const string EditorSavedAnimPatterns = "editor.saved_anim_patterns";

    /// <summary>Movie mode; frozen at 0.</summary>
    public const string MovieMode = "movie.mode";

    /// <summary>Scene root scale.</summary>
    public const string SceneRootScale = "scene.root_scale";

    /// <summary>Derived scene scale.</summary>
    public const string SceneSceneScale = "scene.scene_scale";

    /// <summary>Derived skybox switch.</summary>
    public const string SceneSkyboxEnable = "scene.skybox_enable";

    /// <summary>Derived resolution scale factor.</summary>
    public const string VideoResScaleFactor = "video.res_scale_factor";

    /// <summary>Derived target frame rate.</summary>
    public const string VideoTargetFramerate = "video.target_framerate";

    /// <summary>Derived target time per frame.</summary>
    public const string VideoTargetTimePerFrame = "video.target_time_per_frame";

    /// <summary>Engine mode; always PROFILE.</summary>
    public const string AppEngineMode = "app.engine_mode";

    // Settings this reimplementation adds, with keys of their own (REQ-PERS-001 note).

    /// <summary>
    /// The mute switch of the Audio menu (AMD-010 REQ-UI-200). SPEC-11 REQ-AUD-006 has no
    /// separate mute setting, so this one forces the effective gain to zero and leaves
    /// <see cref="AudioVolume"/> alone.
    /// </summary>
    public const string AudioMute = "audio.mute";

    /// <summary>The frame-rate counter of DEC-10-Q7.</summary>
    public const string UiShowFps = "ui.show_fps";

    private static int Check(int index) => (uint)index < LayerCount
        ? index
        : throw new ArgumentOutOfRangeException(nameof(index), index, "layer index is 0 to 5");
}
