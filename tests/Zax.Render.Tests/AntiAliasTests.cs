using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The full-screen anti-aliasing SPEC-09 REQ-PERS-017 allows and AMD-032 §2 implements as
/// multisampling: the option to sample-count mapping, the fallback to what the hardware grants,
/// and the effect on a capture.
/// </summary>
/// <remarks>
/// The mapping and the fallback are arithmetic and need no context; only the last two tests ask
/// for one. The shadow map is deliberately left out of all this (REQ-SCN-030): nothing here
/// touches it.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class AntiAliasTests
{
    /// <summary>AMD-032 §2: 0 off, 1 is four samples, 2 is eight.</summary>
    [Theory]
    [InlineData(0, 1)]
    [InlineData(1, 4)]
    [InlineData(2, 8)]
    public void The_option_maps_to_a_sample_count(int option, int expected)
        => Assert.Equal(expected, SceneConstants.AntiAliasSamples(option));

    /// <summary>A value no option carries leaves anti-aliasing off rather than guessing.</summary>
    [Theory]
    [InlineData(-1)]
    [InlineData(3)]
    [InlineData(64)]
    public void An_option_outside_the_domain_means_off(int option)
        => Assert.Equal(1, SceneConstants.AntiAliasSamples(option));

    /// <summary>
    /// AMD-032 §2: what is asked for is lowered to the highest power of two the implementation
    /// supports, never raised, and never below one sample.
    /// </summary>
    [Theory]
    // asked, supported, granted
    [InlineData(8, 8, 8)]
    [InlineData(8, 4, 4)]
    [InlineData(8, 2, 2)]
    [InlineData(8, 1, 1)]
    [InlineData(4, 8, 4)]
    [InlineData(1, 8, 1)]
    [InlineData(8, 6, 4)]
    [InlineData(8, 0, 1)]
    [InlineData(0, 8, 1)]
    public void The_sample_count_falls_back_to_what_is_supported(int asked, int supported, int granted)
        => Assert.Equal(granted, SceneConstants.ClampAntiAliasSamples(asked, supported));

    /// <summary>The setting never asks the renderer for more than AMD-032 §2 defines.</summary>
    [Fact]
    public void The_scene_setting_is_clamped_to_the_defined_range()
    {
        RenderSettings settings = new();
        Assert.Equal(1, settings.AntiAliasSamples);

        settings.AntiAliasSamples = 64;
        Assert.Equal(SceneConstants.MaxAntiAliasSamples, settings.AntiAliasSamples);

        settings.AntiAliasSamples = 0;
        Assert.Equal(1, settings.AntiAliasSamples);
    }

    /// <summary>
    /// AMD-032 §2: the renderer reports the sample count it was actually granted, and a plain
    /// desktop GL 3.3 driver grants the four and the eight samples the two levels ask for.
    /// </summary>
    [GlFact]
    public void The_renderer_reports_the_sample_count_it_was_granted()
    {
        using GlFixture gl = GlFixture.Create(320, 240);
        using SceneRenderer renderer = new(gl.Gl);

        foreach ((int option, int expected) in new[] { (0, 1), (1, 4), (2, 8) })
        {
            SceneState scene = Edges();
            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            gl.Render(renderer, scene);

            Assert.Equal(expected, renderer.Statistics.AntiAliasSamples);
        }
    }

    /// <summary>
    /// The effect the user asked for: a slanted silhouette drawn with no anti-aliasing has two
    /// colours along its edge, and multisampling fills it with intermediate ones. The count of
    /// distinct colours is the measure, as SPEC-06 §5 asks for pixel statistics rather than a
    /// reference image.
    /// </summary>
    /// <remarks>
    /// Applying the setting takes no restart: the three captures come from one renderer and one
    /// context, the sample count changing between two frames (AMD-032 §2).
    /// </remarks>
    [GlFact]
    public void Anti_aliasing_adds_intermediate_colours_along_the_edges()
    {
        using GlFixture gl = GlFixture.Create(320, 240);
        using SceneRenderer renderer = new(gl.Gl);

        int[] colors = new int[3];
        for (int option = 0; option < 3; option++)
        {
            SceneState scene = Edges();
            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            colors[option] = gl.Render(renderer, scene).DistinctColors().Count;
        }

        Assert.True(
            colors[1] > colors[0],
            $"MSAA 4x produced {colors[1]} colours, no more than the {colors[0]} of no anti-aliasing.");
        Assert.True(
            colors[2] >= colors[1],
            $"MSAA 8x produced {colors[2]} colours, fewer than the {colors[1]} of MSAA 4x.");
    }

    /// <summary>
    /// A white block over the background, seen from the default angles: its silhouette is
    /// slanted in screen space, so it is all edge.
    /// </summary>
    private static SceneState Edges()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 24, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 12, columns: 4, rows: 4)
            .Build();
}
