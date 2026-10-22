using Zax.Profiles;

namespace Zax.App.Loop;

/// <summary>A library entry resolved to something the loop can start.</summary>
/// <param name="Id">Profile identifier, the key of the library (SPEC-09 REQ-PERS-030).</param>
/// <param name="Title">Title shown for the game.</param>
/// <param name="Profile">The loaded profile; the empty profile for "2D Mode".</param>
/// <param name="Is2DMode">
/// Whether this is the generic "2D Mode" entry, which is reloaded on every launch
/// (REQ-PERS-005, REQ-PERS-006).
/// </param>
/// <param name="IsDefaultThreeDimensional">
/// Whether this is the generic « 3D par défaut (sans profil) » entry of AMD-024 REQ-PUB-001: the
/// same empty profile, but converted in 3D with the default geometry rather than shown as a flat
/// picture (Q-065).
/// </param>
internal sealed record ResolvedGame(
    string Id,
    string Title,
    GameProfile Profile,
    bool Is2DMode,
    bool IsDefaultThreeDimensional = false)
{
    /// <summary>Whether the profile is the built-in empty one, reloaded at every launch.</summary>
    public bool IsGeneric => Is2DMode || IsDefaultThreeDimensional;
}

/// <summary>
/// What the emulation loop needs from the library: resolve an identifier to a profile, find the
/// ROM a user associated with it, and record that it was played.
/// </summary>
/// <remarks>
/// Keeping this an interface is what lets the loop be driven headless: the application plugs the
/// catalogue and <c>library.json</c> of lot L2 behind it, and a test plugs a synthetic ROM and a
/// profile built in memory.
/// </remarks>
internal interface IGameCatalog
{
    /// <summary>
    /// Resolves <paramref name="gameId"/>. Returns null when no profile of that identifier
    /// exists any more, which is the failure of REQ-LOOP-044 step 2.
    /// </summary>
    ResolvedGame? Resolve(string gameId);

    /// <summary>Whether a profile of that identifier exists (REQ-PERS-055).</summary>
    bool Exists(string gameId);

    /// <summary>The ROM path associated with the entry, or null when none is (REQ-PERS-031).</summary>
    string? RomPath(string gameId);

    /// <summary>
    /// The entry already associated with <paramref name="romPath"/>, or null. This is case 1 of
    /// AMD-010 REQ-UI-201: opening a ROM a second time starts straight away with the profile
    /// that was remembered for it.
    /// </summary>
    string? GameForRom(string romPath);

    /// <summary>
    /// Identifier of the generic "2D Mode" entry, which plays any ROM, or null when the
    /// catalogue has none (SPEC-09 §2).
    /// </summary>
    string? TwoDimensionalModeId { get; }

    /// <summary>
    /// Identifier of the generic « 3D par défaut (sans profil) » entry of AMD-024 REQ-PUB-001,
    /// or null when the catalogue has none (Q-065).
    /// </summary>
    string? DefaultThreeDimensionalId { get; }

    /// <summary>Associates a ROM path with the entry and persists it (REQ-PERS-032).</summary>
    void SetRomPath(string gameId, string romPath);

    /// <summary>Stamps the entry with the launch time, in Unix seconds (REQ-PERS-033).</summary>
    void MarkPlayed(string gameId, long unixSeconds);

    /// <summary>
    /// Deletes a user game: its profile file, its cover art and its library entry
    /// (SPEC-09 REQ-PERS-042, SPEC-03 REQ-PROF-065). The slots are the loop's business. A
    /// built-in profile and the "2D Mode" entry are never deletable, and false is returned.
    /// </summary>
    bool DeleteGame(string gameId);

    /// <summary>The displayed title of an entry, or null when there is no such entry.</summary>
    string? Title(string gameId);

    /// <summary>
    /// Whether the entry comes from an imported profile, which is the only kind
    /// <see cref="DeleteGame"/> accepts (REQ-UI-037).
    /// </summary>
    bool CanDelete(string gameId);

    /// <summary>
    /// Records a cover art file name on the entry and persists the library (REQ-PERS-036); null
    /// clears it. Returns false when there is no such entry.
    /// </summary>
    bool SetCover(string gameId, string? fileName);

    /// <summary>The cover art file name recorded on the entry, or null (REQ-PERS-036).</summary>
    string? Cover(string gameId);
}
