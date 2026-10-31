using Zax.Profiles.Catalogue;
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
/// <param name="Origin">
/// Which scanned folder the file came from, shown as the source of the profile (AMD-033 §2).
/// </param>
/// <param name="Favorite">
/// Whether the entry is marked as a favourite (AMD-053 REQ-UI-210). It is read from the library
/// when the chooser is built and refreshed by <see cref="ProfileChooserModel.Refresh"/>, so the
/// mark a <c>favorite.toggle</c> sets is visible at once.
/// </param>
internal sealed record ProfileChoice(
    string Id,
    string Title,
    GameProfileSource Source,
    bool Is2DMode,
    bool IsDefault3D = false,
    ProfileOrigin Origin = ProfileOrigin.User,
    bool Favorite = false)
{
    /// <summary>Whether this is one of the two generic built-in entries.</summary>
    public bool IsGeneric => Is2DMode || IsDefault3D;

    /// <summary>
    /// Where the profile comes from, in the interface language. A user profile says which of the
    /// two scanned folders holds it, which is where AMD-033 §2 asks for the origin to be visible.
    /// </summary>
    public string SourceLabel => UiStrings.Get(Is2DMode
        ? UiKeys.ProfileSourceTwoDimensional
        : IsDefault3D
            ? UiKeys.ProfileSourceDefaultThreeD
            : Source != GameProfileSource.User
                ? UiKeys.ProfileSourceBuiltIn
                : Origin == ProfileOrigin.ProgramFolder
                    ? UiKeys.ProfileSourceProgramFolder
                    : UiKeys.ProfileSourceUser);
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
    private readonly GameLibrary _library;
    private readonly List<ProfileChoice> _all = [];
    private readonly List<ProfileChoice> _filtered = [];

    private string _search = string.Empty;
    private bool _favoritesOnly;

    /// <summary>Builds the chooser over <paramref name="library"/>.</summary>
    public ProfileChooserModel(GameLibrary library)
    {
        ArgumentNullException.ThrowIfNull(library);
        _library = library;

        foreach (GameLibraryEntry entry in library.Sorted(LibrarySortOption.Alphabetical))
        {
            _all.Add(new ProfileChoice(
                entry.Id,
                entry.DisplayTitle,
                entry.Source,
                entry.Is2DMode,
                entry.IsDefaultThreeDimensional,
                entry.Origin,
                entry.Favorite));
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
    /// The search text of AMD-053 REQ-UI-211: an entry is kept when its title <b>contains</b>
    /// it, ignoring case, diacritics and the leading and trailing spaces. The normalisation
    /// itself is <see cref="LibrarySearch"/>, which the enumeration of the welcome screen uses
    /// too — the amendment creates no second search.
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

    /// <summary>
    /// The « Favorites only » filter of AMD-053 REQ-UI-210. It combines with
    /// <see cref="Search"/>: an entry is kept when it satisfies both.
    /// </summary>
    public bool FavoritesOnly
    {
        get => _favoritesOnly;
        set
        {
            _favoritesOnly = value;
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

    /// <summary>
    /// Rebuilds the filtered list, taking the favourite marks from the library again so that a
    /// <c>favorite.toggle</c> shows at once.
    /// </summary>
    public void Refresh()
    {
        for (int index = 0; index < _all.Count; index++)
        {
            bool favorite = _library.Find(_all[index].Id) is { Favorite: true };
            if (favorite != _all[index].Favorite)
            {
                _all[index] = _all[index] with { Favorite = favorite };
            }
        }

        _filtered.Clear();
        foreach (ProfileChoice choice in _all)
        {
            if ((!_favoritesOnly || choice.Favorite) && LibrarySearch.Matches(choice.Title, _search))
            {
                _filtered.Add(choice);
            }
        }
    }
}
