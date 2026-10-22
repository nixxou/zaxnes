using System.Diagnostics;
using System.Globalization;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// The pipeline run against the 101 built-in profiles of <c>assets-neutral/profiles</c>
/// (REQ-E3D-060) and the performance budget of REQ-E3D-360.
/// </summary>
/// <remarks>
/// No ROM is needed: the batches are synthetic, which is what DEC-R2-T7 asks of an acceptance
/// criterion of the clean side.
/// </remarks>
[Collection(AssetCollection.Name)]
public sealed class CorpusTests
{
    private static readonly string[] SmokeProfiles =
    [
        "empty.bytes",
        "Contra.bytes",
        "Final_Fantasy_USA.bytes",
        "Jackal (U).bytes",
        "Castlevania (U) (PRG 1).bytes",
        "Mega Man 2.bytes",
        "punchout.bytes",
        "Felix the Cat.bytes",
        "Adventures of Lolo (U).bytes",
        "DQ3_DW3.bytes",
    ];

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_E3D_060_every_built_in_profile_registers_all_its_patterns()
    {
        List<string> problems = [];
        int totalPatterns = 0;
        int totalInstances = 0;

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            ConversionEngine engine = new();
            engine.LoadProfile(profile);

            int expected = profile.PermanentPatterns.Count + profile.TemporaryPatterns.Count;
            int registered = engine.Patterns.Permanent.Count + engine.Patterns.Temporary.Count;
            if (registered != expected)
            {
                problems.Add($"{Path.GetFileName(path)}: {registered} registered for {expected} in the file");
            }

            if (engine.Patterns.Permanent.Count != profile.PermanentPatterns.Count)
            {
                problems.Add($"{Path.GetFileName(path)}: two permanent patterns share a key");
            }

            totalPatterns += registered;
            foreach (EnginePattern pattern in engine.Patterns.Permanent)
            {
                totalInstances += pattern.Instances.Count;
            }

            foreach (EnginePattern pattern in engine.Patterns.Temporary)
            {
                totalInstances += pattern.Instances.Count;
            }
        }

        Assert.True(problems.Count == 0, string.Join("\n", problems));

        // SPEC-04B annex A: 67 832 patterns and 74 389 3D objects in the 101 profiles.
        Assert.Equal(67832, totalPatterns);
        Assert.Equal(74389, totalInstances);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_E3D_001_the_pipeline_runs_on_synthetic_frames_with_built_in_profiles()
    {
        foreach (string name in SmokeProfiles)
        {
            GameProfile profile = NeutralAssets.Load(name);
            ConversionEngine engine = new();
            RecordingGeometryBuilder geometry = new();
            engine.Geometry = geometry;
            engine.TwoDimensionalMode = string.Equals(name, "empty.bytes", StringComparison.Ordinal);
            engine.LoadProfile(profile);

            // Real tile images of the profile, so that the recognition has something to match.
            List<TileImageKey> images = [];
            foreach (ProfilePattern pattern in profile.PermanentPatterns)
            {
                if (pattern.Grid is not { } grid)
                {
                    continue;
                }

                foreach (ProfileTile? tile in grid.Cells)
                {
                    if (tile is not null && images.Count < 64)
                    {
                        images.Add(new TileImageKey(
                            tile.LowPlane,
                            tile.HighPlane,
                            tile.FlipX,
                            tile.FlipY));
                    }
                }

                if (images.Count >= 64)
                {
                    break;
                }
            }

            if (images.Count == 0)
            {
                images.Add(Img.A);
            }

            int drawnTiles = 0;
            for (int counter = 1; counter <= 4; counter++)
            {
                Batch batch = new();
                batch.Mask(8, 0).Palette(0, 0x0F).Palette(1, 0x16).Palette(17, 0x27);
                int n = counter;
                for (int row = 0; row < 31; row++)
                {
                    for (int column = 0; column < 33; column++)
                    {
                        batch.BgCell(images[n++ % images.Count], column, row, (row + column) & 3);
                    }
                }

                for (int i = 0; i < 64; i++)
                {
                    batch.Sprite(images[(n + i) % images.Count], (i * 3) % 240, 8 + ((i * 5) % 220), i & 3);
                }

                ConversionFrame frame = batch.Convert(engine, counter);
                engine.Publish();

                Assert.Equal(counter, frame.Counter);
                Assert.Equal(0, frame.ScriptErrorCount);
                Assert.True(frame.BackgroundShapeCount <= frame.Shapes.Count);
                foreach (FrameShape shape in frame.Shapes)
                {
                    Assert.NotNull(shape.ReferencePattern);
                    Assert.NotEmpty(shape.Tiles);
                    Assert.Equal(shape.ReferencePattern!.Instances.Count, shape.Instances.Count);
                }

                drawnTiles += engine.BuildDrawList(frame).Tiles.Count;
            }

            Assert.True(drawnTiles > 0, $"{name}: nothing was drawn");

            // The learned state of a real profile round trips as well.
            byte[] state = EngineStateCodec.Write(engine);
            ConversionEngine restored = new();
            restored.LoadProfile(NeutralAssets.Load(name));
            EngineStateCodec.Read(restored, state);
            Assert.Equal(
                Convert.ToHexString(state),
                Convert.ToHexString(EngineStateCodec.Write(restored)));
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_E3D_060_the_built_in_profiles_carry_usable_3D_parameters()
    {
        // A sanity check on the hand-over from SPEC-03: an alpha of 0 everywhere would make
        // every object transparent, which would point at a reader problem rather than a
        // pipeline one.
        GameProfile profile = NeutralAssets.Load("Contra.bytes");
        ConversionEngine engine = new();
        engine.LoadProfile(profile);

        int opaque = 0;
        int total = 0;
        foreach (EnginePattern pattern in engine.Patterns.Permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                total++;
                if (instance.Alpha >= 1f)
                {
                    opaque++;
                }
            }
        }

        Assert.True(total > 0);
        Assert.True(opaque > total / 2, $"only {opaque} of {total} objects are opaque");
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_E3D_360_conversion_budget_benchmark_report()
    {
        // A report, not a gate: the budget is a project target measured on the reference
        // machine (DEC-04-Q06, DEC-R2-12-2). The test only fails if a frame busts the frame
        // period of 16.6 ms, which REQ-E3D-360 states as a MUST.
        const int Iterations = 1000;

        GameProfile profile = NeutralAssets.Load("Final_Fantasy_USA.bytes");
        ConversionEngine engine = new();
        engine.LoadProfile(profile);

        int patterns = engine.Patterns.Permanent.Count;
        List<TileImageKey> images = [];
        foreach (EnginePattern pattern in engine.Patterns.Permanent)
        {
            foreach (TileImage image in pattern.Images)
            {
                if (images.Count < 64)
                {
                    images.Add(image.Key);
                }
            }

            if (images.Count >= 64)
            {
                break;
            }
        }

        Batch batch = new();
        batch.Mask(8, 0).Palette(0, 0x0F);
        int n = 0;
        for (int row = 0; row < 31; row++)
        {
            for (int column = 0; column < 33; column++)
            {
                batch.BgCell(images[n++ % images.Count], column, row, (row + column) & 3);
            }
        }

        for (int i = 0; i < 64; i++)
        {
            batch.Sprite(images[i % images.Count], (i * 3) % 240, 8 + ((i * 5) % 220), i & 3);
        }

        // Warm up: the first frames learn patterns and fill the pools.
        for (int i = 0; i < 50; i++)
        {
            batch.Convert(engine, i);
        }

        double[] samples = new double[Iterations];
        Stopwatch watch = new();
        for (int i = 0; i < Iterations; i++)
        {
            watch.Restart();
            batch.Convert(engine, 100 + i);
            watch.Stop();
            samples[i] = watch.Elapsed.TotalMilliseconds;
        }

        Array.Sort(samples);
        double mean = samples.Average();
        double p99 = samples[(int)(Iterations * 0.99)];
        double worst = samples[^1];

        string report = string.Format(
            CultureInfo.InvariantCulture,
            "REQ-E3D-360 on {0} permanent patterns, full frame (33x31 background, 64 sprites), "
                + "{1} conversions: mean {2:F3} ms, p99 {3:F3} ms, max {4:F3} ms "
                + "(target: mean <= 4 ms, p99 <= 8 ms; hard limit 16.6 ms).",
            patterns,
            Iterations,
            mean,
            p99,
            worst);
        Console.WriteLine(report);
        File.WriteAllText(
            Path.Combine(AppContext.BaseDirectory, "REQ-E3D-360-benchmark.txt"),
            report);

        Assert.True(worst < 16.6, report);
    }
}
