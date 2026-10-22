namespace Zax.Profiles.Catalogue;

/// <summary>
/// The catalogue the publishable build ships: two entries written in code, no file at all
/// (AMD-024 REQ-PUB-001, REQ-PUB-002, Q-065).
/// </summary>
/// <remarks>
/// <para>
/// REQ-PUB-002 removes <c>profiles.json</c> and the 101 <c>.bytes</c> profiles from the
/// publishable build, so there is nothing left to read. REQ-PUB-001 still requires the 2D mode
/// and 3D with the default geometry to work, and Q-065 settles how: both are <b>generic</b>
/// entries whose profile is <see cref="GameProfile.CreateEmpty"/>, so neither needs a file.
/// </para>
/// <list type="bullet">
/// <item>
/// <description>
/// <c>"47"</c> « 2D Mode » — the identifier and the title are the ones REQ-PROF-060 and
/// REQ-PROF-061 write in clear, which are interoperability data like the tile key of
/// REQ-PUB-007: keeping them means a <c>library.json</c> written by an earlier version still
/// finds its 2D entry (REQ-PERS-035).
/// </description>
/// </item>
/// <item>
/// <description>
/// <c>"3d"</c> « 3D par défaut (sans profil) » — the entry that converts any ROM in 3D with the
/// default geometry of SPEC-04. It is new to the publishable build; nothing of the original
/// catalogue is reused for it.
/// </description>
/// </item>
/// </list>
/// <para>
/// Everything else in the library comes from the <c>.3dn</c> files the user imported
/// (REQ-PROF-062, REQ-PROF-063). <c>--assets &lt;folder&gt;</c> still reads a user-supplied
/// <c>profiles.json</c> instead of this, and the entries of that file then replace these two
/// whenever their identifiers collide.
/// </para>
/// </remarks>
// Q-065
public static class BuiltInCatalogue
{
    /// <summary>The identifier of the « 2D Mode » entry, <c>"47"</c> (REQ-PROF-060).</summary>
    public const string TwoDimensionalModeId = "47";

    /// <summary>The identifier of the « 3D par défaut (sans profil) » entry.</summary>
    public const string DefaultThreeDimensionalId = "3d";

    /// <summary>The two entries, « 2D Mode » first.</summary>
    public static IReadOnlyList<CatalogueEntry> Entries { get; } =
    [
        new CatalogueEntry(
            TwoDimensionalModeId,
            ProfileCatalogue.TwoDimensionalModeTitle,
            Publisher: string.Empty,
            Description: "Joue n'importe quelle ROM sans conversion 3D.",
            Author: string.Empty,
            File: string.Empty),
        new CatalogueEntry(
            DefaultThreeDimensionalId,
            ProfileCatalogue.DefaultThreeDimensionalTitle,
            Publisher: string.Empty,
            Description: "Joue n'importe quelle ROM en 3D avec la géométrie par défaut.",
            Author: string.Empty,
            File: string.Empty),
    ];

    /// <summary>The catalogue holding <see cref="Entries"/>, with no asset folder behind it.</summary>
    public static ProfileCatalogue Create() => ProfileCatalogue.FromEntries(string.Empty, Entries);
}
