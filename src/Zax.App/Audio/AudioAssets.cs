using Zax.Platform;

namespace Zax.App.Audio;

/// <summary>The interface sounds of SPEC-11 REQ-AUD-041.</summary>
public enum UiSound
{
    /// <summary>Main-menu tab change: <c>sfx_menu_move3.wav</c>.</summary>
    TabChange,

    /// <summary>Page change in a paged grid: <c>sfx_menu_move2.wav</c>.</summary>
    PageChange,

    /// <summary>Selection moved: <c>sfx_menu_move1.wav</c>.</summary>
    Move,

    /// <summary>Confirmation: <c>sfx_coin_double1.wav</c>.</summary>
    Confirm,
}

/// <summary>
/// The audio files of <c>assets-neutral/audio/</c>: the three menu tracks and the four
/// interface sounds of SPEC-11 §2.
/// </summary>
/// <remarks>
/// <para>
/// The interface sounds are tiny and are decoded eagerly. The tracks are 66 to 164 s of stereo
/// PCM, so only the one being played is held in memory: <see cref="LoadTrack"/> decodes on
/// demand and the playlist drops the previous one.
/// </para>
/// <para>
/// A missing folder is not an error: every accessor then returns null and the host runs without
/// interface audio, as it already runs without an audio device.
/// </para>
/// </remarks>
public sealed class AudioAssets : IUiAudioLibrary
{
    /// <summary>The menu tracks, in the order of the SPEC-11 §2 table.</summary>
    public static readonly string[] TrackFileNames =
    [
        "happynes.wav",
        "Komiku_-_02_-_Home.wav",
        "dire-space-emergency-chiptune.wav",
    ];

    private static readonly (UiSound Sound, string File)[] SoundFileNames =
    [
        (UiSound.TabChange, "sfx_menu_move3.wav"),
        (UiSound.PageChange, "sfx_menu_move2.wav"),
        (UiSound.Move, "sfx_menu_move1.wav"),
        (UiSound.Confirm, "sfx_coin_double1.wav"),
    ];

    private readonly Dictionary<UiSound, WaveAudio> _sounds = [];
    private readonly string _directory;

    private AudioAssets(string directory)
    {
        _directory = directory;
    }

    /// <summary>The folder the files came from.</summary>
    public string Directory => _directory;

    /// <summary>
    /// How many menu tracks the playlist may index. It is the length of
    /// <see cref="TrackFileNames"/> as soon as one of the files is there, so that the draw of
    /// REQ-AUD-030 keeps addressing the table of SPEC-11 §2 by position; a missing file simply
    /// gives a null from <see cref="LoadTrack"/> and the draw moves on.
    /// </summary>
    public int TrackCount => AvailableTracks > 0 ? TrackFileNames.Length : 0;

    /// <summary>How many of the track files were actually found.</summary>
    public int AvailableTracks { get; private set; }

    /// <summary>
    /// Opens <c>&lt;assetRoot&gt;/audio</c> and decodes the interface sounds. Returns null when
    /// the folder is missing or holds nothing usable.
    /// </summary>
    public static AudioAssets? TryOpen(string? assetRoot, Action<string>? log = null)
    {
        if (string.IsNullOrWhiteSpace(assetRoot))
        {
            return null;
        }

        string directory = Path.Combine(assetRoot, "audio");
        if (!System.IO.Directory.Exists(directory))
        {
            return null;
        }

        AudioAssets assets = new(directory);
        foreach ((UiSound sound, string file) in SoundFileNames)
        {
            string path = Path.Combine(directory, file);
            if (!File.Exists(path))
            {
                continue;
            }

            try
            {
                assets._sounds[sound] = WaveFile.Load(path);
            }
            catch (Exception exception) when (exception is InvalidDataException or IOException)
            {
                log?.Invoke($"{file}: {exception.Message}");
            }
        }

        foreach (string track in TrackFileNames)
        {
            if (File.Exists(Path.Combine(directory, track)))
            {
                assets.AvailableTracks++;
            }
        }

        return assets._sounds.Count > 0 || assets.AvailableTracks > 0 ? assets : null;
    }

    /// <inheritdoc/>
    public WaveAudio? Sound(UiSound sound) => _sounds.GetValueOrDefault(sound);

    /// <inheritdoc/>
    public WaveAudio? LoadTrack(int index)
    {
        if (index < 0 || index >= TrackFileNames.Length)
        {
            return null;
        }

        string path = Path.Combine(_directory, TrackFileNames[index]);
        try
        {
            return File.Exists(path) ? WaveFile.Load(path) : null;
        }
        catch (Exception exception) when (exception is InvalidDataException or IOException)
        {
            return null;
        }
    }
}
