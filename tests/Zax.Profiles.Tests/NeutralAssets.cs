namespace Zax.Profiles.Tests;

/// <summary>
/// Locates the extracted assets of <c>cleanroom/assets-neutral</c>, which hold the 101 built-in
/// profiles, the catalogue <c>profiles.json</c> and the key file <c>format-data.json</c>.
/// </summary>
/// <remarks>
/// They sit beside the solution rather than inside it, so the folder is found by walking up
/// from the test binary. A test that needs them is skipped, with a message, when they are not
/// there.
/// </remarks>
internal static class NeutralAssets
{
    /// <summary>Absolute path of <c>assets-neutral</c>, or null when it cannot be found.</summary>
    public static string? Root { get; } = Locate();

    /// <summary>Absolute path of <c>assets-neutral/profiles</c>, or null.</summary>
    public static string? ProfilesDirectory
        => Root is null ? null : Path.Combine(Root, "profiles");

    /// <summary>Absolute path of <c>assets-neutral/profiles.json</c>, or null.</summary>
    public static string? CatalogueFile
        => Root is null ? null : Path.Combine(Root, "profiles.json");

    /// <summary>True when the profiles folder is available.</summary>
    public static bool Available => ProfilesDirectory is not null && Directory.Exists(ProfilesDirectory);

    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "The extracted assets of cleanroom/assets-neutral are not available next to the solution.";

    /// <summary>The 101 built-in profile files, ordered by file name (ordinal).</summary>
    public static IReadOnlyList<string> ProfileFiles()
    {
        if (ProfilesDirectory is null || !Directory.Exists(ProfilesDirectory))
        {
            return [];
        }

        string[] files = Directory.GetFiles(ProfilesDirectory, "*.bytes");
        Array.Sort(files, StringComparer.Ordinal);
        return files;
    }

    /// <summary>Reads one built-in profile by file name, for example <c>empty.bytes</c>.</summary>
    public static byte[] ReadProfile(string fileName)
    {
        if (ProfilesDirectory is null)
        {
            throw new InvalidOperationException(MissingMessage);
        }

        return File.ReadAllBytes(Path.Combine(ProfilesDirectory, fileName));
    }

    /// <summary>Loads one built-in profile, asserting that it loaded.</summary>
    public static ProfileDocument LoadProfile(string fileName)
    {
        ProfileLoadResult result = ProfileFormat.Load(ReadProfile(fileName), sourceName: fileName);
        Assert.True(result.IsSuccess, $"{fileName}: {result.Message}");
        return result.Document!;
    }

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "assets-neutral");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>
/// A fact that is skipped, rather than failed, when <c>assets-neutral</c> is not available.
/// </summary>
public sealed class AssetFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public AssetFactAttribute()
    {
        if (!NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}

/// <summary>
/// A theory that is skipped, rather than failed, when <c>assets-neutral</c> is not available.
/// </summary>
public sealed class AssetTheoryAttribute : TheoryAttribute
{
    /// <summary>Creates the attribute.</summary>
    public AssetTheoryAttribute()
    {
        if (!NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}
