using System.Numerics;
using Zax.Engine3D;
using Zax.Render;
using Zax.Render.Assets;
using Zax.Profiles;

namespace Zax.Render.Tests;

/// <summary>
/// The two skyboxes of SPEC-06 §3.14: Gradient (REQ-SCN-050) and Fantasy (REQ-SCN-051).
/// </summary>
/// <remarks>
/// The acceptance criterion of REQ-SCN-051 asks for uniform test cubemaps injected in place of
/// the ones of <c>assets-neutral/</c>; <see cref="UniformAssets"/> is that injection, which the
/// renderer accepts because it takes its assets from the host (REQ-SCN-057).
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class SkyboxTests
{
    /// <summary>NES 0x22, the sky blue of the reference views.</summary>
    private static readonly Rgb24 Blue = NesPalette.Color(0x22);

    [Fact]
    public void REQ_SCN_050_the_stars_only_show_on_a_dark_background()
    {
        // Grey level 0.299 R + 0.587 G + 0.114 B on the sRGB values, threshold 0.4.
        Assert.False(SkyGradient.StarsVisible(Blue));
        Assert.True(SkyGradient.StarsVisible(NesPalette.Color(0x0F)));
        Assert.Equal(0.4f, SkyGradient.StarGreyThreshold, 5);
        Assert.InRange(SkyGradient.Grey(Blue), 0.4f, 1f);
    }

    [GlFact]
    public void REQ_SCN_050_the_horizon_is_the_background_colour_and_the_poles_scale_it()
    {
        using GlFixture gl = GlFixture.Create(96, 96);
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.Black);

        // Horizon: a direction with y = 0 shows M itself.
        Assert.Equal(Srgb(Blue), Centre(gl, renderer, rx: 0f));

        // Zenith: srgb(lin(1.4 M)) component by component, saturated at display.
        Assert.Equal(Scaled(Blue, 1.4f), Centre(gl, renderer, rx: -90f));

        // Nadir: srgb(lin(0.6 M)).
        Assert.Equal(Scaled(Blue, 0.6f), Centre(gl, renderer, rx: 90f));
    }

    [GlFact]
    public void REQ_SCN_050_a_dark_background_lights_the_stars_of_the_cubemap()
    {
        using GlFixture gl = GlFixture.Create(96, 96);

        // A white Stars_LP_01 makes s = 1 everywhere, so k = min(|0.2 y|, 1) and the zenith,
        // where y = 1, mixes 20 % of the slightly grey white E.
        using SceneRenderer stars = new(gl.Gl, UniformAssets.WhiteStars);
        (int R, int G, int B) zenith = Centre(gl, stars, rx: -90f, background: NesPalette.Color(0x0F));

        // AMD-019 (Q-045): the "×2" factor is the one the sRGB curve of SPEC-06 §2 gives.
        float e = SceneColor.ToLinear(0.5f) * SkyGradient.SrgbDouble;
        byte expected = (byte)Math.Round(SceneColor.ToSrgb(0.2f * e) * 255f);
        Assert.Equal(expected, zenith.R, tolerance: 2);
        Assert.Equal(expected, zenith.G, tolerance: 2);
        Assert.Equal(expected, zenith.B, tolerance: 2);

        // The same sky without the cubemap stays pure black at the zenith.
        using SceneRenderer none = new(gl.Gl, UniformAssets.Black);
        Assert.Equal((0, 0, 0), Centre(gl, none, rx: -90f, background: NesPalette.Color(0x0F)));
    }

    [GlFact]
    public void REQ_SCN_051_a_all_cubemaps_null_shows_f0_in_every_direction()
    {
        using GlFixture gl = GlFixture.Create(64, 64);
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.Black);

        Capture capture = Render(gl, renderer, SkyboxKind.Fantasy, Blue, rx: 0f);
        Assert.Equal([Blue.Packed], capture.DistinctColors());
    }

    [GlFact]
    public void REQ_SCN_051_b_a_green_diffusion_of_one_half_multiplies_f0_by_2_2975()
    {
        using GlFixture gl = GlFixture.Create(64, 64);
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.HalfGreenDiffusion);

        // F0 = grey sRGB 0.2: only the ambient halo contributes, with the weight 1.
        Rgb24 grey = new(51, 51, 51);
        Capture capture = Render(gl, renderer, SkyboxKind.Fantasy, grey, rx: 0f);

        float expected = SceneColor.ToSrgb(2.2975f * SceneColor.ToLinear(51f / 255f)) * 255f;
        (int r, int g, int b) = capture.At(32, 32);
        Assert.Equal((int)Math.Round(expected), r, tolerance: 2);
        Assert.Equal((int)Math.Round(expected), g, tolerance: 2);
        Assert.Equal((int)Math.Round(expected), b, tolerance: 2);
    }

    [GlFact]
    public void REQ_SCN_051_c_a_light_background_takes_nothing_from_the_starfield()
    {
        using GlFixture gl = GlFixture.Create(64, 64);

        // A white Starfield would otherwise push the whole sky towards the star tint F1 / 2;
        // with gamma > 0.4 that tint is transparent, so the sky stays F0.
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.WhiteStarfield);
        Rgb24 light = new(220, 220, 220);
        Capture capture = Render(gl, renderer, SkyboxKind.Fantasy, light, rx: 0f, palette: 0x30);
        Assert.Equal([light.Packed], capture.DistinctColors());

        // The same starfield over a dark background does take the star tint.
        Capture dark = Render(
            gl, renderer, SkyboxKind.Fantasy, NesPalette.Color(0x0F), rx: 0f, palette: 0x30);
        Assert.DoesNotContain(NesPalette.Color(0x0F).Packed, dark.DistinctColors());
    }

    [GlFact]
    public void REQ_SCN_049_the_scene_is_drawn_in_front_of_the_skybox()
    {
        using GlFixture gl = GlFixture.Create(128, 128);
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.Black);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 10, 10, 8, 8)
            .Palette(1, 0x30);
        SceneState scene = fixture.Build();
        scene.Settings.Skybox = SkyboxKind.Gradient;
        scene.Settings.Shadows = false;

        Capture capture = gl.Render(renderer, scene);

        // The tiles are white and lit; the sky around them is the black background colour.
        Assert.True(capture.Count(0x000000) > 0);
        Assert.True(capture.CountOther(0x000000) > 0);
        Assert.True(renderer.Statistics.SkyboxPass);
    }

    [GlFact]
    public void REQ_SCN_049_default_and_color_draw_no_skybox_at_all()
    {
        using GlFixture gl = GlFixture.Create(32, 32);
        using SceneRenderer renderer = new(gl.Gl, UniformAssets.Black);

        SceneState scene = new() { GameRunning = true, NesBackgroundColor = Blue };
        scene.Camera.SnapToSetpoints();

        scene.Settings.Skybox = SkyboxKind.Default;
        gl.Render(renderer, scene);
        Assert.False(renderer.Statistics.SkyboxPass);

        scene.Settings.Skybox = SkyboxKind.Gradient;
        gl.Render(renderer, scene);
        Assert.True(renderer.Statistics.SkyboxPass);
    }

    private static (int R, int G, int B) Centre(
        GlFixture gl,
        SceneRenderer renderer,
        float rx,
        Rgb24? background = null)
    {
        Capture capture = Render(gl, renderer, SkyboxKind.Gradient, background ?? Blue, rx);
        return capture.At(capture.Width / 2, capture.Height / 2);
    }

    /// <summary>
    /// Renders an empty scene with the given skybox, looking along <paramref name="rx"/>. FPV is
    /// used so that the camera really points along <c>q·(0,0,1)</c> (REQ-SCN-010) instead of
    /// being turned back towards <c>P*</c>.
    /// </summary>
    private static Capture Render(
        GlFixture gl,
        SceneRenderer renderer,
        SkyboxKind kind,
        Rgb24 background,
        float rx,
        byte? palette = null)
    {
        SceneState scene = new()
        {
            GameRunning = true,
            NesBackgroundColor = background,
        };

        if (palette is { } entry)
        {
            // The Fantasy sky needs F1 to F3, which come from the frame palette of the draw
            // list; an empty frame is enough to carry them.
            scene.DrawList = new SceneFixture().Palette(1, entry).Palette(2, entry).Palette(3, entry).Build().DrawList;
        }

        scene.Settings.Skybox = kind;
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Angles = new Vector3(rx, 0f, 0f);
        scene.Camera.Current.Distance = 0f;
        scene.Camera.SnapToSetpoints();
        return gl.Render(renderer, scene);
    }

    private static (int R, int G, int B) Srgb(Rgb24 color) => (color.R, color.G, color.B);

    private static (int R, int G, int B) Scaled(Rgb24 color, float factor)
    {
        // REQ-SCN-050: the factor is applied to the sRGB value without saturating it, the
        // result is linearised and re-encoded, and only then saturated for display.
        static byte Channel(byte value, float factor)
        {
            float srgb = value / 255f * factor;
            float linear = SceneColor.ToLinear(srgb);
            return (byte)Math.Clamp(
                MathF.Round(SceneColor.ToSrgb(Math.Clamp(linear, 0f, 1f)) * 255f), 0f, 255f);
        }

        return (Channel(color.R, factor), Channel(color.G, factor), Channel(color.B, factor));
    }

    /// <summary>
    /// Cubemaps of one constant colour, the test injection of REQ-SCN-051. Every face of every
    /// cubemap is a single texel.
    /// </summary>
    private sealed class UniformAssets : ISceneAssets
    {
        private readonly Dictionary<string, ImageData> _cubemaps;

        private UniformAssets(Dictionary<string, ImageData> cubemaps) => _cubemaps = cubemaps;

        /// <summary>Everything black: the "no cubemap" case of both skyboxes.</summary>
        public static UniformAssets Black { get; } = new([]);

        /// <summary>A white <c>Stars_LP_01</c>, so that the star product is 1 everywhere.</summary>
        public static UniformAssets WhiteStars { get; } = new(new Dictionary<string, ImageData>
        {
            ["Stars_LP_01"] = ImageData.Single(255, 255, 255),
        });

        /// <summary><c>NebulaDiffusion</c> = (0, 0.5, 0), the vector (b) of REQ-SCN-051.</summary>
        public static UniformAssets HalfGreenDiffusion { get; } = new(new Dictionary<string, ImageData>
        {
            ["NebulaDiffusion"] = ImageData.Single(0, 128, 0),
        });

        /// <summary>A white <c>Starfield</c>, so that the star weight is its alpha alone.</summary>
        public static UniformAssets WhiteStarfield { get; } = new(new Dictionary<string, ImageData>
        {
            ["Starfield"] = ImageData.Single(255, 255, 255),
        });

        public ImageData? CubemapFace(string cubemap, CubeFace face)
            => _cubemaps.TryGetValue(cubemap, out ImageData? image)
                ? image
                : ImageData.Single(0, 0, 0);

        public ImageData? Texture(string name) => null;

        public TileMesh? Mesh(string name) => null;
    }
}
