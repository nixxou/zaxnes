using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// AMD-058: the requirement of SPEC-06 REQ-SCN-065 holds for <b>every</b> value of
/// <c>video.anti_alias</c> — no dark grid, no light crack and no break in the shadow at the
/// joint of two neighbouring tiles of a flat surface, whether the two belong to one object or
/// to two objects of the same depth and the same layer.
/// </summary>
/// <remarks>
/// <para>
/// The two surfaces of the amendment's automatic criteria are measured as SPEC-06 §5 asks, by
/// pixel statistics: a flat surface of one colour has exactly one, so any joint that shows at
/// all departs from it.
/// </para>
/// <para>
/// Those two surfaces are made of identical tiles, which makes them necessary but not
/// sufficient: every texel of such a surface carries the same colour, so reading the wrong one
/// changes nothing. <see cref="REQ_SCN_065_the_joint_of_two_objects_never_shows_a_texel_from_across_the_texture"/>
/// is the measure that actually bites — it is the one that fails without the centroid sampling
/// of <see cref="Zax.Render.Gl.SceneShaders.SceneFragment"/>.
/// </para>
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class AntiAliasSeamTests
{
    private const int Black = 0;

    /// <summary>AMD-012: no joint pixel may be more than 2 levels off the surface colour.</summary>
    private const int Tolerance = 2;

    /// <summary>AMD-032 §2: anti-aliasing off, MSAA 4x and MSAA 8x.</summary>
    private static readonly int[] SampleCounts = [1, 4, 8];

    /// <summary>
    /// AMD-058, first automatic criterion: the surface of REQ-SCN-065 — 4 × 4 identical tiles
    /// of one object, lit, casting and receiving the shadow — shows no seam with
    /// <c>video.anti_alias</c> 0, 1 and 2.
    /// </summary>
    [GlFact]
    public void REQ_SCN_065_a_four_by_four_surface_shows_no_seam_under_every_sample_count()
    {
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        foreach (int samples in SampleCounts)
        {
            SceneState scene = OneObject();
            scene.Settings.AntiAliasSamples = samples;
            Capture capture = gl.Render(renderer, scene);

            // The shadow has to be in play: the requirement covers the break in the shadow at a
            // joint as much as the crack in the colour.
            Assert.True(renderer.Statistics.ShadowPass);
            Assert.Equal(samples, renderer.Statistics.AntiAliasSamples);

            (int surface, int worst, int offenders) = Measure(capture);
            Assert.True(
                offenders == 0,
                $"with {samples} sample(s) per pixel, {offenders} pixel(s) of the surface differ "
                    + $"from {Rgb(surface)} by up to {worst} levels");
        }
    }

    /// <summary>
    /// AMD-058, second automatic criterion: the same 4 × 4 surface split over <b>two</b>
    /// neighbouring objects of two columns each, for the three values.
    /// </summary>
    [GlFact]
    public void REQ_SCN_065_a_surface_split_over_two_objects_shows_no_seam_under_every_sample_count()
    {
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        foreach (int samples in SampleCounts)
        {
            SceneState scene = TwoObjects();
            scene.Settings.AntiAliasSamples = samples;
            Capture capture = gl.Render(renderer, scene);

            (int surface, int worst, int offenders) = Measure(capture);
            Assert.True(
                offenders == 0,
                $"with {samples} sample(s) per pixel, {offenders} pixel(s) of the two-object "
                    + $"surface differ from {Rgb(surface)} by up to {worst} levels");
        }
    }

    /// <summary>
    /// The measure AMD-058 is really about: at the joint of two neighbouring objects, no sample
    /// may take its texel from somewhere else in the object's texture.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Each object is two cells wide, so each texture is 16 texels across and the read of
    /// REQ-E3D-563, taken modulo that size, wraps column 16 onto column 0 and column −1 onto
    /// column 15. The <b>first</b> column of the left object and the <b>last</b> column of the
    /// right one are painted a loud colour, every other column the colour of the surface: the
    /// two loud columns therefore sit against the outer silhouettes of the surface, the joint
    /// between the two objects stays <b>flat</b> — the left object shows its column 15 there
    /// and the right object its own column 0, both the surface colour — and a read one texel
    /// too far, in either direction, lands on a loud one. Covering both directions is what
    /// makes the measure independent of which of the two primitives at a joint happens to be
    /// the one whose pixel centre falls outside.
    /// </para>
    /// <para>
    /// With the fragment shader's varyings interpolated at the pixel centre, a pixel of the
    /// joint that a primitive covers only in part has that centre outside the primitive, the
    /// texel coordinate is extrapolated past the edge of the rectangle REQ-E3D-506 gave the
    /// face, and the loud colour bleeds into the resolve. Centroid sampling keeps the
    /// interpolation point inside the primitive, so the coordinate stays in the rectangle.
    /// </para>
    /// </remarks>
    [GlFact]
    public void REQ_SCN_065_the_joint_of_two_objects_never_shows_a_texel_from_across_the_texture()
    {
        using GlFixture gl = GlFixture.Create(1280, 960);
        using SceneRenderer renderer = new(gl.Gl);

        // The colour the surface has to keep is the one the capture without anti-aliasing
        // gives it: AMD-058 asks the anti-aliased capture to differ from that one at the edges
        // of the objects alone, never inside a flat surface.
        int reference = 0;
        foreach (int samples in SampleCounts)
        {
            SceneState scene = TwoObjectsOneLoudColumn();
            scene.Settings.AntiAliasSamples = samples;
            Capture capture = gl.Render(renderer, scene);

            // The middle half of the drawn box: it holds the joint between the two objects and
            // the internal joint of each, and leaves out the loud column, which sits against
            // the left silhouette of the left object.
            (int x0, int y0, int x1, int y1) = Band(capture);
            if (samples == 1)
            {
                reference = Commonest(capture, x0, y0, x1, y1);

                // The window really is on the white surface and not on the loud red column.
                (int red, int green, int blue) = capture.At((x0 + x1) / 2, (y0 + y1) / 2);
                Assert.True(
                    Math.Abs(red - green) <= 8 && Math.Abs(green - blue) <= 8,
                    $"the measured window is not on the white surface but on ({red}, {green}, {blue})");
            }

            int surface = reference;
            int worst = 0;
            int offenders = 0;
            for (int y = y0; y <= y1; y++)
            {
                for (int x = x0; x <= x1; x++)
                {
                    int deviation = Deviation(capture.Packed(x, y), surface);
                    worst = Math.Max(worst, deviation);
                    offenders += deviation > Tolerance ? 1 : 0;
                }
            }

            Assert.True(
                offenders == 0,
                $"with {samples} sample(s) per pixel, {offenders} pixel(s) across the joint "
                    + $"differ from {Rgb(surface)} by up to {worst} levels: a sample took its "
                    + "texel from across the object's texture");
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
            .PatternBlock(SceneFixture.Solid(3), 2, 4, depth: 6, ProfileLayer.L1)
            .PatternBlock(SceneFixture.Solid(2), 2, 4, depth: 6, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 2, rows: 4)
            .Rect(SceneFixture.Solid(2), column: 16, row: 14, columns: 2, rows: 4)
            .Build();

    /// <summary>
    /// Two neighbouring objects of two cells each, repeated over four rows. The left object's
    /// very first texel column and the right object's very last one are loud red, everything
    /// else is the white of the surface: the surface is flat at every joint while a texel read
    /// one column too far, to the left or to the right, wraps onto the red.
    /// </summary>
    private static SceneState TwoObjectsOneLoudColumn()
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
    /// The drawn box inset by a twelfth of its side, so that only the front face of the surface
    /// is sampled and never its silhouette or its side faces.
    /// </summary>
    private static (int X0, int Y0, int X1, int Y1) Interior(Capture capture)
    {
        (int firstX, int lastX) = capture.ColumnSpan(Black);
        (int firstY, int lastY) = capture.RowSpan(Black);
        Assert.True(lastX - firstX > 24 && lastY - firstY > 24, "the surface is too small to measure");

        int insetX = (lastX - firstX + 1) / 12;
        int insetY = (lastY - firstY + 1) / 12;
        return (firstX + insetX, firstY + insetY, lastX - insetX, lastY - insetY);
    }

    /// <summary>
    /// The middle half of the drawn box across, its twelfths cut off down: the joint between the
    /// two objects runs through it and the loud column, which hugs the left silhouette, does not.
    /// </summary>
    private static (int X0, int Y0, int X1, int Y1) Band(Capture capture)
    {
        (int firstX, int lastX) = capture.ColumnSpan(Black);
        (int firstY, int lastY) = capture.RowSpan(Black);
        Assert.True(lastX - firstX > 48 && lastY - firstY > 48, "the surface is too small to measure");

        int width = lastX - firstX + 1;
        int insetY = (lastY - firstY + 1) / 12;
        return (firstX + (width / 4), firstY + insetY, lastX - (width / 4), lastY - insetY);
    }

    /// <summary>
    /// The colour of the surface — the commonest one of the window — then the worst deviation
    /// from it and how many pixels pass the 2 levels of AMD-012.
    /// </summary>
    private static (int Surface, int Worst, int Offenders) Measure(Capture capture)
    {
        (int x0, int y0, int x1, int y1) = Interior(capture);
        int surface = Commonest(capture, x0, y0, x1, y1);

        int worst = 0;
        int offenders = 0;
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                int deviation = Deviation(capture.Packed(x, y), surface);
                worst = Math.Max(worst, deviation);
                offenders += deviation > Tolerance ? 1 : 0;
            }
        }

        return (surface, worst, offenders);
    }

    private static int Commonest(Capture capture, int x0, int y0, int x1, int y1)
    {
        Dictionary<int, int> counts = [];
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                int packed = capture.Packed(x, y);
                counts[packed] = counts.GetValueOrDefault(packed) + 1;
            }
        }

        return counts.MaxBy(entry => entry.Value).Key;
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
