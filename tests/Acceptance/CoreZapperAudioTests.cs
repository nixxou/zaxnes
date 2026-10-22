using Zax.Nes;
using Zax.Nes.Apu;

namespace Zax.Acceptance.Tests;

/// <summary>Round-1 acceptance tests for SPEC-02 §3.6 (Zapper) and §3.7 (audio).</summary>
public sealed class CoreZapperAudioTests
{
    /// <summary>The nine NES colours REQ-CORE-062 calls dark.</summary>
    private static readonly int[] DarkColours = [0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F];

    /// <summary>Advances the machine one CPU cycle at a time until the PPU reaches a scanline.</summary>
    private static void RunToScanline(NesCore core, int scanline)
    {
        for (int guard = 0; guard < 200_000 && core.Ppu.Scanline != scanline; guard++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(scanline, core.Ppu.Scanline);
    }

    [Fact] // REQ-CORE-060
    public void The_aimed_position_is_sampled_once_per_frame()
    {
        NesCore core = SpecRom.Warm();
        core.SetPortType(1, ControllerPortType.Zapper);

        int frame = 0;
        core.ZapperPositionSource = _ => new ZapperPosition(10 + frame, 20 + frame);

        for (frame = 0; frame < 5; frame++)
        {
            core.RunFrame(0);
            Assert.Equal(new ZapperPosition(10 + frame, 20 + frame), core.Input.GetZapperPosition(1));
        }

        // Changing the source mid-frame does not move the position already sampled.
        frame = 99;
        Assert.Equal(new ZapperPosition(14, 24), core.Input.GetZapperPosition(1));
    }

    /// <remarks>
    /// <c>Q-058</c>: the second half of this test used to read <c>$4017</c> after two
    /// <c>RunFrame</c> calls, that is <b>during the vblank</b>, where AMD-021 R2-006 now makes
    /// the Zapper see the universal backdrop and nothing else — over a black backdrop the byte
    /// is therefore <c>0x08</c>, not <c>0x00</c>. The criterion of REQ-CORE-061 asks for a
    /// pointer "sur un pixel blanc déjà balayé", which is a state of the visible scan: the read
    /// is moved there. The vblank reading that R2-006 replaced is kept below as the third case,
    /// with the value the amendment gives it.
    /// </remarks>
    [Fact] // REQ-CORE-061, AMD-021 R2-006 (Q-058)
    public void The_zapper_byte_follows_the_trigger_bit_and_the_light_bit()
    {
        NesCore core = SpecRom.Warm();
        core.SetPortType(1, ControllerPortType.Zapper);
        core.ZapperPositionSource = _ => new ZapperPosition(-1, -1);

        // Trigger released (bit 4 = 1), pointer off screen (no light -> bit 3 = 1) -> 0x18.
        core.RunFrame(0x1000);
        Assert.Equal(0x18, core.Bus.Read(0x4017));
        Assert.Equal(0x18, core.Bus.Read(0x4017)); // no serial register advances

        // Trigger pressed (bit 4 = 0) on an already-scanned white pixel -> 0x00.
        SpecRom.Vram(core, 0x3F00, 0x0F);
        SpecRom.Vram(core, 0x3F11, 0x30);
        SpecRom.Vram(core, 0x3F12, 0x30);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 0, y: 99, tile: 0x80, attribute: 0x00, x: 100); // white sprite
        core.ZapperPositionSource = _ => new ZapperPosition(103, 103);
        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0x0000);
        core.RunFrame(0x0000);

        // The scan of the frame under way has passed line 103: the white sprite is seen.
        RunToScanline(core, 150);
        Assert.Equal(0x00, core.Bus.Read(0x4017));

        // AMD-021 R2-006: the same read made during the vblank sees only the universal
        // backdrop, which is black here, so the light bit goes back to 1.
        RunToScanline(core, 245);
        Assert.Equal(0x08, core.Bus.Read(0x4017));

        // With a light backdrop the vblank read does see light again (§4.2, criterion a).
        SpecRom.Vram(core, 0x3F00, 0x30);
        core.RunFrame(0x0000);
        RunToScanline(core, 245);
        Assert.Equal(0x00, core.Bus.Read(0x4017));
    }

    [Fact] // REQ-CORE-062 (unit-test part of the criterion)
    public void Light_appears_only_once_the_scan_has_passed_the_pixel()
    {
        NesCore core = SpecRom.Warm();
        SpecRom.Vram(core, 0x3F00, 0x0F);  // universal background: black
        SpecRom.Vram(core, 0x3F11, 0x30);  // sprite sub-palette 0, colour 1: white
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 0, y: 99, tile: 0x80, attribute: 0x00, x: 100);
        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);
        core.RunFrame(0);

        // Walk a fresh frame scanline by scanline.
        RunToScanline(core, 0);
        for (int line = 0; line <= 103; line++)
        {
            RunToScanline(core, line);
            Assert.False(core.Ppu.SeesLightAt(103, 103), $"light already seen at line {line}");
        }

        RunToScanline(core, 104);
        Assert.True(core.Ppu.SeesLightAt(103, 103));
    }

    [Fact] // REQ-CORE-062 (a point outside the image never lights up)
    public void An_off_screen_aim_never_sees_light()
    {
        NesCore core = SpecRom.Warm();
        SpecRom.Vram(core, 0x3F00, 0x30); // white everywhere
        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);
        core.RunFrame(0);

        Assert.False(core.Ppu.SeesLightAt(-1, 100));
        Assert.False(core.Ppu.SeesLightAt(256, 100));
        Assert.False(core.Ppu.SeesLightAt(100, -1));
        Assert.False(core.Ppu.SeesLightAt(100, 240));
    }

    [Fact] // REQ-CORE-062, SPEC-02 §5.4: exactly nine dark colours
    public void Exactly_nine_colours_are_dark()
    {
        List<int> dark = [];
        for (int colour = 0; colour < 64; colour++)
        {
            NesCore core = SpecRom.Warm();
            SpecRom.Vram(core, 0x3F00, (byte)colour);
            core.WriteMemory(0x2000, 0x00);
            core.WriteMemory(0x2001, 0x0A); // background on, so the universal colour is shown
            core.RunFrame(0);
            core.RunFrame(0);

            if (!core.Ppu.SeesLightAt(100, 100))
            {
                dark.Add(colour);
            }
        }

        Assert.Equal(DarkColours, dark);
    }

    // ------------------------------------------------------------------ audio

    [Fact] // REQ-CORE-070
    public void Ntsc_at_48k_produces_800_samples_per_frame()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        core.Region = NesRegion.Ntsc;
        core.Apu.DiscardSamples();

        int produced = 0;
        float[] sink = new float[4096];
        for (int i = 0; i < 600; i++)
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(sink.AsSpan(0, 256))) > 0)
            {
                produced += got;
            }
        }

        produced += core.ReadAudio(sink.AsSpan(0, core.Apu.AvailableSamples));
        Assert.InRange(produced, 480_000 - 800, 480_000 + 800);
    }

    [Fact] // REQ-CORE-070 (the configurable output rate)
    public void The_output_rate_spans_22050_to_96000()
    {
        NesCore core = SpecRom.Warm();
        foreach (int rate in new[] { 22_050, 44_100, 48_000, 96_000 })
        {
            core.SampleRate = rate;
            Assert.Equal(rate, core.SampleRate);
        }
    }

    [Fact] // REQ-CORE-070 (fine adjustment of the resampling ratio, +/- 0.5 %)
    public void The_fine_ratio_adjustment_is_accepted_over_its_domain()
    {
        NesCore core = SpecRom.Warm();
        foreach (double adjustment in new[] { -0.005, -0.001, 0.0, 0.004, 0.005 })
        {
            core.AudioRatioAdjustment = adjustment;
            Assert.Equal(adjustment, core.AudioRatioAdjustment, 9);
        }

        // Beyond the +/- 0.5 % domain the core refuses rather than silently distorting.
        Assert.ThrowsAny<ArgumentException>(() => core.AudioRatioAdjustment = 0.05);
    }

    [Fact] // REQ-CORE-070 (the adjustment shifts the sample production, emulation untouched)
    public void The_fine_adjustment_shifts_production_without_touching_the_emulation()
    {
        static (int Samples, long Cycles) Run(double adjustment)
        {
            NesCore core = SpecRom.Warm();
            core.SampleRate = 48_000;
            core.AudioRatioAdjustment = adjustment;
            core.Apu.DiscardSamples();

            int produced = 0;
            float[] sink = new float[4096];
            long start = core.CycleCount;
            for (int i = 0; i < 600; i++)
            {
                core.RunFrame(0);
                int got;
                while ((got = core.ReadAudio(sink.AsSpan(0, 256))) > 0)
                {
                    produced += got;
                }
            }

            produced += core.ReadAudio(sink.AsSpan(0, core.Apu.AvailableSamples));
            return (produced, core.CycleCount - start);
        }

        (int plain, long plainCycles) = Run(0.0);
        (int faster, long fasterCycles) = Run(0.005);

        Assert.Equal(plainCycles, fasterCycles);
        Assert.NotEqual(plain, faster);
        Assert.InRange(Math.Abs(faster - plain) / (double)plain, 0.002, 0.008);
    }

    [Fact] // REQ-CORE-071
    public void Reading_audio_is_all_or_nothing()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        core.Region = NesRegion.Ntsc;
        core.Apu.DiscardSamples();
        core.RunFrame(0);

        int available = core.Apu.AvailableSamples;
        Assert.InRange(available, 780, 820);

        Assert.Equal(0, core.ReadAudio(new float[1024]));
        Assert.Equal(available, core.Apu.AvailableSamples);

        Assert.Equal(512, core.ReadAudio(new float[512]));
        Assert.Equal(available - 512, core.Apu.AvailableSamples);

        // 512 interleaved values consume 256 mono samples, and about 288 are left.
        int remaining = core.Apu.AvailableSamples;
        Assert.True(remaining >= 256);
        float[] stereo = new float[512];
        Assert.Equal(512, core.ReadAudio(stereo, stereo: true));
        Assert.Equal(remaining - 256, core.Apu.AvailableSamples);
    }

    [Fact] // REQ-CORE-071 (stereo is interleaved with L = R)
    public void Stereo_reads_are_interleaved_and_identical()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        core.Apu.DiscardSamples();
        core.RunFrame(0);

        float[] buffer = new float[512];
        Assert.Equal(512, core.ReadAudio(buffer, stereo: true));
        for (int i = 0; i < buffer.Length; i += 2)
        {
            Assert.Equal(buffer[i], buffer[i + 1]);
        }

        Assert.All(buffer, v => Assert.InRange(v, -1f, 1f));
    }

    [Fact] // REQ-CORE-071 (buffer of at least 10 000 samples)
    public void The_internal_buffer_holds_at_least_ten_thousand_samples()
        => Assert.True(new NesCore().Apu.BufferCapacity >= 10_000);

    [Fact] // REQ-CORE-071 (read from another thread)
    public void Reading_from_another_thread_does_not_corrupt()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;

        int read = 0;
        using ManualResetEventSlim stop = new(false);
        Thread consumer = new(() =>
        {
            float[] buffer = new float[256];
            while (!stop.IsSet)
            {
                int got = core.ReadAudio(buffer);
                read += got;
                foreach (float v in buffer.AsSpan(0, got))
                {
                    Assert.InRange(v, -1f, 1f);
                }
            }
        });

        consumer.Start();
        for (int i = 0; i < 300; i++)
        {
            core.RunFrame(0);
        }

        stop.Set();
        consumer.Join();
        Assert.True(read > 0);
    }

    [Theory] // REQ-CORE-072
    [InlineData(2.0, 240_000)]
    [InlineData(0.5, 960_000)]
    public void Speed_only_retimes_the_audio(double speed, int expected)
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        core.Region = NesRegion.Ntsc;
        core.Speed = speed;
        core.Apu.DiscardSamples();

        int produced = 0;
        float[] sink = new float[8192];
        for (int i = 0; i < 600; i++)
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(sink.AsSpan(0, 256))) > 0)
            {
                produced += got;
            }
        }

        produced += core.ReadAudio(sink.AsSpan(0, core.Apu.AvailableSamples));
        Assert.InRange(produced, expected - 800, expected + 800);
    }

    [Fact] // REQ-CORE-073
    public void The_mixer_uses_the_linear_approximation_steps()
    {
        Assert.Equal(0.00752, Apu2A03.PulseStep, 8);
        Assert.Equal(0.00851, Apu2A03.TriangleStep, 8);
        Assert.Equal(0.00494, Apu2A03.NoiseStep, 8);
        Assert.Equal(0.00335, Apu2A03.DmcStep, 8);
        Assert.Equal(1.526, Apu2A03.OutputGain, 6);
    }

    [Fact] // REQ-CORE-073 (a lone pulse at volume 15, 50 % duty, peak to peak 0.172 +/- 0.02)
    public void A_lone_pulse_has_the_expected_peak_to_peak_level()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;

        core.WriteMemory(0x4015, 0x01);  // enable pulse 1
        core.WriteMemory(0x4000, 0xBF);  // duty 50 %, constant volume 15, length halted
        core.WriteMemory(0x4001, 0x00);  // sweep off
        core.WriteMemory(0x4002, 0xFD);  // timer 253 -> about 440 Hz
        core.WriteMemory(0x4003, 0x08);

        List<float> samples = [];
        float[] buffer = new float[256];
        for (int i = 0; i < 60; i++)
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(buffer)) > 0)
            {
                samples.AddRange(buffer.AsSpan(0, got).ToArray());
            }
        }

        // Skip the start-up transient of the DC blocker, then measure the two plateaus:
        // the median of the samples above and below the mean. Taking raw min and max would
        // measure the band-limited resampler's edge ringing instead of the signal level.
        float[] tail = [.. samples.Skip(samples.Count / 2)];
        float mean = tail.Average();
        float high = Median([.. tail.Where(v => v > mean)]);
        float low = Median([.. tail.Where(v => v <= mean)]);

        Assert.InRange(high - low, 0.172f - 0.02f, 0.172f + 0.02f);
    }

    private static float Median(float[] values)
    {
        Array.Sort(values);
        return values[values.Length / 2];
    }

    [Fact] // REQ-CORE-073 (the DC blocker settles to zero in under 0.2 s)
    public void Silence_decays_to_zero()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        core.WriteMemory(0x4015, 0x00);

        List<float> samples = [];
        float[] buffer = new float[256];
        for (int i = 0; i < 20; i++) // 20 frames = 0.33 s
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(buffer)) > 0)
            {
                samples.AddRange(buffer.AsSpan(0, got).ToArray());
            }
        }

        // After 0.2 s (9 600 samples at 48 kHz) the output must be settled.
        float[] tail = [.. samples.Skip(9_600)];
        Assert.All(tail, v => Assert.InRange(v, -0.01f, 0.01f));
    }

    [Fact] // REQ-CORE-074, AMD-004 (Q-006)
    public void Restoring_a_state_drops_the_pending_samples()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        byte[] state = core.SaveState();

        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.Apu.AvailableSamples > 512);
        Assert.True(core.TryLoadState(state, 0, out _, out _, out _));
        Assert.Equal(0, core.ReadAudio(new float[512]));

        core.RunFrame(0);
        Assert.Equal(512, core.ReadAudio(new float[512]));
    }

    [Fact] // REQ-CORE-074 (the same holds after a reset)
    public void Resetting_drops_the_pending_samples()
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = 48_000;
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        core.Reset();
        Assert.Equal(0, core.ReadAudio(new float[512]));
    }
}
