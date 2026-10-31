using Zax.Engine3D;

namespace Zax.Harness;

/// <summary>
/// The degradation knobs AMD-051 §3 point 8 asks the harness for: the total texture budget of
/// SPEC-04B REQ-E3D-564 (<c>--atlas-budget</c>, the criterion of REQ-E3D-644) and the delay a
/// construction is held back by (<c>--delay-mesh</c>, the criterion of REQ-E3D-646).
/// </summary>
/// <remarks>
/// Held statically because a harness run is one command in one process: the options are parsed
/// once, before any scene is built, and every engine the run creates reads the same values. The
/// diagnostics of REQ-E3D-644 and REQ-E3D-646 go to the standard error stream, which is the
/// harness's log, in English like every other line it prints (AMD-035 §2).
/// </remarks>
internal static class GeometryLimits
{
    /// <summary>
    /// Total texture occupancy allowed, in texels. Unbounded by default, so a run that does not
    /// pass <c>--atlas-budget</c> behaves exactly as before.
    /// </summary>
    public static long AtlasBudgetTexels { get; set; } = long.MaxValue;

    /// <summary>Emulated frames the construction of every 3D object is held back by.</summary>
    public static int BuildDelayFrames { get; set; }

    /// <summary>Whether a run asked for a degradation, for the report.</summary>
    public static bool IsActive => AtlasBudgetTexels != long.MaxValue || BuildDelayFrames > 0;

    /// <summary>Attaches a geometry builder to <paramref name="engine"/> with these limits.</summary>
    /// <param name="engine">The conversion engine.</param>
    /// <returns>The builder, so the caller can drive its transfers as before.</returns>
    public static GeometryBuilder Attach(ConversionEngine engine)
    {
        ArgumentNullException.ThrowIfNull(engine);
        return Apply(engine.UseGeometry());
    }

    /// <summary>Puts these limits on <paramref name="builder"/>.</summary>
    /// <param name="builder">The builder to configure.</param>
    /// <returns><paramref name="builder"/>.</returns>
    public static GeometryBuilder Apply(GeometryBuilder builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        builder.TextureTexelBudget = AtlasBudgetTexels;
        builder.BuildDelayFrames = BuildDelayFrames;
        builder.Diagnostics = static line => Console.Error.WriteLine($"geometry: {line}");
        return builder;
    }
}
