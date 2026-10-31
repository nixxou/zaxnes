using Zax.Nes;
using Zax.Nes.Apu;
using Zax.Nes.Rom;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-054 — the test point of REQ-CORE-076: the <c>mixer</c> command reads the three integers
/// <c>A</c>, <c>M</c> and <c>S</c> of REQ-CORE-073 at a chosen instant of the emulation, which
/// is where every integer criterion of the amendment is measured.
/// </summary>
public sealed class Amd054HarnessTests
{
    /// <summary>A machine on the command's own built cartridge, as a run without --rom has.</summary>
    private static NesCore Core(int mapper)
    {
        var core = new NesCore();
        RomLoadResult load = core.LoadRom(MixerProbe.BuildTestCartridge(mapper), "mixer.nes");
        Assert.True(load.Success, load.Error);
        return core;
    }

    /// <summary>The command and its three integers are declared in the usage text.</summary>
    [Fact] // REQ-CORE-076
    public void The_test_point_is_documented()
    {
        Assert.Contains("--poke <a>=<v>", MixerProbe.Usage, StringComparison.Ordinal);
        Assert.Contains("A=<a> M=<m> S=<s>", MixerProbe.Usage, StringComparison.Ordinal);
        Assert.Contains("REQ-CORE-076", MixerProbe.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-CORE-076, acceptance criterion: silence reads <c>A = M = S = 0</c>.
    /// </summary>
    [Fact] // REQ-CORE-076
    public void Silence_reads_zero_at_the_test_point()
    {
        MixerProbe.Reading reading = MixerProbe.Measure(Core(0), [], cycles: 2000, peak: true);

        Assert.Equal(0, reading.Apu);
        Assert.Equal(0, reading.Expansion);
        Assert.Equal(0, reading.Output);
        Assert.Equal("A=0 M=0 S=0", reading.ToString());
        Assert.Equal("{\"a\":0,\"m\":0,\"s\":0}", reading.ToJson());
    }

    /// <summary>
    /// REQ-CORE-073, first criterion, through the command: a lone pulse at volume 15 and 50 %
    /// duty peaks at <c>A</c> = 4 890, which is what the amendment's figures are read on.
    /// </summary>
    [Fact] // REQ-CORE-073, REQ-CORE-076
    public void A_lone_pulse_reads_the_documented_integer()
    {
        (ushort, byte)[] pokes =
        [
            (0x4015, 0x01), // Pulse 1 enabled.
            (0x4000, 0xBF), // Duty 50 %, constant volume 15, length halted.
            (0x4002, 100),  // Timer period.
            (0x4003, 0xF8),
        ];

        MixerProbe.Reading reading = MixerProbe.Measure(Core(0), pokes, cycles: 4000, peak: true);

        Assert.Equal(4890, reading.Apu);
        Assert.Equal(0, reading.Expansion);
        Assert.Equal(4890, reading.Output);
    }

    /// <summary>
    /// REQ-CORE-075, first criterion, through the command: on an MMC5 cartridge a PCM level of
    /// 128 reads <c>M</c> = 3 968, and the sum <c>S</c> carries it.
    /// </summary>
    [Fact] // REQ-CORE-075, REQ-CORE-076
    public void The_board_pcm_level_reads_the_documented_integer()
    {
        MixerProbe.Reading reading =
            MixerProbe.Measure(Core(5), [((ushort)0x5011, (byte)128)], cycles: 1, peak: false);

        Assert.Equal(0, reading.Apu);
        Assert.Equal(3968, reading.Expansion);
        Assert.Equal(3968, reading.Output);
        Assert.Equal(ApuMixer.Mmc5PcmStep * 128, reading.Expansion);
    }

    /// <summary>Addresses and values are hexadecimal, or decimal behind a <c>d</c>.</summary>
    [Theory] // REQ-CORE-076
    [InlineData("5011=80", 0x5011, 0x80)]
    [InlineData("4015=0F", 0x4015, 0x0F)]
    [InlineData("d20497=d128", 20_497, 128)]
    public void A_poke_parses_in_either_base(string text, int address, int value)
    {
        Assert.True(MixerProbe.TryParsePoke(text, out (ushort Address, byte Value) poke));
        Assert.Equal(address, poke.Address);
        Assert.Equal(value, poke.Value);
    }

    [Theory] // REQ-CORE-076
    [InlineData("5011")]      // No '='.
    [InlineData("=80")]       // No address.
    [InlineData("5011=")]     // No value.
    [InlineData("10000=01")]  // Address past the CPU bus.
    [InlineData("5011=100")]  // Value past a byte.
    [InlineData("zz=01")]     // Not a number.
    public void A_malformed_poke_is_refused(string text)
        => Assert.False(MixerProbe.TryParsePoke(text, out _));
}
