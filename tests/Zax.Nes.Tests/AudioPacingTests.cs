namespace Zax.Nes.Tests;

/// <summary>
/// Audio pacing and the all-or-nothing read contract (REQ-CORE-070 to REQ-CORE-072).
/// </summary>
/// <remarks>
/// Only the pacing and the read semantics are checked here; the mix of REQ-CORE-073 and the
/// spectral criterion of REQ-CORE-070 are in <see cref="ApuOutputTests"/>. The ROM these
/// tests run never touches the APU, so what is being counted is silence produced at exactly
/// the right rate.
/// </remarks>
public sealed class AudioPacingTests
{
    private static NesCore LoadedCore()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Audio.nes").Success);
        return core;
    }

    [Theory]
    // REQ-CORE-070: a frame produces sampleRate / nominalFps samples, so 800 per frame at
    // 48 kHz in NTSC and 960 in PAL.
    [InlineData(NesRegion.Ntsc, 48_000, 480_000, 800)]
    [InlineData(NesRegion.Pal, 48_000, 576_000, 960)]
    [InlineData(NesRegion.Dendy, 48_000, 576_000, 960)]
    [InlineData(NesRegion.Multi, 48_000, 480_000, 800)]
    [InlineData(NesRegion.Ntsc, 44_100, 441_000, 800)]
    [InlineData(NesRegion.Ntsc, 22_050, 220_500, 800)]
    [InlineData(NesRegion.Ntsc, 96_000, 960_000, 800)]
    public void REQ_CORE_070_six_hundred_frames_produce_the_expected_sample_count(
        NesRegion region, int sampleRate, int expected, int tolerance)
    {
        NesCore core = LoadedCore();
        core.Region = region;
        core.SampleRate = sampleRate;

        int produced = 0;
        var sink = new float[4096];

        for (int frame = 0; frame < 600; frame++)
        {
            core.RunFrame(0);
            while (core.Apu.AvailableSamples >= sink.Length)
            {
                produced += core.ReadAudio(sink);
            }
        }

        produced += core.Apu.AvailableSamples;

        Assert.InRange(produced, expected - tolerance, expected + tolerance);
    }

    [Theory]
    [InlineData(2.0, 240_000)]
    [InlineData(0.5, 960_000)]
    public void REQ_CORE_072_the_speed_factor_only_retimes_the_audio(double speed, int expected)
    {
        NesCore core = LoadedCore();
        core.SampleRate = 48_000;
        core.Speed = speed;

        int produced = 0;
        var sink = new float[4096];

        for (int frame = 0; frame < 600; frame++)
        {
            core.RunFrame(0);
            while (core.Apu.AvailableSamples >= sink.Length)
            {
                produced += core.ReadAudio(sink);
            }
        }

        produced += core.Apu.AvailableSamples;

        Assert.InRange(produced, expected - 800, expected + 800);
    }

    [Fact]
    public void REQ_CORE_072_the_speed_factor_does_not_change_the_emulation()
    {
        NesCore fast = LoadedCore();
        NesCore normal = LoadedCore();
        fast.Speed = 1.5; // Fast forward (DEC-07-Q2).

        for (int frame = 0; frame < 120; frame++)
        {
            Assert.Equal(normal.RunFrame((ushort)frame), fast.RunFrame((ushort)frame));
        }

        for (int address = 0; address < 0x800; address++)
        {
            Assert.Equal(normal.ReadMemory((ushort)address), fast.ReadMemory((ushort)address));
        }
    }

    [Fact]
    public void REQ_CORE_071_a_read_is_all_or_nothing()
    {
        NesCore core = LoadedCore();
        core.SampleRate = 48_000;
        core.RunFrame(0); // About 800 samples.

        int available = core.Apu.AvailableSamples;
        Assert.InRange(available, 780, 820);

        // Asking for more than there is consumes nothing.
        Assert.Equal(0, core.ReadAudio(new float[1024]));
        Assert.Equal(available, core.Apu.AvailableSamples);

        // Asking for less fills the whole buffer.
        Assert.Equal(512, core.ReadAudio(new float[512]));
        Assert.Equal(available - 512, core.Apu.AvailableSamples);
    }

    [Fact]
    public void REQ_CORE_071_a_stereo_read_consumes_half_as_many_mono_samples()
    {
        NesCore core = LoadedCore();
        core.SampleRate = 48_000;
        core.RunFrame(0);

        int available = core.Apu.AvailableSamples;
        var buffer = new float[512];

        Assert.Equal(512, core.ReadAudio(buffer, stereo: true));
        Assert.Equal(available - 256, core.Apu.AvailableSamples);

        // Left equals right.
        for (int i = 0; i < buffer.Length; i += 2)
        {
            Assert.Equal(buffer[i], buffer[i + 1]);
        }
    }

    [Fact]
    public void REQ_CORE_071_samples_stay_inside_minus_one_and_one()
    {
        NesCore core = LoadedCore();
        for (int frame = 0; frame < 5; frame++)
        {
            core.RunFrame(0);
        }

        var buffer = new float[2048];
        Assert.Equal(buffer.Length, core.ReadAudio(buffer));

        foreach (float sample in buffer)
        {
            Assert.InRange(sample, -1.0f, 1.0f);
        }
    }

    [Fact]
    public void REQ_CORE_071_a_stereo_buffer_must_hold_an_even_number_of_values()
    {
        NesCore core = LoadedCore();
        Assert.Throws<ArgumentException>(() => core.ReadAudio(new float[511], stereo: true));
    }

    [Theory]
    [InlineData(22_050)]
    [InlineData(48_000)]
    [InlineData(96_000)]
    public void REQ_CORE_070_the_sample_rate_range_is_accepted(int sampleRate)
    {
        NesCore core = LoadedCore();
        core.SampleRate = sampleRate;

        Assert.Equal(sampleRate, core.SampleRate);
    }

    [Theory]
    [InlineData(22_049)]
    [InlineData(96_001)]
    public void REQ_CORE_070_a_sample_rate_outside_the_range_is_rejected(int sampleRate)
    {
        NesCore core = LoadedCore();
        Assert.Throws<ArgumentOutOfRangeException>(() => core.SampleRate = sampleRate);
    }

    [Theory]
    [InlineData(0.005, 482_400)]
    [InlineData(-0.005, 477_600)]
    public void REQ_CORE_070_the_fine_ratio_adjustment_shifts_the_sample_count_without_touching_the_emulation(
        double adjustment, int expected)
    {
        NesCore core = LoadedCore();
        NesCore reference = LoadedCore();
        core.SampleRate = 48_000;
        core.AudioRatioAdjustment = adjustment;

        int produced = 0;
        var sink = new float[4096];

        for (int frame = 0; frame < 600; frame++)
        {
            Assert.Equal(reference.RunFrame((ushort)frame), core.RunFrame((ushort)frame));
            while (core.Apu.AvailableSamples >= sink.Length)
            {
                produced += core.ReadAudio(sink);
            }
        }

        produced += core.Apu.AvailableSamples;

        Assert.InRange(produced, expected - 800, expected + 800);

        for (int address = 0; address < 0x800; address++)
        {
            Assert.Equal(reference.ReadMemory((ushort)address), core.ReadMemory((ushort)address));
        }
    }

    [Fact]
    public void REQ_CORE_070_the_fine_ratio_adjustment_is_capped_at_half_a_percent()
    {
        NesCore core = LoadedCore();

        core.AudioRatioAdjustment = 0.005;
        Assert.Equal(0.005, core.AudioRatioAdjustment);

        core.AudioRatioAdjustment = -0.005;
        Assert.Equal(-0.005, core.AudioRatioAdjustment);

        Assert.Throws<ArgumentOutOfRangeException>(() => core.AudioRatioAdjustment = 0.01);
    }

    [Fact]
    public void REQ_CORE_072_a_speed_of_zero_or_less_is_rejected()
    {
        NesCore core = LoadedCore();

        Assert.Throws<ArgumentOutOfRangeException>(() => core.Speed = 0.0);
        Assert.Throws<ArgumentOutOfRangeException>(() => core.Speed = -1.0);
    }

    [Fact]
    public void REQ_CORE_071_reading_from_another_thread_does_not_corrupt_the_buffer()
    {
        NesCore core = LoadedCore();
        core.SampleRate = 48_000;

        int read = 0;
        using var stop = new ManualResetEventSlim(false);

        var consumer = new Thread(() =>
        {
            var buffer = new float[256];
            while (!stop.IsSet)
            {
                int count = core.ReadAudio(buffer);
                if (count == 0)
                {
                    Thread.Yield();
                    continue;
                }

                Interlocked.Add(ref read, count);
                foreach (float sample in buffer)
                {
                    Assert.InRange(sample, -1.0f, 1.0f);
                }
            }
        });

        consumer.Start();

        for (int frame = 0; frame < 300; frame++)
        {
            core.RunFrame(0);
        }

        stop.Set();
        Assert.True(consumer.Join(TimeSpan.FromSeconds(10)));

        Assert.True(Volatile.Read(ref read) > 0);
    }

    [Theory]
    // REQ-CORE-071 asks for at least 10 000 samples; SPEC-11 REQ-AUD-005 asks for a bounded
    // buffer, 100 to 200 ms being the recommended range.
    [InlineData(22_050)]
    [InlineData(48_000)]
    [InlineData(96_000)]
    public void REQ_CORE_071_the_buffer_holds_at_least_ten_thousand_samples(int sampleRate)
    {
        NesCore core = LoadedCore();
        core.SampleRate = sampleRate;

        Assert.True(core.Apu.BufferCapacity >= Apu.Apu2A03.MinBufferSamples);
        Assert.True(core.Apu.BufferCapacity <= Math.Max(Apu.Apu2A03.MinBufferSamples, sampleRate / 5));
    }

    [Fact]
    public void REQ_CORE_071_an_overflowing_buffer_drops_the_oldest_samples()
    {
        NesCore core = LoadedCore();
        core.SampleRate = 48_000;

        // Far more than the buffer can hold, with nothing read in between.
        for (int frame = 0; frame < 60; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(core.Apu.BufferCapacity, core.Apu.AvailableSamples);
    }
}
