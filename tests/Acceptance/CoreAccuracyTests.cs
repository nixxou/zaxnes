using Zax.Nes;
using Zax.Nes.Cpu;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-02 §3.12 (accuracy, public test ROMs) and the
/// corresponding SPEC-12 REQ-ACC-001.
/// </summary>
public sealed class CoreAccuracyTests
{
    [Fact] // REQ-CORE-110, REQ-ACC-001
    public void Nestest_trace_matches_the_public_log()
    {
        string rom = RomSuite.Find("nestest.nes")!;
        string log = RomSuite.Find("nestest.log")!;

        NesCore core = new();
        Assert.True(core.LoadRom(File.ReadAllBytes(rom), "nestest.nes").Success);

        // Automation mode: start at $C000 (public nestest documentation).
        core.Cpu.PC = 0xC000;
        core.Cpu.S = 0xFD;
        core.Cpu.P = 0x24;
        core.Cpu.A = 0;
        core.Cpu.X = 0;
        core.Cpu.Y = 0;

        string[] lines = File.ReadAllLines(log);
        long cycleBias = -1;

        for (int i = 0; i < lines.Length; i++)
        {
            (ushort pc, byte a, byte x, byte y, byte p, byte s, long cyc) = RomSuite.ParseLogLine(lines[i]);
            CpuTraceLine trace = CpuTracer.Capture(core.Cpu, core.Bus);
            cycleBias = cycleBias < 0 ? trace.Cycles - cyc : cycleBias;
            Assert.Equal(0, cycleBias); // the CYC column must match the log absolutely

            Assert.True(
                pc == trace.Pc && a == trace.A && x == trace.X && y == trace.Y
                && p == trace.P && s == trace.S && cyc == trace.Cycles - cycleBias,
                $"line {i + 1}:\n  expected {lines[i]}\n  actual   {trace.Format()}");

            core.Cpu.Step();
        }

        // nestest reports its verdict in $0002 and $0003: zero means every test passed.
        Assert.Equal(0, core.ReadMemory(0x0002));
        Assert.Equal(0, core.ReadMemory(0x0003));
    }

    [Fact] // REQ-CORE-110, REQ-ACC-001
    public void Instr_test_v5_all_instrs_passes()
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find("instr_test-v5/all_instrs.nes")!, 120_000);
        Assert.True(outcome.Code == 0, outcome.Describe());
        Assert.Contains("passed", outcome.Text, StringComparison.OrdinalIgnoreCase);
    }

    [Theory] // REQ-CORE-111, REQ-ACC-001
    [InlineData("instr_timing/instr_timing.nes")]
    [InlineData("cpu_timing_test6/cpu_timing_test.nes")]
    public void Cpu_timing_suites_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 80_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    /// <summary>
    /// REQ-CORE-111 (P2) requires <c>cpu_interrupts_v2</c> to pass, the combined ROM and each of
    /// its five parts: the CLI latency, an NMI that hijacks a BRK, an NMI racing an IRQ, the IRQ
    /// and DMA interaction, and the cycle a taken branch polls /IRQ on.
    /// </summary>
    /// <remarks>
    /// Round 1: parts 2, 3 and 5 failed and the combined ROM stopped on test 2 of 5
    /// (<c>verify/round1/R1-report-part-a.md</c>). Corrected by the <c>R1-fixes</c> batch — vector
    /// hijacking by a latched NMI, no poll at the end of the interrupt sequence, and the
    /// taken-branch poll quirk. Re-verified 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Theory] // REQ-CORE-111
    [InlineData("cpu_interrupts_v2/cpu_interrupts.nes")]
    [InlineData("cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes")]
    [InlineData("cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes")]
    [InlineData("cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes")]
    public void Cpu_interrupts_v2_must_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 80_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-111 (the two parts that already passed at round 1)
    [InlineData("cpu_interrupts_v2/rom_singles/1-cli_latency.nes")]
    [InlineData("cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes")]
    public void Cpu_interrupts_v2_irq_and_dma_parts_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 80_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-111, REQ-ACC-001
    [InlineData("branch_timing_tests/1.Branch_Basics.nes")]
    [InlineData("branch_timing_tests/2.Backward_Branch.nes")]
    [InlineData("branch_timing_tests/3.Forward_Branch.nes")]
    public void Branch_timing_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!);
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-112, REQ-ACC-001
    [InlineData("sprite_hit_tests_2005.10.05/01.basics.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/02.alignment.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/03.corners.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/04.flip.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/05.left_clip.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/06.right_edge.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/07.screen_bottom.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/08.double_height.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/09.timing_basics.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/10.timing_order.nes")]
    [InlineData("sprite_hit_tests_2005.10.05/11.edge_timing.nes")]
    public void Sprite_zero_hit_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!);
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-112, REQ-ACC-001
    [InlineData("ppu_vbl_nmi/rom_singles/01-vbl_basics.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes")]
    public void Ppu_vbl_nmi_tests_one_to_three_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 40_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-112, REQ-ACC-001 (power_up_palette is excluded by the requirement)
    [InlineData("blargg_ppu_tests_2005.09.15b/palette_ram.nes")]
    [InlineData("blargg_ppu_tests_2005.09.15b/sprite_ram.nes")]
    [InlineData("blargg_ppu_tests_2005.09.15b/vram_access.nes")]
    [InlineData("blargg_ppu_tests_2005.09.15b/vbl_clear_time.nes")]
    public void Blargg_ppu_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!, RomSuite.BlarggResult);
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Fact] // REQ-CORE-112: control that the result address discriminates pass from failure
    public void Power_up_palette_is_the_one_blargg_ppu_test_that_fails()
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(
            RomSuite.Find("blargg_ppu_tests_2005.09.15b/power_up_palette.nes")!, RomSuite.BlarggResult);

        // REQ-CORE-112 excludes it on purpose: the palette starts at 0.
        Assert.True(outcome.Code != 1, $"power_up_palette was expected to fail: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-116
    [InlineData("ppu_vbl_nmi/rom_singles/04-nmi_control.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/05-nmi_timing.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/06-suppression.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes")]
    [InlineData("ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes")]
    [InlineData("oam_read/oam_read.nes")]
    [InlineData("ppu_open_bus/ppu_open_bus.nes")]
    public void Important_ppu_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 40_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-116
    [InlineData("vbl_nmi_timing/1.frame_basics.nes")]
    [InlineData("vbl_nmi_timing/2.vbl_timing.nes")]
    [InlineData("vbl_nmi_timing/3.even_odd_frames.nes")]
    [InlineData("vbl_nmi_timing/4.vbl_clear_timing.nes")]
    [InlineData("vbl_nmi_timing/5.nmi_suppression.nes")]
    [InlineData("vbl_nmi_timing/6.nmi_disable.nes")]
    [InlineData("vbl_nmi_timing/7.nmi_timing.nes")]
    public void Vbl_nmi_timing_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!);
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-113, REQ-ACC-001
    [InlineData("apu_test/apu_test.nes")]
    public void Apu_test_passes(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 60_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-113, REQ-ACC-001
    [InlineData("blargg_apu_2005.07.30/01.len_ctr.nes")]
    [InlineData("blargg_apu_2005.07.30/02.len_table.nes")]
    [InlineData("blargg_apu_2005.07.30/03.irq_flag.nes")]
    [InlineData("blargg_apu_2005.07.30/04.clock_jitter.nes")]
    [InlineData("blargg_apu_2005.07.30/05.len_timing_mode0.nes")]
    [InlineData("blargg_apu_2005.07.30/06.len_timing_mode1.nes")]
    [InlineData("blargg_apu_2005.07.30/07.irq_flag_timing.nes")]
    [InlineData("blargg_apu_2005.07.30/08.irq_timing.nes")]
    [InlineData("blargg_apu_2005.07.30/09.reset_timing.nes")]
    [InlineData("blargg_apu_2005.07.30/10.len_halt_timing.nes")]
    [InlineData("blargg_apu_2005.07.30/11.len_reload_timing.nes")]
    public void Blargg_apu_2005_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!, RomSuite.BlarggResult);
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-114, REQ-ACC-001 (test 6, "MMC3 alt", may fail)
    [InlineData("mmc3_test_2/rom_singles/1-clocking.nes")]
    [InlineData("mmc3_test_2/rom_singles/2-details.nes")]
    [InlineData("mmc3_test_2/rom_singles/3-A12_clocking.nes")]
    [InlineData("mmc3_test_2/rom_singles/4-scanline_timing.nes")]
    [InlineData("mmc3_test_2/rom_singles/5-MMC3.nes")]
    public void Mmc3_tests_one_to_five_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunShell(RomSuite.Find(relative)!, 40_000);
        Assert.True(outcome.Code == 0, $"{relative}: {outcome.Describe()}");
    }

    [Theory] // REQ-CORE-033 (mmc3_test gives the same verdict with and without the callback)
    [InlineData("mmc3_test_2/rom_singles/1-clocking.nes")]
    [InlineData("mmc3_test_2/rom_singles/3-A12_clocking.nes")]
    [InlineData("mmc3_test_2/rom_singles/4-scanline_timing.nes")]
    [InlineData("mmc3_test_2/rom_singles/5-MMC3.nes")]
    public void Mmc3_tests_are_unaffected_by_the_video_callback(string relative)
    {
        string path = RomSuite.Find(relative)!;
        byte[] image = File.ReadAllBytes(path);

        static int Run(byte[] image, bool withCallback)
        {
            NesCore core = new();
            core.LoadRom(image, "mmc3.nes");
            if (withCallback)
            {
                core.VideoCallback = (in FrameBatch _) => { };
            }

            for (int frame = 0; frame < 40_000; frame++)
            {
                core.RunFrame(0);
                if (core.ReadMemory(0x6001) == 0xDE && core.ReadMemory(0x6002) == 0xB0
                    && core.ReadMemory(0x6003) == 0x61)
                {
                    byte status = core.ReadMemory(0x6000);
                    if (status is not (0x80 or 0x81) && frame > 10)
                    {
                        return status;
                    }
                }
            }

            return -1;
        }

        Assert.Equal(Run(image, false), Run(image, true));
    }

    [Theory] // REQ-CORE-116, REQ-ACC-001 (sprite_overflow_tests is P3: "PEUT passer"; it does)
    [InlineData("sprite_overflow_tests/1.Basics.nes")]
    [InlineData("sprite_overflow_tests/2.Details.nes")]
    [InlineData("sprite_overflow_tests/3.Timing.nes")]
    [InlineData("sprite_overflow_tests/4.Obscure.nes")]
    [InlineData("sprite_overflow_tests/5.Emulator.nes")]
    public void Sprite_overflow_tests_pass(string relative)
    {
        RomSuite.Outcome outcome = RomSuite.RunLegacy(RomSuite.Find(relative)!);

        // P3: the requirement only says "PEUT passer"; the five parts in fact pass.
        Assert.True(outcome.Code == 1, $"{relative}: {outcome.Describe()}");
    }

    [Fact] // REQ-CORE-004, note: cpu_reset/ram_after_reset fails on purpose
    public void Ram_after_reset_fails_on_purpose()
    {
        string? path = RomSuite.Find("cpu_reset/ram_after_reset.nes");
        if (path is null)
        {
            return;
        }

        RomSuite.Outcome outcome = RomSuite.RunShell(path, 20_000);
        Assert.True(outcome.Code != 0, $"the spec expects this ROM to fail: {outcome.Describe()}");
    }
}
