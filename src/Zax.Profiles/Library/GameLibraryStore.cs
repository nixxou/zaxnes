using System.Text.Json;
using System.Text.Json.Serialization;
using Zax.Profiles.Catalogue;

namespace Zax.Profiles.Library;

/// <summary>One game in <c>library.json</c> (SPEC-09 §4.5.2).</summary>
public sealed class GameLibraryRecord
{
    /// <summary>Profile identifier. Not to be edited by hand.</summary>
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    /// <summary>Displayed title.</summary>
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    /// <summary>Absolute path of the chosen ROM, empty when none.</summary>
    [JsonPropertyName("romPath")]
    public string? RomPath { get; set; }

    /// <summary>Last played, in Unix seconds.</summary>
    [JsonPropertyName("lastPlayed")]
    public long LastPlayed { get; set; }

    /// <summary>Cover-art file name.</summary>
    [JsonPropertyName("cover")]
    public string? Cover { get; set; }
}

/// <summary>The contents of <c>library.json</c> (SPEC-09 §4.5.2, REQ-PERS-024).</summary>
public sealed class GameLibraryFile
{
    /// <summary>The schema version this implementation writes.</summary>
    public const int CurrentSchemaVersion = 1;

    /// <summary>The notice written beside the identifiers.</summary>
    public const string DefaultNotice = "Do not edit the id field.";

    /// <summary>Schema version of the file (REQ-PERS-024).</summary>
    [JsonPropertyName("schemaVersion")]
    public int SchemaVersion { get; set; } = CurrentSchemaVersion;

    /// <summary>A note for whoever opens the file.</summary>
    [JsonPropertyName("notice")]
    public string Notice { get; set; } = DefaultNotice;

    /// <summary>The games.</summary>
    [JsonPropertyName("games")]
    public IList<GameLibraryRecord> Games { get; init; } = [];
}

/// <summary>
/// One entry of the original's <c>rom.json</c> (SPEC-09 §4.4): the ROM paths and the last-played
/// dates, by profile identifier. Reading it is the optional one-off import of REQ-PERS-025.
/// </summary>
/// <param name="Id">Profile identifier.</param>
/// <param name="RomPath">Path of the ROM.</param>
/// <param name="Title">Title the original displayed.</param>
/// <param name="LastTime">Last played, in Unix seconds.</param>
public sealed record RomPathRecord(string Id, string RomPath, string Title, long LastTime);

/// <summary>
/// Reads and writes <c>library.json</c>, and reads the original's <c>rom.json</c>.
/// </summary>
/// <remarks>
/// <para>
/// SPEC-03 REQ-PROF-064 says the chosen ROM path is remembered "in <c>rom.json</c>", naming the
/// original's own file; SPEC-09, which it refers to for the format, describes that file as an
/// observation of the original (§4.4, informative) and fixes <c>library.json</c> as the
/// reimplementation's own store (§4.5.2, REQ-PERS-031). This class therefore persists
/// <c>library.json</c> and can import <c>rom.json</c> once, as REQ-PERS-025 allows. See Q-009.
/// </para>
/// <para>
/// Writing is atomic: a temporary file beside the target, then a replace (REQ-PERS-021).
/// </para>
/// </remarks>
public sealed class GameLibraryStore
{
    private static readonly JsonSerializerOptions WriteOptions = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    private readonly IProfileFileSystem _files;
    private readonly IProfileLog _log;

    /// <summary>Creates a store over a file system.</summary>
    public GameLibraryStore(IProfileFileSystem files, IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(files);
        _files = files;
        _log = log ?? NullProfileLog.Instance;
    }

    /// <summary>
    /// Reads the library file. A missing file gives <see langword="null"/>, which means "first
    /// run": the library is then built from the profiles alone (REQ-PERS-075). An unreadable
    /// file is set aside as <c>library.json.corrupt-&lt;timestamp&gt;</c> and also gives
    /// <see langword="null"/> (REQ-PERS-037).
    /// </summary>
    public GameLibraryFile? Load(string path, DateTimeOffset now)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        if (!_files.FileExists(path))
        {
            return null;
        }

        try
        {
            GameLibraryFile? file = JsonSerializer.Deserialize<GameLibraryFile>(_files.ReadAllText(path));
            if (file is null)
            {
                throw new JsonException("the root of the file is null");
            }

            if (file.SchemaVersion > GameLibraryFile.CurrentSchemaVersion)
            {
                _log.Warning(
                    $"{path}: written by a newer version (schema {file.SchemaVersion}); "
                    + "known values are used and the file is left alone");
            }

            return file;
        }
        catch (Exception exception) when (exception is JsonException or IOException or NotSupportedException)
        {
            string aside = $"{path}.corrupt-{now:yyyyMMdd-HHmmss}";
            _log.Error($"{path}: unreadable library ({exception.Message}); set aside as {aside}");
            try
            {
                _files.MoveFile(path, aside, overwrite: true);
            }
            catch (IOException moveFailure)
            {
                _log.Error($"{path}: could not be set aside ({moveFailure.Message})");
            }

            return null;
        }
    }

    // Q-009: REQ-PROF-064 names rom.json, SPEC-09 names library.json for the reimplementation
    // and makes rom.json an optional one-off import (REQ-PERS-025).

    /// <summary>Writes the library atomically.</summary>
    public void Save(string path, GameLibraryFile file)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        ArgumentNullException.ThrowIfNull(file);
        file.SchemaVersion = GameLibraryFile.CurrentSchemaVersion;
        string temporary = path + ".tmp";
        _files.WriteAllText(temporary, JsonSerializer.Serialize(file, WriteOptions));
        _files.MoveFile(temporary, path, overwrite: true);
    }

    /// <summary>
    /// Reads the ROM associations of an original <c>rom.json</c>. An unreadable file gives an
    /// empty list and a log line.
    /// </summary>
    public IReadOnlyList<RomPathRecord> LoadRomPaths(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        if (!_files.FileExists(path))
        {
            return [];
        }

        try
        {
            return ParseRomPaths(_files.ReadAllText(path));
        }
        catch (Exception exception) when (exception is JsonException or IOException)
        {
            _log.Warning($"{path}: unreadable rom.json ({exception.Message})");
            return [];
        }
    }

    /// <summary>Parses the JSON of an original <c>rom.json</c> (SPEC-09 §4.4).</summary>
    public static IReadOnlyList<RomPathRecord> ParseRomPaths(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        using JsonDocument document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty("Items", out JsonElement items)
            || items.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        List<RomPathRecord> records = [];
        foreach (JsonElement item in items.EnumerateArray())
        {
            string id = ReadString(item, "id");
            if (id.Length == 0)
            {
                continue;
            }

            records.Add(new RomPathRecord(
                id,
                ReadString(item, "romPath"),
                ReadString(item, "title"),
                ReadInt64(item, "lastTime")));
        }

        return records;
    }

    private static string ReadString(JsonElement item, string name)
        => item.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() ?? string.Empty
            : string.Empty;

    private static long ReadInt64(JsonElement item, string name)
        => item.TryGetProperty(name, out JsonElement value)
        && value.ValueKind == JsonValueKind.Number
        && value.TryGetInt64(out long number)
            ? number
            : 0L;
}
