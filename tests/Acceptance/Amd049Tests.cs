using System.Numerics;
using Zax.Engine3D;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-049 — REQ-SCN-075, the lighting of the backdrop: the Standard reflection model of a
/// non-metallic, almost matt surface replaces, for the backdrop and for it alone, the diffuse
/// term REQ-SCN-048 used to inherit from REQ-SCN-028. The two automatic vectors of the
/// amendment, the sRGB form of the second one, the degenerate case <c>L + V = 0</c> and the
/// consequence the amendment writes into the acceptance criterion of REQ-SCN-048 (§3 point 2).
/// </summary>
public sealed class Amd049Tests
{
    /// <summary>The configuration of both automatic vectors: the front face of the box.</summary>
    private static Vector3 Normal => new(0f, 0f, -1f);

    /// <summary>The eye on the axis, so <c>V = (0, 0, −1)</c>.</summary>
    private static Vector3 View => new(0f, 0f, -1f);

    /// <summary>The light of both automatic vectors, which comes from the camera's side.</summary>
    private static Vector3 FrontLight => new(0f, 0f, 1f);

    /// <summary>The default light direction of REQ-SCN-025.</summary>
    private static Vector3 DefaultLight => new(1f, -1f, 1f);

    private static Rgb24 White => new(255, 255, 255);

    /// <summary>White light of intensity 1, which is <c>E = (1, 1, 1)</c> in linear space.</summary>
    private static Vector3 UnitLight => Vector3.One;

    // ------------------------------------------------------- §2, the two automatic vectors

    /// <summary>
    /// REQ-SCN-075, first criterion (linear, ±0.001): <c>N = V = (0, 0, −1)</c>,
    /// <c>Light.Direction = (0, 0, 1)</c>, white light of intensity 1, out of shadow. All four
    /// products are 1, so the colour is <c>c · 0.96 · 1.1812 + 0.01041</c> per channel: 0.2372
    /// for <c>c = 0.2</c>.
    /// </summary>
    [Fact] // REQ-SCN-075
    public void REQ_SCN_075_the_first_vector_is_0_2372_for_an_albedo_of_0_2()
    {
        Vector3 lit = Linear(0.2f, shadow: 1f);

        Assert.Equal(0.2372f, lit.X, 1e-3);
        Assert.Equal(0.2372f, lit.Y, 1e-3);
        Assert.Equal(0.2372f, lit.Z, 1e-3);

        // The two terms of the sum, read separately: the additive specular reflection is the
        // value a zero albedo keeps (Spec · F = 0.2603 × 0.04), and the slope in c is 0.96 ×
        // (Amb + 1) = 0.96 × 1.1812.
        Assert.Equal(0.01041f, Linear(0f, shadow: 1f).X, 1e-3);
        Assert.Equal(1.133952f, (Linear(0.4f, shadow: 1f).X - Linear(0.2f, shadow: 1f).X) / 0.2f, 1e-3);
    }

    /// <summary>
    /// REQ-SCN-075, second criterion (linear, ±0.001): the same configuration in shadow
    /// (<c>s = 0.2</c>) gives <c>c · 0.96 · (0.1812 + 0.2) + 0.2 · 0.01041</c>, that is 0.0753
    /// for <c>c = 0.2</c>. The shadow factor of REQ-SCN-031 multiplies the direct terms —
    /// diffuse and specular alike — and leaves the ambient alone.
    /// </summary>
    [Fact] // REQ-SCN-075, REQ-SCN-031
    public void REQ_SCN_075_the_second_vector_is_0_0753_in_shadow()
    {
        Vector3 shadowed = Linear(0.2f, SceneColor.ShadowFactor);

        Assert.Equal(0.0753f, shadowed.X, 1e-3);
        Assert.Equal(0.0753f, shadowed.Y, 1e-3);
        Assert.Equal(0.0753f, shadowed.Z, 1e-3);

        // The specular part is reduced in the same ratio s as the direct diffuse part: there is
        // no environment reflection to keep it alive in the shade.
        Assert.Equal(
            SceneColor.ShadowFactor * 0.01041f,
            Linear(0f, SceneColor.ShadowFactor).X,
            1e-3);

        // And the ambient is not touched: c · 0.96 · 0.1812 survives a zero direct light.
        Assert.Equal(
            0.2f * 0.96f * SceneColor.AmbientLinear,
            Linear(0.2f, shadow: 0f).X,
            1e-3);
    }

    /// <summary>
    /// REQ-SCN-075, third criterion (sRGB 8 bits, ±2): a <b>zero</b> albedo is no longer
    /// rendered black — only the specular reflection is left, <c>s · E · Spec · F</c> — and in
    /// the configuration of the first two criteria, where <c>Light.Direction = (0, 0, 1)</c> and
    /// every product is 1, it reads sRGB (26, 26, 26) out of shadow and (7, 7, 7) in shadow:
    /// <c>Spec · F = 0.2603 × 0.04 = 0.01041</c>, which encodes to 26.13, and a fifth of that,
    /// 0.00208, which is still on the linear leg of the sRGB curve and encodes to 6.86.
    /// </summary>
    /// <remarks>
    /// Q-083: the amendment publishes (21, 21, 21) and (5, 5, 5) for "the same configuration",
    /// but those two figures only hold under the <b>default</b> light (1, −1, 1) — the
    /// configuration of its own §3 point 2, locked by
    /// <see cref="REQ_SCN_048_a_black_backdrop_reads_21_lit_and_5_shadowed"/>. Both readings are
    /// pinned here, so whichever way the question is answered the model does not move.
    /// </remarks>
    [Fact] // REQ-SCN-075, Q-083
    public void REQ_SCN_075_a_zero_albedo_reads_sRGB_26_and_7_under_the_front_light()
    {
        Rgb24 lit = Srgb(default, FrontLight, shadow: 1f);
        Rgb24 shadowed = Srgb(default, FrontLight, SceneColor.ShadowFactor);

        Assert.Equal(0.01041f, Linear(0f, shadow: 1f).X, 1e-4);
        Assert.InRange(lit.R, 24, 28);
        Assert.InRange(lit.G, 24, 28);
        Assert.InRange(lit.B, 24, 28);
        Assert.InRange(shadowed.R, 5, 9);
        Assert.InRange(shadowed.G, 5, 9);
        Assert.InRange(shadowed.B, 5, 9);

        // The diffuse model of REQ-SCN-028, which the backdrop no longer follows, does render a
        // zero albedo black: this is the whole observable change of the amendment.
        Assert.Equal(
            new Rgb24(0, 0, 0),
            SceneColor.Shade(default, Normal, FrontLight, White, 1f, 1f));
    }

    /// <summary>
    /// REQ-SCN-075: <c>H = N</c> when <c>L + V = 0</c>. The configuration only exists on a face
    /// seen from behind or exactly edge-on, and the model must stay finite there. With
    /// <c>N = (0, 0, −1)</c>, <c>V = (0, 0, 1)</c> and <c>L = (0, 0, −1)</c> every product is
    /// again 1, so the colour is the one of the first vector.
    /// </summary>
    [Fact] // REQ-SCN-075
    public void REQ_SCN_075_H_is_N_when_L_plus_V_vanishes()
    {
        Vector3 degenerate = SceneColor.ShadeBackdropLinear(
            new Vector3(0.2f),
            Normal,
            new Vector3(0f, 0f, 1f),
            FrontLight,
            UnitLight,
            1f);

        Assert.True(float.IsFinite(degenerate.X), $"L + V = 0 produced {degenerate}.");
        Assert.Equal(0.2372f, degenerate.X, 1e-3);
    }

    /// <summary>
    /// REQ-SCN-075: the result is saturated to 1 and encoded in sRGB as REQ-SCN-028 does,
    /// without any tone mapping, and the alpha of <c>Backdrop.Color</c> plays no part — the box
    /// is opaque (REQ-SCN-058).
    /// </summary>
    [Fact] // REQ-SCN-075, REQ-SCN-028, REQ-SCN-058
    public void REQ_SCN_075_the_result_is_saturated_and_encoded_in_sRGB()
    {
        // A white albedo under the intensity 1.5 of REQ-SCN-026 goes past 1 in linear space.
        Assert.True(
            SceneColor.ShadeBackdropLinear(
                Vector3.One, Normal, View, FrontLight, SceneColor.ToLinear(1.5f) * Vector3.One, 1f).X > 1f);

        Assert.Equal(255, Srgb(White, FrontLight, shadow: 1f, intensity: 1.5f).R);

        // R2-review-AMD-049 point 49-6: the measurable form of the two automatic vectors. The
        // linear 0.2372 and 0.0753 of c = 0.2 are sRGB 134 and 78 once encoded.
        Assert.Equal(134, SceneColor.ToSrgbBytes(Linear(0.2f, shadow: 1f)).R);
        Assert.Equal(78, SceneColor.ToSrgbBytes(Linear(0.2f, SceneColor.ShadowFactor)).R);
    }

    // ------------------------------------------------- §3 point 2, the REQ-SCN-048 criterion

    /// <summary>
    /// AMD-049 §3 point 2: the acceptance criterion of REQ-SCN-048 no longer shows "a black
    /// panel" but "an almost black panel". With the <b>default</b> light (1, −1, 1), white, of
    /// intensity 1, a front face seen head-on has <c>N·L = 0.577</c> and keeps
    /// <c>Spec · F = 0.00754</c> of specular reflection, which is sRGB 21 out of shadow and 5 in
    /// shadow for <c>Backdrop.Color = Color(0, 0, 0)</c>.
    /// </summary>
    /// <remarks>Q-083: the other reading of the amendment's sRGB criterion.</remarks>
    [Fact] // REQ-SCN-048, REQ-SCN-075, Q-083
    public void REQ_SCN_048_a_black_backdrop_reads_21_lit_and_5_shadowed()
    {
        Vector3 litLinear = SceneColor.ShadeBackdropLinear(
            Vector3.Zero, Normal, View, DefaultLight, UnitLight, 1f);

        Assert.Equal(0.00754f, litLinear.X, 1e-4);
        Assert.Equal(21, Srgb(default, DefaultLight, shadow: 1f).R);
        Assert.Equal(5, Srgb(default, DefaultLight, SceneColor.ShadowFactor).R);

        // The shadows of the front layers can therefore be seen on a default backdrop, which a
        // strictly diffuse model made impossible: black stayed black lit or not.
        Assert.Equal(
            SceneColor.Shade(default, Normal, DefaultLight, White, 1f, 1f),
            SceneColor.Shade(default, Normal, DefaultLight, White, 1f, SceneColor.ShadowFactor));
    }

    /// <summary>
    /// AMD-049 §1, as R2-review-AMD-049 point 49-7 restates it: compared with a diffuse
    /// lighting, the diffuse part is multiplied by 0.96 — the ambient included — and modulated
    /// by a back-scattering factor worth 1 near normal incidence; a weak specular reflection of
    /// the <b>light's</b> colour is added, independent of the albedo. So a saturated blue
    /// backdrop under a pink light picks up a red the albedo does not carry.
    /// </summary>
    [Fact] // REQ-SCN-075
    public void REQ_SCN_075_the_specular_part_carries_the_colour_of_the_light_not_the_albedo()
    {
        // NES 0x12, a blue with no red at all, under the pink light of a profile script.
        Rgb24 blue = NesPalette.Color(0x12);
        Rgb24 pink = new(255, 204, 204);

        Assert.Equal(0, blue.R);
        Rgb24 diffuse = SceneColor.Shade(blue, Normal, DefaultLight, pink, 1f, 1f);
        Rgb24 standard = SceneColor.ShadeBackdrop(blue, Normal, View, DefaultLight, pink, 1f, 1f);

        Assert.Equal(0, diffuse.R);
        Assert.True(standard.R > 15, $"the light's red should show through, got {standard.R}.");

        // Near normal incidence the diffuse part is 4 % darker, so the channels the albedo does
        // carry lose a little instead of gaining.
        Assert.True(standard.B < diffuse.B, $"blue went from {diffuse.B} to {standard.B}.");
    }

    // ------------------------------------------------------------------------ helpers

    /// <summary>The linear colour of the automatic configuration for a linear albedo <c>c</c>.</summary>
    private static Vector3 Linear(float albedo, float shadow) => SceneColor.ShadeBackdropLinear(
        new Vector3(albedo), Normal, View, FrontLight, UnitLight, shadow);

    /// <summary>The sRGB colour of the automatic configuration, light colour and intensity included.</summary>
    private static Rgb24 Srgb(Rgb24 albedo, Vector3 light, float shadow, float intensity = 1f)
        => SceneColor.ShadeBackdrop(albedo, Normal, View, light, White, intensity, shadow);
}

/// <summary>
/// The one AMD-049 case that needs a GL 3.3 core context: the backdrop's front face, rendered in
/// the configuration of the first automatic vector of REQ-SCN-075, must come out of the shader
/// at the colour the platform-free model computes.
/// </summary>
/// <remarks>Its own collection, so that two contexts are never created at once.</remarks>
[Collection("opengl-acceptance")]
public sealed class Amd049CaptureTests
{
    /// <summary>
    /// REQ-SCN-075, first criterion, measured on a capture: the local camera of REQ-SCN-009 with
    /// zero angles looks along <c>(0, 0, 1)</c>, so the centre pixel of the picture sees the
    /// front face of the box (<c>N = (0, 0, −1)</c>) with <c>V = (0, 0, −1)</c> exactly. With
    /// <c>Light.Direction = (0, 0, 1)</c>, a white light of intensity 1 and no caster, the pixel
    /// must carry the colour of
    /// <see cref="SceneColor.ShadeBackdrop"/>, and not the diffuse colour of REQ-SCN-028.
    /// </summary>
    [GlFact] // REQ-SCN-075, REQ-SCN-048
    public void REQ_SCN_075_the_front_face_of_the_backdrop_matches_the_model()
    {
        // sRGB 124, whose linear value is 0.2016 — as close to the c = 0.2 of the amendment as
        // an 8-bit Backdrop.Color can get.
        Rgb24 albedo = new(124, 124, 124);
        Vector3 normal = new(0f, 0f, -1f);
        Vector3 view = new(0f, 0f, -1f);
        Vector3 light = new(0f, 0f, 1f);
        Rgb24 white = new(255, 255, 255);

        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = new();

        // REQ-SCN-009: the local camera looks exactly along q·(0, 0, 1), which with zero angles
        // puts the eye on the axis of the front face.
        scene.Camera.LocalCamera = true;
        scene.Camera.Setpoints(CameraMode.Tpv).Angles = Vector3.Zero;
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Light.Direction = light;
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = albedo;

        Shot shot = target.Render(renderer, scene);

        // The eye looks along +z from the axis of the face, so V is exactly (0, 0, −1) there.
        Vector3 forward = renderer.LastView.Forward;
        Assert.Equal(0f, forward.X, 1e-5);
        Assert.Equal(0f, forward.Y, 1e-5);
        Assert.Equal(1f, forward.Z, 1e-5);

        (int r, int g, int b) = shot.At(shot.Width / 2, shot.Height / 2);
        Rgb24 expected = SceneColor.ShadeBackdrop(albedo, normal, view, light, white, 1f, 1f);

        Assert.InRange(r, expected.R - 2, expected.R + 2);
        Assert.InRange(g, expected.G - 2, expected.G + 2);
        Assert.InRange(b, expected.B - 2, expected.B + 2);

        // At N·L = 1 the two models nearly coincide on a mid grey — the 4 % the diffuse part
        // loses is almost exactly what the specular part adds back — so the shader is told apart
        // from REQ-SCN-028 on a zero albedo, which the diffuse model renders black and this one
        // renders sRGB 26 (Q-083).
        scene.Backdrop.Color = default;
        (int blackR, int blackG, int blackB) = target
            .Render(renderer, scene)
            .At(shot.Width / 2, shot.Height / 2);

        Assert.Equal(0, SceneColor.Shade(default, normal, light, white, 1f, 1f).R);
        Rgb24 standard = SceneColor.ShadeBackdrop(default, normal, view, light, white, 1f, 1f);
        Assert.Equal(26, standard.R);
        Assert.InRange(blackR, standard.R - 2, standard.R + 2);
        Assert.InRange(blackG, standard.G - 2, standard.G + 2);
        Assert.InRange(blackB, standard.B - 2, standard.B + 2);
    }
}
