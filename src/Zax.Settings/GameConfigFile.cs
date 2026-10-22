using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Zax.Settings;

/// <summary>
/// The per-ROM choices the classic interface needs: the profile the user picked for a ROM file
/// and the ten most recent ROMs (AMD-010 REQ-UI-201, REQ-UI-200, REQ-UI-207).
/// </summary>
/// <remarks>
/// <para>
/// This file deliberately holds <b>no settings</b>. REQ-PERS-031 is explicit: what is remembered
/// per game is the ROM path, the title, the date of the last session and the cover, all four
/// keyed by profile identifier in <c>library.json</c> — "no other setting is remembered per
/// game: neither camera, nor speed, nor region, nor Lua data". The two things here are not
/// settings and not per game: they are per <b>ROM file</b>, which SPEC-09 never had to describe
/// because the original always starts from a game, never from a ROM. AMD-010 reverses that
/// order, so the association has to live somewhere; it lives beside <c>library.json</c> rather
/// than inside it, because its key is a path and not a profile identifier. See Q-036.
/// </para>
/// <para>
/// Same rules as the settings file: a <c>schemaVersion</c>, atomic writes, an unreadable file
/// set aside as <c>*.corrupt-&lt;timestamp&gt;</c> with a notification, a file from a newer
/// version read at best and left alone (REQ-PERS-021, REQ-PERS-022, REQ-PERS-024).
/// </para>
/// </remarks>
// Q-036
public sealed class GameConfigFile
{
    /// <summary>The blob name, beside <c>library.json</c> in the user root.</summary>
    public const string DefaultName = "games.json";

    /// <summary>The schema version this build writes.</summary>
    public const int CurrentSchemaVersion = 1;

    /// <summary>How many recent ROMs the File menu lists (AMD-010 REQ-UI-200).</summary>
    public const int RecentRomLimit = 10;

    /// <summary>Notification shown when the file could not be read.</summary>
    public const string UnreadableMessage = "Game list was unreadable and has been reset.";

    /// <summary>Notification shown for a file from a newer version.</summary>
    public const string NewerVersionMessage = "Game list was created by a newer version.";

    private static readonly UTF8Encoding Utf8WithoutMark = new(encoderShouldEmitUTF8Identifier: false);

    private readonly ISettingsStorage _storage;
    private readonly string _name;
    private readonly List<string> _recent = [];
    private readonly Dictionary<string, string> _romProfiles = new(StringComparer.OrdinalIgnoreCase);

    private bool _readOnlySession;

    /// <summary>Creates the file over <paramref name="storage"/>.</summary>
    public GameConfigFile(ISettingsStorage storage, string? name = null)
    {
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _name = string.IsNullOrWhiteSpace(name) ? DefaultName : name;
    }

    /// <summary>The recent ROM paths, most recent first, at most ten.</summary>
    public IReadOnlyList<string> RecentRoms => _recent;

    /// <summary>Whether a change has not been written yet.</summary>
    public bool IsDirty { get; private set; }

    /// <summary>
    /// Records that <paramref name="romPath"/> was just started: it moves to the head of the
    /// recent list, which keeps at most <see cref="RecentRomLimit"/> entries.
    /// </summary>
    public void NoteRomOpened(string romPath)
    {
        if (string.IsNullOrWhiteSpace(romPath))
        {
            return;
        }

        _recent.RemoveAll(path => string.Equals(path, romPath, StringComparison.OrdinalIgnoreCase));
        _recent.Insert(0, romPath);
        while (_recent.Count > RecentRomLimit)
        {
            _recent.RemoveAt(_recent.Count - 1);
        }

        IsDirty = true;
    }

    /// <summary>Forgets <paramref name="romPath"/>, for a file that has disappeared.</summary>
    public bool ForgetRom(string romPath)
    {
        if (string.IsNullOrWhiteSpace(romPath))
        {
            return false;
        }

        int removed = _recent.RemoveAll(
            path => string.Equals(path, romPath, StringComparison.OrdinalIgnoreCase));
        bool dropped = _romProfiles.Remove(romPath);
        IsDirty |= removed > 0 || dropped;
        return removed > 0 || dropped;
    }

    /// <summary>
    /// The profile identifier the user chose for <paramref name="romPath"/>, or null when this
    /// ROM has never been started (AMD-010 REQ-UI-201 step 1).
    /// </summary>
    public string? ProfileFor(string romPath)
        => romPath is not null && _romProfiles.TryGetValue(romPath, out string? id) ? id : null;

    /// <summary>Remembers the profile chosen for a ROM file (AMD-010 REQ-UI-201).</summary>
    public void SetProfileFor(string romPath, string profileId)
    {
        if (string.IsNullOrWhiteSpace(romPath) || string.IsNullOrWhiteSpace(profileId))
        {
            return;
        }

        if (_romProfiles.TryGetValue(romPath, out string? current)
            && string.Equals(current, profileId, StringComparison.Ordinal))
        {
            return;
        }

        _romProfiles[romPath] = profileId;
        IsDirty = true;
    }

    /// <summary>Reads the file; an absent file leaves an empty list (REQ-PERS-075).</summary>
    public SettingsLoadResult Load(DateTimeOffset now)
    {
        _recent.Clear();
        _romProfiles.Clear();
        _readOnlySession = false;
        IsDirty = false;

        if (!_storage.Exists(_name))
        {
            return new SettingsLoadResult(SettingsLoadStatus.Absent, null, CurrentSchemaVersion);
        }

        JsonObject? root = null;
        if (_storage.Read(_name) is { } bytes)
        {
            try
            {
                root = JsonNode.Parse(bytes) as JsonObject;
            }
            catch (JsonException)
            {
                root = null;
            }
        }

        if (root is null)
        {
            _storage.Rename(_name, SettingsFile.CorruptName(_name, now));
            IsDirty = true;
            return new SettingsLoadResult(SettingsLoadStatus.Unreadable, UnreadableMessage, 0);
        }

        int version = root["schemaVersion"]?.GetValueKind() == JsonValueKind.Number
            ? root["schemaVersion"]!.GetValue<int>()
            : CurrentSchemaVersion;

        if (root["recentRoms"] is JsonArray recent)
        {
            foreach (JsonNode? entry in recent)
            {
                if (entry?.GetValueKind() == JsonValueKind.String
                    && entry.GetValue<string>() is { Length: > 0 } path
                    && _recent.Count < RecentRomLimit
                    && !_recent.Contains(path, StringComparer.OrdinalIgnoreCase))
                {
                    _recent.Add(path);
                }
            }
        }

        if (root["romProfiles"] is JsonObject associations)
        {
            foreach ((string path, JsonNode? id) in associations)
            {
                if (id?.GetValueKind() == JsonValueKind.String && id.GetValue<string>() is { Length: > 0 } value)
                {
                    _romProfiles[path] = value;
                }
            }
        }

        if (version > CurrentSchemaVersion)
        {
            _readOnlySession = true;
            return new SettingsLoadResult(SettingsLoadStatus.NewerVersion, NewerVersionMessage, version);
        }

        if (version < CurrentSchemaVersion)
        {
            _storage.Copy(_name, $"{_name}.v{version.ToString(CultureInfo.InvariantCulture)}.bak");
            IsDirty = true;
            return new SettingsLoadResult(SettingsLoadStatus.Migrated, null, version);
        }

        return new SettingsLoadResult(SettingsLoadStatus.Loaded, null, version);
    }

    /// <summary>Writes the file atomically.</summary>
    public bool Save()
    {
        if (!_storage.IsWritable || _readOnlySession)
        {
            return false;
        }

        JsonArray recent = [];
        foreach (string path in _recent)
        {
            recent.Add(path);
        }

        JsonObject associations = [];
        foreach ((string path, string id) in _romProfiles)
        {
            associations[path] = id;
        }

        JsonObject root = new()
        {
            ["schemaVersion"] = CurrentSchemaVersion,
            ["recentRoms"] = recent,
            ["romProfiles"] = associations,
        };

        bool written = _storage.Write(
            _name,
            Utf8WithoutMark.GetBytes(root.ToJsonString(SettingsJson.WriteOptions)));
        IsDirty &= !written;
        return written;
    }
}
