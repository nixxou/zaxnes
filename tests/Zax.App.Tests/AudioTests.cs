using Zax.App.Audio;
using Zax.App.Loop;
using Zax.Platform;

namespace Zax.App.Tests;

/// <summary>
/// The audio of SPEC-11: the output model, the gains, the fades, the menu music and the
/// interface sounds.
/// </summary>
public sealed class AudioTests
{
    private const int Rate = 48_000;

    /// <summary>
    /// REQ-AUD-002: the console signal is monophonic and goes to both channels unchanged, so
    /// the left and right channels are identical sample by sample.
    /// </summary>
    [Fact]
    public void REQ_AUD_002_the_console_signal_is_duplicated_on_both_channels()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        mixer.EmulationSource = destination =>
        {
            for (int i = 0; i < destination.Length; i++)
            {
                destination[i] = i % 2 == 0 ? 0.5f : -0.5f;
            }

            return destination.Length;
        };

        float[] block = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(block);

        for (int frame = 0; frame < AudioMixer.BlockFrames; frame++)
        {
            Assert.Equal(block[frame * 2], block[(frame * 2) + 1], 1e-6f);
        }

        Assert.NotEqual(0f, block[0]);
    }

    /// <summary>
    /// REQ-AUD-003: a block is taken all or nothing. When the core has less than a whole block
    /// the console contribution is digital silence and the block never mixes signal with
    /// padding.
    /// </summary>
    [Fact]
    public void REQ_AUD_003_an_incomplete_block_becomes_silence()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        int available = AudioMixer.BlockFrames / 2;
        mixer.EmulationSource = destination =>
        {
            if (destination.Length > available)
            {
                // All or nothing, exactly as NesCore.ReadAudio behaves (REQ-CORE-071).
                return 0;
            }

            destination.Fill(0.75f);
            return destination.Length;
        };

        float[] block = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(block);

        Assert.All(block, value => Assert.Equal(0f, value));
        Assert.Equal(1, mixer.UnderrunBlocks);
    }

    /// <summary>
    /// REQ-AUD-006 and vector A2: the console gain is <c>V</c>, so a quarter volume gives a
    /// quarter of the peak.
    /// </summary>
    [Fact]
    public void REQ_AUD_006_the_console_gain_is_the_volume()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true, Volume = 0.25f };
        mixer.EmulationSource = destination =>
        {
            destination.Fill(1f);
            return destination.Length;
        };

        float[] block = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(block);

        Assert.Equal(0.25f, block[0], 1e-5f);
        Assert.Equal(0.25f, mixer.EmulationGain, 1e-5f);
    }

    /// <summary>
    /// REQ-AUD-020 acceptance: the first launch after start-up begins at gain <c>V</c> — no
    /// previous fade has lowered it — and a complete fade then lasts 200/60 s whatever the
    /// render rate.
    /// </summary>
    [Fact]
    public void REQ_AUD_020_the_console_fade_lasts_three_and_a_third_seconds()
    {
        AudioMixer mixer = new(Rate);
        Assert.Equal(1f, mixer.EmulationGain, 1e-6f);

        mixer.EmulationAudible = true;
        Assert.Equal(1f, mixer.EmulationGain, 1e-6f);

        // Pausing starts the fade-out; it reaches 0 in 200/60 s of real time.
        mixer.EmulationAudible = false;
        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(0.5f, mixer.EmulationGain, 0.02f);

        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.Equal(0f, mixer.EmulationGain, 1e-6f);

        // And the fade-in takes the same time, in as many small steps as one likes.
        mixer.EmulationAudible = true;
        for (int step = 0; step < 200; step++)
        {
            mixer.Advance(FadeGain.FullFadeSeconds / 200.0);
        }

        Assert.Equal(1f, mixer.EmulationGain, 1e-5f);
    }

    /// <summary>
    /// REQ-AUD-021 and REQ-AUD-022: the menu music rises to <c>0.02 × V</c> while the state is
    /// not Running and fades to 0 and suspends as soon as it is.
    /// </summary>
    [Fact]
    public void REQ_AUD_022_the_menu_music_fades_with_the_state()
    {
        AudioMixer mixer = new(Rate);
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);

        mixer.Advance(FadeGain.FullFadeSeconds);
        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);

        mixer.EmulationAudible = true;
        mixer.Advance(FadeGain.FullFadeSeconds / 2.0);
        Assert.InRange(mixer.MusicGain, 0.004f, 0.012f);

        mixer.Advance(FadeGain.FullFadeSeconds);
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);
        Assert.True(mixer.MusicSuspended);
    }

    /// <summary>
    /// REQ-AUD-023: switching <c>BG Music</c> off takes the music gain to 0 at once, with no
    /// fade, and switching it back on restores the target immediately.
    /// </summary>
    [Fact]
    public void REQ_AUD_023_toggling_the_menu_music_is_immediate()
    {
        AudioMixer mixer = new(Rate);
        mixer.Advance(FadeGain.FullFadeSeconds);
        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);

        mixer.MenuMusicEnabled = false;
        Assert.Equal(0f, mixer.MusicGain, 1e-9f);

        mixer.MenuMusicEnabled = true;
        Assert.Equal(AudioMixer.MusicGainFactor, mixer.MusicGain, 1e-6f);
    }

    /// <summary>
    /// REQ-AUD-021: the interface sounds are played at <c>0.07 × V</c> and the volume reaches
    /// them with no fade (REQ-AUD-023).
    /// </summary>
    [Fact]
    public void REQ_AUD_021_the_interface_sound_gain_follows_the_volume()
    {
        AudioMixer mixer = new(Rate);
        Assert.Equal(AudioMixer.SoundGainFactor, mixer.SoundGain, 1e-6f);

        mixer.Volume = 0.5f;
        Assert.Equal(AudioMixer.SoundGainFactor * 0.5f, mixer.SoundGain, 1e-6f);
    }

    /// <summary>
    /// REQ-AUD-040: the interface sounds share one voice — a new one cuts the one playing — and
    /// they are heard whatever the emulation does.
    /// </summary>
    [Fact]
    public void REQ_AUD_040_interface_sounds_share_a_single_voice()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = true };
        WaveAudio clip = Tone(seconds: 1.0);

        mixer.PlayClip(clip);
        Assert.True(mixer.SoundPlaying);

        float[] block = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(block);

        // Audible even while the game runs, at the interface gain.
        Assert.Contains(block, value => Math.Abs(value) > 1e-4f);

        mixer.PlayClip(clip);
        Assert.True(mixer.SoundPlaying);

        mixer.StopSound();
        Assert.False(mixer.SoundPlaying);
    }

    /// <summary>
    /// REQ-AUD-010: in Paused and Stopped no emulated frame produces samples, so the console
    /// output is exact silence and an empty buffer is not counted as an under-run.
    /// </summary>
    [Fact]
    public void REQ_AUD_010_pausing_gives_digital_silence()
    {
        AudioMixer mixer = new(Rate) { EmulationAudible = false };
        mixer.EmulationSource = _ => 0;

        float[] block = new float[AudioMixer.BlockFrames * 2];
        mixer.Fill(block);

        Assert.All(block, value => Assert.Equal(0f, value));
        Assert.Equal(0, mixer.UnderrunBlocks);
    }

    /// <summary>
    /// REQ-AUD-030: a menu track does not loop, and when one ends while the music is active a
    /// different one is drawn — never the same twice in a row.
    /// </summary>
    [Fact]
    public void REQ_AUD_030_a_new_track_is_drawn_among_the_others()
    {
        string directory = Path.Combine(Path.GetTempPath(), $"zax-audio-{Guid.NewGuid():N}", "audio");
        Directory.CreateDirectory(directory);
        try
        {
            // Three very short tracks, so the playlist moves on inside a handful of blocks.
            foreach (string name in AudioAssets.TrackFileNames)
            {
                File.WriteAllBytes(Path.Combine(directory, name), WriteWave(Tone(seconds: 0.02)));
            }

            AudioAssets assets = AudioAssets.TryOpen(Path.GetDirectoryName(directory))!;
            Assert.Equal(3, assets.TrackCount);

            AudioMixer mixer = new(Rate, assets, new Random(7));
            float[] block = new float[AudioMixer.BlockFrames * 2];

            List<int> order = [];
            for (int pass = 0; pass < 40; pass++)
            {
                mixer.Fill(block);
                if (order.Count == 0 || order[^1] != mixer.CurrentTrack)
                {
                    order.Add(mixer.CurrentTrack);
                }
            }

            Assert.True(order.Count >= 3, $"only {order.Count} tracks played");
            for (int index = 1; index < order.Count; index++)
            {
                Assert.NotEqual(order[index - 1], order[index]);
            }
        }
        finally
        {
            Directory.Delete(Path.GetDirectoryName(directory)!, recursive: true);
        }
    }

    /// <summary>
    /// REQ-AUD-041: each interface event maps to its clip, and the four files of SPEC-11 §2 are
    /// the ones read.
    /// </summary>
    [Fact]
    public void REQ_AUD_041_every_interface_event_has_its_clip()
    {
        string root = Path.Combine(Path.GetTempPath(), $"zax-audio-{Guid.NewGuid():N}");
        string directory = Path.Combine(root, "audio");
        Directory.CreateDirectory(directory);
        try
        {
            foreach (string name in new[]
            {
                "sfx_menu_move3.wav", "sfx_menu_move2.wav", "sfx_menu_move1.wav", "sfx_coin_double1.wav",
            })
            {
                File.WriteAllBytes(Path.Combine(directory, name), WriteWave(Tone(seconds: 0.05)));
            }

            AudioAssets assets = AudioAssets.TryOpen(root)!;

            Assert.NotNull(assets.Sound(UiSound.TabChange));
            Assert.NotNull(assets.Sound(UiSound.PageChange));
            Assert.NotNull(assets.Sound(UiSound.Move));
            Assert.NotNull(assets.Sound(UiSound.Confirm));

            AudioMixer mixer = new(Rate, assets);
            mixer.PlaySound(UiSound.Confirm);
            Assert.True(mixer.SoundPlaying);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    /// <summary>
    /// REQ-AUD-011: the audio follows the factor <c>k</c> by retiming — the core is told the
    /// factor, which changes the pitch proportionally — and REQ-AUD-004 keeps the sample rate
    /// of real time constant.
    /// </summary>
    [Fact]
    public void REQ_AUD_011_the_speed_factor_reaches_the_core()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.Settings.GameSpeed = 4;
        harness.Tick();
        Assert.Equal(2.0, harness.Host.Session!.Core.Speed, 1e-9);

        harness.Settings.GameSpeed = -1;
        harness.Tick();
        Assert.Equal(0.8, harness.Host.Session!.Core.Speed, 1e-9);
    }

    /// <summary>
    /// REQ-AUD-010: launching another game empties the core's buffer, so no fragment of the
    /// previous session can be heard.
    /// </summary>
    [Fact]
    public void REQ_AUD_010_a_new_game_drops_the_pending_audio()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(60);

        EmulationStateMachineTests.Launch(harness, "g2");

        Assert.Equal(0, harness.Host.Session!.Core.Apu.AvailableSamples);
    }

    /// <summary>
    /// REQ-AUD-006: <c>V = 0</c> silences everything, which is also what the Mute toggle of
    /// AMD-010 does without touching the stored volume.
    /// </summary>
    [Fact]
    public void REQ_AUD_006_mute_silences_everything_without_losing_the_volume()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Settings.Volume = 0.8f;
        harness.Run(ZaxCommands.ToggleMute);

        Assert.True(harness.Settings.Mute);
        Assert.Equal(0f, harness.Settings.EffectiveVolume);
        Assert.Equal(0.8f, harness.Settings.Volume, 1e-6f);
        Assert.Equal(0f, harness.Mixer.Volume, 1e-6f);

        harness.Run(ZaxCommands.ToggleMute);
        Assert.Equal(0.8f, harness.Mixer.Volume, 1e-6f);
    }

    /// <summary>A mono tone, as a decoded clip.</summary>
    private static WaveAudio Tone(double seconds, int frequency = 440)
    {
        int frames = (int)(seconds * Rate);
        float[] samples = new float[frames];
        for (int frame = 0; frame < frames; frame++)
        {
            samples[frame] = MathF.Sin(2f * MathF.PI * frequency * frame / Rate);
        }

        return new WaveAudio(samples, 1, Rate);
    }

    /// <summary>Encodes a clip as a 16-bit PCM WAVE file, to exercise the decoder.</summary>
    private static byte[] WriteWave(WaveAudio audio)
    {
        using MemoryStream stream = new();
        using BinaryWriter writer = new(stream);

        int dataBytes = audio.Samples.Length * 2;
        writer.Write("RIFF"u8);
        writer.Write(36 + dataBytes);
        writer.Write("WAVE"u8);
        writer.Write("fmt "u8);
        writer.Write(16);
        writer.Write((short)1);
        writer.Write((short)audio.Channels);
        writer.Write(audio.SampleRate);
        writer.Write(audio.SampleRate * audio.Channels * 2);
        writer.Write((short)(audio.Channels * 2));
        writer.Write((short)16);
        writer.Write("data"u8);
        writer.Write(dataBytes);

        foreach (float sample in audio.Samples)
        {
            writer.Write((short)Math.Clamp(sample * 32767f, -32768f, 32767f));
        }

        writer.Flush();
        return stream.ToArray();
    }
}
