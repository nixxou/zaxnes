namespace Zax.Nes.Tests;

/// <summary>
/// The two public reset ROMs SPEC-02 §3.12, as amended by AMD-044, adds to the required list:
/// <c>cpu_reset/ram_after_reset</c> and <c>cpu_reset/registers</c> (blargg), driven by the
/// <b>soft</b> reset of REQ-CORE-122.
/// </summary>
/// <remarks>
/// Both ROMs press RESET in the middle of the run ($6000 = $81) and then check what a real
/// console keeps across it: the RAM in the first, A, X, Y, S and the I flag in the second. They
/// therefore fail under the hard reset of REQ-CORE-004, which is a power cycle (DEC-02-Q3) —
/// the note of REQ-CORE-004 says so and stays true, and
/// <c>Zax.Acceptance.Tests.CoreAccuracyTests</c> still checks it.
/// </remarks>
public sealed class CpuResetRomTests
{
    [RomTheory("cpu_reset/ram_after_reset.nes", "cpu_reset/registers.nes")]
    [InlineData("ram_after_reset.nes")]
    [InlineData("registers.nes")]
    public void REQ_CORE_122_the_cpu_reset_roms_pass_under_a_soft_reset(string rom)
    {
        string path = TestRomLibrary.Find($"cpu_reset/{rom}")!;

        TestRomRunner.Result result = TestRomRunner.RunShellRom(
            path,
            maxFrames: 20_000,
            pressReset: true,
            softReset: true);

        Assert.True(result.Passed, $"cpu_reset/{rom}: {result.Describe()}");
    }
}
