using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The occlusion transparency of SPEC-06 REQ-SCN-022: in FPV, an opaque tile of another shape
/// that crosses the segment from the eye to the followed target is drawn at 0.5 for that frame.
/// </summary>
[Collection(GlCollection.Name)]
public sealed class OcclusionTests
{
    [Fact]
    public void REQ_SCN_022_the_opacity_of_an_occluder_is_one_half()
        => Assert.Equal(0.5f, SceneConstants.OccluderAlpha, 5);

    [Fact]
    public void A_segment_that_crosses_a_box_is_detected_and_one_that_stops_short_is_not()
    {
        Vector3 min = new(-1f, -1f, 4f);
        Vector3 max = new(1f, 1f, 6f);

        Assert.True(Occlusion.SegmentHitsBox(Vector3.Zero, new Vector3(0f, 0f, 10f), min, max));
        Assert.False(Occlusion.SegmentHitsBox(Vector3.Zero, new Vector3(0f, 0f, 3f), min, max));
        Assert.False(Occlusion.SegmentHitsBox(
            new Vector3(5f, 0f, 0f), new Vector3(5f, 0f, 10f), min, max));

        // A segment that lies inside the box meets it.
        Assert.True(Occlusion.SegmentHitsBox(
            new Vector3(0f, 0f, 4.5f), new Vector3(0f, 0f, 5.5f), min, max));
    }

    [Fact]
    public void REQ_SCN_022_the_test_is_off_outside_fpv_and_without_a_followed_shape()
    {
        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        CameraView view = scene.Camera.BuildView(scene.Clipping, 1f);

        Assert.False(Occlusion.For(scene, view).Enabled);

        scene.Camera.SetMode(CameraMode.Fpv);
        Assert.False(Occlusion.For(scene, view).Enabled);
    }

    [GlFact]
    public void REQ_SCN_022_a_wall_in_front_of_the_followed_shape_becomes_transparent()
    {
        using GlFixture gl = GlFixture.Create(128, 128);
        using SceneRenderer renderer = new(gl.Gl);

        // A background wall in L1 at z = 0 and a sprite "character" behind it in L2.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(21, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 12, 12, 6, 6)
            .Sprite(SceneFixture.Solid(1, id: 3), 112, 112, palette: 1);

        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = new Vector3(116f, 128f, 20f);
        scene.Camera.Current.Distance = 120f;
        scene.Camera.Current.Angles = Vector3.Zero;
        scene.Camera.SnapToSetpoints();

        // Without a followed shape nothing fades.
        gl.Render(renderer, scene);
        Assert.Equal(0, renderer.Statistics.Occluders);

        // Following the sprite makes the wall tiles on the line of sight transparent.
        FrameShape sprite = fixture.Engine!.PublishedFrame!.Shapes[^1];
        scene.FollowedShape = sprite;
        gl.Render(renderer, scene);

        Assert.True(renderer.Statistics.Occluders > 0, "the wall should hide the followed shape");
        Assert.True(
            renderer.Statistics.TransparentTiles >= renderer.Statistics.Occluders,
            "every occluder is drawn with the transparent objects");
    }

    [GlFact]
    public void REQ_SCN_022_the_tiles_of_the_followed_shape_itself_never_fade()
    {
        using GlFixture gl = GlFixture.Create(96, 96);
        using SceneRenderer renderer = new(gl.Gl);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 12, 12, 4, 4);

        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = new Vector3(128f, 128f, 0f);
        scene.Camera.Current.Distance = 150f;
        scene.Camera.Current.Angles = Vector3.Zero;
        scene.Camera.SnapToSetpoints();

        // The only shape of the frame is the followed one, so nothing can occlude it.
        scene.FollowedShape = fixture.Engine!.PublishedFrame!.Shapes[0];
        gl.Render(renderer, scene);

        Assert.Equal(0, renderer.Statistics.Occluders);
    }
}
