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

    /// <summary>
    /// AMD-041 REQ-LOOP-064: Run-ahead is a list of four values whose out-of-domain rule is a
    /// <b>clamp</b>, not the default — 7 gives 3 and −1 gives 0, where a plain choice domain
    /// would give 1.
    /// </summary>
    [Theory]
    [InlineData(0, 0)]
    [InlineData(1, 1)]
    [InlineData(3, 3)]
    [InlineData(7, 3)]
    [InlineData(-1, 0)]
    public void Run_ahead_is_clamped_to_its_four_values(int written, int expected)
    {
        SettingsModel model = new();
        Assert.Equal(1, model.GetInt(SettingKeys.EmuRunAhead));

        model.SetInt(SettingKeys.EmuRunAhead, written);
        Assert.Equal(expected, model.GetInt(SettingKeys.EmuRunAhead));
        Assert.Equal(expected, SettingsCatalogue.ClampRunAhead(written));
    }

    /// <summary>
    /// A finite list whose rule is « take the default » keeps that behaviour, which is what every
    /// other enumerated setting of §4.1 relies on.
    /// </summary>
    [Fact]
    public void A_plain_choice_still_falls_back_to_the_default()
    {
        SettingDomain choice = SettingDomain.Choice(
            new SettingOption(0d, "a"),
            new SettingOption(2d, "b"));

        Assert.Equal(
            2,
            choice.Coerce(SettingValue.OfInt(9), SettingValue.OfInt(2)).AsInt);
        Assert.Equal(
            0,
            choice.Coerce(SettingValue.OfInt(0), SettingValue.OfInt(2)).AsInt);
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

    /// <summary>
    /// §4.1.5 as amended by AMD-047: a clipping margin keeps any finite real, negative or
    /// larger than the reference dimensions included.
    /// </summary>
    [Theory] // REQ-PERS-087
    [InlineData(SettingKeys.ClipLeft, -128f)]
    [InlineData(SettingKeys.ClipLeft, 300f)]
    [InlineData(SettingKeys.ClipBottom, -80f)]
    [InlineData(SettingKeys.ClipBottom, 300f)]
    [InlineData(SettingKeys.ClipTop, -4f)]
    [InlineData(SettingKeys.ClipRight, -16f)]
    public void A_clipping_margin_keeps_any_finite_real(string key, float written)
    {
        SettingsModel model = new();
        model.SetFloat(key, written);
        Assert.Equal(written, model.GetFloat(key));
    }

    /// <summary>
    /// §4.1.5 as amended by AMD-047: a non-finite margin is out of domain and no rule repairs
    /// it, so the margin's own default applies (REQ-PERS-003).
    /// </summary>
    [Theory] // REQ-PERS-087, REQ-PERS-003
    [InlineData(SettingKeys.ClipLeft, 0.1f)]
    [InlineData(SettingKeys.ClipBottom, 8.1f)]
    [InlineData(SettingKeys.ClipRight, 8.1f)]
    [InlineData(SettingKeys.ClipTop, 0.1f)]
    public void A_non_finite_clipping_margin_takes_its_default(string key, float fallback)
    {
        foreach (float written in new[] { float.NaN, float.PositiveInfinity, float.NegativeInfinity })
        {
            SettingsModel model = new();
            model.SetFloat(key, -32f);
            model.SetFloat(key, written);
            Assert.Equal(fallback, model.GetFloat(key), 4);
        }
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
