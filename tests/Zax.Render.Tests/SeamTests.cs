using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The two artefacts AMD-012 is about: the faces that close each tile mesh on itself
/// (SPEC-04B REQ-E3D-505) must not show between the tiles of a translucent object
/// (REQ-SCN-040, Q-020 revised), nor at the joints of a plain surface (REQ-SCN-065).
/// </summary>
/// <remarks>
/// Both are measured by pixel statistics on an offscreen capture, as SPEC-06 §5 asks: the
/// surface of a flat, uniformly lit plane is one single colour, so any joint that shows at all
/// turns up as a column or a row that departs from it.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class SeamTests
{
    private const int Black = 0;

    /// <summary>AMD-012: no joint pixel may be more than 2 levels off the surface colour.</summary>
    private const int Tolerance = 2;

    [GlFact]
    public void REQ_SCN_065_a_four_by_four_surface_shows_no_seam_at_its_joints()
    {
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = Surface();

        // The criterion is stated for the default view. The seam the users reported comes from
        // the shadow map, so the map resolution is what decides whether it shows: one texel of
        // the map has to stay smaller than the bias for the inner walls of the tiles not to
        // shadow their neighbours. The default of 4096 texels (AMD-034 §2) is checked first,
        // then the two fallback sizes and coarser maps still, which is where the artefact used
        // to appear.
        foreach (int mapSize in new[] { 4096, 2048, 1024, 512, 256, 128 })
        {
            scene.Settings.ShadowMapSize = mapSize;
            Capture capture = gl.Render(renderer, scene);
            Assert.True(renderer.Statistics.ShadowPass);

            (int surface, int worst, int offenders) = Measure(capture);
            Assert.True(
                offenders == 0,
                $"with a {mapSize}-texel shadow map, {offenders} pixel(s) of the surface differ "
                    + $"from {Rgb(surface)} by up to {worst} levels");
        }
    }

    [GlFact]
    public void REQ_SCN_065_a_deep_surface_shows_no_seam_and_no_fin_from_any_angle()
    {
        // FIX-2 (Q-067): the ground of Zelda II and the track of Excitebike are tiles of
        // depth 24 to 56 whose walls, kept by REQ-E3D-501, used to show as a grid at every
        // joint and as dark fins sticking out of the silhouette. The surface is flat and of
        // one colour, so the drawn box has to stay a plain rectangle of that colour whatever
        // the angle.
        using GlFixture gl = GlFixture.Create(800, 600);
        using SceneRenderer renderer = new(gl.Gl);

        foreach (int depth in new[] { 8, 24, 56 })
        {
            SceneState scene = new SceneFixture()
                .Background(0x0F)
                .Palette(3, 0x30)
                .Pattern(SceneFixture.Solid(3), depth, ProfileLayer.L1, fullWarp: true)
                .Rect(SceneFixture.Solid(3), column: 12, row: 12, columns: 6, rows: 6)
                .Build();

            // Head-on views, pitched up and down: the ones the users reported, and the ones
            // where "a line across the surface" and "the lower edge" mean one face each.
            foreach ((float rx, float ry) in new[] { (0.81f, 0.81f), (14f, 0.81f), (20f, 0.81f), (-14f, 0.81f) })
            {
                scene.Camera.Current.Angles = new Vector3(rx, ry, 0f);
                scene.Camera.SnapToSetpoints();
                Capture capture = gl.Render(renderer, scene);

                // The grid of walls crossed the front face: a line across it used to change
                // colour at every joint. On a surface of one colour it has to stay single.
                HashSet<int> line = FrontFaceLine(capture);
                Assert.True(
                    line.Count == 1,
                    $"depth {depth} seen from ({rx}, {ry}): a line across the surface shows "
                        + $"{line.Count} colours — the joints show");

                // The fins of the walls used to hang a tile below the silhouette, one per
                // tile column: the lower edge of a flat surface became a sawtooth. The edge of
                // a box moves by less than a pixel from one column to the next.
                int jumps = SawtoothJumps(capture);
                Assert.True(
                    jumps == 0,
                    $"depth {depth} seen from ({rx}, {ry}): the lower edge jumps at {jumps} "
                        + "column(s) — walls stick out of the silhouette");
            }
        }
    }

    [GlFact]
    public void REQ_SCN_065_the_joints_stay_invisible_when_the_surface_is_in_shadow()
    {
        // The same surface, with an invisible caster above it (REQ-SCN-041 style 2) so that part
        // of it is in shadow while nothing else is drawn: the shadowed part must be as flat as
        // the lit one (REQ-SCN-031, REQ-SCN-032).
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Palette(1, 0x16)
            .Pattern(SceneFixture.Solid(3), depth: 6, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(1), depth: 8, ProfileLayer.L1, style: VariantRenderStyle.ShadowOnly)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 4, rows: 4)
            .Rect(SceneFixture.Solid(1), column: 12, row: 12, columns: 2, rows: 2);

        // The surface has to cast as well as receive, so that the walls closing its own tiles go
        // into the shadow map: that is what used to darken the joints. L2 only receives by
        // default (REQ-SCN-033), and Lua may turn its casting on.
        SceneState scene = fixture.Build(settings => settings.Layer(ZLayer.L2).CastShadow = true);
        scene.Settings.ShadowMapSize = 256;

        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);
        Capture capture = gl.Render(renderer, scene);

        // Both shades of the surface are flat; only the two of them plus the block are allowed.
        (int lit, int shadowed) = TwoShades(capture);
        Assert.NotEqual(lit, shadowed);

        int strays = 0;
        (int x0, int y0, int x1, int y1) = Interior(capture);
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                int packed = capture.Packed(x, y);
                if (Deviation(packed, lit) > Tolerance && Deviation(packed, shadowed) > Tolerance)
                {
                    strays++;
                }
            }
        }

        Assert.True(
            strays == 0,
            $"{strays} pixel(s) are neither the lit surface {Rgb(lit)} nor its shadow {Rgb(shadowed)}");
    }

    [GlFact]
    public void REQ_SCN_040_a_translucent_multi_tile_object_shows_no_internal_face()
    {
        // AMD-012 (Q-020 revised): every tile of one object writes its depth before the object's
        // colour pass, so the walls that close each tile against its neighbours never blend.
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = Translucent();

        // Seen from an angle, so that the walls between the tiles have a real width on screen —
        // head-on they are edge-on and the artefact hides itself — and from the four corners,
        // because a wall only shows on the side of the neighbour that is drawn after it.
        foreach ((float rx, float ry) in new[] { (8f, 8f), (8f, -8f), (-8f, 8f), (-8f, -8f) })
        {
            scene.Camera.Current.Angles = new Vector3(rx, ry, 0f);
            scene.Camera.SnapToSetpoints();
            Capture capture = gl.Render(renderer, scene);

            // One object of sixteen tiles: the grouping of AMD-012 has to see them as one.
            Assert.Equal(16, renderer.Statistics.TransparentTiles);
            Assert.Equal(1, renderer.Statistics.TransparentObjects);

            // Up to two pixels are tolerated: where a wall meets the face it closes against,
            // the two are coincident to the last bit of the depth buffer, so the colour pass —
            // which keeps the fragments whose depth equals the one the depth pass stored — keeps
            // them both and blends that pixel twice (Q-032). A slope-scaled polygon offset would
            // separate them, but on a scene of blocks it costs more speckle than it saves. The
            // artefact AMD-012 is about covers hundreds of pixels here.
            (int surface, int worst, int offenders) = Measure(capture, inset: 6);
            Assert.True(
                offenders <= 2,
                $"seen from ({rx}, {ry}), {offenders} pixel(s) inside the translucent object "
                    + $"differ from {Rgb(surface)} by up to {worst} levels: internal faces show");
        }
    }

    [GlFact]
    public void REQ_SCN_040_transparent_objects_are_sorted_by_the_centre_of_the_whole_object()
    {
        // Two translucent objects, a red one and a blue one, moved on top of each other. The
        // nearer one is blended last, so its colour weighs twice as much as the farther one's;
        // swapping their depths has to swap which colour dominates (AMD-012: the sort key is the
        // centre of the transformed bounding box of the whole object).
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)
            .Palette(2, 0x12)
            .PatternBlock(SceneFixture.Solid(1), 3, 3, depth: 2, ProfileLayer.L1, alpha: 0.5f)
            .PatternBlock(SceneFixture.Solid(2), 3, 3, depth: 2, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(1), column: 12, row: 13, columns: 3, rows: 3)
            .Rect(SceneFixture.Solid(2), column: 18, row: 13, columns: 3, rows: 3);

        SceneState scene = fixture.Build();
        ConversionEngine engine = fixture.Engine!;
        List<FrameInstance> objects = Instances(scene);
        Assert.Equal(2, objects.Count);

        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        // Slide the red object the six tiles that separate them, once in front of the blue one
        // and once behind it. +z goes away from the eye (REQ-SCN-001).
        float middle = objects[0].Center.Z;
        Capture inFront = Moved(gl, renderer, scene, engine, objects[0], 48f, middle - 40f);
        Assert.Equal(2, renderer.Statistics.TransparentObjects);
        Capture behind = Moved(gl, renderer, scene, engine, objects[0], 48f, middle + 40f);

        // The two objects end up exactly on top of each other, so the drawn box is their overlap.
        (int x0, int y0, int x1, int y1) = Interior(inFront);
        (int frontR, _, int frontB) = inFront.At((x0 + x1) / 2, (y0 + y1) / 2);
        (int backR, _, int backB) = behind.At((x0 + x1) / 2, (y0 + y1) / 2);

        Assert.True(
            frontR > backR && frontB < backB,
            $"the sort did not follow the depth: in front ({frontR}, _, {frontB}), "
                + $"behind ({backR}, _, {backB})");
    }

    /// <summary>
    /// Shifts one instance by <paramref name="offsetX"/> units across and puts its depth centre
    /// at <paramref name="centreZ"/>, rebuilds the draw list and captures the result. The tile
    /// centres of REQ-E3D-405 take the instance's offset and depth centre, while its anchor
    /// cancels out of REQ-E3D-406 at scale 1 without rotation.
    /// </summary>
    private static Capture Moved(
        GlFixture gl,
        SceneRenderer renderer,
        SceneState scene,
        ConversionEngine engine,
        FrameInstance instance,
        float offsetX,
        float centreZ)
    {
        instance.Offset = new Vec3(offsetX, 0f, 0f);
        instance.Center = instance.Center with { Z = centreZ };
        scene.DrawList = engine.BuildDrawList();
        return gl.Render(renderer, scene);
    }

    // ------------------------------------------------------------------ scenes

    /// <summary>
    /// The surface of AMD-012: 4 × 4 identical tiles, lit, casting and receiving shadows.
    /// </summary>
    private static SceneState Surface()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 6, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 4, rows: 4)
            .Build();

    /// <summary>
    /// One translucent object of 4 × 4 tiles, alone over the background: the only thing in the
    /// capture, so the interior measured is the object's own front face.
    /// </summary>
    private static SceneState Translucent()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .PatternBlock(SceneFixture.Solid(3), 4, 4, depth: 24, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 4, rows: 4)
            .Build();

    private static List<FrameInstance> Instances(SceneState scene)
    {
        List<FrameInstance> instances = [];
        foreach (DrawTile tile in scene.DrawList!.Tiles)
        {
            if (tile.Material == MaterialClass.Transparent && !instances.Contains(tile.Instance))
            {
                instances.Add(tile.Instance);
            }
        }

        return instances;
    }

    // ------------------------------------------------------------------ measures

    /// <summary>
    /// The drawn box of the capture, inset by a twelfth of its side so that only the front face
    /// of the surface is sampled, never its silhouette or its side faces. A 4 × 4 surface keeps
    /// its three joints in each direction, at a quarter, a half and three quarters of the box.
    /// </summary>
    private static (int X0, int Y0, int X1, int Y1) Interior(Capture capture, int inset = 12)
    {
        (int firstX, int lastX) = capture.ColumnSpan(Black);
        (int firstY, int lastY) = capture.RowSpan(Black);
        Assert.True(lastX - firstX > 24 && lastY - firstY > 24, "the surface is too small to measure");

        int insetX = (lastX - firstX + 1) / inset;
        int insetY = (lastY - firstY + 1) / inset;
        return (firstX + insetX, firstY + insetY, lastX - insetX, lastY - insetY);
    }

    /// <summary>
    /// The colour of the surface — the commonest one of the interior — then the worst deviation
    /// from it and how many pixels pass the 2 levels of AMD-012.
    /// </summary>
    private static (int Surface, int Worst, int Offenders) Measure(Capture capture, int inset = 12)
    {
        (int x0, int y0, int x1, int y1) = Interior(capture, inset);
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

    /// <summary>
    /// The colours a horizontal line across the lower part of the drawn shape meets, away from
    /// its edges. A flat surface of one colour gives exactly one; the walls of Q-067 used to
    /// add a darker one at every joint.
    /// </summary>
    private static HashSet<int> FrontFaceLine(Capture capture)
    {
        (int firstX, int lastX) = capture.ColumnSpan(Black);
        (int firstY, int lastY) = capture.RowSpan(Black);
        int marginX = Math.Max(2, (lastX - firstX + 1) / 5);
        int y = lastY - Math.Max(2, (lastY - firstY + 1) / 5);

        HashSet<int> colors = [];
        for (int x = firstX + marginX; x <= lastX - marginX; x++)
        {
            colors.Add(capture.Packed(x, y));
        }

        return colors;
    }

    /// <summary>
    /// How many times the lower edge of the drawn shape jumps by more than two pixels from one
    /// column to the next. The silhouette of a box is a straight line in each of its segments,
    /// so a jump means something hangs below it — the fin of a tile wall (Q-067).
    /// </summary>
    private static int SawtoothJumps(Capture capture)
    {
        // The slanted left and right sides of the box end in a staircase of their own, so the
        // edge is only read over the middle of the shape, where a flat surface is flat.
        (int spanStart, int spanEnd) = capture.ColumnSpan(Black);
        int margin = Math.Max(2, (spanEnd - spanStart + 1) / 10);
        int firstX = spanStart + margin;
        int lastX = spanEnd - margin;
        int[] bottom = new int[Math.Max(0, lastX - firstX + 1)];
        for (int x = firstX; x <= lastX; x++)
        {
            int lowest = -1;
            for (int y = capture.Height - 1; y >= 0; y--)
            {
                if (capture.Packed(x, y) != Black)
                {
                    lowest = y;
                    break;
                }
            }

            bottom[x - firstX] = lowest;
        }

        int jumps = 0;
        for (int n = 1; n < bottom.Length; n++)
        {
            if (bottom[n] >= 0 && bottom[n - 1] >= 0 && Math.Abs(bottom[n] - bottom[n - 1]) > 2)
            {
                jumps++;
            }
        }

        return jumps;
    }

    /// <summary>The two commonest colours of the interior: the lit surface and its shadow.</summary>
    private static (int Lit, int Shadowed) TwoShades(Capture capture)
    {
        (int x0, int y0, int x1, int y1) = Interior(capture);
        Dictionary<int, int> counts = Histogram(capture, x0, y0, x1, y1);
        List<int> ranked = [.. counts.OrderByDescending(entry => entry.Value).Select(entry => entry.Key)];
        Assert.True(ranked.Count >= 2, "the surface is not partly in shadow");

        // The lit shade is the lighter of the two.
        int first = ranked[0];
        int second = ranked[1];
        return Luma(first) >= Luma(second) ? (first, second) : (second, first);
    }

    private static Dictionary<int, int> Histogram(Capture capture, int x0, int y0, int x1, int y1)
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

        return counts;
    }

    private static int Commonest(Capture capture, int x0, int y0, int x1, int y1)
        => Histogram(capture, x0, y0, x1, y1).MaxBy(entry => entry.Value).Key;

    private static int Deviation(int left, int right)
        => Math.Max(
            Math.Abs(((left >> 16) & 0xFF) - ((right >> 16) & 0xFF)),
            Math.Max(
                Math.Abs(((left >> 8) & 0xFF) - ((right >> 8) & 0xFF)),
                Math.Abs((left & 0xFF) - (right & 0xFF))));

    private static double Luma(int packed)
        => (0.299 * ((packed >> 16) & 0xFF))
            + (0.587 * ((packed >> 8) & 0xFF))
            + (0.114 * (packed & 0xFF));

    private static string Rgb(int packed)
        => $"({(packed >> 16) & 0xFF}, {(packed >> 8) & 0xFF}, {packed & 0xFF})";
}
