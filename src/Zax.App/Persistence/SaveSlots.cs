using System.Text.Json;
using System.Text.Json.Serialization;

namespace Zax.App.Persistence;

/// <summary>One slot as <c>saves/index.json</c> carries it (SPEC-09 §4.5.3).</summary>
public sealed class SaveSlotRecord
{
    /// <summary>
    /// Slot identifier: <c>1</c>…<c>36</c>, <c>quick</c> or <c>resume</c> (AMD-053 §3 point 4).
    /// </summary>
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

    /// <summary>The 38 slots (AMD-053 §3 point 12).</summary>
    [JsonPropertyName("slots")]
    public List<SaveSlotRecord> Slots { get; set; } = [];
}

/// <summary>
/// The 36 numbered slots, the quick slot and the resume slot of SPEC-09 REQ-PERS-050 as
/// AMD-053 §3 point 4 amends it, their metadata file and the consistency pass of REQ-PERS-055.
/// </summary>
/// <remarks>
/// <para>
/// Slots are common to every game: a slot does not belong to a game, it records which game it
/// holds. The menus of AMD-010 REQ-UI-208 expose the first <see cref="ExposedSlotCount"/>
/// numbered slots plus the quick slot; the model keeps SPEC-09's count, so a state written by a
/// future UI into slot 20 stays loadable.
/// </para>
/// <para>
/// The resume slot of AMD-053 REQ-LOOP-076 is the thirty-eighth: it is written only by the
/// normal exit and read only by the <c>resume.last</c> command, and it shows in no Load or Save
/// list (REQ-PERS-052 as AMD-053 §3 point 5 extends it). <see cref="IsUserSlotId"/> is what the
/// lists and the save/load commands ask; <see cref="IsValidSlotId"/> is the model's own range.
/// </para>
/// <para>
/// Writing is atomic (REQ-PERS-021) and happens immediately when a slot changes
/// (REQ-LOOP-042).
/// </para>
/// </remarks>
public sealed class SaveSlotIndex
{
    /// <summary>
    /// Schema version written to the file. Version 2 added the <c>resume</c> entry
    /// (AMD-053 §3 point 12).
    /// </summary>
    public const int SchemaVersion = 2;

    /// <summary>Numbered slots the model holds (REQ-PERS-050).</summary>
    public const int NumberedSlotCount = 36;

    /// <summary>Numbered slots the menus expose (AMD-010 REQ-UI-208).</summary>
    public const int ExposedSlotCount = 10;

    /// <summary>Identifier of the quick slot.</summary>
    public const string QuickSlotId = "quick";

    /// <summary>Identifier of the resume slot (AMD-053 REQ-LOOP-076).</summary>
    public const string ResumeSlotId = "resume";

    private readonly Dictionary<string, SaveSlotRecord> _byId = [];
    private readonly List<SaveSlotRecord> _order = [];

    /// <summary>Creates an index whose 38 slots are all empty.</summary>
    public SaveSlotIndex()
    {
        for (int number = 1; number <= NumberedSlotCount; number++)
        {
            Add(new SaveSlotRecord { Id = number.ToString(System.Globalization.CultureInfo.InvariantCulture) });
        }

        Add(new SaveSlotRecord { Id = QuickSlotId });
        Add(new SaveSlotRecord { Id = ResumeSlotId });
    }

    /// <summary>Every slot: <c>1</c>…<c>36</c>, then <c>quick</c>, then <c>resume</c>.</summary>
    public IReadOnlyList<SaveSlotRecord> Slots => _order;

    /// <summary>Whether the index was rebuilt because its file was unreadable.</summary>
    public bool WasRecovered { get; private set; }

    /// <summary>
    /// The schema version the file carried when it was older than
    /// <see cref="SchemaVersion"/>, or null. A migrated index is written again at the current
    /// version straight away, a <c>.v&lt;N&gt;.bak</c> copy having been kept (REQ-PERS-024).
    /// </summary>
    public int? MigratedFromVersion { get; private set; }

    /// <summary>The slot of that identifier, or null.</summary>
    public SaveSlotRecord? Find(string slotId) => _byId.GetValueOrDefault(slotId);

    /// <summary>The identifier of numbered slot <paramref name="number"/>, 1-based.</summary>
    public static string NumberedSlotId(int number)
        => number.ToString(System.Globalization.CultureInfo.InvariantCulture);

    /// <summary>
    /// Whether <paramref name="slotId"/> designates a slot the model holds: one of the 38 of
    /// REQ-PERS-050 as amended, the resume slot included.
    /// </summary>
    public static bool IsValidSlotId(string? slotId)
        => IsUserSlotId(slotId) || string.Equals(slotId, ResumeSlotId, StringComparison.Ordinal);

    /// <summary>
    /// Whether <paramref name="slotId"/> is a slot the user saves to and loads from by hand: a
    /// numbered slot or the quick slot. The resume slot is excluded, which is what keeps it out
    /// of the Load and Save lists and out of the save/load commands (REQ-PERS-052 as AMD-053 §3
    /// point 5 extends it).
    /// </summary>
    public static bool IsUserSlotId(string? slotId)
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

            // REQ-PERS-024 with AMD-053 §3 point 12: a version-1 file is migrated at read time
            // — it simply has no `resume` entry, and the constructor already made an empty one —
            // and a copy is kept before the migrated file is written again.
            if (file.SchemaVersion < SchemaVersion)
            {
                index.MigratedFromVersion = file.SchemaVersion;
                KeepBackup(paths.SaveIndexFile, file.SchemaVersion);
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
    /// Keeps the <c>&lt;name&gt;.v&lt;N&gt;.bak</c> copy REQ-PERS-024 asks for before a migrated
    /// file is written again. A failure costs nothing: the migration itself is in memory.
    /// </summary>
    private static void KeepBackup(string path, int version)
    {
        try
        {
            File.Copy(
                path,
                $"{path}.v{version.ToString(System.Globalization.CultureInfo.InvariantCulture)}.bak",
                overwrite: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A read-only root must not stop the start-up (REQ-PERS-073).
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
    /// <param name="skipResumeSlot">
    /// Whether the resume slot is left strictly alone, which <c>-noresume</c> asks for: that run
    /// neither reads it, writes it nor erases it (AMD-053 REQ-UI-213, Q-088).
    /// </param>
    /// <returns>How many slots were emptied.</returns>
    public int Reconcile(UserDataPaths paths, Func<string, bool> gameExists, bool skipResumeSlot = false)
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

            if (skipResumeSlot && string.Equals(slot.Id, ResumeSlotId, StringComparison.Ordinal))
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

    /// <summary>
    /// The start-up check AMD-053 REQ-LOOP-076 rule 3 asks for on the resume slot: the file is
    /// validated <b>whole</b> — signature, format version and the consistency of every section
    /// length (SPEC-09 §4.5.4) — because a savestate truncated in its body keeps a valid
    /// signature and version, and only the length check can tell. An unreadable or incompatible
    /// slot is emptied and its files deleted, with <b>no</b> notification, and the caller
    /// journals what came back.
    /// </summary>
    /// <param name="paths">Where the files live.</param>
    /// <returns>Why the slot was cleared, or null when it is sound or already empty.</returns>
    /// <remarks>
    /// It costs one read of one file per start-up, bounded by REQ-ARCH-013; the 36 numbered
    /// slots and the quick slot keep the cheap existence check of REQ-PERS-055, since a
    /// corrupted one of those is caught at load time by REQ-LOOP-046.
    /// </remarks>
    public string? ValidateResumeSlot(UserDataPaths paths)
    {
        ArgumentNullException.ThrowIfNull(paths);

        if (Find(ResumeSlotId) is not { IsOccupied: true } slot)
        {
            return null;
        }

        string file = paths.SlotStateFile(ResumeSlotId);
        string? failure = null;
        try
        {
            if (!File.Exists(file))
            {
                failure = "its file is missing";
            }
            else if (!SaveStateFile.TryRead(File.ReadAllBytes(file), out SaveStateContent? content)
                || content is null)
            {
                failure = "its file is unreadable or incompatible";
            }
            else if (!string.Equals(content.Header.GameId, slot.GameId, StringComparison.Ordinal))
            {
                failure = "its file belongs to another game";
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            failure = exception.Message;
        }

        if (failure is null)
        {
            return null;
        }

        DeleteFiles(paths, ResumeSlotId);
        Vacate(ResumeSlotId);
        return failure;
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
