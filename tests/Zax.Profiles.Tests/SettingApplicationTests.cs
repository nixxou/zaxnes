namespace Zax.Profiles.Tests;

/// <summary>
/// What a profile does to the user's settings when it loads (REQ-PROF-040, REQ-PROF-041,
/// REQ-PROF-042) and the editor presets it carries (REQ-PROF-044).
/// </summary>
public sealed class SettingApplicationTests
{
    [Fact]
    public void REQ_PROF_041_the_applied_and_preserved_keys_are_two_disjoint_sets()
    {
        HashSet<string> applied = [.. ProfileSettingApplication.AppliedKeys];
        HashSet<string> preserved = [.. ProfileSettingApplication.PreservedKeys];

        Assert.Empty(applied.Intersect(preserved));
        Assert.Equal(ProfileSettingApplication.AppliedKeys.Count, applied.Count);
        Assert.Equal(ProfileSettingApplication.PreservedKeys.Count, preserved.Count);
    }

    [Fact]
    public void REQ_PROF_041_the_light_direction_and_colour_are_applied_but_not_its_intensity()
    {
        Assert.Contains("light.direction", ProfileSettingApplication.AppliedKeys);
        Assert.Contains("light.color", ProfileSettingApplication.AppliedKeys);
        Assert.Contains("light.intensity", ProfileSettingApplication.PreservedKeys);
        Assert.DoesNotContain("light.intensity", ProfileSettingApplication.AppliedKeys);
    }

    [Fact]
    public void REQ_PROF_041_the_user_settings_of_the_second_list_are_all_preserved()
    {
        string[] expected =
        [
            "light.intensity", "render.mesh_algo", "vr.runtime", "scene.skybox", "emu.core",
            "emu.game_speed", "render.shadow", "audio.volume", "video.frame_rate", "video.vsync",
            "vr.control_method", "vr.scene_ratio", "input.emulated_menu_button",
            "video.render_method", "video.res_scale", "video.skip_frame", "audio.bg_music",
            // scene.tv_box stood between emu.auto_tv_mode and ui.sort_option; AMD-031 retired
            // the setting with the TV box, so there is no preference left to preserve.
            "video.anti_alias", "vr.mirroring", "emu.auto_tv_mode",
            "ui.sort_option", "scene.custom_bg_color", "camera.fov",
        ];

        Assert.Equal(expected, ProfileSettingApplication.PreservedKeys);
    }

    [Fact]
    public void REQ_PROF_041_the_applied_keys_cover_the_whole_first_list()
    {
        string[] expected =
        [
            "light.direction", "light.color",
            "clip.left", "clip.bottom", "clip.right", "clip.top",
            "scene.map_enable", "layer.default_layer", "layer.auto",
            "video.graphic_quality", "editor.saved_patterns", "editor.saved_anim_patterns",
            "conv.same_bg_palette", "conv.same_sp_palette", "conv.sprite_tolerance",
            "conv.scan_direction", "conv.empty_tile", "camera.local_cam", "emu.tv_mode",
            "scene.use_nes_bg_color",
        ];

        foreach (string key in expected)
        {
            Assert.Contains(key, ProfileSettingApplication.AppliedKeys);
        }

        for (int i = 0; i < LayerSettingsSet.LayerCount; i++)
        {
            Assert.Contains($"layer[{i}].offset", ProfileSettingApplication.AppliedKeys);
            Assert.Contains($"layer[{i}].aligned", ProfileSettingApplication.AppliedKeys);
            Assert.Contains($"layer[{i}].cast_shadow", ProfileSettingApplication.AppliedKeys);
            Assert.Contains($"layer[{i}].recv_shadow", ProfileSettingApplication.AppliedKeys);
        }

        Assert.Equal(expected.Length + (LayerSettingsSet.LayerCount * 4), ProfileSettingApplication.AppliedKeys.Count);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_041_loading_empty_bytes_applies_the_documented_values()
    {
        ProfileSettings settings = NeutralAssets.LoadProfile("empty.bytes").Profile.Settings!;
        IReadOnlyDictionary<string, object?> values = ProfileSettingApplication.ValuesToApply(settings);

        Assert.Equal(new Vec3F(1f, 0.2f, 1f), values["light.direction"]);
        Assert.Equal(new ArgbColor(-1), values["light.color"]);
        Assert.Equal(4.1f, (float)values["clip.left"]!);
        Assert.Equal(16.1f, (float)values["clip.bottom"]!);
        Assert.Equal(4.1f, (float)values["clip.right"]!);
        Assert.Equal(0.1f, (float)values["clip.top"]!);
        Assert.Equal(2, values["video.graphic_quality"]);
        Assert.Equal(ProfileLayer.L1, values["layer.default_layer"]);
        Assert.Equal(TvRegionMode.Ntsc, values["emu.tv_mode"]);

        // Forced true whatever the profile says.
        Assert.Equal(true, values["scene.use_nes_bg_color"]);

        // Never applied, so never offered here.
        Assert.False(values.ContainsKey("light.intensity"));
        Assert.False(values.ContainsKey("audio.volume"));
        Assert.False(values.ContainsKey("camera.fov"));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_042_the_applied_shadow_flags_ignore_what_the_profile_stores()
    {
        ProfileSettings settings = NeutralAssets.LoadProfile("empty.bytes").Profile.Settings!;

        // The file stores (F,F) (F,F) (T,T) (F,T) (F,T) (F,T).
        Assert.True(settings.Layers!.Layers[2].StoredCastShadow);
        Assert.False(settings.Layers.Layers[3].StoredCastShadow);
        Assert.False(settings.Layers.Layers[0].StoredRecvShadow);

        IReadOnlyDictionary<string, object?> values = ProfileSettingApplication.ValuesToApply(settings);
        bool[] expectedCast = [false, false, true, false, false, false];
        bool[] expectedRecv = [false, false, true, true, true, true];

        for (int i = 0; i < LayerSettingsSet.LayerCount; i++)
        {
            Assert.Equal(expectedCast[i], values[$"layer[{i}].cast_shadow"]);
            Assert.Equal(expectedRecv[i], values[$"layer[{i}].recv_shadow"]);
        }
    }

    [Fact]
    public void REQ_PROF_040_a_settings_message_starts_at_the_documented_defaults()
    {
        ProfileSettings settings = new();

        Assert.Equal(70f, settings.Fov);
        Assert.Equal(-1, settings.Skybox);
        Assert.True(settings.Mirroring);
        Assert.Equal(1f, settings.SceneRatio);
        Assert.True(settings.SkipFrame);
        Assert.True(settings.BackgroundMusic);

        Assert.False(settings.EmptyTile);
        Assert.False(settings.UseNesBackgroundColor);
        Assert.Equal(0, settings.GraphicQuality);
        Assert.Equal(0f, settings.Volume);
        Assert.Equal(TvRegionMode.Ntsc, settings.TvMode);
    }

    [Fact]
    public void REQ_PROF_040_a_layer_without_shadow_flags_stores_them_as_false()
    {
        LayerSettings layer = new();

        Assert.False(layer.StoredCastShadow);
        Assert.False(layer.StoredRecvShadow);
        Assert.Equal(LayerAlignment.Front, layer.Aligned);
        Assert.Equal(1f, layer.Coefficient);
    }

    [Fact]
    public void REQ_PERS_013_the_layer_coefficient_follows_the_alignment()
    {
        Assert.Equal(1f, new LayerSettings { Aligned = LayerAlignment.Front }.Coefficient);
        Assert.Equal(0f, new LayerSettings { Aligned = LayerAlignment.Center }.Coefficient);
        Assert.Equal(-1f, new LayerSettings { Aligned = LayerAlignment.Back }.Coefficient);
    }

    [Fact]
    public void REQ_PERS_002_the_default_layer_set_has_the_documented_offsets()
    {
        LayerSettingsSet layers = LayerSettingsSet.CreateDefault();

        Assert.Equal(LayerSettingsSet.LayerCount, layers.Layers.Count);
        Assert.Equal(ProfileLayer.L1, layers.DefaultLayer);
        Assert.Equal(0f, layers[2]!.Offset);
        Assert.Equal(12f, layers[3]!.Offset);
        Assert.Equal(24f, layers[4]!.Offset);
        Assert.Equal(36f, layers[5]!.Offset);
        Assert.Equal(LayerAlignment.Center, layers[1]!.Aligned);
        Assert.Equal(LayerAlignment.Front, layers[2]!.Aligned);
        Assert.Null(layers[6]);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_044_the_memorised_slots_are_kept_with_their_names_and_patterns()
    {
        ProfileSettings settings = NeutralAssets.LoadProfile("empty.bytes").Profile.Settings!;

        Assert.Equal(5, settings.SavedPatterns.Count);
        Assert.Equal(5, settings.SavedAnimPatterns.Count);

        foreach (RecordedPatternSlot slot in settings.SavedPatterns)
        {
            ProfilePattern pattern = slot.Pattern!;
            Assert.Null(pattern.Grid);
            Assert.Equal(new Vec2I(0, 0), pattern.TileSize);
            Assert.Single(pattern.Variants);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_044_every_built_in_profile_carries_ten_slots()
    {
        int slots = 0;
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            ProfileSettings settings = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile.Settings!;
            Assert.Equal(5, settings.SavedPatterns.Count);
            Assert.Equal(5, settings.SavedAnimPatterns.Count);
            slots += settings.SavedPatterns.Count + settings.SavedAnimPatterns.Count;
        }

        Assert.Equal(1010, slots);
    }
}
