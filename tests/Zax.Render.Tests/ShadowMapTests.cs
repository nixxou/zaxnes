using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// AMD-034 §2: the shadow map is 4096 texels of side by default, falls back to the largest size
/// the hardware accepts (2048, then 1024), and the depth bias is one texel of the map in use,
/// never under 0.02 unit.
/// </summary>
/// <remarks>
/// The size chain and the bias are arithmetic and need no context; only the last two tests ask
/// for one. AMD-034 §1 measured that the lighting is already right, so nothing here touches the
/// ambient, the shadow strength or the light direction.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class ShadowMapTests
{
    /// <summary>The floor in shadow: NES 0x30 seen from the front (SPEC-06 §5.2).</summary>
    private static (int R, int G, int B) ShadowedFloor => (146, 144, 146);

    /// <summary>AMD-034 §2, replacing the 1024 texels of DEC-R2-D4.</summary>
    [Fact] // REQ-SCN-030, AMD-034
    public void The_default_shadow_map_is_four_thousand_ninety_six_texels()
    {
        Assert.Equal(4096, SceneConstants.ShadowMapSize);
        Assert.Equal(4096, new RenderSettings().ShadowMapSize);
    }

    /// <summary>
    /// The sizes tried, largest first. OpenGL 3.3 only guarantees a 1024-texel texture, so the
    /// chain the amendment names — 4096, 2048, 1024 — is the one a default map walks down.
    /// </summary>
    [Fact] // REQ-SCN-030, AMD-034
    public void The_fallback_chain_halves_the_requested_size()
        => Assert.Equal(
            [4096, 2048, 1024, 512, 256, 128, 64],
            SceneConstants.ShadowMapSizes(4096, 16384));

    /// <summary>
    /// A hardware limit below what was asked for removes the sizes the driver already says it
    /// cannot hold, so the first size tried is the largest possible one (AMD-034 §2).
    /// </summary>
    [Theory] // REQ-SCN-030, AMD-034
    // requested, GL_MAX_TEXTURE_SIZE, largest size tried
    [InlineData(4096, 16384, 4096)]
    [InlineData(4096, 4096, 4096)]
    [InlineData(4096, 2048, 2048)]
    [InlineData(4096, 1024, 1024)]
    [InlineData(2048, 16384, 2048)]
    public void The_largest_size_tried_is_capped_by_the_hardware(int requested, int max, int first)
        => Assert.Equal(first, SceneConstants.ShadowMapSizes(requested, max).First());

    /// <summary>
    /// A driver that answers nonsense, or a map asked for below the floor, still yields one
    /// size to try: the renderer must never end up with no shadow map at all.
    /// </summary>
    [Theory] // REQ-SCN-030, AMD-034
    [InlineData(4096, 0)]
    [InlineData(4096, -1)]
    [InlineData(0, 16384)]
    [InlineData(-8, 16384)]
    public void A_nonsense_size_or_limit_still_leaves_the_floor_to_try(int requested, int max)
        => Assert.Equal([SceneConstants.MinShadowMapSize], SceneConstants.ShadowMapSizes(requested, max));

    /// <summary>
    /// AMD-034 §2 lowers the bias of Q-032 from 1.5 texels to 1 texel, and keeps the 0.02 unit
    /// of REQ-SCN-031 as a floor it may never go under.
    /// </summary>
    [Fact] // REQ-SCN-031, AMD-034
    public void The_bias_is_one_texel_and_never_under_two_hundredths()
    {
        Assert.Equal(1f, SceneConstants.ShadowBiasTexels);
        Assert.Equal(0.02f, SceneConstants.ShadowDepthBias);

        // Above the floor the bias is exactly the texel.
        Assert.Equal(0.5f, SceneConstants.ShadowBias(0.5f), 1e-6f);
        Assert.Equal(0.1f, SceneConstants.ShadowBias(0.1f), 1e-6f);

        // Below it, and at the degenerate sizes, the floor wins.
        Assert.Equal(0.02f, SceneConstants.ShadowBias(0.019f), 1e-6f);
        Assert.Equal(0.02f, SceneConstants.ShadowBias(0f), 1e-6f);
    }

    /// <summary>
    /// The point of the amendment, as arithmetic: on the same covered area a 4096-texel map has
    /// a texel four times finer than a 1024-texel one, and the bias that is subtracted from
    /// every shadow shrinks with it — six times less at 1 texel than at the 1.5 texels of Q-032.
    /// </summary>
    [Fact] // REQ-SCN-031, AMD-034
    public void A_finer_map_shortens_the_bias_the_shadows_lose()
    {
        // The extent of the SMB3 world map across the light, about 410 units.
        const float extent = 410f;
        float before = MathF.Max(0.02f, 1.5f * (extent / 1024f));
        float after = SceneConstants.ShadowBias(extent / 4096f);

        Assert.True(after < before / 5f, $"bias {after} is not six times under the {before} of Q-032");
    }

    /// <summary>
    /// With a context: the renderer reports the size it was granted, which on a desktop GL 3.3
    /// driver is the 4096 texels asked for, and a changed setting is honoured on the next frame.
    /// </summary>
    [GlFact] // REQ-SCN-030, AMD-034
    public void The_renderer_reports_the_shadow_map_size_it_was_granted()
    {
        using GlFixture gl = GlFixture.Create(320, 240);
        using SceneRenderer renderer = new(gl.Gl);

        foreach (int asked in new[] { 4096, 2048, 1024, 4096 })
        {
            SceneState scene = ThinStrokes();
            scene.Settings.ShadowMapSize = asked;
            gl.Render(renderer, scene);

            Assert.True(renderer.Statistics.ShadowPass);
            Assert.Equal(asked, renderer.Statistics.RequestedShadowMapSize);
            Assert.Equal(asked, renderer.Statistics.ShadowMapSize);
        }
    }

    /// <summary>
    /// What the user reported: on the same scene, a finer map casts more shadow. The glyphs of
    /// AMD-034 §1 are one-unit strokes standing 8 units in front of a panel; here single-tile
    /// columns standing in front of a wall stand in for them, and the shadowed pixels of the
    /// wall are counted.
    /// </summary>
    [GlFact] // REQ-SCN-030, AMD-034
    public void A_finer_map_casts_more_shadow_under_a_thin_stroke()
    {
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        int coarse = ShadowedPixels(gl, renderer, 1024);
        int fine = ShadowedPixels(gl, renderer, SceneConstants.ShadowMapSize);

        Assert.True(coarse > 0, "the coarse map casts no shadow at all; the scene is wrong");
        Assert.True(
            fine > coarse,
            $"the {SceneConstants.ShadowMapSize}-texel map shadows {fine} pixels, no more than "
                + $"the {coarse} of a 1024-texel one");
    }

    private static int ShadowedPixels(GlFixture gl, SceneRenderer renderer, int mapSize)
    {
        SceneState scene = ThinStrokes();
        scene.Settings.ShadowMapSize = mapSize;
        Capture capture = gl.Render(renderer, scene);

        int count = 0;
        for (int y = 0; y < capture.Height; y++)
        {
            for (int x = 0; x < capture.Width; x++)
            {
                (int r, int g, int b) = capture.At(x, y);
                if (Math.Abs(r - ShadowedFloor.R) <= 4
                    && Math.Abs(g - ShadowedFloor.G) <= 4
                    && Math.Abs(b - ShadowedFloor.B) <= 4)
                {
                    count++;
                }
            }
        }

        return count;
    }

    /// <summary>
    /// A flat white wall with one-tile-wide columns standing 32 units in front of it: at 1024
    /// texels a texel is wider than a column, so their shadows break into dots (AMD-034 §1).
    /// </summary>
    private static SceneState ThinStrokes()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(2, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(2), depth: 32, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 6, row: 8, columns: 20, rows: 14);

        for (int column = 8; column < 24; column += 2)
        {
            fixture.Rect(SceneFixture.Solid(2), column, row: 10, columns: 1, rows: 10);
        }

        return fixture.Build();
    }
}
