using Zax.Platform;

namespace Zax.App.Audio;

/// <summary>
/// Where the mixer takes the interface sounds and the menu tracks of SPEC-11 §2.
/// </summary>
/// <remarks>
/// AMD-024 REQ-PUB-005 replaces the audio files of the original build by code: the publishable
/// build plugs <see cref="SynthAudioLibrary"/> in, which synthesises one distinct sound per
/// event of REQ-AUD-041 and reports <see cref="TrackCount"/> 0 — the revision of 2026-10-21
/// leaves the publishable version with <b>no menu music at all</b>. <see cref="AudioAssets"/>
/// is the other implementation, for the optional <c>--assets &lt;folder&gt;</c> mode where the
/// user supplies WAV files, and it is the only one that ever returns a track. The playback
/// rules of SPEC-11 — single voice, no loop, random draw among the others, fades and gains —
/// live in <see cref="AudioMixer"/> and are the same either way.
/// </remarks>
public interface IUiAudioLibrary
{
    /// <summary>
    /// How many menu tracks exist. The valid indices for <see cref="LoadTrack"/> are
    /// <c>0</c> to <c>TrackCount − 1</c>; REQ-AUD-030 needs at least two for the « never twice in
    /// a row » rule to mean anything.
    /// </summary>
    int TrackCount { get; }

    /// <summary>The clip of an interface event, or null when there is none.</summary>
    WaveAudio? Sound(UiSound sound);

    /// <summary>
    /// The menu track <paramref name="index"/>, or null when the index is out of range or the
    /// track cannot be produced. Only the track being played is held in memory, so this is
    /// called again every time the playlist moves on.
    /// </summary>
    WaveAudio? LoadTrack(int index);
}
