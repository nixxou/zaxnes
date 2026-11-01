using System.Globalization;
using System.Text.Json;
using Xunit.Sdk;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-055 — the headless capture the lot is for: <c>render --style</c> with the Enhanced
/// identifier produces a frame that differs from the one the default style produces, so an
/// Enhanced frame can be verified without the application (SPEC-01 REQ-ARCH-031).
/// </summary>
/// <remarks>
/// <para>
/// The two runs are the real command line, through <see cref="RenderExport.Run"/>, and the
/// comparison is read from the <c>render.json</c> each of them writes: the mean luma of the
/// capture and the style block of V2-21. A machine without a GL 3.3 core context skips the test
/// rather than failing it.
/// </para>
/// <para>
/// Each run builds its own hidden GL context, which GLFW only tolerates one at a time; the
/// assembly already runs its tests one at a time (<c>GeometryLimitTests.cs</c>), so nothing
/// more is needed here.
/// </para>
/// </remarks>
public sealed class Amd055StyleCaptureTests
{
    /// <summary>
    /// The voxel grid of REQ-SCN-081 darkens the grooves it cuts into the albedo, so the
    /// Enhanced capture is measurably darker than the Classic one of the same scene — and its
    /// style block says what produced it.
    /// </summary>
    [GlFact] // REQ-SCN-078, REQ-SCN-081, REQ-ARCH-031
    public void The_enhanced_style_captures_a_different_frame()
    {
        using TempDirectory classic = new();
        using TempDirectory enhanced = new();

        Assert.Equal(0, RenderExport.Run([.. Common(classic.Path)]));
        Assert.Equal(
            0,
            RenderExport.Run([.. Common(enhanced.Path), "--style", "4", "--style-param", "spl.voxel_grid=2"]));

        Capture before = Read(classic.Path);
        Capture after = Read(enhanced.Path);

        Assert.Equal(0, before.VisualStyle);
        Assert.Equal(4, after.VisualStyle);
        Assert.Equal(0, before.VoxelGrid);
        Assert.Equal(2, after.VoxelGrid);

        // The grooves cut 38 % of the albedo out of a 3.5 % band of every cell and brighten the
        // 12.5 % band beside it by 10 % at most (REQ-SCN-081), so the frame as a whole loses a
        // little luma. A thousandth of a level is far above the noise of a deterministic
        // renderer and far below what the effect actually takes off.
        Assert.True(
            after.MeanLuma < before.MeanLuma - 0.001,
            $"the Enhanced capture should be darker: {after.MeanLuma} against {before.MeanLuma}");
    }

    /// <summary>
    /// REQ-SCN-078, the alias: <c>--style 1</c> and <c>--style 4</c> select the same style, so
    /// the two captures agree, down to the 4 the style block reads back.
    /// </summary>
    [GlFact] // REQ-SCN-078
    public void The_alias_one_captures_the_same_frame_as_four()
    {
        using TempDirectory four = new();
        using TempDirectory one = new();

        Assert.Equal(
            0,
            RenderExport.Run([.. Common(four.Path), "--style", "4", "--style-param", "spl.voxel_grid=2"]));
        Assert.Equal(
            0,
            RenderExport.Run([.. Common(one.Path), "--style", "1", "--style-param", "spl.voxel_grid=2"]));

        Capture a = Read(four.Path);
        Capture b = Read(one.Path);

        Assert.Equal(4, b.VisualStyle);
        Assert.Equal(a.MeanLuma, b.MeanLuma);
        Assert.Equal(a.DistinctColors, b.DistinctColors);
    }

    /// <summary>
    /// REQ-SCN-080: <c>--style 4</c> with every effect left off captures what Classic captures,
    /// which is what makes the test above a measurement of the effect and not of the style.
    /// </summary>
    [GlFact] // REQ-SCN-080
    public void Enhanced_with_every_effect_off_captures_the_classic_frame()
    {
        using TempDirectory classic = new();
        using TempDirectory enhanced = new();

        Assert.Equal(0, RenderExport.Run([.. Common(classic.Path)]));
        Assert.Equal(0, RenderExport.Run([.. Common(enhanced.Path), "--style", "4"]));

        Capture before = Read(classic.Path);
        Capture after = Read(enhanced.Path);

        Assert.Equal(4, after.VisualStyle);
        Assert.Equal(before.MeanLuma, after.MeanLuma);
        Assert.Equal(before.DistinctColors, after.DistinctColors);
    }

    /// <summary>
    /// The scene and the capture size the two runs share: one synthetic scene, small, with no
    /// anti-aliasing so that the comparison is of the shading and nothing else.
    /// </summary>
    private static string[] Common(string output) =>
    [
        "--out", output,
        "--frames", "synthetic:framing",
        "--size", "320x240",
        "--anti-alias", "0",
    ];

    /// <summary>Reads the one record of a <c>render.json</c> the command just wrote.</summary>
    private static Capture Read(string output)
    {
        string path = Path.Combine(output, "render.json");
        Assert.True(File.Exists(path), path);

        using JsonDocument document = JsonDocument.Parse(File.ReadAllBytes(path));
        JsonElement record = Assert.Single(document.RootElement.EnumerateArray().ToArray());
        JsonElement style = record.GetProperty("style");
        return new Capture(
            record.GetProperty("meanLuma").GetDouble(),
            record.GetProperty("distinctColors").GetInt32(),
            style.GetProperty("visualStyle").GetInt32(),
            style.GetProperty("spl.voxel_grid").GetInt32());
    }

    /// <summary>What a capture's <c>render.json</c> record says about it.</summary>
    private readonly record struct Capture(
        double MeanLuma,
        int DistinctColors,
        int VisualStyle,
        int VoxelGrid);

    /// <summary>A capture directory removed with its files when the test ends.</summary>
    private sealed class TempDirectory : IDisposable
    {
        public TempDirectory()
            => Path = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                string.Create(CultureInfo.InvariantCulture, $"zax-v221-{Guid.NewGuid():N}"));

        /// <summary>The directory, which the command creates itself.</summary>
        public string Path { get; }

        public void Dispose()
        {
            try
            {
                if (Directory.Exists(Path))
                {
                    Directory.Delete(Path, recursive: true);
                }
            }
            catch (IOException)
            {
                // A leftover capture in the temporary folder is not worth failing a test for.
            }
        }
    }
}

/// <summary>
/// A fact skipped, rather than failed, when the machine cannot give the GL 3.3 core context an
/// offscreen capture needs.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class GlFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public GlFactAttribute()
    {
        if (GlAvailability.Reason is { } reason)
        {
            Skip = reason;
        }
    }
}

/// <summary>Probes once whether a hidden GL 3.3 core context can be created in this process.</summary>
internal static class GlAvailability
{
    /// <summary>Why no context is available, or null when one is.</summary>
    public static string? Reason { get; } = Probe();

    private static string? Probe()
    {
        try
        {
            using OffscreenContext context = OffscreenContext.Create(16, 16);
            return null;
        }
        catch (Exception ex)
        {
            return $"no OpenGL 3.3 core context on this machine: {ex.Message}";
        }
    }
}
