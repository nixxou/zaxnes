using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.Render.Tests;

/// <summary>
/// The free assets of AMD-024: the procedural skyboxes of REQ-PUB-003, which must render with
/// <b>no asset source at all</b>. REQ-PUB-004 asked for procedural TV cabinets as well; AMD-031
/// removed the decorative TV box from the product, so there is nothing left to cover there.
/// </summary>
/// <remarks>
/// Every renderer here is built with the default <c>assets</c>, that is null: nothing can read a
/// file, so what the captures show is what the code produces.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class ProceduralAssetsTests
{
    /// <summary>NES 0x22, the sky blue of the reference views.</summary>
    private static readonly Rgb24 Blue = NesPalette.Color(0x22);

    /// <summary>NES 0x0F, black: the background that lights the stars (REQ-SCN-050).</summary>
    private static readonly Rgb24 Black = NesPalette.Color(0x0F);

    [GlFact] // REQ-PUB-003, REQ-SCN-050
    public void REQ_PUB_003_the_gradient_sky_renders_with_no_image_file()
    {
        using GlFixture gl = GlFixture.Create(96, 96);
        using SceneRenderer renderer = new(gl.Gl);

        // The gradient formula of REQ-SCN-050 is untouched by the change of field source: the
        // horizon is M, the zenith 1.4 M and the nadir 0.6 M, and a bright M shows no star.
        Assert.Equal(Srgb(Blue), Centre(gl, renderer, rx: 0f));
        Assert.Equal(Scaled(Blue, 1.4f), Centre(gl, renderer, rx: -90f));
        Assert.Equal(Scaled(Blue, 0.6f), Centre(gl, renderer, rx: 90f));
    }

    [GlFact] // REQ-PUB-003, REQ-SCN-050
    public void REQ_PUB_003_the_procedural_stars_only_appear_over_a_dark_background()
    {
        using GlFixture gl = GlFixture.Create(192, 192);
        using SceneRenderer renderer = new(gl.Gl);

        // Looking at the zenith, where the |0.2 y| factor is at its largest.
        Capture dark = Render(gl, renderer, SkyboxKind.Gradient, Black, rx: -90f);
        int lit = dark.CountOther(0x000000);
        Assert.True(lit > 0, "a dark sky should carry stars");
        Assert.True(lit < dark.Width * dark.Height / 2, "the star field should stay sparse");

        // A bright background turns them off entirely (grey level above 0.4): the image is then
        // the smooth gradient alone, which the twinkle of REQ-SCN-050 cannot touch.
        Assert.False(SkyGradient.StarsVisible(Blue));
        Capture bright = Render(gl, renderer, SkyboxKind.Gradient, Blue, rx: -90f);
        Capture brightLater = Render(gl, renderer, SkyboxKind.Gradient, Blue, rx: -90f, realTime: 20f);
        Assert.Equal(bright.Pixels, brightLater.Pixels);
    }

    [GlFact] // REQ-PUB-003, REQ-SCN-059
    public void REQ_PUB_003_the_stars_twinkle_with_real_time()
    {
        using GlFixture gl = GlFixture.Create(128, 128);
        using SceneRenderer renderer = new(gl.Gl);

        Capture first = Render(gl, renderer, SkyboxKind.Gradient, Black, rx: -90f);
        Capture later = Render(gl, renderer, SkyboxKind.Gradient, Black, rx: -90f, realTime: 20f);

        // 20 s at 3°/s turns the twinkle mask by 60°, which changes the brightness of the stars
        // without moving them.
        Assert.NotEqual(first.Pixels, later.Pixels);
    }

    [GlFact] // REQ-PUB-003, REQ-SCN-051
    public void REQ_PUB_003_the_fantasy_sky_has_structure_and_follows_the_palette()
    {
        using GlFixture gl = GlFixture.Create(128, 128);
        using SceneRenderer renderer = new(gl.Gl);

        Capture nebula = Render(gl, renderer, SkyboxKind.Fantasy, Blue, rx: 0f, palette: 0x21);
        Assert.True(nebula.DistinctColors().Count > 8, "the nebula should not be a flat colour");

        // REQ-SCN-052: the sky follows the colour of the displayed frame, so another background
        // gives another image.
        Capture other = Render(gl, renderer, SkyboxKind.Fantasy, NesPalette.Color(0x16), rx: 0f, palette: 0x21);
        Assert.NotEqual(nebula.Pixels, other.Pixels);
    }

    [GlFact] // REQ-PUB-003 (Q-064)
    public void REQ_PUB_003_the_sun_the_moon_and_the_clouds_are_off_by_default_and_change_the_image()
    {
        using GlFixture gl = GlFixture.Create(128, 128);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = SkyScene(Blue, rx: 0f);

        // REQ-SCN-050: by default the sun, the moon and the clouds are disabled, so the sky is
        // the pure gradient — one colour per row of the capture.
        Assert.False(scene.Settings.SkyBodies.AnyVisible);
        Capture plain = gl.Render(renderer, scene);

        // The sun, placed straight ahead, is a bright disc the gradient cannot produce.
        scene.Settings.SkyBodies.SunDirection = new Vector3(0f, 0f, 1f);
        scene.Settings.SkyBodies.SunRadiusDegrees = 8f;
        scene.Settings.SkyBodies.SunGain = 1f;
        Capture sun = gl.Render(renderer, scene);
        Assert.NotEqual(plain.Pixels, sun.Pixels);
        Assert.True(
            sun.Luma(sun.Width / 2, sun.Height / 2) > plain.Luma(plain.Width / 2, plain.Height / 2) + 0.2,
            "the sun disc should brighten the centre of the view");

        // The moon, on the other side, is another disc.
        scene.Settings.SkyBodies.Reset();
        scene.Settings.SkyBodies.MoonDirection = new Vector3(0f, 0f, 1f);
        scene.Settings.SkyBodies.MoonRadiusDegrees = 8f;
        scene.Settings.SkyBodies.MoonGain = 1f;
        Capture moon = gl.Render(renderer, scene);
        Assert.NotEqual(plain.Pixels, moon.Pixels);

        // The clouds are a band around the horizon, which is where this view looks.
        scene.Settings.SkyBodies.Reset();
        scene.Settings.SkyBodies.CloudGain = 1f;
        scene.Settings.SkyBodies.CloudCoverage = 0.9f;
        Capture clouds = gl.Render(renderer, scene);
        Assert.NotEqual(plain.Pixels, clouds.Pixels);

        // Back to the defaults, the image is the one of the start: nothing is stateful.
        scene.Settings.SkyBodies.Reset();
        Assert.Equal(plain.Pixels, gl.Render(renderer, scene).Pixels);
    }

    private static (int R, int G, int B) Centre(GlFixture gl, SceneRenderer renderer, float rx)
    {
        Capture capture = Render(gl, renderer, SkyboxKind.Gradient, Blue, rx);
        return capture.At(capture.Width / 2, capture.Height / 2);
    }

    private static Capture Render(
        GlFixture gl,
        SceneRenderer renderer,
        SkyboxKind kind,
        Rgb24 background,
        float rx,
        byte? palette = null,
        float realTime = 0f)
    {
        SceneState scene = SkyScene(background, rx, palette);
        scene.Settings.Skybox = kind;
        scene.RealTime = realTime;
        return gl.Render(renderer, scene);
    }

    /// <summary>
    /// An empty scene looking along <paramref name="rx"/> with the Gradient sky. FPV is used so
    /// that the camera really points along <c>q·(0,0,1)</c> (REQ-SCN-010).
    /// </summary>
    private static SceneState SkyScene(Rgb24 background, float rx, byte? palette = null)
    {
        SceneState scene = new()
        {
            GameRunning = true,
            NesBackgroundColor = background,
        };

        if (palette is { } entry)
        {
            scene.DrawList = new SceneFixture()
                .Palette(1, entry)
                .Palette(2, entry)
                .Palette(3, entry)
                .Build()
                .DrawList;
        }

        scene.Settings.Skybox = SkyboxKind.Gradient;
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Angles = new Vector3(rx, 0f, 0f);
        scene.Camera.Current.Distance = 0f;
        scene.Camera.SnapToSetpoints();
        return scene;
    }

    private static (int R, int G, int B) Srgb(Rgb24 color) => (color.R, color.G, color.B);

    private static (int R, int G, int B) Scaled(Rgb24 color, float factor)
    {
        static byte Channel(byte value, float factor)
        {
            float srgb = value / 255f * factor;
            float linear = SceneColor.ToLinear(srgb);
            return (byte)Math.Clamp(
                MathF.Round(SceneColor.ToSrgb(Math.Clamp(linear, 0f, 1f)) * 255f), 0f, 255f);
        }

        return (Channel(color.R, factor), Channel(color.G, factor), Channel(color.B, factor));
    }
}
