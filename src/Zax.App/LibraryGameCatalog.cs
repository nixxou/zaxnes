using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.App;

/// <summary>
/// The application's library: the embedded catalogue, the user profiles of
/// <c>&lt;root&gt;/profiles</c> and <c>library.json</c>, behind the
/// <see cref="IGameCatalog"/> the emulation loop drives.
/// </summary>
/// <remarks>
/// Profiles are loaded lazily and cached, because a profile is several megabytes of patterns and
/// the loop only ever needs the one being played. The ROM paths and launch dates are persisted
/// as SPEC-09 REQ-PERS-032 and REQ-PERS-037 require: right after a change, and at exit.
/// </remarks>
internal sealed class LibraryGameCatalog : IGameCatalog
{
    private readonly GameLibrary _library;
    private readonly GameLibraryStore _store;
    private readonly UserDataPaths _paths;
    private readonly Action<string> _log;
    private readonly Dictionary<string, GameProfile> _profiles = new(StringComparer.Ordinal);

    private LibraryGameCatalog(
        GameLibrary library,
        GameLibraryStore store,
        UserDataPaths paths,
        Action<string> log)
    {
        _library = library;
        _store = store;
        _paths = paths;
        _log = log;
    }

    /// <summary>The library model, for the UI and the tests.</summary>
    public GameLibrary Library => _library;

    /// <inheritdoc/>
    public string? TwoDimensionalModeId
        => _library.Entries.FirstOrDefault(static entry => entry.Is2DMode)?.Id;

    /// <inheritdoc/>
    public string? DefaultThreeDimensionalId
        => _library.Entries.FirstOrDefault(static entry => entry.IsDefaultThreeDimensional)?.Id;

    /// <summary>
    /// Builds the library: the catalogue under <paramref name="assetRoot"/>, the user profiles
    /// of the user root and the persisted <c>library.json</c>, reconciled as REQ-PERS-035 asks.
    /// </summary>
    public static LibraryGameCatalog Open(string? assetRoot, UserDataPaths paths, Action<string>? log = null)
    {
        ArgumentNullException.ThrowIfNull(paths);
        Action<string> logger = log ?? (static _ => { });

        ProfileCatalogue? catalogue = null;
        if (assetRoot is not null)
        {
            try
            {
                catalogue = NeutralAssets.LoadCatalogue(assetRoot);
            }
            catch (Exception exception) when (exception is ProfileCatalogueException or IOException)
            {
                logger($"the profile catalogue could not be read: {exception.Message}");
            }
        }

        // AMD-024 REQ-PUB-001 (Q-065): with no asset folder the catalogue is the one written in
        // code — « 2D Mode » and « 3D par défaut (sans profil) », neither of which has a file.
        catalogue ??= BuiltInCatalogue.Create();

        GameLibraryStore store = new(PhysicalProfileFileSystem.Instance);
        GameLibraryFile? persisted = store.Load(paths.LibraryFile, DateTimeOffset.UtcNow);

        IReadOnlyList<UserProfile> userProfiles = [];
        try
        {
            UserProfileFolder folder = new(PhysicalProfileFileSystem.Instance, paths.ProfilesDirectory);
            userProfiles = folder.Scan(catalogue.Ids as IReadOnlyCollection<string>);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            logger($"the user profile folder could not be read: {exception.Message}");
        }

        GameLibrary library = GameLibrary.Build(catalogue, userProfiles, persisted);

        return new LibraryGameCatalog(library, store, paths, logger);
    }

    /// <summary>
    /// Builds the catalogue over an already-built library, which is how a test assembles a
    /// library without an <c>assets-neutral</c> folder on disk.
    /// </summary>
    public static LibraryGameCatalog FromLibrary(
        GameLibrary library,
        UserDataPaths paths,
        Action<string>? log = null)
    {
        ArgumentNullException.ThrowIfNull(library);
        ArgumentNullException.ThrowIfNull(paths);

        return new LibraryGameCatalog(
            library,
            new GameLibraryStore(PhysicalProfileFileSystem.Instance),
            paths,
            log ?? (static _ => { }));
    }

    /// <inheritdoc/>
    public ResolvedGame? Resolve(string gameId)
    {
        if (_library.Find(gameId) is not { } entry)
        {
            return null;
        }

        if (!_profiles.TryGetValue(gameId, out GameProfile? profile))
        {
            profile = LoadProfile(entry);
            _profiles[gameId] = profile;
        }

        return new ResolvedGame(
            entry.Id,
            entry.DisplayTitle,
            profile,
            entry.Is2DMode,
            entry.IsDefaultThreeDimensional);
    }

    /// <inheritdoc/>
    public bool Exists(string gameId) => _library.Find(gameId) is not null;

    /// <inheritdoc/>
    public string? RomPath(string gameId)
        => _library.Find(gameId) is { HasRom: true } entry ? entry.RomPath : null;

    /// <inheritdoc/>
    public string? GameForRom(string romPath)
    {
        foreach (GameLibraryEntry entry in _library.Entries)
        {
            if (entry.HasRom
                && string.Equals(
                    Path.GetFullPath(entry.RomPath),
                    Path.GetFullPath(romPath),
                    StringComparison.OrdinalIgnoreCase))
            {
                return entry.Id;
            }
        }

        return null;
    }

    /// <inheritdoc/>
    public void SetRomPath(string gameId, string romPath)
    {
        if (_library.SetRomPath(gameId, romPath))
        {
            Save();
        }
    }

    /// <inheritdoc/>
    public void MarkPlayed(string gameId, long unixSeconds)
    {
        if (_library.MarkPlayed(gameId, unixSeconds))
        {
            Save();
        }
    }

    /// <inheritdoc/>
    public string? Title(string gameId) => _library.Find(gameId)?.DisplayTitle;

    /// <inheritdoc/>
    public bool CanDelete(string gameId) => _library.Find(gameId) is { CanDelete: true };

    /// <inheritdoc/>
    public string? Cover(string gameId) => _library.Find(gameId)?.Cover;

    /// <inheritdoc/>
    public bool SetCover(string gameId, string? fileName)
    {
        if (_library.Find(gameId) is not { } entry)
        {
            return false;
        }

        entry.Cover = fileName;
        Save();
        return true;
    }

    /// <summary>
    /// The deletion of SPEC-09 REQ-PERS-042 and SPEC-03 REQ-PROF-065: the <c>.3dn</c> file, the
    /// cover art and the library entry go, and <c>library.json</c> is written at once
    /// (REQ-PERS-037). The slots belong to the loop, which clears them before calling this.
    /// </summary>
    /// <inheritdoc/>
    public bool DeleteGame(string gameId)
    {
        if (_library.Find(gameId) is not { CanDelete: true } entry)
        {
            // A built-in profile and the "2D Mode" entry must not be deletable (REQ-UI-037).
            return false;
        }

        string title = entry.Title;
        string? cover = entry.Cover;
        string path = entry.ProfilePath;

        if (!string.IsNullOrEmpty(path))
        {
            try
            {
                UserProfileFolder folder = new(PhysicalProfileFileSystem.Instance, _paths.ProfilesDirectory);
                folder.Delete(new UserProfile(entry.Id, title, path, default));
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                _log($"the profile file of '{gameId}' could not be deleted: {exception.Message}");
            }
        }

        GameArt.DeleteCover(_paths, title, gameId, cover);

        if (!_library.Remove(gameId))
        {
            return false;
        }

        _profiles.Remove(gameId);
        Save();
        _log($"game '{gameId}' deleted with its profile file and its cover.");
        return true;
    }

    /// <summary>
    /// Imports a <c>.3dn</c> file chosen from the interface (AMD-010 REQ-UI-200 "Importer un
    /// profil…", SPEC-03 REQ-PROF-063): the file is copied into
    /// <c>&lt;root&gt;/profiles</c> and the library gains — or updates — its entry, without a
    /// restart.
    /// </summary>
    /// <param name="path">The chosen file.</param>
    /// <returns>
    /// The outcome and the message to show. The texts are the ones of SPEC-10 REQ-UI-084, which
    /// <see cref="ProfileImportMessages"/> holds.
    /// </returns>
    public ProfileImportResult ImportProfile(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        List<UserProfile> known = [];
        foreach (GameLibraryEntry entry in _library.Entries)
        {
            if (entry.Source == GameProfileSource.User)
            {
                known.Add(new UserProfile(entry.Id, entry.Title, entry.ProfilePath, default));
            }
        }

        HashSet<string> builtIn = [];
        foreach (GameLibraryEntry entry in _library.Entries)
        {
            if (entry.Source == GameProfileSource.BuiltIn)
            {
                builtIn.Add(entry.Id);
            }
        }

        UserProfileFolder folder = new(PhysicalProfileFileSystem.Instance, _paths.ProfilesDirectory);
        ProfileImportResult result;
        try
        {
            result = folder.Import(path, builtIn, known);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return new ProfileImportResult(
                ProfileImportOutcome.Rejected,
                null,
                ProfileImportMessages.ObsoleteFormat(path));
        }

        if (result is { Outcome: ProfileImportOutcome.Added or ProfileImportOutcome.Replaced, Profile: { } imported })
        {
            _library.AddUserProfile(imported);

            // A replaced profile must be read again rather than served from the cache.
            _profiles.Remove(imported.Id);
            Save();
        }

        return result;
    }

    /// <summary>Writes <c>library.json</c> (REQ-PERS-037).</summary>
    public void Save()
    {
        if (!_paths.IsWritable)
        {
            return;
        }

        try
        {
            _store.Save(_paths.LibraryFile, _library.ToFile());
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _log($"library.json could not be written: {exception.Message}");
        }
    }

    private GameProfile LoadProfile(GameLibraryEntry entry)
    {
        if (string.IsNullOrEmpty(entry.ProfilePath) || !File.Exists(entry.ProfilePath))
        {
            return GameProfile.CreateEmpty();
        }

        try
        {
            ProfileLoadResult result = ProfileFormat.Load(
                File.ReadAllBytes(entry.ProfilePath),
                sourceName: entry.ProfilePath);

            if (result.IsSuccess && result.Profile is { } profile)
            {
                return profile;
            }

            // REQ-PROF-033: a refused profile falls back to the empty profile rather than
            // stopping the launch.
            _log($"{entry.DisplayTitle}: {result.Message}; falling back to the empty profile.");
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _log($"{entry.DisplayTitle}: {exception.Message}");
        }

        return GameProfile.CreateEmpty();
    }
}
