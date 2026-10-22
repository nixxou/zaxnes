using System.Numerics;
using Zax.Engine3D;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The light and the shading of SPEC-06 §3.9 to §3.11 and the test vectors of §5.2.
/// </summary>
public sealed class ShadingTests
{
    /// <summary>The default light direction of REQ-SCN-025, white, intensity 1.</summary>
    private static Vector3 Direction => new(1f, -1f, 1f);

    private static Rgb24 White => new(255, 255, 255);

    [Fact]
    public void REQ_SCN_027_the_ambient_term_is_0_1812_in_linear_space()
        => Assert.Equal(0.1812f, SceneColor.AmbientLinear, 1e-4);

    [Fact]
    public void REQ_SCN_025_the_three_faces_towards_the_light_get_the_same_illumination()
    {
        foreach (Vector3 normal in new[]
                 {
                     new Vector3(0f, 0f, -1f),
                     new Vector3(0f, 1f, 0f),
                     new Vector3(-1f, 0f, 0f),
                 })
        {
            float ndl = Vector3.Dot(normal, Vector3.Normalize(-Direction));
            Assert.Equal(0.577f, ndl, 1e-3);
        }

        // The opposite faces only get the ambient light (REQ-SCN-025 criterion).
        foreach (Vector3 normal in new[]
                 {
                     new Vector3(1f, 0f, 0f),
                     new Vector3(0f, -1f, 0f),
                     new Vector3(0f, 0f, 1f),
                 })
        {
            float ndl = MathF.Max(0f, Vector3.Dot(normal, Vector3.Normalize(-Direction)));
            Assert.Equal(0f, ndl, 1e-6);
        }
    }

    [Fact]
    public void Section_5_2_linear_factors_are_0_7585_0_2966_and_0_1812()
    {
        const float ndl = 0.5773503f;
        Assert.Equal(0.7585f, SceneColor.AmbientLinear + ndl, 1e-3);
        Assert.Equal(0.2966f, SceneColor.AmbientLinear + (ndl * SceneColor.ShadowFactor), 1e-3);
        Assert.Equal(0.1812f, SceneColor.AmbientLinear, 1e-3);
    }

    [Theory]
    // NES colour, sRGB input, lit, in shadow, ambient only (SPEC-06 §5.2, ±3 per channel).
    [InlineData(0x30, 252, 248, 252, 223, 219, 223, 146, 144, 146, 117, 115, 117)]
    [InlineData(0x22, 104, 136, 252, 91, 120, 223, 57, 76, 146, 44, 60, 117)]
    [InlineData(0x17, 228, 96, 24, 202, 84, 20, 132, 52, 9, 105, 40, 5)]
    [InlineData(0x29, 188, 248, 24, 166, 219, 20, 108, 144, 9, 85, 115, 5)]
    public void Section_5_2_shading_vectors(
        int nesColor,
        int inR, int inG, int inB,
        int litR, int litG, int litB,
        int shadowR, int shadowG, int shadowB,
        int ambientR, int ambientG, int ambientB)
    {
        Rgb24 albedo = NesPalette.Color(nesColor);
        Assert.Equal(new Rgb24((byte)inR, (byte)inG, (byte)inB), albedo);

        AssertShade(albedo, new Vector3(0f, 0f, -1f), 1f, litR, litG, litB);
        AssertShade(albedo, new Vector3(0f, 0f, -1f), SceneColor.ShadowFactor, shadowR, shadowG, shadowB);
        AssertShade(albedo, new Vector3(0f, 0f, 1f), 1f, ambientR, ambientG, ambientB);
    }

    [Fact]
    public void Section_5_2_pure_white_vector()
    {
        Rgb24 albedo = new(255, 255, 255);
        AssertShade(albedo, new Vector3(0f, 0f, -1f), 1f, 226, 226, 226);
        AssertShade(albedo, new Vector3(0f, 0f, -1f), SceneColor.ShadowFactor, 148, 148, 148);
        AssertShade(albedo, new Vector3(0f, 0f, 1f), 1f, 118, 118, 118);
    }

    [Fact]
    public void REQ_SCN_027_a_white_face_lit_only_by_the_ambient_shows_sRGB_117()
    {
        Rgb24 shaded = SceneColor.Shade(
            NesPalette.Color(0x30),
            new Vector3(0f, 0f, 1f),
            Direction,
            White,
            1f,
            1f);

        Assert.InRange(shaded.R, 115, 119);
        Assert.InRange(shaded.G, 113, 117);
        Assert.InRange(shaded.B, 115, 119);
    }

    /// <summary>
    /// Q-072, lot FIX-4: what the original's own 1080 p capture shows on the unlit side faces of
    /// the extruded HUD glyphs of Super Mario Bros. 3, measured in
    /// <c>verify/reference/smb3-map-original-1080p.png</c>. The white glyphs are NES 0x30 over
    /// a cyan panel; the faces the light does not reach come out at sRGB 117 in the original and
    /// 116 here, so the ambient term of REQ-SCN-027 is the one the original uses and an
    /// "ambient intensity of 0,3" — whichever way it is read, 0,054 or 0,3 in linear space —
    /// is not.
    /// </summary>
    [Theory]
    // linear ambient, expected sRGB of a white face lit by the ambient alone
    [InlineData(0.1812f, 117)]   // REQ-SCN-027, what both the original and Zaxnes show
    [InlineData(0.0544f, 65)]    // 0,3 read as a factor on the ambient colour: far too dark
    [InlineData(0.3f, 147)]      // 0,3 read as the linear ambient itself: too light
    public void Q_072_only_the_ambient_of_REQ_SCN_027_matches_the_reference_capture(
        float ambientLinear,
        int expected)
    {
        // The shading of REQ-SCN-028 for a face with N·L = 0, spelled out so that the three
        // candidate ambients can be compared without changing the renderer's constant.
        float albedo = SceneColor.ToLinear(NesPalette.Color(0x30).R / 255f);
        int shown = (int)MathF.Round(SceneColor.ToSrgb(albedo * ambientLinear) * 255f);
        Assert.Equal(expected, shown);

        // And the one the renderer actually uses is the first of the three.
        if (expected == 117)
        {
            Assert.Equal(ambientLinear, SceneColor.AmbientLinear, 1e-4);
        }
    }

    [Fact]
    public void REQ_SCN_031_a_white_front_face_goes_from_223_to_146_in_shadow()
    {
        // A face facing the viewer: N = (0, 0, −1), so N·L = 0.577 (REQ-SCN-031 criterion).
        Vector3 normal = new(0f, 0f, -1f);
        Rgb24 lit = SceneColor.Shade(NesPalette.Color(0x30), normal, Direction, White, 1f, 1f);
        Rgb24 shadowed = SceneColor.Shade(
            NesPalette.Color(0x30),
            normal,
            Direction,
            White,
            1f,
            SceneColor.ShadowFactor);

        Assert.InRange(lit.R, 220, 226);
        Assert.InRange(shadowed.R, 143, 149);
    }

    [Fact]
    public void DEC_06_Q2_the_intensity_goes_through_the_same_sRGB_curve_as_the_colours()
    {
        // Intensity 1.5 in sRGB becomes about 1.5^2.4 in linear space, not 1.5.
        float linear = SceneColor.ToLinear(1.5f);
        Assert.Equal(MathF.Pow((1.5f + 0.055f) / 1.055f, 2.4f), linear, 1e-5);
        Assert.Equal(1f, SceneColor.ToLinear(1f), 1e-6);
        Assert.Equal(0.2140411f, SceneColor.ToLinear(0.5f), 1e-5);
    }

    [Fact]
    public void REQ_SCN_026_the_intensity_setting_is_clamped_to_its_bounds()
    {
        RenderSettings settings = new();

        settings.LightIntensity = 9f;
        Assert.Equal(1.5f, settings.LightIntensity);
        settings.LightIntensity = 0f;
        Assert.Equal(0.5f, settings.LightIntensity);
    }

    [Fact]
    public void REQ_SCN_028_the_result_is_saturated_without_tone_mapping()
    {
        Rgb24 shaded = SceneColor.Shade(
            new Rgb24(255, 255, 255),
            Vector3.Normalize(-Direction),
            Direction,
            White,
            1.5f,
            1f);

        Assert.Equal(255, shaded.R);
    }

    [Fact]
    public void Section_2_the_transfer_functions_are_inverses_of_each_other()
    {
        for (int step = 0; step <= 255; step++)
        {
            float value = step / 255f;
            Assert.Equal(value, SceneColor.ToSrgb(SceneColor.ToLinear(value)), 1e-5);
        }
    }

    private static void AssertShade(Rgb24 albedo, Vector3 normal, float shadow, int r, int g, int b)
    {
        Rgb24 shaded = SceneColor.Shade(albedo, normal, Direction, White, 1f, shadow);
        Assert.InRange(shaded.R, r - 3, r + 3);
        Assert.InRange(shaded.G, g - 3, g + 3);
        Assert.InRange(shaded.B, b - 3, b + 3);
    }
}
