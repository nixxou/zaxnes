using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The layers of values and what a profile load does to them (REQ-PERS-005 to REQ-PERS-013,
/// REQ-PERS-016, REQ-PERS-017, REQ-PERS-082).
/// </summary>
public sealed class LayeringTests
{
    /// <summary>
    /// REQ-PERS-005 and §5.4: loading a different game replaces every P setting with the
    /// profile's value, keeps every U setting, forces <c>BgColor</c> true and resets the six
    /// pairs of layer shadow flags.
    /// </summary>
    [Fact]
    public void Loading_a_different_game_replaces_the_profile_settings_and_keeps_the_preferences()
    {
        SettingsModel model = new();

        // State before: the user's preferences and the previous game's profile values.
        model.SetFloat(SettingKeys.LightIntensity, 1.2f);
        model.SetInt(SettingKeys.SceneSkybox, 1);
        model.SetFloat(SettingKeys.CameraFov, 80f);
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Profile);
        model.SetInt(SettingKeys.EmuTvMode, 1, SettingLayer.Profile);
        model.SetInt(SettingKeys.VideoGraphicQuality, 2, SettingLayer.Profile);

        model.ApplyProfile(
        [
            new(SettingKeys.LightIntensity, SettingValue.OfFloat(1f)),
            new(SettingKeys.LightDirection, SettingValue.OfVector3(new SettingVector3(-1f, -1f, 1f))),
            new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false)),
            new(SettingKeys.EmuTvMode, SettingValue.OfEnum(0)),
            new(SettingKeys.VideoGraphicQuality, SettingValue.OfEnum(0)),
            new(SettingKeys.SceneUseNesBgColor, SettingValue.OfBool(false)),
            new(SettingKeys.SceneSkybox, SettingValue.OfEnum(0)),
            new(SettingKeys.CameraFov, SettingValue.OfFloat(60f)),
        ]);

        // Preferences survive, the light intensity included (REQ-PERS-005 point 2).
        Assert.Equal(1.2f, model.GetFloat(SettingKeys.LightIntensity), 4);
        Assert.Equal(1, model.GetInt(SettingKeys.SceneSkybox));
        Assert.Equal(80f, model.GetFloat(SettingKeys.CameraFov));

        // Profile settings are replaced.
        Assert.Equal(new SettingVector3(-1f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.Equal(0, model.GetInt(SettingKeys.EmuTvMode));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoGraphicQuality));

        // Forced true whatever the profile says (point 3).
        Assert.True(model.GetBool(SettingKeys.SceneUseNesBgColor));

        // Shadow flags reset whatever the profile stores (point 4).
        Assert.Equal(
            [false, false, true, false, false, false],
            Enumerable.Range(0, 6).Select(i => model.GetBool(SettingKeys.LayerCastShadow(i))));
        Assert.Equal(
            [false, false, true, true, true, true],
            Enumerable.Range(0, 6).Select(i => model.GetBool(SettingKeys.LayerRecvShadow(i))));
    }

    /// <summary>
    /// REQ-PERS-005 acceptance criterion: a profile that sets <c>CastShadow</c> on every layer
    /// still ends up with layer 2 alone casting.
    /// </summary>
    [Fact]
    public void A_profile_that_sets_every_cast_shadow_flag_is_still_reset()
    {
        SettingsModel model = new();
        model.SetColor(SettingKeys.SceneCustomBgColor, new SettingColor(1f, 0f, 0f, 1f));
        model.SetFloat(SettingKeys.LightIntensity, 1.3f);

        List<KeyValuePair<string, SettingValue>> values =
        [
            new(SettingKeys.LightDirection, SettingValue.OfVector3(new SettingVector3(0f, -1f, 1f))),
            new(SettingKeys.SceneUseNesBgColor, SettingValue.OfBool(false)),
        ];
        for (int i = 0; i < SettingKeys.LayerCount; i++)
        {
            values.Add(new KeyValuePair<string, SettingValue>(
                SettingKeys.LayerCastShadow(i),
                SettingValue.OfBool(true)));
        }

        model.ApplyProfile(values);

        Assert.Equal(1.3f, model.GetFloat(SettingKeys.LightIntensity), 4);
        Assert.Equal(new SettingVector3(0f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));
        Assert.Equal(new SettingColor(1f, 0f, 0f, 1f), model.GetColor(SettingKeys.SceneCustomBgColor));
        Assert.True(model.GetBool(SettingKeys.SceneUseNesBgColor));
        Assert.Equal(
            [false, false, true, false, false, false],
            Enumerable.Range(0, 6).Select(i => model.GetBool(SettingKeys.LayerCastShadow(i))));
    }

    /// <summary>
    /// REQ-PERS-005 note: a profile setting the file does not carry takes the default of
    /// REQ-PROF-040, not the value the previous game left behind.
    /// </summary>
    [Fact]
    public void A_setting_absent_from_the_profile_takes_its_default_not_the_previous_value()
    {
        SettingsModel model = new();
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Profile);
        model.SetFloat(SettingKeys.ClipBottom, 16.1f, SettingLayer.Profile);

        model.ApplyProfile([new(SettingKeys.CameraLocalCam, SettingValue.OfBool(true))]);

        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.Equal(8.1f, model.GetFloat(SettingKeys.ClipBottom), 4);
        Assert.True(model.GetBool(SettingKeys.CameraLocalCam));
    }

    /// <summary>
    /// REQ-PERS-006: relaunching the same game does not reapply the profile, so what the scripts
    /// changed during the previous session of that game survives.
    /// </summary>
    [Fact]
    public void Relaunching_the_same_game_keeps_what_the_scripts_changed()
    {
        SettingsModel model = new();
        model.ApplyProfile([new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false))]);

        // A script turns it on during the game, outside the start-up hook.
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Session);
        model.EndOfGame();

        // Same game again: no profile application, so the value survives.
        Assert.True(model.GetBool(SettingKeys.ConvEmptyTile));

        // Game B then game A: the profile's value is back.
        model.ApplyProfile([new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false))]);
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
    }

    /// <summary>
    /// REQ-PERS-009: an effective change raises one event with its key; writing the same value
    /// raises none.
    /// </summary>
    [Fact]
    public void Only_an_effective_change_raises_an_event()
    {
        SettingsModel model = new();
        List<SettingChange> seen = [];
        model.Changed += seen.Add;

        model.SetFloat(SettingKeys.CameraFov, 80f);
        Assert.Equal([SettingKeys.CameraFov], seen.Select(static change => change.Key));
        Assert.Equal(SettingEffect.Immediate, seen[0].Effect);
        Assert.Equal(80f, seen[0].Value.AsFloat);

        seen.Clear();
        model.SetFloat(SettingKeys.CameraFov, 80f);
        Assert.Empty(seen);
    }

    /// <summary>
    /// REQ-PERS-010 and DEC-09-Q3: a script's write on a user preference is a session value; it
    /// is what everyone reads until the game ends, and the preference comes back afterwards.
    /// </summary>
    [Fact]
    public void A_script_write_on_a_preference_lives_in_the_session_layer()
    {
        SettingsModel model = new();
        SettingColor blue = new(0f, 0f, 1f, 1f);
        SettingColor scripted = new(0.2f, 0.2f, 0.2f, 1f);
        model.SetColor(SettingKeys.SceneCustomBgColor, blue);

        model.SetColor(SettingKeys.SceneCustomBgColor, scripted, SettingLayer.Session);
        Assert.Equal(scripted, model.GetColor(SettingKeys.SceneCustomBgColor));
        Assert.True(model.HasSessionValue(SettingKeys.SceneCustomBgColor));

        // What gets persisted is still the preference, never the script's value.
        Assert.Contains(
            model.UserPreferences(),
            pair => pair.Key == SettingKeys.SceneCustomBgColor && pair.Value.AsColor == blue);

        model.EndOfGame();
        Assert.Equal(blue, model.GetColor(SettingKeys.SceneCustomBgColor));
    }

    /// <summary>REQ-PERS-011: the VR runtime reads 0 and a write is ignored without error.</summary>
    [Fact]
    public void The_vr_runtime_always_reads_zero()
    {
        SettingsModel model = new();
        Assert.False(model.Set(SettingKeys.VrRuntime, SettingValue.OfInt(1)));
        Assert.Equal(0, model.GetInt(SettingKeys.VrRuntime));
    }

    /// <summary>
    /// REQ-PERS-011: the scene ratio reads 1, the written value is kept, and a change of the
    /// kept value is still notified so the scene can run its view reset.
    /// </summary>
    [Fact]
    public void The_scene_ratio_reads_one_but_keeps_what_was_written()
    {
        SettingsModel model = new();
        List<string> seen = [];
        model.Changed += change => seen.Add(change.Key);

        Assert.True(model.SetFloat(SettingKeys.VrSceneRatio, 5f));
        Assert.Equal(1f, model.GetFloat(SettingKeys.VrSceneRatio));
        Assert.Equal(5f, model.GetStored(SettingKeys.VrSceneRatio).AsFloat);
        Assert.Contains(SettingKeys.VrSceneRatio, seen);

        // A second write of the same value changes nothing.
        seen.Clear();
        Assert.False(model.SetFloat(SettingKeys.VrSceneRatio, 5f));
        Assert.Empty(seen);
    }

    /// <summary>REQ-PERS-013 point 3: writing <c>SceneScale</c> writes the scene ratio.</summary>
    [Fact]
    public void Writing_the_scene_scale_writes_the_scene_ratio()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.SceneSceneScale, 3f);
        Assert.Equal(3f, model.GetStored(SettingKeys.VrSceneRatio).AsFloat);

        // On desktop the ratio reads 1, so the scale stays RootScale × 1.
        Assert.Equal(1f, model.GetFloat(SettingKeys.SceneSceneScale));
    }

    /// <summary>
    /// REQ-PERS-012: at the end of a game the speed goes back to 0 and the smoothing time to
    /// 0.4.
    /// </summary>
    [Fact]
    public void The_end_of_a_game_resets_the_speed_and_the_smoothing_time()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.EmuGameSpeed, 4f);
        model.SetFloat(SettingKeys.CameraSmoothTime, 0.7f, SettingLayer.Session);

        model.EndOfGame();

        Assert.Equal(0f, model.GetFloat(SettingKeys.EmuGameSpeed));
        Assert.Equal(0.4f, model.GetFloat(SettingKeys.CameraSmoothTime), 4);
    }

    /// <summary>
    /// REQ-PERS-012 and DEC-09-Q6: both ports go back to Gamepad at every load of a different
    /// game, so a Zapper profile cannot leak into the next game.
    /// </summary>
    [Fact]
    public void Loading_a_different_game_puts_both_ports_back_to_gamepad()
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.InputPortType(0), 1);
        Assert.True(model.GetBool(SettingKeys.InputZapperEnable));

        model.ApplyProfile([]);

        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(0)));
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(1)));
        Assert.False(model.GetBool(SettingKeys.InputZapperEnable));
    }

    /// <summary>REQ-PERS-013 points 1 and 2: the target frame rate and the frame time.</summary>
    [Fact]
    public void The_target_frame_rate_follows_vsync_then_the_render_rate()
    {
        SettingsModel model = new() { DisplayRefreshRate = 144f };

        model.SetInt(SettingKeys.VideoVSync, 0);
        model.SetInt(SettingKeys.VideoFrameRate, 120);
        Assert.Equal(120f, model.GetFloat(SettingKeys.VideoTargetFramerate));
        Assert.Equal(0.008333f, model.GetFloat(SettingKeys.VideoTargetTimePerFrame), 5);

        model.SetInt(SettingKeys.VideoVSync, 1);
        Assert.Equal(144f, model.GetFloat(SettingKeys.VideoTargetFramerate));

        model.SetInt(SettingKeys.VideoVSync, 2);
        Assert.Equal(72f, model.GetFloat(SettingKeys.VideoTargetFramerate));
    }

    /// <summary>REQ-PERS-013 points 4 to 8: the other derived values.</summary>
    [Fact]
    public void The_derived_values_follow_the_settings_that_feed_them()
    {
        SettingsModel model = new();

        Assert.False(model.GetBool(SettingKeys.SceneSkyboxEnable));
        model.SetInt(SettingKeys.SceneSkybox, 2);
        Assert.True(model.GetBool(SettingKeys.SceneSkyboxEnable));

        Assert.Equal(0f, model.GetFloat(SettingKeys.LayerCoef(1)));
        Assert.Equal(1f, model.GetFloat(SettingKeys.LayerCoef(2)));

        foreach ((int index, float factor) in new[] { (0, 1f), (1, 2f / 3f), (2, 0.5f), (3, 1f / 3f), (4, 0.25f) })
        {
            model.SetInt(SettingKeys.VideoResScale, index);
            Assert.Equal(factor, model.GetFloat(SettingKeys.VideoResScaleFactor), 5);
        }
    }

    /// <summary>
    /// REQ-PERS-013: a change of a feeding setting also announces the derived value, so a
    /// subsystem can watch the derived key alone.
    /// </summary>
    [Fact]
    public void A_change_announces_the_derived_values_it_makes_stale()
    {
        SettingsModel model = new();
        List<string> seen = [];
        model.Changed += change => seen.Add(change.Key);

        model.SetInt(SettingKeys.SceneSkybox, 1);
        Assert.Contains(SettingKeys.SceneSkyboxEnable, seen);

        seen.Clear();
        model.SetInt(SettingKeys.LayerAligned(3), 2);
        Assert.Contains(SettingKeys.LayerCoef(3), seen);
    }

    /// <summary>REQ-PERS-016: the movie mode stays at NONE.</summary>
    [Fact]
    public void The_movie_mode_is_frozen_at_none()
    {
        SettingsModel model = new();
        Assert.False(model.SetInt(SettingKeys.MovieMode, 2));
        Assert.Equal(0, model.GetInt(SettingKeys.MovieMode));
    }

    /// <summary>
    /// REQ-PERS-017: the three settings with no observable effect are still stored and handed
    /// back, and they say so through their effect metadata.
    /// </summary>
    [Theory]
    [InlineData(SettingKeys.SceneMapEnable)]
    [InlineData(SettingKeys.InputEmulatedMenuButton)]
    public void The_settings_without_effect_are_still_stored(string key)
    {
        SettingsModel model = new();
        Assert.Equal(SettingEffect.None, SettingsCatalogue.Get(key).Effect);

        SettingLayer layer = SettingsCatalogue.Get(key).Scope == SettingScope.Profile
            ? SettingLayer.Profile
            : SettingLayer.User;
        model.SetBool(key, true, layer);
        Assert.True(model.GetBool(key));
    }

    /// <summary>
    /// REQ-PERS-017: <c>video.anti_alias</c> is stored and exposed; it may drive a full-screen
    /// pass, so its effect is immediate rather than none.
    /// </summary>
    [Fact]
    public void The_anti_aliasing_setting_is_stored_and_exposed()
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.VideoAntiAlias, 2);
        Assert.Equal(2, model.GetInt(SettingKeys.VideoAntiAlias));
        Assert.True(SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).IsLibretroExposed);
    }

    /// <summary>
    /// REQ-PERS-082: two models in the same process are independent and nothing touches a disk.
    /// </summary>
    [Fact]
    public void Two_models_are_independent()
    {
        SettingsModel first = new();
        SettingsModel second = new();

        first.SetFloat(SettingKeys.CameraFov, 95f);
        first.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Profile);

        Assert.Equal(70f, second.GetFloat(SettingKeys.CameraFov));
        Assert.False(second.GetBool(SettingKeys.ConvEmptyTile));
    }

    /// <summary>
    /// Q-066: a command-line override masks the preference without replacing it, so the
    /// preferences that get persisted are untouched (REQ-PERS-020); the user's own change ends
    /// the override, and a script's write still masks it while a game runs (REQ-PERS-010).
    /// </summary>
    [Fact]
    public void A_command_line_override_masks_the_preference_without_replacing_it()
    {
        SettingsModel model = new();
        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(2));

        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(0), SettingLayer.CommandLine);
        Assert.Equal(0, model.GetInt(SettingKeys.VideoVSync));
        Assert.True(model.HasCommandLineValue(SettingKeys.VideoVSync));

        // Only the user layer is persisted, and it still holds what the user chose.
        Assert.Equal(
            SettingValue.OfEnum(2),
            Assert.Single(model.UserPreferences(), pair => pair.Key == SettingKeys.VideoVSync).Value);

        // A script's write wins while the game runs and is undone at the end of it; the
        // override is then back in force.
        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(1), SettingLayer.Session);
        Assert.Equal(1, model.GetInt(SettingKeys.VideoVSync));
        model.ClearSessionLayer();
        Assert.Equal(0, model.GetInt(SettingKeys.VideoVSync));

        // The user's own change ends the override for that key, and is persisted.
        List<string> seen = [];
        model.Changed += change => seen.Add(change.Key);
        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(1));

        Assert.Equal(1, model.GetInt(SettingKeys.VideoVSync));
        Assert.False(model.HasCommandLineValue(SettingKeys.VideoVSync));
        Assert.Contains(SettingKeys.VideoVSync, seen);
        Assert.Equal(
            SettingValue.OfEnum(1),
            Assert.Single(model.UserPreferences(), pair => pair.Key == SettingKeys.VideoVSync).Value);
    }

    /// <summary>AMD-010 REQ-UI-202: the per-theme "Default values" button.</summary>
    [Fact]
    public void Resetting_a_theme_takes_its_settings_back_to_their_defaults()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraFov, 95f);
        model.SetFloat(SettingKeys.AudioVolume, 0.2f);

        model.ResetTheme(SettingTheme.Camera);

        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(0.2f, model.GetFloat(SettingKeys.AudioVolume), 4);
    }
}
