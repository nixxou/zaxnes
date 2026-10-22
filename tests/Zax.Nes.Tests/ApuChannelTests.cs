namespace Zax.Nes.Tests;

/// <summary>
/// The five sound channels, driven through their registers and observed through the mixed
/// output level and $4015 (REQ-CORE-004, REQ-CORE-015, REQ-CORE-073, REQ-CORE-113).
/// </summary>
/// <remarks>
/// The test ROMs of <see cref="ApuRomTests"/> cover what the CPU can measure: length
/// counters, the frame counter and the DMC's memory reader. What they cannot check is the
/// sound itself — the duty sequences, the envelope, the sweep, the triangle's staircase, the
/// noise shift register and the region period tables — which is what this file is for.
/// </remarks>
public sealed class ApuChannelTests
{
    /// <summary>Level of one pulse step at volume 15 (REQ-CORE-073).</summary>
    private const double PulseFull = 1.526 * 0.00752 * 15;

    /// <summary>Sweep register value that never mutes: negate with a shift of 0.</summary>
    private const byte SweepNeverMutes = 0x08;

    private static NesCore Core(NesRegion region = NesRegion.Ntsc)
    {
        var core = new NesCore { Region = region };
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Apu.nes").Success);
        core.Region = region;
        return core;
    }

    private static void Tick(NesCore core, int cycles)
    {
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
        }
    }

    /// <summary>Highest mixed level seen over <paramref name="cycles"/> CPU cycles.</summary>
    private static double Peak(NesCore core, int cycles)
    {
        double peak = double.NegativeInfinity;
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixedLevel);
        }

        return peak;
    }

    /// <summary>Fraction of <paramref name="cycles"/> CPU cycles the output is above zero.</summary>
    private static double HighFraction(NesCore core, int cycles)
    {
        int high = 0;
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            if (core.Apu.MixedLevel > 0.0)
            {
                high++;
            }
        }

        return (double)high / cycles;
    }

    /// <summary>Number of times the mixed level changes over <paramref name="cycles"/> cycles.</summary>
    private static int Transitions(NesCore core, int cycles)
    {
        double previous = core.Apu.MixedLevel;
        int changes = 0;

        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            if (core.Apu.MixedLevel != previous)
            {
                changes++;
                previous = core.Apu.MixedLevel;
            }
        }

        return changes;
    }

    /// <summary>Starts pulse 1 at constant volume 15 with timer period <paramref name="period"/>.</summary>
    private static void StartPulse1(NesCore core, int duty, int period, byte sweep = SweepNeverMutes)
    {
        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, (byte)((duty << 6) | 0x10 | 0x0F));
        core.WriteMemory(0x4001, sweep);
        core.WriteMemory(0x4002, (byte)period);
        core.WriteMemory(0x4003, (byte)(0xF8 | ((period >> 8) & 0x07))); // Length index 31 → 30.
    }

    // ---------------------------------------------------------------- pulse

    [Theory]
    // The four duty sequences of nesdev: 1, 2, 4 and 6 of the eight steps are high.
    [InlineData(0, 1.0 / 8.0)]
    [InlineData(1, 2.0 / 8.0)]
    [InlineData(2, 4.0 / 8.0)]
    [InlineData(3, 6.0 / 8.0)]
    public void REQ_CORE_073_each_duty_sequence_holds_its_documented_fraction_high(int duty, double expected)
    {
        NesCore core = Core();
        StartPulse1(core, duty, period: 100);

        // Twenty full sequences: 8 steps of (period + 1) APU cycles, two CPU cycles each.
        double fraction = HighFraction(core, 20 * 8 * 2 * 101);

        Assert.InRange(fraction, expected - 0.01, expected + 0.01);
    }

    [Fact]
    public void REQ_CORE_073_a_pulse_at_volume_fifteen_swings_between_zero_and_its_step_value()
    {
        NesCore core = Core();
        StartPulse1(core, duty: 2, period: 100);

        var levels = new HashSet<double>();
        for (int i = 0; i < 4000; i++)
        {
            core.Bus.Tick();
            levels.Add(core.Apu.MixedLevel);
        }

        Assert.Equal(2, levels.Count);
        Assert.Contains(0.0, levels);
        Assert.Contains(levels, level => Math.Abs(level - PulseFull) < 1e-9);
    }

    [Theory]
    // nesdev, "APU Envelope": the start flag reloads the decay to 15, then each quarter frame
    // steps it down by one once the divider (period 0 here) expires.
    [InlineData(1, 15)]
    [InlineData(2, 14)]
    [InlineData(6, 10)]
    [InlineData(16, 0)]
    public void REQ_CORE_073_the_envelope_decays_one_step_per_quarter_frame(int quarterFrames, int expectedVolume)
    {
        NesCore core = Core();

        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0x80); // Duty 2, envelope mode, period 0, no loop.
        core.WriteMemory(0x4001, SweepNeverMutes);
        core.WriteMemory(0x4002, 10);
        core.WriteMemory(0x4003, 0xF8); // Sets the envelope's start flag.

        // Writing $80 to $4017 generates a quarter and a half frame straight away (nesdev),
        // which is a precise way to clock the envelope a known number of times.
        for (int i = 0; i < quarterFrames; i++)
        {
            core.WriteMemory(0x4017, 0x80);
            Tick(core, 2);
        }

        double peak = Peak(core, 200);

        Assert.InRange(peak, (expectedVolume * PulseFull / 15) - 1e-9, (expectedVolume * PulseFull / 15) + 1e-9);
    }

    [Fact]
    public void REQ_CORE_073_a_looping_envelope_reloads_to_fifteen_instead_of_staying_at_zero()
    {
        NesCore core = Core();

        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0xA0); // Duty 2, envelope mode, loop set, period 0.
        core.WriteMemory(0x4001, SweepNeverMutes);
        core.WriteMemory(0x4002, 10);
        core.WriteMemory(0x4003, 0xF8);

        for (int i = 0; i < 17; i++)
        {
            core.WriteMemory(0x4017, 0x80);
            Tick(core, 2);
        }

        // Sixteen quarter frames take the decay to 0; the seventeenth wraps it back to 15.
        Assert.InRange(Peak(core, 200), PulseFull - 1e-9, PulseFull + 1e-9);
    }

    [Theory]
    // nesdev, "APU Sweep": a period below 8, or a target period above $7FF, mutes the
    // channel, and the target is recomputed whether the sweep is enabled or not.
    [InlineData(7, SweepNeverMutes, false)]
    [InlineData(8, SweepNeverMutes, true)]
    [InlineData(0x7FF, SweepNeverMutes, true)]
    [InlineData(0x400, 0x00, false)] // Shift 0, no negate: target = 2 x period > $7FF.
    [InlineData(0x100, 0x00, true)]
    public void REQ_CORE_073_the_sweep_unit_mutes_a_period_out_of_range(int period, byte sweep, bool audible)
    {
        NesCore core = Core();
        StartPulse1(core, duty: 2, period: period, sweep: sweep);

        double peak = Peak(core, 8 * 2 * (period + 1));

        Assert.Equal(audible, peak > 0.0);
    }

    [Fact]
    public void REQ_CORE_073_a_negative_sweep_halves_the_period_and_doubles_the_frequency()
    {
        NesCore core = Core();
        StartPulse1(core, duty: 2, period: 0x100, sweep: 0x89); // Enabled, period 0, negate, shift 1.

        const int window = 8 * 2 * 0x101;
        int before = Transitions(core, window);

        // One half frame loads the target period, $100 − $80 − 1 = $7F (nesdev: pulse 1
        // negates with a one's complement).
        core.WriteMemory(0x4017, 0x80);
        Tick(core, 2);

        int after = Transitions(core, window);

        Assert.InRange(after, 2 * before - 4, 2 * before + 4);
    }

    // ---------------------------------------------------------------- triangle

    [Fact]
    public void REQ_CORE_073_the_triangle_walks_its_staircase_in_steps_of_one()
    {
        NesCore core = Core();
        const double step = 1.526 * 0.00851;

        core.WriteMemory(0x4015, 0x04);
        core.WriteMemory(0x4008, 0x7F); // Control clear, linear counter reload 127.
        core.WriteMemory(0x400A, 50);
        core.WriteMemory(0x400B, 0xF8); // Length index 31, linear reload flag.
        core.WriteMemory(0x4017, 0x80); // Quarter frame: load the linear counter.

        var levels = new SortedSet<double>();
        for (int i = 0; i < 40 * 51; i++)
        {
            core.Bus.Tick();
            levels.Add(core.Apu.MixedLevel);
        }

        // The staircase visits all sixteen output values, one step apart.
        Assert.Equal(16, levels.Count);
        double[] sorted = [.. levels];
        for (int i = 1; i < sorted.Length; i++)
        {
            Assert.InRange(sorted[i] - sorted[i - 1], step - 1e-9, step + 1e-9);
        }
    }

    [Fact]
    public void REQ_CORE_073_the_triangle_holds_its_level_while_the_linear_counter_is_zero()
    {
        NesCore core = Core();

        core.WriteMemory(0x4015, 0x04);
        core.WriteMemory(0x4008, 0x00); // Control clear, reload value 0: the counter stays 0.
        core.WriteMemory(0x400A, 50);
        core.WriteMemory(0x400B, 0xF8);
        core.WriteMemory(0x4017, 0x80);

        Assert.Equal(0, Transitions(core, 20 * 51));
    }

    [Fact]
    public void REQ_CORE_073_the_triangle_is_held_when_its_period_is_ultrasonic()
    {
        NesCore core = Core();

        core.WriteMemory(0x4015, 0x04);
        core.WriteMemory(0x4008, 0x7F);
        core.WriteMemory(0x400A, 1); // Period 1: below the audible range.
        core.WriteMemory(0x400B, 0xF8);
        core.WriteMemory(0x4017, 0x80);

        Assert.Equal(0, Transitions(core, 4000));
    }

    // ---------------------------------------------------------------- noise

    /// <summary>
    /// Samples the noise output once per shift of the register, which with period index 0
    /// happens every four CPU cycles, the first one on the very next cycle.
    /// </summary>
    private static bool[] NoisePattern(NesCore core, int shifts)
    {
        var pattern = new bool[shifts];
        core.Bus.Tick();
        pattern[0] = core.Apu.MixedLevel > 0.0;

        for (int i = 1; i < shifts; i++)
        {
            Tick(core, 4);
            pattern[i] = core.Apu.MixedLevel > 0.0;
        }

        return pattern;
    }

    private static void StartNoise(NesCore core, byte period)
    {
        core.WriteMemory(0x4015, 0x08);
        core.WriteMemory(0x400C, 0x1F); // Constant volume 15, no halt.
        core.WriteMemory(0x400E, period);
        core.WriteMemory(0x400F, 0xF8); // Length index 31 → 30.
    }

    [Fact]
    public void REQ_CORE_073_the_noise_register_repeats_every_ninety_three_shifts_in_short_mode()
    {
        NesCore core = Core();
        StartNoise(core, 0x80); // Mode flag set: feedback from bit 6.

        bool[] pattern = NoisePattern(core, 300);

        for (int i = 93; i < pattern.Length; i++)
        {
            Assert.Equal(pattern[i - 93], pattern[i]);
        }
    }

    [Fact]
    public void REQ_CORE_073_the_noise_register_does_not_repeat_within_ninety_three_shifts_in_long_mode()
    {
        NesCore core = Core();
        StartNoise(core, 0x00); // Feedback from bit 1: a period of 32 767 shifts.

        bool[] pattern = NoisePattern(core, 300);

        Assert.Contains(Enumerable.Range(93, pattern.Length - 93), i => pattern[i - 93] != pattern[i]);
    }

    [Theory]
    // Noise period tables of REQ-CORE-015: the PAL table in PAL, the NTSC one in NTSC, MULTI
    // and DENDY. Every gap between two output changes is a whole number of shifts, and a
    // maximal shift register flips bit 0 on consecutive shifts often enough that the shortest
    // gap seen is exactly one period.
    [InlineData(NesRegion.Ntsc, 0, 4)]
    [InlineData(NesRegion.Pal, 0, 4)]
    [InlineData(NesRegion.Ntsc, 2, 16)]
    [InlineData(NesRegion.Pal, 2, 14)]
    [InlineData(NesRegion.Dendy, 2, 16)]
    [InlineData(NesRegion.Multi, 2, 16)]
    [InlineData(NesRegion.Ntsc, 4, 64)]
    [InlineData(NesRegion.Pal, 4, 60)]
    public void REQ_CORE_015_the_noise_period_table_follows_the_region(
        NesRegion region, byte periodIndex, int cpuCyclesPerShift)
    {
        NesCore core = Core(region);
        StartNoise(core, periodIndex);

        // The first gap starts from whatever phase the timer was left in by the power-on
        // sequence, so it is dropped.
        CyclesToNextTransition(core);

        int shortest = int.MaxValue;
        for (int i = 0; i < 80; i++)
        {
            shortest = Math.Min(shortest, CyclesToNextTransition(core));
        }

        Assert.Equal(cpuCyclesPerShift, shortest);
    }

    private static int CyclesToNextTransition(NesCore core)
    {
        double start = core.Apu.MixedLevel;
        for (int cycles = 1; cycles <= 400_000; cycles++)
        {
            core.Bus.Tick();
            if (core.Apu.MixedLevel != start)
            {
                return cycles;
            }
        }

        return -1;
    }

    // ---------------------------------------------------------------- DMC

    [Fact]
    public void REQ_CORE_073_a_write_to_4011_loads_the_dmc_output_level_directly()
    {
        NesCore core = Core();

        core.WriteMemory(0x4011, 64);
        core.Bus.Tick();

        Assert.InRange(core.Apu.MixedLevel, (1.526 * 0.00335 * 64) - 1e-9, (1.526 * 0.00335 * 64) + 1e-9);

        core.WriteMemory(0x4011, 0xFF); // Only the low seven bits are kept.
        core.Bus.Tick();

        Assert.InRange(core.Apu.MixedLevel, (1.526 * 0.00335 * 127) - 1e-9, (1.526 * 0.00335 * 127) + 1e-9);
    }

    /// <summary>Starts a DMC sample of <c>16 x length + 1</c> bytes at <paramref name="rate"/>.</summary>
    private static void StartDmc(NesCore core, int rate, byte length, bool irq, bool loop)
    {
        core.WriteMemory(0x4010, (byte)((irq ? 0x80 : 0) | (loop ? 0x40 : 0) | rate));
        core.WriteMemory(0x4012, 0x00); // Sample at $C000.
        core.WriteMemory(0x4013, length);
        core.WriteMemory(0x4015, 0x10);
    }

    [Fact]
    public void REQ_CORE_113_a_dmc_sample_fetch_steals_four_cpu_cycles()
    {
        NesCore core = Core();
        StartDmc(core, rate: 15, length: 2, irq: false, loop: false);

        long before = core.CycleCount;
        const int ticks = 4000;
        Tick(core, ticks);

        long stolen = core.CycleCount - before - ticks;

        Assert.True(core.Bus.DmcDmaCount > 0);
        Assert.Equal(NesBus.DmcDmaCycles * core.Bus.DmcDmaCount, stolen);
    }

    [Theory]
    // DMC rate tables of REQ-CORE-015: rate 15 is 54 CPU cycles per bit in NTSC and 50 in
    // PAL, the NTSC table serving MULTI and DENDY too. A sample raises its IRQ once the last
    // byte has been fetched, so lengthening it by 32 bytes costs exactly 256 more bits. The
    // difference between two lengths is measured rather than one absolute duration, because
    // the DMC's timer is left mid-period by the power-on sequence.
    [InlineData(NesRegion.Ntsc, 54)]
    [InlineData(NesRegion.Pal, 50)]
    [InlineData(NesRegion.Dendy, 54)]
    [InlineData(NesRegion.Multi, 54)]
    public void REQ_CORE_015_the_dmc_rate_table_follows_the_region(NesRegion region, int cyclesPerBit)
    {
        long shortSample = CyclesToDmcIrq(region, length: 1); // 17 bytes.
        long longSample = CyclesToDmcIrq(region, length: 3);  // 49 bytes.

        Assert.Equal(32 * 8 * cyclesPerBit, longSample - shortSample);
    }

    private static long CyclesToDmcIrq(NesRegion region, byte length)
    {
        NesCore core = Core(region);
        StartDmc(core, rate: 15, length: length, irq: true, loop: false);

        long before = core.CycleCount;
        while (!core.Apu.IrqAsserted && core.CycleCount - before < 200_000)
        {
            core.Bus.Tick();
        }

        Assert.True(core.Apu.IrqAsserted);

        // The channel keeps clocking while a fetch halts the CPU, so the cycles the DMA stole
        // are part of its own timeline and are counted here as well.
        return core.CycleCount - before;
    }

    [Fact]
    public void REQ_CORE_113_4015_reports_the_dmc_and_its_irq()
    {
        NesCore core = Core();
        StartDmc(core, rate: 15, length: 1, irq: true, loop: false);
        Tick(core, 10);

        Assert.Equal(0x10, core.Apu.PeekStatus() & 0x10); // Bit 4: a sample is playing.

        while (!core.Apu.IrqAsserted)
        {
            core.Bus.Tick();
        }

        Assert.Equal(0x80, core.Apu.PeekStatus() & 0x80); // Bit 7: the DMC IRQ.
        Assert.Equal(0x00, core.Apu.PeekStatus() & 0x10); // The sample has ended.

        // A $4015 write clears the DMC IRQ flag; a read does not (apu_test 7-dmc_basics).
        core.Apu.ReadStatus();
        Assert.True(core.Apu.IrqAsserted);

        core.WriteMemory(0x4015, 0x00);
        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_113_a_looping_dmc_sample_never_ends_and_never_raises_its_irq()
    {
        NesCore core = Core();
        StartDmc(core, rate: 15, length: 0, irq: true, loop: true);

        Tick(core, 20_000);

        Assert.False(core.Apu.IrqAsserted);
        Assert.Equal(0x10, core.Apu.PeekStatus() & 0x10);

        // Clearing bit 4 of $4015 stops it.
        core.WriteMemory(0x4015, 0x00);
        Tick(core, 10);
        Assert.Equal(0x00, core.Apu.PeekStatus() & 0x10);
    }

    [Fact]
    public void REQ_CORE_113_clearing_the_dmc_irq_enable_bit_clears_a_pending_irq()
    {
        NesCore core = Core();
        StartDmc(core, rate: 15, length: 0, irq: true, loop: false);

        while (!core.Apu.IrqAsserted)
        {
            core.Bus.Tick();
        }

        core.WriteMemory(0x4010, 0x0F); // IRQ enable cleared.
        Assert.False(core.Apu.IrqAsserted);
    }

    // ---------------------------------------------------------------- reset

    [Fact]
    public void REQ_CORE_004_reset_silences_every_channel()
    {
        NesCore core = Core();
        StartPulse1(core, duty: 2, period: 100);
        core.WriteMemory(0x4011, 100);
        StartNoise(core, 0x04);
        Tick(core, 1000);

        Assert.True(Peak(core, 1000) > 0.0);

        core.Reset();
        core.Bus.Tick();

        Assert.Equal(0.0, core.Apu.MixedLevel);
        Assert.Equal(0x00, core.Apu.PeekStatus());
        Assert.Equal(0, core.Apu.AvailableSamples);
    }
}
