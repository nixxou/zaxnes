using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>Value validation (REQ-PERS-003) against the vectors of SPEC-09 §5.1 and §5.2.</summary>
public sealed class ValidationTests
{
    /// <summary>REQ-PERS-003 rule 1 and §5.1: a render rate outside {60, 90, 120} becomes 90.</summary>
    [Theory]
    [InlineData(60, 60)]
    [InlineData(90, 90)]
    [InlineData(120, 120)]
    [InlineData(75, 90)]
    [InlineData(0, 90)]
    public void Render_rate_outside_the_three_values_becomes_ninety(int written, int expected)
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.VideoFrameRate, written);
        Assert.Equal(expected, model.GetInt(SettingKeys.VideoFrameRate));
    }

    /// <summary>REQ-PERS-003 rule 3 and §5.1: the speed index is bounded to [−8, 8].</summary>
    [Theory]
    [InlineData(-9f, -8f)]
    [InlineData(3.4f, 3.4f)]
    [InlineData(8f, 8f)]
    [InlineData(12f, 8f)]
    public void Game_speed_is_bounded(float written, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.EmuGameSpeed, written);
        Assert.Equal(expected, model.GetFloat(SettingKeys.EmuGameSpeed), 4);
    }

    /// <summary>REQ-PERS-003 rule 4 and §5.1: a negative sprite tolerance becomes its absolute value.</summary>
    [Fact]
    public void Sprite_tolerance_drops_its_sign()
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.ConvSpriteTolerance, -3);
        Assert.Equal(3, model.GetInt(SettingKeys.ConvSpriteTolerance));
    }

    /// <summary>§5.1: a skybox outside its four values falls back to the default.</summary>
    [Fact]
    public void An_unknown_skybox_falls_back_to_the_default()
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.SceneSkybox, 7);
        Assert.Equal(-1, model.GetInt(SettingKeys.SceneSkybox));
    }

    /// <summary>§5.1: a resolution scale index outside 0 to 4 falls back to the default.</summary>
    [Fact]
    public void An_unknown_resolution_scale_falls_back_to_the_default()
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.VideoResScale, 5);
        Assert.Equal(0, model.GetInt(SettingKeys.VideoResScale));
    }

    /// <summary>
    /// REQ-PERS-003 rule 2 and §5.1: the core index is the remainder of the division by the
    /// number of cores, which is 1 in v1, so the effective value is always 0.
    /// </summary>
    [Fact]
    public void The_core_index_is_taken_modulo_the_number_of_cores()
    {
        SettingsModel single = new();
        single.SetInt(SettingKeys.EmuCore, 3);
        Assert.Equal(0, single.GetInt(SettingKeys.EmuCore));

        SettingsModel dual = new() { CoreCount = 2 };
        dual.SetInt(SettingKeys.EmuCore, 3);
        Assert.Equal(1, dual.GetInt(SettingKeys.EmuCore));
    }

    /// <summary>§4.1.5: a clipping margin outside its interval is clamped.</summary>
    [Theory]
    [InlineData(SettingKeys.ClipLeft, -4f, 0f)]
    [InlineData(SettingKeys.ClipLeft, 300f, 256f)]
    [InlineData(SettingKeys.ClipBottom, 300f, 248f)]
    [InlineData(SettingKeys.ClipTop, 300f, 248f)]
    [InlineData(SettingKeys.ClipRight, 300f, 256f)]
    public void A_clipping_margin_is_clamped(string key, float written, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(key, written);
        Assert.Equal(expected, model.GetFloat(key));
    }

    /// <summary>§4.1.2: a script's field of view is bounded to [10, 150].</summary>
    [Theory]
    [InlineData(5f, 10f)]
    [InlineData(80f, 80f)]
    [InlineData(400f, 150f)]
    public void The_field_of_view_is_bounded(float written, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraFov, written);
        Assert.Equal(expected, model.GetFloat(SettingKeys.CameraFov));
    }

    /// <summary>§4.1.9: the volume is bounded to [0, 1].</summary>
    [Theory]
    [InlineData(-1f, 0f)]
    [InlineData(0.37f, 0.37f)]
    [InlineData(4f, 1f)]
    public void The_volume_is_bounded(float written, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.AudioVolume, written);
        Assert.Equal(expected, model.GetFloat(SettingKeys.AudioVolume), 4);
    }

    /// <summary>
    /// §4.1.2: the smoothing time's domain is "&gt; 0" with no repair rule, so a value outside it
    /// takes the default (REQ-PERS-003).
    /// </summary>
    [Theory]
    [InlineData(0.7f, 0.7f)]
    [InlineData(0f, 0.4f)]
    [InlineData(-2f, 0.4f)]
    public void A_non_positive_smoothing_time_takes_the_default(float written, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraSmoothTime, written);
        Assert.Equal(expected, model.GetFloat(SettingKeys.CameraSmoothTime), 4);
    }

    /// <summary>§4.1.3: the light direction's domain is "non nul".</summary>
    [Fact]
    public void A_zero_light_direction_takes_the_default()
    {
        SettingsModel model = new();
        model.SetVector3(SettingKeys.LightDirection, new SettingVector3(0f, 0f, 0f));
        Assert.Equal(new SettingVector3(1f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));

        model.SetVector3(SettingKeys.LightDirection, new SettingVector3(0f, -1f, 1f));
        Assert.Equal(new SettingVector3(0f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));
    }

    /// <summary>
    /// SPEC-03 REQ-PROF-041 acceptance criterion: a light intensity of 2 survives, so the domain
    /// is not the slider's 0.5 to 1.5.
    /// </summary>
    [Fact]
    public void The_light_intensity_is_not_bounded_to_the_slider_range()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.LightIntensity, 2f);
        Assert.Equal(2f, model.GetFloat(SettingKeys.LightIntensity));
    }

    /// <summary>REQ-PERS-004: a setting accepts the number of its enumeration.</summary>
    [Fact]
    public void An_enumerated_setting_accepts_its_number()
    {
        SettingsModel model = new();

        // Setting.Layer[2].Aligned = 2 gives layer 2 the BACK alignment.
        model.SetInt(SettingKeys.LayerAligned(2), 2);
        Assert.Equal(2, model.GetInt(SettingKeys.LayerAligned(2)));
        Assert.Equal(-1f, model.GetFloat(SettingKeys.LayerCoef(2)));
    }

    /// <summary>§5.2: the UI slider positions map to the documented values.</summary>
    [Theory]
    [InlineData(-5, 45f)]
    [InlineData(0, 70f)]
    [InlineData(5, 95f)]
    public void The_field_of_view_slider_positions_map_to_their_values(int position, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraFov, 70f + (5f * position));
        Assert.Equal(expected, model.GetFloat(SettingKeys.CameraFov));
    }

    /// <summary>§5.2: the light slider runs 0 to 100 for 0.5 to 1.5.</summary>
    [Theory]
    [InlineData(0, 0.5f)]
    [InlineData(50, 1f)]
    [InlineData(100, 1.5f)]
    public void The_light_slider_positions_map_to_their_values(int position, float expected)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.LightIntensity, 0.5f + (position / 100f));
        Assert.Equal(expected, model.GetFloat(SettingKeys.LightIntensity), 4);
    }
}
