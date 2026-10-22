using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// Texturing of the tiles: the indexed palette lookup of SPEC-06 REQ-SCN-034 and the texel
/// addressing of SPEC-04B REQ-E3D-506 and REQ-E3D-563, checked on captures where every texel has
/// a known place.
/// </summary>
[Collection(GlCollection.Name)]
public sealed class MaterialTests
{
    private const int Black = 0;

    [GlFact]
    public void REQ_E3D_506_the_texture_of_a_tile_is_not_flipped_or_mirrored()
    {
        // Colour 1 fills the top-left quadrant, colour 2 the bottom-right one.
        (ulong Low, ulong High) quadrants = Quadrants();

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)   // red,  top-left
            .Palette(2, 0x12)   // blue, bottom-right
            .Pattern(quadrants, depth: 1, ProfileLayer.L1)
            .Rect(quadrants, column: 14, row: 14, columns: 2, rows: 2);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int firstRow, int lastRow) = capture.RowSpan(Black);
        (int firstColumn, int lastColumn) = capture.ColumnSpan(Black);

        // Sample well inside the two quadrants of the top-left tile of the 2 x 2 block.
        int quarterX = firstColumn + ((lastColumn - firstColumn) / 8);
        int quarterY = firstRow + ((lastRow - firstRow) / 8);
        int threeQuarterX = lastColumn - ((lastColumn - firstColumn) / 8);
        int threeQuarterY = lastRow - ((lastRow - firstRow) / 8);

        (int topLeftR, _, int topLeftB) = capture.At(quarterX, quarterY);
        (int bottomRightR, _, int bottomRightB) = capture.At(threeQuarterX, threeQuarterY);

        Assert.True(topLeftR > topLeftB, $"the top-left quadrant is not red: ({topLeftR}, _, {topLeftB})");
        Assert.True(
            bottomRightB > bottomRightR,
            $"the bottom-right quadrant is not blue: ({bottomRightR}, _, {bottomRightB})");
    }

    [GlFact]
    public void REQ_E3D_506_each_cell_of_a_two_cell_pattern_keeps_its_own_texels()
    {
        // A two-cell pattern [A B] whose texture is 16 x 8 texels: A has to stay on the left.
        (ulong Low, ulong High) left = SceneFixture.Solid(1, id: 0);
        (ulong Low, ulong High) right = SceneFixture.Solid(2, id: 1);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)
            .Palette(2, 0x12)
            .PatternRow(depth: 1, ProfileLayer.L1, left, right);

        for (int row = 12; row < 18; row++)
        {
            fixture.Tile(left, 15 * 8, row * 8);
            fixture.Tile(right, 16 * 8, row * 8);
        }

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int firstColumn, int lastColumn) = capture.ColumnSpan(Black);
        int middleY = capture.Height / 2;

        (int leftR, _, int leftB) = capture.At(firstColumn + ((lastColumn - firstColumn) / 4), middleY);
        (int rightR, _, int rightB) = capture.At(lastColumn - ((lastColumn - firstColumn) / 4), middleY);

        Assert.True(leftR > leftB, $"the left cell is not red: ({leftR}, _, {leftB})");
        Assert.True(rightB > rightR, $"the right cell is not blue: ({rightR}, _, {rightB})");
    }

    [GlFact]
    public void REQ_SCN_055_in_2d_mode_a_learned_band_keeps_its_tiles_in_order()
    {
        // No profile at all: every tile image is learned as it appears, and the background pass
        // groups them into a band (SPEC-04 REQ-E3D-381). Four tiles of four different colours in
        // a row have to come out in that order.
        (ulong Low, ulong High) a = SceneFixture.Solid(1, id: 0);
        (ulong Low, ulong High) b = SceneFixture.Solid(2, id: 1);
        (ulong Low, ulong High) c = SceneFixture.Solid(3, id: 2);
        (ulong Low, ulong High) d = SceneFixture.Solid(1, id: 3);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)   // red
            .Palette(2, 0x1A)   // green
            .Palette(3, 0x12);  // blue
        fixture.TwoDimensionalMode = true;

        for (int row = 13; row < 17; row++)
        {
            fixture.Tile(a, 12 * 8, row * 8);
            fixture.Tile(b, 13 * 8, row * 8);
            fixture.Tile(c, 14 * 8, row * 8);
            fixture.Tile(d, 15 * 8, row * 8);
        }

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int firstColumn, int lastColumn) = capture.ColumnSpan(Black);
        int middleY = capture.Height / 2;
        int span = lastColumn - firstColumn + 1;

        (int R, int G, int B) first = capture.At(firstColumn + (span / 8), middleY);
        (int R, int G, int B) second = capture.At(firstColumn + (3 * span / 8), middleY);
        (int R, int G, int B) third = capture.At(firstColumn + (5 * span / 8), middleY);

        Assert.True(first.R > first.G && first.R > first.B, $"cell 1 is not red: {first}");
        Assert.True(second.G > second.R && second.G > second.B, $"cell 2 is not green: {second}");
        Assert.True(third.B > third.R && third.B > third.G, $"cell 3 is not blue: {third}");
    }

    [GlFact]
    public void REQ_SCN_034_a_background_tile_reads_its_colour_at_4P_plus_the_texel()
    {
        // Palette 2 of the background, colour index 1, is entry 2*4 + 1 = 9 of the frame palette.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(9, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 12, row: 12, columns: 8, rows: 6, palette: 2);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        // NES 0x16 is (252, 56, 0); lit at N.L = 0.577 it stays strongly red.
        Assert.True(r > 150 && g < 100 && b < 60, $"expected the palette-2 red, got ({r}, {g}, {b})");
    }

    [GlFact]
    public void REQ_SCN_034_a_sprite_tile_reads_its_colour_at_4_times_palette_plus_four()
    {
        // Palette 1 of the sprites, colour index 1, is entry (1 + 4) * 4 + 1 = 21.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(21, 0x1A)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1);

        for (int row = 12; row < 18; row++)
        {
            for (int column = 12; column < 20; column++)
            {
                fixture.Sprite(SceneFixture.Solid(1), column * 8, row * 8, palette: 1);
            }
        }

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        // NES 0x1A is (0, 168, 0): a strong green.
        Assert.True(g > r + 40 && g > b + 40, $"expected the sprite palette green, got ({r}, {g}, {b})");
    }

    [GlFact]
    public void REQ_E3D_565_a_palette_change_shows_on_the_next_frame_without_a_rebuild()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 12, row: 12, columns: 8, rows: 6);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture red = gl.Render(renderer, scene);
        int uploadsAfterFirst = renderer.Statistics.MeshUploads;

        // Change colour 1 of the frame palette only; the meshes must not be rebuilt.
        ConversionEngine engine = fixture.Engine!;
        engine.SetColorIndex(1, 0x12);
        scene.DrawList = engine.BuildDrawList();

        Capture blue = gl.Render(renderer, scene);

        (int redR, _, int redB) = red.At(red.Width / 2, red.Height / 2);
        (int blueR, _, int blueB) = blue.At(blue.Width / 2, blue.Height / 2);

        Assert.True(redR > redB);
        Assert.True(blueB > blueR);
        Assert.Equal(uploadsAfterFirst, renderer.Statistics.MeshUploads);
    }

    /// <summary>
    /// A tile image whose top-left 4 × 4 pixels are colour 1 and whose bottom-right 4 × 4 are
    /// colour 2; the rest is transparent.
    /// </summary>
    private static (ulong Low, ulong High) Quadrants()
    {
        ulong low = 0;
        ulong high = 0;
        for (int y = 0; y < 8; y++)
        {
            for (int x = 0; x < 8; x++)
            {
                int color = x < 4 && y < 4 ? 1 : x >= 4 && y >= 4 ? 2 : 0;
                int bit = (y << 3) | (7 - x);
                if ((color & 1) != 0)
                {
                    low |= 1UL << bit;
                }

                if ((color & 2) != 0)
                {
                    high |= 1UL << bit;
                }
            }
        }

        return (low, high);
    }
}
