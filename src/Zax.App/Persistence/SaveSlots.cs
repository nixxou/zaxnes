using System.Text.Json;
using System.Text.Json.Serialization;

namespace Zax.App.Persistence;

/// <summary>One slot as <c>saves/index.json</c> carries it (SPEC-09 §4.5.3).</summary>
public sealed class SaveSlotRecord
{
    /// <summary>Slot identifier: <c>1</c>…<c>36</c> or <c>quick</c>.</summary>
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    /// <summary>Profile identifier of the game, empty when the slot is empty.</summary>
    [JsonPropertyName("gameId")]
    public string GameId { get; set; } = string.Empty;

    /// <summary>Save time in Unix seconds, 0 when the slot is empty.</summary>
    [JsonPropertyName("savedAt")]
    public long SavedAt { get; set; }

    /// <summary>State file name, relative to <c>saves/</c>.</summary>
    [JsonPropertyName("file")]
    public string? File { get; set; }

    /// <summary>Thumbnail file name, relative to <c>saves/</c>.</summary>
    [JsonPropertyName("thumbnail")]
    public string? Thumbnail { get; set; }

    /// <summary>Whether the slot holds a savestate.</summary>
    [JsonIgnore]
    public bool IsOccupied => !string.IsNullOrEmpty(GameId);
}

/// <summary>The contents of <c>saves/index.json</c> (SPEC-09 §4.5.3, REQ-PERS-024).</summary>
public sealed class SaveSlotIndexFile
{
    /// <summary>Schema version of the file.</summary>
    [JsonPropertyName("schemaVersion")]
    public int SchemaVersion { get; set; } = SaveSlotIndex.SchemaVersion;

    /// <summary>The 37 slots.</summary>
    [JsonPropertyName("slots")]
    public List<SaveSlotRecord> Slots { get; set; } = [];
}

/// <summary>
/// The 36 numbered slots and the quick slot of SPEC-09 REQ-PERS-050, their metadata file and
/// the consistency pass of REQ-PERS-055.
/// </summary>
/// <remarks>
/// <para>
/// Slots are common to every game: a slot does not belong to a game, it records which game it
/// holds. The menus of AMD-010 REQ-UI-208 expose the first <see cref="ExposedSlotCount"/>
/// numbered slots plus the quick slot; the model keeps SPEC-09's count, so a state written by a
/// future UI into slot 20 stays loadable.
/// </para>
/// <para>
/// Writing is atomic (REQ-PERS-021) and happens immediately when a slot changes
/// (REQ-LOOP-042).
/// </para>
/// </remarks>
public sealed class SaveSlotIndex
{
    /// <summary>Schema version written to the file.</summary>
    public const int SchemaVersion = 1;

    /// <summary>Numbered slots the model holds (REQ-PERS-050).</summary>
    public const int NumberedSlotCount = 36;

    /// <summary>Numbered slots the menus expose (AMD-010 REQ-UI-208).</summary>
    public const int ExposedSlotCount = 10;

    /// <summary>Identifier of the quick slot.</summary>
    public const string QuickSlotId = "quick";

    private readonly Dictionary<string, SaveSlotRecord> _byId = [];
    private readonly List<SaveSlotRecord> _order = [];

    /// <summary>Creates an index whose 37 slots are all empty.</summary>
    public SaveSlotIndex()
    {
        for (int number = 1; number <= NumberedSlotCount; number++)
        {
            Add(new SaveSlotRecord { Id = number.ToString(System.Globalization.CultureInfo.InvariantCulture) });
        }

        Add(new SaveSlotRecord { Id = QuickSlotId });
    }

    /// <summary>Every slot: <c>1</c>…<c>36</c> then <c>quick</c>.</summary>
    public IReadOnlyList<SaveSlotRecord> Slots => _order;

    /// <summary>Whether the index was rebuilt because its file was unreadable.</summary>
    public bool WasRecovered { get; private set; }

    /// <summary>The slot of that identifier, or null.</summary>
    public SaveSlotRecord? Find(string slotId) => _byId.GetValueOrDefault(slotId);

    /// <summary>The identifier of numbered slot <paramref name="number"/>, 1-based.</summary>
    public static string NumberedSlotId(int number)
        => number.ToString(System.Globalization.CultureInfo.InvariantCulture);

    /// <summary>Whether <paramref name="slotId"/> designates a slot the model holds.</summary>
    public static bool IsValidSlotId(string? slotId)
    {
        if (string.IsNullOrEmpty(slotId))
        {
            return false;
        }

        if (string.Equals(slotId, QuickSlotId, StringComparison.Ordinal))
        {
            return true;
        }

        return int.TryParse(slotId, out int number) && number >= 1 && number <= NumberedSlotCount;
    }

    /// <summary>Records a save in a slot.</summary>
    public SaveSlotRecord Occupy(string slotId, string gameId, long savedAt, bool hasThumbnail)
    {
        SaveSlotRecord slot = Find(slotId)
            ?? throw new ArgumentException($"Unknown slot '{slotId}'.", nameof(slotId));

        slot.GameId = gameId;
        slot.SavedAt = savedAt;
        slot.File = slotId + ".state";
        slot.Thumbnail = hasThumbnail ? slotId + ".png" : null;
        return slot;
    }

    /// <summary>Empties a slot (REQ-PERS-054, REQ-PERS-055).</summary>
    public void Vacate(string slotId)
    {
        if (Find(slotId) is not { } slot)
        {
            return;
        }

        slot.GameId = string.Empty;
        slot.SavedAt = 0;
        slot.File = null;
        slot.Thumbnail = null;
    }

    /// <summary>The occupied slots of a game, newest first.</summary>
    public IReadOnlyList<SaveSlotRecord> OccupiedFor(string gameId) =>
        [.. _order.Where(slot => slot.IsOccupied && string.Equals(slot.GameId, gameId, StringComparison.Ordinal))
            .OrderByDescending(static slot => slot.SavedAt)];

    /// <summary>
    /// Reads <c>saves/index.json</c>. An unreadable file is set aside and an empty index is
    /// returned with <see cref="WasRecovered"/> set, which is what REQ-PERS-055 asks the host to
    /// report (SPEC-09 falls back on all-empty slots, like the original).
    /// </summary>
    public static SaveSlotIndex Load(UserDataPaths paths, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(paths);

        SaveSlotIndex index = new();
        if (!File.Exists(paths.SaveIndexFile))
        {
            return index;
        }

        try
        {
            SaveSlotIndexFile? file = JsonSerializer.Deserialize<SaveSlotIndexFile>(
                File.ReadAllBytes(paths.SaveIndexFile));
            if (file is null)
            {
                throw new JsonException("The slot index is empty.");
            }

            foreach (SaveSlotRecord record in file.Slots)
            {
                if (index.Find(record.Id) is not { } slot || !record.IsOccupied)
                {
                    continue;
                }

                slot.GameId = record.GameId;
                slot.SavedAt = record.SavedAt;
                slot.File = record.File ?? (record.Id + ".state");
                slot.Thumbnail = record.Thumbnail;
            }

            return index;
        }
        catch (Exception exception) when (exception is JsonException or IOException or UnauthorizedAccessException)
        {
            UserDataPaths.SetAsideCorruptFile(paths.SaveIndexFile, now);
            SaveSlotIndex recovered = new() { WasRecovered = true };
            recovered.RebuildFromFiles(paths);
            return recovered;
        }
    }

    /// <summary>
    /// Rebuilds the metadata from the headers of the savestate files present, which is the
    /// SHOULD behaviour of REQ-PERS-055.
    /// </summary>
    public void RebuildFromFiles(UserDataPaths paths)
    {
        ArgumentNullException.ThrowIfNull(paths);

        foreach (SaveSlotRecord slot in _order)
        {
            string file = paths.SlotStateFile(slot.Id);
            if (!File.Exists(file))
            {
                continue;
            }

            try
            {
                if (!SaveStateFile.TryRead(File.ReadAllBytes(file), out SaveStateContent? content)
                    || content is null)
                {
                    continue;
                }

                slot.GameId = content.Header.GameId;
                slot.SavedAt = content.Header.SavedAt;
                slot.File = slot.Id + ".state";
                slot.Thumbnail = File.Exists(paths.SlotThumbnailFile(slot.Id)) ? slot.Id + ".png" : null;
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A slot whose file cannot be read stays empty.
            }
        }
    }

    /// <summary>
    /// The start-up consistency pass of REQ-PERS-055: a slot whose game is gone is emptied and
    /// its file deleted; a slot whose state file is missing is emptied.
    /// </summary>
    /// <param name="paths">Where the files live.</param>
    /// <param name="gameExists">Whether a profile identifier still names a library entry.</param>
    /// <returns>How many slots were emptied.</returns>
    public int Reconcile(UserDataPaths paths, Func<string, bool> gameExists)
    {
        ArgumentNullException.ThrowIfNull(paths);
        ArgumentNullException.ThrowIfNull(gameExists);

        int cleared = 0;
        foreach (SaveSlotRecord slot in _order)
        {
            if (!slot.IsOccupied)
            {
                continue;
            }

            if (!gameExists(slot.GameId))
            {
                DeleteFiles(paths, slot.Id);
                Vacate(slot.Id);
                cleared++;
                continue;
            }

            if (!File.Exists(paths.SlotStateFile(slot.Id)))
            {
                Vacate(slot.Id);
                cleared++;
            }
        }

        return cleared;
    }

    /// <summary>Deletes the state and the thumbnail of a slot, ignoring failures.</summary>
    public static void DeleteFiles(UserDataPaths paths, string slotId)
    {
        ArgumentNullException.ThrowIfNull(paths);

        foreach (string path in new[] { paths.SlotStateFile(slotId), paths.SlotThumbnailFile(slotId) })
        {
            try
            {
                if (File.Exists(path))
                {
                    File.Delete(path);
                }
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // Nothing to do; the slot is empty in the index either way.
            }
        }
    }

    /// <summary>Writes the metadata file atomically (REQ-PERS-021, REQ-PERS-051).</summary>
    public void Save(UserDataPaths paths)
    {
        ArgumentNullException.ThrowIfNull(paths);
        if (!paths.IsWritable)
        {
            return;
        }

        SaveSlotIndexFile file = new()
        {
            SchemaVersion = SchemaVersion,
            Slots = [.. _order],
        };

        string json = JsonSerializer.Serialize(file, new JsonSerializerOptions { WriteIndented = true });
        UserDataPaths.WriteAtomicText(paths.SaveIndexFile, json);
    }

    private void Add(SaveSlotRecord slot)
    {
        _byId.Add(slot.Id, slot);
        _order.Add(slot);
    }
}
