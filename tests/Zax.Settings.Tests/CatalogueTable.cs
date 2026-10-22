using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// An independent transcription of the tables of SPEC-09 §4.1.1 to §4.1.10, used to check the
/// catalogue the implementation builds (REQ-PERS-001). It is deliberately written out again from
/// the specification rather than derived from <see cref="SettingsCatalogue"/>, so that a mistake
/// in the catalogue shows up as a disagreement between the two.
/// </summary>
/// <param name="Key">Column "Clé".</param>
/// <param name="Lua">Column "Lua", with <c>Setting.</c> left off; null when not exposed.</param>
/// <param name="Kind">Column "Type".</param>
/// <param name="Default">Column "Défaut", as text: the number, <c>true</c>/<c>false</c>, the
/// <c>#RRGGBBAA</c> colour or the comma-separated vector.</param>
/// <param name="Scope">Column "Portée".</param>
/// <param name="Effect">Column "Effet".</param>
public readonly record struct CatalogueRow(
    string Key,
    string? Lua,
    SettingKind Kind,
    string Default,
    SettingScope Scope,
    SettingEffect Effect);

/// <summary>The transcription itself.</summary>
public static class CatalogueTable
{
    private const SettingScope U = SettingScope.User;
    private const SettingScope P = SettingScope.Profile;
    private const SettingScope S = SettingScope.Session;
    private const SettingScope D = SettingScope.Derived;

    private const SettingEffect Imm = SettingEffect.Immediate;
    private const SettingEffect Game = SettingEffect.NextGameLoad;
    private const SettingEffect Rom = SettingEffect.NextRomLoad;
    private const SettingEffect Restart = SettingEffect.Restart;
    private const SettingEffect None = SettingEffect.None;

    private const SettingKind Flag = SettingKind.Bool;
    private const SettingKind Whole = SettingKind.Int;
    private const SettingKind Real = SettingKind.Float;
    private const SettingKind Choice = SettingKind.Enum;
    private const SettingKind Colour = SettingKind.Color;
    private const SettingKind Vector = SettingKind.Vector3;
    private const SettingKind Text = SettingKind.String;
    private const SettingKind Box = SettingKind.Rect;
    private const SettingKind Opaque = SettingKind.Opaque;

    /// <summary>Every row of §4.1, in the order of the specification.</summary>
    public static IReadOnlyList<CatalogueRow> Rows { get; } = Build();

    /// <summary>The rows as xUnit member data.</summary>
    public static TheoryData<CatalogueRow> Theory
    {
        get
        {
            TheoryData<CatalogueRow> data = [];
            foreach (CatalogueRow row in Rows)
            {
                data.Add(row);
            }

            return data;
        }
    }

    private static List<CatalogueRow> Build()
    {
        List<CatalogueRow> rows =
        [
            // §4.1.1 Video and rendering
            new("video.graphic_quality", "GraphicQuality", Choice, "0", P, Imm),
            new("video.vsync", "VSync", Choice, "0", U, Imm),
            new("video.frame_rate", "FrameRate", Whole, "90", U, Imm),
            new("video.skip_frame", "SkipFrame", Flag, "true", U, Imm),
            new("video.render_method", "RenderMethod", Choice, "0", U, Imm),
            new("video.res_scale", "ResScaleIndex", Choice, "0", U, Imm),
            new("video.anti_alias", "AntiAlias", Choice, "0", U, Imm),
            new("render.mesh_algo", "RenderMode", Choice, "0", U, Imm),
            new("render.shadow", "Shadow", Flag, "true", U, Imm),
            new("ui.fullscreen", null, Flag, "false", U, Imm),

            // §4.1.2 Camera
            new("camera.fov", "Fov", Real, "70", U, Imm),
            new("camera.local_cam", "LocalCam", Flag, "false", P, Imm),
            new("camera.smooth_time", "SmoothTime", Real, "0.4", S, Imm),

            // §4.1.3 Light
            new("light.direction", "Light.Direction", Vector, "1,-1,1", P, Imm),
            new("light.color", "Light.Color", Colour, "#FFFFFFFF", P, Imm),
            new("light.intensity", "Light.Intensity", Real, "1", U, Imm),

            // §4.1.4 Background, skybox, television
            new("scene.skybox", "Skybox", Choice, "-1", U, Imm),
            new("scene.custom_bg_color", "CustomBgColor", Colour, "#000000FF", U, Imm),
            new("scene.use_nes_bg_color", "BgColor", Flag, "true", P, Imm),
            new("scene.tv_box", "TVBox", Choice, "0", U, Imm),
            new("scene.map_enable", "MapEnable", Flag, "false", P, None),

            // §4.1.5 Clipping: index 0 left, 1 bottom, 2 right, 3 top (DEC-R2-T1)
            new("clip.left", "Clipping[0]", Real, "0.1", P, Imm),
            new("clip.bottom", "Clipping[1]", Real, "8.1", P, Imm),
            new("clip.right", "Clipping[2]", Real, "8.1", P, Imm),
            new("clip.top", "Clipping[3]", Real, "0.1", P, Imm),

            // §4.1.6 Layers
            new("layer.default_layer", "Layer.DefLayer", Choice, "2", P, Game),
            new("layer.auto", "Layer.Auto", Flag, "false", P, None),
        ];

        // Six layers: offsets 0, 0, 0, 12, 24, 36; alignment FRONT, CENTER then FRONT; the
        // derived coefficient; cast only for layer 2; receive for layers 2 to 5.
        string[] offsets = ["0", "0", "0", "12", "24", "36"];
        string[] alignments = ["0", "1", "0", "0", "0", "0"];
        string[] coefficients = ["1", "0", "1", "1", "1", "1"];
        for (int i = 0; i < 6; i++)
        {
            rows.Add(new CatalogueRow($"layer[{i}].offset", $"Layer[{i}].Offset", Real, offsets[i], P, Imm));
            rows.Add(new CatalogueRow($"layer[{i}].aligned", $"Layer[{i}].Aligned", Choice, alignments[i], P, Imm));
            rows.Add(new CatalogueRow($"layer[{i}].coef", $"Layer[{i}].Coef", Real, coefficients[i], D, None));
            rows.Add(new CatalogueRow(
                $"layer[{i}].cast_shadow",
                $"Layer[{i}].CastShadow",
                Flag,
                i == 2 ? "true" : "false",
                P,
                Imm));
            rows.Add(new CatalogueRow(
                $"layer[{i}].recv_shadow",
                $"Layer[{i}].RecvShadow",
                Flag,
                i >= 2 ? "true" : "false",
                P,
                Imm));
        }

        rows.AddRange(
        [
            // §4.1.7 Emulation
            new CatalogueRow("emu.game_speed", "GameSpeed", Real, "0", U, Imm),
            new CatalogueRow("emu.tv_mode", "TVMode", Choice, "0", P, Imm),
            new CatalogueRow("emu.auto_tv_mode", "AutoTVMode", Flag, "false", U, Rom),
            new CatalogueRow("emu.core", "Core", Whole, "0", U, Imm),

            // §4.1.8 Conversion, all profile-scoped
            new CatalogueRow("conv.same_bg_palette", "SameBgPalette", Flag, "false", P, Imm),
            new CatalogueRow("conv.same_sp_palette", "SameSpPalette", Flag, "false", P, Imm),
            new CatalogueRow("conv.sprite_tolerance", "SpriteTolerance", Whole, "0", P, Imm),
            new CatalogueRow("conv.scan_direction", "ScanDirection", Choice, "0", P, Imm),
            new CatalogueRow("conv.empty_tile", "EmptyTile", Flag, "false", P, Imm),

            // §4.1.9 Audio, inputs, interface
            new CatalogueRow("audio.volume", "Volume", Real, "1", U, Imm),
            new CatalogueRow("audio.bg_music", "BGMusic", Flag, "true", U, Imm),
            new CatalogueRow("input.port_type[0]", null, Choice, "0", S, Imm),
            new CatalogueRow("input.port_type[1]", null, Choice, "0", S, Imm),
            new CatalogueRow("input.zapper_enable", "ZapperEnable", Flag, "false", D, None),
            new CatalogueRow("input.emulated_menu_button", "EmulatedMenuButton", Flag, "false", U, None),
            new CatalogueRow("ui.sort_option", "SortOption", Choice, "0", U, Imm),
            new CatalogueRow("ui.last_browse_dir", null, Text, string.Empty, U, Imm),
            new CatalogueRow("ui.window_rect", null, Box, "-1,-1,1280,720", U, Restart),

            // §4.1.10 VR, editor, movie, internal and derived
            new CatalogueRow("vr.runtime", "VrRuntime", Whole, "0", U, None),
            new CatalogueRow("vr.scene_ratio", "SceneRatio", Real, "1", U, None),
            new CatalogueRow("vr.mirroring", "Mirroring", Flag, "true", U, None),
            new CatalogueRow("vr.control_method", "ControlMethod", Whole, "0", U, None),
            new CatalogueRow("editor.selection_mode", "SelectionMode", Choice, "0", S, None),
            new CatalogueRow("editor.saved_patterns", "SavedPattern3D", Opaque, string.Empty, P, None),
            new CatalogueRow("editor.saved_anim_patterns", "SaveAnimPattern3D", Opaque, string.Empty, P, None),
            new CatalogueRow("movie.mode", "MovieMode", Choice, "0", S, None),
            new CatalogueRow("scene.root_scale", "RootScale", Real, "1", S, Imm),
            new CatalogueRow("scene.scene_scale", "SceneScale", Real, "1", D, Imm),
            new CatalogueRow("scene.skybox_enable", "SkyboxEnable", Flag, "false", D, None),
            new CatalogueRow("video.res_scale_factor", "ResScale", Real, "1", D, None),
            new CatalogueRow("video.target_framerate", "TargetFramerate", Real, "90", D, None),
            new CatalogueRow("video.target_time_per_frame", "TargetTimePerFrame", Real, "0.011111", D, None),
            new CatalogueRow("emu.tv_data", "TVData", Opaque, string.Empty, D, None),
            new CatalogueRow("app.engine_mode", "EngineMode", Choice, "1", S, None),
        ]);

        return rows;
    }
}
