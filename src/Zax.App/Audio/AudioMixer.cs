using Zax.Platform;

namespace Zax.App.Audio;

/// <summary>Reads at most <c>destination.Length</c> mono samples; returns how many it wrote.</summary>
/// <param name="destination">Where the samples go.</param>
public delegate int AudioSampleSource(Span<float> destination);

/// <summary>
/// The output mixer of SPEC-11: the console signal, the menu music and the interface sounds,
/// each with its own gain and fade, mixed into interleaved stereo blocks.
/// </summary>
/// <remarks>
/// <para>
/// The console signal is monophonic and goes to both channels unchanged (REQ-AUD-002). A block
/// of console audio is taken all or nothing: when the core has less than a full block, the
/// console contribution of that block is digital silence and the samples stay in the core's
/// buffer, so no block ever mixes signal with padding (REQ-AUD-003). The interface sounds are
/// still mixed in, because they are played whatever the emulation does (REQ-AUD-040).
/// </para>
/// <para>
/// Gains: <c>V</c> for the console, <c>0.02 × V</c> for the music, <c>0.07 × V</c> for the
/// interface sounds, applied to the file samples without normalisation (REQ-AUD-006,
/// REQ-AUD-021). The music plays while the state is not Running and fades out when it becomes
/// Running (REQ-AUD-022); tracks never loop and the next one is drawn among the other two
/// (REQ-AUD-030). Interface sounds share a single voice: a new one cuts the one playing
/// (REQ-AUD-040).
/// </para>
/// <para>
/// Nothing here touches the audio device: <see cref="Fill"/> is driven by
/// <see cref="Zax.Platform.AudioOutput"/> in the application and directly by the tests.
/// </para>
/// </remarks>
public sealed class AudioMixer
{
    /// <summary>Frames per output block, 512 (REQ-AUD-003).</summary>
    public const int BlockFrames = AudioOutput.BlockFrames;

    /// <summary>Music gain factor applied to <c>V</c> (REQ-AUD-021).</summary>
    public const float MusicGainFactor = 0.02f;

    /// <summary>Interface-sound gain factor applied to <c>V</c> (REQ-AUD-021).</summary>
    public const float SoundGainFactor = 0.07f;

    private readonly FadeGain _emulationGain;
    private readonly FadeGain _musicGain = new(0.0);
    private readonly IUiAudioLibrary? _assets;
    private readonly Random _random;

    private float[] _mono = new float[BlockFrames];

    private float _volume = 1f;
    private bool _menuMusicEnabled = true;
    private bool _emulationAudible;
    private bool _everRan;

    private WaveAudio? _track;
    private int _trackIndex = -1;
    private double _trackPosition;
    private bool _trackSuspended;

    private WaveAudio? _sound;
    private double _soundPosition;

    /// <summary>Creates a mixer for an output at <paramref name="sampleRate"/> Hz.</summary>
    /// <param name="sampleRate">Effective output rate (REQ-AUD-001).</param>
    /// <param name="assets">
    /// Where the clips and the tracks come from, or null to run without interface audio. The
    /// publishable build passes <see cref="SynthAudioLibrary"/> (AMD-024 REQ-PUB-005); the
    /// optional <c>--assets</c> mode passes <see cref="AudioAssets"/>.
    /// </param>
    /// <param name="random">
    /// Source of the track draw; a fixed seed makes the choice reproducible in a test, while the
    /// product uses the default so that it varies from one run to the next (REQ-AUD-030, notes).
    /// </param>
    public AudioMixer(int sampleRate, IUiAudioLibrary? assets = null, Random? random = null)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(sampleRate, 1);

        SampleRate = sampleRate;
        _assets = assets;
        _random = random ?? Random.Shared;

        // REQ-AUD-020: the first launch of a game starts at gain V — no previous fade has
        // lowered it — so the console gain starts settled at V and only fades once a game has
        // actually run. REQ-AUD-022: the music starts at 0 and rises with the same slope.
        _emulationGain = new FadeGain(_volume);
        _emulationGain.FadeTo(EmulationTarget, _volume);
        _musicGain.FadeTo(MusicTarget, MusicRange);
    }

    /// <summary>The output rate the mixer resamples the files to.</summary>
    public int SampleRate { get; }

    /// <summary>Where the console samples come from; null runs without emulation audio.</summary>
    public AudioSampleSource? EmulationSource { get; set; }

    /// <summary><c>V</c>, the general volume in [0 ; 1] (REQ-AUD-006).</summary>
    public float Volume
    {
        get => _volume;
        set
        {
            float clamped = Math.Clamp(value, 0f, 1f);
            if (Math.Abs(clamped - _volume) < 1e-7f)
            {
                return;
            }

            _volume = clamped;

            // REQ-AUD-006 and REQ-AUD-023: a volume change is immediate when the gain had
            // settled, and does not cut a fade that is still running.
            _emulationGain.Retarget(EmulationTarget, _volume);
            _musicGain.Retarget(MusicTarget, MusicRange);
        }
    }

    /// <summary><c>M</c>, the <c>BG Music</c> setting; a change takes effect at once.</summary>
    public bool MenuMusicEnabled
    {
        get => _menuMusicEnabled;
        set
        {
            if (_menuMusicEnabled == value)
            {
                return;
            }

            _menuMusicEnabled = value;

            // REQ-AUD-023: enabling or disabling M brings the music gain to its new target
            // immediately, with no fade.
            _musicGain.Snap(MusicTarget, MusicRange);
            if (value)
            {
                _trackSuspended = false;
            }
        }
    }

    /// <summary>
    /// Whether the emulation is audible, that is whether the state is Running
    /// (REQ-LOOP-003, REQ-AUD-010).
    /// </summary>
    public bool EmulationAudible
    {
        get => _emulationAudible;
        set
        {
            if (_emulationAudible == value)
            {
                return;
            }

            _emulationAudible = value;
            _everRan |= value;

            // REQ-AUD-022: leaving Running makes the menu music active again, so the track has to
            // come off suspension — otherwise MixMusic returns at once and the menu stays silent
            // for the rest of the session even though MusicGain climbs back to its target.
            if (!value)
            {
                _trackSuspended = false;
            }

            // REQ-AUD-020 and REQ-AUD-022: the state change is what starts a fade.
            _emulationGain.FadeTo(EmulationTarget, _volume);
            _musicGain.FadeTo(MusicTarget, MusicRange);
        }
    }

    /// <summary>Current console gain, between 0 and <c>V</c>.</summary>
    public float EmulationGain => _emulationGain.Value;

    /// <summary>Current music gain, between 0 and <c>0.02 × V</c>.</summary>
    public float MusicGain => _musicGain.Value;

    /// <summary>Gain of the interface sounds, <c>0.07 × V</c> with no fade.</summary>
    public float SoundGain => SoundGainFactor * _volume;

    /// <summary>Blocks whose console contribution was silence for want of samples.</summary>
    public long UnderrunBlocks { get; private set; }

    /// <summary>Blocks the mixer has produced.</summary>
    public long Blocks { get; private set; }

    /// <summary>Index of the menu track playing, or −1 when none is.</summary>
    public int CurrentTrack => _trackIndex;

    /// <summary>How many times a new menu track was drawn.</summary>
    public int TrackChanges { get; private set; }

    /// <summary>Whether the menu track is suspended at its position (DEC-11-Q1).</summary>
    public bool MusicSuspended => _trackSuspended;

    /// <summary>Whether an interface sound is playing.</summary>
    public bool SoundPlaying => _sound is not null;

    /// <summary>
    /// How many interface sounds have been started since power-on. It is what makes "exactly
    /// one validation sound per change of value" of AMD-021 R2-013 observable: the single voice
    /// of REQ-AUD-040 cuts the previous clip, so <see cref="SoundPlaying"/> alone cannot tell
    /// one sound from two.
    /// </summary>
    public long SoundsPlayed { get; private set; }

    /// <summary>
    /// Plays an interface sound on the single voice, cutting whatever was playing
    /// (REQ-AUD-040). Does nothing when the clip is missing.
    /// </summary>
    public void PlaySound(UiSound sound) => PlayClip(_assets?.Sound(sound));

    /// <summary>Plays a clip directly; used by tests.</summary>
    public void PlayClip(WaveAudio? clip)
    {
        if (clip is null || clip.FrameCount == 0)
        {
            return;
        }

        _sound = clip;
        _soundPosition = 0.0;
        SoundsPlayed++;
    }

    /// <summary>Stops the interface voice.</summary>
    public void StopSound()
    {
        _sound = null;
        _soundPosition = 0.0;
    }

    /// <summary>
    /// Fills <paramref name="stereo"/> — interleaved left/right — with one or more blocks of
    /// audio and advances the fades by the real time those blocks represent.
    /// </summary>
    public void Fill(Span<float> stereo)
    {
        if (stereo.Length < 2)
        {
            return;
        }

        int frames = stereo.Length / 2;
        stereo.Clear();

        MixEmulation(stereo, frames);
        MixMusic(stereo, frames);
        MixSound(stereo, frames);

        for (int i = 0; i < stereo.Length; i++)
        {
            stereo[i] = Math.Clamp(stereo[i], -1f, 1f);
        }

        Advance((double)frames / SampleRate);
        Blocks++;
    }

    /// <summary>
    /// Advances the fades and the music selection by <paramref name="realSeconds"/> without
    /// producing audio: what the host does when no device is open, so that the state machine
    /// behaves the same silently.
    /// </summary>
    public void Advance(double realSeconds)
    {
        _emulationGain.Advance(realSeconds);
        _musicGain.Advance(realSeconds);

        // REQ-AUD-022: once the fade-out has reached 0 the track is suspended, and DEC-11-Q1
        // says it resumes at that position.
        if (!MusicActive && _musicGain.Value <= 0f)
        {
            _trackSuspended = true;
        }
    }

    private bool MusicActive => !_emulationAudible;

    private double MusicTarget => MusicActive && _menuMusicEnabled ? MusicGainFactor * _volume : 0.0;

    private double MusicRange => MusicGainFactor * _volume;

    private double EmulationTarget => _emulationAudible || !_everRan ? _volume : 0.0;

    private void MixEmulation(Span<float> stereo, int frames)
    {
        if (EmulationSource is not { } source)
        {
            return;
        }

        if (_mono.Length < frames)
        {
            _mono = new float[frames];
        }

        Span<float> mono = _mono.AsSpan(0, frames);
        int written = source(mono);
        if (written < frames)
        {
            // REQ-AUD-003: all or nothing. The samples the core did have stay in its buffer for
            // the next call, so nothing is lost and no block is half filled. Outside Running no
            // frame produces samples, so an empty buffer is the expected silence of
            // REQ-AUD-010, not an under-run.
            if (_emulationAudible)
            {
                UnderrunBlocks++;
            }

            return;
        }

        float gain = _emulationGain.Value;
        for (int frame = 0; frame < frames; frame++)
        {
            float value = mono[frame] * gain;
            stereo[frame * 2] += value;
            stereo[(frame * 2) + 1] += value;
        }
    }

    private void MixMusic(Span<float> stereo, int frames)
    {
        if (_trackSuspended)
        {
            return;
        }

        if (_track is null && MusicActive && _assets is { TrackCount: > 0 })
        {
            SelectTrack();
        }

        if (_track is not { } track)
        {
            return;
        }

        float gain = _musicGain.Value;
        double ratio = (double)track.SampleRate / SampleRate;
        int channels = track.Channels;
        int count = track.FrameCount;

        for (int frame = 0; frame < frames; frame++)
        {
            if (_trackPosition >= count - 1)
            {
                // REQ-AUD-030: a track does not loop; while the music is active a different one
                // starts straight away.
                if (MusicActive && _assets is { TrackCount: > 1 })
                {
                    SelectTrack();
                    track = _track!;
                    ratio = (double)track.SampleRate / SampleRate;
                    channels = track.Channels;
                    count = track.FrameCount;
                }
                else
                {
                    _track = null;
                    _trackIndex = -1;
                    return;
                }
            }

            (float left, float right) = Sample(track.Samples, channels, count, _trackPosition);
            stereo[frame * 2] += left * gain;
            stereo[(frame * 2) + 1] += right * gain;
            _trackPosition += ratio;
        }
    }

    private void MixSound(Span<float> stereo, int frames)
    {
        if (_sound is not { } clip)
        {
            return;
        }

        float gain = SoundGain;
        double ratio = (double)clip.SampleRate / SampleRate;
        int channels = clip.Channels;
        int count = clip.FrameCount;

        for (int frame = 0; frame < frames; frame++)
        {
            if (_soundPosition >= count - 1)
            {
                StopSound();
                return;
            }

            (float left, float right) = Sample(clip.Samples, channels, count, _soundPosition);
            stereo[frame * 2] += left * gain;
            stereo[(frame * 2) + 1] += right * gain;
            _soundPosition += ratio;
        }
    }

    private static (float Left, float Right) Sample(float[] samples, int channels, int count, double position)
    {
        int index = (int)position;
        if (index < 0)
        {
            return (0f, 0f);
        }

        int next = Math.Min(index + 1, count - 1);
        float blend = (float)(position - index);

        if (channels == 1)
        {
            float mono = Lerp(samples[index], samples[next], blend);
            return (mono, mono);
        }

        float left = Lerp(samples[index * 2], samples[next * 2], blend);
        float right = Lerp(samples[(index * 2) + 1], samples[(next * 2) + 1], blend);
        return (left, right);
    }

    private static float Lerp(float a, float b, float t) => a + ((b - a) * t);

    private void SelectTrack()
    {
        if (_assets is not { TrackCount: > 0 } assets)
        {
            return;
        }

        int total = assets.TrackCount;
        int previous = _trackIndex;

        // REQ-AUD-030: a random track at start-up, then a random one among the others, so the
        // same track never plays twice in a row.
        for (int attempt = 0; attempt < 16; attempt++)
        {
            int candidate = _random.Next(total);
            if (candidate == previous && total > 1)
            {
                continue;
            }

            if (assets.LoadTrack(candidate) is { FrameCount: > 0 } track)
            {
                _track = track;
                _trackIndex = candidate;
                _trackPosition = 0.0;
                TrackChanges++;
                return;
            }
        }

        // Every draw failed: fall back to the first readable track, whichever it is.
        for (int candidate = 0; candidate < total; candidate++)
        {
            if (assets.LoadTrack(candidate) is { FrameCount: > 0 } track)
            {
                _track = track;
                _trackIndex = candidate;
                _trackPosition = 0.0;
                TrackChanges++;
                return;
            }
        }

        _track = null;
        _trackIndex = -1;
    }
}
