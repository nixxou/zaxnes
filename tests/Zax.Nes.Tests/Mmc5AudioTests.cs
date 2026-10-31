using Zax.Nes.Apu;

namespace Zax.Nes.Tests;

/// <summary>
/// The MMC5's expansion audio (REQ-CORE-075, AMD-054): two square channels, the PCM channel,
/// $5010 and $5015, and the way their output <c>M</c> joins the APU's <c>A</c> in the mixer
/// (REQ-CORE-073), read at the test point of REQ-CORE-076.
/// </summary>
public sealed class Mmc5AudioTests
{
    /// <summary>A machine with a synthetic MMC5 cartridge, powered on.</summary>
    private static NesCore Core()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(MapperFixture.Image(5), "Mmc5Audio.nes").Success);
        return core;
    }

    /// <summary>
    /// Sets square <paramref name="channel"/> (0 or 1) up, without touching $5015: the enable
    /// register is written on its own because clearing a bit puts that channel's length
    /// counter back to 0, so the two channels cannot be enabled one after the other.
    /// </summary>
    /// <param name="core">The machine.</param>
    /// <param name="channel">0 for $5000-$5003, 1 for $5004-$5007.</param>
    /// <param name="period">Timer period.</param>
    /// <param name="control">
    /// First register; the default is duty 2 (50 %), length halted, constant volume 15.
    /// </param>
    private static void ConfigureSquare(NesCore core, int channel, int period = 100, byte control = 0xBF)
    {
        ushort channelBase = (ushort)(0x5000 + (4 * channel));

        core.WriteMemory(channelBase, control);
        core.WriteMemory((ushort)(channelBase + 2), (byte)period);
        core.WriteMemory((ushort)(channelBase + 3), (byte)(0xF8 | ((period >> 8) & 0x07)));
    }

    /// <summary>Enables and starts square <paramref name="channel"/> alone.</summary>
    private static void StartSquare(NesCore core, int channel, int period = 100)
    {
        core.WriteMemory(0x5015, (byte)(0x01 << channel));
        ConfigureSquare(core, channel, period);
    }

    /// <summary>
    /// Enables and starts both squares on the same period, in phase: a write to $5003 or
    /// $5007 restarts the duty sequencer, and no cycle passes between the two.
    /// </summary>
    private static void StartBothSquares(NesCore core, int period = 100)
    {
        core.WriteMemory(0x5015, 0x03);
        ConfigureSquare(core, 0, period);
        ConfigureSquare(core, 1, period);
    }

    /// <summary>Largest <c>M</c> the test point reports over <paramref name="cycles"/> cycles.</summary>
    private static int PeakExpansion(NesCore core, int cycles = 4000)
    {
        int peak = 0;
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixerExpansion);
        }

        return peak;
    }

    // ---------------------------------------------------------------- REQ-CORE-075, levels

    /// <summary>
    /// REQ-CORE-075 (AMD-054), first criterion: one square channel at volume 15 gives
    /// <c>M</c> = 4 623, the two of them 8 000, and a PCM level of 128 gives 3 968.
    /// </summary>
    [Fact] // REQ-CORE-075, REQ-CORE-076
    public void REQ_CORE_075_the_board_produces_the_documented_integers()
    {
        NesCore one = Core();
        StartSquare(one, channel: 0);
        Assert.Equal(4623, PeakExpansion(one));

        NesCore other = Core();
        StartSquare(other, channel: 1);
        Assert.Equal(4623, PeakExpansion(other));

        // Both squares on the same period and started in phase.
        NesCore both = Core();
        StartBothSquares(both);
        Assert.Equal(8000, PeakExpansion(both));

        NesCore pcm = Core();
        pcm.WriteMemory(0x5011, 128);
        pcm.Bus.Tick();
        Assert.Equal(3968, pcm.Apu.MixerExpansion);
        Assert.Equal(31 * 128, pcm.Apu.MixerExpansion);
    }

    /// <summary>
    /// REQ-CORE-075 (AMD-054) — <c>Km = 8000 / P(30)</c>, so the table the board's squares use
    /// is the pulse function of REQ-CORE-073 scaled to top out at 8 000 instead of at the APU's
    /// 8 462, and the PCM channel steps by 31 per unit.
    /// </summary>
    [Fact] // REQ-CORE-075
    public void REQ_CORE_075_the_board_table_is_the_pulse_function_scaled_to_eight_thousand()
    {
        Assert.Equal(31, ApuMixer.Mmc5PulseTable.Length);
        Assert.Equal(0, ApuMixer.Mmc5PulseTable[0]);
        Assert.Equal(4623, ApuMixer.Mmc5PulseTable[15]);
        Assert.Equal(ApuMixer.Mmc5FullScale, ApuMixer.Mmc5PulseTable[30]);

        Assert.Equal(8000.0 / ApuMixer.Pulse(30), ApuMixer.Mmc5Normalisation, 6);
        Assert.Equal(30_949.798, ApuMixer.Mmc5Normalisation, 3);

        for (int i = 0; i <= 30; i++)
        {
            Assert.Equal(ApuMixer.Round(ApuMixer.Mmc5Normalisation * ApuMixer.Pulse(i)), ApuMixer.Mmc5PulseTable[i]);
        }

        Assert.Equal(31, ApuMixer.Mmc5PcmStep);
    }

    /// <summary>
    /// REQ-CORE-073 (AMD-054) — <c>M</c> is added to <c>A</c> and the sum is clipped to 32 767,
    /// never wrapped: the board's full scale on top of the APU's would reach 48 672.
    /// </summary>
    [Fact] // REQ-CORE-073, REQ-CORE-075, REQ-CORE-076
    public void REQ_CORE_073_the_board_output_is_added_to_the_apu_and_the_sum_is_clipped()
    {
        NesCore core = Core();

        // A DMC level written through $4011 is a pure step, so A sits still and the sum is
        // easy to read: A = T(100), M = 31 x 100.
        core.WriteMemory(0x4011, 100);
        core.WriteMemory(0x5011, 100);
        core.Bus.Tick();

        Assert.Equal(ApuMixer.TndTable[100], core.Apu.MixerApu);
        Assert.Equal(3100, core.Apu.MixerExpansion);
        Assert.Equal(core.Apu.MixerApu + 3100, core.Apu.MixerOutput);
        Assert.Equal(core.Apu.MixerOutput / 32_768.0, core.Apu.MixedLevel, 12);

        // Both pulses at volume 15 on top of a DMC level of 127 take A to 8 462 + 18 376,
        // and a PCM level of 255 takes M to 7 905: the sum is past full scale, where
        // REQ-CORE-073 asks for a clip rather than a wrap.
        core.WriteMemory(0x4015, 0x03);
        core.WriteMemory(0x4000, 0xBF);
        core.WriteMemory(0x4001, 0x08); // Negate with a shift of 0: the sweep never mutes.
        core.WriteMemory(0x4002, 100);
        core.WriteMemory(0x4003, 0xF8);
        core.WriteMemory(0x4004, 0xBF);
        core.WriteMemory(0x4005, 0x08);
        core.WriteMemory(0x4006, 100);
        core.WriteMemory(0x4007, 0xF8);
        core.WriteMemory(0x4011, 127);
        core.WriteMemory(0x5011, 255);

        int peakApu = 0;
        int peakOutput = 0;
        double peakLevel = 0.0;
        for (int i = 0; i < 4000; i++)
        {
            core.Bus.Tick();
            if (core.Apu.MixerApu > peakApu)
            {
                peakApu = core.Apu.MixerApu;
                peakOutput = core.Apu.MixerOutput;
                peakLevel = core.Apu.MixedLevel;
            }
        }

        Assert.Equal(ApuMixer.PulseTable[30] + ApuMixer.TndTable[127], peakApu);
        Assert.Equal(31 * 255, core.Apu.MixerExpansion);
        Assert.True(peakApu + (31 * 255) > ApuMixer.FullScale);
        Assert.Equal(ApuMixer.FullScale, peakOutput);
        Assert.InRange(peakLevel, 0.99996, 1.0);
    }

    // ---------------------------------------------------------------- REQ-CORE-075, registers

    /// <summary>
    /// REQ-CORE-075 (AMD-054), first criterion, second half: a write of 0 to $5011 leaves the
    /// level alone and raises the PCM IRQ flag; a read of $5010 then returns $80 and the next
    /// one $00. The flag never pulls the CPU's IRQ line (an accepted divergence).
    /// </summary>
    [Fact] // REQ-CORE-075
    public void REQ_CORE_075_a_write_of_zero_to_5011_keeps_the_level_and_raises_the_pcm_irq()
    {
        NesCore core = Core();

        core.WriteMemory(0x5010, 0x80); // PCM IRQ enabled, write mode.
        core.WriteMemory(0x5011, 200);
        core.Bus.Tick();
        Assert.Equal(31 * 200, core.Apu.MixerExpansion);
        Assert.Equal(0x00, core.Bus.Read(0x5010));

        core.WriteMemory(0x5011, 0x00);
        core.Bus.Tick();

        // The level is unchanged, and the flag is up.
        Assert.Equal(31 * 200, core.Apu.MixerExpansion);
        Assert.Equal(0x80, core.Bus.Read(0x5010));

        // Reading it acknowledged it.
        Assert.Equal(0x00, core.Bus.Read(0x5010));

        // And it is not the CPU's IRQ line.
        Assert.False(core.Bus.IrqAsserted);
    }

    /// <summary>
    /// REQ-CORE-081 — the host and Lua accessors read $4020-$5FFF without side effects, so
    /// peeking at $5010 must not acknowledge the PCM IRQ flag.
    /// </summary>
    [Fact] // REQ-CORE-075, REQ-CORE-081
    public void REQ_CORE_081_peeking_at_5010_does_not_acknowledge_the_pcm_irq()
    {
        NesCore core = Core();

        core.WriteMemory(0x5011, 0x00);
        Assert.Equal(0x80, core.ReadMemory(0x5010));
        Assert.Equal(0x80, core.ReadMemory(0x5010));
        Assert.Equal(0x80, core.Bus.Read(0x5010));
        Assert.Equal(0x00, core.ReadMemory(0x5010));
    }

    /// <summary>
    /// REQ-CORE-075 (AMD-054) — $5015: bits 0 and 1 enable the two squares, clearing one puts
    /// its length counter to 0, and a read reports the two counters.
    /// </summary>
    [Fact] // REQ-CORE-075
    public void REQ_CORE_075_5015_enables_the_squares_and_reports_their_length_counters()
    {
        NesCore core = Core();

        Assert.Equal(0x00, core.Bus.Read(0x5015));

        StartBothSquares(core);
        core.Bus.Tick();

        Assert.Equal(0x03, core.Bus.Read(0x5015));
        Assert.True(PeakExpansion(core) > 0);

        // Clearing bit 0 silences that channel at once: its length counter goes to 0.
        core.WriteMemory(0x5015, 0x02);
        core.Bus.Tick();

        Assert.Equal(0x02, core.Bus.Read(0x5015));
        Assert.Equal(4623, PeakExpansion(core));

        core.WriteMemory(0x5015, 0x00);
        core.Bus.Tick();

        Assert.Equal(0x00, core.Bus.Read(0x5015));
        Assert.Equal(0, PeakExpansion(core));
    }

    /// <summary>
    /// REQ-CORE-075 (AMD-054) — the squares take the duty, the timer and the muting of periods
    /// below 8 from the APU's pulses, and have <strong>no sweep unit</strong>: writing the
    /// register an APU pulse would read as a sweep setting changes nothing, and in particular
    /// a period above $3FF — which an APU pulse with a zeroed sweep register mutes — stays
    /// audible.
    /// </summary>
    [Theory] // REQ-CORE-075
    [InlineData(7, false)] // Below 8: muted, as on an APU pulse.
    [InlineData(8, true)]
    [InlineData(0x400, true)] // An APU pulse with $4001 = 0 would mute this one.
    [InlineData(0x7FF, true)]
    public void REQ_CORE_075_the_squares_have_no_sweep_unit(int period, bool audible)
    {
        NesCore core = Core();
        StartSquare(core, channel: 0, period: period);
        core.WriteMemory(0x5001, 0x00); // The APU's sweep register: ignored by the board.

        Assert.Equal(audible, PeakExpansion(core, 8 * 2 * (period + 1)) > 0);
    }

    [Theory] // REQ-CORE-075
    // The four duty sequences of an APU pulse, with the same high fractions.
    [InlineData(0, 1.0 / 8.0)]
    [InlineData(1, 2.0 / 8.0)]
    [InlineData(2, 4.0 / 8.0)]
    [InlineData(3, 6.0 / 8.0)]
    public void REQ_CORE_075_the_squares_use_the_duty_sequences_of_an_apu_pulse(int duty, double expected)
    {
        NesCore core = Core();
        StartSquare(core, channel: 0);
        core.WriteMemory(0x5000, (byte)((duty << 6) | 0x3F)); // Length halted, constant volume 15.

        // Twenty full sequences: 8 steps of (period + 1) APU cycles, two CPU cycles each.
        const int cycles = 20 * 8 * 2 * 101;
        int high = 0;
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            if (core.Apu.MixerExpansion > 0)
            {
                high++;
            }
        }

        Assert.InRange((double)high / cycles, expected - 0.01, expected + 0.01);
    }

    // ---------------------------------------------------------------- REQ-CORE-075, cadence

    /// <summary>
    /// REQ-CORE-075 (AMD-054), second criterion: a square with a decaying envelope (start
    /// volume 15, envelope period 0) goes from 15 to 0 in 15 × 1/240 s ± 5 %, whatever $4017
    /// holds — the board has no frame sequencer, so its cadence is fixed.
    /// </summary>
    [Theory] // REQ-CORE-075
    [InlineData(0x00)]
    [InlineData(0x80)]
    [InlineData(0xC0)]
    public void REQ_CORE_075_the_envelope_runs_at_a_fixed_cadence_whatever_4017_holds(byte frameCounter)
    {
        NesCore core = Core();

        core.WriteMemory(0x4017, frameCounter);

        core.WriteMemory(0x5015, 0x01);
        core.WriteMemory(0x5000, 0x80); // Duty 2, envelope mode, period 0, no loop.
        core.WriteMemory(0x5002, ShortPeriod);
        core.WriteMemory(0x5003, 0x08); // Length index 1 (254 steps at 120 Hz), envelope restart.

        // Wait for the first envelope clock, which loads the decay with 15, then time the
        // fifteen clocks that take it back to 0.
        long cycles = 0;
        long start = -1;
        long end = -1;
        while (cycles < 400_000 && end < 0)
        {
            int volume = StepOneSequence(core);
            cycles += SequenceCycles;

            if (start < 0)
            {
                if (volume == 15)
                {
                    start = cycles;
                }
            }
            else if (volume == 0)
            {
                end = cycles;
            }
        }

        Assert.True(start >= 0, "the envelope never reached 15.");
        Assert.True(end > start, "the envelope never reached 0.");

        Assert.InRange(Seconds(core, end - start), 15.0 / 240.0 * 0.95, 15.0 / 240.0 * 1.05);
    }

    /// <summary>
    /// REQ-CORE-075 (AMD-054) — the length counter runs at 120 Hz, one envelope clock in two:
    /// a counter loaded with 10 steps therefore silences the channel after 10 / 120 s.
    /// </summary>
    [Fact] // REQ-CORE-075
    public void REQ_CORE_075_the_length_counter_runs_at_half_the_envelope_cadence()
    {
        NesCore core = Core();

        core.WriteMemory(0x5015, 0x01);
        core.WriteMemory(0x5000, 0x9F); // Duty 2, constant volume 15, length running.
        core.WriteMemory(0x5002, ShortPeriod);
        core.WriteMemory(0x5003, 0x00); // Length index 0: 10 steps.

        long cycles = 0;
        long silent = -1;
        while (cycles < 400_000 && silent < 0)
        {
            int volume = StepOneSequence(core);
            cycles += SequenceCycles;
            if (volume == 0)
            {
                silent = cycles;
            }
        }

        Assert.True(silent > 0, "the length counter never ran out.");
        Assert.Equal(0x00, core.Bus.Read(0x5015));

        Assert.InRange(Seconds(core, silent), 10.0 / 120.0 * 0.95, 10.0 / 120.0 * 1.05);
    }

    /// <summary>
    /// REQ-CORE-090, REQ-CORE-091 — the board's sound is part of the savestate: restoring one
    /// taken mid-tone brings the level, the envelope and the PCM level back.
    /// </summary>
    [Fact] // REQ-CORE-090, REQ-CORE-091
    public void REQ_CORE_090_the_board_sound_survives_a_savestate_round_trip()
    {
        NesCore core = Core();
        StartSquare(core, channel: 0);
        core.WriteMemory(0x5011, 77);

        for (int i = 0; i < 500; i++)
        {
            core.Bus.Tick();
        }

        byte[] state = core.SaveState();
        var expected = new int[256];
        for (int i = 0; i < expected.Length; i++)
        {
            core.Bus.Tick();
            expected[i] = core.Apu.MixerExpansion;
        }

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);
        for (int i = 0; i < expected.Length; i++)
        {
            core.Bus.Tick();
            Assert.Equal(expected[i], core.Apu.MixerExpansion);
        }
    }

    /// <summary>
    /// REQ-CORE-004 — power-on leaves the board silent, and the APU reads 0 at the test point
    /// of REQ-CORE-076.
    /// </summary>
    [Fact] // REQ-CORE-004, REQ-CORE-076
    public void REQ_CORE_004_the_board_powers_on_silent()
    {
        NesCore core = Core();
        StartSquare(core, channel: 0);
        core.WriteMemory(0x5011, 200);
        core.Bus.Tick();
        Assert.True(core.Apu.MixerExpansion > 0);

        core.Reset();
        core.Bus.Tick();

        Assert.Equal(0, core.Apu.MixerExpansion);
        Assert.Equal(0, core.Apu.MixerOutput);
        Assert.Equal(0, PeakExpansion(core));
    }

    /// <summary>
    /// Shortest timer period that is not muted (REQ-CORE-075 keeps the APU's cut below 8), so
    /// that one turn of the duty sequencer is as short as possible and the cadence tests can
    /// read the envelope's volume often.
    /// </summary>
    private const int ShortPeriod = 8;

    /// <summary>
    /// CPU cycles one turn of the duty sequencer takes at <see cref="ShortPeriod"/>: eight
    /// steps of nine APU cycles, each of two CPU cycles.
    /// </summary>
    private const int SequenceCycles = 8 * (ShortPeriod + 1) * 2;

    /// <summary>
    /// Runs one turn of the duty sequencer and returns the volume the first square was driven
    /// at over it: the channel's output is only non-zero for part of a turn, so the volume has
    /// to be read from the peak rather than from any one cycle.
    /// </summary>
    private static int StepOneSequence(NesCore core)
    {
        int peak = 0;
        for (int i = 0; i < SequenceCycles; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixerExpansion);
        }

        for (int volume = 0; volume <= 15; volume++)
        {
            if (ApuMixer.Mmc5PulseTable[volume] == peak)
            {
                return volume;
            }
        }

        return -1;
    }

    /// <summary>Seconds <paramref name="cycles"/> CPU cycles last, on the current region.</summary>
    private static double Seconds(NesCore core, long cycles)
    {
        double cyclesPerFrame =
            (double)RegionTiming.DotsPerScanline * core.Timing.ScanlinesPerFrame
            * core.Timing.DotsPerCycleDenominator / core.Timing.DotsPerCycleNumerator;

        return cycles / (core.Timing.NominalFramesPerSecond * cyclesPerFrame);
    }
}
