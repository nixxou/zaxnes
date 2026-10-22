using Zax.Platform;

namespace Zax.App.Audio;

/// <summary>
/// The interface audio of the publishable build (AMD-024 REQ-PUB-005): the four sounds of
/// SPEC-11 REQ-AUD-041, synthesised by <see cref="UiSoundSynth"/>. No file is read, and there is
/// no menu music.
/// </summary>
/// <remarks>
/// <para>
/// REQ-PUB-005 was revised on 2026-10-21: the publishable version carries <b>no menu music at
/// all</b>. The three WAV tracks of SPEC-11 §2 were copies taken from the original build
/// (<c>legal/P7-ASSET-INVENTORY.md</c>) and are not replaced; the playback rules of REQ-AUD-021,
/// REQ-AUD-022 and REQ-AUD-030 therefore have nothing to play and become a divergence <b>D</b>.
/// <see cref="TrackCount"/> is 0, so <see cref="AudioMixer"/> never starts a track and the
/// welcome screen and the menus are silent apart from the interface sounds.
/// </para>
/// <para>
/// The mixer keeps its music path: a folder given with <c>--assets</c> supplies
/// <see cref="AudioAssets"/>, whose tracks are the user's own files, and that is what the
/// opt-in tests of SPEC-11 §3.3 and §3.4 exercise.
/// </para>
/// <para>
/// The four clips are a few kilobytes in all and are rendered when the library is built.
/// </para>
/// </remarks>
public sealed class SynthAudioLibrary : IUiAudioLibrary
{
    private readonly Dictionary<UiSound, WaveAudio> _sounds = [];

    /// <summary>Creates the library at <paramref name="sampleRate"/> Hz.</summary>
    /// <param name="sampleRate">
    /// Rate the clips are produced at. The mixer resamples anyway, so this only decides the
    /// quality of the synthesis; the output rate of REQ-AUD-001 is the natural choice.
    /// </param>
    public SynthAudioLibrary(int sampleRate = AudioOutput.PreferredSampleRate)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(sampleRate, 8000);

        foreach (UiSound sound in Enum.GetValues<UiSound>())
        {
            _sounds[sound] = UiSoundSynth.Render(sound, sampleRate);
        }
    }

    /// <inheritdoc/>
    /// <remarks>Always 0: REQ-PUB-005 ships no menu music.</remarks>
    public int TrackCount => 0;

    /// <inheritdoc/>
    public WaveAudio? Sound(UiSound sound) => _sounds.GetValueOrDefault(sound);

    /// <inheritdoc/>
    /// <remarks>Always null: REQ-PUB-005 ships no menu music.</remarks>
    public WaveAudio? LoadTrack(int index) => null;
}
