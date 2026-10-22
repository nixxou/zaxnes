using Zax.App.Audio;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Nes.Apu;
using Zax.Platform;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-11: the console output, its behaviour per state, the fades
/// between game and menu, the menu-music selection and the interface sounds.
/// </summary>
public sealed class AudioTests
{
    private const int Rate = 48_000;

    /// <summary>A pure sine of <paramref name="hertz"/>, as a mono clip.</summary>
    private static WaveAudio Tone(double hertz = 440.0, double seconds = 0.25, int rate = Rate)
    {
        int frames = (int)(seconds * rate);
        float[] samples = new float[frames];
        for (int frame = 0; frame < frames; frame++)
        {
            samples[frame] = (float)Math.Sin(2.0 * Math.PI * hertz * frame / rate);
        }

        return new WaveAudio(samples, 1, rate);
    }

    /// <summary>
    /// A core running a square wave on pulse 1 at the given period, so that the output has a
    /// known fundamental (SPEC-02 §3.7).
    /// </summary>
    private static NesCore Square(int period, int sampleRate = Rate, NesRegion region = NesRegion.Ntsc)
    {
        NesCore core = new();
        core.LoadRom(LoopRom.Build(), "square.nes");
        core.Reset();
        core.SampleRate = sampleRate;
        core.Region = region;

        core.Apu.Write(0x4015, 0x01);
        core.Apu.Write(0x4000, 0b1011_1111);
        core.Apu.Write(0x4002, (byte)(period & 0xFF));
        core.Apu.Write(0x4003, (byte)(((period >> 8) & 0x07) | 0x08));
        return core;
    }

    /// <summary>The strongest frequency of a mono signal, by a plain DFT around the peak.</summary>
    private static double Fundamental(float[] samples, int rate, double from, double to)
    {
        double best = 0;
        double bestPower = -1;
        for (double hertz = from; hertz <= to; hertz += 0.5)
        {
            double real = 0;
            double imaginary = 0;
            for (int index = 0; index < samples.Length; index++)
            {
                double angle = -2.0 * Math.PI * hertz * index / rate;
                real += samples[index] * Math.Cos(angle);
                imaginary += samples[index] * Math.Sin(angle);
            }

            double power = (real * real) + (imaginary * imaginary);
            if (power > bestPower)
            {
                bestPower = power;
                best = hertz;
            }
        }

        return best;
    }

    /// <summary>
    /// Drains the core exactly: a read is all or nothing (SPEC-02 REQ-CORE-071), so the buffer
    /// handed over is sized to what the core has, never larger.
    /// </summary>
    private static int Drain(NesCore core, List<float>? into = null)
    {
        int available = core.Apu.AvailableSamples;
        if (available <= 0)
        {
            return 0;
        }

        float[] buffer = new float[available];
        int read = core.Apu.ReadSamples(buffer, stereo: false);
        into?.AddRange(buffer.AsSpan(0, read));
        return read;
    }

    private static float[] Pull(NesCore core, int frames)
    {
        List<float> collected = [];
        for (int frame = 0; frame < frames; frame++)
        {
            core.RunFrame(0);
            Drain(core, collected);
        }

        return [.. collected];
    }

    // ------------------------------------------------- §3.1 the console output

    /// <summary>
    /// REQ-AUD-001 — the output is opened at 48 000 Hz when the device accepts it, otherwise at
    /// the device's own rate, and the core produces its samples directly at the effective rate:
    /// a 440 Hz square comes out at 440 Hz ± 0.5 % at both 48 000 and 44 100 Hz.
    /// </summary>
    [Theory] // REQ-AUD-001
    [InlineData(48_000)]
    [InlineData(44_100)]
    public void The_core_resamples_to_the_effective_output_rate(int rate)
    {
        Assert.Equal(48_000, AudioOutput.PreferredSampleRate);
        Assert.Equal(48_000, Apu2A03.DefaultSampleRate);

        // Period 253 gives about 440.3 Hz on NTSC (f = 1789773 / (16 × (t + 1))).
        NesCore core = Square(253, rate);
        Assert.Equal(rate, core.SampleRate);

        float[] samples = Pull(core, 40);
        Assert.True(samples.Length > rate / 4, $"only {samples.Length} samples at {rate} Hz.");

        double measured = Fundamental(samples.AsSpan(samples.Length / 4, rate / 8).ToArray(), rate, 400, 480);
        Assert.Equal(440.3, measured, 440.3 * 0.005);
    }

    /// <summary>
    /// REQ-AUD-002 — the console signal is monophonic: on a stereo output the same sample goes to
    /// both channels, sample by sample.
    /// </summary>
    [Fact] // REQ-AUD-002
    public void The_console_signal_goes_identically_to_both_channels()
    {
        Assert.Equal(2, AudioOutput.Channels);

        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        float position = 0f;
        mixer.EmulationSource = destination =>
        {
            for (int index = 0; index < destination.Length; index++)
            {
                destination[index] = MathF.Sin(position);
                position += 0.05f;
            }

            return destination.Length;
        };

        float[] stereo = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(stereo);

        for (int frame = 0; frame < AudioMixer.BlockFrames; frame++)
        {
            Assert.Equal(stereo[frame * 2], stereo[(frame * 2) + 1]);
        }

        Assert.Contains(stereo, value => Math.Abs(value) > 1e-4f);
    }

    /// <summary>
    /// REQ-AUD-003 — the device asks for blocks of about 512 frames; a block is filled all or
    /// nothing: when the core has less than a whole block the block is digital silence and the
    /// samples stay for the next call, so no block ever mixes signal with padding.
    /// </summary>
    [Fact] // REQ-AUD-003
    public void A_block_is_filled_all_or_nothing()
    {
        Assert.Equal(512, AudioMixer.BlockFrames);
        Assert.Equal(512, AudioOutput.BlockFrames);

        AudioMixer mixer = new(Rate) { EmulationAudible = true };

        int available = 100;
        int consumed = 0;
        mixer.EmulationSource = destination =>
        {
            if (available < destination.Length)
            {
                // The source hands nothing back: the samples stay in the core's buffer.
                return 0;
            }

            destination.Fill(0.5f);
            consumed += destination.Length;
            available -= destination.Length;
            return destination.Length;
        };

        float[] stereo = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(stereo);

        Assert.All(stereo, value => Assert.Equal(0f, value));
        Assert.Equal(0, consumed);
        Assert.Equal(1, mixer.UnderrunBlocks);

        // Once a whole block is there the block is filled with signal only.
        available = 2048;
        mixer.Fill(stereo);
        Assert.All(stereo, value => Assert.True(Math.Abs(value) > 1e-4f, "a padded sample appeared."));
        Assert.Equal(AudioMixer.BlockFrames, consumed);
    }

    /// <summary>
    /// REQ-AUD-004 — each emulated frame produces on average
    /// <c>rate / (nominal × k)</c> samples, the nominal rate being 60 in NTSC and 50 in PAL and
    /// Dendy (DEC-R2-T3), so the samples per real second stay equal to the output rate at any
    /// speed.
    /// </summary>
    [Theory] // REQ-AUD-004, REQ-AUD-011
    [InlineData(NesRegion.Ntsc, 1.0, 800)]
    [InlineData(NesRegion.Pal, 1.0, 960)]
    [InlineData(NesRegion.Dendy, 1.0, 960)]
    [InlineData(NesRegion.Ntsc, 2.0, 400)]
    [InlineData(NesRegion.Ntsc, 1.5, 533)]
    public void Each_emulated_frame_produces_the_documented_number_of_samples(
        NesRegion region, double speed, int expected)
    {
        NesCore core = Square(253, Rate, region);
        core.Speed = speed;

        // Warm-up, then the measured run.
        Pull(core, 20);
        float[] samples = Pull(core, 300);

        double perFrame = samples.Length / 300.0;
        Assert.Equal(expected, perFrame, expected * 0.01);
    }

    /// <summary>
    /// REQ-AUD-005 — the core's buffer is bounded and the host may nudge the resampling ratio by
    /// at most ±0.5 %.
    /// </summary>
    [Fact] // REQ-AUD-005
    public void The_buffer_is_bounded_and_the_ratio_nudge_is_capped()
    {
        Assert.Equal(0.005, Apu2A03.MaxRatioAdjustment, 1e-9);

        NesCore core = Square(253);
        Assert.True(core.Apu.BufferCapacity >= Apu2A03.MinBufferSamples);

        // The whole documented range is accepted and nothing beyond it.
        core.AudioRatioAdjustment = Apu2A03.MaxRatioAdjustment;
        Assert.Equal(Apu2A03.MaxRatioAdjustment, core.AudioRatioAdjustment, 1e-9);
        core.AudioRatioAdjustment = -Apu2A03.MaxRatioAdjustment;
        Assert.Equal(-Apu2A03.MaxRatioAdjustment, core.AudioRatioAdjustment, 1e-9);
        Assert.Throws<ArgumentOutOfRangeException>(() => core.AudioRatioAdjustment = 1.0);
        core.AudioRatioAdjustment = 0.0;

        // The buffer never grows past its capacity: the oldest samples are dropped.
        for (int frame = 0; frame < 600; frame++)
        {
            core.RunFrame(0);
        }

        Assert.True(
            core.Apu.AvailableSamples <= core.Apu.BufferCapacity,
            $"{core.Apu.AvailableSamples} samples for a capacity of {core.Apu.BufferCapacity}.");
    }

    /// <summary>
    /// REQ-AUD-006, A1, A2 — the console gain is <c>V</c>: at 50 % the peak is halved (−6 dB),
    /// at 25 % quartered (−12 dB), and a change takes effect in under 100 ms. <c>V = 0</c>
    /// silences everything, the interface sounds included.
    /// </summary>
    [Fact] // REQ-AUD-006
    public void The_console_gain_is_the_general_volume()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        mixer.EmulationSource = destination =>
        {
            destination.Fill(1f);
            return destination.Length;
        };

        float[] stereo = new float[AudioMixer.BlockFrames * 2];

        mixer.Fill(stereo);
        float full = Peak(stereo);
        Assert.Equal(1f, full, 1e-4f);

        mixer.Volume = 0.5f;
        mixer.Fill(stereo);
        Assert.Equal(0.5f, Peak(stereo), 0.01f);
        Assert.Equal(-6.0, 20.0 * Math.Log10(Peak(stereo) / full), 0.5);

        mixer.Volume = 0.25f;
        mixer.Fill(stereo);
        Assert.Equal(0.25f, Peak(stereo), 0.01f);
        Assert.Equal(-12.0, 20.0 * Math.Log10(Peak(stereo) / full), 0.5);

        // V = 0 silences the console and the interface sounds alike.
        mixer.Volume = 0f;
        mixer.PlayClip(Tone());
        mixer.Fill(stereo);
        Assert.Equal(0f, Peak(stereo), 1e-6f);
        Assert.Equal(0f, mixer.SoundGain, 1e-9f);

        static float Peak(float[] samples)
        {
            float peak = 0f;
            foreach (float value in samples)
            {
                peak = MathF.Max(peak, MathF.Abs(value));
            }

            return peak;
        }
    }

    /// <summary>
    /// REQ-AUD-007 — under libretro the frames of one emulated frame are handed over in one lot:
    /// 800 frames per <c>retro_run</c> on average at NTSC and 48 000 Hz, 960 at PAL.
    /// </summary>
    [Theory] // REQ-AUD-007
    [InlineData(NesRegion.Ntsc, 800)]
    [InlineData(NesRegion.Pal, 960)]
    public void The_libretro_frontend_hands_over_one_lot_per_run(NesRegion region, int expected)
    {
        NesCore core = Square(253, Rate, region);
        Pull(core, 20);

        List<int> lots = [];
        for (int call = 0; call < 600; call++)
        {
            core.RunFrame(0);
            lots.Add(Drain(core));
        }

        Assert.Equal(expected, lots.Average(), 1.0);
    }

    // ----------------------------------------------- §3.2 behaviour per state

    /// <summary>
    /// REQ-AUD-010, A6 — in Paused and in Stopped no emulated frame produces samples, so the
    /// output becomes digital silence; at a game load, at a savestate load and at a reset the
    /// core's buffer is emptied so nothing of the previous session is heard.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-AUD-010
    public void The_output_is_digital_silence_in_pause_and_the_buffer_is_dropped_at_a_load()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Session!.Core.Apu.Write(0x4015, 0x01);
        host.Session!.Core.Apu.Write(0x4000, 0b1011_1111);
        host.Session!.Core.Apu.Write(0x4002, 253);
        host.Session!.Core.Apu.Write(0x4003, 0x08);
        host.RunFrames(30);

        Assert.True(host.Session!.Core.Apu.AvailableSamples > 0);

        // In pause the mixer's console contribution is silence once the buffer is drained.
        host.Run(ZaxCommands.Pause);
        float[] stereo = new float[AudioMixer.BlockFrames * 2];
        host.Mixer.EmulationSource = destination =>
            host.Session!.Core.Apu.ReadSamples(destination, stereo: false);

        for (int block = 0; block < 64; block++)
        {
            host.Mixer.Fill(stereo);
        }

        Assert.All(stereo, value => Assert.Equal(0f, value));

        // A reset empties what was left.
        host.Run(ZaxCommands.Resume);
        host.RunFrames(10);
        host.Run(ZaxCommands.Reset);
        Assert.Equal(0, host.Session!.Core.Apu.AvailableSamples);
    }

    /// <summary>
    /// REQ-AUD-011, A5 — the audio follows the speed factor by time compression with a
    /// proportional pitch change: at <c>k = 2</c> a 440 Hz square comes out at 880 Hz, at
    /// <c>k = 1.5</c> at 660 Hz and at <c>k = 0.8</c> at 352 Hz, the sample rate staying the same.
    /// </summary>
    [Theory] // REQ-AUD-011
    [InlineData(2.0, 880.0)]
    [InlineData(1.5, 660.0)]
    [InlineData(0.8, 352.0)]
    public void The_speed_factor_changes_the_pitch_proportionally(double speed, double expected)
    {
        NesCore core = Square(253);
        core.Speed = speed;

        Pull(core, 30);
        float[] samples = Pull(core, 120);

        double measured = Fundamental(
            samples.AsSpan(samples.Length / 4, Math.Min(Rate / 8, samples.Length / 2)).ToArray(),
            Rate,
            expected * 0.9,
            expected * 1.1);

        Assert.Equal(expected, measured, expected * 0.01);
    }

    /// <summary>
    /// REQ-AUD-012 (and Q-006, AMD-004) — restoring a snapshot does not cut the output: only the
    /// queue of pending samples is dropped, the output chain of the core is not reinitialised.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-AUD-012
    public void A_rollback_does_not_cut_the_output()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Session!.Core.Apu.Write(0x4015, 0x01);
        host.Session!.Core.Apu.Write(0x4000, 0b1011_1111);
        host.Session!.Core.Apu.Write(0x4002, 253);
        host.Session!.Core.Apu.Write(0x4003, 0x08);

        host.RunFrames(130);
        host.Run(ZaxCommands.Rollback);

        // The sound comes back inside one second of play.
        host.RunFrames(60);
        Assert.True(
            host.Session!.Core.Apu.AvailableSamples > 0,
            "the output must resume from the restored APU state.");
        Assert.True(host.Mixer.EmulationAudible);
    }

    /// <summary>
    /// REQ-AUD-013 — losing the window focus neither cuts nor dims the sound: the host has no
    /// focus notion, so the gain is untouched and the emulation keeps producing samples.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-AUD-013
    public void Losing_the_focus_leaves_the_sound_alone()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Mixer.Advance(FadeGain.FullFadeSeconds);

        float gain = host.Mixer.EmulationGain;
        host.RunFrames(60);

        Assert.Equal(gain, host.Mixer.EmulationGain, 1e-6f);
        Assert.True(host.Mixer.EmulationAudible);
    }

    // ----------------------------------------- §3.3 the fades between game and menu

    /// <summary>
    /// REQ-AUD-020 — a complete fade of the console gain lasts 200/60 s ≈ 3.33 s of <b>real</b>
    /// time whatever the render rate, and the very first launch after start-up begins directly at
    /// gain <c>V</c> because no earlier fade lowered it.
    /// </summary>
    [Fact] // REQ-AUD-020
    public void The_console_fade_lasts_three_and_a_third_seconds_at_any_rate()
    {
        Assert.Equal(200.0 / 60.0, FadeGain.FullFadeSeconds, 1e-9);

        AudioMixer mixer = new(Rate);

        // The first launch starts at V: no fade has lowered the gain.
        Assert.Equal(1f, mixer.EmulationGain, 1e-6f);
        mixer.EmulationAudible = true;
        Assert.Equal(1f, mixer.EmulationGain, 1e-6f);

        foreach (int rate in new[] { 60, 120 })
        {
            mixer.EmulationAudible = false;
            Advance(mixer, FadeGain.FullFadeSeconds, rate);
            Assert.Equal(0f, mixer.EmulationGain, 1e-5f);

            mixer.EmulationAudible = true;
            Advance(mixer, FadeGain.FullFadeSeconds / 2.0, rate);
            Assert.Equal(0.5f, mixer.EmulationGain, 0.03f);
            Advance(mixer, FadeGain.FullFadeSeconds / 2.0, rate);
            Assert.Equal(1f, mixer.EmulationGain, 1e-5f);
        }

        static void Advance(AudioMixer mixer, double seconds, int rate)
        {
            int steps = (int)Math.Round(seconds * rate);
            for (int step = 0; step < steps; step++)
            {
                mixer.Advance(1.0 / rate);
            }
        }
    }

    /// <summary>
    /// REQ-AUD-020 — a fade under way is replaced by the new one, from the gain it had reached
    /// and with the same slope.
    /// </summary>
    [Fact] // REQ-AUD-020
    public void A_fade_under_way_is_replaced_from_the_current_gain()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };

        mixer.EmulationAudible = false;
        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        float halfway = mixer.EmulationGain;
        Assert.Equal(0.5f, halfway, 0.03f);

        mixer.EmulationAudible = true;
        mixer.Advance(FadeGain.FullFadeSeconds / 4.0);
        Assert.Equal(halfway + 0.25f, mixer.EmulationGain, 0.03f);
    }

    /// <summary>
    /// REQ-AUD-021, A3, A4 — the gains: <c>0.02 × V</c> for the menu music when <c>M</c> is true
    /// and 0 otherwise, and <c>0.07 × V</c> for the interface sounds, applied to the file samples
    /// without normalisation.
    /// </summary>
    [Fact] // REQ-AUD-021
    public void The_music_and_interface_gains_are_the_documented_fractions_of_V()
    {
        Assert.Equal(0.02f, AudioMixer.MusicGainFactor, 1e-9f);
        Assert.Equal(0.07f, AudioMixer.SoundGainFactor, 1e-9f);

        AudioMixer mixer = new(Rate);
        mixer.Advance(FadeGain.FullFadeSeconds);
        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);
        Assert.Equal(AudioMixer.SoundGainFactor, mixer.SoundGain, 1e-6f);

        mixer.Volume = 0.5f;
        Assert.Equal(AudioMixer.MusicGainFactor * 0.5f, mixer.MusicGain, 1e-6f);
        Assert.Equal(AudioMixer.SoundGainFactor * 0.5f, mixer.SoundGain, 1e-6f);

        mixer.MenuMusicEnabled = false;
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);

        // The gain really reaches the samples: a 0.07 × V peak for a unit interface clip.
        AudioMixer loud = new(Rate);
        loud.PlayClip(Tone());
        float[] stereo = new float[AudioMixer.BlockFrames * 2];
        loud.Fill(stereo);

        float peak = 0f;
        foreach (float value in stereo)
        {
            peak = MathF.Max(peak, MathF.Abs(value));
        }

        Assert.True(
            peak <= AudioMixer.SoundGainFactor + 1e-3f,
            $"the interface peak {peak} must not exceed 0.07.");
    }

    /// <summary>
    /// REQ-AUD-022 — as long as the state is not Running the menu music plays and rises to its
    /// target; as soon as it becomes Running it falls to 0 and is suspended. Coming back out of
    /// Running, it must come back progressively.
    /// </summary>
    /// <remarks>
    /// Round 1: the gain rose again after a pause but the track stayed suspended, so nothing was
    /// mixed any more and the music never actually came back
    /// (<c>verify/round1/R1-report-part-b.md</c> §2). Corrected by the <c>R1-fixes</c> batch —
    /// <c>AudioMixer</c> lifts the suspension when the emulation goes quiet. Re-verified
    /// 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-AUD-022
    public void The_menu_music_must_come_back_after_a_game()
    {
        // AMD-024 REQ-PUB-005 (revised 2026-10-21) drops the menu music from the published
        // build, so this rule now only applies to the tracks a user supplies with --assets.
        AudioAssets? assets = AudioAssets.TryOpen(NeutralAssets.Root);
        Assert.NotNull(assets);

        AudioMixer mixer = new(Rate, assets, new Random(7));
        float[] stereo = new float[AudioMixer.BlockFrames * 2];

        // Menu: the music rises and is audible.
        for (int block = 0; block < 400; block++)
        {
            mixer.Fill(stereo);
        }

        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);
        Assert.True(Audible(mixer, stereo), "the menu music must be audible before the game.");

        // A game runs: the music fades out and is suspended.
        mixer.EmulationAudible = true;
        for (int block = 0; block < 400; block++)
        {
            mixer.Fill(stereo);
        }

        Assert.Equal(0f, mixer.MusicGain, 1e-9f);
        Assert.True(mixer.MusicSuspended);

        // Back to the menu: the music must come back.
        mixer.EmulationAudible = false;
        for (int block = 0; block < 400; block++)
        {
            mixer.Fill(stereo);
        }

        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);
        Assert.True(Audible(mixer, stereo), "the menu music must come back after the game.");

        static bool Audible(AudioMixer mixer, float[] stereo)
        {
            for (int block = 0; block < 8; block++)
            {
                mixer.Fill(stereo);
                foreach (float value in stereo)
                {
                    if (Math.Abs(value) > 1e-5f)
                    {
                        return true;
                    }
                }
            }

            return false;
        }
    }

    /// <summary>
    /// REQ-AUD-022 — the music fade has the same 3.33 s duration as the console one, and the
    /// music starts from 0 at application start-up.
    /// </summary>
    [Fact] // REQ-AUD-022
    public void The_music_fade_lasts_the_same_three_and_a_third_seconds()
    {
        AudioMixer mixer = new(Rate);
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);

        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(AudioMixer.MusicGainFactor / 2f, mixer.MusicGain, AudioMixer.MusicGainFactor * 0.05f);

        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);

        mixer.EmulationAudible = true;
        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(AudioMixer.MusicGainFactor / 2f, mixer.MusicGain, AudioMixer.MusicGainFactor * 0.05f);
        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);
    }

    /// <summary>
    /// REQ-AUD-023 — a change of <c>V</c> updates the interface and music gains at once, without
    /// a fade, and toggling <c>M</c> takes the music gain to its new target immediately.
    /// </summary>
    [Fact] // REQ-AUD-023
    public void A_volume_or_music_change_is_immediate()
    {
        AudioMixer mixer = new(Rate);
        mixer.Advance(FadeGain.FullFadeSeconds);

        mixer.Volume = 0.5f;
        Assert.Equal(AudioMixer.MusicGainFactor * 0.5f, mixer.MusicGain, 1e-6f);
        Assert.Equal(AudioMixer.SoundGainFactor * 0.5f, mixer.SoundGain, 1e-6f);

        mixer.MenuMusicEnabled = false;
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);
        mixer.MenuMusicEnabled = true;
        Assert.Equal(AudioMixer.MusicGainFactor * 0.5f, mixer.MusicGain, 1e-6f);

        // A change of V during a fade does not cut the fade short.
        AudioMixer fading = new(Rate) { EmulationAudible = true };
        fading.EmulationAudible = false;
        fading.Advance(FadeGain.FullFadeSeconds / 2.0);
        float halfway = fading.EmulationGain;
        fading.Volume = 0.5f;
        Assert.Equal(halfway, fading.EmulationGain, 1e-6f);
    }

    // ------------------------------------------------- §3.4 the menu music choice

    /// <summary>
    /// REQ-AUD-030, A7 — the three tracks are played without looping; one is drawn at random at
    /// start-up and, when the current one ends while the music is active, the next is drawn among
    /// the <b>other two</b>: the same track never plays twice in a row.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-AUD-030
    public void A_track_never_follows_itself()
    {
        AudioAssets? assets = AudioAssets.TryOpen(NeutralAssets.Root);
        Assert.NotNull(assets);
        Assert.Equal(3, assets.TrackCount);
        Assert.Equal(3, AudioAssets.TrackFileNames.Length);

        // The three files, their channel count and their rough duration (§2).
        foreach ((string file, double seconds) in new[]
        {
            ("happynes.wav", 112.1),
            ("Komiku_-_02_-_Home.wav", 66.1),
            ("dire-space-emergency-chiptune.wav", 164.1),
        })
        {
            WaveAudio track = WaveFile.Load(Path.Combine(NeutralAssets.Root, "audio", file));
            Assert.Equal(2, track.Channels);
            Assert.Equal(44_100, track.SampleRate);
            Assert.Equal(seconds, track.Duration, 0.5);
        }

        // The draw never repeats: whatever the seed, two consecutive choices differ.
        for (int seed = 0; seed < 32; seed++)
        {
            AudioMixer mixer = new(Rate, assets, new Random(seed));
            float[] stereo = new float[AudioMixer.BlockFrames * 2];
            mixer.Fill(stereo);

            int first = mixer.CurrentTrack;
            Assert.InRange(first, 0, 2);

            // Force the end of the track by consuming the whole of it.
            int blocks = 0;
            while (mixer.CurrentTrack == first && blocks++ < 40_000)
            {
                mixer.Fill(stereo);
            }

            Assert.True(blocks < 40_000, "the track never ended.");
            Assert.NotEqual(first, mixer.CurrentTrack);
            Assert.True(mixer.TrackChanges >= 2);
        }
    }

    // ------------------------------------------------- §3.5 the interface sounds

    /// <summary>
    /// REQ-AUD-040 — the interface sounds share one voice: a new one cuts the one playing, and
    /// they are heard whatever the emulation does.
    /// </summary>
    [Fact] // REQ-AUD-040
    public void The_interface_sounds_share_one_voice()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        WaveAudio clip = Tone(seconds: 1.0);

        mixer.PlayClip(clip);
        Assert.True(mixer.SoundPlaying);

        float[] stereo = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(stereo);
        Assert.Contains(stereo, value => Math.Abs(value) > 1e-4f);

        // A second trigger restarts from the beginning: the voice was cut.
        mixer.PlayClip(clip);
        Assert.True(mixer.SoundPlaying);

        mixer.StopSound();
        Assert.False(mixer.SoundPlaying);

        // A clip shorter than a block ends on its own.
        mixer.PlayClip(Tone(seconds: 0.001));
        mixer.Fill(stereo);
        Assert.False(mixer.SoundPlaying);
    }

    /// <summary>
    /// REQ-AUD-041 — the event to sound mapping: the four clips exist and each one is reachable
    /// through its <see cref="UiSound"/> value.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-AUD-041
    public void Each_interface_event_has_its_clip()
    {
        AudioAssets? assets = AudioAssets.TryOpen(NeutralAssets.Root);
        Assert.NotNull(assets);

        foreach ((UiSound sound, string file, double seconds) in new[]
        {
            (UiSound.TabChange, "sfx_menu_move3.wav", 0.076),
            (UiSound.PageChange, "sfx_menu_move2.wav", 0.060),
            (UiSound.Move, "sfx_menu_move1.wav", 0.039),
            (UiSound.Confirm, "sfx_coin_double1.wav", 0.209),
        })
        {
            WaveAudio? clip = assets.Sound(sound);
            Assert.NotNull(clip);
            Assert.Equal(1, clip.Channels);
            Assert.Equal(44_100, clip.SampleRate);
            Assert.Equal(seconds, clip.Duration, 0.01);

            WaveAudio onDisk = WaveFile.Load(Path.Combine(NeutralAssets.Root, "audio", file));
            Assert.Equal(onDisk.FrameCount, clip.FrameCount);
        }
    }

    /// <summary>
    /// REQ-AUD-041 — a validation plays the confirmation clip: a slot save and a slot load both
    /// trigger it through the host.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-AUD-041
    public void A_slot_save_and_a_slot_load_play_the_confirmation_sound()
    {
        using HostDriver host = HostDriver.Create(audio: true);
        host.Launch2DMode();

        host.Mixer.StopSound();
        host.Run(ZaxCommands.QuickSave);
        Assert.True(host.Mixer.SoundPlaying, "a quick save must play the confirmation sound.");

        host.Mixer.StopSound();
        host.Run(ZaxCommands.QuickLoad);
        Assert.True(host.Mixer.SoundPlaying, "a quick load must play the confirmation sound.");
    }

    /// <summary>
    /// §4 — the audio constants of the specification.
    /// </summary>
    [Fact] // REQ-AUD-001, REQ-AUD-003, REQ-AUD-021
    public void The_audio_constants_of_section_4()
    {
        Assert.Equal(48_000, AudioOutput.PreferredSampleRate);
        Assert.Equal(512, AudioOutput.BlockFrames);
        Assert.Equal(2, AudioOutput.Channels);
        Assert.Equal(0.02f, AudioMixer.MusicGainFactor, 1e-9f);
        Assert.Equal(0.07f, AudioMixer.SoundGainFactor, 1e-9f);
        Assert.Equal(200.0 / 60.0, FadeGain.FullFadeSeconds, 1e-9);
    }
}
