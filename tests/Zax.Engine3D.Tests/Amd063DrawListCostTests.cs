using System.Diagnostics;
using System.Globalization;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// What AMD-063 REQ-SCN-087 costs the draw list: the per-frame background cell map is built on
/// every frame, drawn tiles or not, and a transparent full tile looks its four neighbours up.
/// </summary>
/// <remarks>
/// <para>
/// The frame is the worst case the requirement can meet: the whole background covered with
/// transparent full tiles, so every one of the 1 023 drawn background tiles is a candidate and
/// every cell of the map is claimed, plus 64 sprites that are candidates for nothing.
/// </para>
/// <para>
/// Only <see cref="ConversionEngine.BuildDrawList(ConversionFrame)"/> is timed: the conversion
/// itself is unchanged, and REQ-E3D-360 already measures it (<c>CorpusTests</c>). The assertion
/// is on the <b>median</b> against the hard frame limit of 16.6 ms, which catches an
/// order-of-magnitude regression and nothing else: a tighter bound on a mean would only measure
/// what else the machine is doing — the cost here is dominated by the thousand
/// <see cref="DrawTile"/> the list allocates, so the tail is the garbage collector's. The point
/// of the test is the numbers it prints, which the progress file of the lot records before and
/// after the change.
/// </para>
/// </remarks>
public sealed class Amd063DrawListCostTests
{
    private const int Iterations = 300;

    /// <summary>The hard per-frame limit of SPEC-07 §2, 60 frames per second in NTSC.</summary>
    private const double FrameBudgetMs = 16.6d;

    /// <summary>
    /// Building the draw list of a full transparent frame stays inside one frame, and the
    /// numbers are printed so that a regression is visible.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, REQ-E3D-641
    public void Building_the_draw_list_of_a_full_transparent_frame_stays_cheap()
    {
        Assert.True(Measure(alpha: 0.7f, out string transparent) <= FrameBudgetMs, transparent);
        Assert.True(Measure(alpha: 1f, out string opaque) <= FrameBudgetMs, opaque);

        Console.WriteLine(transparent);
        Console.WriteLine(opaque);
    }

    /// <summary>
    /// Times <see cref="Iterations"/> draw lists of one converted frame and returns the median
    /// in milliseconds, with a readable report.
    /// </summary>
    private static double Measure(float alpha, out string report)
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.LoadProfile(Profile(alpha));

        Batch batch = new();
        batch.Palette(0, 0x0F);
        for (int row = 0; row < 31; row++)
        {
            for (int column = 0; column < 33; column++)
            {
                batch.BgCell(Img.Full((row + column) % 4), column, row);
            }
        }

        for (int sprite = 0; sprite < 64; sprite++)
        {
            batch.Sprite(Img.Full(8), (sprite * 3) % 240, 8 + ((sprite * 5) % 220));
        }

        ConversionFrame frame = batch.ConvertAndPublish(engine);

        // Warm up: the mesh variants of REQ-SCN-087 are memoised on first use.
        for (int n = 0; n < 20; n++)
        {
            engine.BuildDrawList(frame);
        }

        int tiles = engine.BuildDrawList(frame).Tiles.Count;

        double[] samples = new double[Iterations];
        Stopwatch watch = new();
        for (int n = 0; n < Iterations; n++)
        {
            watch.Restart();
            engine.BuildDrawList(frame);
            watch.Stop();
            samples[n] = watch.Elapsed.TotalMilliseconds;
        }

        Array.Sort(samples);
        double median = samples[Iterations / 2];
        report = string.Format(
            CultureInfo.InvariantCulture,
            "REQ-SCN-087 draw list, alpha {0}, {1} drawn tiles, {2} builds: "
                + "median {3:F4} ms, mean {4:F4} ms, p99 {5:F4} ms, max {6:F4} ms",
            alpha,
            tiles,
            Iterations,
            median,
            samples.Average(),
            samples[(int)(Iterations * 0.99)],
            samples[^1]);
        return median;
    }

    /// <summary>Four one-cell patterns of full tiles, all at the same opacity.</summary>
    private static GameProfile Profile(float alpha)
    {
        ProfileBuilder builder = new();
        for (int id = 0; id < 4; id++)
        {
            builder.Pattern(1, 1, [Img.Full(id)], depth: 4, alpha: alpha);
        }

        builder.Pattern(1, 1, [Img.Full(8)], depth: 4, alpha: alpha);
        return builder.Build();
    }
}
