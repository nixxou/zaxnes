using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Xunit.Sdk;

namespace Zax.App.Tests;

/// <summary>
/// Locates the public test ROMs under <c>impl/tests/roms/</c>, which the repository does not
/// redistribute (<c>tests/roms/README.md</c> says where to fetch them).
/// </summary>
internal static class PublicRoms
{
    /// <summary>Absolute path of <c>impl/tests/roms</c>, or null when it cannot be found.</summary>
    public static string? Directory { get; } = Locate();

    /// <summary>The full path of a ROM under the folder, or null when it is absent.</summary>
    public static string? Find(string relativePath)
    {
        if (Directory is null)
        {
            return null;
        }

        string candidate = Path.Combine(Directory, relativePath);
        return File.Exists(candidate) ? candidate : null;
    }

    /// <summary>The message a skipped test reports, naming what to download.</summary>
    public static string MissingMessage(params string[] relativePaths)
        => $"Public test ROM not available: {string.Join(", ", relativePaths)}. "
            + $"See {Path.Combine(Directory ?? "impl/tests/roms", "README.md")} for its source and licence.";

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "tests", "roms");
            if (System.IO.Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>
/// A fact that reports as skipped, naming the missing files, unless every public test ROM it
/// lists is present. The suite therefore stays green on a checkout without the ROMs while the
/// gap stays visible in the report.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class PublicRomFactAttribute : FactAttribute
{
    /// <summary>Requires <paramref name="requiredRoms"/>, relative to <c>impl/tests/roms/</c>.</summary>
    public PublicRomFactAttribute(params string[] requiredRoms)
    {
        ArgumentNullException.ThrowIfNull(requiredRoms);

        string[] missing = [.. requiredRoms.Where(static rom => PublicRoms.Find(rom) is null)];
        if (missing.Length > 0)
        {
            Skip = PublicRoms.MissingMessage(missing);
        }
    }
}

/// <summary>
/// Savestate round-trip determinism on a public test ROM, which is the automatable part of
/// SPEC-12 REQ-ACC-040 together with REQ-ARCH-012 and REQ-ARCH-013.
/// </summary>
/// <remarks>
/// The synthetic-ROM version of these checks lives in <see cref="SaveStateTests"/> and always
/// runs; these two add a real program — a CPU test ROM exercising the whole instruction set —
/// so that the round trip is proven on something the core really has to emulate.
/// </remarks>
public sealed class PublicRomTests
{
    private const string Nestest = "nestest.nes";

    /// <summary>
    /// REQ-ARCH-012 and REQ-LOOP-060: saving at frame N on a public test ROM, running on,
    /// reloading and replaying the same input sequence reproduces the core state byte for byte.
    /// </summary>
    [PublicRomFact(Nestest)]
    public void REQ_ARCH_012_a_public_rom_replays_identically_after_a_load()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("nestest", PublicRoms.Find(Nestest));

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("nestest"));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.RunFrames(120);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));

        // A deliberate input sequence, so the replay is not a trivially identical idle run.
        byte[] firstRun = PlayAndCapture(harness);

        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(1));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        byte[] secondRun = PlayAndCapture(harness);

        Assert.Equal(firstRun, secondRun);
    }

    /// <summary>
    /// REQ-ARCH-013 and REQ-PERS-056: on a real ROM the file stays under the announced bound,
    /// and two saves taken without an emulated frame in between are identical.
    /// </summary>
    [PublicRomFact(Nestest)]
    public void REQ_ARCH_013_a_public_rom_stays_under_the_announced_bound()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("nestest", PublicRoms.Find(Nestest));

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("nestest"));
        harness.RunFrames(60);

        int bound = harness.Host.MaxSaveStateSize;
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));

        long written = new FileInfo(harness.Paths.SlotStateFile("2")).Length;
        Assert.True(written <= bound, $"{written} bytes written, bound announced {bound}");

        harness.Run(ZaxCommands.Pause);
        SaveStateContent first = harness.Host.CaptureSaveState("2")!;
        SaveStateContent second = harness.Host.CaptureSaveState("2")!;
        Assert.Equal(first.Core, second.Core);
        Assert.Equal(first.Conversion, second.Conversion);
    }

    private static byte[] PlayAndCapture(LoopHarness harness)
    {
        InputAction[] sequence =
        [
            InputAction.None, InputAction.Start, InputAction.None,
            InputAction.A, InputAction.Right, InputAction.A | InputAction.Right,
        ];

        harness.RunExactFrames(180, sequence);
        return harness.Host.Session!.Core.SaveState(includeRomImage: false);
    }
}
