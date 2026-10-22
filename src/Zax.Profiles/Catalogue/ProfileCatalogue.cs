using System.Text.Json;

namespace Zax.Profiles.Catalogue;

/// <summary>
/// One entry of the built-in catalogue (SPEC-03 REQ-PROF-060).
/// </summary>
/// <param name="Id">Game identifier in the library; a string, not a number.</param>
/// <param name="Title">Displayed title.</param>
/// <param name="Publisher">Publisher of the game.</param>
/// <param name="Description">Displayed description.</param>
/// <param name="Author">Author of the profile.</param>
/// <param name="File">Path of the profile bytes, relative to the asset root.</param>
public sealed record CatalogueEntry(
    string Id,
    string Title,
    string Publisher,
    string Description,
    string Author,
    string File)
{
    /// <summary>
    /// True for the "2D Mode" entry, the game that plays without a 3D conversion
    /// (REQ-PROF-061).
    /// </summary>
    public bool Is2DMode
        => string.Equals(Title, ProfileCatalogue.TwoDimensionalModeTitle, StringComparison.Ordinal);

    /// <summary>
    /// True for the built-in « 3D par défaut (sans profil) » entry of AMD-024 REQ-PUB-001: the
    /// game that converts any ROM in 3D with the default geometry, with no profile at all
    /// (Q-065).
    /// </summary>
    // Q-065
    public bool IsDefaultThreeDimensional => string.Equals(
        Title,
        ProfileCatalogue.DefaultThreeDimensionalTitle,
        StringComparison.Ordinal);
}

/// <summary>
/// The catalogue of the built-in profiles, read from <c>assets-neutral/profiles.json</c>
/// (SPEC-03 REQ-PROF-060).
/// </summary>
/// <remarks>
/// <para>
/// The file holds two editions. Only <c>full</c>, with its 101 entries, is used: the
/// <c>demo</c> edition is kept data and ignored, and its identifiers overlap those of
/// <c>full</c> with other games (DEC-03-Q04).
/// </para>
/// <para>
/// For a built-in profile the identity, title, publisher and author shown in the library come
/// from here, not from the metadata inside the profile file (REQ-PROF-022): <c>Jackal (U)</c>
/// carries the title "Tennis" but is listed as "Jackal".
/// </para>
/// </remarks>
public sealed class ProfileCatalogue
{
    /// <summary>The only edition the application uses.</summary>
    public const string FullEdition = "full";

    /// <summary>The edition that is kept as data and never used.</summary>
    public const string DemoEdition = "demo";

    /// <summary>The title of the 2D Mode entry (REQ-PROF-061).</summary>
    public const string TwoDimensionalModeTitle = "2D Mode";

    /// <summary>
    /// The title of the built-in « 3D par défaut (sans profil) » entry of AMD-024 REQ-PUB-001
    /// (Q-065).
    /// </summary>
    // Q-065
    public const string DefaultThreeDimensionalTitle = "3D par défaut (sans profil)";

    private readonly Dictionary<string, CatalogueEntry> _byId;

    /// <summary>
    /// Builds a catalogue over entries the caller already has, which is how the built-in
    /// catalogue of AMD-024 REQ-PUB-001 is produced without reading a file (Q-065,
    /// <see cref="BuiltInCatalogue"/>).
    /// </summary>
    /// <param name="assetRoot">The folder the <c>file</c> paths are relative to.</param>
    /// <param name="entries">The entries, in the order they are shown.</param>
    public static ProfileCatalogue FromEntries(string assetRoot, IReadOnlyList<CatalogueEntry> entries)
    {
        ArgumentNullException.ThrowIfNull(assetRoot);
        ArgumentNullException.ThrowIfNull(entries);
        return new ProfileCatalogue(assetRoot, entries);
    }

    private ProfileCatalogue(string assetRoot, IReadOnlyList<CatalogueEntry> entries)
    {
        AssetRoot = assetRoot;
        Entries = entries;
        _byId = new Dictionary<string, CatalogueEntry>(entries.Count, StringComparer.Ordinal);
        foreach (CatalogueEntry entry in entries)
        {
            _byId[entry.Id] = entry;
        }
    }

    /// <summary>The folder the <c>file</c> paths of the entries are relative to.</summary>
    public string AssetRoot { get; }

    /// <summary>The entries of the <c>full</c> edition, in file order.</summary>
    public IReadOnlyList<CatalogueEntry> Entries { get; }

    /// <summary>The 2D Mode entry, or <see langword="null"/> when the catalogue has none.</summary>
    public CatalogueEntry? TwoDimensionalMode
    {
        get
        {
            foreach (CatalogueEntry entry in Entries)
            {
                if (entry.Is2DMode)
                {
                    return entry;
                }
            }

            return null;
        }
    }

    /// <summary>The identifiers of the built-in entries.</summary>
    public IReadOnlyCollection<string> Ids => _byId.Keys;

    /// <summary>Finds an entry by identifier.</summary>
    public CatalogueEntry? Find(string id) => _byId.GetValueOrDefault(id);

    /// <summary>
    /// Reads the catalogue from the JSON of <c>profiles.json</c>.
    /// </summary>
    /// <param name="json">The file contents.</param>
    /// <param name="assetRoot">The folder the <c>file</c> paths are relative to.</param>
    /// <param name="edition">The edition to read; <see cref="FullEdition"/> by default.</param>
    /// <exception cref="ProfileCatalogueException">The JSON is not a readable catalogue.</exception>
    public static ProfileCatalogue Parse(string json, string assetRoot, string edition = FullEdition)
    {
        ArgumentNullException.ThrowIfNull(json);
        ArgumentNullException.ThrowIfNull(assetRoot);
        try
        {
            using JsonDocument document = JsonDocument.Parse(json);
            if (!document.RootElement.TryGetProperty("editions", out JsonElement editions))
            {
                throw new ProfileCatalogueException("the catalogue has no \"editions\" object");
            }

            if (!editions.TryGetProperty(edition, out JsonElement list) || list.ValueKind != JsonValueKind.Array)
            {
                throw new ProfileCatalogueException($"the catalogue has no \"{edition}\" edition");
            }

            List<CatalogueEntry> entries = [];
            foreach (JsonElement item in list.EnumerateArray())
            {
                entries.Add(new CatalogueEntry(
                    ReadString(item, "id"),
                    ReadString(item, "title"),
                    ReadString(item, "publisher"),
                    ReadString(item, "description"),
                    ReadString(item, "author"),
                    ReadString(item, "file")));
            }

            return new ProfileCatalogue(assetRoot, entries);
        }
        catch (JsonException exception)
        {
            throw new ProfileCatalogueException($"the catalogue is not valid JSON ({exception.Message})", exception);
        }
    }

    /// <summary>
    /// Reads the catalogue from a file; the asset root is the folder that holds it.
    /// </summary>
    /// <exception cref="ProfileCatalogueException">The file is missing or unreadable.</exception>
    public static ProfileCatalogue Load(
        IProfileFileSystem files,
        string cataloguePath,
        string edition = FullEdition)
    {
        ArgumentNullException.ThrowIfNull(files);
        ArgumentNullException.ThrowIfNull(cataloguePath);
        if (!files.FileExists(cataloguePath))
        {
            throw new ProfileCatalogueException($"the catalogue file {cataloguePath} is missing");
        }

        string root = Path.GetDirectoryName(cataloguePath) ?? string.Empty;
        return Parse(files.ReadAllText(cataloguePath), root, edition);
    }

    /// <summary>
    /// The full path of the profile bytes of an entry, or the empty string for an entry that has
    /// no file — the two built-in entries of AMD-024 REQ-PUB-001, whose profile is the empty
    /// profile built in code (Q-065).
    /// </summary>
    public string ProfilePath(CatalogueEntry entry)
    {
        ArgumentNullException.ThrowIfNull(entry);
        return string.IsNullOrEmpty(entry.File)
            ? string.Empty
            : Path.Combine(AssetRoot, entry.File.Replace('/', Path.DirectorySeparatorChar));
    }

    /// <summary>
    /// Loads the profile of an entry. A profile that is refused gives a failed result and the
    /// game falls back to the empty profile (REQ-PROF-033).
    /// </summary>
    public ProfileLoadResult LoadProfile(
        IProfileFileSystem files,
        CatalogueEntry entry,
        IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(files);
        ArgumentNullException.ThrowIfNull(entry);
        string path = ProfilePath(entry);
        if (!files.FileExists(path))
        {
            (log ?? NullProfileLog.Instance).Error($"{path}: the built-in profile file is missing");
            return ProfileLoadResult.Failure(ProfileLoadError.Malformed, "the profile file is missing");
        }

        return ProfileFormat.Load(files.ReadAllBytes(path), log, path);
    }

    private static string ReadString(JsonElement item, string name)
        => item.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() ?? string.Empty
            : string.Empty;
}

/// <summary>The catalogue file could not be read (SPEC-03 REQ-PROF-060).</summary>
public sealed class ProfileCatalogueException : Exception
{
    /// <summary>Creates the exception.</summary>
    public ProfileCatalogueException(string message)
        : base(message)
    {
    }

    /// <summary>Creates the exception with an inner cause.</summary>
    public ProfileCatalogueException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
