using Zax.Profiles.Library;

namespace Zax.App.Ui;

/// <summary>One line of the profile chooser.</summary>
/// <param name="Id">The library identifier the chooser hands back.</param>
/// <param name="Title">The displayed title.</param>
/// <param name="Source">Built-in catalogue or imported user profile.</param>
/// <param name="Is2DMode">Whether this is the « 2D Mode » entry (SPEC-03, REQ-SCN-055).</param>
/// <param name="IsDefault3D">
/// Whether this is the « 3D par défaut (sans profil) » entry of AMD-024 REQ-PUB-001 (Q-065).
/// </param>
internal sealed record ProfileChoice(
    string Id,
    string Title,
    GameProfileSource Source,
    bool Is2DMode,
    bool IsDefault3D = false)
{
    /// <summary>Whether this is one of the two generic built-in entries.</summary>
    public bool IsGeneric => Is2DMode || IsDefault3D;

    /// <summary>The French word for where the profile comes from.</summary>
    public string SourceLabel => Is2DMode
        ? "sans conversion 3D"
        : IsDefault3D
            ? "géométrie 3D par défaut"
            : Source == GameProfileSource.User ? "profil utilisateur" : "profil intégré";
}

/// <summary>
/// The profile chooser of AMD-010 REQ-UI-201 step 2: the embedded catalogue, the user profiles
/// and the « 2D Mode » entry, searchable by title.
/// </summary>
/// <remarks>
/// The list is the library of SPEC-03 reconciled with <c>library.json</c>, which already holds
/// the three families (<see cref="GameProfileSource"/>), so the chooser never has to read the
/// catalogue itself. « 2D Mode » is pinned first: it is the entry a ROM with no 3D profile is
/// played on (REQ-SCN-055), hence the common answer.
/// </remarks>
internal sealed class ProfileChooserModel
{
    private readonly List<ProfileChoice> _all = [];
    private readonly List<ProfileChoice> _filtered = [];

    private string _search = string.Empty;

    /// <summary>Builds the chooser over <paramref name="library"/>.</summary>
    public ProfileChooserModel(GameLibrary library)
    {
        ArgumentNullException.ThrowIfNull(library);

        foreach (GameLibraryEntry entry in library.Sorted(LibrarySortOption.Alphabetical))
        {
            _all.Add(new ProfileChoice(
                entry.Id,
                entry.DisplayTitle,
                entry.Source,
                entry.Is2DMode,
                entry.IsDefaultThreeDimensional));
        }

        // Q-065: « 2D Mode » first, then « 3D par défaut (sans profil) », then the titles in
        // alphabetical order.
        _all.Sort(static (a, b) =>
        {
            int byRank = Rank(a).CompareTo(Rank(b));
            return byRank != 0
                ? byRank
                : string.Compare(a.Title, b.Title, StringComparison.OrdinalIgnoreCase);

            static int Rank(ProfileChoice choice) => choice.Is2DMode ? 0 : choice.IsDefault3D ? 1 : 2;
        });

        Refresh();
    }

    /// <summary>The ROM file the choice applies to, or null when the current game is being changed.</summary>
    public string? RomPath { get; set; }

    /// <summary>Every choice, 2D Mode first.</summary>
    public IReadOnlyList<ProfileChoice> All => _all;

    /// <summary>The choices the search text keeps.</summary>
    public IReadOnlyList<ProfileChoice> Filtered => _filtered;

    /// <summary>The identifier the user has highlighted, or null.</summary>
    public string? SelectedId { get; set; }

    /// <summary>
    /// The search text. Matching ignores case and leading and trailing spaces, the rule
    /// SPEC-10 REQ-UI-103 sets for the file browser's search.
    /// </summary>
    public string Search
    {
        get => _search;
        set
        {
            _search = value ?? string.Empty;
            Refresh();
        }
    }

    /// <summary>Clears the search and the selection, which opening the chooser does.</summary>
    public void Reset(string? romPath)
    {
        RomPath = romPath;
        SelectedId = null;
        Search = string.Empty;
    }

    private void Refresh()
    {
        _filtered.Clear();
        string needle = _search.Trim();

        foreach (ProfileChoice choice in _all)
        {
            if (needle.Length == 0
                || choice.Title.Contains(needle, StringComparison.OrdinalIgnoreCase))
            {
                _filtered.Add(choice);
            }
        }
    }
}
