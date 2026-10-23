using Zax.App.Audio;
using Zax.App.Ui;
using Zax.Platform;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// The options window of AMD-010 REQ-UI-202: generated from the settings catalogue of SPEC-09,
/// grouped in the six tabs the amendment names, with a reset per tab.
/// </summary>
[Collection(ImGuiCollection.Name)]
public sealed class OptionsWindowTests
{
    /// <summary>
    /// REQ-UI-202: every user preference of SPEC-09 is in the window, except the ones that are
    /// out of scope (VR, REQ-PERS-011) or that are remembered state rather than a choice.
    /// </summary>
    [Fact]
    public void REQ_UI_202_every_user_preference_is_shown()
    {
        OptionsModel model = new();
        HashSet<string> shown = [.. model.AllRows().Select(static row => row.Descriptor.Key)];

        List<string> missing = [];
        foreach (SettingDescriptor descriptor in SettingsCatalogue.OfScope(SettingScope.User))
        {
            if (shown.Contains(descriptor.Key)
                || OptionsModel.ExcludedKeys.Contains(descriptor.Key)
                || descriptor.Theme is SettingTheme.Vr or SettingTheme.Editor
                    or SettingTheme.Movie or SettingTheme.Internal)
            {
                continue;
            }

            missing.Add(descriptor.Key);
        }

        Assert.Empty(missing);
    }

    /// <summary>
    /// REQ-UI-202: the settings SPEC-10 REQ-UI-061 made visible on desktop are all reachable in
    /// the window — that is the amendment's own acceptance criterion.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_settings_spec10_showed_are_all_there()
    {
        OptionsModel model = new();
        HashSet<string> shown = [.. model.AllRows().Select(static row => row.Descriptor.Key)];

        // The rows of the table of REQ-UI-061, in its order. « Musique de menu »
        // (audio.bg_music) left the window with the menu music itself, which AMD-024
        // REQ-PUB-005 removed on 2026-10-21; the setting stays in the model, without effect.
        string[] visible =
        [
            SettingKeys.LightIntensity,
            SettingKeys.AudioVolume,
            SettingKeys.RenderShadow,
            SettingKeys.VideoFrameRate,
            SettingKeys.VideoSkipFrame,
            SettingKeys.EmuGameSpeed,
            SettingKeys.CameraFov,
            SettingKeys.EmuTvMode,
            SettingKeys.SceneSkybox,
            SettingKeys.UiFullscreen,
        ];

        Assert.All(visible, key => Assert.Contains(key, shown));

        // REQ-UI-063 and REQ-UI-090: the custom background colour goes with the skybox list.
        Assert.Contains(SettingKeys.SceneCustomBgColor, shown);
    }

    /// <summary>
    /// REQ-UI-202 with AMD-035: the six tabs the amendment names, in the interface language,
    /// and every row lands in one of them with a non-empty label.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_six_tabs_are_the_ones_the_amendment_names()
    {
        using UiLanguageScope language = new();

        Assert.Equal(
            ["Video", "Camera", "Light and shadows", "Audio", "Emulation", "Interface"],
            OptionsModel.Tabs.Select(OptionsModel.TitleOf));

        UiStrings.Language = UiLanguage.French;
        Assert.Equal(
            ["Vidéo", "Caméra", "Lumière et ombres", "Audio", "Émulation", "Interface"],
            OptionsModel.Tabs.Select(OptionsModel.TitleOf));

        UiStrings.Language = UiLanguage.English;

        OptionsModel model = new();
        foreach (OptionsTab tab in OptionsModel.Tabs)
        {
            Assert.NotEmpty(model.Rows(tab));
            Assert.All(model.Rows(tab), row => Assert.NotEmpty(row.Label));
        }

        // The field of view belongs to the camera tab, the shadows to the light one, and the
        // scene decor to the video one.
        Assert.Contains(SettingKeys.CameraFov, Keys(model, OptionsTab.Camera));
        Assert.Contains(SettingKeys.RenderShadow, Keys(model, OptionsTab.Light));
        Assert.Contains(SettingKeys.LightIntensity, Keys(model, OptionsTab.Light));
        Assert.Contains(SettingKeys.SceneSkybox, Keys(model, OptionsTab.Video));
        Assert.Contains(SettingKeys.AudioVolume, Keys(model, OptionsTab.Audio));
        Assert.Contains(SettingKeys.EmuGameSpeed, Keys(model, OptionsTab.Emulation));
        Assert.Contains(SettingKeys.UiShowFps, Keys(model, OptionsTab.Interface));
    }

    /// <summary>
    /// REQ-UI-202: the VR settings never appear, since SPEC-10 §1 puts them out of scope and
    /// REQ-PERS-011 keeps them in the model only.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_vr_settings_are_absent()
    {
        OptionsModel model = new();
        HashSet<string> shown = [.. model.AllRows().Select(static row => row.Descriptor.Key)];

        foreach (SettingDescriptor descriptor in SettingsCatalogue.OfTheme(SettingTheme.Vr))
        {
            Assert.DoesNotContain(descriptor.Key, shown);
        }

        Assert.DoesNotContain(SettingKeys.UiLastBrowseDir, shown);
        Assert.DoesNotContain(SettingKeys.UiWindowRect, shown);
    }

    /// <summary>
    /// REQ-UI-202: the bounds a widget offers are the interface ones, not the wider range a
    /// script may write — 45 to 95 degrees of field of view (SPEC-10 REQ-UI-061, vectors V2) and
    /// 0,5 to 1,5 of light intensity.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_widgets_use_the_interface_bounds()
    {
        OptionsModel model = new();

        OptionsRow fov = Row(model, SettingKeys.CameraFov);
        Assert.Equal(45d, fov.Minimum);
        Assert.Equal(95d, fov.Maximum);
        Assert.Equal(5d, fov.Step);

        OptionsRow light = Row(model, SettingKeys.LightIntensity);
        Assert.Equal(0.5d, light.Minimum, 1e-9);
        Assert.Equal(1.5d, light.Maximum, 1e-9);

        OptionsRow speed = Row(model, SettingKeys.EmuGameSpeed);
        Assert.Equal(-8d, speed.Minimum);
        Assert.Equal(8d, speed.Maximum);
    }

    /// <summary>
    /// REQ-UI-202: a change made in the window takes effect through the model of SPEC-09 and is
    /// persisted, and « Valeurs par défaut » puts the whole tab back to the defaults of
    /// REQ-PERS-002.
    /// </summary>
    [Fact]
    public void REQ_UI_202_a_change_takes_effect_and_the_reset_restores_the_defaults()
    {
        using UiHarness harness = new("Arkanoid");
        OptionsModel model = harness.Ui.Options;
        SettingsModel settings = harness.Settings.Model;

        settings.SetFloat(SettingKeys.CameraFov, 85f);
        settings.SetBool(SettingKeys.RenderShadow, false);

        // REQ-PERS-009: the change reaches the scene at once.
        Assert.Equal(85f, harness.Scene.Settings.Fov, 1e-4f);
        Assert.False(harness.Scene.Settings.Shadows);
        Assert.True(harness.Store.HasPendingWrite);

        model.ResetTab(settings, OptionsTab.Camera);
        Assert.Equal(70f, settings.GetFloat(SettingKeys.CameraFov), 1e-4f);

        // The shadows are on the light tab, so the camera reset leaves them alone.
        Assert.False(settings.GetBool(SettingKeys.RenderShadow));

        model.ResetTab(settings, OptionsTab.Light);
        Assert.True(settings.GetBool(SettingKeys.RenderShadow));
    }

    /// <summary>
    /// REQ-UI-202: the window really draws, in a headless ImGui frame — every tab, every widget
    /// kind, and the reset button.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_window_draws()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        harness.Ui.State.Options = true;
        imgui.Frames(3, () => harness.Ui.Draw(1f / 60f));

        Assert.True(harness.Ui.State.Options);
    }

    /// <summary>
    /// AMD-021 R2-013 (which replaces the note and the table of SPEC-11 REQ-AUD-041): Speed and
    /// FOV change value without a sound; every other setting of the page plays the validation
    /// sound.
    /// </summary>
    [Fact]
    public void AMD_021_R2_013_speed_and_fov_are_the_only_silent_settings()
    {
        Assert.Equal<string[]>(
            [SettingKeys.EmuGameSpeed, SettingKeys.CameraFov],
            [.. OptionsModel.SilentKeys]);

        Assert.False(OptionsModel.PlaysValidationSound(SettingKeys.EmuGameSpeed));
        Assert.False(OptionsModel.PlaysValidationSound(SettingKeys.CameraFov));

        // Every other row of every tab, the drop-downs, the check boxes and the colour picker
        // included: the validation sound.
        OptionsModel model = new();
        foreach (OptionsRow row in model.AllRows())
        {
            string key = row.Descriptor.Key;
            bool silent = key == SettingKeys.EmuGameSpeed || key == SettingKeys.CameraFov;
            Assert.Equal(!silent, OptionsModel.PlaysValidationSound(key));
        }

        // The named rows of the table of REQ-AUD-041, now extended to the whole page.
        foreach (string key in new[]
        {
            SettingKeys.AudioVolume,
            SettingKeys.LightIntensity,
            SettingKeys.RenderShadow,
            SettingKeys.SceneSkybox,
            SettingKeys.VideoFrameRate,
            SettingKeys.UiFullscreen,
            SettingKeys.SceneCustomBgColor,
        })
        {
            Assert.True(OptionsModel.PlaysValidationSound(key), key);
        }
    }

    [Fact] // AMD-021 R2-013 (the audio model: exactly one sound per change of value)
    public void AMD_021_R2_013_a_change_plays_the_validation_sound_exactly_once()
    {
        // The validation clip of REQ-AUD-041; the harness mixer carries no assets, so the clip
        // is supplied directly. REQ-AUD-040 has a single interface voice, which is why the
        // count and not SoundPlaying is what tells one sound from two.
        AudioMixer mixer = new(48_000);
        WaveAudio validation = new(new float[480], 1, 48_000);

        long Play(string key)
        {
            long before = mixer.SoundsPlayed;
            if (OptionsModel.PlaysValidationSound(key))
            {
                mixer.PlayClip(validation);
            }

            return mixer.SoundsPlayed - before;
        }

        // Three changes of the shadow check box: three sounds, one each.
        Assert.Equal(1, Play(SettingKeys.RenderShadow));
        Assert.Equal(1, Play(SettingKeys.RenderShadow));
        Assert.Equal(1, Play(SettingKeys.AudioVolume));

        // Speed and FOV: none.
        Assert.Equal(0, Play(SettingKeys.EmuGameSpeed));
        Assert.Equal(0, Play(SettingKeys.CameraFov));
    }

    private static IEnumerable<string> Keys(OptionsModel model, OptionsTab tab)
        => model.Rows(tab).Select(static row => row.Descriptor.Key);

    private static OptionsRow Row(OptionsModel model, string key)
        => model.AllRows().Single(row => string.Equals(row.Descriptor.Key, key, StringComparison.Ordinal));
}
