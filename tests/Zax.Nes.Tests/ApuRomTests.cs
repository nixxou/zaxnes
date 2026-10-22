namespace Zax.Nes.Tests;

/// <summary>
/// The public APU test ROMs of SPEC-02 §3.12 (REQ-CORE-113) and the interrupt and DMA parts
/// of REQ-CORE-111 that the APU answers to.
/// </summary>
/// <remarks>
/// Each ROM is run through <see cref="TestRomRunner"/>, which reads back the result with the
/// protocol that ROM documents, and nothing else: no emulator hook, no screen scraping. The
/// ROMs that need a mapper beyond NROM, or that measure behaviour this lot deliberately does
/// not reproduce, are collected in <see cref="ApuRomPendingTests"/> so that the gap is
/// visible in the report instead of silently passing.
/// </remarks>
public sealed class ApuRomTests
{
    /// <summary>
    /// Result variable of the 2005-era <c>blargg_apu</c> runtime, the same address its
    /// <c>blargg_ppu_tests</c> sibling uses. The support sources shipped with the ROMs do not
    /// include the console, so the address was located by running the suite: every ROM parks
    /// with its result code in $00F0 and prints the same value on screen.
    /// </summary>
    private const ushort BlarggApuResult = 0x00F0;

    // ---------------------------------------------------------------- REQ-CORE-113

    [RomTheory(
        "apu_test/rom_singles/1-len_ctr.nes",
        "apu_test/rom_singles/2-len_table.nes",
        "apu_test/rom_singles/3-irq_flag.nes",
        "apu_test/rom_singles/4-jitter.nes",
        "apu_test/rom_singles/5-len_timing.nes",
        "apu_test/rom_singles/6-irq_flag_timing.nes",
        "apu_test/rom_singles/7-dmc_basics.nes",
        "apu_test/rom_singles/8-dmc_rates.nes")]
    [InlineData("1-len_ctr.nes")]
    [InlineData("2-len_table.nes")]
    [InlineData("3-irq_flag.nes")]
    [InlineData("4-jitter.nes")]
    [InlineData("5-len_timing.nes")]
    [InlineData("6-irq_flag_timing.nes")]
    [InlineData("7-dmc_basics.nes")]
    [InlineData("8-dmc_rates.nes")]
    public void REQ_CORE_113_apu_test_tests_1_to_8_pass(string rom)
        => AssertShellPass($"apu_test/rom_singles/{rom}");

    [RomTheory(
        "blargg_apu_2005.07.30/01.len_ctr.nes",
        "blargg_apu_2005.07.30/02.len_table.nes",
        "blargg_apu_2005.07.30/03.irq_flag.nes",
        "blargg_apu_2005.07.30/04.clock_jitter.nes",
        "blargg_apu_2005.07.30/05.len_timing_mode0.nes",
        "blargg_apu_2005.07.30/06.len_timing_mode1.nes",
        "blargg_apu_2005.07.30/07.irq_flag_timing.nes",
        "blargg_apu_2005.07.30/08.irq_timing.nes",
        "blargg_apu_2005.07.30/09.reset_timing.nes",
        "blargg_apu_2005.07.30/10.len_halt_timing.nes",
        "blargg_apu_2005.07.30/11.len_reload_timing.nes")]
    [InlineData("01.len_ctr.nes")]
    [InlineData("02.len_table.nes")]
    [InlineData("03.irq_flag.nes")]
    [InlineData("04.clock_jitter.nes")]
    [InlineData("05.len_timing_mode0.nes")]
    [InlineData("06.len_timing_mode1.nes")]
    [InlineData("07.irq_flag_timing.nes")]
    [InlineData("08.irq_timing.nes")]
    [InlineData("09.reset_timing.nes")]
    [InlineData("10.len_halt_timing.nes")]
    [InlineData("11.len_reload_timing.nes")]
    public void REQ_CORE_113_blargg_apu_2005_tests_pass(string rom)
        => AssertLegacyPass($"blargg_apu_2005.07.30/{rom}", BlarggApuResult);

    // ---------------------------------------------------------------- REQ-CORE-111

    [RomTheory(
        "cpu_interrupts_v2/rom_singles/1-cli_latency.nes",
        "cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes",
        "cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes",
        "cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes",
        "cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes")]
    [InlineData("1-cli_latency.nes")]
    [InlineData("2-nmi_and_brk.nes")]
    [InlineData("3-nmi_and_irq.nes")]
    [InlineData("4-irq_and_dma.nes")]
    [InlineData("5-branch_delays_irq.nes")]
    public void REQ_CORE_111_the_five_cpu_interrupts_parts_pass(string rom)
        => AssertShellPass($"cpu_interrupts_v2/rom_singles/{rom}");

    /// <summary>
    /// REQ-CORE-111: the combined MMC1 build, whose result code names the first failing
    /// sub-test. Round 1 reported it stopping on <c>2-nmi_and_brk</c>; the interrupt sequencing
    /// of <see cref="Cpu.Cpu6502"/> — vector hijacking and the taken-branch poll — now answers
    /// for all five.
    /// </summary>
    [RomFact("cpu_interrupts_v2/cpu_interrupts.nes")]
    public void REQ_CORE_111_the_combined_cpu_interrupts_rom_passes()
        => AssertShellPass("cpu_interrupts_v2/cpu_interrupts.nes");

    // ---------------------------------------------------------------- APU reset state

    [RomTheory(
        "apu_reset/4015_cleared.nes",
        "apu_reset/4017_timing.nes",
        "apu_reset/4017_written.nes",
        "apu_reset/irq_flag_cleared.nes",
        "apu_reset/len_ctrs_enabled.nes",
        "apu_reset/works_immediately.nes")]
    [InlineData("4015_cleared.nes")]
    [InlineData("4017_timing.nes")]
    [InlineData("4017_written.nes")]
    [InlineData("irq_flag_cleared.nes")]
    [InlineData("len_ctrs_enabled.nes")]
    [InlineData("works_immediately.nes")]
    public void REQ_CORE_004_the_power_on_half_of_the_apu_reset_tests_passes(string rom)
    {
        // Each of these ROMs checks its power-on condition, then asks for the reset button
        // ($6000 = $81) and checks the same thing again. The reset half cannot run here
        // (ApuRomPendingTests says why), but reaching the prompt proves the power-on half
        // passed: a failed check would have exited with its own code instead.
        string path = TestRomLibrary.Find($"apu_reset/{rom}")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, pressReset: false);

        Assert.Equal(0x81, result.Code);
        Assert.Contains("Press RESET", result.Text, StringComparison.Ordinal);
    }

    [RomFact("apu_reset/4017_timing.nes")]
    public void REQ_CORE_004_the_frame_sequencer_starts_nine_to_twelve_cycles_before_the_first_instruction()
    {
        // blargg: "After reset or power-up, APU acts as if $4017 were written with $00 from 9
        // to 12 clocks before first instruction begins." The ROM prints the delay it measured.
        string path = TestRomLibrary.Find("apu_reset/4017_timing.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, pressReset: false);

        Assert.Contains("Delay after effective $4017 write: 10", result.Text, StringComparison.Ordinal);
    }

    // ---------------------------------------------------------------- helpers

    private static void AssertLegacyPass(string rom, ushort resultAddress = TestRomRunner.LegacyResult)
    {
        string path = TestRomLibrary.Find(rom)!;
        TestRomRunner.Result result = TestRomRunner.RunLegacyRom(path, resultAddress: resultAddress);
        Assert.True(result.Passed, $"{rom}: {result.Describe()}");
    }

    private static void AssertShellPass(string rom)
    {
        string path = TestRomLibrary.Find(rom)!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);
        Assert.True(result.Passed, $"{rom}: {result.Describe()}");
    }
}

/// <summary>
/// APU-related test ROMs this lot does not answer to, each parked with the reason.
/// </summary>
public sealed class ApuRomPendingTests
{
    [PendingTheory(
        "Reset is a power cycle here (REQ-CORE-004, DEC-02-Q3), so it clears work RAM and "
        + "rewrites $4017 with $00. These ROMs keep their reset counter in RAM across the "
        + "reset (run_at_reset.s: num_resets_ lives in nv_ram) and 4017_written also requires "
        + "the last $4017 value to survive, so the reset half of each test cannot run. This is "
        + "the same deliberate divergence the REQ-CORE-004 notes record for "
        + "cpu_reset/ram_after_reset. The power-on half of all six is checked by "
        + "ApuRomTests.REQ_CORE_004_the_power_on_half_of_the_apu_reset_tests_passes.")]
    [InlineData("apu_reset/4015_cleared.nes")]
    [InlineData("apu_reset/4017_timing.nes")]
    [InlineData("apu_reset/4017_written.nes")]
    [InlineData("apu_reset/irq_flag_cleared.nes")]
    [InlineData("apu_reset/len_ctrs_enabled.nes")]
    [InlineData("apu_reset/works_immediately.nes")]
    public void REQ_CORE_004_apu_reset_needs_a_reset_that_keeps_ram(string rom) => Assert.True(rom.Length > 0);

    [PendingTheory(
        "P3 in REQ-CORE-113. A DMC sample fetch halts the CPU for four cycles on the cycle the "
        + "APU asks for the byte, instead of stopping it at a halt-able cycle and letting the "
        + "interrupted read run again; the hardware's two or three extra reads of $2007 or "
        + "$4016 are therefore not reproduced, which is exactly what these ROMs measure. They "
        + "also report only on screen and over the devcart serial link — their framework has no "
        + "$6000 status byte — so there is no CPU-readable verdict to assert on either.")]
    [InlineData("dmc_dma_during_read4/dma_2007_read.nes")]
    [InlineData("dmc_dma_during_read4/dma_2007_write.nes")]
    [InlineData("dmc_dma_during_read4/dma_4016_read.nes")]
    [InlineData("dmc_dma_during_read4/double_2007_read.nes")]
    [InlineData("dmc_dma_during_read4/read_write_2007.nes")]
    public void REQ_CORE_113_dmc_dma_during_read4_is_P3(string rom) => Assert.True(rom.Length > 0);

    [PendingTheory(
        "Not required by SPEC-02 §3.12. The ROM measures how long an OAM DMA takes when a DMC "
        + "fetch collides with it: the hardware interleaves the fetch into the sprite DMA's "
        + "spare cycles, costing one or two cycles, while this core halts the CPU for a full "
        + "four cycles per fetch, so it reports 528 or 529 clocks instead of the hardware's "
        + "513 to 518. Same root cause as dmc_dma_during_read4 above.")]
    [InlineData("sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes")]
    [InlineData("sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes")]
    public void the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact(string rom)
        => Assert.True(rom.Length > 0);
}
