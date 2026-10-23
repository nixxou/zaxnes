using Zax.Profiles.Catalogue;

namespace Zax.Profiles.Library;

/// <summary>Where the profile of a library entry comes from.</summary>
public enum GameProfileSource
{
    /// <summary>One of the 101 built-in profiles of the catalogue.</summary>
    BuiltIn,

    /// <summary>A <c>.3dn</c> file imported by the user.</summary>
    User,
}

/// <summary>How the library is ordered (SPEC-09 REQ-PERS-033, setting <c>ui.sort_option</c>).</summary>
public enum LibrarySortOption
{
    /// <summary>By title, ascending.</summary>
    Alphabetical = 0,

    /// <summary>By last played, most recent first, then by title.</summary>
    Recent = 1,
}

/// <summary>
/// One game of the library (SPEC-03 REQ-PROF-064, SPEC-09 REQ-PERS-030 and REQ-PERS-031).
/// </summary>
/// <remarks>
/// The library is a list of <b>profiles</b>, not of ROMs. There is no automatic ROM
/// recognition — no hash, no CRC, no game database: the user picks a ROM for a game once and
/// the path is remembered under the profile identifier. Any ROM can be paired with any profile;
/// the conversion is only correct if they match.
/// </remarks>
public sealed class GameLibraryEntry
{
    internal GameLibraryEntry(
        string id,
        string title,
        GameProfileSource source,
        string profilePath,
        bool is2DMode,
        bool isDefaultThreeDimensional = false,
        ProfileOrigin origin = ProfileOrigin.User)
    {
        Id = id;
        Title = title;
        Source = source;
        ProfilePath = profilePath;
        Is2DMode = is2DMode;
        IsDefaultThreeDimensional = isDefaultThreeDimensional;
        Origin = origin;
    }

    /// <summary>Profile identifier; the key of everything stored per game. Never changes.</summary>
    public string Id { get; }

    /// <summary>
    /// Displayed title. It starts as the profile's title and the user may change it
    /// (REQ-PERS-034).
    /// </summary>
    public string Title { get; set; }

    /// <summary>Where the profile comes from.</summary>
    public GameProfileSource Source { get; }

    /// <summary>Path of the profile file to load for this game.</summary>
    public string ProfilePath { get; internal set; }

    /// <summary>
    /// Which folder the profile file was found in (AMD-033 §2). It is
    /// <see cref="ProfileOrigin.User"/> for a built-in profile, which comes from the asset
    /// folder or from the code and never from either scanned folder.
    /// </summary>
    public ProfileOrigin Origin { get; internal set; }

    /// <summary>
    /// True for a profile read from the <c>profiles</c> folder next to the executable, whose
    /// file Zaxnes never writes to (AMD-033 §1).
    /// </summary>
    public bool IsFromProgramFolder
        => Source == GameProfileSource.User && Origin == ProfileOrigin.ProgramFolder;

    /// <summary>
    /// True for the "2D Mode" game: it is shown under that name whatever the custom title, and
    /// its profile is read again at every start (REQ-PROF-061).
    /// </summary>
    public bool Is2DMode { get; }

    /// <summary>
    /// True for the built-in « 3D par défaut (sans profil) » game of AMD-024 REQ-PUB-001: any
    /// ROM, converted in 3D with the default geometry and no profile. Like the 2D game, its
    /// profile is read again at every start (Q-065).
    /// </summary>
    // Q-065
    public bool IsDefaultThreeDimensional { get; }

    /// <summary>
    /// True for either of the two generic built-in entries, the ones whose profile is built in
    /// code rather than read from a file.
    /// </summary>
    public bool IsGeneric => Is2DMode || IsDefaultThreeDimensional;

    /// <summary>Absolute path of the ROM the user chose, empty when none.</summary>
    public string RomPath { get; set; } = string.Empty;

    /// <summary>Last time the game was started, in Unix seconds; 0 when never.</summary>
    public long LastPlayed { get; set; }

    /// <summary>Cover-art file name, or <see langword="null"/> when there is none.</summary>
    public string? Cover { get; set; }

    /// <summary>
    /// The name to show: always "2D Mode" for the 2D game (REQ-PROF-061), and likewise the fixed
    /// title for the default-3D game of AMD-024 REQ-PUB-001 (Q-065).
    /// </summary>
    public string DisplayTitle => Is2DMode
        ? ProfileCatalogue.TwoDimensionalModeTitle
        : IsDefaultThreeDimensional
            ? ProfileCatalogue.DefaultThreeDimensionalTitle
            : Title;

    /// <summary>True when a ROM has been chosen, so no file browser is needed to start.</summary>
    public bool HasRom => !string.IsNullOrEmpty(RomPath);

    /// <summary>
    /// True when the game can be deleted: only games that come from a user profile
    /// (REQ-PROF-065), and only from the user's own folder — deleting a game deletes its file,
    /// and the folder next to the executable is read-only for Zaxnes (AMD-033 §1, Q-074).
    /// </summary>
    // Q-074
    public bool CanDelete => Source == GameProfileSource.User && Origin == ProfileOrigin.User;

    /// <inheritdoc/>
    public override string ToString() => $"{Id} {DisplayTitle}";
}

/// <summary>
/// The game library: one entry per available profile, built-in or user, with the ROM path the
/// user chose for each (SPEC-03 §3.9, SPEC-09 §3.3).
/// </summary>
/// <remarks>
/// This is the model and the persistence API only; the screens that show it belong to SPEC-10.
/// </remarks>
public sealed class GameLibrary
{
    private readonly List<GameLibraryEntry> _entries = [];
    private readonly Dictionary<string, GameLibraryEntry> _byId = new(StringComparer.Ordinal);

    private GameLibrary()
    {
    }

    /// <summary>The entries: the built-in games in catalogue order, then the user games.</summary>
    public IReadOnlyList<GameLibraryEntry> Entries => _entries;

    /// <summary>
    /// Builds the library from the catalogue and the user profiles, carrying over what was
    /// persisted and reconciling it (SPEC-09 REQ-PERS-035): an entry whose identifier matches no
    /// available profile is dropped, and a blank entry is created for every new profile.
    /// </summary>
    public static GameLibrary Build(
        ProfileCatalogue catalogue,
        IReadOnlyList<UserProfile>? userProfiles = null,
        GameLibraryFile? persisted = null,
        IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(catalogue);
        IProfileLog logger = log ?? NullProfileLog.Instance;
        GameLibrary library = new();

        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            library.Add(new GameLibraryEntry(
                entry.Id,
                entry.Title,
                GameProfileSource.BuiltIn,
                catalogue.ProfilePath(entry),
                entry.Is2DMode,
                entry.IsDefaultThreeDimensional), logger);
        }

        foreach (UserProfile profile in userProfiles ?? [])
        {
            library.Add(new GameLibraryEntry(
                profile.Id,
                profile.Title,
                GameProfileSource.User,
                profile.FilePath,
                is2DMode: false,
                isDefaultThreeDimensional: false,
                profile.Origin), logger);
        }

        if (persisted is not null)
        {
            library.Apply(persisted, logger);
        }

        return library;
    }

    /// <summary>The entry with that identifier, or <see langword="null"/>.</summary>
    public GameLibraryEntry? Find(string id) => _byId.GetValueOrDefault(id);

    /// <summary>
    /// Adds the entry of a profile imported while the program runs (REQ-PROF-063,
    /// SPEC-09 REQ-PERS-041). A profile that replaces another user profile keeps that entry's ROM
    /// path, title, date and cover, and only its file path is updated; a built-in identifier is
    /// never touched (REQ-PERS-030).
    /// </summary>
    /// <returns>The entry the profile ended up in, or null when the identifier is built in.</returns>
    public GameLibraryEntry? AddUserProfile(UserProfile profile, IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(profile);

        if (Find(profile.Id) is { } existing)
        {
            if (existing.Source != GameProfileSource.User)
            {
                return null;
            }

            // A profile imported into the user folder takes over the entry of a profile of the
            // same identifier found next to the executable (AMD-033 §2: the user folder wins).
            existing.ProfilePath = profile.FilePath;
            existing.Origin = profile.Origin;
            return existing;
        }

        GameLibraryEntry entry = new(
            profile.Id,
            profile.Title,
            GameProfileSource.User,
            profile.FilePath,
            is2DMode: false,
            isDefaultThreeDimensional: false,
            profile.Origin);
        Add(entry, log ?? NullProfileLog.Instance);
        return entry;
    }

    /// <summary>
    /// Remembers the ROM the user chose for a game (REQ-PROF-064, SPEC-09 REQ-PERS-032). Returns
    /// false when the identifier is unknown.
    /// </summary>
    public bool SetRomPath(string id, string romPath)
    {
        ArgumentNullException.ThrowIfNull(romPath);
        if (Find(id) is not { } entry)
        {
            return false;
        }

        entry.RomPath = romPath;
        return true;
    }

    /// <summary>
    /// Records that a game has just been started (SPEC-09 REQ-PERS-033). Returns false when the
    /// identifier is unknown.
    /// </summary>
    public bool MarkPlayed(string id, long unixSeconds)
    {
        if (Find(id) is not { } entry)
        {
            return false;
        }

        entry.LastPlayed = unixSeconds;
        return true;
    }

    /// <summary>
    /// Removes a game. Only a game that comes from a user profile can be removed
    /// (REQ-PROF-065); the file itself is deleted through
    /// <see cref="UserProfileFolder.Delete"/>.
    /// </summary>
    public bool Remove(string id)
    {
        if (Find(id) is not { CanDelete: true } entry)
        {
            return false;
        }

        _entries.Remove(entry);
        _byId.Remove(id);
        return true;
    }

    /// <summary>The entries in the order of <paramref name="option"/> (SPEC-09 REQ-PERS-033).</summary>
    public IReadOnlyList<GameLibraryEntry> Sorted(LibrarySortOption option)
    {
        List<GameLibraryEntry> sorted = [.. _entries];
        sorted.Sort((left, right) => option == LibrarySortOption.Recent
            ? CompareByRecent(left, right)
            : CompareByTitle(left, right));
        return sorted;
    }

    /// <summary>
    /// Applies the ROM paths and dates of an original <c>rom.json</c>
    /// (SPEC-09 §4.4, REQ-PERS-025). Unknown identifiers are ignored. Returns how many entries
    /// were updated.
    /// </summary>
    public int ApplyRomPaths(IReadOnlyList<RomPathRecord> records)
    {
        ArgumentNullException.ThrowIfNull(records);
        int applied = 0;
        foreach (RomPathRecord record in records)
        {
            if (Find(record.Id) is not { } entry)
            {
                continue;
            }

            entry.RomPath = record.RomPath;
            entry.LastPlayed = record.LastTime;
            if (!string.IsNullOrEmpty(record.Title))
            {
                entry.Title = record.Title;
            }

            applied++;
        }

        return applied;
    }

    /// <summary>The library as it is written to <c>library.json</c>.</summary>
    public GameLibraryFile ToFile()
    {
        GameLibraryFile file = new();
        foreach (GameLibraryEntry entry in _entries)
        {
            file.Games.Add(new GameLibraryRecord
            {
                Id = entry.Id,
                Title = entry.Title,
                RomPath = entry.RomPath,
                LastPlayed = entry.LastPlayed,
                Cover = entry.Cover,
            });
        }

        return file;
    }

    private void Add(GameLibraryEntry entry, IProfileLog log)
    {
        if (_byId.TryGetValue(entry.Id, out GameLibraryEntry? existing))
        {
            // A built-in profile always wins; between user profiles the first one wins
            // (SPEC-09 REQ-PERS-030).
            log.Warning(
                $"duplicated profile identifier \"{entry.Id}\": keeping the {existing.Source} profile "
                + $"\"{existing.Title}\" and ignoring \"{entry.Title}\"");
            return;
        }

        _entries.Add(entry);
        _byId.Add(entry.Id, entry);
    }

    private void Apply(GameLibraryFile persisted, IProfileLog log)
    {
        foreach (GameLibraryRecord record in persisted.Games)
        {
            if (record.Id is null)
            {
                continue;
            }

            if (Find(record.Id) is not { } entry)
            {
                log.Information(
                    $"library entry \"{record.Id}\" matches no available profile and has been dropped");
                continue;
            }

            if (!string.IsNullOrEmpty(record.Title))
            {
                entry.Title = record.Title;
            }

            entry.RomPath = record.RomPath ?? string.Empty;
            entry.LastPlayed = record.LastPlayed;
            entry.Cover = record.Cover;
        }
    }

    private static int CompareByTitle(GameLibraryEntry left, GameLibraryEntry right)
    {
        int byTitle = string.Compare(left.DisplayTitle, right.DisplayTitle, StringComparison.OrdinalIgnoreCase);
        if (byTitle != 0)
        {
            return byTitle;
        }

        int byDate = right.LastPlayed.CompareTo(left.LastPlayed);
        return byDate != 0 ? byDate : string.CompareOrdinal(left.Id, right.Id);
    }

    private static int CompareByRecent(GameLibraryEntry left, GameLibraryEntry right)
    {
        int byDate = right.LastPlayed.CompareTo(left.LastPlayed);
        return byDate != 0 ? byDate : CompareByTitle(left, right);
    }
}
