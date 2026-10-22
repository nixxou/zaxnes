using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Zax.Settings;

/// <summary>What happened while a persistent file was being read (REQ-PERS-022, REQ-PERS-024).</summary>
public enum SettingsLoadStatus
{
    /// <summary>No file: every setting holds its default (REQ-PERS-002).</summary>
    Absent,

    /// <summary>Read and applied.</summary>
    Loaded,

    /// <summary>Read, migrated from an earlier schema version and ready to be written again.</summary>
    Migrated,

    /// <summary>
    /// Unreadable: the defaults are in force, the file has been set aside and a notification is
    /// owed to the user.
    /// </summary>
    Unreadable,

    /// <summary>
    /// Written by a newer version: the known keys are applied and the file is left untouched
    /// for the rest of the session.
    /// </summary>
    NewerVersion,
}

/// <summary>The outcome of a read.</summary>
/// <param name="Status">What happened.</param>
/// <param name="Message">The non-blocking notification the host owes the user, or null.</param>
/// <param name="Version">The schema version the file carried.</param>
public readonly record struct SettingsLoadResult(SettingsLoadStatus Status, string? Message, int Version)
{
    /// <summary>
    /// Whether the file must be left alone for the rest of the session, which is the case for a
    /// file from a newer version (REQ-PERS-024).
    /// </summary>
    public bool IsReadOnlySession => Status == SettingsLoadStatus.NewerVersion;
}

/// <summary>
/// One step of the migration chain of REQ-PERS-024: it rewrites the <c>values</c> object of a
/// file at version <see cref="FromVersion"/> into the shape of the next version.
/// </summary>
public interface ISettingsMigration
{
    /// <summary>The version this step upgrades from; it produces <c>FromVersion + 1</c>.</summary>
    int FromVersion { get; }

    /// <summary>Rewrites <paramref name="values"/> in place.</summary>
    void Apply(JsonObject values);
}

/// <summary>
/// Reads and writes <c>settings.json</c> (SPEC-09 §4.5.1): the user preferences indexed by
/// stable key, a <c>schemaVersion</c>, and every key this version does not know kept as it is.
/// </summary>
/// <remarks>
/// <para>
/// Only scope U is persisted, <c>ui.fullscreen</c> among them; the P and S settings are not
/// needed at start-up and their presence in the file is ignored (REQ-PERS-020). Unknown keys
/// survive a rewrite (REQ-PERS-023), so a file written by a newer version — or holding the VR
/// keys the desktop build does not expose — loses nothing. The retired keys of
/// <see cref="SettingsCatalogue.RetiredKeys"/> are the exception: AMD-031 §3 has them read
/// without error and dropped, not written back.
/// </para>
/// <para>
/// A file that cannot be parsed, or whose root is not an object, takes the whole model back to
/// the defaults, is renamed <c>settings.json.corrupt-&lt;yyyyMMdd-HHmmss&gt;</c> and asks for the
/// notification of REQ-PERS-022. A single value of the wrong type or outside its domain is
/// replaced on its own, silently (REQ-PERS-003).
/// </para>
/// </remarks>
public sealed class SettingsFile
{
    /// <summary>The blob name of the settings file (REQ-PERS-072).</summary>
    public const string DefaultName = "settings.json";

    /// <summary>The schema version this build writes.</summary>
    public const int CurrentSchemaVersion = 1;

    /// <summary>Notification of REQ-PERS-022, word for word.</summary>
    public const string UnreadableMessage = "Settings file was unreadable and has been reset.";

    /// <summary>Notification of REQ-PERS-024, word for word.</summary>
    public const string NewerVersionMessage = "Settings were created by a newer version.";

    private static readonly UTF8Encoding Utf8WithoutMark = new(encoderShouldEmitUTF8Identifier: false);

    private readonly ISettingsStorage _storage;
    private readonly string _name;
    private readonly IReadOnlyList<ISettingsMigration> _migrations;

    private JsonObject _unknown = [];
    private bool _readOnlySession;

    /// <summary>Creates a settings file over <paramref name="storage"/>.</summary>
    /// <param name="storage">The blob store the host supplies.</param>
    /// <param name="name">Blob name; <c>settings.json</c> by default.</param>
    /// <param name="migrations">
    /// The migration chain, ordered or not; empty when the schema has never changed.
    /// </param>
    public SettingsFile(
        ISettingsStorage storage,
        string? name = null,
        IEnumerable<ISettingsMigration>? migrations = null)
    {
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _name = string.IsNullOrWhiteSpace(name) ? DefaultName : name;

        List<ISettingsMigration> steps = migrations is null ? [] : [.. migrations];
        steps.Sort(static (a, b) => a.FromVersion.CompareTo(b.FromVersion));
        _migrations = steps;
    }

    /// <summary>The blob name this instance reads and writes.</summary>
    public string Name => _name;

    /// <summary>
    /// Reads the file into <paramref name="model"/>: every U setting it holds is applied through
    /// <see cref="SettingsModel.Set(string, SettingValue, SettingLayer)"/>, so the validation of
    /// REQ-PERS-003 and the notifications of REQ-PERS-009 apply exactly as they do to a change
    /// from the UI.
    /// </summary>
    /// <param name="model">The model to fill.</param>
    /// <param name="now">The clock, for the name of a set-aside file.</param>
    public SettingsLoadResult Load(SettingsModel model, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(model);

        _unknown = [];
        _readOnlySession = false;

        if (!_storage.Exists(_name))
        {
            return new SettingsLoadResult(SettingsLoadStatus.Absent, null, CurrentSchemaVersion);
        }

        byte[]? bytes = _storage.Read(_name);
        JsonObject? root = Parse(bytes);
        if (root is null)
        {
            _storage.Rename(_name, CorruptName(_name, now));
            return new SettingsLoadResult(SettingsLoadStatus.Unreadable, UnreadableMessage, 0);
        }

        int version = root["schemaVersion"]?.GetValueKind() == JsonValueKind.Number
            ? root["schemaVersion"]!.GetValue<int>()
            : CurrentSchemaVersion;

        JsonObject values = root["values"] is JsonObject stored ? (JsonObject)stored.DeepClone() : [];

        SettingsLoadStatus status = SettingsLoadStatus.Loaded;
        string? message = null;

        if (version > CurrentSchemaVersion)
        {
            _readOnlySession = true;
            status = SettingsLoadStatus.NewerVersion;
            message = NewerVersionMessage;
        }
        else if (version < CurrentSchemaVersion)
        {
            // Keep a copy at the old version before the first migrated rewrite (REQ-PERS-024).
            _storage.Copy(_name, $"{_name}.v{version.ToString(CultureInfo.InvariantCulture)}.bak");
            Migrate(values, version);
            status = SettingsLoadStatus.Migrated;
        }

        Apply(model, values);
        return new SettingsLoadResult(status, message, version);
    }

    /// <summary>
    /// Writes the file atomically (REQ-PERS-021). Does nothing when the store refuses writes
    /// (REQ-PERS-073) or when the file came from a newer version (REQ-PERS-024).
    /// </summary>
    public bool Save(SettingsModel model)
    {
        ArgumentNullException.ThrowIfNull(model);

        if (!_storage.IsWritable || _readOnlySession)
        {
            return false;
        }

        JsonObject values = (JsonObject)_unknown.DeepClone();
        HashSet<string> written = new(StringComparer.Ordinal);
        foreach ((string key, SettingValue value) in model.UserPreferences())
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(key);
            if (SettingsJson.ToJson(descriptor, value) is { } node)
            {
                values[key] = node;
                written.Add(key);
            }
        }

        // A preference back at its default is dropped from the file, so the next default change
        // reaches a user who never touched the setting. What counts is the preference layer,
        // which is what UserPreferences() hands back: the value in force may be a script's
        // (REQ-PERS-010) or an explicitly given command-line option's (Q-066), and neither must
        // disturb what the file keeps.
        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            if (descriptor.IsPersisted
                && !_unknown.ContainsKey(descriptor.Key)
                && !written.Contains(descriptor.Key))
            {
                values.Remove(descriptor.Key);
            }
        }

        JsonObject root = new()
        {
            ["schemaVersion"] = CurrentSchemaVersion,
            ["values"] = values,
        };

        return _storage.Write(_name, Utf8WithoutMark.GetBytes(root.ToJsonString(SettingsJson.WriteOptions)));
    }

    /// <summary>
    /// Re-reads the file and applies <b>only</b> the U settings it holds, which is what
    /// <c>Setting:Load()</c> does from Lua (REQ-PERS-015).
    /// </summary>
    public SettingsLoadResult Reload(SettingsModel model, DateTimeOffset now) => Load(model, now);

    /// <summary>The name an unreadable blob is set aside under (REQ-PERS-022).</summary>
    public static string CorruptName(string name, DateTimeOffset now)
        => $"{name}.corrupt-{now.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture)}";

    private static JsonObject? Parse(byte[]? bytes)
    {
        if (bytes is null)
        {
            return null;
        }

        try
        {
            return JsonNode.Parse(bytes) as JsonObject;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private void Migrate(JsonObject values, int fromVersion)
    {
        int version = fromVersion;
        foreach (ISettingsMigration migration in _migrations)
        {
            if (migration.FromVersion != version)
            {
                continue;
            }

            migration.Apply(values);
            version++;
        }
    }

    private void Apply(SettingsModel model, JsonObject values)
    {
        foreach ((string key, JsonNode? node) in values)
        {
            if (SettingsCatalogue.IsRetired(key))
            {
                // AMD-031 §3: a value stored for a retired key — the TV box — is ignored
                // without error and is not kept, so the next save no longer writes it. That is
                // the one exception to the unknown keys of REQ-PERS-023.
                continue;
            }

            SettingDescriptor? descriptor = SettingsCatalogue.Find(key);
            if (descriptor is null)
            {
                // Unknown key: kept as it is and written back untouched (REQ-PERS-023).
                _unknown[key] = node?.DeepClone();
                continue;
            }

            if (!descriptor.IsPersisted)
            {
                // A P or S setting in the file is ignored, not an error (REQ-PERS-020).
                continue;
            }

            if (SettingsJson.TryFromJson(descriptor, node, out SettingValue value))
            {
                model.Set(descriptor.Key, value, SettingLayer.User);
            }
        }
    }
}
