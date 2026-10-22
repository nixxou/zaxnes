using Zax.Profiles.Catalogue;

namespace Zax.App;

/// <summary>
/// The optional local asset folder of AMD-024: a folder of files the <b>user</b> supplies, named
/// by <c>--assets &lt;folder&gt;</c>.
/// </summary>
/// <remarks>
/// <para>
/// AMD-024 REQ-PUB-001 and REQ-PUB-002: neither the repository nor the release contains any asset
/// file, and nothing in either depends on one. The folder is therefore <b>never discovered</b>:
/// it exists only when the command line names it. Earlier lots walked up the directory tree
/// looking for <c>assets-neutral</c>, which made a build started from the solution tree silently
/// pick up the extracted assets — exactly the dependency REQ-PUB-001 forbids.
/// </para>
/// <para>
/// Without the option the host runs on what the code produces: the built-in catalogue of
/// <see cref="BuiltInCatalogue"/>, the procedural skyboxes of REQ-PUB-003, the synthesised
/// audio of REQ-PUB-005 and a system font (REQ-PUB-006). The television is the one shipped
/// file, found by <see cref="TelevisionAsset"/> (AMD-025).
/// </para>
/// </remarks>
internal static class NeutralAssets
{
    /// <summary>The name the extracted asset folder has, used in messages only.</summary>
    public const string FolderName = "assets-neutral";

    /// <summary>Name of the catalogue file inside it.</summary>
    public const string CatalogueFileName = "profiles.json";

    /// <summary>
    /// The folder <paramref name="hint"/> names, or null when the option was not given or the
    /// folder does not exist.
    /// </summary>
    public static string? Locate(string? hint = null)
        => !string.IsNullOrWhiteSpace(hint) && Directory.Exists(hint)
            ? Path.GetFullPath(hint)
            : null;

    /// <summary>Loads the <c>full</c> edition of the catalogue from <paramref name="root"/>.</summary>
    public static ProfileCatalogue LoadCatalogue(string root)
        => ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance,
            Path.Combine(root, CatalogueFileName));
}
