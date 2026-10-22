using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The decorative TV box of SPEC-06 §3.15: the tunnel of REQ-SCN-053 and the frame of
/// REQ-SCN-054, checked against the vectors of §5.3. The television model itself is AMD-025's
/// and lives in <see cref="TvModelTests"/>.
/// </summary>
[Collection(GlCollection.Name)]
public sealed class TvBoxTests
{
    private static readonly Vec4F Default = new(0.1f, 8.1f, 8.1f, 0.1f);

    [Fact]
    public void Section_5_3_the_five_tunnel_boxes_have_the_published_bounds()
    {
        TunnelBox[] tunnel = TvBox.Tunnel(Default);
        Assert.Equal(TvBox.TunnelBoxCount, tunnel.Length);

        // Left wall: x in [-8, 0.1], y in [4.1, 243.9], z in [-16, 112].
        Box(tunnel[0], (-8f, 0.1f), (4.1f, 243.9f), (-16f, 112f));

        // Right wall: x in [247.9, 264].
        Box(tunnel[1], (247.9f, 264f), (4.1f, 243.9f), (-16f, 112f));

        // Bottom wall: y in [-4, 8.1].
        Box(tunnel[2], (-8f, 264f), (-4f, 8.1f), (-16f, 112f));

        // Top wall: y in [239.9, 244].
        Box(tunnel[3], (-8f, 264f), (239.9f, 244f), (-16f, 112f));

        // Back: z in [112, 120], over the reference frame.
        Box(tunnel[4], (0f, 256f), (0f, 240f), (112f, 120f));
    }

    [Fact]
    public void REQ_SCN_053_the_inner_walls_of_the_tunnel_coincide_with_the_visible_rectangle()
    {
        Vec4F clipping = new(16.1f, 7.9f, 24.1f, 0.5f);
        TunnelBox[] tunnel = TvBox.Tunnel(clipping);
        VisibleRect rect = VisibleRect.From(clipping);

        Assert.Equal(rect.MinX, tunnel[0].Max.X, 3);
        Assert.Equal(rect.MaxX, tunnel[1].Min.X, 3);
        Assert.Equal(rect.MinY, tunnel[2].Max.Y, 3);
        Assert.Equal(rect.MaxY, tunnel[3].Min.Y, 3);
    }

    [Fact] // AMD-025
    public void AMD_025_the_tunnel_is_clipped_to_what_the_cabinet_hides_and_keeps_its_inner_walls()
    {
        Vec4F clipping = new(16.1f, 7.9f, 24.1f, 0.5f);
        VisibleRect rect = VisibleRect.From(clipping);

        // A cabinet narrower than the padding of REQ-SCN-053, the case a very large clipping
        // produces: the outer edges of the tunnel come in, the inner ones do not move.
        VisibleRect footprint = new(rect.MinX - 4f, rect.MinY - 4f, rect.MaxX + 4f, rect.MaxY + 4f);
        TunnelBox[] tunnel = [.. TvBox.Tunnel(clipping).Select(box => box.ClampTo(footprint))];

        Assert.Equal(footprint.MinX, tunnel[0].Min.X, 3);
        Assert.Equal(rect.MinX, tunnel[0].Max.X, 3);
        Assert.Equal(rect.MaxX, tunnel[1].Min.X, 3);
        Assert.Equal(footprint.MaxX, tunnel[1].Max.X, 3);
        Assert.Equal(footprint.MinY, tunnel[2].Min.Y, 3);
        Assert.Equal(rect.MinY, tunnel[2].Max.Y, 3);
        Assert.Equal(rect.MaxY, tunnel[3].Min.Y, 3);
        Assert.Equal(footprint.MaxY, tunnel[3].Max.Y, 3);

        // Depth is never touched: the tunnel keeps the z span of REQ-SCN-053.
        Assert.All(tunnel[..4], box =>
        {
            Assert.Equal(-16f, box.Min.Z, 3);
            Assert.Equal(112f, box.Max.Z, 3);
        });

        // A cabinet larger than the tunnel leaves it exactly as it was.
        VisibleRect wide = new(-100f, -100f, 400f, 400f);
        TunnelBox[] untouched = TvBox.Tunnel(clipping);
        for (int index = 0; index < untouched.Length; index++)
        {
            Assert.Equal(untouched[index], untouched[index].ClampTo(wide));
        }
    }

    [Fact]
    public void Section_5_3_the_tv_frame_origin_and_scales_are_those_of_the_vector()
    {
        Vector3 origin = TvBox.FrameOrigin(Default);
        Vector3 scale = TvBox.FrameScale(Default);

        Assert.Equal(SceneConstants.Center.X - 0.03125f, origin.X, 4);
        Assert.Equal(SceneConstants.Center.Y + 0.03333f, origin.Y, 4);
        Assert.Equal(-16f, origin.Z, 4);
        Assert.Equal(0.96797f, scale.X, 4);
        Assert.Equal(0.96583f, scale.Y, 4);
        Assert.Equal(1f, scale.Z, 4);
    }

    [Fact]
    public void REQ_SCN_054_the_tv_frame_follows_the_clip_margins()
    {
        // With no margins the TV frame is the identity scale centred on C.
        Vector3 scale = TvBox.FrameScale(new Vec4F(0f, 0f, 0f, 0f));
        Assert.Equal(1f, scale.X, 5);
        Assert.Equal(1f, scale.Y, 5);

        // Cutting 32 units off each side shrinks the television by the same ratio.
        Vector3 narrow = TvBox.FrameScale(new Vec4F(32f, 0f, 32f, 0f));
        Assert.Equal((256f - 64f) / 256f, narrow.X, 5);
    }

    [GlFact] // AMD-025
    public void AMD_025_without_a_model_the_tv_setting_draws_nothing_at_all()
    {
        using GlFixture gl = GlFixture.Create(160, 160);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = new SceneFixture().Background(0x22).Build();
        scene.Settings.Shadows = false;
        scene.Settings.TvBox = TvBoxKind.Tv;

        Capture capture = gl.Render(renderer, scene);

        // The TV box option is disabled when the file is missing: neither the cabinet nor the
        // tunnel is drawn, and the image is the plain background of REQ-SCN-046.
        Assert.Equal(0, renderer.Statistics.TvParts);
        Assert.Equal(0, renderer.Statistics.TunnelBoxes);
        Assert.Single(capture.DistinctColors());
    }

    [GlFact] // REQ-SCN-053, AMD-025
    public void REQ_SCN_053_the_tunnel_frames_the_visible_rectangle_in_the_background_colour()
    {
        if (TestTelevision.Model is not { } television)
        {
            return;
        }

        using GlFixture gl = GlFixture.Create(200, 200);
        using SceneRenderer renderer = new(gl.Gl, null, television);

        // A red backdrop well beyond the tunnel back, which sits at z in [112, 120]: the
        // tunnel must hide everything deeper than that (REQ-SCN-053).
        SceneFixture fixture = new SceneFixture().Background(0x22);
        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = new Rgb24(255, 0, 0);
        scene.Backdrop.Offset = new Vector3(0f, 0f, 200f);

        Capture open = gl.Render(renderer, scene);
        Assert.Equal(0, renderer.Statistics.TunnelBoxes);
        int backdropPixels = open.CountOther(NesPalette.Color(0x22).Packed);
        Assert.True(backdropPixels > 0, "the backdrop should be visible without the TV box");

        scene.Settings.TvBox = TvBoxKind.Tv;
        Capture framed = gl.Render(renderer, scene);
        Assert.Equal(TvBox.TunnelBoxCount, renderer.Statistics.TunnelBoxes);

        // The tunnel hides everything deeper than its back wall: the red backdrop is gone.
        Assert.Equal(0, framed.Count(0xFF0000));

        // The tunnel is unlit and flat, with exactly the background colour of REQ-SCN-046, so
        // that is what the opening shows. Around it stands the cabinet, which is lit, so the
        // rest of the image is no longer uniform.
        Assert.Equal(NesPalette.Color(0x22).Packed, framed.Packed(framed.Width / 2, framed.Height / 2));
        Assert.True(framed.DistinctColors().Count > 2, "the cabinet should be drawn around it");
    }

    private static void Box(
        TunnelBox box,
        (float Min, float Max) x,
        (float Min, float Max) y,
        (float Min, float Max) z)
    {
        Assert.Equal(x.Min, box.Min.X, 3);
        Assert.Equal(x.Max, box.Max.X, 3);
        Assert.Equal(y.Min, box.Min.Y, 3);
        Assert.Equal(y.Max, box.Max.Y, 3);
        Assert.Equal(z.Min, box.Min.Z, 3);
        Assert.Equal(z.Max, box.Max.Z, 3);
    }
}
