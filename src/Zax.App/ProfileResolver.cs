using Zax.Profiles;
using Zax.Profiles.Catalogue;

namespace Zax.App;

/// <summary>What a profile request resolved to.</summary>
/// <param name="Profile">The loaded profile; the empty profile for "2D Mode".</param>
/// <param name="Title">Title to show.</param>
/// <param name="Source">Where it came from, for the log.</param>
internal sealed record ResolvedProfile(GameProfile Profile, string Title, string Source);

/// <summary>
/// Resolves <c>--profile</c> to a profile: a built-in catalogue title, a profile file, or the
/// "2D Mode" entry when nothing is asked for (SPEC-03 REQ-PROF-060, SPEC-06 REQ-SCN-055).
/// </summary>
internal static class ProfileResolver
{
    /// <summary>
    /// Resolves <paramref name="request"/> against the catalogue found under
    /// <paramref name="assetRoot"/>.
    /// </summary>
    /// <param name="request">A catalogue title, a file path, or null for "2D Mode".</param>
    /// <param name="assetRoot">The <c>assets-neutral</c> folder, or null when it is missing.</param>
    /// <param name="log">Receives one line per notable step.</param>
    public static ResolvedProfile Resolve(string? request, string? assetRoot, Action<string> log)
    {
        ArgumentNullException.ThrowIfNull(log);

        if (!string.IsNullOrWhiteSpace(request) && File.Exists(request))
        {
            ProfileLoadResult fromFile = ProfileFormat.Load(File.ReadAllBytes(request), sourceName: request);
            if (!fromFile.IsSuccess)
            {
                throw new InvalidOperationException($"{request}: {fromFile.Message}");
            }

            GameProfile loaded = fromFile.Profile!;
            return new ResolvedProfile(
                loaded,
                loaded.Title ?? Path.GetFileNameWithoutExtension(request),
                request);
        }

        // AMD-024 REQ-PUB-001 (Q-065): with no asset folder the catalogue is the built-in one,
        // which holds « 2D Mode » and « 3D par défaut (sans profil) » and no file.
        ProfileCatalogue catalogue = assetRoot is null
            ? BuiltInCatalogue.Create()
            : NeutralAssets.LoadCatalogue(assetRoot);
        CatalogueEntry? entry = string.IsNullOrWhiteSpace(request)
            ? catalogue.TwoDimensionalMode
            : FindByTitleOrId(catalogue, request);

        if (entry is null)
        {
            throw new InvalidOperationException(
                $"no built-in profile matches '{request}'. Use --list-profiles to see the titles.");
        }

        // A built-in entry with no file is the empty profile, built in code (Q-065).
        if (string.IsNullOrEmpty(entry.File))
        {
            return new ResolvedProfile(GameProfile.CreateEmpty(), entry.Title, "built-in");
        }

        ProfileLoadResult result = catalogue.LoadProfile(PhysicalProfileFileSystem.Instance, entry);
        if (!result.IsSuccess)
        {
            // REQ-PROF-033: a refused profile falls back to the empty profile.
            log($"{entry.Title}: {result.Message}; falling back to the empty profile.");
            return new ResolvedProfile(GameProfile.CreateEmpty(), entry.Title, entry.File);
        }

        return new ResolvedProfile(result.Profile!, entry.Title, entry.File);
    }

    /// <summary>The catalogue entries, ordered by title.</summary>
    public static IReadOnlyList<CatalogueEntry> List(string? assetRoot)
    {
        List<CatalogueEntry> entries = assetRoot is null
            ? [.. BuiltInCatalogue.Entries]
            : [.. NeutralAssets.LoadCatalogue(assetRoot).Entries];
        entries.Sort(static (a, b) => string.Compare(a.Title, b.Title, StringComparison.OrdinalIgnoreCase));
        return entries;
    }

    private static CatalogueEntry? FindByTitleOrId(ProfileCatalogue catalogue, string request)
    {
        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            if (string.Equals(entry.Title, request, StringComparison.OrdinalIgnoreCase)
                || string.Equals(entry.Id, request, StringComparison.OrdinalIgnoreCase)
                || string.Equals(entry.File, request, StringComparison.OrdinalIgnoreCase))
            {
                return entry;
            }
        }

        // Then a unique prefix, so that "Super Mario" is enough on the command line.
        CatalogueEntry? match = null;
        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            if (!entry.Title.StartsWith(request, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (match is not null)
            {
                return null;
            }

            match = entry;
        }

        return match;
    }
}
