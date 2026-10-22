namespace Zax.App.Persistence;

/// <summary>
/// Persists the battery-backed cartridge RAM (SPEC-09 REQ-PERS-060), one file per ROM named
/// after its SHA-1: <c>battery/&lt;sha1&gt;.srm</c>.
/// </summary>
/// <remarks>
/// <para>
/// This is a deliberate divergence (class D): the original loses a game's internal saves when
/// it closes. Keying on the ROM fingerprint rather than on the profile identifier is what lets
/// "2D Mode" be used with several ROMs.
/// </para>
/// <para>
/// The file is read when the ROM is loaded and written when the game stops, when the program
/// exits and at most every 30 s while its content changes.
/// </para>
/// </remarks>
public sealed class BatteryRamStore
{
    /// <summary>Shortest interval between two automatic writes (REQ-PERS-060).</summary>
    public const double AutoSaveSeconds = 30.0;

    private readonly UserDataPaths _paths;

    private string? _fingerprint;
    private byte[] _lastWritten = [];
    private double _sinceWrite;

    /// <summary>Creates a store under <paramref name="paths"/>.</summary>
    public BatteryRamStore(UserDataPaths paths)
    {
        _paths = paths ?? throw new ArgumentNullException(nameof(paths));
    }

    /// <summary>Number of writes performed, for tests and logs.</summary>
    public int Writes { get; private set; }

    /// <summary>The ROM the store currently follows, or null.</summary>
    public string? Fingerprint => _fingerprint;

    /// <summary>
    /// Starts following the ROM of fingerprint <paramref name="romSha1"/> and returns its saved
    /// RAM, or null when there is none.
    /// </summary>
    public byte[]? Attach(string romSha1)
    {
        ArgumentException.ThrowIfNullOrEmpty(romSha1);

        _fingerprint = romSha1;
        _sinceWrite = 0.0;
        _lastWritten = [];

        string file = _paths.BatteryFile(romSha1);
        try
        {
            if (File.Exists(file))
            {
                byte[] data = File.ReadAllBytes(file);
                _lastWritten = data;
                return data;
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A ROM whose save file cannot be read simply starts with empty RAM.
        }

        return null;
    }

    /// <summary>Stops following the current ROM, without writing.</summary>
    public void Detach()
    {
        _fingerprint = null;
        _lastWritten = [];
        _sinceWrite = 0.0;
    }

    /// <summary>
    /// Writes <paramref name="ram"/> when it differs from what is on disk. Does nothing when no
    /// ROM is attached, when the cartridge has no battery (empty span) or when the root is not
    /// writable (REQ-PERS-073).
    /// </summary>
    /// <returns>True when a file was written.</returns>
    public bool Flush(ReadOnlySpan<byte> ram)
    {
        if (_fingerprint is null || ram.IsEmpty || !_paths.IsWritable)
        {
            return false;
        }

        if (ram.SequenceEqual(_lastWritten))
        {
            return false;
        }

        try
        {
            byte[] copy = ram.ToArray();
            UserDataPaths.WriteAtomic(_paths.BatteryFile(_fingerprint), copy);
            _lastWritten = copy;
            _sinceWrite = 0.0;
            Writes++;
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Advances the automatic-write timer and writes when 30 s have passed and the content
    /// changed.
    /// </summary>
    /// <returns>True when a file was written.</returns>
    public bool Advance(double realSeconds, ReadOnlySpan<byte> ram)
    {
        if (_fingerprint is null || ram.IsEmpty)
        {
            return false;
        }

        _sinceWrite += realSeconds;
        return _sinceWrite >= AutoSaveSeconds && Flush(ram);
    }

    /// <summary>Deletes the saved RAM of a ROM; used by tests and by a clean-up action.</summary>
    public void Delete(string romSha1)
    {
        try
        {
            string file = _paths.BatteryFile(romSha1);
            if (File.Exists(file))
            {
                File.Delete(file);
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // Nothing to do.
        }
    }
}
