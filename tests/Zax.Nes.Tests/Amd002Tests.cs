using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Tests;

/// <summary>
/// The normative answers of AMD-002 to the L1a question tickets: typed failure causes
/// (Q-001) and the frame-length tolerance after a region change (Q-003).
/// </summary>
public sealed class Amd002Tests
{
    // ---------------------------------------------------------------- Q-001

    [Fact]
    public void Q001_a_corrupt_header_reports_the_typed_cause()
    {
        var core = new NesCore();

        RomLoadResult tooShort = core.LoadRom(new byte[16], "Zeros.nes");
        Assert.False(tooShort.Success);
        Assert.Equal(RomLoadFailure.CorruptHeader, tooShort.Failure);
        Assert.NotNull(tooShort.Error);

        RomLoadResult empty = core.LoadRom([], "Empty.nes");
        Assert.Equal(RomLoadFailure.CorruptHeader, empty.Failure);
    }

    [Fact]
    public void Q001_a_truncated_image_reports_the_typed_cause()
    {
        byte[] image = TestRoms.Minimal();
        var core = new NesCore();

        RomLoadResult result = core.LoadRom(image.AsSpan(0, image.Length - 1), "Short.nes");

        Assert.False(result.Success);
        Assert.Equal(RomLoadFailure.TruncatedFile, result.Failure);
    }

    [Fact]
    public void Q001_an_unsupported_mapper_reports_the_typed_cause_and_still_names_the_number()
    {
        // Mapper 200: high nibble of byte 6 is 8, high nibble of byte 7 is 12 -> 0xC8 = 200.
        byte[] image = TestRoms.Minimal();
        image[6] = (byte)((image[6] & 0x0F) | 0x80);
        image[7] = (byte)((image[7] & 0x0F) | 0xC0);

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(image, "Mapper200.nes");

        Assert.False(result.Success);
        Assert.Equal(RomLoadFailure.UnsupportedMapper, result.Failure);

        // REQ-CORE-011 only requires the number to appear in the message; the typed cause
        // above is what AMD-002 makes normative.
        Assert.Contains("200", result.Error!, StringComparison.Ordinal);
    }

    [Fact]
    public void Q001_a_successful_load_reports_no_failure()
    {
        var core = new NesCore();
        RomLoadResult result = core.LoadRom(TestRoms.Minimal(), "Minimal.nes");

        Assert.True(result.Success);
        Assert.Equal(RomLoadFailure.None, result.Failure);
        Assert.Null(result.Error);
    }

    [Fact]
    public void Q001_a_corrupt_savestate_reports_the_typed_cause()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);
        core.RunFrame(0);

        byte[] state = core.SaveState();
        byte[] halved = state.AsSpan(0, state.Length / 2).ToArray();

        Assert.False(core.TryLoadState(
            halved,
            0,
            out _,
            out _,
            out StateLoadFailure failure,
            out string? error));

        Assert.Equal(StateLoadFailure.CorruptState, failure);
        Assert.NotNull(error);
        Assert.False(core.IsRunning);
    }

    [Fact]
    public void Q001_a_block_from_another_core_reports_the_foreign_core_cause_and_the_literal_message()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);
        core.RunFrame(0);

        byte[] state = core.SaveState();

        // The core name is a length-prefixed UTF-8 string right after the magic and the
        // format version; flipping one of its bytes makes the block foreign.
        state[12] ^= 0x20;

        Assert.False(core.TryLoadState(
            state,
            0,
            out _,
            out string romName,
            out StateLoadFailure failure,
            out string? error));

        Assert.Equal(StateLoadFailure.ForeignCore, failure);

        // REQ-CORE-120 is one of the few literal strings of the spec.
        Assert.Equal("Save state used a different core.", error);
        Assert.Equal(string.Empty, romName);
    }

    [Fact]
    public void Q001_a_successful_restore_reports_no_failure()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);
        core.RunFrame(0);

        byte[] state = core.SaveState();

        Assert.True(core.TryLoadState(
            state,
            0,
            out _,
            out _,
            out StateLoadFailure failure,
            out string? error));

        Assert.Equal(StateLoadFailure.None, failure);
        Assert.Null(error);
    }

    // ---------------------------------------------------------------- Q-003

    [Theory]
    [InlineData(NesRegion.Ntsc, NesRegion.Dendy)]
    [InlineData(NesRegion.Dendy, NesRegion.Ntsc)]
    [InlineData(NesRegion.Ntsc, NesRegion.Pal)]
    [InlineData(NesRegion.Pal, NesRegion.Dendy)]
    public void Q003_the_first_frame_after_a_region_change_stays_under_two_nominal_frames(
        NesRegion from,
        NesRegion to)
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.Region = from;
        for (int i = 0; i < 5; i++)
        {
            core.RunFrame(0);
        }

        core.Region = to;

        // AMD-002 (answer to Q-003): the transition frame may be of any length as long as it
        // does not exceed two nominal frames of the new region; the tolerance of
        // REQ-CORE-015 applies from the frame after it.
        long transition = core.RunFrame(0);
        double nominal = (double)RegionTiming.For(to).CpuClockHz / RegionTiming.For(to).NominalFramesPerSecond;

        Assert.InRange(transition, 1, (long)(2 * nominal));

        long total = 0;
        const int frames = 300;
        for (int frame = 0; frame < frames; frame++)
        {
            total += core.RunFrame(0);
        }

        double expected = to switch
        {
            NesRegion.Pal => 33247.5,
            NesRegion.Dendy => 35464.0,
            _ => 29780.5,
        };

        Assert.InRange((double)total / frames, expected - 1.0, expected + 1.0);
    }
}
