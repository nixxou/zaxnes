namespace Zax.Acceptance.Tests;

/// <summary>Locates <c>cleanroom/assets-neutral</c> from the test binary.</summary>
/// <remarks>
/// AMD-024 REQ-PUB-001: the folder is not part of the publishable project any more, so it may
/// simply not be there. Nothing here throws on that: <see cref="Available"/> says so,
/// <see cref="ProfilePaths"/> comes back empty and <see cref="Root"/> points at a path that does
/// not exist, so a <c>TryOpen</c> on it gives null. The tests that do need the original assets
/// carry <c>[Trait("Requires", "OriginalAssets")]</c> and are skipped by
/// <see cref="CorpusFactAttribute"/>, <see cref="HostFactAttribute"/> and their kin.
/// </remarks>
public static class NeutralAssets
{
    /// <summary>
    /// Root of the neutral asset tree, or a path that does not exist when it was not found.
    /// Check <see cref="Available"/> before reading anything from it.
    /// </summary>
    public static string Root { get; } = Locate()
        ?? Path.Combine(AppContext.BaseDirectory, "assets-neutral-not-available");

    /// <summary>Whether the asset tree is really there.</summary>
    public static bool Available { get; } = Locate() is not null;

    /// <summary>The embedded profile files, sorted by name; empty when there are none.</summary>
    public static IReadOnlyList<string> ProfilePaths { get; } = ReadProfilePaths();

    /// <summary>Reads one embedded profile by file name.</summary>
    public static byte[] ReadProfile(string file) => File.ReadAllBytes(Path.Combine(Root, "profiles", file));

    /// <summary>The catalogue file.</summary>
    public static string CataloguePath => Path.Combine(Root, "profiles.json");

    private static IReadOnlyList<string> ReadProfilePaths()
    {
        string folder = Path.Combine(Root, "profiles");
        return Directory.Exists(folder)
            ? [.. Directory.GetFiles(folder, "*.bytes").Order(StringComparer.Ordinal)]
            : [];
    }

    private static string? Locate()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "assets-neutral");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            at = at.Parent;
        }

        return null;
    }
}
