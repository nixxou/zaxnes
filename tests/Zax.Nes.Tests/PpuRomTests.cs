using System.Globalization;

namespace Zax.Nes.Tests;

/// <summary>
/// The public test ROMs of SPEC-02 §3.12 that the PPU and the CPU branch timing answer to
/// (REQ-CORE-111, REQ-CORE-112, REQ-CORE-116).
/// </summary>
/// <remarks>
/// <para>
/// Each ROM is run through <see cref="TestRomRunner"/>, which reads back the result with the
/// protocol that ROM documents, and nothing else: no emulator hook, no screen scraping.
/// </para>
/// <para>
/// The ROMs that need a mapper beyond NROM — the combined <c>ppu_vbl_nmi</c> build and
/// <c>test_ppu_read_buffer</c> among them — live in <see cref="MapperRomTests"/>.
/// </para>
/// </remarks>
public sealed class PpuRomTests
{
    /// <summary>
    /// Result variable of the 2005-era <c>blargg_ppu_tests</c> runtime. Its support sources
    /// are not shipped with the ROMs, unlike the other suites of the same era, so the
    /// address was located by running them: all four park with 1 in $00F0 and print "$01" on
    /// screen, which is the "tests passed" code of that console.
    /// </summary>
    private const ushort BlarggPpuResult = 0x00F0;

    // ---------------------------------------------------------------- REQ-CORE-111

    [RomTheory(
        "branch_timing_tests/1.Branch_Basics.nes",
        "branch_timing_tests/2.Backward_Branch.nes",
        "branch_timing_tests/3.Forward_Branch.nes")]
    [InlineData("1.Branch_Basics.nes")]
    [InlineData("2.Backward_Branch.nes")]
    [InlineData("3.Forward_Branch.nes")]
    public void REQ_CORE_111_branch_timing_tests_pass(string rom)
        => AssertLegacyPass($"branch_timing_tests/{rom}");

    // ---------------------------------------------------------------- REQ-CORE-112

    [RomTheory(
        "sprite_hit_tests_2005.10.05/01.basics.nes",
        "sprite_hit_tests_2005.10.05/02.alignment.nes",
        "sprite_hit_tests_2005.10.05/03.corners.nes",
        "sprite_hit_tests_2005.10.05/04.flip.nes",
        "sprite_hit_tests_2005.10.05/05.left_clip.nes",
        "sprite_hit_tests_2005.10.05/06.right_edge.nes",
        "sprite_hit_tests_2005.10.05/07.screen_bottom.nes",
        "sprite_hit_tests_2005.10.05/08.double_height.nes",
        "sprite_hit_tests_2005.10.05/09.timing_basics.nes",
        "sprite_hit_tests_2005.10.05/10.timing_order.nes",
        "sprite_hit_tests_2005.10.05/11.edge_timing.nes")]
    [InlineData("01.basics.nes")]
    [InlineData("02.alignment.nes")]
    [InlineData("03.corners.nes")]
    [InlineData("04.flip.nes")]
    [InlineData("05.left_clip.nes")]
    [InlineData("06.right_edge.nes")]
    [InlineData("07.screen_bottom.nes")]
    [InlineData("08.double_height.nes")]
    [InlineData("09.timing_basics.nes")]
    [InlineData("10.timing_order.nes")]
    [InlineData("11.edge_timing.nes")]
    public void REQ_CORE_112_sprite_hit_tests_pass(string rom)
        => AssertLegacyPass($"sprite_hit_tests_2005.10.05/{rom}");

    [RomTheory(
        "blargg_ppu_tests_2005.09.15b/palette_ram.nes",
        "blargg_ppu_tests_2005.09.15b/sprite_ram.nes",
        "blargg_ppu_tests_2005.09.15b/vram_access.nes",
        "blargg_ppu_tests_2005.09.15b/vbl_clear_time.nes")]
    [InlineData("palette_ram.nes")]
    [InlineData("sprite_ram.nes")]
    [InlineData("vram_access.nes")]
    [InlineData("vbl_clear_time.nes")]
    public void REQ_CORE_112_blargg_ppu_tests_pass(string rom)
        => AssertLegacyPass($"blargg_ppu_tests_2005.09.15b/{rom}", BlarggPpuResult);

    [RomTheory(
        "ppu_vbl_nmi/rom_singles/01-vbl_basics.nes",
        "ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes",
        "ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes")]
    [InlineData("01-vbl_basics.nes")]
    [InlineData("02-vbl_set_time.nes")]
    [InlineData("03-vbl_clear_time.nes")]
    public void REQ_CORE_112_ppu_vbl_nmi_tests_1_to_3_pass(string rom)
        => AssertShellPass($"ppu_vbl_nmi/rom_singles/{rom}");

    // ---------------------------------------------------------------- REQ-CORE-116

    [RomTheory(
        "ppu_vbl_nmi/rom_singles/04-nmi_control.nes",
        "ppu_vbl_nmi/rom_singles/05-nmi_timing.nes",
        "ppu_vbl_nmi/rom_singles/06-suppression.nes",
        "ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes",
        "ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes",
        "ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes",
        "ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes")]
    [InlineData("04-nmi_control.nes")]
    [InlineData("05-nmi_timing.nes")]
    [InlineData("06-suppression.nes")]
    [InlineData("07-nmi_on_timing.nes")]
    [InlineData("08-nmi_off_timing.nes")]
    [InlineData("09-even_odd_frames.nes")]
    [InlineData("10-even_odd_timing.nes")]
    public void REQ_CORE_116_ppu_vbl_nmi_tests_4_to_10_pass(string rom)
        => AssertShellPass($"ppu_vbl_nmi/rom_singles/{rom}");

    [RomTheory(
        "vbl_nmi_timing/1.frame_basics.nes",
        "vbl_nmi_timing/2.vbl_timing.nes",
        "vbl_nmi_timing/3.even_odd_frames.nes",
        "vbl_nmi_timing/4.vbl_clear_timing.nes",
        "vbl_nmi_timing/5.nmi_suppression.nes",
        "vbl_nmi_timing/6.nmi_disable.nes",
        "vbl_nmi_timing/7.nmi_timing.nes")]
    [InlineData("1.frame_basics.nes")]
    [InlineData("2.vbl_timing.nes")]
    [InlineData("3.even_odd_frames.nes")]
    [InlineData("4.vbl_clear_timing.nes")]
    [InlineData("5.nmi_suppression.nes")]
    [InlineData("6.nmi_disable.nes")]
    [InlineData("7.nmi_timing.nes")]
    public void REQ_CORE_116_vbl_nmi_timing_tests_pass(string rom)
        => AssertLegacyPass($"vbl_nmi_timing/{rom}");

    [RomFact("oam_read/oam_read.nes")]
    public void REQ_CORE_116_oam_read_passes()
        => AssertShellPass("oam_read/oam_read.nes");

    [RomFact("ppu_open_bus/ppu_open_bus.nes")]
    public void REQ_CORE_116_ppu_open_bus_passes()
        => AssertShellPass("ppu_open_bus/ppu_open_bus.nes");

    [RomTheory(
        "sprite_overflow_tests/1.Basics.nes",
        "sprite_overflow_tests/2.Details.nes",
        "sprite_overflow_tests/3.Timing.nes",
        "sprite_overflow_tests/4.Obscure.nes",
        "sprite_overflow_tests/5.Emulator.nes")]
    [InlineData("1.Basics.nes")]
    [InlineData("2.Details.nes")]
    [InlineData("3.Timing.nes")]
    [InlineData("4.Obscure.nes")]
    [InlineData("5.Emulator.nes")]
    public void REQ_CORE_116_sprite_overflow_tests_pass(string rom)
        => AssertLegacyPass($"sprite_overflow_tests/{rom}");

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
