using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The libretro core option table generated from the model (REQ-PERS-081, REQ-PERS-083,
/// REQ-PERS-085, SPEC-09 §4.6).
/// </summary>
public sealed class LibretroOptionTests
{
    /// <summary>REQ-PERS-083: the key mapping of the acceptance criterion.</summary>
    [Theory]
    [InlineData("camera.fov", "zaxnes_camera_fov")]
    [InlineData("layer[3].offset", "zaxnes_layer_3__offset")]
    [InlineData("input.port_type[0]", "zaxnes_input_port_type_0_")]
    public void The_core_option_key_replaces_the_separators(string key, string expected)
        => Assert.Equal(expected, SettingDescriptor.BuildLibretroKey(key));

    /// <summary>
    /// REQ-PERS-083 acceptance criterion: the generated table holds <c>zaxnes_camera_fov</c>
    /// with the values 45 to 95 and the default 70.
    /// </summary>
    [Fact]
    public void The_field_of_view_option_is_generated_from_the_model()
    {
        LibretroCoreOption option = Assert.NotNull(LibretroCoreOptions.Find("zaxnes_camera_fov"));

        Assert.Equal(SettingKeys.CameraFov, option.SettingKey);
        Assert.Equal(["45", "50", "55", "60", "65", "70", "75", "80", "85", "90", "95"], option.Values);
        Assert.Equal("70", option.DefaultValue);
        Assert.Equal("Camera", option.Category);
    }

    /// <summary>
    /// SPEC-09 §4.6: the normative extract, entry by entry. The eleven entries of the table are
    /// what the generator must produce, with <c>video.graphic_quality</c> and
    /// <c>emu.tv_mode</c> withdrawn by v1.1.
    /// </summary>
    [Fact]
    public void The_table_is_the_normative_extract_of_the_specification()
    {
        Assert.Equal(
            [
                "zaxnes_video_anti_alias",
                "zaxnes_render_mesh_algo",
                "zaxnes_render_shadow",
                "zaxnes_camera_fov",
                "zaxnes_light_intensity",
                "zaxnes_scene_skybox",
                "zaxnes_scene_custom_bg_color",
                "zaxnes_scene_tv_box",
                "zaxnes_emu_game_speed",
                "zaxnes_emu_auto_tv_mode",
                "zaxnes_audio_volume",
            ],
            LibretroCoreOptions.All.Select(static option => option.Key));
    }

    /// <summary>SPEC-09 §4.6: the values and defaults of the five single-line entries.</summary>
    [Theory]
    [InlineData("zaxnes_scene_skybox", "Default", "Default,Color,Gradient,Fantasy")]
    [InlineData("zaxnes_scene_tv_box", "None", "None,TV")] // AMD-025: one model
    [InlineData("zaxnes_render_mesh_algo", "GREEDY", "GREEDY,MARCHING,GREEDY_MULTICOLOR")]
    [InlineData("zaxnes_render_shadow", "enabled", "enabled,disabled")]
    [InlineData("zaxnes_emu_auto_tv_mode", "disabled", "enabled,disabled")]
    public void The_single_line_entries_match(string key, string expectedDefault, string expectedValues)
    {
        LibretroCoreOption option = Assert.NotNull(LibretroCoreOptions.Find(key));
        Assert.Equal(expectedDefault, option.DefaultValue);
        Assert.Equal(expectedValues.Split(','), option.Values);
    }

    /// <summary>SPEC-09 §4.6: the light intensity runs 0.50 to 1.50 in steps of 0.05.</summary>
    [Fact]
    public void The_light_intensity_is_discretised_in_steps_of_five_hundredths()
    {
        LibretroCoreOption option = Assert.NotNull(LibretroCoreOptions.Find("zaxnes_light_intensity"));

        Assert.Equal(21, option.Values.Count);
        Assert.Equal("0.50", option.Values[0]);
        Assert.Equal("1.50", option.Values[^1]);
        Assert.Equal("1.00", option.DefaultValue);
    }

    /// <summary>SPEC-09 §4.6: the speed runs −8 to 8.</summary>
    [Fact]
    public void The_game_speed_runs_from_minus_eight_to_eight()
    {
        LibretroCoreOption option = Assert.NotNull(LibretroCoreOptions.Find("zaxnes_emu_game_speed"));

        Assert.Equal(17, option.Values.Count);
        Assert.Equal("-8", option.Values[0]);
        Assert.Equal("8", option.Values[^1]);
        Assert.Equal("0", option.DefaultValue);
    }

    /// <summary>DEC-09-Q5: the background colour is exposed as eight presets.</summary>
    [Fact]
    public void The_background_colour_is_exposed_as_eight_presets()
    {
        LibretroCoreOption option = Assert.NotNull(LibretroCoreOptions.Find("zaxnes_scene_custom_bg_color"));

        Assert.Equal(
            ["Black", "White", "Gray", "NES Sky Blue", "Dark Blue", "Green", "Red", "Purple"],
            option.Values);
        Assert.Equal("Black", option.DefaultValue);
    }

    /// <summary>
    /// REQ-PERS-081: every exposed option offers a non-empty list, without duplicates, that
    /// holds its default.
    /// </summary>
    [Fact]
    public void Every_option_offers_a_clean_list_holding_its_default()
    {
        Assert.NotEmpty(LibretroCoreOptions.All);
        Assert.All(LibretroCoreOptions.All, option =>
        {
            Assert.NotEmpty(option.Values);
            Assert.Equal(option.Values.Count, option.Values.Distinct(StringComparer.Ordinal).Count());
            Assert.Contains(option.DefaultValue, option.Values);
            Assert.StartsWith("zaxnes_", option.Key, StringComparison.Ordinal);
        });
    }

    /// <summary>
    /// REQ-PERS-085: no P, S or D setting is exposed, and the keys §4.6 lists as host-owned are
    /// absent.
    /// </summary>
    [Theory]
    [InlineData(SettingKeys.VideoVSync)]
    [InlineData(SettingKeys.VideoFrameRate)]
    [InlineData(SettingKeys.VideoSkipFrame)]
    [InlineData(SettingKeys.VideoRenderMethod)]
    [InlineData(SettingKeys.VideoResScale)]
    [InlineData(SettingKeys.AudioBgMusic)]
    [InlineData(SettingKeys.InputEmulatedMenuButton)]
    [InlineData(SettingKeys.EmuCore)]
    [InlineData(SettingKeys.UiSortOption)]
    [InlineData(SettingKeys.UiFullscreen)]
    [InlineData(SettingKeys.VideoGraphicQuality)]
    [InlineData(SettingKeys.EmuTvMode)]
    [InlineData(SettingKeys.ConvEmptyTile)]
    [InlineData(SettingKeys.CameraSmoothTime)]
    [InlineData(SettingKeys.VrRuntime)]
    public void The_settings_the_host_owns_are_not_exposed(string key)
    {
        Assert.False(SettingsCatalogue.Get(key).IsLibretroExposed);
        Assert.DoesNotContain(key, LibretroCoreOptions.All.Select(static option => option.SettingKey));
    }

    /// <summary>REQ-PERS-085: no exposed option belongs to a scope other than U.</summary>
    [Fact]
    public void Every_exposed_option_is_a_user_preference()
        => Assert.All(
            LibretroCoreOptions.All,
            option => Assert.Equal(
                SettingScope.User,
                SettingsCatalogue.Get(option.SettingKey).Scope));

    /// <summary>
    /// REQ-PERS-085: a change from the host takes the same path as one from the UI, so it is
    /// validated and notified.
    /// </summary>
    [Fact]
    public void A_change_from_the_host_goes_through_the_model()
    {
        SettingsModel model = new();
        List<string> seen = [];
        model.Changed += change => seen.Add(change.Key);

        Assert.True(LibretroCoreOptions.TryApply(model, "zaxnes_camera_fov", "85"));
        Assert.Equal(85f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Contains(SettingKeys.CameraFov, seen);

        Assert.True(LibretroCoreOptions.TryApply(model, "zaxnes_render_shadow", "disabled"));
        Assert.False(model.GetBool(SettingKeys.RenderShadow));

        Assert.True(LibretroCoreOptions.TryApply(model, "zaxnes_audio_volume", "35%"));
        Assert.Equal(0.35f, model.GetFloat(SettingKeys.AudioVolume), 4);

        Assert.True(LibretroCoreOptions.TryApply(model, "zaxnes_scene_custom_bg_color", "NES Sky Blue"));
        Assert.Equal(SettingColor.FromRgb(0x0078FC), model.GetColor(SettingKeys.SceneCustomBgColor));
    }

    /// <summary>An unknown key or label changes nothing.</summary>
    [Fact]
    public void An_unknown_option_or_label_is_refused()
    {
        SettingsModel model = new();
        Assert.False(LibretroCoreOptions.TryApply(model, "zaxnes_not_a_setting", "1"));
        Assert.False(LibretroCoreOptions.TryApply(model, "zaxnes_camera_fov", "123"));
        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
    }
}
