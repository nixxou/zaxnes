using Zax.Engine3D;
using Zax.Profiles;

// AMD-051 §3 point 8: the degradation knobs of the harness are static — a harness run is one
// command in one process — and the tests below read what the command writes to the error
// stream. Both are process-wide, so this assembly runs its tests one at a time, as
// Zax.App.Tests and Zax.Render.Tests already do for the interface language and the GL context.
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Zax.Harness.Tests;

/// <summary>
/// The two degradation knobs AMD-051 §3 point 8 asks the harness for: <c>--atlas-budget</c>
/// (the criterion of REQ-E3D-644) and <c>--delay-mesh</c> (the criterion of REQ-E3D-646).
/// </summary>
public sealed class GeometryLimitTests : IDisposable
{
    /// <summary>A 1 × 1 pattern carries an 8 × 8 texture, so 64 texels (REQ-E3D-426).</summary>
    private const int TexelsPerPattern = 64;

    /// <inheritdoc/>
    public void Dispose()
    {
        GeometryLimits.AtlasBudgetTexels = long.MaxValue;
        GeometryLimits.BuildDelayFrames = 0;
    }

    /// <summary>
    /// REQ-E3D-644, acceptance criterion through the harness: with the total budget cut to C
    /// texels, loading a profile whose objects ask for more than C renders the objects that fit,
    /// reports one error per object that does not, and the next frame is produced. The capture
    /// goes through, so no exception was raised and the replay carried on.
    /// </summary>
    [Fact] // REQ-E3D-644
    public void REQ_E3D_644_atlas_budget_houses_what_fits_and_reports_the_rest()
    {
        GeometryLimits.AtlasBudgetTexels = 2 * TexelsPerPattern;

        (SyntheticScene scene, string[] log) = Capture(FourPatterns());
        GeometryBuilder geometry = Assert.IsType<GeometryBuilder>(scene.Engine.Geometry);

        Assert.Equal(4, geometry.ObjectCount);
        Assert.Equal(2, geometry.UnhousedObjects);
        Assert.Equal(2 * TexelsPerPattern, geometry.HousedTexels);

        // One error per object that was refused, on the harness's log stream.
        Assert.Equal(2, log.Length);
        Assert.All(log, line => Assert.Contains("REQ-E3D-644", line, StringComparison.Ordinal));

        // Four real frames ran after the load: the emulation was never interrupted, and the
        // two objects that fit are the ones the scene can draw.
        Assert.NotNull(scene.Engine.PublishedFrame);
        Assert.Equal(2, CountDrawable(geometry, scene));
    }

    /// <summary>Without the option nothing is refused and nothing is reported.</summary>
    [Fact] // REQ-E3D-644, REQ-E3D-603
    public void REQ_E3D_644_the_default_budget_refuses_nothing()
    {
        (SyntheticScene scene, string[] log) = Capture(FourPatterns());
        GeometryBuilder geometry = Assert.IsType<GeometryBuilder>(scene.Engine.Geometry);

        Assert.Equal(0, geometry.UnhousedObjects);
        Assert.Equal(4 * TexelsPerPattern, geometry.HousedTexels);
        Assert.Empty(log);
        Assert.False(GeometryLimits.IsActive);
        Assert.Equal(4, CountDrawable(geometry, scene));
    }

    /// <summary>
    /// REQ-E3D-646, acceptance criterion through the harness: a construction held back by 10
    /// emulated frames is still waiting after 4 of them, lands with no diagnostic once the ten
    /// have gone by, and the run never throws.
    /// </summary>
    [Fact] // REQ-E3D-646, REQ-E3D-601
    public void REQ_E3D_646_delay_mesh_holds_the_construction_back_without_a_diagnostic()
    {
        GeometryLimits.BuildDelayFrames = 10;

        (SyntheticScene early, string[] earlyLog) = Capture(FourPatterns(), runFrames: 4);
        GeometryBuilder waiting = Assert.IsType<GeometryBuilder>(early.Engine.Geometry);
        Assert.Equal(0, waiting.ObjectCount);
        Assert.Empty(earlyLog);

        (SyntheticScene late, string[] lateLog) = Capture(FourPatterns(), runFrames: 12);
        GeometryBuilder built = Assert.IsType<GeometryBuilder>(late.Engine.Geometry);
        Assert.Equal(4, built.ObjectCount);
        Assert.Equal(0, built.UnhousedObjects);
        Assert.Empty(lateLog);
        Assert.Equal(4, CountDrawable(built, late));
    }

    /// <summary>Both options are documented in the usage text of the <c>render</c> command.</summary>
    [Fact] // AMD-051 §3 point 8
    public void AMD_051_the_two_knobs_are_documented()
    {
        Assert.Contains("--atlas-budget <texels>", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("--delay-mesh <frames>", RenderExport.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// Captures a replay with the error stream collected, which is where the command sends the
    /// diagnostics of REQ-E3D-644 and REQ-E3D-646.
    /// </summary>
    private static (SyntheticScene Scene, string[] Log) Capture(
        GameProfile profile,
        int runFrames = 4)
    {
        TextWriter previous = Console.Error;
        StringWriter collected = new();
        Console.SetError(collected);
        SyntheticScene scene;
        try
        {
            scene = CaptureHarness.Capture(profile, runFrames, title: "amd-051");
        }
        finally
        {
            Console.SetError(previous);
        }

        return (
            scene,
            collected.ToString().Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries));
    }

    /// <summary>3D objects of the profile whose tiles the renderer may draw (REQ-E3D-543).</summary>
    private static int CountDrawable(GeometryBuilder geometry, SyntheticScene scene)
    {
        int count = 0;
        foreach (EnginePattern pattern in scene.Engine.Patterns.Permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                if (geometry.IsReady(instance))
                {
                    count++;
                }
            }
        }

        return count;
    }

    /// <summary>Four single-tile permanent patterns, one 3D object each.</summary>
    private static GameProfile FourPatterns()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "amd-051";
        for (int n = 0; n < 4; n++)
        {
            ProfilePattern pattern = new()
            {
                TileSize = new Vec2I(1, 1),
                Permanent = true,
                Grid = new PatternGrid(
                    1,
                    1,
                    [new ProfileTile(ulong.MaxValue, (ulong)(n + 1), TileFlip.None)]),
            };
            pattern.Variants.Add(new PatternVariant
            {
                Depth = 4,
                Alpha = 1f,
                RenderStyle = VariantRenderStyle.Normal,
            });
            profile.PermanentPatterns.Add(pattern);
        }

        return profile;
    }
}
