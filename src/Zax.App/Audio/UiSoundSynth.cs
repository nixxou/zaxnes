using Zax.Platform;

namespace Zax.App.Audio;

/// <summary>
/// The interface sounds of SPEC-11 REQ-AUD-041, synthesised (AMD-024 REQ-PUB-005): one distinct
/// clip per event, produced from the oscillators of <see cref="ChipVoices"/>.
/// </summary>
/// <remarks>
/// <para>
/// The four events keep the <b>roles</b> and the rough lengths the table of SPEC-11 §2 gives —
/// a 39 ms blip for a change of selection, 60 ms for a change of page, 76 ms for a change of tab
/// and 209 ms for a validation — and each one has a timbre of its own so that they are told
/// apart by ear. The exact sound of the original clips is a divergence <b>D</b> (AMD-024).
/// </para>
/// <para>
/// The clips are mono, like the files SPEC-11 §2 describes, and are produced once: four short
/// arrays, a few kilobytes in all.
/// </para>
/// </remarks>
public static class UiSoundSynth
{
    /// <summary>Length of each clip in seconds, from the table of SPEC-11 §2.</summary>
    public static double Duration(UiSound sound) => sound switch
    {
        UiSound.TabChange => 0.076,
        UiSound.PageChange => 0.060,
        UiSound.Move => 0.039,
        _ => 0.209,
    };

    /// <summary>Renders the clip of <paramref name="sound"/> at <paramref name="sampleRate"/> Hz.</summary>
    /// <param name="sound">The interface event.</param>
    /// <param name="sampleRate">Sample rate of the produced clip.</param>
    public static WaveAudio Render(UiSound sound, int sampleRate)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(sampleRate, 8000);

        double seconds = Duration(sound);
        int frames = Math.Max(1, (int)Math.Round(seconds * sampleRate));
        float[] samples = new float[frames];

        ChipVoices.NoiseSource noise = new((ushort)(1 + (int)sound));
        double phase = 0.0;
        double noisePhase = 0.0;
        float noiseValue = noise.Next();

        for (int frame = 0; frame < frames; frame++)
        {
            double time = (double)frame / sampleRate;
            double t = time / seconds;
            float value = sound switch
            {
                // Selection: a bare triangle blip, high and dry. The quietest of the four, since
                // it fires on every movement of the cursor.
                UiSound.Move => 0.45f
                    * ChipVoices.Triangle(phase)
                    * ChipVoices.Envelope(time, seconds, 0.001, 0.012),

                // Page: a pulse falling a fifth — « further back » in the list.
                UiSound.PageChange => 0.5f
                    * ChipVoices.Pulse(phase, 0.5f)
                    * ChipVoices.Envelope(time, seconds, 0.002, 0.022),

                // Tab: a pulse rising a fifth, with a noise tick on the attack so that it reads
                // as a wider move than a page change.
                UiSound.TabChange => (0.42f
                        * ChipVoices.Pulse(phase, 0.25f)
                        * ChipVoices.Envelope(time, seconds, 0.002, 0.030))
                    + (0.20f * noiseValue * ChipVoices.Envelope(time, seconds, 0.0005, 0.006)),

                // Validation: two notes a fourth apart, the second louder — a coin, not a beep.
                _ => 0.55f
                    * ChipVoices.Pulse(phase, 0.5f)
                    * ChipVoices.Envelope(time, seconds, 0.002, 0.090),
            };

            samples[frame] = Math.Clamp(value, -1f, 1f);

            phase += Pitch(sound, t) / sampleRate;

            // The noise source is clocked well below the sample rate, which is what gives it a
            // grain rather than a hiss.
            noisePhase += 5500.0 / sampleRate;
            if (noisePhase >= 1.0)
            {
                noisePhase -= Math.Floor(noisePhase);
                noiseValue = noise.Next();
            }
        }

        return new WaveAudio(samples, Channels: 1, sampleRate);
    }

    /// <summary>
    /// The frequency of the clip at the normalised time <paramref name="t"/> in [0, 1]. This is
    /// what makes each event recognisable: a flat blip, a fall, a rise, and a two-note step.
    /// </summary>
    private static double Pitch(UiSound sound, double t) => sound switch
    {
        UiSound.Move => ChipVoices.Frequency(93),
        UiSound.PageChange => ChipVoices.Frequency(86 - (7.0 * t)),
        UiSound.TabChange => ChipVoices.Frequency(79 + (7.0 * t)),
        _ => ChipVoices.Frequency(t < 0.33 ? 83 : 88),
    };
}
