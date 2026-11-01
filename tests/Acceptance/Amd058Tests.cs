using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-058 — the requirement of SPEC-06 REQ-SCN-065 holds for <b>every</b> value of
/// <c>video.anti_alias</c>. The two automatic criteria of §2: the 4 × 4 surface of identical
/// tiles, lit and receiving the shadow, with <c>anti_alias</c> 0, 1 and 2; and the same surface
/// made of tiles belonging to two neighbouring objects, for the three values.
/// </summary>
/// <remarks>
/// <para>
/// A third measure is added, because the two the amendment names cannot fail on their own: a
/// surface of identical tiles carries the same texel everywhere, so reading the wrong one
/// changes no colour. The third builds two objects whose textures carry one loud column each,
/// placed where a read one texel past the edge of a face wraps onto it — which is the defect
/// the amendment reports, measured on a joint that stays flat.
/// </para>
/// <para>
/// The means AMD-058 excludes are checked too: the walls of AMD-057 are still in the drawn
/// meshes, and the sample count the setting asks for is still the one the renderer is granted
/// (AMD-032 §2).
/// </para>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Amd058Tests
{
    /// <summary>AMD-012: no joint pixel may be more than 2 levels off the surface colour.</summary>
    private const int Tolerance = 2;

    /// <summary>The three values of <c>video.anti_alias</c> (AMD-032 §2).</summary>
    private static readonly int[] Options = [0, 1, 2];

    /// <summary>
    /// AMD-058 §2, first automatic criterion — the criterion of REQ-SCN-065 (a 4 × 4 surface of
    /// identical tiles, lit, receiving the shadow) passes with <c>video.anti_alias</c> 0, 1
    /// and 2.
    /// </summary>
    [GlFact] // AMD-058, REQ-SCN-065, AMD-032 §2
    public void The_four_by_four_surface_shows_no_seam_for_the_three_values()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        foreach (int option in Options)
        {
            SceneState scene = OneObject();

            // The walls AMD-058 forbids removing are there: a full tile is the closed box of
            // REQ-E3D-501, 24 vertices and 12 triangles (AMD-057).
            AssertWholeBoxes(scene, expected: 16);

            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            Shot shot = target.Render(renderer, scene);

            // Neither the anti-aliasing nor the sample count is lowered to get there.
            Assert.Equal(SceneConstants.AntiAliasSamples(option), renderer.Statistics.AntiAliasSamples);
            Assert.True(renderer.Statistics.ShadowPass);

            (int surface, int worst, int offenders) = Measure(shot);
            Assert.True(
                offenders == 0,
                $"with video.anti_alias = {option}, {offenders} pixel(s) of the surface differ "
                    + $"from {Rgb(surface)} by up to {worst} levels");
        }
    }

    /// <summary>
    /// AMD-058 §2, second automatic criterion — the same surface made of tiles belonging to
    /// <b>two</b> objects, two columns of 2 × 4 tiles, for the three values.
    /// </summary>
    [GlFact] // AMD-058, REQ-SCN-065
    public void The_surface_split_over_two_objects_shows_no_seam_for_the_three_values()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        foreach (int option in Options)
        {
            SceneState scene = TwoObjects();
            Assert.Equal(2, scene.DrawList!.Tiles.Select(static tile => tile.Instance).Distinct().Count());
            AssertWholeBoxes(scene, expected: 16);

            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            Shot shot = target.Render(renderer, scene);

            (int surface, int worst, int offenders) = Measure(shot);
            Assert.True(
                offenders == 0,
                $"with video.anti_alias = {option}, {offenders} pixel(s) of the two-object "
                    + $"surface differ from {Rgb(surface)} by up to {worst} levels");
        }
    }

    /// <summary>
    /// The defect AMD-058 reports, measured: at a joint no sample may take its texel from
    /// somewhere else in the object's texture.
    /// </summary>
    /// <remarks>
    /// Each object is two cells wide, so its texture is 16 texels across and the read of
    /// REQ-E3D-563, taken modulo that size, wraps column 16 onto column 0 and column −1 onto
    /// column 15. The first column of the left object and the last column of the right one are
    /// loud red, everything else the white of the surface, so both loud columns stand against
    /// the outer silhouettes and the joint between the two objects stays flat — while a read
    /// one texel too far, in either direction, lands on a loud one.
    /// </remarks>
    [GlFact] // AMD-058, REQ-SCN-065, REQ-E3D-563, REQ-E3D-506
    public void No_sample_at_a_joint_takes_its_texel_from_across_the_texture()
    {
        using GlTarget target = GlTarget.Create(1280, 960);
        using SceneRenderer renderer = new(target.Gl);

        // What the surface must keep is the colour the capture without anti-aliasing gives it:
        // AMD-058 asks the anti-aliased capture to differ from that one at the edges of the
        // objects alone, never inside a flat surface.
        int reference = 0;
        foreach (int option in Options)
        {
            SceneState scene = TwoObjectsOneLoudColumnEach();
            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            Shot shot = target.Render(renderer, scene);

            // The middle half of the drawn box across: the joint between the two objects runs
            // through it and the two loud columns, which hug the silhouettes, do not.
            (int x0, int y0, int x1, int y1) = Band(shot);
            if (option == 0)
            {
                reference = Commonest(shot, x0, y0, x1, y1);
                (int red, int green, int blue) = shot.At((x0 + x1) / 2, (y0 + y1) / 2);
                Assert.True(
                    Math.Abs(red - green) <= 8 && Math.Abs(green - blue) <= 8,
                    $"the measured window is not on the white surface but on ({red}, {green}, {blue})");
            }

            int worst = 0;
            int offenders = 0;
            for (int y = y0; y <= y1; y++)
            {
                for (int x = x0; x <= x1; x++)
                {
                    int deviation = Deviation(shot.Packed(x, y), reference);
                    worst = Math.Max(worst, deviation);
                    offenders += deviation > Tolerance ? 1 : 0;
                }
            }

            Assert.True(
                offenders == 0,
                $"with video.anti_alias = {option}, {offenders} pixel(s) across the joint differ "
                    + $"from {Rgb(reference)} by up to {worst} levels: a sample took its texel "
                    + "from across the object's texture");
        }
    }

    // ------------------------------------------------------------------ scenes

    /// <summary>The surface of REQ-SCN-065: 4 × 4 identical tiles, lit, casting and receiving.</summary>
    private static SceneState OneObject()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 6, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 4, rows: 4)
            .Build();

    /// <summary>
    /// The same 4 × 4 surface, split over two objects of 2 × 4 tiles. The two palette entries
    /// carry the same NES colour, so the surface is of one colour while the two tile images
    /// differ — which is what makes them two patterns, hence two objects.
    /// </summary>
    private static SceneState TwoObjects()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Palette(2, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 6, ProfileLayer.L1, columns: 2, rows: 4)
            .Pattern(SceneFixture.Solid(2), depth: 6, ProfileLayer.L1, columns: 2, rows: 4)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 2, rows: 4)
            .Rect(SceneFixture.Solid(2), column: 16, row: 14, columns: 2, rows: 4)
            .Build();

    /// <summary>
    /// Two neighbouring objects of two cells each, over four rows, the left one's first texel
    /// column and the right one's last texel column loud red.
    /// </summary>
    private static SceneState TwoObjectsOneLoudColumnEach()
    {
        (ulong Low, ulong High) leftEdge = SceneFixture.Striped(3, column: 0, stripe: 1);
        (ulong Low, ulong High) leftInner = SceneFixture.Solid(3, id: 1);
        (ulong Low, ulong High) rightInner = SceneFixture.Solid(3, id: 2);
        (ulong Low, ulong High) rightEdge = SceneFixture.Striped(3, column: 7, stripe: 1);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Palette(1, 0x16)
            .PatternRow(depth: 6, ProfileLayer.L1, leftEdge, leftInner)
            .PatternRow(depth: 6, ProfileLayer.L1, rightInner, rightEdge);

        for (int row = 14; row < 18; row++)
        {
            fixture
                .Tile(leftEdge, 14 * 8, row * 8)
                .Tile(leftInner, 15 * 8, row * 8)
                .Tile(rightInner, 16 * 8, row * 8)
                .Tile(rightEdge, 17 * 8, row * 8);
        }

        return fixture.Build();
    }

    // ------------------------------------------------------------------ measures

    /// <summary>
    /// Checks that every tile of the surface is drawn with the whole mesh of REQ-E3D-501 — the
    /// walls AMD-058 forbids removing are still there (AMD-057).
    /// </summary>
    private static void AssertWholeBoxes(SceneState scene, int expected)
    {
        Assert.NotNull(scene.DrawList);
        Assert.Equal(expected, scene.DrawList.Tiles.Count);
        foreach (Zax.Engine3D.DrawTile tile in scene.DrawList.Tiles)
        {
            Assert.NotNull(tile.Mesh);
            Assert.Equal(24, tile.Mesh.VertexCount);
            Assert.Equal(12, tile.Mesh.TriangleCount);
        }
    }

    /// <summary>
    /// The drawn box inset by a twelfth of its side, so that only the front face of the surface
    /// is sampled and never its silhouette or its side faces. A 4 × 4 surface keeps its three
    /// joints in each direction.
    /// </summary>
    private static (int X0, int Y0, int X1, int Y1) Interior(Shot shot)
    {
        (int firstX, int lastX) = shot.ColumnSpan(0);
        (int firstY, int lastY) = shot.RowSpan(0);
        Assert.True(lastX - firstX > 24 && lastY - firstY > 24, "the surface is too small to measure");

        int insetX = (lastX - firstX + 1) / 12;
        int insetY = (lastY - firstY + 1) / 12;
        return (firstX + insetX, firstY + insetY, lastX - insetX, lastY - insetY);
    }

    /// <summary>The middle half of the drawn box across, its twelfths cut off down.</summary>
    private static (int X0, int Y0, int X1, int Y1) Band(Shot shot)
    {
        (int firstX, int lastX) = shot.ColumnSpan(0);
        (int firstY, int lastY) = shot.RowSpan(0);
        Assert.True(lastX - firstX > 48 && lastY - firstY > 48, "the surface is too small to measure");

        int width = lastX - firstX + 1;
        int insetY = (lastY - firstY + 1) / 12;
        return (firstX + (width / 4), firstY + insetY, lastX - (width / 4), lastY - insetY);
    }

    private static (int Surface, int Worst, int Offenders) Measure(Shot shot)
    {
        (int x0, int y0, int x1, int y1) = Interior(shot);
        int surface = Commonest(shot, x0, y0, x1, y1);

        int worst = 0;
        int offenders = 0;
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                int deviation = Deviation(shot.Packed(x, y), surface);
                worst = Math.Max(worst, deviation);
                offenders += deviation > Tolerance ? 1 : 0;
            }
        }

        return (surface, worst, offenders);
    }

    private static int Commonest(Shot shot, int x0, int y0, int x1, int y1)
    {
        Dictionary<int, int> counts = [];
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                int packed = shot.Packed(x, y);
                counts[packed] = counts.GetValueOrDefault(packed) + 1;
            }
        }

        return counts.MaxBy(static entry => entry.Value).Key;
    }

    /// <summary>The largest departure of one channel between two packed colours.</summary>
    private static int Deviation(int left, int right)
        => Math.Max(
            Math.Abs(((left >> 16) & 0xFF) - ((right >> 16) & 0xFF)),
            Math.Max(
                Math.Abs(((left >> 8) & 0xFF) - ((right >> 8) & 0xFF)),
                Math.Abs((left & 0xFF) - (right & 0xFF))));

    private static string Rgb(int packed)
        => $"({(packed >> 16) & 0xFF}, {(packed >> 8) & 0xFF}, {packed & 0xFF})";
}
