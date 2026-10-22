using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The 2D mode of SPEC-06 REQ-SCN-055 as AMD-009 (Q-022) settles it: the core's 256 × 240 pixel
/// frame mapped on the NES plane of REQ-E3D-410, drawn by the same 3D pipeline.
/// </summary>
[Collection(GlCollection.Name)]
public sealed class TwoDimensionalModeTests
{
    private const int Width = 256;
    private const int Height = 240;

    [GlFact]
    public void AMD_009_the_pixel_frame_replaces_the_tile_list_with_a_single_quad()
    {
        using GlFixture gl = GlFixture.Create(320, 300);
        using SceneRenderer renderer = new(gl.Gl);

        // A frame built from tile records, so that the draw list is not empty: in 2D mode the
        // renderer must ignore it and draw the picture instead (REQ-E3D-381 no longer applies).
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x17)
            .Rect(SceneFixture.Solid(1), 4, 4, 8, 8);
        fixture.TwoDimensionalMode = true;
        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.TwoDimensionalMode = true;
        scene.NesPicture = Picture(0x30);

        gl.Render(renderer, scene);

        Assert.Equal(1, renderer.Statistics.OpaqueTiles);
        Assert.Equal(0, renderer.Statistics.TransparentTiles);
        Assert.Equal(2, renderer.Statistics.Triangles);
        Assert.False(renderer.Statistics.ShadowPass);
    }

    [GlFact]
    public void AMD_009_a_white_picture_comes_out_at_the_lit_srgb_of_section_5_2()
    {
        using GlFixture gl = GlFixture.Create(320, 300);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = Scene(Picture(0x30));
        Capture capture = gl.Render(renderer, scene);

        // The quad faces the default camera, so N·L = 0.577 and the factor is 0.7585:
        // NES 0x30 (252, 248, 252) comes out at (223, 219, 223), the vector of §5.2.
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);
        Assert.Equal(223, r, tolerance: 3);
        Assert.Equal(219, g, tolerance: 3);
        Assert.Equal(223, b, tolerance: 3);
    }

    [GlFact]
    public void AMD_009_a_blue_picture_comes_out_at_the_lit_srgb_of_section_5_2()
    {
        using GlFixture gl = GlFixture.Create(320, 300);
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, Scene(Picture(0x22)));

        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);
        Assert.Equal(91, r, tolerance: 3);
        Assert.Equal(120, g, tolerance: 3);
        Assert.Equal(223, b, tolerance: 3);
    }

    [GlFact]
    public void REQ_E3D_410_the_picture_is_not_turned_upside_down()
    {
        using GlFixture gl = GlFixture.Create(320, 300);
        using SceneRenderer renderer = new(gl.Gl);

        // NES row 0 is red, row 239 is green; row 0 is at the top of the screen.
        byte[] picture = Picture(0x0F);
        for (int x = 0; x < Width; x++)
        {
            for (int y = 0; y < 40; y++)
            {
                picture[(y * Width) + x] = 0x16;
                picture[((Height - 1 - y) * Width) + x] = 0x1A;
            }
        }

        Capture capture = gl.Render(renderer, Scene(picture));

        int middle = capture.Width / 2;
        (int topR, _, _) = capture.At(middle, 30);
        (int bottomR, int bottomG, _) = capture.At(middle, capture.Height - 40);
        Assert.True(topR > 100, "the top of the capture should show the red rows of the picture");
        Assert.True(bottomG > bottomR, "the bottom of the capture should show the green rows");
    }

    [GlFact]
    public void REQ_SCN_036_the_picture_is_cut_by_the_visible_rectangle()
    {
        using GlFixture gl = GlFixture.Create(320, 300);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState wide = Scene(Picture(0x30));
        wide.Clipping = new Vec4F(0.1f, 8.1f, 8.1f, 0.1f);
        (int First, int Last) full = gl.Render(renderer, wide).ColumnSpan(0x000000);

        SceneState narrow = Scene(Picture(0x30));
        narrow.Clipping = new Vec4F(64.1f, 8.1f, 64.1f, 0.1f);
        (int First, int Last) cut = gl.Render(renderer, narrow).ColumnSpan(0x000000);

        Assert.True(cut.First > full.First, "the left margin should bite into the picture");
        Assert.True(cut.Last < full.Last, "the right margin should bite into the picture");
    }

    [GlFact]
    public void AMD_009_a_picture_of_the_wrong_size_falls_back_to_the_tile_list()
    {
        using GlFixture gl = GlFixture.Create(160, 150);
        using SceneRenderer renderer = new(gl.Gl);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 4, 4, 8, 8);
        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.TwoDimensionalMode = true;
        scene.NesPicture = new byte[16];

        gl.Render(renderer, scene);

        Assert.True(renderer.Statistics.OpaqueTiles > 1);
    }

    private static SceneState Scene(byte[] picture)
    {
        SceneState scene = new()
        {
            GameRunning = true,
            TwoDimensionalMode = true,
            NesPicture = picture,
            NesBackgroundColor = default,
        };
        scene.Settings.Shadows = false;
        scene.Camera.SnapToSetpoints();
        return scene;
    }

    private static byte[] Picture(byte nesColor)
    {
        byte[] picture = new byte[Width * Height];
        Array.Fill(picture, nesColor);
        return picture;
    }
}
