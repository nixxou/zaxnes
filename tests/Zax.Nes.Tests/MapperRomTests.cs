using System.Text;

namespace Zax.Nes.Tests;

/// <summary>
/// The public test ROMs that needed a mapper: the MMC3 suites of REQ-CORE-114 and the
/// combined builds of the blargg suites, which are MMC1 or CNROM cartridges
/// (REQ-CORE-110, REQ-CORE-111, REQ-CORE-113, REQ-CORE-116).
/// </summary>
/// <remarks>
/// Each ROM is read through its own documented protocol: the $6000 shell of the later
/// suites, the low-memory result byte of the 2005-era ones, or — for the ROMs that only ever
/// print — the text the game left in the nametable, which is written as plain character codes
/// (<see cref="ScreenText"/>).
/// </remarks>
public sealed class MapperRomTests
{
    // ---------------------------------------------------------------- MMC3, REQ-CORE-114

    [RomTheory(
        "mmc3_test_2/rom_singles/1-clocking.nes",
        "mmc3_test_2/rom_singles/2-details.nes",
        "mmc3_test_2/rom_singles/3-A12_clocking.nes",
        "mmc3_test_2/rom_singles/4-scanline_timing.nes",
        "mmc3_test_2/rom_singles/5-MMC3.nes")]
    [InlineData("1-clocking.nes")]
    [InlineData("2-details.nes")]
    [InlineData("3-A12_clocking.nes")]
    [InlineData("4-scanline_timing.nes")]
    [InlineData("5-MMC3.nes")]
    public void REQ_CORE_114_mmc3_test_2_tests_1_to_5_pass(string rom)
    {
        string path = TestRomLibrary.Find($"mmc3_test_2/rom_singles/{rom}")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.True(result.Passed, $"{rom}: {result.Describe()}");
    }

    [RomFact("mmc3_test_2/rom_singles/6-MMC3_alt.nes")]
    public void REQ_CORE_114_mmc3_test_2_test_6_reports_the_revision_difference()
    {
        // REQ-CORE-100 asks for the "revision B" counter and REQ-CORE-114 allows test 6,
        // "MMC3 alt", to fail: it checks the revision A rule, under which a counter that
        // reached zero on its own does not fire again when it reloads. Failing it with code 2
        // is therefore the expected outcome, and is asserted so that a change of behaviour
        // cannot pass unnoticed.
        string path = TestRomLibrary.Find("mmc3_test_2/rom_singles/6-MMC3_alt.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.Equal(2, result.Code);
        Assert.Contains("reloading to 0", result.Text, StringComparison.Ordinal);
    }

    [RomTheory(
        "mmc3_irq_tests/1.Clocking.nes",
        "mmc3_irq_tests/2.Details.nes",
        "mmc3_irq_tests/3.A12_clocking.nes",
        "mmc3_irq_tests/4.Scanline_timing.nes",
        "mmc3_irq_tests/6.MMC3_rev_B.nes")]
    [InlineData("1.Clocking.nes")]
    [InlineData("2.Details.nes")]
    [InlineData("3.A12_clocking.nes")]
    [InlineData("4.Scanline_timing.nes")]
    [InlineData("6.MMC3_rev_B.nes")]
    public void REQ_CORE_114_the_earlier_mmc3_irq_suite_passes(string rom)
    {
        // The 2005-era builds of the same tests, which report through the low-memory result
        // byte. Test 5 is the revision A counterpart of test 6 and is expected to fail.
        string path = TestRomLibrary.Find($"mmc3_irq_tests/{rom}")!;
        TestRomRunner.Result result = TestRomRunner.RunLegacyRom(path);

        Assert.True(result.Passed, $"{rom}: {result.Describe()}");
    }

    [RomFact("mmc3_irq_tests/5.MMC3_rev_A.nes")]
    public void REQ_CORE_100_the_revision_a_mmc3_test_fails_as_the_revision_b_rule_requires()
    {
        string path = TestRomLibrary.Find("mmc3_irq_tests/5.MMC3_rev_A.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunLegacyRom(path);

        Assert.False(result.Passed, result.Describe());
    }

    [RomFact("mmc3_test/6-MMC6.nes")]
    public void REQ_CORE_100_the_mmc6_build_is_read_as_an_mmc3()
    {
        // An iNES header cannot say whether a mapper 4 board is an MMC3 or an MMC6, so this
        // build runs as an MMC3 and fails on the revision rule, as REQ-CORE-114 allows for
        // test 6. The MMC6 RAM window itself is covered by a unit test on a NES 2.0 header.
        string path = TestRomLibrary.Find("mmc3_test/6-MMC6.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.False(result.Passed, result.Describe());
    }

    // ---------------------------------------------------------------- CPU, REQ-CORE-110/111

    [RomFact("instr_test-v5/all_instrs.nes")]
    public void REQ_CORE_110_instr_test_v5_all_instrs_passes()
    {
        // The combined build is an MMC1 cartridge: this is the ROM REQ-CORE-110 names.
        string path = TestRomLibrary.Find("instr_test-v5/all_instrs.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, maxFrames: 4000);

        Assert.True(result.Passed, result.Describe());
        Assert.Contains("All 16 tests passed", result.Text, StringComparison.Ordinal);
    }

    [RomTheory(
        "instr_test-v5/rom_singles/01-basics.nes",
        "instr_test-v5/rom_singles/02-implied.nes",
        "instr_test-v5/rom_singles/03-immediate.nes",
        "instr_test-v5/rom_singles/04-zero_page.nes",
        "instr_test-v5/rom_singles/05-zp_xy.nes",
        "instr_test-v5/rom_singles/06-absolute.nes",
        "instr_test-v5/rom_singles/07-abs_xy.nes",
        "instr_test-v5/rom_singles/08-ind_x.nes",
        "instr_test-v5/rom_singles/09-ind_y.nes",
        "instr_test-v5/rom_singles/10-branches.nes",
        "instr_test-v5/rom_singles/11-stack.nes",
        "instr_test-v5/rom_singles/12-jmp_jsr.nes",
        "instr_test-v5/rom_singles/13-rts.nes",
        "instr_test-v5/rom_singles/14-rti.nes",
        "instr_test-v5/rom_singles/15-brk.nes",
        "instr_test-v5/rom_singles/16-special.nes")]
    [InlineData("01-basics.nes")]
    [InlineData("02-implied.nes")]
    [InlineData("03-immediate.nes")]
    [InlineData("04-zero_page.nes")]
    [InlineData("05-zp_xy.nes")]
    [InlineData("06-absolute.nes")]
    [InlineData("07-abs_xy.nes")]
    [InlineData("08-ind_x.nes")]
    [InlineData("09-ind_y.nes")]
    [InlineData("10-branches.nes")]
    [InlineData("11-stack.nes")]
    [InlineData("12-jmp_jsr.nes")]
    [InlineData("13-rts.nes")]
    [InlineData("14-rti.nes")]
    [InlineData("15-brk.nes")]
    [InlineData("16-special.nes")]
    public void REQ_CORE_110_every_instr_test_v5_single_passes(string rom)
    {
        string path = TestRomLibrary.Find($"instr_test-v5/rom_singles/{rom}")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.True(result.Passed, $"{rom}: {result.Describe()}");
    }

    [RomFact("instr_timing/instr_timing.nes")]
    public void REQ_CORE_111_instr_timing_passes()
    {
        string path = TestRomLibrary.Find("instr_timing/instr_timing.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, maxFrames: 3000);

        Assert.True(result.Passed, result.Describe());
        Assert.Contains("All 2 tests passed", result.Text, StringComparison.Ordinal);
    }

    [RomFact("instr_misc/instr_misc.nes")]
    public void REQ_CORE_110_instr_misc_passes()
    {
        string path = TestRomLibrary.Find("instr_misc/instr_misc.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.True(result.Passed, result.Describe());
    }

    [RomFact("cpu_timing_test6/cpu_timing_test.nes")]
    public void REQ_CORE_111_cpu_timing_test6_passes_with_the_unofficial_instructions()
    {
        // This one only ever prints: its readme says to hold B at power-on to have the
        // unofficial instructions tested as well, and the verdict is read off the screen.
        string path = TestRomLibrary.Find("cpu_timing_test6/cpu_timing_test.nes")!;
        var core = new NesCore();
        Assert.True(core.LoadRom(File.ReadAllBytes(path), "cpu_timing_test.nes").Success);

        for (int frame = 0; frame < 1200; frame++)
        {
            core.RunFrame(0x0002); // B held: official plus undocumented.
        }

        string screen = ScreenText(core);
        Assert.Contains("OFFICIAL + UNDOCUMENTED", screen, StringComparison.Ordinal);
        Assert.Contains("PASSED", screen, StringComparison.Ordinal);
    }

    // ---------------------------------------------------------------- PPU and APU

    [RomFact("ppu_vbl_nmi/ppu_vbl_nmi.nes")]
    public void REQ_CORE_116_the_combined_ppu_vbl_nmi_rom_passes()
    {
        string path = TestRomLibrary.Find("ppu_vbl_nmi/ppu_vbl_nmi.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, maxFrames: 3000);

        Assert.True(result.Passed, result.Describe());
        Assert.Contains("All 10 tests passed", result.Text, StringComparison.Ordinal);
    }

    [RomFact("ppu_read_buffer/test_ppu_read_buffer.nes")]
    public void REQ_CORE_116_test_ppu_read_buffer_passes()
    {
        string path = TestRomLibrary.Find("ppu_read_buffer/test_ppu_read_buffer.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path, maxFrames: 3000);

        Assert.True(result.Passed, result.Describe());
    }

    [RomFact("apu_test/apu_test.nes")]
    public void REQ_CORE_113_the_combined_apu_test_rom_passes()
    {
        string path = TestRomLibrary.Find("apu_test/apu_test.nes")!;
        TestRomRunner.Result result = TestRomRunner.RunShellRom(path);

        Assert.True(result.Passed, result.Describe());
        Assert.Contains("All 8 tests passed", result.Text, StringComparison.Ordinal);
    }

    [RomFact("cpu_dummy_reads/cpu_dummy_reads.nes")]
    public void REQ_CORE_100_cnrom_runs_cpu_dummy_reads()
    {
        // A CNROM cartridge; the suite itself is not required by SPEC-02, but it exercises
        // the board and the dummy reads of the CPU together.
        string path = TestRomLibrary.Find("cpu_dummy_reads/cpu_dummy_reads.nes")!;
        var core = new NesCore();
        Assert.True(core.LoadRom(File.ReadAllBytes(path), "cpu_dummy_reads.nes").Success);

        for (int frame = 0; frame < 600; frame++)
        {
            core.RunFrame(0);
        }

        string screen = ScreenText(core);
        Assert.Contains("PASSED", screen, StringComparison.OrdinalIgnoreCase);
    }

    // ---------------------------------------------------------------- boots, REQ-CORE-100

    [RomTheory(
        "MMC1_A12/mmc1_a12.nes",
        "240pee/240pee.nes",
        "240pee/240pee-bnrom.nes",
        "m22chrbankingtest/0-127.nes",
        "mmc5test/mmc5test.nes",
        "mmc5test_v2/mmc5test.nes",
        "exram/mmc5exram.nes",
        "other/firefly.nes",
        "other/BladeBuster.nes",
        "other/fceuxd.nes",
        "other/oam3.nes",
        "other/test28.nes",
        "other/Streemerz_bundle.nes",
        "other/quantum_disco_brothers_by_wAMMA.nes",
        "nes15-1.0.0/nes15-NTSC.nes")]
    [InlineData("MMC1_A12/mmc1_a12.nes", 1)]
    [InlineData("240pee/240pee.nes", 2)]
    [InlineData("240pee/240pee-bnrom.nes", 34)]
    [InlineData("m22chrbankingtest/0-127.nes", 22)]
    [InlineData("mmc5test/mmc5test.nes", 5)]
    [InlineData("mmc5test_v2/mmc5test.nes", 5)]
    [InlineData("exram/mmc5exram.nes", 5)]
    [InlineData("other/firefly.nes", 5)]
    [InlineData("other/BladeBuster.nes", 4)]
    [InlineData("other/fceuxd.nes", 11)]
    [InlineData("other/oam3.nes", 7)]
    [InlineData("other/test28.nes", 28)]
    [InlineData("other/Streemerz_bundle.nes", 28)]
    [InlineData("other/quantum_disco_brothers_by_wAMMA.nes", 3)]
    [InlineData("nes15-1.0.0/nes15-NTSC.nes", 0)]
    public void REQ_CORE_100_each_public_rom_of_a_required_mapper_boots_and_draws(string rom, int mapper)
    {
        // The acceptance criterion of REQ-CORE-100: every mapper of the table starts at least
        // one public test ROM or free homebrew. These are the ones available here; the mappers
        // no public ROM covers are pinned by the unit tests of MapperTests instead.
        var core = new NesCore();
        Rom.RomLoadResult load = core.LoadRom(File.ReadAllBytes(TestRomLibrary.Find(rom)!), rom);

        Assert.True(load.Success, load.Error);
        Assert.Equal(mapper, core.Cartridge!.MapperNumber);

        int batches = 0;
        core.VideoCallback = (in FrameBatch batch) => batches++;

        for (int frame = 0; frame < 180; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(180, batches);

        // Something was drawn: a ROM that crashed or hung before switching rendering on would
        // produce an empty batch every frame.
        Assert.NotEmpty(core.TileRecords.ToArray());
    }

    [RomFact("other/test28.nes")]
    public void REQ_CORE_100_action53_powers_on_with_the_last_bank_at_c000()
    {
        // The first thing the board's own test ROM checks; it prints the complaint below when
        // the power-on banks are wrong.
        var core = new NesCore();
        Assert.True(core.LoadRom(File.ReadAllBytes(TestRomLibrary.Find("other/test28.nes")!), "test28.nes").Success);

        for (int frame = 0; frame < 200; frame++)
        {
            core.RunFrame(0);
        }

        Assert.DoesNotContain("DOES NOT POWER ON", ScreenText(core), StringComparison.Ordinal);
    }

    /// <summary>
    /// The text a ROM left in the first nametable, as characters.
    /// </summary>
    /// <remarks>
    /// blargg's console and most homebrew put one character code per tile, with a CHR set
    /// whose glyphs sit at the ASCII positions, so the nametable reads back as text. It is the
    /// only way to read the verdict of the ROMs that report nothing but a printed message.
    /// </remarks>
    private static string ScreenText(NesCore core)
    {
        var text = new StringBuilder();
        ReadOnlySpan<byte> vram = core.Ppu.Vram;

        for (int row = 0; row < 30; row++)
        {
            for (int column = 0; column < 32; column++)
            {
                byte tile = vram[(row * 32) + column];
                text.Append(tile is >= 32 and < 127 ? (char)tile : ' ');
            }

            text.AppendLine();
        }

        return text.ToString();
    }
}
