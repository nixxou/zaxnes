using Xunit.Sdk;

namespace Zax.Nes.Tests;

/// <summary>
/// A <see cref="FactAttribute"/> that reports the test as skipped, with a message naming
/// the missing files, unless every public test ROM it lists is present under
/// <c>impl/tests/roms/</c>.
/// </summary>
/// <remarks>
/// The ROMs are public but are not redistributed by this repository, so the suite has to
/// stay green on a checkout that does not carry them while still making the gap visible in
/// the test report rather than silently passing. <c>tests/roms/README.md</c> says where to
/// get each file.
/// </remarks>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class RomFactAttribute : FactAttribute
{
    /// <summary>Requires <paramref name="requiredRoms"/>, relative to <c>impl/tests/roms/</c>.</summary>
    public RomFactAttribute(params string[] requiredRoms)
    {
        ArgumentNullException.ThrowIfNull(requiredRoms);

        string[] missing = [.. requiredRoms.Where(static rom => TestRomLibrary.Find(rom) is null)];
        if (missing.Length > 0)
        {
            Skip = TestRomLibrary.MissingMessage(missing);
        }
    }
}

/// <summary>
/// The <see cref="TheoryAttribute"/> counterpart of <see cref="RomFactAttribute"/>: the
/// whole theory is reported as skipped, naming the missing files, unless every public test
/// ROM of <paramref name="requiredRoms"/> is present.
/// </summary>
/// <remarks>
/// xUnit 2 can only skip a theory as a whole, so a theory lists every ROM of the suite it
/// walks through.
/// </remarks>
[XunitTestCaseDiscoverer("Xunit.Sdk.TheoryDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class RomTheoryAttribute : TheoryAttribute
{
    /// <summary>Requires <paramref name="requiredRoms"/>, relative to <c>impl/tests/roms/</c>.</summary>
    public RomTheoryAttribute(params string[] requiredRoms)
    {
        ArgumentNullException.ThrowIfNull(requiredRoms);

        string[] missing = [.. requiredRoms.Where(static rom => TestRomLibrary.Find(rom) is null)];
        if (missing.Length > 0)
        {
            Skip = TestRomLibrary.MissingMessage(missing);
        }
    }
}

/// <summary>
/// A test that is deliberately parked: the reason is reported as the skip message so that
/// the gap is visible in the lot report instead of silently passing.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.TheoryDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class PendingTheoryAttribute : TheoryAttribute
{
    /// <summary>Parks the theory with <paramref name="reason"/>.</summary>
    public PendingTheoryAttribute(string reason) => Skip = reason;
}
