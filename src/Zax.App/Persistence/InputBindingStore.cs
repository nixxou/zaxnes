using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using Zax.App.Input;
using Zax.Platform.Input;

namespace Zax.App.Persistence;

/// <summary>
/// Where the bindings of SPEC-08 REQ-INP-050 are kept. The lot that owns the whole settings
/// model is L3; the input lot only needs this seam (SPEC-09 REQ-PERS-065).
/// </summary>
public interface IInputBindingStore
{
    /// <summary>
    /// Reads the bindings into <paramref name="bindings"/>. An absent or unreadable source
    /// leaves the defaults in place without a blocking error (REQ-INP-052).
    /// </summary>
    void Load(InputBindings bindings);

    /// <summary>Writes the bindings (REQ-INP-051).</summary>
    void Save(InputBindings bindings);

    /// <summary>A notification the last operation asks the host to show, or null.</summary>
    string? Message { get; }
}

/// <summary>A store that keeps nothing, for the headless tests and the libretro frontend.</summary>
public sealed class NullInputBindingStore : IInputBindingStore
{
    /// <summary>The shared instance.</summary>
    public static NullInputBindingStore Instance { get; } = new();

    /// <inheritdoc/>
    public string? Message => null;

    /// <inheritdoc/>
    public void Load(InputBindings bindings)
    {
    }

    /// <inheritdoc/>
    public void Save(InputBindings bindings)
    {
    }
}

/// <summary>
/// Reads and writes <c>input.json</c> in the user root (SPEC-09 REQ-PERS-065): the keyboard sets
/// of both players, one set per gamepad identity ever seen, and the last pad of each player.
/// </summary>
/// <remarks>
/// <para>
/// The file carries a <c>schemaVersion</c> like every other persistent file of the
/// reimplementation (REQ-PERS-024). A file from a newer version is read as best it can be and is
/// not written again during the session; an unreadable file is set aside as
/// <c>input.json.corrupt-&lt;timestamp&gt;</c> and the defaults are used (REQ-INP-052).
/// </para>
/// <para>
/// A single malformed entry degrades to "unbound" on its own rather than failing the whole file,
/// which is the same tolerance REQ-PERS-003 asks of a single invalid setting value.
/// </para>
/// </remarks>
public sealed class InputBindingStore : IInputBindingStore
{
    /// <summary>Schema version this lot writes.</summary>
    public const int SchemaVersion = 1;

    /// <summary>Notification shown when the file could not be read (REQ-INP-052).</summary>
    public const string UnreadableMessage = "Input bindings were unreadable and have been reset.";

    /// <summary>Notification shown for a file from a newer version (REQ-PERS-024).</summary>
    public const string NewerVersionMessage = "Input bindings were created by a newer version.";

    private readonly UserDataPaths _paths;
    private readonly Func<DateTimeOffset> _now;

    /// <summary>Creates a store for the input file of <paramref name="paths"/>.</summary>
    public InputBindingStore(UserDataPaths paths, Func<DateTimeOffset>? now = null)
    {
        ArgumentNullException.ThrowIfNull(paths);
        _paths = paths;
        _now = now ?? (static () => DateTimeOffset.UtcNow);
    }

    /// <inheritdoc/>
    public string? Message { get; private set; }

    /// <summary>
    /// Whether the file came from a newer schema version, in which case it is not written again
    /// during this session (REQ-PERS-024).
    /// </summary>
    public bool IsReadOnlySession { get; private set; }

    /// <inheritdoc/>
    public void Load(InputBindings bindings)
    {
        ArgumentNullException.ThrowIfNull(bindings);

        Message = null;
        IsReadOnlySession = false;

        if (!File.Exists(_paths.InputFile))
        {
            // REQ-PERS-075: first launch starts on the default bindings, no dialogue required.
            return;
        }

        JsonObject? root;
        try
        {
            root = JsonNode.Parse(File.ReadAllBytes(_paths.InputFile)) as JsonObject;
        }
        catch (Exception exception) when (exception is JsonException or IOException or UnauthorizedAccessException)
        {
            root = null;
        }

        if (root is null)
        {
            UserDataPaths.SetAsideCorruptFile(_paths.InputFile, _now());
            Message = UnreadableMessage;
            return;
        }

        if (TryInt(root["schemaVersion"]) is { } version && version > SchemaVersion)
        {
            IsReadOnlySession = true;
            Message = NewerVersionMessage;
        }

        // Loading is one change, not one per binding: the Changed event must not write the file
        // back while it is being read.
        using (bindings.Mute())
        {
            bindings.ResetAll();
            ReadKeyboards(root, bindings);
            ReadGamepads(root, bindings);
            ReadLastGamepads(root, bindings);
        }
    }

    /// <inheritdoc/>
    public void Save(InputBindings bindings)
    {
        ArgumentNullException.ThrowIfNull(bindings);

        if (!_paths.IsWritable || IsReadOnlySession)
        {
            return;
        }

        JsonObject keyboards = [];
        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            keyboards[Index(player)] = WriteSet(bindings.Keyboard(player));
        }

        JsonObject gamepads = [];
        foreach (string identity in bindings.GamepadIdentities)
        {
            if (bindings.FindGamepad(identity) is { } set)
            {
                gamepads[identity] = WriteSet(set);
            }
        }

        JsonObject last = [];
        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            string? identity = bindings.LastGamepad(player);
            last[Index(player)] = identity is null ? null : JsonValue.Create(identity);
        }

        JsonObject root = new()
        {
            ["schemaVersion"] = SchemaVersion,
            ["keyboard"] = keyboards,
            ["gamepads"] = gamepads,
            ["lastGamepad"] = last,
        };

        try
        {
            UserDataPaths.WriteAtomicText(
                _paths.InputFile,
                root.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            Message = _paths.UnwritableMessage ?? exception.Message;
        }
    }

    private static void ReadKeyboards(JsonObject root, InputBindings bindings)
    {
        if (root["keyboard"] is not JsonObject keyboards)
        {
            return;
        }

        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            if (keyboards[Index(player)] is JsonObject entries)
            {
                bindings.Keyboard(player).Replace(ReadSet(entries));
            }
        }
    }

    private static void ReadGamepads(JsonObject root, InputBindings bindings)
    {
        if (root["gamepads"] is not JsonObject gamepads)
        {
            return;
        }

        foreach ((string identity, JsonNode? node) in gamepads)
        {
            if (string.IsNullOrWhiteSpace(identity) || node is not JsonObject entries)
            {
                continue;
            }

            // The set is created empty and then filled: a remembered set is authoritative and
            // must not be mixed with the §4.3 defaults (REQ-INP-025).
            bindings.Gamepad(identity, standard: false).Replace(ReadSet(entries));
        }
    }

    private static void ReadLastGamepads(JsonObject root, InputBindings bindings)
    {
        if (root["lastGamepad"] is not JsonObject last)
        {
            return;
        }

        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            JsonNode? node = last[Index(player)];
            string? identity = node?.GetValueKind() == JsonValueKind.String
                ? node.GetValue<string>()
                : null;

            bindings.SetLastGamepad(player, string.IsNullOrWhiteSpace(identity) ? null : identity);
        }
    }

    private static JsonObject WriteSet(BindingSet set)
    {
        JsonObject entries = [];
        foreach ((BindingTarget target, PhysicalControl control) in set.Bindings)
        {
            entries[target.Key] = control.ToString();
        }

        return entries;
    }

    private static IEnumerable<KeyValuePair<BindingTarget, PhysicalControl>> ReadSet(JsonObject entries)
    {
        foreach ((string key, JsonNode? node) in entries)
        {
            BindingTarget target = BindingTarget.Parse(key);
            if (target.IsEmpty || node?.GetValueKind() != JsonValueKind.String)
            {
                continue;
            }

            PhysicalControl control = PhysicalControl.Parse(node.GetValue<string>());
            if (control.IsBound)
            {
                yield return new KeyValuePair<BindingTarget, PhysicalControl>(target, control);
            }
        }
    }

    private static string Index(int player) => player.ToString(CultureInfo.InvariantCulture);

    private static int? TryInt(JsonNode? node)
        => node?.GetValueKind() == JsonValueKind.Number && node.AsValue().TryGetValue(out double value)
            ? (int)Math.Round(value)
            : null;
}
