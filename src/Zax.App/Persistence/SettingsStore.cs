using Zax.Settings;

namespace Zax.App.Persistence;

/// <summary>
/// The blob store of REQ-PERS-082 backed by the files of the user root: a name is a file of
/// <see cref="UserDataPaths.Root"/>, written atomically (REQ-PERS-021).
/// </summary>
/// <remarks>
/// Nothing throws out of here. A root that cannot be written to leaves
/// <see cref="UserDataPaths.IsWritable"/> false and every write simply fails, which is the
/// degraded mode of REQ-PERS-073: the program runs on in-memory defaults and the write actions
/// report the notification.
/// </remarks>
public sealed class UserDataSettingsStorage(UserDataPaths paths) : ISettingsStorage
{
    private readonly UserDataPaths _paths = paths ?? throw new ArgumentNullException(nameof(paths));

    /// <inheritdoc/>
    public bool IsWritable => _paths.IsWritable;

    /// <inheritdoc/>
    public bool Exists(string name) => File.Exists(PathOf(name));

    /// <inheritdoc/>
    public byte[]? Read(string name)
    {
        try
        {
            string path = PathOf(name);
            return File.Exists(path) ? File.ReadAllBytes(path) : null;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <inheritdoc/>
    public bool Write(string name, ReadOnlySpan<byte> contents)
    {
        if (!IsWritable)
        {
            return false;
        }

        try
        {
            UserDataPaths.WriteAtomic(PathOf(name), contents);
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <inheritdoc/>
    public bool Rename(string name, string target)
    {
        try
        {
            string path = PathOf(name);
            if (!File.Exists(path))
            {
                return false;
            }

            File.Move(path, PathOf(target), overwrite: true);
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <inheritdoc/>
    public bool Copy(string name, string target)
    {
        try
        {
            string path = PathOf(name);
            if (!File.Exists(path))
            {
                return false;
            }

            File.Copy(path, PathOf(target), overwrite: true);
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    private string PathOf(string name) => Path.Combine(_paths.Root, name);
}

/// <summary>
/// Reads and writes <c>settings.json</c> and <c>games.json</c> of the user root, and tells the
/// host which notification it owes the user (REQ-PERS-020 to REQ-PERS-024).
/// </summary>
/// <remarks>
/// The work itself is in <see cref="SettingsFile"/> and <see cref="GameConfigFile"/>, which know
/// nothing of the file system; this holds them together over a
/// <see cref="UserDataSettingsStorage"/> and keeps the deferred write of REQ-PERS-021 — the
/// settings file is written at the latest two seconds after a change made from the UI, so a
/// brutal stop does not lose it.
/// </remarks>
public sealed class SettingsStore
{
    /// <summary>Schema version this build writes.</summary>
    public const int SchemaVersion = SettingsFile.CurrentSchemaVersion;

    /// <summary>Notification shown when the file could not be read (REQ-PERS-022).</summary>
    public const string UnreadableMessage = SettingsFile.UnreadableMessage;

    /// <summary>Notification shown for a file from a newer version (REQ-PERS-024).</summary>
    public const string NewerVersionMessage = SettingsFile.NewerVersionMessage;

    /// <summary>How long a change from the UI may wait before it reaches the disk.</summary>
    public static readonly TimeSpan WriteDelay = TimeSpan.FromSeconds(2);

    private readonly SettingsFile _file;
    private readonly GameConfigFile _games;
    private readonly List<string> _messages = [];

    private SettingsModel? _model;
    private TimeSpan? _dueAt;
    private bool _pending;
    private bool _subscribed;

    /// <summary>Creates a store for the settings file of <paramref name="paths"/>.</summary>
    public SettingsStore(UserDataPaths paths)
    {
        ArgumentNullException.ThrowIfNull(paths);
        UserDataSettingsStorage storage = new(paths);
        _file = new SettingsFile(storage, SettingsFile.DefaultName, SettingsMigrations.All);
        _games = new GameConfigFile(storage);
    }

    /// <summary>The notification the last read asks the host to show, or null.</summary>
    public string? Message => _messages.Count > 0 ? _messages[0] : null;

    /// <summary>Every notification the last read produced, in order.</summary>
    public IReadOnlyList<string> Messages => _messages;

    /// <summary>
    /// Whether the file came from a newer schema version, in which case it is not written again
    /// during this session (REQ-PERS-024).
    /// </summary>
    public bool IsReadOnlySession { get; private set; }

    /// <summary>The per-ROM choices and the recent ROM list (AMD-010 REQ-UI-200, REQ-UI-201).</summary>
    public GameConfigFile Games => _games;

    /// <summary>
    /// Reads the files into <paramref name="settings"/>. Absent files → every default
    /// (REQ-PERS-002, REQ-PERS-075).
    /// </summary>
    public void Load(AppSettings settings, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(settings);

        _messages.Clear();
        _model = settings.Model;

        SettingsLoadResult result = _file.Load(settings.Model, now);
        IsReadOnlySession = result.IsReadOnlySession;
        if (result.Message is { } message)
        {
            _messages.Add(message);
        }

        if (_games.Load(now).Message is { } gameMessage)
        {
            _messages.Add(gameMessage);
        }

        if (result.Status == SettingsLoadStatus.Migrated)
        {
            // A migrated file is written again at the current version straight away
            // (REQ-PERS-024).
            _file.Save(settings.Model);
        }

        // From now on, a change made from the UI schedules a write (REQ-PERS-021). Subscribed
        // once: Setting:Load() comes back through here and must not stack handlers.
        if (!_subscribed)
        {
            _subscribed = true;
            settings.Changed += OnSettingChanged;
        }
    }

    /// <summary>Writes both files now; the normal exit does this (REQ-LOOP-052).</summary>
    public void Save(AppSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);

        _dueAt = null;
        _pending = false;
        _file.Save(settings.Model);
        _games.Save();
    }

    /// <summary>
    /// Writes the files if a change is waiting and its two seconds are up (REQ-PERS-021). The
    /// loop calls this once per frame with its own clock.
    /// </summary>
    public void Tick(AppSettings settings, TimeSpan elapsed)
    {
        ArgumentNullException.ThrowIfNull(settings);

        if (_pending && _dueAt is null)
        {
            _dueAt = elapsed + WriteDelay;
        }

        if (_dueAt is { } due && elapsed >= due)
        {
            Save(settings);
        }
    }

    /// <summary>Asks for a write within <see cref="WriteDelay"/> of <paramref name="elapsed"/>.</summary>
    public void ScheduleWrite(TimeSpan elapsed) => _dueAt ??= elapsed + WriteDelay;

    /// <summary>Whether a change is waiting to be written.</summary>
    public bool HasPendingWrite => _pending || _dueAt is not null;

    private void OnSettingChanged(string key)
    {
        if (_model is null || SettingsCatalogue.Find(key) is not { IsPersisted: true })
        {
            return;
        }

        // A script's write never becomes a preference, so it never schedules a write either
        // (REQ-PERS-010). Neither does a command-line override: it holds for this run only and
        // the file keeps the user's own choice (Q-066).
        if (_model.HasSessionValue(key) || _model.HasCommandLineValue(key))
        {
            return;
        }

        _pending = true;
    }
}

/// <summary>
/// The migration chain of the settings file (REQ-PERS-024). Empty for now: version 1 is the
/// first one this reimplementation ever published, so there is nothing earlier to come from.
/// The hook is here, and tested, so that the first schema change has somewhere to go.
/// </summary>
public static class SettingsMigrations
{
    /// <summary>Every step, in no particular order; the file sorts them.</summary>
    public static IReadOnlyList<ISettingsMigration> All { get; } = [];
}
