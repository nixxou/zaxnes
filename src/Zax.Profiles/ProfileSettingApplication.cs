using Zax.Settings;

namespace Zax.Profiles;

/// <summary>
/// Which of a profile's embedded settings replace the user's settings when the profile loads,
/// and with what value (SPEC-03 REQ-PROF-041 and REQ-PROF-042, SPEC-09 REQ-PERS-005).
/// </summary>
/// <remarks>
/// <para>
/// The keys are the stable setting keys of SPEC-09 §4.1, so that the settings model of a later
/// lot can apply this table without knowing anything about the profile format. REQ-PROF-041 is
/// authoritative on the split: everything it does not list keeps the user's value, the light
/// <b>intensity</b> included, although its direction and colour are replaced.
/// </para>
/// <para>
/// Two values do not come from the file: <c>scene.use_nes_bg_color</c> is forced to true, and
/// the six pairs of layer shadow flags are reset to "layer 2 casts, layers 2 to 5 receive"
/// whatever the file stores.
/// </para>
/// </remarks>
public static class ProfileSettingApplication
{
    /// <summary>Setting key of the light direction.</summary>
    public const string LightDirectionKey = "light.direction";

    /// <summary>Setting key of the light colour.</summary>
    public const string LightColorKey = "light.color";

    /// <summary>Setting key forced to true by a profile load.</summary>
    public const string UseNesBackgroundColorKey = "scene.use_nes_bg_color";

    private static readonly string[] ClippingKeys = ["clip.left", "clip.bottom", "clip.right", "clip.top"];

    /// <summary>
    /// The keys a profile replaces, in a stable order. Forty-four entries: the two light
    /// vectors, the four clipping margins, the map flag, the two layer-set keys, the four keys
    /// of each of the six layers, the graphic quality, the two editor preset lists, the five
    /// conversion settings, the top-down camera, the television region, and the forced
    /// background flag.
    /// </summary>
    public static IReadOnlyList<string> AppliedKeys { get; } = BuildAppliedKeys();

    /// <summary>
    /// The keys a profile leaves alone, so the user's preference survives a profile load
    /// (REQ-PROF-041, second list).
    /// </summary>
    public static IReadOnlyList<string> PreservedKeys { get; } =
    [
        "light.intensity",
        "render.mesh_algo",
        "vr.runtime",
        "scene.skybox",
        "emu.core",
        "emu.game_speed",
        "render.shadow",
        "audio.volume",
        "video.frame_rate",
        "video.vsync",
        "vr.control_method",
        "vr.scene_ratio",
        "input.emulated_menu_button",
        "video.render_method",
        "video.res_scale",
        "video.skip_frame",
        "audio.bg_music",
        "video.anti_alias",
        "vr.mirroring",
        "emu.auto_tv_mode",
        "scene.tv_box",
        "ui.sort_option",
        "scene.custom_bg_color",
        "camera.fov",
    ];

    /// <summary>
    /// The values to apply, keyed by setting key. A key whose field is absent from the profile
    /// takes the default of REQ-PROF-040, not the user's value, which is already the case here
    /// because <see cref="ProfileSettings"/> starts at those defaults.
    /// </summary>
    public static IReadOnlyDictionary<string, object?> ValuesToApply(ProfileSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        Dictionary<string, object?> values = new(AppliedKeys.Count, StringComparer.Ordinal)
        {
            [LightDirectionKey] = settings.Light?.Direction,
            [LightColorKey] = settings.Light?.Color,
            ["scene.map_enable"] = settings.MapEnable,
            ["video.graphic_quality"] = settings.GraphicQuality,
            ["editor.saved_patterns"] = settings.SavedPatterns,
            ["editor.saved_anim_patterns"] = settings.SavedAnimPatterns,
            ["conv.same_bg_palette"] = settings.SameBackgroundPalette,
            ["conv.same_sp_palette"] = settings.SameSpritePalette,
            ["conv.sprite_tolerance"] = settings.SpriteTolerance,
            ["conv.scan_direction"] = settings.ScanDirection,
            ["conv.empty_tile"] = settings.EmptyTile,
            ["camera.local_cam"] = settings.LocalCamera,
            ["emu.tv_mode"] = settings.TvMode,

            // Forced, whatever the profile says (REQ-PROF-041 point 3).
            [UseNesBackgroundColorKey] = true,
        };

        for (int i = 0; i < ClippingKeys.Length; i++)
        {
            values[ClippingKeys[i]] = settings.Clipping?[i];
        }

        values["layer.default_layer"] = settings.Layers?.DefaultLayer;
        values["layer.auto"] = settings.Layers?.Auto;
        for (int i = 0; i < LayerSettingsSet.LayerCount; i++)
        {
            LayerSettings? layer = settings.Layers?[i];
            values[$"layer[{i}].offset"] = layer?.Offset;
            values[$"layer[{i}].aligned"] = layer?.Aligned;

            // Reset, whatever the profile stores (REQ-PROF-042).
            values[$"layer[{i}].cast_shadow"] = LayerSettingsSet.EffectiveCastShadow(i);
            values[$"layer[{i}].recv_shadow"] = LayerSettingsSet.EffectiveRecvShadow(i);
        }

        return values;
    }

    /// <summary>
    /// The same values typed for the settings model, ready for
    /// <see cref="SettingsModel.ApplyProfile"/> (SPEC-09 REQ-PERS-005).
    /// </summary>
    /// <remarks>
    /// Only the settings the model carries as values are converted. The two editor preset lists
    /// stay out of it: they are kept in the profile itself, which is what REQ-PERS-016 asks for,
    /// and the model holds them as opaque values the host fills in if it wants to.
    /// </remarks>
    public static IReadOnlyList<KeyValuePair<string, SettingValue>> ToSettingValues(ProfileSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);

        List<KeyValuePair<string, SettingValue>> values = [];
        void Add(string key, SettingValue value)
            => values.Add(new KeyValuePair<string, SettingValue>(key, value));

        if (settings.Light?.Direction is { } direction)
        {
            Add(
                SettingKeys.LightDirection,
                SettingValue.OfVector3(new SettingVector3(direction.X, direction.Y, direction.Z)));
        }

        if (settings.Light?.Color is { } colour)
        {
            Add(SettingKeys.LightColor, SettingValue.OfColor(ToSettingColor(colour)));
        }

        if (settings.Clipping is { } clipping)
        {
            Add(SettingKeys.ClipLeft, SettingValue.OfFloat(clipping.X));
            Add(SettingKeys.ClipBottom, SettingValue.OfFloat(clipping.Y));
            Add(SettingKeys.ClipRight, SettingValue.OfFloat(clipping.Z));
            Add(SettingKeys.ClipTop, SettingValue.OfFloat(clipping.W));
        }

        Add(SettingKeys.SceneMapEnable, SettingValue.OfBool(settings.MapEnable));
        Add(SettingKeys.VideoGraphicQuality, SettingValue.OfEnum(settings.GraphicQuality));
        Add(SettingKeys.ConvSameBgPalette, SettingValue.OfBool(settings.SameBackgroundPalette));
        Add(SettingKeys.ConvSameSpPalette, SettingValue.OfBool(settings.SameSpritePalette));
        Add(SettingKeys.ConvSpriteTolerance, SettingValue.OfInt(settings.SpriteTolerance));
        Add(SettingKeys.ConvScanDirection, SettingValue.OfEnum(settings.ScanDirection));
        Add(SettingKeys.ConvEmptyTile, SettingValue.OfBool(settings.EmptyTile));
        Add(SettingKeys.CameraLocalCam, SettingValue.OfBool(settings.LocalCamera));
        Add(SettingKeys.EmuTvMode, SettingValue.OfEnum((int)settings.TvMode));

        if (settings.Layers is { } layers)
        {
            Add(SettingKeys.LayerDefaultLayer, SettingValue.OfEnum((int)layers.DefaultLayer));
            Add(SettingKeys.LayerAuto, SettingValue.OfBool(layers.Auto));
            for (int i = 0; i < LayerSettingsSet.LayerCount && i < layers.Layers.Count; i++)
            {
                LayerSettings layer = layers.Layers[i];
                Add(SettingKeys.LayerOffset(i), SettingValue.OfFloat(layer.Offset));
                Add(SettingKeys.LayerAligned(i), SettingValue.OfEnum((int)layer.Aligned));

                // The shadow flags are not taken from the file at all: the model resets them
                // itself (REQ-PERS-005 point 4, REQ-PROF-042).
            }
        }

        return values;
    }

    /// <summary>
    /// A profile colour as a model colour. The stored alpha is deliberately dropped: readers of
    /// a profile colour treat it as fully opaque (SPEC-03 §4.7), and the byte is kept in the
    /// profile itself for a lossless rewrite (REQ-PROF-050).
    /// </summary>
    private static SettingColor ToSettingColor(ArgbColor colour)
        => new(colour.R / 255f, colour.G / 255f, colour.B / 255f, 1f);

    private static string[] BuildAppliedKeys()
    {
        List<string> keys =
        [
            LightDirectionKey,
            LightColorKey,
            .. ClippingKeys,
            "scene.map_enable",
            "layer.default_layer",
            "layer.auto",
        ];

        for (int i = 0; i < LayerSettingsSet.LayerCount; i++)
        {
            keys.Add($"layer[{i}].offset");
            keys.Add($"layer[{i}].aligned");
            keys.Add($"layer[{i}].cast_shadow");
            keys.Add($"layer[{i}].recv_shadow");
        }

        keys.AddRange(
        [
            "video.graphic_quality",
            "editor.saved_patterns",
            "editor.saved_anim_patterns",
            "conv.same_bg_palette",
            "conv.same_sp_palette",
            "conv.sprite_tolerance",
            "conv.scan_direction",
            "conv.empty_tile",
            "camera.local_cam",
            "emu.tv_mode",
            UseNesBackgroundColorKey,
        ]);

        return [.. keys];
    }
}
